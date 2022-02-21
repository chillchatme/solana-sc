use crate::{
    error::ChillNftError,
    state::Config,
    utils::{assert, pda},
};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    program_option::COption, program_pack::Pack, pubkey::Pubkey,
};
use spl_token::state::{Account, Mint};

pub fn owned_by(account: &AccountInfo, program_id: &Pubkey) -> ProgramResult {
    if account.owner != program_id {
        return Err(ProgramError::IllegalOwner);
    }
    Ok(())
}

pub fn is_chill_metadata_pubkey(
    chill_metadata: &Pubkey,
    nft_mint: &Pubkey,
    program_id: &Pubkey,
) -> ProgramResult {
    let chill_metadata_pubkey = pda::chill_metadata(nft_mint, program_id).0;
    if *chill_metadata != chill_metadata_pubkey {
        return Err(ChillNftError::ChillMetadataWrongPubkey.into());
    }
    Ok(())
}

pub fn is_config_pubkey(config: &Pubkey, mint: &Pubkey, program_id: &Pubkey) -> ProgramResult {
    let config_pubkey = pda::config(mint, program_id).0;
    if *config != config_pubkey {
        return Err(ChillNftError::ConfigHasWrongPubkey.into());
    }
    Ok(())
}

pub fn is_config(config: &AccountInfo) -> ProgramResult {
    assert::owned_by(config, &crate::ID)?;
    Config::unpack(&config.data.borrow()).map(|_| ())
}

pub fn is_mint_authority(mint: &AccountInfo, authority: &Pubkey) -> ProgramResult {
    assert::owned_by(mint, &spl_token::ID)?;

    let mint_account = Mint::unpack(&mint.data.borrow())?;
    if mint_account.mint_authority != COption::Some(*authority) {
        return Err(ChillNftError::MintHasAnotherAuthority.into());
    }

    Ok(())
}

pub fn is_token_account(token: &AccountInfo, owner: &Pubkey, mint: &Pubkey) -> ProgramResult {
    assert::owned_by(token, &spl_token::ID)?;

    let token_account = Account::unpack(&token.data.borrow())?;
    if token_account.owner != *owner {
        return Err(ChillNftError::TokenHasAnotherOwner.into());
    }
    if token_account.mint != *mint {
        return Err(ChillNftError::TokenHasAnotherMint.into());
    }

    Ok(())
}

pub fn recipients_match(
    config: &Config,
    recipients_token_accounts: &[AccountInfo],
) -> ProgramResult {
    for recipient in recipients_token_accounts {
        let recipient_token_account = Account::unpack(&recipient.data.borrow())?;
        if recipient_token_account.mint != config.mint {
            return Err(ChillNftError::WrongRecipientsList.into());
        }

        if !config
            .recipients
            .iter()
            .any(|r| r.address == recipient_token_account.owner)
        {
            return Err(ChillNftError::WrongRecipientsList.into());
        }
    }

    Ok(())
}
