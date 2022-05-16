use crate::{context::*, lazy_vector::GetLazyVector, state::SEC_PER_DAY};
use anchor_lang::prelude::*;
use anchor_spl::token;

pub mod context;
pub mod error;
pub mod event;
pub mod lazy_vector;
pub mod state;

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
        let staking = &ctx.accounts.staking_info;
        let staking_amounts = staking.get_vector()?;
        staking_amounts.get(index as usize)
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
}
