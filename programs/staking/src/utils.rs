use crate::{
    lazy_vector::{GetLazyVector, LazyVector},
    state::{StakingInfo, StakingTokenAuthority, UserInfo, DAYS_IN_WINDOW, SEC_PER_DAY},
};
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount};
use ethnum::U256;
use std::cmp;

pub fn transfer_tokens<'info>(
    amount: u64,
    staking_info: &Account<'info, StakingInfo>,
    staking_token_authority: &Account<'info, StakingTokenAuthority>,
    staking_token_account: &Account<'info, TokenAccount>,
    recipient_token_account: &Account<'info, TokenAccount>,
    token_program: &Program<'info, Token>,
) -> Result<()> {
    let staking_info_pubkey = staking_info.key();
    let signers = &[
        staking_info_pubkey.as_ref(),
        &[staking_token_authority.bump],
    ];
    let signers = &[signers.as_ref()];

    let cpi_context = CpiContext::new_with_signer(
        token_program.to_account_info(),
        token::Transfer {
            from: staking_token_account.to_account_info(),
            to: recipient_token_account.to_account_info(),
            authority: staking_token_authority.to_account_info(),
        },
        signers,
    );

    token::transfer(cpi_context, amount)
}

pub fn current_day() -> Result<u64> {
    let clock = Clock::get()?;
    let timestamp = clock.unix_timestamp as u64;
    Ok(timestamp.checked_div(SEC_PER_DAY).unwrap())
}

pub fn calculate_unspent_amount_from_days_with_no_reward(
    days_with_no_reward: u64,
    total_days: u64,
    reward_tokens_amount: u64,
) -> u64 {
    U256::from(reward_tokens_amount)
        .checked_mul(days_with_no_reward.into())
        .and_then(|v| v.checked_div(total_days.into()))
        .unwrap()
        .as_u64()
}

pub fn calculate_daily_staking_reward(
    day_index: u64,
    total_days: u64,
    unspent_amount: u64,
    total_rewarded_free_amount: u64,
    reward_tokens_amount: u64,
) -> (u64, u64) {
    let remaining_days = total_days.checked_sub(day_index).unwrap();
    let total_days = U256::from(total_days);

    let max_daily_reward_x_total_days = reward_tokens_amount;
    let max_rewarded_x_total_days = U256::from(max_daily_reward_x_total_days)
        .checked_mul(day_index.into())
        .unwrap();

    let denomenator = U256::from(remaining_days)
        .checked_mul(U256::new(2))
        .and_then(|v| v.checked_mul(total_days))
        .unwrap();

    let unspent_amount_x_total_days = U256::from(unspent_amount).checked_mul(total_days).unwrap();

    let total_rewarded_free_amount_x_total_days = U256::from(total_rewarded_free_amount)
        .checked_mul(total_days)
        .unwrap();

    let free_amount_x_total_days = unspent_amount_x_total_days
        .checked_sub(total_rewarded_free_amount_x_total_days)
        .unwrap();

    let reward_tokens_amount_x_total_days = U256::from(reward_tokens_amount)
        .checked_mul(total_days)
        .unwrap();

    let numerator = U256::from(reward_tokens_amount_x_total_days)
        .checked_add(free_amount_x_total_days)
        .and_then(|v| v.checked_sub(max_rewarded_x_total_days))
        .unwrap();

    let daily_reward = numerator.checked_div(denomenator).unwrap().as_u64();

    let remaining_days_x_total_days = U256::from(remaining_days).checked_mul(total_days).unwrap();
    let daily_unspent_reward = U256::from(free_amount_x_total_days)
        .checked_div(remaining_days_x_total_days)
        .unwrap()
        .as_u64();

    (daily_reward, daily_unspent_reward)
}

pub fn calculate_total_staked_amount_before_day(
    day_index: u64,
    staked_amounts: &LazyVector<u64>,
) -> Result<u64> {
    let mut total_staked = 0u64;

    let from_index = day_index
        .checked_sub(DAYS_IN_WINDOW)
        .and_then(|v| v.checked_add(1))
        .unwrap_or(0);

    for index in from_index..day_index {
        let stake_amount = staked_amounts.get(index as usize)?;
        total_staked = total_staked.checked_add(stake_amount).unwrap();
    }

    Ok(total_staked)
}

