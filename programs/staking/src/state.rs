use crate::{
    lazy_vector::{GetLazyVector, LazyVector},
    utils, StakingErrorCode,
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

    pub reward_tokens_amount: u64,

    // Daily reward
    pub last_daily_reward: u64,
    pub last_update_day: u64,
    pub days_with_new_stake: u64,
    pub unspent_boosted_rewards: u64,
    pub rewarded_free_amount: u64,

    // Statistics
    pub total_stakes_number: u64,
    pub total_boost_amount: u64,
    pub total_staked_amount: u64,
    pub total_rewarded_amount: u64,
}

impl StakingInfo {
    pub const LEN: usize =
        DESCRIMINATOR_LEN + 32 + 32 + 8 + 8 + 8 + 8 + 8 + 8 + 8 + 8 + 8 + 8 + 8 + 8;

    pub fn assert_not_finished(&self) -> Result<()> {
        let current_day = utils::current_day()?;
        require_gt!(
            self.end_day,
            current_day,
            StakingErrorCode::StakingIsFinished
        );

        Ok(())
    }

    pub fn assert_finished(&self) -> Result<()> {
        let current_day = utils::current_day()?;
        require_gte!(
            current_day,
            self.end_day,
            StakingErrorCode::StakingIsNotFinished
        );

        Ok(())
    }

    pub fn daily_staking_reward(&mut self) -> Result<u64> {
        let current_day = utils::current_day()?;

        if self.last_update_day != current_day {
            let day_index = self.day_index()?;
            let total_days = self.total_days();

            self.last_update_day = current_day;

            let (new_daily_reward, free_amount) = utils::calculate_daily_staking_reward(
                day_index,
                self.days_with_new_stake,
                total_days,
                self.unspent_boosted_rewards,
                self.rewarded_free_amount,
                self.reward_tokens_amount,
            );

            self.rewarded_free_amount = self.rewarded_free_amount.checked_add(free_amount).unwrap();
            self.last_daily_reward = new_daily_reward;
        }

        Ok(self.last_daily_reward)
    }

    pub fn day_index(&self) -> Result<u64> {
        let current_day = utils::current_day()?;
        current_day
            .checked_sub(self.start_day)
            .ok_or_else(|| StakingErrorCode::StakingIsNotStarted.into())
    }

    pub fn total_days(&self) -> u64 {
        self.end_day.checked_sub(self.start_day).unwrap()
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
    pub user: Pubkey,
    pub staking_info: Pubkey,
    pub bump: u8,

    pub start_day: Option<u64>,
    pub staked_amount: u64,
    pub pending_amount: u64,
    pub rewarded_amount: u64,

    // Statistics
    pub total_staked_amount: u64,
    pub total_rewarded_amount: u64,
    pub total_boost_amount: u64,
}

impl UserInfo {
    pub const LEN: usize = DESCRIMINATOR_LEN + 32 + 32 + 1 + 1 + 8 + 8 + 8 + 8 + 8 + 8 + 8;

    pub fn has_active_stake(&self) -> bool {
        self.start_day.is_some()
    }

    pub fn has_ended_stake(&self) -> Result<bool> {
        let current_day = utils::current_day()?;
        self.start_day.map_or(Ok(false), |start_day| {
            Ok(current_day.checked_sub(start_day).unwrap() >= DAYS_IN_WINDOW)
        })
    }
}

impl<'info> GetLazyVector<'info, bool> for Account<'info, UserInfo> {
    fn get_vector(&self) -> Result<LazyVector<'info, bool>> {
        let account_info = self.to_account_info();
        LazyVector::new(
            UserInfo::LEN,
            DAYS_IN_WINDOW.try_into().unwrap(),
            std::mem::size_of::<bool>(),
            account_info.data,
        )
    }
}
