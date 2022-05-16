use crate::{
    state::{StakingInfo, StakingTokenAuthority},
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

    #[account(mint::authority = primary_wallet)]
    pub chill_mint: Account<'info, Mint>,

    pub system_program: Program<'info, System>,

    pub rent: Sysvar<'info, Rent>,

    pub token_program: Program<'info, Token>,

    pub associated_token_program: Program<'info, AssociatedToken>,
}

#[derive(Accounts)]
pub struct AddRewardTokens<'info> {
    pub primary_wallet: Signer<'info>,

    pub token_authority: Signer<'info>,

    #[account(mut, token::authority = token_authority, token::mint = staking_info.mint)]
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
pub struct ViewStaking<'info> {
    pub staking_info: Account<'info, StakingInfo>,
}
