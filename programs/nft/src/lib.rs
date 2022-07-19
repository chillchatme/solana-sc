use anchor_lang::{
    prelude::*,
    solana_program::{program::invoke, program_option::COption},
};
use anchor_spl::token::{Mint, Token, TokenAccount};
use metaplex_adapter::{Metadata, TokenMetadataProgram};
use mpl_token_metadata::{
    instruction::update_metadata_accounts_v2,
    state::{Creator, DataV2, EDITION, PREFIX},
};
use state::{ChillNftMetadata, Config, Fees, NftType, Recipient, AUTHORITY_SHARE};
use std::collections::HashSet;
use utils::{
    calculate_amounts, check_recipients, create_master_edition, create_metadata, transfer_chill,
    NftArgs, TokenBuilder,
};

declare_id!("E9Zy6VNmQNXj4MiCLjgzJ2png3zfQfosdxRiQ5bornAM");

pub mod event;
pub mod metaplex_adapter;
pub mod state;
pub mod utils;

#[program]
pub mod chill_nft {

    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        fees: Fees,
        recipients: Vec<Recipient>,
    ) -> Result<()> {
        let config = &mut ctx.accounts.config;
        let bump = ctx.bumps["config"];
        let set = recipients.iter().map(|r| r.address).collect::<HashSet<_>>();

        require!(set.len() == recipients.len(), DuplicateRecipients);

        require_gte!(
            Config::MAX_RECIPIENT_NUMBER,
            recipients.len(),
            ErrorCode::MaximumRecipientsNumberExceeded,
        );

        if !recipients.is_empty() {
            let mint_share_sum = recipients.iter().map(|r| r.mint_share).sum::<u8>();
            let transaction_share_sum = recipients.iter().map(|r| r.transaction_share).sum::<u8>();

            require_eq!(mint_share_sum, 100, ErrorCode::InvalidShares);
            require_eq!(transaction_share_sum, 100, ErrorCode::InvalidShares);
        }

        config.bump = bump;
        config.mint = ctx.accounts.chill_mint.key();
        config.primary_wallet = ctx.accounts.primary_wallet.key();
        config.fees = fees;
        config.recipients = recipients;

        Ok(())
    }

    pub fn mint_nft<'info>(
        ctx: Context<'_, '_, '_, 'info, MintNft<'info>>,
        nft_type: NftType,
        args: NftArgs,
        creator: Option<Pubkey>,
    ) -> Result<()> {
        let nft_chill_metadata = &mut ctx.accounts.nft_chill_metadata;
        let nft_chill_bump = ctx.bumps["nft_chill_metadata"];
        nft_chill_metadata.bump = nft_chill_bump;
        nft_chill_metadata.nft_type = nft_type;

        let primary_wallet_key = ctx.accounts.primary_wallet.key();
        let creators = match creator {
            Some(creator) if creator != primary_wallet_key => {
                vec![
                    Creator {
                        address: primary_wallet_key,
                        verified: true,
                        share: AUTHORITY_SHARE,
                    },
                    Creator {
                        address: creator,
                        verified: false,
                        share: 100 - AUTHORITY_SHARE,
                    },
                ]
            }
            _ => {
                vec![Creator {
                    address: primary_wallet_key,
                    verified: true,
                    share: 100,
                }]
            }
        };

        let token_builder = TokenBuilder {
            name: args.name,
            symbol: args.symbol,
            uri: args.uri,
            creators: Some(creators),
            seller_fee_basis_points: args.fees,
        };

        let accounts = &ctx.accounts;
        create_metadata(
            &accounts.primary_wallet,
            &accounts.payer,
            &accounts.nft_mint,
            &accounts.nft_metadata,
            &accounts.system_program,
            &accounts.rent,
            &accounts.token_metadata_program,
            token_builder,
        )?;

        create_master_edition(
            &accounts.primary_wallet,
            &accounts.payer,
            &accounts.nft_mint,
            &accounts.nft_metadata,
            &accounts.nft_master_edition,
            &accounts.rent,
            &accounts.token_metadata_program,
        )?;

        let recipients = ctx.remaining_accounts;
        check_recipients(&accounts.config, recipients)?;

        let recipients_amounts = calculate_amounts(&accounts.config, recipients, nft_type)?;
        transfer_chill(
            &accounts.chill_payer,
            &accounts.chill_payer_token_account,
            &accounts.token_program,
            recipients,
            recipients_amounts,
        )?;

        emit!(event::MintNft {
            mint: accounts.nft_mint.key(),
            nft_type
        });

        Ok(())
    }

    pub fn update_nft(ctx: Context<UpdateNft>, args: NftArgs) -> Result<()> {
        let primary_wallet = &ctx.accounts.primary_wallet;
        let metadata = &ctx.accounts.nft_metadata;
        let token_metadata_program = &ctx.accounts.token_metadata_program;

        let data = DataV2 {
            name: args.name,
            symbol: args.symbol,
            uri: args.uri,
            seller_fee_basis_points: args.fees,
            creators: metadata.data.creators.clone(),
            collection: metadata.collection.clone(),
            uses: metadata.uses.clone(),
        };

        let ix = update_metadata_accounts_v2(
            mpl_token_metadata::ID,
            metadata.key(),
            primary_wallet.key(),
            None,
            Some(data),
            None,
            None,
        );

        invoke(
            &ix,
            &[
                primary_wallet.to_account_info(),
                metadata.to_account_info(),
                token_metadata_program.to_account_info(),
            ],
        )?;

        emit!(event::UpdateNft {
            mint: ctx.accounts.nft_metadata.mint,
        });

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    pub primary_wallet: SystemAccount<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(init, payer = payer, space = Config::LEN,
              seeds = [Config::SEED, chill_mint.key().as_ref()], bump)]
    pub config: Box<Account<'info, Config>>,

    #[account(constraint = chill_mint.mint_authority == COption::Some(primary_wallet.key()))]
    pub chill_mint: Account<'info, Mint>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct MintNft<'info> {
    pub primary_wallet: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub chill_payer: Signer<'info>,

    #[account(mut, token::authority = chill_payer, token::mint = chill_mint)]
    pub chill_payer_token_account: Box<Account<'info, TokenAccount>>,

    #[account(seeds = [Config::SEED, config.mint.as_ref()], bump = config.bump)]
    pub config: Box<Account<'info, Config>>,

    #[account(address = config.mint)]
    pub chill_mint: Box<Account<'info, Mint>>,

    #[account(mut, mint::authority = primary_wallet, mint::decimals = 0)]
    pub nft_mint: Box<Account<'info, Mint>>,

    #[account(mut, seeds = [PREFIX.as_bytes(), mpl_token_metadata::ID.as_ref(), nft_mint.key().as_ref()],
              seeds::program = mpl_token_metadata::ID, bump)]
    pub nft_metadata: SystemAccount<'info>,

    #[account(mut, seeds = [PREFIX.as_bytes(), mpl_token_metadata::ID.as_ref(),
              nft_mint.key().as_ref(), EDITION.as_bytes()], seeds::program = mpl_token_metadata::ID, bump)]
    pub nft_master_edition: SystemAccount<'info>,

    #[account(init, payer = payer, space = ChillNftMetadata::LEN,
              seeds = [ChillNftMetadata::SEED, nft_mint.key().as_ref()], bump)]
    pub nft_chill_metadata: Box<Account<'info, ChillNftMetadata>>,

    pub rent: Sysvar<'info, Rent>,

    pub system_program: Program<'info, System>,

    pub token_program: Program<'info, Token>,

    pub token_metadata_program: Program<'info, TokenMetadataProgram>,
}

#[derive(Accounts)]
pub struct UpdateNft<'info> {
    pub primary_wallet: Signer<'info>,

    #[account(mut)]
    pub nft_metadata: Account<'info, Metadata>,

    pub token_metadata_program: Program<'info, TokenMetadataProgram>,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Recipients list should have unique addresses")]
    DuplicateRecipients,

    #[msg("Maximum recipients number exceeded")]
    MaximumRecipientsNumberExceeded,

    #[msg("Wrong recipients list")]
    WrongRecipientsList,

    #[msg("Sum of all recipient shares must equal 100")]
    InvalidShares,

    #[msg("Provided owner is not allowed")]
    IllegalOwner,
}
