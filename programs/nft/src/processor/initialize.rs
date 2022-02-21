use crate::{
    error::ChillNftError,
    instruction::InitializeArgs,
    state::{Config, CONFIG_SEED},
    utils::{assert, pda},
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program::invoke_signed,
    program_pack::Pack,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};
use std::convert::TryInto;

pub fn process_initialize(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: InitializeArgs,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let authority = next_account_info(accounts_iter)?;
    let config = next_account_info(accounts_iter)?;
    let mint = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;

    assert::is_mint_authority(mint, authority.key)?;
    assert::is_config_pubkey(config.key, mint.key, program_id)?;

    if !config.data_is_empty() {
        return Err(ChillNftError::ConfigAlreadyInitialized.into());
    }

    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(Config::LEN);

    let (config_pubkey, bump) = pda::config(mint.key, program_id);
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

    let config_account = Config::new(mint.key, args.fees, args.recipients)?;
    Config::pack(config_account, &mut config.data.borrow_mut())
}