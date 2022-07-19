use anchor_client::solana_sdk::pubkey::Pubkey;
use chill_nft::state::{ChillNftMetadata, Config};
use chill_wallet::state::ProxyWallet;
use mpl_token_metadata::state::{EDITION, PREFIX};

pub fn staking_token_authority(staking_info: Pubkey, program_id: Pubkey) -> Pubkey {
    let seeds = &[staking_info.as_ref()];
    Pubkey::find_program_address(seeds, &program_id).0
}

pub fn config(mint: Pubkey, program_id: Pubkey) -> Pubkey {
    let seeds = &[Config::SEED, mint.as_ref()];
    Pubkey::find_program_address(seeds, &program_id).0
}

pub fn chill_metadata(mint: Pubkey, program_id: Pubkey) -> Pubkey {
    let seeds = &[ChillNftMetadata::SEED, mint.as_ref()];
    Pubkey::find_program_address(seeds, &program_id).0
}

pub fn proxy_wallet(user: Pubkey, primary_wallet: Pubkey, program_id: Pubkey) -> Pubkey {
    let seeds = &[ProxyWallet::SEED, user.as_ref(), primary_wallet.as_ref()];
    Pubkey::find_program_address(seeds, &program_id).0
}

pub fn metadata(mint: Pubkey) -> Pubkey {
    let seeds = &[
        PREFIX.as_bytes(),
        mpl_token_metadata::ID.as_ref(),
        mint.as_ref(),
    ];

    Pubkey::find_program_address(seeds, &mpl_token_metadata::ID).0
}

pub fn master_edition(mint: Pubkey) -> Pubkey {
    let seeds = &[
        PREFIX.as_bytes(),
        mpl_token_metadata::ID.as_ref(),
        mint.as_ref(),
        EDITION.as_bytes(),
    ];

    Pubkey::find_program_address(seeds, &mpl_token_metadata::ID).0
}
