use crate::{
    lazy_vector::{GetLazyVector, LazyVector},
    state::{StakingInfo, UserInfo, DAYS_IN_WINDOW, SEC_PER_DAY},
};
use anchor_lang::prelude::*;
use ethnum::U256;
use std::cmp;

pub fn current_day() -> Result<u64> {
    let clock = Clock::get()?;
    let timestamp = clock.unix_timestamp as u64;
    Ok(timestamp.checked_div(SEC_PER_DAY).unwrap())
}

pub fn calculate_daily_staking_reward(
    current_day: u64,
    end_day: u64,
    remaining_reward_tokens: u64,
) -> u64 {
    let calculate_from = current_day.saturating_sub(DAYS_IN_WINDOW);
    let remaining_days = end_day.checked_sub(calculate_from).unwrap();
    remaining_reward_tokens
        .checked_div(remaining_days.checked_mul(2).unwrap())
        .unwrap()
}

pub fn calculate_total_staking_amount_before(
    day_index: u64,
    staking_amounts: &LazyVector<u64>,
) -> Result<u64> {
    let mut total_staked = 0u64;

    let from_index = day_index
        .checked_sub(DAYS_IN_WINDOW)
        .and_then(|v| v.checked_add(1))
        .unwrap_or(0);

    for index in from_index..day_index {
        let stake_amount = staking_amounts.get(index as usize)?;
        total_staked = total_staked.checked_add(stake_amount).unwrap();
    }

    Ok(total_staked)
}

pub fn calculate_user_reward(
    user_staked_amount: u64,
    user_start_day_index: u64,
    user_boosted_days: &LazyVector<bool>,
    staking_amounts: &LazyVector<u64>,
    total_days: u64,
    daily_staking_reward: u64,
) -> Result<u64> {
    let daily_staking_reward = U256::from(daily_staking_reward);

    let mut total_staked_at_day_index =
        calculate_total_staking_amount_before(user_start_day_index, staking_amounts)?;

    let last_stake_day = user_start_day_index.checked_add(DAYS_IN_WINDOW).unwrap();
    let to = cmp::min(total_days, last_stake_day);

    let mut rewards = 0u64;
    for day_index in user_start_day_index..to {
        let staked_amount = staking_amounts.get(day_index as usize)?;
        total_staked_at_day_index = total_staked_at_day_index
            .checked_add(staked_amount)
            .unwrap();

        let mut increase = daily_staking_reward
            .checked_mul(user_staked_amount.into())
            .and_then(|v| v.checked_div(total_staked_at_day_index.into()))
            .unwrap()
            .as_u64();

        let boosted_day_index = day_index.checked_sub(user_start_day_index).unwrap();
        let boost = user_boosted_days.get(boosted_day_index as usize)?;
        if boost {
            increase = increase.checked_mul(2).unwrap();
        }

        rewards = rewards.checked_add(increase).unwrap();

        let min_window_index_next_day = day_index
            .checked_add(1)
            .and_then(|v| v.checked_sub(DAYS_IN_WINDOW));

        if let Some(min_window_index_next_day) = min_window_index_next_day {
            let staked_amount = staking_amounts.get(min_window_index_next_day as usize)?;
            total_staked_at_day_index = total_staked_at_day_index
                .checked_sub(staked_amount)
                .unwrap();
        }
    }

    Ok(rewards)
}

pub fn update_user_reward(
    user_info: &mut Account<UserInfo>,
    staking_info: &mut Account<StakingInfo>,
) -> Result<()> {
    if !user_info.has_ended_stake()? {
        return Ok(());
    }

    let staking_start_day = staking_info.start_day;
    let staking_amounts = staking_info.get_vector()?;
    let daily_staking_reward = staking_info.daily_staking_reward()?;
    let total_days = staking_info.total_days();

    let user_start_day = user_info.start_day.unwrap();
    let user_staked_amount = user_info.staked_amount;
    let user_start_day_index = user_start_day.checked_sub(staking_start_day).unwrap();
    let user_boosted_days = user_info.get_vector()?;

    let reward = calculate_user_reward(
        user_staked_amount,
        user_start_day_index,
        &user_boosted_days,
        &staking_amounts,
        total_days,
        daily_staking_reward,
    )?;

    user_info.rewarded_amount = user_info.rewarded_amount.checked_add(reward).unwrap();
    user_info.start_day = None;

    staking_info.total_rewarded_amount = staking_info
        .total_rewarded_amount
        .checked_add(reward)
        .unwrap();

    Ok(())
}
