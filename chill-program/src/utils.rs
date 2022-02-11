use crate::error::ChillError;
use chill_api::pda;
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    program_option::COption, program_pack::Pack, pubkey::Pubkey,
};
use spl_token::state::{Account, Mint};

pub mod assert {

    use super::*;

    pub fn config_pubkey(config: &Pubkey, mint: &Pubkey, program_id: &Pubkey) -> ProgramResult {
        let config_pda = pda::config(mint, program_id).0;
        if *config != config_pda {
            return Err(ChillError::ConfigHasWrongPubkey.into());
        }
        Ok(())
    }

    pub fn owner(account: &AccountInfo, program_id: &Pubkey) -> ProgramResult {
        if account.owner != program_id {
            return Err(ProgramError::IllegalOwner);
        }
        Ok(())
    }

    pub fn mint_authority(mint: &AccountInfo, authority: &Pubkey) -> ProgramResult {
        assert::owner(mint, &spl_token::ID)?;

        let mint_account = Mint::unpack(&mint.data.borrow())?;
        if mint_account.mint_authority != COption::Some(*authority) {
            return Err(ChillError::MintHasAnotherAuthority.into());
        }

        Ok(())
    }

    pub fn token_account(token: &AccountInfo, owner: &Pubkey, mint: &Pubkey) -> ProgramResult {
        assert::owner(token, &spl_token::ID)?;

        let token_account = Account::unpack(&token.data.borrow())?;
        if token_account.owner != *owner {
            return Err(ChillError::TokenHasAnotherOwner.into());
        }
        if token_account.mint != *mint {
            return Err(ChillError::TokenHasAnotherMint.into());
        }

        Ok(())
    }
}
