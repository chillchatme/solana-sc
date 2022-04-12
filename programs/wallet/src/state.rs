use anchor_lang::prelude::*;

pub const DESCRIMINATOR_LEN: usize = 8;

#[account]
pub struct ProxyWallet {
    pub bump: u8,
    pub primary_wallet: Pubkey,
    pub user: Pubkey,
    pub total_money_withdrawn_user: u64,
    pub total_money_withdrawn_primary_wallet: u64,
    pub total_ft_withdrawn_user: u64,
    pub total_ft_withdrawn_primary_wallet: u64,
    pub total_nft_withdrawn_user: u64,
    pub total_nft_withdrawn_primary_wallet: u64,
}

impl ProxyWallet {
    pub const LEN: usize = DESCRIMINATOR_LEN + 1 + 32 + 32 + 8 + 8 + 8 + 8 + 8 + 8;

    pub const SEED: &'static [u8] = b"wallet";
}
