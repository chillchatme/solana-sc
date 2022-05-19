use crate::{
    context::*,
    error::ErrorCode,
    lazy_vector::GetLazyVector,
    state::{DAYS_IN_WINDOW, SEC_PER_DAY},
};
use anchor_lang::prelude::*;
use anchor_spl::token;

pub mod context;
pub mod error;
pub mod event;
pub mod lazy_vector;
pub mod state;
pub mod utils;

declare_id!("7EbJfNdsRx1VgHbQgFCZsZZJBm2eDQC3PkKxTSjiabHm");

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct InitializeArgs {
    pub start_time: u64,
    pub end_time: u64,
}

impl InitializeArgs {
    pub fn start_day(&self) -> u64 {
        self.start_time.checked_div(SEC_PER_DAY).unwrap()
    }

    pub fn end_day(&self) -> u64 {
        self.end_time.checked_div(SEC_PER_DAY).unwrap()
    }

    pub fn days_amount(&self) -> usize {
        self.end_day().checked_sub(self.start_day()).unwrap() as usize
    }
}

#[program]
pub mod chill_staking {

    use super::*;

    // Views

    pub fn view_staking_amount_in_day(ctx: Context<ViewStaking>, index: u64) -> Result<u64> {
        let staking_info = &ctx.accounts.staking_info;
        let staking_amounts = staking_info.get_vector()?;
        staking_amounts.get(index as usize)
    }

    pub fn view_daily_staking_reward(ctx: Context<ViewStaking>) -> Result<u64> {
        let staking_info = &ctx.accounts.staking_info;
        staking_info.daily_staking_reward()
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

        staking_info.primary_wallet = ctx.accounts.primary_wallet.key();
        staking_info.mint = ctx.accounts.chill_mint.key();
        staking_info.start_day = args.start_day();
        staking_info.end_day = args.end_day();

        let bump = ctx.bumps["staking_token_authority"];
        let staking_token_authority = &mut ctx.accounts.staking_token_authority;
        staking_token_authority.bump = bump;

        Ok(())
    }

