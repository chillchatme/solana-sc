use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};
use state::ProxyWallet;
use utils::{check_authority, transfer_tokens};

declare_id!("9HjUbHc9JmSwEa9vkATjJCoaAhJYbkcqXE64CafXDrPt");

pub mod state;
pub mod utils;

#[program]
pub mod chill_wallet {

    use super::*;

    pub fn create_wallet(ctx: Context<CreateWallet>) -> Result<()> {
        let bump = ctx.bumps["proxy_wallet"];
        let proxy_wallet = &mut ctx.accounts.proxy_wallet;
        proxy_wallet.bump = bump;
        proxy_wallet.primary_wallet = ctx.accounts.primary_wallet.key();
        proxy_wallet.user = ctx.accounts.user.key();
        Ok(())
    }

    #[access_control(check_authority(&ctx.accounts.authority, &ctx.accounts.proxy_wallet))]
    pub fn withdraw_lamports(ctx: Context<WithdrawLamports>, amount: u64) -> Result<()> {
        let authority_key = ctx.accounts.authority.key();
        let proxy_wallet_info = ctx.accounts.proxy_wallet.to_account_info();
        let receiver_info = ctx.accounts.receiver.to_account_info();

        if proxy_wallet_info.key() == receiver_info.key() {
            return Ok(());
        }

        let rent = Rent::get()?;
        let minimum_balance = rent.minimum_balance(ProxyWallet::LEN);

        let new_receiver_balance = receiver_info.lamports().checked_add(amount).unwrap();
        let new_wallet_balance = proxy_wallet_info
            .lamports()
            .checked_sub(amount)
            .ok_or(ErrorCode::InsufficientFunds)?;

        require_gte!(
            new_wallet_balance,
            minimum_balance,
            ErrorCode::InsufficientFunds
        );

        **receiver_info.lamports.borrow_mut() = new_receiver_balance;
        **proxy_wallet_info.lamports.borrow_mut() = new_wallet_balance;

        let proxy_wallet = &mut ctx.accounts.proxy_wallet;
        if authority_key == proxy_wallet.primary_wallet {
            proxy_wallet.total_money_withdrawn_primary_wallet = proxy_wallet
                .total_money_withdrawn_primary_wallet
                .checked_add(amount)
                .unwrap();
        } else {
            proxy_wallet.total_money_withdrawn_user = proxy_wallet
                .total_money_withdrawn_user
                .checked_add(amount)
                .unwrap();
        }

        Ok(())
    }

    #[access_control(check_authority(&ctx.accounts.authority, &ctx.accounts.proxy_wallet))]
    pub fn withdraw_ft(ctx: Context<WithdrawFt>, amount: u64) -> Result<()> {
        let proxy_wallet = &mut ctx.accounts.proxy_wallet;
        let proxy_wallet_token_account = &ctx.accounts.proxy_wallet_token_account;
        let receiver_token_account = &ctx.accounts.receiver_token_account;

        if proxy_wallet_token_account.key() == receiver_token_account.key() {
            return Ok(());
        }

        transfer_tokens(
            proxy_wallet,
            &ctx.accounts.proxy_wallet_token_account,
            &ctx.accounts.receiver_token_account,
            &ctx.accounts.token_program,
            amount,
        )?;

        let authority_key = ctx.accounts.authority.key();
        if authority_key == proxy_wallet.primary_wallet {
            proxy_wallet.total_ft_withdrawn_primary_wallet = proxy_wallet
                .total_ft_withdrawn_primary_wallet
                .checked_add(amount)
                .unwrap();
        } else {
            proxy_wallet.total_ft_withdrawn_user = proxy_wallet
                .total_ft_withdrawn_user
                .checked_add(amount)
                .unwrap();
        }

        Ok(())
    }

    #[access_control(check_authority(&ctx.accounts.authority, &ctx.accounts.proxy_wallet))]
    pub fn withdraw_nft(ctx: Context<WithdrawNft>) -> Result<()> {
        let proxy_wallet = &mut ctx.accounts.proxy_wallet;
        let proxy_wallet_token_account = &ctx.accounts.proxy_wallet_token_account;
        let receiver_token_account = &ctx.accounts.receiver_token_account;

        if proxy_wallet_token_account.key() == receiver_token_account.key() {
            return Ok(());
        }

        transfer_tokens(
            proxy_wallet,
            &ctx.accounts.proxy_wallet_token_account,
            &ctx.accounts.receiver_token_account,
            &ctx.accounts.token_program,
            1,
        )?;

        let authority_key = ctx.accounts.authority.key();
        if authority_key == proxy_wallet.primary_wallet {
            proxy_wallet.total_nft_withdrawn_primary_wallet = proxy_wallet
                .total_nft_withdrawn_primary_wallet
                .checked_add(1)
                .unwrap();
        } else {
            proxy_wallet.total_nft_withdrawn_user = proxy_wallet
                .total_nft_withdrawn_user
                .checked_add(1)
                .unwrap();
        }

        Ok(())
    }
}

#[derive(Accounts)]
pub struct CreateWallet<'info> {
    pub primary_wallet: SystemAccount<'info>,

    pub user: SystemAccount<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(init, payer = payer, space = ProxyWallet::LEN,
              seeds = [ProxyWallet::SEED, user.key.as_ref(), primary_wallet.key.as_ref()], bump)]
    pub proxy_wallet: Account<'info, ProxyWallet>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct WithdrawLamports<'info> {
    pub authority: Signer<'info>,

    #[account(mut)]
    pub proxy_wallet: Account<'info, ProxyWallet>,

    /// CHECK: this account is not being read
    #[account(mut)]
    pub receiver: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct WithdrawFt<'info> {
    pub authority: Signer<'info>,

    #[account(mut)]
    pub proxy_wallet: Account<'info, ProxyWallet>,

    #[account(constraint = mint.decimals != 0 || mint.supply != 1 @ ErrorCode::TokenIsNft)]
    pub mint: Account<'info, Mint>,

    #[account(mut, constraint = proxy_wallet_token_account.owner == proxy_wallet.key(),
              constraint = proxy_wallet_token_account.mint == mint.key())]
    pub proxy_wallet_token_account: Account<'info, TokenAccount>,

    #[account(mut, constraint = receiver_token_account.mint == mint.key())]
    pub receiver_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct WithdrawNft<'info> {
    pub authority: Signer<'info>,

    #[account(mut)]
    pub proxy_wallet: Account<'info, ProxyWallet>,

    #[account(constraint = nft_mint.decimals == 0 && nft_mint.supply == 1 @ ErrorCode::TokenIsNotNft)]
    pub nft_mint: Account<'info, Mint>,

    #[account(mut, constraint = proxy_wallet_token_account.owner == proxy_wallet.key(),
              constraint = proxy_wallet_token_account.mint == nft_mint.key())]
    pub proxy_wallet_token_account: Account<'info, TokenAccount>,

    #[account(mut, constraint = receiver_token_account.mint == nft_mint.key())]
    pub receiver_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Insufficient funds")]
    InsufficientFunds,

    #[msg("Token is an NFT")]
    TokenIsNft,

    #[msg("Token is not an NFT")]
    TokenIsNotNft,

    #[msg("Wrong authority")]
    WrongAuthority,
}
