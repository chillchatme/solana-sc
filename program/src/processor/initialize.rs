use crate::{
    error::ChillError,
    state::{Config, Fees, Recipient},
    utils::{
        assert,
        pda::{self, CONFIG_SEED},
    },
};
use borsh::BorshSerialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program::invoke_signed,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};
use std::convert::TryInto;

pub fn process_initialize(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    fees: Fees,
    recipients: Vec<Recipient>,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let authority = next_account_info(accounts_iter)?;
    let config = next_account_info(accounts_iter)?;
    let mint = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;

    assert::mint_authority(mint, authority.key)?;
    assert::config_pubkey(config.key, mint.key, program_id)?;

    if !config.data_is_empty() {
        return Err(ChillError::ConfigAlreadyInitialized.into());
    }

    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(Config::LEN);

    let (config_pubkey, bump) = pda::config(mint.key, program_id);
    let config_account = Config::new(mint.key, fees, recipients)?;
    let seeds = &[CONFIG_SEED.as_bytes(), mint.key.as_ref(), &[bump]];

    invoke_signed(
        &system_instruction::create_account(
            authority.key,
            &config_pubkey,
            lamports,
            Config::LEN.try_into().unwrap(),
            program_id,
        ),
        &[authority.clone(), config.clone(), system_program.clone()],
        &[seeds],
    )?;

    config_account
        .serialize(&mut *config.data.borrow_mut())
        .map_err(|e| e.into())
}
