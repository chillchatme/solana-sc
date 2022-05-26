use crate::{state::ProxyWallet, ErrorCode};
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount};

pub fn check_authority(authority: &Signer, proxy_wallet: &Account<ProxyWallet>) -> Result<()> {
    let authority_key = authority.key();
    let proxy_wallet_with_bump;

    if authority_key == proxy_wallet.primary_wallet {
        proxy_wallet_with_bump = Pubkey::find_program_address(
            &[
                ProxyWallet::SEED,
                proxy_wallet.user.as_ref(),
                authority_key.as_ref(),
            ],
            &crate::ID,
        );
    } else if authority_key == proxy_wallet.user {
        proxy_wallet_with_bump = Pubkey::find_program_address(
            &[
                ProxyWallet::SEED,
                authority_key.as_ref(),
                proxy_wallet.primary_wallet.as_ref(),
            ],
            &crate::ID,
        );
    } else {
        return err!(WrongAuthority);
    }

    require_keys_eq!(
        proxy_wallet.key(),
        proxy_wallet_with_bump.0,
        ErrorCode::WrongAuthority
    );

    require_eq!(
        proxy_wallet.bump,
        proxy_wallet_with_bump.1,
        ErrorCode::WrongAuthority
    );

    Ok(())
}

pub fn transfer_tokens<'info>(
    proxy_wallet: &Account<'info, ProxyWallet>,
    proxy_wallet_token: &Account<'info, TokenAccount>,
    receiver_token: &Account<'info, TokenAccount>,
    token_program: &Program<'info, Token>,
    amount: u64,
) -> Result<()> {
    if proxy_wallet_token.key() == receiver_token.key() {
        return Ok(());
    }

    let seeds = &[
        ProxyWallet::SEED,
        proxy_wallet.user.as_ref(),
        proxy_wallet.primary_wallet.as_ref(),
        &[proxy_wallet.bump],
    ];

    token::transfer(
        CpiContext::new(
            token_program.to_account_info(),
            token::Transfer {
                from: proxy_wallet_token.to_account_info(),
                to: receiver_token.to_account_info(),
                authority: proxy_wallet.to_account_info(),
            },
        )
        .with_signer(&[seeds]),
        amount,
    )
}
