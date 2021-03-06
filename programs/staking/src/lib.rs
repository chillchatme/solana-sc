use crate::{
    context::*,
    lazy_vector::GetLazyVector,
    state::{DAYS_IN_WINDOW, SEC_PER_DAY},
};
use anchor_lang::prelude::*;
use anchor_spl::token;

pub mod context;
pub mod event;
pub mod lazy_vector;
pub mod state;
pub mod utils;

declare_id!("7yrE1GFJD9LGvLJCRJkXpfuF5vtaB14KjxyX2Jn4uYE3");

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct InitializeArgs {
    pub start_time: u64,
    pub end_time: u64,
    pub min_stake_size: u64,
}

impl InitializeArgs {
    pub fn start_day(&self) -> u64 {
        self.start_time.checked_div(SEC_PER_DAY).unwrap()
    }

    pub fn end_day(&self) -> u64 {
        self.end_time.checked_div(SEC_PER_DAY).unwrap()
    }

    pub fn total_days(&self) -> usize {
        self.end_day().checked_sub(self.start_day()).unwrap() as usize
    }
}

#[program]
pub mod chill_staking {

    use super::*;

    // Views

    pub fn view_user_reward_amount(ctx: Context<ViewUserRewardAmount>) -> Result<u64> {
        let user_info = &mut ctx.accounts.user_info;
        let staking_info = &mut ctx.accounts.staking_info;

        utils::update_state_accounts(user_info, staking_info)?;

        Ok(user_info.rewarded_amount)
    }

    pub fn view_current_day_number(_ctx: Context<ViewState>) -> Result<u64> {
        utils::current_day()
    }

    pub fn view_staked_amount_in_day(ctx: Context<ViewStaking>, index: u64) -> Result<u64> {
        let staking_info = &ctx.accounts.staking_info;
        let staked_amounts = staking_info.get_vector()?;
        staked_amounts.get(index as usize)
    }

    pub fn view_daily_staking_reward(ctx: Context<ViewStaking>) -> Result<u64> {
        let staking_info = &mut ctx.accounts.staking_info;
        staking_info.update_daily_reward()?;
        Ok(staking_info.last_daily_reward)
    }

    pub fn view_boosted_days_list(ctx: Context<ViewUser>) -> Result<Vec<bool>> {
        let user_info = &ctx.accounts.user_info;
        let boosted_days = user_info.get_vector()?;
        Ok((0..DAYS_IN_WINDOW)
            .map(|i| boosted_days.get(i as usize).unwrap())
            .collect())
    }

    // Methods

    pub fn initialize(ctx: Context<Initialize>, args: InitializeArgs) -> Result<()> {
        let staking_info = &mut ctx.accounts.staking_info;

        let current_day = utils::current_day()?;
        let start_day = args.start_day();
        let end_day = args.end_day();

        require_gt!(
            start_day,
            current_day,
            StakingErrorCode::StakingMustStartInFuture
        );

        require_gt!(
            end_day,
            start_day,
            StakingErrorCode::EndDayMustBeBiggerThanStartDay,
        );

        staking_info.primary_wallet = ctx.accounts.primary_wallet.key();
        staking_info.mint = ctx.accounts.mint.key();
        staking_info.min_stake_size = args.min_stake_size;
        staking_info.start_day = start_day;
        staking_info.end_day = end_day;

        staking_info.get_vector()?;

        let bump = ctx.bumps["staking_token_authority"];
        let staking_token_authority = &mut ctx.accounts.staking_token_authority;
        staking_token_authority.bump = bump;

        Ok(())
    }

    pub fn close_staking_info(ctx: Context<CloseStakingInfo>) -> Result<()> {
        let staking_info = &ctx.accounts.staking_info;
        staking_info.assert_finished()
    }

    pub fn close_user_info(ctx: Context<CloseUserInfo>) -> Result<()> {
        let user_info = &ctx.accounts.user_info;

        let remainings_tokens = user_info
            .staked_amount
            .checked_add(user_info.pending_amount)
            .and_then(|v| v.checked_add(user_info.rewarded_amount))
            .unwrap();

        require_eq!(
            remainings_tokens,
            0,
            StakingErrorCode::UserInfoHasTokensToWithdraw
        );

        Ok(())
    }

