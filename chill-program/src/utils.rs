use chill_api::state::{Config, NftType};
use mpl_token_metadata::state::Creator;
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program::invoke, program_pack::Pack,
};
use spl_token::state::Account;

pub mod assert;
pub mod nft;

pub struct TokenBuilder {
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub creators: Option<Vec<Creator>>,
    pub seller_fee_basis_points: u16,
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

pub fn set_primary_sell_happened<'info>(
    metadata: &AccountInfo<'info>,
    owner: &AccountInfo<'info>,
    token_account: &AccountInfo<'info>,
    metadata_program: &AccountInfo<'info>,
) -> ProgramResult {
    let ix = mpl_token_metadata::instruction::update_primary_sale_happened_via_token(
        mpl_token_metadata::ID,
        *metadata.key,
        *owner.key,
        *token_account.key,
    );

    invoke(
        &ix,
        &[
            owner.clone(),
            metadata.clone(),
            token_account.clone(),
            metadata_program.clone(),
        ],
    )
}

pub fn transfer_chill<'info>(
    owner: &AccountInfo<'info>,
    from_token_account: &AccountInfo<'info>,
    recipients_token_accounts: &[AccountInfo<'info>],
    token_program: &AccountInfo<'info>,
    config: &Config,
    nft_type: NftType,
) -> ProgramResult {
    let price = config.fees.of(nft_type);
    for recipient_token_account in recipients_token_accounts {
        let token_account = Account::unpack(&recipient_token_account.data.borrow())?;
        let token_owner = token_account.owner;

        if *owner.key == token_owner {
            continue;
        }

        let recipient = config
            .recipients
            .iter()
            .find(|r| r.address == token_owner)
            .unwrap();

        let amount = price
            .checked_mul(recipient.mint_share.into())
            .unwrap()
            .checked_div(100)
            .unwrap();

        let ix = spl_token::instruction::transfer(
            &spl_token::ID,
            from_token_account.key,
            recipient_token_account.key,
            owner.key,
            &[],
            amount,
        )?;

        invoke(
            &ix,
            &[
                owner.clone(),
                from_token_account.clone(),
                recipient_token_account.clone(),
                token_program.clone(),
            ],
        )?;
    }

    Ok(())
}
