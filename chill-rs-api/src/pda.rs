use solana_program::pubkey::Pubkey;

pub const CONFIG_SEED: &str = "config";

pub fn config(mint: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    let seeds = &[CONFIG_SEED.as_bytes(), mint.as_ref()];
    Pubkey::find_program_address(seeds, program_id)
}
