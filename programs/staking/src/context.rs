use crate::{
    state::{StakingInfo, StakingTokenAuthority, UserInfo, DAYS_IN_WINDOW},
    InitializeArgs,
};
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount},
};

#[derive(Accounts)]
#[instruction(args: InitializeArgs)]
pub struct Initialize<'info> {
    pub primary_wallet: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(init, payer = payer, space = StakingInfo::LEN + args.days_amount() * 8)]
    pub staking_info: Account<'info, StakingInfo>,

    #[account(init, payer = payer, space = StakingTokenAuthority::LEN, seeds = [staking_info.key().as_ref()], bump)]
    pub staking_token_authority: Account<'info, StakingTokenAuthority>,

    #[account(init, payer = payer, associated_token::mint = chill_mint, associated_token::authority = staking_token_authority)]
    pub staking_token_account: Account<'info, TokenAccount>,

    pub chill_mint: Account<'info, Mint>,

    pub system_program: Program<'info, System>,

    pub rent: Sysvar<'info, Rent>,

    pub token_program: Program<'info, Token>,

    pub associated_token_program: Program<'info, AssociatedToken>,
}

#[derive(Accounts)]
pub struct CloseStakingInfo<'info> {
    pub primary_wallet: Signer<'info>,

    #[account(mut, has_one = primary_wallet, close = recipient)]
    pub staking_info: Account<'info, StakingInfo>,

    /// CHECK: recipient
    #[account(mut)]
    pub recipient: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct CloseUserInfo<'info> {
    pub user: Signer<'info>,

    #[account(mut, has_one = user, close = recipient)]
    pub user_info: Account<'info, UserInfo>,

    /// CHECK: recipient
    #[account(mut)]
    pub recipient: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct AddRewardTokens<'info> {
    pub primary_wallet: Signer<'info>,

    pub token_account_authority: Signer<'info>,

    #[account(mut, token::authority = token_account_authority, token::mint = staking_info.mint)]
    pub token_account: Account<'info, TokenAccount>,

    #[account(mut, has_one = primary_wallet)]
    pub staking_info: Account<'info, StakingInfo>,

    #[account(seeds = [staking_info.key().as_ref()], bump = staking_token_authority.bump)]
    pub staking_token_authority: Account<'info, StakingTokenAuthority>,

    #[account(mut, associated_token::mint = staking_info.mint, associated_token::authority = staking_token_authority)]
    pub staking_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct RedeemRemainingRewardTokens<'info> {
    pub primary_wallet: Signer<'info>,

    #[account(mut, has_one = primary_wallet)]
    pub staking_info: Account<'info, StakingInfo>,

    #[account(seeds = [staking_info.key().as_ref()], bump = staking_token_authority.bump)]
    pub staking_token_authority: Account<'info, StakingTokenAuthority>,

    #[account(mut, associated_token::mint = staking_info.mint, associated_token::authority = staking_token_authority)]
    pub staking_token_account: Account<'info, TokenAccount>,

    #[account(mut, token::mint = staking_info.mint)]
    pub recipient_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct ViewState {}

#[derive(Accounts)]
pub struct ViewStaking<'info> {
    pub staking_info: Account<'info, StakingInfo>,
}

#[derive(Accounts)]
pub struct ViewUser<'info> {
    pub user_info: Account<'info, UserInfo>,
}

#[derive(Accounts)]
pub struct ViewUserRewardAmount<'info> {
    #[account(has_one = staking_info)]
    pub user_info: Account<'info, UserInfo>,

    pub staking_info: Account<'info, StakingInfo>,
}

#[derive(Accounts)]
pub struct Stake<'info> {
    pub user: Signer<'info>,

    pub token_account_authority: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(mut, token::authority = token_account_authority, token::mint = staking_info.mint)]
    pub from_token_account: Account<'info, TokenAccount>,

    #[account(init_if_needed, payer = payer, space = UserInfo::LEN + DAYS_IN_WINDOW as usize,
              seeds = [staking_info.key().as_ref(), user.key().as_ref()], bump)]
    pub user_info: Account<'info, UserInfo>,

    #[account(mut)]
    pub staking_info: Account<'info, StakingInfo>,

    #[account(seeds = [staking_info.key().as_ref()], bump = staking_token_authority.bump)]
    pub staking_token_authority: Account<'info, StakingTokenAuthority>,

    #[account(mut, associated_token::mint = staking_info.mint, associated_token::authority = staking_token_authority)]
    pub staking_token_account: Account<'info, TokenAccount>,

    pub system_program: Program<'info, System>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct Claim<'info> {
    pub user: Signer<'info>,

    #[account(mut, seeds = [staking_info.key().as_ref(), user.key().as_ref()], bump = user_info.bump)]
    pub user_info: Account<'info, UserInfo>,

    #[account(mut, token::mint = staking_info.mint)]
    pub recipient_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub staking_info: Account<'info, StakingInfo>,

    #[account(seeds = [staking_info.key().as_ref()], bump = staking_token_authority.bump)]
    pub staking_token_authority: Account<'info, StakingTokenAuthority>,

    #[account(mut, associated_token::mint = staking_info.mint, associated_token::authority = staking_token_authority)]
    pub staking_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct UserUpdatesUserInfo<'info> {
    pub user: Signer<'info>,

    #[account(mut, seeds = [staking_info.key().as_ref(), user.key().as_ref()], bump = user_info.bump)]
    pub user_info: Account<'info, UserInfo>,

    #[account(mut)]
    pub staking_info: Account<'info, StakingInfo>,
}
