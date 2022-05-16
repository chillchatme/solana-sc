use crate::{
    error::ErrorCode,
    lazy_vector::{GetLazyVector, LazyVector},
};
use anchor_lang::prelude::*;

pub const DESCRIMINATOR_LEN: usize = 8;
pub const VECTOR_SIZE_LEN: usize = 4;
pub const DAYS_IN_WINDOW: u64 = 7;

#[cfg(not(feature = "short-day"))]
pub const SEC_PER_DAY: u64 = 86400;

#[cfg(feature = "short-day")]
pub const SEC_PER_DAY: u64 = 2;

#[account]
pub struct StakingTokenAuthority {
    pub bump: u8,
}

impl StakingTokenAuthority {
    pub const LEN: usize = DESCRIMINATOR_LEN + 1;
}

#[account]
pub struct StakingInfo {
    pub primary_wallet: Pubkey,
    pub mint: Pubkey,

    // Staking interval = [start_day; end_day)
    pub start_day: u64,
    pub end_day: u64,

    pub last_update_time: i64,
    pub reward_tokens_amount: u64,

    // Statistics
    pub total_rewarded_amount: u64,
    pub total_staked_amount: u64,
}

impl StakingInfo {
    pub const LEN: usize = DESCRIMINATOR_LEN + 32 + 32 + 8 + 8 + 8 + 8 + 8 + 8;

    pub fn assert_not_finished(&self) -> Result<()> {
        let current_day = self.current_day()?;
        require_gt!(self.end_day, current_day, ErrorCode::StakingIsFinished);

        Ok(())
    }

    pub fn assert_finished(&self) -> Result<()> {
        let current_day = self.current_day()?;
        require_gte!(current_day, self.end_day, ErrorCode::StakingIsNotFinished);

        Ok(())
    }

    pub fn update(&mut self) -> Result<()> {
        let clock = Clock::get()?;
        self.last_update_time = clock.unix_timestamp;

        Ok(())
    }

    pub fn day_index(&self) -> Result<u64> {
        let current_day = self.current_day()?;
        Ok(current_day.checked_sub(self.start_day).unwrap())
    }

    pub fn current_day(&self) -> Result<u64> {
        let clock = Clock::get()?;
        let timestamp = clock.unix_timestamp as u64;
        Ok(timestamp.checked_div(SEC_PER_DAY).unwrap())
    }
}

impl<'info> GetLazyVector<'info, u64> for Account<'info, StakingInfo> {
    fn get_vector(&self) -> Result<LazyVector<'info, u64>> {
        let account_info = self.to_account_info();
        let days_amount = self.end_day.checked_sub(self.start_day).unwrap();

        LazyVector::new(
            StakingInfo::LEN,
            days_amount.try_into().unwrap(),
            std::mem::size_of::<u64>(),
            account_info.data,
        )
    }
}

#[account]
pub struct UserInfo {
    pub pubkey: Pubkey,
    pub staked_amount: u64,
    pub pending_amount: u64,

    // Statistics
    pub total_staked_amount: u64,
    pub total_rewarded_amount: u64,
}
