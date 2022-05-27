use crate::{
    lazy_vector::{GetLazyVector, LazyVector},
    utils, StakingErrorCode,
};
use anchor_lang::prelude::*;
use std::cmp;

pub const DESCRIMINATOR_LEN: usize = 8;
pub const VECTOR_SIZE_LEN: usize = 4;
pub const DAYS_IN_WINDOW: u64 = 7;

#[cfg(not(feature = "short-day"))]
pub const SEC_PER_DAY: u64 = 86400;

#[cfg(feature = "short-day")]
pub const SEC_PER_DAY: u64 = 3;

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
    pub active_stakes_number: u64,

    // Daily reward
    pub last_daily_reward: u64,
    pub last_update_day: u64,
    pub daily_unspent_reward: u64,
    pub rewarded_unspent_amount: u64,
    pub total_unspent_amount: u64,

    // Statistics
    pub total_boost_number: u64,
    pub total_stakes_number: u64,
    pub total_cancel_number: u64,

    pub total_days_with_no_reward: u64,
    pub total_staked_amount: u64,
    pub total_rewarded_amount: u64,
}

impl StakingInfo {
    pub const LEN: usize = DESCRIMINATOR_LEN + 32 * 2 + 8 * 16;

    pub fn assert_active(&self) -> Result<()> {
        let current_day = utils::current_day()?;

        require_gt!(
            self.end_day,
            current_day,
            StakingErrorCode::StakingIsFinished
        );

        require_gte!(
            current_day,
            self.start_day,
            StakingErrorCode::StakingIsNotStarted,
        );

        Ok(())
    }

    pub fn assert_not_started(&self) -> Result<()> {
        let current_day = utils::current_day()?;

        require_gt!(
            self.end_day,
            current_day,
            StakingErrorCode::StakingIsFinished
        );

        require_gt!(
            self.start_day,
            current_day,
            StakingErrorCode::StakingIsAlreadyStarted
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

    pub fn update_daily_reward(&mut self) -> Result<()> {
        let current_day = utils::current_day()?;
        if self.last_update_day == current_day || current_day >= self.end_day {
            return Ok(());
        }

        let days_with_no_reward;
        if self.last_update_day == 0 {
            days_with_no_reward = current_day.checked_sub(self.start_day).unwrap();
        } else {
            days_with_no_reward = current_day
                .checked_sub(self.last_update_day)
                .and_then(|v| v.checked_sub(DAYS_IN_WINDOW))
                .unwrap_or(0);
        }

        let total_days = self.total_days();
        let unspent_amount = utils::calculate_unspent_amount_from_days_with_no_reward(
            days_with_no_reward,
            total_days,
            self.reward_tokens_amount,
        );

        self.total_days_with_no_reward = self
            .total_days_with_no_reward
            .checked_add(days_with_no_reward)
            .unwrap();

        self.total_unspent_amount = self
            .total_unspent_amount
            .checked_add(unspent_amount)
            .unwrap();

        let day_index = self.day_index()?;
        let days_without_update = current_day.checked_sub(self.last_update_day).unwrap();

        self.rewarded_unspent_amount = self
            .daily_unspent_reward
            .checked_mul(cmp::min(days_without_update, DAYS_IN_WINDOW))
            .and_then(|v| v.checked_add(self.rewarded_unspent_amount))
            .unwrap();

        let (new_daily_reward, daily_unspent_reward) = utils::calculate_daily_staking_reward(
            day_index,
            total_days,
            self.total_unspent_amount,
            self.rewarded_unspent_amount,
            self.reward_tokens_amount,
        );

        self.daily_unspent_reward = daily_unspent_reward;
        self.last_daily_reward = new_daily_reward;
        self.last_update_day = current_day;

        Ok(())
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

    pub fn is_finished(&self) -> Result<bool> {
        let current_day = utils::current_day()?;
        Ok(self.end_day >= current_day)
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
    pub daily_staking_reward: u64,

    // Statistics
    pub total_staked_amount: u64,
    pub total_rewarded_amount: u64,
    pub total_boost_number: u64,
}

impl UserInfo {
    pub const LEN: usize = DESCRIMINATOR_LEN + 32 + 32 + 1 + 1 + 8 + 8 + 8 + 8 + 8 + 8 + 8 + 8;

    pub fn has_active_stake(&self) -> bool {
        self.start_day.is_some()
    }

    pub fn has_ended_stake(&self, staking_end_day: u64) -> Result<bool> {
        self.start_day.map_or(Ok(false), |start_day| {
            let current_day = utils::current_day()?;
            let user_staking_end = cmp::min(
                start_day.checked_add(DAYS_IN_WINDOW).unwrap(),
                staking_end_day,
            );

            Ok(current_day >= user_staking_end)
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