    pub fn add_reward_tokens(ctx: Context<AddRewardTokens>, amount: u64) -> Result<()> {
        let staking_info = &mut ctx.accounts.staking_info;
        staking_info.assert_not_started()?;

        staking_info.reward_tokens_amount = staking_info
            .reward_tokens_amount
            .checked_add(amount)
            .unwrap();

        let cpi_context = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.token_account.to_account_info(),
                to: ctx.accounts.staking_token_account.to_account_info(),
                authority: ctx.accounts.token_account_authority.to_account_info(),
            },
        );

        token::transfer(cpi_context, amount)?;
        emit!(event::AddRewardTokens { amount });

        Ok(())
    }

    pub fn redeem_remaining_reward_tokens(
        ctx: Context<RedeemRemainingRewardTokens>,
        amount: u64,
    ) -> Result<()> {
        let staking_info = &mut ctx.accounts.staking_info;

        staking_info.assert_finished()?;

        let free_amount = staking_info
            .reward_tokens_amount
            .checked_sub(staking_info.total_rewarded_amount)
            .unwrap();

        require_gte!(free_amount, amount, StakingErrorCode::InsufficientFunds);

        utils::transfer_tokens(
            amount,
            staking_info,
            &ctx.accounts.staking_token_authority,
            &ctx.accounts.staking_token_account,
            &ctx.accounts.recipient_token_account,
            &ctx.accounts.token_program,
        )?;

        staking_info.reward_tokens_amount = staking_info
            .reward_tokens_amount
            .checked_sub(amount)
            .unwrap();

        Ok(())
    }

    pub fn stake(ctx: Context<Stake>, amount: u64) -> Result<()> {
        let user_info = &mut ctx.accounts.user_info;
        let staking_info = &mut ctx.accounts.staking_info;

        staking_info.assert_active()?;

        utils::update_state_accounts(user_info, staking_info)?;

        let bump = ctx.bumps["user_info"];
        user_info.user = ctx.accounts.user.key();
        user_info.staking_info = staking_info.key();
        user_info.bump = bump;

        let cpi_context = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.from_token_account.to_account_info(),
                to: ctx.accounts.staking_token_account.to_account_info(),
                authority: ctx.accounts.token_account_authority.to_account_info(),
            },
        );

        token::transfer(cpi_context, amount)?;
        emit!(event::Stake {
            user: ctx.accounts.user.key(),
            amount
        });

        if user_info.has_active_stake() {
            require_neq!(amount, 0, StakingErrorCode::AddZeroTokensToPendingAmount);
            user_info.pending_amount = user_info.pending_amount.checked_add(amount).unwrap();
            return Ok(());
        }

        let increment = user_info.pending_amount.checked_add(amount).unwrap();
        user_info.staked_amount = user_info.staked_amount.checked_add(increment).unwrap();
        user_info.pending_amount = 0;

        require_neq!(
            user_info.staked_amount,
            0,
            StakingErrorCode::StakeZeroTokens
        );

        require_gte!(
            user_info.staked_amount,
            staking_info.min_stake_size,
            StakingErrorCode::SmallStakeSize
        );

        user_info.daily_staking_reward = staking_info.last_daily_reward;
        user_info.start_day = Some(utils::current_day()?);
        user_info.total_staked_amount = user_info
            .total_staked_amount
            .checked_add(user_info.staked_amount)
            .unwrap();

        let mut user_boosted_days = user_info.get_vector()?;
        user_boosted_days.clear();

        let mut staked_amounts = staking_info.get_vector()?;
        let day_index = staking_info.day_index()? as usize;
        let previous_amount = staked_amounts.get(day_index)?;
        let new_amount = previous_amount
            .checked_add(user_info.staked_amount)
            .unwrap();

        staked_amounts.set(day_index, &new_amount)?;

        staking_info.last_day_with_stake = utils::current_day()?;

        staking_info.active_stakes_number =
            staking_info.active_stakes_number.checked_add(1).unwrap();

        staking_info.total_stakes_number = staking_info.total_stakes_number.checked_add(1).unwrap();

        staking_info.total_staked_amount = staking_info
            .total_staked_amount
            .checked_add(user_info.staked_amount)
            .unwrap();

        Ok(())
    }

    pub fn cancel(ctx: Context<UserUpdatesUserInfo>) -> Result<()> {
        let user_info = &mut ctx.accounts.user_info;
        let staking_info = &mut ctx.accounts.staking_info;

        utils::update_state_accounts(user_info, staking_info)?;

        require!(
            user_info.has_active_stake(),
            StakingErrorCode::NoActiveStake
        );

        let boosted_days = user_info.get_vector()?;
        let boost_number = (0..DAYS_IN_WINDOW)
            .map(|day| boosted_days.get(day as usize).unwrap() as u64)
            .sum();

        let user_start_day_index = user_info
            .start_day
            .unwrap()
            .checked_sub(staking_info.start_day)
            .unwrap() as usize;

        let mut staked_amounts = staking_info.get_vector()?;
        let total_staked_that_day = staked_amounts.get(user_start_day_index)?;
        let new_staked_amount = total_staked_that_day
            .checked_sub(user_info.staked_amount)
            .unwrap();
        staked_amounts.set(user_start_day_index, &new_staked_amount)?;

        staking_info.total_stakes_number = staking_info.total_stakes_number.checked_sub(1).unwrap();

        staking_info.active_stakes_number =
            staking_info.active_stakes_number.checked_sub(1).unwrap();

        staking_info.total_staked_amount = staking_info
            .total_staked_amount
            .checked_sub(user_info.staked_amount)
            .unwrap();

        staking_info.total_boost_number = staking_info
            .total_boost_number
            .checked_sub(boost_number)
            .unwrap();

        staking_info.total_cancel_number = staking_info.total_cancel_number.checked_add(1).unwrap();

        user_info.start_day = None;

        user_info.total_boost_number = user_info
            .total_boost_number
            .checked_sub(boost_number)
            .unwrap();

        user_info.total_staked_amount = user_info
            .total_staked_amount
            .checked_sub(user_info.staked_amount)
            .unwrap();

        user_info.pending_amount = user_info
            .pending_amount
            .checked_add(user_info.staked_amount)
            .unwrap();

        user_info.staked_amount = 0;

        emit!(event::Cancel {
            user: ctx.accounts.user.key()
        });

        Ok(())
    }

    pub fn claim(ctx: Context<Claim>, amount: u64) -> Result<()> {
        require_neq!(amount, 0u64, StakingErrorCode::WithdrawZeroTokens);

        let user_info = &mut ctx.accounts.user_info;
        let staking_info = &mut ctx.accounts.staking_info;

        utils::update_state_accounts(user_info, staking_info)?;

        let total_amount = user_info
            .rewarded_amount
            .checked_add(user_info.pending_amount)
            .unwrap();

        require_gte!(total_amount, amount, StakingErrorCode::InsufficientFunds);

        if amount > user_info.rewarded_amount {
            user_info.pending_amount = u128::from(user_info.pending_amount)
                .checked_add(user_info.rewarded_amount.into())
                .and_then(|v| v.checked_sub(amount.into()))
                .unwrap()
                .try_into()
                .unwrap();

            user_info.rewarded_amount = 0;
        } else {
            user_info.rewarded_amount = user_info.rewarded_amount.checked_sub(amount).unwrap();
        }

        emit!(event::Claim {
            user: ctx.accounts.user.key(),
            amount
        });

        utils::transfer_tokens(
            amount,
            staking_info,
            &ctx.accounts.staking_token_authority,
            &ctx.accounts.staking_token_account,
            &ctx.accounts.recipient_token_account,
            &ctx.accounts.token_program,
        )
    }

    pub fn transfer_reward_to_pending_amount(
        ctx: Context<UserUpdatesUserInfo>,
        amount: u64,
    ) -> Result<()> {
        let user_info = &mut ctx.accounts.user_info;
        let staking_info = &mut ctx.accounts.staking_info;
        staking_info.assert_active()?;

        utils::update_state_accounts(user_info, staking_info)?;

        require_gte!(
            user_info.rewarded_amount,
            amount,
            StakingErrorCode::InsufficientFunds
        );

        user_info.rewarded_amount = user_info.rewarded_amount.checked_sub(amount).unwrap();
        user_info.pending_amount = user_info.pending_amount.checked_add(amount).unwrap();

        emit!(event::TransferRewardToPendingAmount {
            user: ctx.accounts.user.key(),
            amount
        });

        Ok(())
    }

    pub fn boost(ctx: Context<UserUpdatesUserInfo>) -> Result<()> {
        let user_info = &mut ctx.accounts.user_info;
        let staking_info = &mut ctx.accounts.staking_info;

        utils::update_state_accounts(user_info, staking_info)?;

        require!(
            user_info.has_active_stake(),
            StakingErrorCode::NoActiveStake
        );

        let mut boosted_days = user_info.get_vector()?;
        let current_day = utils::current_day()?;
        let index = current_day
            .checked_sub(user_info.start_day.unwrap())
            .unwrap() as usize;

        require_eq!(
            boosted_days.get(index)?,
            false,
            StakingErrorCode::AlreadyBoosted
        );
        boosted_days.set(index, &true)?;

        user_info.total_boost_number = user_info.total_boost_number.checked_add(1).unwrap();
        staking_info.total_boost_number = staking_info.total_boost_number.checked_add(1).unwrap();

        emit!(event::Boost {
            user: ctx.accounts.user.key()
        });

        Ok(())
    }
}

