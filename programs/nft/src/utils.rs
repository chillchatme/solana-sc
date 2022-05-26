use crate::{
    metaplex_adapter::TokenMetadataProgram,
    state::{Config, NftType},
    ErrorCode,
};
use anchor_lang::{
    prelude::{
        borsh, error, Account, AccountInfo, CpiContext, Program, Rent, Result, Signer, System,
        SystemAccount, Sysvar,
    },
    require, require_eq, require_keys_eq,
    solana_program::{entrypoint::ProgramResult, program::invoke},
    AccountDeserialize, AnchorDeserialize, AnchorSerialize, Key, ToAccountInfo,
};
use anchor_spl::token::{transfer, Mint, Token, TokenAccount, Transfer};
use mpl_token_metadata::{
    instruction::{create_master_edition_v3, create_metadata_accounts_v2},
    state::Creator,
};
use std::collections::HashSet;

#[repr(C)]
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct NftArgs {
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub fees: u16, // 10000 = 100%
}

pub struct TokenBuilder {
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub creators: Option<Vec<Creator>>,
    pub seller_fee_basis_points: u16,
}

#[allow(clippy::too_many_arguments)]
pub fn create_metadata<'info>(
    primary_wallet: &Signer<'info>,
    payer: &Signer<'info>,
    mint: &Account<'info, Mint>,
    metadata: &SystemAccount<'info>,
    system_program: &Program<'info, System>,
    rent_program: &Sysvar<'info, Rent>,
    token_metadata_program: &Program<'info, TokenMetadataProgram>,
    token_builder: TokenBuilder,
) -> ProgramResult {
    invoke(
        &create_metadata_accounts_v2(
            mpl_token_metadata::ID,
            metadata.key(),
            mint.key(),
            primary_wallet.key(),
            payer.key(),
            primary_wallet.key(),
            token_builder.name,
            token_builder.symbol,
            token_builder.uri,
            token_builder.creators,
            token_builder.seller_fee_basis_points,
            true,
            true,
            None,
            None,
        ),
        &[
            primary_wallet.to_account_info(),
            payer.to_account_info(),
            mint.to_account_info(),
            metadata.to_account_info(),
            system_program.to_account_info(),
            rent_program.to_account_info(),
            token_metadata_program.to_account_info(),
        ],
    )
}

pub fn create_master_edition<'info>(
    primary_wallet: &Signer<'info>,
    payer: &Signer<'info>,
    mint: &Account<'info, Mint>,
    metadata: &SystemAccount<'info>,
    master_edition: &SystemAccount<'info>,
    rent_program: &Sysvar<'info, Rent>,
    token_metadata_program: &Program<'info, TokenMetadataProgram>,
) -> ProgramResult {
    invoke(
        &create_master_edition_v3(
            mpl_token_metadata::ID,
            master_edition.key(),
            mint.key(),
            primary_wallet.key(),
            primary_wallet.key(),
            metadata.key(),
            payer.key(),
            Some(0),
        ),
        &[
            master_edition.to_account_info(),
            mint.to_account_info(),
            primary_wallet.to_account_info(),
            primary_wallet.to_account_info(),
            metadata.to_account_info(),
            payer.to_account_info(),
            rent_program.to_account_info(),
            token_metadata_program.to_account_info(),
        ],
    )
}

pub fn sign_metadata<'info>(
    creator: &AccountInfo<'info>,
    metadata: &AccountInfo<'info>,
    metadata_program: &AccountInfo<'info>,
) -> ProgramResult {
    let ix = mpl_token_metadata::instruction::sign_metadata(
        mpl_token_metadata::ID,
        *metadata.key,
        *creator.key,
    );

    invoke(
        &ix,
        &[creator.clone(), metadata.clone(), metadata_program.clone()],
    )
}

pub fn check_recipients(
    config: &Account<Config>,
    recipients_token_accounts: &[AccountInfo],
) -> Result<()> {
    require_eq!(
        config.recipients.len(),
        recipients_token_accounts.len(),
        ErrorCode::WrongRecipientsList
    );

    let mut owners = HashSet::with_capacity(recipients_token_accounts.len());
    for recipient in recipients_token_accounts {
        require_keys_eq!(*recipient.owner, spl_token::ID, ErrorCode::IllegalOwner);

        let recipient_token_account =
            TokenAccount::try_deserialize(&mut recipient.data.borrow().as_ref())?;

        require_eq!(
            recipient_token_account.mint,
            config.mint,
            ErrorCode::WrongRecipientsList
        );

        owners.insert(recipient_token_account.owner);
    }

    require!(
        config
            .recipients
            .iter()
            .all(|r| owners.contains(&r.address)),
        ErrorCode::WrongRecipientsList
    );

    Ok(())
}

pub fn calculate_amounts(
    config: &Config,
    remaining_accounts: &[AccountInfo],
    nft_type: NftType,
) -> Result<Vec<u64>> {
    if config.recipients.is_empty() {
        return Ok(Vec::new());
    }

    let fees = config.fees.of(nft_type);
    let mut amounts = Vec::with_capacity(config.recipients.len());
    amounts.push(0);

    for recipient_token_account in remaining_accounts.iter().skip(1) {
        require_keys_eq!(
            *recipient_token_account.owner,
            spl_token::ID,
            ErrorCode::IllegalOwner
        );

        let token_account =
            TokenAccount::try_deserialize(&mut recipient_token_account.data.borrow().as_ref())?;

        let token_account_owner = token_account.owner;
        let recipient = config
            .recipients
            .iter()
            .find(|r| r.address == token_account_owner)
            .unwrap();

        let amount = (fees as u128)
            .checked_mul(recipient.mint_share.into())
            .and_then(|a| a.checked_div(100))
            .and_then(|a| a.try_into().ok())
            .unwrap();

        amounts.push(amount);
    }

    amounts[0] = fees.checked_sub(amounts.iter().sum()).unwrap();
    Ok(amounts)
}

#[allow(clippy::too_many_arguments)]
pub fn transfer_chill<'info>(
    chill_payer: &Signer<'info>,
    chill_payer_token_account: &Account<'info, TokenAccount>,
    token_program: &Program<'info, Token>,
    remaining_accounts: &[AccountInfo<'info>],
    amounts: Vec<u64>,
) -> Result<()> {
    for (receiver_token_account, amount) in remaining_accounts.iter().zip(amounts) {
        if chill_payer_token_account.key() == receiver_token_account.key() {
            continue;
        }

        let ctx = CpiContext::new(
            token_program.to_account_info(),
            Transfer {
                from: chill_payer_token_account.to_account_info(),
                to: receiver_token_account.to_account_info(),
                authority: chill_payer.to_account_info(),
            },
        );

        transfer(ctx, amount)?;
    }

    Ok(())
}
