use mpl_token_metadata::state::{EDITION, PREFIX};
use solana_program::pubkey::Pubkey;

pub const CONFIG_SEED: &str = "config";
pub const CHILL_METADATA_SEED: &str = "chill-metadata";

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
