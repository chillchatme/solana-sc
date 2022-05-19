use anchor_lang::prelude::*;

#[event]
pub struct AddRewardTokens {
    pub amount: u64,
}

#[event]
pub struct Stake {
    pub user: Pubkey,
    pub amount: u64,
}

#[event]
pub struct Claim {
    pub user: Pubkey,
    pub amount: u64,
}

#[event]
pub struct Boost {
    pub user: Pubkey,
}
