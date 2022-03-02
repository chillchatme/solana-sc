use chill_nft::{
    instruction::MintNftArgs,
    state::{Recipient, AUTHORITY_SHARE},
};
use common::{client::Client, random_fees, random_nft_args, random_recipients};
use mpl_token_metadata::state::Creator;
use solana_program::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signer};
use spl_associated_token_account::get_associated_token_address;

mod common;

const DECIMALS: u8 = 9;
const LAMPORTS: u64 = 500_000_000;
const TOKEN_AMOUNT: u64 = 1_000;

fn initialize(client: &Client, authority: &Keypair, explicit_recipient: Option<Pubkey>) -> Pubkey {
    let mint = client.create_mint(authority, DECIMALS).unwrap();
    let fees = random_fees();
    let mut recipients = random_recipients();

    if let Some(recipient_pubkey) = explicit_recipient {
        if recipients.is_empty() {
            let recipient = Recipient {
                address: recipient_pubkey,
                mint_share: 100,
                transaction_share: 100,
            };
            recipients.push(recipient);
        } else {
            recipients[0].address = recipient_pubkey;
        }
    }

    for recipient in recipients.iter() {
        client
            .create_token_account(authority, recipient.address, mint)
            .unwrap();
    }

    let args = chill_nft::instruction::InitializeArgs {
        fees: fees.clone(),
        recipients: recipients.clone(),
    };

    let ix = chill_nft::instruction::initialize(chill_nft::ID, authority.pubkey(), mint, args);
    client
        .run_transaction(&[ix.clone()], authority.pubkey(), &[authority])
        .unwrap();

    let config = client.config(mint).unwrap();
    assert_eq!(config.fees, fees);
    assert_eq!(config.recipients, recipients);
    assert_eq!(config.mint, mint);

    // Already initialized
    assert!(client
        .run_transaction(&[ix], authority.pubkey(), &[authority])
        .is_err());

    mint
}

fn recipients_token_accounts(client: &Client, mint: Pubkey) -> Vec<Pubkey> {
    let config = client.config(mint).unwrap();
    config
        .recipients
        .iter()
        .map(|r| get_associated_token_address(&r.address, &mint))
        .collect()
}

fn recipients_balances(client: &Client, mint: Pubkey) -> Vec<u64> {
    let config = client.config(mint).unwrap();

    config
        .recipients
        .iter()
        .map(|r| client.token_balance(r.address, mint).unwrap())
        .collect()
}

fn assert_metadata(
    authority: Pubkey,
    nft_mint: Pubkey,
    creators: Vec<Creator>,
    mint_args: &MintNftArgs,
    primary_sale_happened: bool,
    client: &Client,
) {
    let metadata = client.metadata(nft_mint).unwrap();
    assert_eq!(metadata.update_authority, authority);
    assert_eq!(metadata.mint, nft_mint);
    assert_eq!(metadata.primary_sale_happened, primary_sale_happened);

    let data = metadata.data;
    let zero = char::from(0);
    assert_eq!(data.name.trim_end_matches(zero), mint_args.name);
    assert_eq!(data.symbol.trim_end_matches(zero), mint_args.symbol);
    assert_eq!(data.uri.trim_end_matches(zero), mint_args.url);
    assert_eq!(data.seller_fee_basis_points, mint_args.fees);
    assert_eq!(data.creators, Some(creators));

    let chill_metadata = client.chill_metadata(nft_mint).unwrap();
    assert_eq!(chill_metadata.nft_type, mint_args.nft_type);
}

#[test]
fn mint_nft() {
    let authority = Keypair::new();
    let client = Client::new();
    let user = Keypair::new();

    client.airdrop(authority.pubkey(), LAMPORTS).unwrap();
    client.airdrop(user.pubkey(), LAMPORTS).unwrap();

    for _ in 0..2 {
        let mint = initialize(&client, &authority, None);
        let user_token_account = client
            .create_token_account(&user, user.pubkey(), mint)
            .unwrap();
        client
            .mint_to(&authority, mint, user_token_account, TOKEN_AMOUNT)
            .unwrap();

        for _ in 0..2 {
            let mint_nft_args = random_nft_args();
            let initial_balances = recipients_balances(&client, mint);
            let nft_mint = client.create_mint(&authority, 0).unwrap();
            let nft_token = client
                .create_token_account(&authority, user.pubkey(), nft_mint)
                .unwrap();
            client.mint_to(&authority, nft_mint, nft_token, 1).unwrap();

            let recipients_tokens = recipients_token_accounts(&client, mint);
            let ix = chill_nft::instruction::mint_nft(
                chill_nft::ID,
                authority.pubkey(),
                user.pubkey(),
                mint,
                user_token_account,
                nft_mint,
                nft_token,
                &recipients_tokens,
                mint_nft_args.clone(),
            );

            client
                .run_transaction(&[ix], user.pubkey(), &[&authority, &user])
                .unwrap();

            let creators = vec![
                Creator {
                    address: authority.pubkey(),
                    verified: true,
                    share: AUTHORITY_SHARE,
                },
                Creator {
                    address: user.pubkey(),
                    verified: true,
                    share: 100 - AUTHORITY_SHARE,
                },
            ];

            assert_metadata(
                authority.pubkey(),
                nft_mint,
                creators,
                &mint_nft_args,
                true,
                &client,
            );

            let config = client.config(mint).unwrap();
            let fees = config.fees.of(mint_nft_args.nft_type);
            for (recipient, initial_balance) in config.recipients.iter().zip(initial_balances) {
                let balance = client.token_balance(recipient.address, mint).unwrap();
                let expected_balance = initial_balance + fees * recipient.mint_share as u64 / 100;
                assert_eq!(balance, expected_balance);
            }
        }
    }
}

