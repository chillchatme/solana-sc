use anchor_lang::prelude::*;

#[event]
pub struct CreateWallet {
    pub user: Pubkey,
}

#[event]
pub struct WithdrawLamports {
    pub authority: Pubkey,
    pub amount: u64,
}

#[event]
pub struct WithdrawFt {
    pub authority: Pubkey,
    pub amount: u64,
}

#[event]
pub struct WithdrawNft {
    pub authority: Pubkey,
}