    pub fn add_reward_tokens(ctx: Context<AddRewardTokens>, amount: u64) -> Result<()> {
        let staking_info = &mut ctx.accounts.staking_info;
        staking_info.assert_not_finished()?;

        staking_info.reward_tokens_amount = staking_info
            .reward_tokens_amount
            .checked_add(amount)
            .unwrap();

        let cpi_context = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.token_account.to_account_info(),
                to: ctx.accounts.staking_token_account.to_account_info(),
                authority: ctx.accounts.token_authority.to_account_info(),
            },
        );

        token::transfer(cpi_context, amount)?;
        emit!(event::AddRewardTokens { amount });

        Ok(())
    }

    pub fn redeem_remaining_reward_tokens(ctx: Context<RedeemRemainingRewardTokens>) -> Result<()> {
        let stake_info = &mut ctx.accounts.staking_info;
        let staking_token_authority = &ctx.accounts.staking_token_authority;
        let staking_token_account = &ctx.accounts.staking_token_account;

        stake_info.assert_finished()?;

        let stake_info_pubkey = stake_info.key();
        let seeds = &[stake_info_pubkey.as_ref(), &[staking_token_authority.bump]];
        let seeds = &[seeds.as_ref()];

        let cpi_context = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: staking_token_account.to_account_info(),
                to: ctx.accounts.recipient_token_account.to_account_info(),
                authority: staking_token_authority.to_account_info(),
            },
            seeds,
        );

        token::transfer(cpi_context, staking_token_account.amount)?;
        stake_info.reward_tokens_amount = 0;

        Ok(())
    }

    pub fn stake(ctx: Context<Stake>, amount: u64) -> Result<()> {
        let user_info = &mut ctx.accounts.user_info;
        let staking_info = &mut ctx.accounts.staking_info;

        staking_info.assert_not_finished()?;

        utils::update_user_reward(user_info, staking_info)?;

        let bump = ctx.bumps["user_info"];
        user_info.user = ctx.accounts.user.key();
        user_info.staking_info = staking_info.key();
        user_info.bump = bump;

        let cpi_context = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.from_token_account.to_account_info(),
                to: ctx.accounts.staking_token_account.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        );

        token::transfer(cpi_context, amount)?;
        emit!(event::Stake {
            user: ctx.accounts.user.key(),
            amount
        });

        if user_info.has_active_stake() {
            require_neq!(amount, 0, ErrorCode::AddZeroTokensToPendingAmount);
            user_info.pending_amount = user_info.pending_amount.checked_add(amount).unwrap();
            return Ok(());
        }

        let increase = user_info.pending_amount.checked_add(amount).unwrap();
        user_info.pending_amount = 0;

        user_info.staked_amount = user_info.staked_amount.checked_add(increase).unwrap();
        require_neq!(user_info.staked_amount, 0, ErrorCode::StakeZeroTokens);

        user_info.start_day = Some(utils::current_day()?);
        user_info.total_staked_amount = user_info
            .total_staked_amount
            .checked_add(user_info.staked_amount)
            .unwrap();

        let mut user_boosted_days = user_info.get_vector()?;
        user_boosted_days.clear();

        let mut staking_amounts = staking_info.get_vector()?;
        let day_index = staking_info.day_index()? as usize;
        let previous_amount = staking_amounts.get(day_index)?;
        let new_amount = previous_amount
            .checked_add(user_info.staked_amount)
            .unwrap();

        staking_amounts.set(day_index, &new_amount)?;

        if previous_amount == 0 {
            staking_info.days_with_new_stake =
                staking_info.days_with_new_stake.checked_add(1).unwrap();
        }

        staking_info.total_stakes_number = staking_info.total_stakes_number.checked_add(1).unwrap();
        staking_info.total_staked_amount = staking_info
            .total_staked_amount
            .checked_add(user_info.staked_amount)
            .unwrap();

        Ok(())
    }

    pub fn claim(ctx: Context<Claim>, amount: u64) -> Result<()> {
        require_neq!(amount, 0, ErrorCode::WithdrawZeroTokens);

        let user_info = &mut ctx.accounts.user_info;
        let staking_info = &mut ctx.accounts.staking_info;
        let staking_token_authority = &ctx.accounts.staking_token_authority;

        utils::update_user_reward(user_info, staking_info)?;

        let staking_info_pubkey = staking_info.key();
        let signers = &[
            staking_info_pubkey.as_ref(),
            &[staking_token_authority.bump],
        ];
        let signers = &[signers.as_ref()];

        let cpi_context = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.staking_token_account.to_account_info(),
                to: ctx.accounts.recipient_token_account.to_account_info(),
                authority: staking_token_authority.to_account_info(),
            },
            signers,
        );

        token::transfer(cpi_context, amount)?;

        user_info.rewarded_amount = user_info.rewarded_amount.checked_sub(amount).unwrap();
        user_info.total_rewarded_amount =
            user_info.total_rewarded_amount.checked_add(amount).unwrap();

        staking_info.total_rewarded_amount = staking_info
            .total_rewarded_amount
            .checked_add(amount)
            .unwrap();

        emit!(event::Claim {
            user: ctx.accounts.user.key(),
            amount
        });

        Ok(())
    }

    pub fn boost(ctx: Context<Boost>) -> Result<()> {
        let user_info = &mut ctx.accounts.user_info;
        let staking_info = &mut ctx.accounts.staking_info;

        utils::update_user_reward(user_info, staking_info)?;

        require!(user_info.has_active_stake(), ErrorCode::NoActiveStake);

        let mut boosted_days = user_info.get_vector()?;
        let current_day = utils::current_day()?;
        let index = current_day
            .checked_sub(user_info.start_day.unwrap())
            .unwrap() as usize;

        require_eq!(boosted_days.get(index)?, false, ErrorCode::AlreadyBoosted);
        boosted_days.set(index, &true)?;

        user_info.total_boost_amount = user_info.total_boost_amount.checked_add(1).unwrap();
        staking_info.total_boost_amount = staking_info.total_boost_amount.checked_add(1).unwrap();

        emit!(event::Boost {
            user: ctx.accounts.user.key()
        });

        Ok(())
    }
}