#[test]
fn mint_nft_to_authority() {
    let authority = Keypair::new();
    let client = Client::new();
    client.airdrop(authority.pubkey(), LAMPORTS).unwrap();

    for _ in 0..2 {
        let mint = initialize(&client, &authority, None);
        let authority_token_account = client
            .create_token_account(&authority, authority.pubkey(), mint)
            .unwrap();
        client
            .mint_to(&authority, mint, authority_token_account, TOKEN_AMOUNT)
            .unwrap();

        for _ in 0..2 {
            let mint_nft_args = random_nft_args();
            let initial_balances = recipients_balances(&client, mint);

            let nft_mint = client.create_mint(&authority, 0).unwrap();
            let nft_token = client
                .create_token_account(&authority, authority.pubkey(), nft_mint)
                .unwrap();

            client.mint_to(&authority, nft_mint, nft_token, 1).unwrap();

            let recipients_tokens = recipients_token_accounts(&client, mint);
            let ix = chill_nft::instruction::mint_nft(
                chill_nft::ID,
                authority.pubkey(),
                authority.pubkey(),
                mint,
                authority_token_account,
                nft_mint,
                nft_token,
                &recipients_tokens,
                mint_nft_args.clone(),
            );

            client
                .run_transaction(&[ix], authority.pubkey(), &[&authority])
                .unwrap();

            let creators = vec![Creator {
                address: authority.pubkey(),
                verified: true,
                share: 100,
            }];

            assert_metadata(
                authority.pubkey(),
                nft_mint,
                creators,
                &mint_nft_args,
                false,
                &client,
            );

            let config = client.config(mint).unwrap();
            let fees = config.fees.of(mint_nft_args.nft_type);
            for (recipient, initial_balance) in config.recipients.iter().zip(initial_balances) {
                let balance = client.token_balance(recipient.address, mint).unwrap();
                let expected_balance = initial_balance + fees * recipient.mint_share as u64 / 100;
                assert_eq!(balance, expected_balance);
            }
        }
    }
}

#[test]
fn mint_nft_with_recipient_authority() {
    let authority = Keypair::new();
    let client = Client::new();
    client.airdrop(authority.pubkey(), LAMPORTS).unwrap();

    for _ in 0..2 {
        let mint = initialize(&client, &authority, Some(authority.pubkey()));
        let authority_token_account = get_associated_token_address(&authority.pubkey(), &mint);
        client
            .mint_to(&authority, mint, authority_token_account, TOKEN_AMOUNT)
            .unwrap();

        for _ in 0..2 {
            let mint_nft_args = random_nft_args();
            let initial_balances = recipients_balances(&client, mint);

            let nft_mint = client.create_mint(&authority, 0).unwrap();
            let nft_token = client
                .create_token_account(&authority, authority.pubkey(), nft_mint)
                .unwrap();
            client.mint_to(&authority, nft_mint, nft_token, 1).unwrap();

            let initial_authority_balance = client.token_balance(authority.pubkey(), mint).unwrap();

            let recipients_tokens = recipients_token_accounts(&client, mint);
            let ix = chill_nft::instruction::mint_nft(
                chill_nft::ID,
                authority.pubkey(),
                authority.pubkey(),
                mint,
                authority_token_account,
                nft_mint,
                nft_token,
                &recipients_tokens,
                mint_nft_args.clone(),
            );

            client
                .run_transaction(&[ix], authority.pubkey(), &[&authority])
                .unwrap();

            let creators = vec![Creator {
                address: authority.pubkey(),
                verified: true,
                share: 100,
            }];

            assert_metadata(
                authority.pubkey(),
                nft_mint,
                creators,
                &mint_nft_args,
                false,
                &client,
            );

            let config = client.config(mint).unwrap();
            let fees = config.fees.of(mint_nft_args.nft_type);

            let mut authority_expected_balance = initial_authority_balance;
            for (recipient, initial_balance) in
                config.recipients.iter().zip(initial_balances).skip(1)
            {
                let balance = client.token_balance(recipient.address, mint).unwrap();
                let recipient_fees = fees * recipient.mint_share as u64 / 100;
                let expected_balance = initial_balance + recipient_fees;
                assert_eq!(balance, expected_balance);

                authority_expected_balance -= recipient_fees;
            }

            let authority_balance = client.token_balance(authority.pubkey(), mint).unwrap();
            assert_eq!(authority_balance, authority_expected_balance);
        }
    }
}