pub fn calculate_user_reward_with_unspent_rewards(
    user_staked_amount: u64,
    user_start_day_index: u64,
    user_boosted_days: &LazyVector<bool>,
    staked_amounts: &LazyVector<u64>,
    total_days: u64,
    daily_staking_reward: u64,
) -> Result<(u64, u64)> {
    let daily_staking_reward = U256::from(daily_staking_reward);

    let mut total_staked_at_day_index =
        calculate_total_staked_amount_before_day(user_start_day_index, staked_amounts)?;

    let last_stake_day = user_start_day_index.checked_add(DAYS_IN_WINDOW).unwrap();
    let to = cmp::min(total_days, last_stake_day);

    let mut reward = 0u64;
    let mut remainings = 0u64;
    for day_index in user_start_day_index..to {
        let staked_amount = staked_amounts.get(day_index as usize)?;
        total_staked_at_day_index = total_staked_at_day_index
            .checked_add(staked_amount)
            .unwrap();

        let mut increase = daily_staking_reward
            .checked_mul(user_staked_amount.into())
            .unwrap();

        let boosted_day_index = day_index.checked_sub(user_start_day_index).unwrap();
        let boost = user_boosted_days.get(boosted_day_index as usize)?;
        if boost {
            increase = increase.checked_mul(U256::new(2)).unwrap();
        }

        let increase = increase
            .checked_div(total_staked_at_day_index.into())
            .unwrap()
            .as_u64();

        if !boost {
            remainings = remainings.checked_add(increase).unwrap();
        }

        reward = reward.checked_add(increase).unwrap();

        let min_window_index_next_day = day_index
            .checked_add(1)
            .and_then(|v| v.checked_sub(DAYS_IN_WINDOW));

        if let Some(min_window_index_next_day) = min_window_index_next_day {
            let staked_amount = staked_amounts.get(min_window_index_next_day as usize)?;
            total_staked_at_day_index = total_staked_at_day_index
                .checked_sub(staked_amount)
                .unwrap();
        }
    }

    Ok((reward, remainings))
}

