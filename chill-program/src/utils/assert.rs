use crate::utils::assert;
use chill_api::{error::ChillProgramError, pda, state::Config};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    program_option::COption, program_pack::Pack, pubkey::Pubkey,
};
use spl_token::state::{Account, Mint};

pub fn owner(account: &AccountInfo, program_id: &Pubkey) -> ProgramResult {
    if account.owner != program_id {
        return Err(ProgramError::IllegalOwner);
    }
    Ok(())
}

pub fn config_pubkey(config: &Pubkey, mint: &Pubkey, program_id: &Pubkey) -> ProgramResult {
    let config_pda = pda::config(mint, program_id).0;
    if *config != config_pda {
        return Err(ChillProgramError::ConfigHasWrongPubkey.into());
    }
    Ok(())
}

pub fn is_config(config: &AccountInfo) -> ProgramResult {
    assert::owner(config, &chill_api::ID)?;
    Config::unpack(&config.data.borrow()).map(|_| ())
}

pub fn mint_authority(mint: &AccountInfo, authority: &Pubkey) -> ProgramResult {
    assert::owner(mint, &spl_token::ID)?;

    let mint_account = Mint::unpack(&mint.data.borrow())?;
    if mint_account.mint_authority != COption::Some(*authority) {
        return Err(ChillProgramError::MintHasAnotherAuthority.into());
    }

    Ok(())
}

pub fn token_account(token: &AccountInfo, owner: &Pubkey, mint: &Pubkey) -> ProgramResult {
    assert::owner(token, &spl_token::ID)?;

    let token_account = Account::unpack(&token.data.borrow())?;
    if token_account.owner != *owner {
        return Err(ChillProgramError::TokenHasAnotherOwner.into());
    }
    if token_account.mint != *mint {
        return Err(ChillProgramError::TokenHasAnotherMint.into());
    }

    Ok(())
}

pub fn recipients(config: &Config, recipients_token_accounts: &[AccountInfo]) -> ProgramResult {
    for recipient in recipients_token_accounts {
        let recipient_token_account = Account::unpack(&recipient.data.borrow())?;
        if recipient_token_account.mint != config.mint {
            return Err(ChillProgramError::WrongRecipientsList.into());
        }

        if !config
            .recipients
            .iter()
            .any(|r| r.address == recipient_token_account.owner)
        {
            return Err(ChillProgramError::WrongRecipientsList.into());
        }
    }

    Ok(())
}
