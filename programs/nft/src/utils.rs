use crate::state::{Config, NftType, CHILL_METADATA_SEED, CONFIG_SEED};
use mpl_token_metadata::state::{EDITION, PREFIX};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program::invoke, program_pack::Pack,
    pubkey::Pubkey,
};
use spl_token::state::Account;

pub mod assert;
pub mod nft;

pub mod pda {
    use super::*;

    pub fn config(mint: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
        let seeds = &[CONFIG_SEED.as_bytes(), mint.as_ref()];
        Pubkey::find_program_address(seeds, program_id)
    }

    pub fn chill_metadata(mint: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
        let seeds = &[CHILL_METADATA_SEED.as_bytes(), mint.as_ref()];
        Pubkey::find_program_address(seeds, program_id)
    }

    pub fn metadata(mint: &Pubkey) -> Pubkey {
        let seeds = &[
            PREFIX.as_bytes(),
            mpl_token_metadata::ID.as_ref(),
            mint.as_ref(),
        ];

        Pubkey::find_program_address(seeds, &mpl_token_metadata::ID).0
    }

    pub fn master_edition(mint: &Pubkey) -> Pubkey {
        let seeds = &[
            PREFIX.as_bytes(),
            mpl_token_metadata::ID.as_ref(),
            mint.as_ref(),
            EDITION.as_bytes(),
        ];

        Pubkey::find_program_address(seeds, &mpl_token_metadata::ID).0
    }
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
