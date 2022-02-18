use crate::utils::{self, assert, nft, TokenBuilder};
use chill_api::{
    instruction::MintNftArgs,
    pda::{self, CHILL_METADATA_SEED},
    state::{ChillNftMetadata, Config, AUTHORITY_SHARE},
};
use mpl_token_metadata::state::Creator;
use solana_program::{
    account_info::{next_account_info, next_account_infos, AccountInfo},
    entrypoint::ProgramResult,
    program::invoke_signed,
    program_pack::Pack,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};

pub fn initialize_chill_metadata<'info>(
    chill_metadata: &AccountInfo<'info>,
    user: &AccountInfo<'info>,
    system_program: &AccountInfo<'info>,
    nft_mint: &Pubkey,
    program_id: &Pubkey,
) -> ProgramResult {
    let rent = Rent::get()?;
    let lamports = rent.minimum_balance(ChillNftMetadata::LEN);
    let (chill_metadata_pubkey, bump) = pda::chill_metadata(nft_mint, program_id);
    let seeds = &[CHILL_METADATA_SEED.as_bytes(), nft_mint.as_ref(), &[bump]];

    let ix = system_instruction::create_account(
        user.key,
        &chill_metadata_pubkey,
        lamports,
        ChillNftMetadata::LEN.try_into().unwrap(),
        program_id,
    );

    invoke_signed(
        &ix,
        &[user.clone(), chill_metadata.clone(), system_program.clone()],
        &[seeds],
    )
}

pub fn process_mint_nft(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: MintNftArgs,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let authority = next_account_info(accounts_iter)?;
    let user = next_account_info(accounts_iter)?;
    let config = next_account_info(accounts_iter)?;
    let chill_mint = next_account_info(accounts_iter)?;
    let chill_token_account = next_account_info(accounts_iter)?;
    let nft_mint = next_account_info(accounts_iter)?;
    let nft_token_account = next_account_info(accounts_iter)?;
    let metadata = next_account_info(accounts_iter)?;
    let master_edition = next_account_info(accounts_iter)?;
    let chill_metadata = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;
    let rent_program = next_account_info(accounts_iter)?;
    let token_program = next_account_info(accounts_iter)?;
    let metadata_program = next_account_info(accounts_iter)?;

    assert::owner(config, program_id)?;
    assert::config_pubkey(config.key, chill_mint.key, program_id)?;
    assert::chill_metadata_pubkey(chill_metadata.key, nft_mint.key, program_id)?;

    let config = Config::unpack(&config.data.borrow())?;
    let recipients_token_accounts = next_account_infos(accounts_iter, config.recipients.len())?;

    assert::recipients(&config, recipients_token_accounts)?;
    assert::token_account(chill_token_account, user.key, chill_mint.key)?;

    utils::transfer_chill(
        user,
        chill_token_account,
        recipients_token_accounts,
        token_program,
        &config,
        args.nft_type,
    )?;

    let creators;
    if authority.key != user.key {
        creators = vec![
            Creator {
                address: *authority.key,
                verified: true,
                share: AUTHORITY_SHARE,
            },
            Creator {
                address: *user.key,
                verified: false,
                share: 100 - AUTHORITY_SHARE,
            },
        ];
    } else {
        creators = vec![Creator {
            address: *authority.key,
            verified: true,
            share: 100,
        }];
    }

    let token_builder = TokenBuilder {
        name: args.name,
        symbol: args.symbol,
        url: args.url,
        creators: Some(creators),
        seller_fee_basis_points: args.fees,
    };

    nft::metadata(
        authority,
        nft_mint,
        user,
        metadata,
        token_builder,
        system_program,
        rent_program,
        metadata_program,
    )?;

    nft::master_edition(
        authority,
        user,
        nft_mint,
        metadata,
        master_edition,
        system_program,
        rent_program,
        metadata_program,
    )?;

    initialize_chill_metadata(
        chill_metadata,
        user,
        system_program,
        nft_mint.key,
        program_id,
    )?;

    let chill_metadata_account = ChillNftMetadata::new(args.nft_type);
    ChillNftMetadata::pack(
        chill_metadata_account,
        &mut chill_metadata.data.borrow_mut(),
    )?;

    if authority.key != user.key {
        utils::sign_metadata(user, metadata, metadata_program)?;
        utils::set_primary_sell_happened(metadata, user, nft_token_account, metadata_program)?;
    }

    Ok(())
}