#[error_code]
pub enum StakingErrorCode {
    #[msg("Insufficient funds")]
    InsufficientFunds,

    #[msg("Use force=true to deactivate stake")]
    UseForceToDeactivateStake,

    #[msg("Staking must start in the future")]
    StakingMustStartInFuture,

    #[msg("Small stake size")]
    SmallStakeSize,

    #[msg("End day must be bigger then start day")]
    EndDayMustBeBiggerThanStartDay,

    #[msg("UserInfo has tokens to withdraw")]
    UserInfoHasTokensToWithdraw,

    #[msg("Wrong vector size")]
    WrongVectorSize,

    #[msg("Out of bounds")]
    OutOfBounds,

    #[msg("Max vector size has been reached")]
    MaxSizeReached,

    #[msg("Already boosted today")]
    AlreadyBoosted,

    #[msg("User doesn't have active stake")]
    NoActiveStake,

    #[msg("Staking is already started")]
    StakingIsAlreadyStarted,

    #[msg("Staking is finished")]
    StakingIsFinished,

    #[msg("Staking is not started yet")]
    StakingIsNotStarted,

    #[msg("Staking is not finished yet")]
    StakingIsNotFinished,

    #[msg("Adding zero tokens to pending amount")]
    AddZeroTokensToPendingAmount,

    #[msg("Stake zero tokens")]
    StakeZeroTokens,

    #[msg("Withdraw zero tokens")]
    WithdrawZeroTokens,
}
