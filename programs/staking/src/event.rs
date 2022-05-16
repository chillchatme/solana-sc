use anchor_lang::prelude::*;

#[event]
pub struct AddRewardTokens {
    pub amount: u64,
}