pub fn update_state_accounts(
    user_info: &mut Account<UserInfo>,
    staking_info: &mut Account<StakingInfo>,
) -> Result<()> {
    let user_has_ended_stake = user_info.has_ended_stake(staking_info.end_day)?;
    if !user_has_ended_stake {
        return Ok(());
    }

    let total_days = staking_info.total_days();
    let staking_start_day = staking_info.start_day;
    let staked_amounts = staking_info.get_vector()?;

    let user_start_day = user_info.start_day.unwrap();
    let user_staked_amount = user_info.staked_amount;
    let daily_staking_reward = user_info.daily_staking_reward;
    let user_start_day_index = user_start_day.checked_sub(staking_start_day).unwrap();
    let user_boosted_days = user_info.get_vector()?;

    let (reward, unspent_amount) = calculate_user_reward_with_unspent_rewards(
        user_staked_amount,
        user_start_day_index,
        &user_boosted_days,
        &staked_amounts,
        total_days,
        daily_staking_reward,
    )?;

    user_info.start_day = None;
    user_info.total_rewarded_amount = user_info.total_rewarded_amount.checked_add(reward).unwrap();
    user_info.rewarded_amount = user_info.rewarded_amount.checked_add(reward).unwrap();
    user_info.pending_amount = user_info
        .pending_amount
        .checked_add(user_info.staked_amount)
        .unwrap();

    user_info.staked_amount = 0;

    staking_info.active_stakes_number = staking_info.active_stakes_number.checked_sub(1).unwrap();
    staking_info.total_unspent_amount = staking_info
        .total_unspent_amount
        .checked_add(unspent_amount)
        .unwrap();

    staking_info.total_rewarded_amount = staking_info
        .total_rewarded_amount
        .checked_add(reward)
        .unwrap();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, rc::Rc};

    #[test]
    fn daily_staking_reward() {
        let start_day = 500;
        let end_day = start_day + 100;
        let total_days = end_day - start_day;

        let reward_tokens_amount = 100_000_000;

        let mut total_rewarded_free_amount = 0;
        for i in 0..total_days {
            let (daily_reward, rewarded_free_amount) = calculate_daily_staking_reward(
                i,
                total_days,
                0,
                total_rewarded_free_amount,
                reward_tokens_amount,
            );

            total_rewarded_free_amount += rewarded_free_amount;
            assert_eq!(daily_reward, 500_000);
        }

        // 1 day without stake
        // 100_000_000 / 99 / 2 = 505050
        let mut total_rewarded_free_amount = 0;
        for i in 1..total_days {
            let (daily_reward, rewarded_free_amount) = calculate_daily_staking_reward(
                i,
                total_days,
                1_000_000,
                total_rewarded_free_amount,
                reward_tokens_amount,
            );

            total_rewarded_free_amount += rewarded_free_amount;
            assert_eq!(50505, daily_reward / 10);
        }

        // 2 days without stake
        // 100_000_000 / 98 / 2 = 510204
        let mut total_rewarded_free_amount = 0;
        for i in 2..total_days {
            let (daily_reward, rewarded_free_amount) = calculate_daily_staking_reward(
                i,
                total_days,
                2_000_000,
                total_rewarded_free_amount,
                reward_tokens_amount,
            );

            total_rewarded_free_amount += rewarded_free_amount;
            assert_eq!(51020, daily_reward / 10);
        }

        // 10 days without stake
        // 100_000_000 / 90 / 2 = 555555
        let mut total_rewarded_free_amount = 0;
        for i in (10..total_days).step_by(2) {
            let (daily_reward, rewarded_free_amount) = calculate_daily_staking_reward(
                i,
                total_days,
                10_000_000,
                total_rewarded_free_amount,
                reward_tokens_amount,
            );

            total_rewarded_free_amount += 2 * rewarded_free_amount;
            assert_eq!(55555, daily_reward / 10);
        }

        // 1 day without boost
        // 99_500_000 / 99 / 2 = 502525
        let mut total_rewarded_free_amount = 0;
        for i in 1..total_days {
            let (daily_reward, rewarded_free_amount) = calculate_daily_staking_reward(
                i,
                total_days,
                500_000,
                total_rewarded_free_amount,
                reward_tokens_amount,
            );

            total_rewarded_free_amount += rewarded_free_amount;
            assert_eq!(50252, daily_reward / 10);
        }

        // 2 days without boost
        // 99_000_000 / 98 / 2 = 505102
        let mut total_rewarded_free_amount = 0;
        for i in 2..total_days {
            let (daily_reward, rewarded_free_amount) = calculate_daily_staking_reward(
                i,
                total_days,
                1_000_000,
                total_rewarded_free_amount,
                reward_tokens_amount,
            );

            total_rewarded_free_amount += rewarded_free_amount;
            assert_eq!(50510, daily_reward / 10);
        }

        // 1 day without stake
        // 1 days without boost
        // 99_500_000 / 98 / 2 = 507653
        let mut total_rewarded_free_amount = 0;
        for i in 2..total_days {
            let (daily_reward, rewarded_free_amount) = calculate_daily_staking_reward(
                i,
                total_days,
                1_500_000,
                total_rewarded_free_amount,
                reward_tokens_amount,
            );

            total_rewarded_free_amount += rewarded_free_amount;
            assert_eq!(50765, daily_reward / 10);
        }
    }

    #[test]
    fn total_staked_amount_before() {
        let mut staked_amounts_buffer = [0u8; 144];
        let staked_amounts_data = Rc::new(RefCell::new(staked_amounts_buffer.as_mut()));
        let mut staked_amounts = LazyVector::new(0, 18, 8, staked_amounts_data).unwrap();

        staked_amounts.set(0, &1000).unwrap();
        staked_amounts.set(2, &2000).unwrap();
        staked_amounts.set(4, &2500).unwrap();
        staked_amounts.set(7, &2000).unwrap();
        staked_amounts.set(9, &5000).unwrap();
        staked_amounts.set(10, &200).unwrap();

        // Day, Staked, Staked during last 6 days
        //  0   1000    0
        //  1   0       1000
        //  2   2000    1000
        //  3   0       3000
        //  4   2500    3000
        //  5   0       5500
        //  6   0       5500
        //  7   2000    5500 - 1000 = 4500
        //  8   0       6500 - 0 = 6500
        //  9   5000    6500 - 2000 = 4500
        // 10   200     9500 - 0 = 9500
        // 11   0       9700 - 2500 = 7200
        // 12   0       7200 - 0 = 7200
        // 13   0       7200 - 0 = 7200
        // 14   0       7200 - 2000 = 5200
        // 15   0       5200 - 0 = 5200
        // 16   0       5200 - 5000 = 200
        // 17   0       200 - 200 = 0
        // 18   0       0 - 0 = 0

        let expected_values = vec![
            0, 1000, 1000, 3000, 3000, 5500, 5500, 4500, 6500, 4500, 9500, 7200, 7200, 7200, 5200,
            5200, 200, 0, 0,
        ];

        for (index, expected_value) in expected_values.iter().enumerate() {
            let actual_value =
                calculate_total_staked_amount_before_day(index as u64, &staked_amounts).unwrap();

            assert_eq!(actual_value, *expected_value, "Index: {}", index);
        }
    }

    #[test]
    fn user_reward() {
        let total_days = 12;
        let daily_staking_reward = 100;

        let mut staked_amounts_buffer = [0u8; 96];
        let staked_amounts_data = Rc::new(RefCell::new(staked_amounts_buffer.as_mut()));
        let mut staked_amounts = LazyVector::new(0, 12, 8, staked_amounts_data).unwrap();

        // User 1 staked 500 tokens in day 0
        staked_amounts.set(0, &500).unwrap();

        // User 2 staked 1500 tokens in day 2
        // User 3 staked 500 tokens in day 2
        staked_amounts.set(2, &2000).unwrap();

        // User 4 staked 2500 tokens in day 4
        staked_amounts.set(4, &2500).unwrap();

        // User 5 staked 1000 tokens in day 8
        staked_amounts.set(8, &1000).unwrap();

        let mut boosted_days_buffer = [0u8; 7];
        let boosted_days_data = Rc::new(RefCell::new(boosted_days_buffer.as_mut()));
        let mut boosted_days = LazyVector::new(0, 7, 1, boosted_days_data).unwrap();

        boosted_days.set(0, &true).unwrap();
        boosted_days.set(1, &true).unwrap();
        boosted_days.set(3, &true).unwrap();
        boosted_days.set(4, &true).unwrap();
        boosted_days.set(6, &true).unwrap();

        // User 1
        // 0: 500 / (0 + 500) * 100 * 2 = 200
        // 1: 500 / 500 * 100 * 2 = 200
        // 2: 500 / (500 + 2000) * 100 = 20
        // 3: 500 / 2500 * 100 * 2 = 40
        // 4: 500 / (2500 + 2500) * 100 * 2 = 20
        // 5: 500 / 5000 * 100 = 10
        // 6: 500 / 5000 * 100 * 2 = 20
        // Total: 510

        // Remainings = 20 + 10 = 30

        let (reward, remainings) = calculate_user_reward_with_unspent_rewards(
            500,
            0,
            &boosted_days,
            &staked_amounts,
            total_days,
            daily_staking_reward,
        )
        .unwrap();

        assert_eq!(reward, 510);
        assert_eq!(remainings, 30);

        boosted_days.clear();

        // User 2
        // 2: 1500 / 2500 * 100 = 60
        // 3: 1500 / 2500 * 100 = 60
        // 4: 1500 / 5000 * 100 = 30
        // 5: 1500 / 5000 * 100 = 30
        // 6: 1500 / 5000 * 100 = 30
        // 7: 1500 / 4500 * 100 = 33
        // 8: 1500 / 5500 * 100 = 27
        // Total: 270

        let (reward, remainings) = calculate_user_reward_with_unspent_rewards(
            1500,
            2,
            &boosted_days,
            &staked_amounts,
            total_days,
            daily_staking_reward,
        )
        .unwrap();

        assert_eq!(reward, 270);
        assert_eq!(remainings, 270);

        // User 3
        // 2: 500 / 2500 * 100 = 20
        // 3: 500 / 2500 * 100 = 20
        // 4: 500 / 5000 * 100 = 10
        // 5: 500 / 5000 * 100 = 10
        // 6: 500 / 5000 * 100 = 10
        // 7: 500 / 4500 * 100 = 11
        // 8: 500 / 5500 * 100 = 9
        // Total: 90

        let (reward, remainings) = calculate_user_reward_with_unspent_rewards(
            500,
            2,
            &boosted_days,
            &staked_amounts,
            total_days,
            daily_staking_reward,
        )
        .unwrap();

        assert_eq!(reward, 90);
        assert_eq!(remainings, 90);

        // User 4
        // 4: 2500 / 5000 * 100 = 50
        // 5: 2500 / 5000 * 100 = 50
        // 6: 2500 / 5000 * 100 = 50
        // 7: 2500 / 4500 * 100 = 55
        // 8: 2500 / 5500 * 100 = 45
        // 9: 2500 / 3500 * 100 = 71
        // 10: 2500 / 3500 * 100 = 71
        // Total: 392

        let (reward, remainings) = calculate_user_reward_with_unspent_rewards(
            2500,
            4,
            &boosted_days,
            &staked_amounts,
            total_days,
            daily_staking_reward,
        )
        .unwrap();

        assert_eq!(reward, 392);
        assert_eq!(remainings, 392);

        // User 5
        // 8: 1000 / 5500 * 100 = 18
        // 9: 1000 / 3500 * 100 = 28
        // 10: 1000 / 3500 * 100 = 28
        // 11: 1000 / 1000 * 100 = 100
        // Total: 174

        let (reward, remainings) = calculate_user_reward_with_unspent_rewards(
            1000,
            8,
            &boosted_days,
            &staked_amounts,
            total_days,
            daily_staking_reward,
        )
        .unwrap();

        assert_eq!(reward, 174);
        assert_eq!(remainings, 174);

        // Reward after staking period
        let (reward, remainings) = calculate_user_reward_with_unspent_rewards(
            1000,
            12,
            &boosted_days,
            &staked_amounts,
            total_days,
            daily_staking_reward,
        )
        .unwrap();
        assert_eq!(reward, 0);
        assert_eq!(remainings, 0);
    }
}
