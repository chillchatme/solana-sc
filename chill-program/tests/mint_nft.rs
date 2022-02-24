use chill_api::{
    instruction::MintNftArgs,
    pda,
    state::{Recipient, AUTHORITY_SHARE},
};
use chill_client::client::Client;
use common::{
    random_fees, random_nft_args, random_recipients, sequential_airdrop, DECIMALS, RPC_URL,
    TOKEN_AMOUNT,
};
use mpl_token_metadata::state::{Creator, Metadata};
use solana_program::{borsh::try_from_slice_unchecked, pubkey::Pubkey};
use solana_sdk::signature::{Keypair, Signer};

mod common;

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

    client
        .initialize(
            chill_api::ID,
            authority,
            mint,
            fees.clone(),
            recipients.clone(),
        )
        .unwrap();

    for recipient in recipients.iter() {
        client
            .create_token_account(authority, recipient.address, mint)
            .unwrap();
    }

    let config = client.config(chill_api::ID, mint).unwrap();
    assert_eq!(config.fees, fees);
    assert_eq!(config.recipients, recipients);
    assert_eq!(config.mint, mint);

    // Already initialized
    assert!(client
        .initialize(chill_api::ID, authority, mint, fees, recipients)
        .is_err());

    mint
}

fn recipients_balances(client: &Client, mint: Pubkey) -> Vec<u64> {
    let config = client.config(chill_api::ID, mint).unwrap();
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
    let metadata_pubkey = pda::metadata(&nft_mint);
    let metadata_data = client.account_data(metadata_pubkey).unwrap();
    let metadata = try_from_slice_unchecked::<Metadata>(&metadata_data).unwrap();

    assert_eq!(metadata.update_authority, authority);
    assert_eq!(metadata.mint, nft_mint);
    assert_eq!(metadata.primary_sale_happened, primary_sale_happened);

    let data = metadata.data;
    let zero = char::from(0);
    assert_eq!(data.name.trim_end_matches(zero), mint_args.name);
    assert_eq!(data.symbol.trim_end_matches(zero), mint_args.symbol);
    assert_eq!(data.uri.trim_end_matches(zero), mint_args.uri);
    assert_eq!(data.seller_fee_basis_points, mint_args.fees);
    assert_eq!(data.creators, Some(creators));
}

fn create_token_account(
    client: &Client,
    authority: &Keypair,
    owner: &Keypair,
    mint: Pubkey,
) -> Pubkey {
    let owner_token_account = client
        .create_token_account(owner, owner.pubkey(), mint)
        .unwrap();
    client
        .mint_to(authority, mint, owner_token_account, TOKEN_AMOUNT)
        .unwrap();

    owner_token_account
}

fn accounts_for_mint_nft(
    client: &Client,
    authority: &Keypair,
    nft_owner: &Keypair,
) -> (Pubkey, Pubkey) {
    let nft_mint = client.create_mint(authority, 0).unwrap();

    let nft_token = client
        .create_token_account(nft_owner, nft_owner.pubkey(), nft_mint)
        .unwrap();

    client.mint_to(authority, nft_mint, nft_token, 1).unwrap();

    (nft_mint, nft_token)
}

#[test]
fn mint_nft() {
    let authority = Keypair::new();
    let client = Client::init(RPC_URL);
    let user = Keypair::new();

    sequential_airdrop(&client, authority.pubkey()).unwrap();
    sequential_airdrop(&client, user.pubkey()).unwrap();

    for _ in 0..2 {
        let mint = initialize(&client, &authority, None);
        let user_token_account = create_token_account(&client, &authority, &user, mint);

        for _ in 0..2 {
            let mint_nft_args = random_nft_args();
            let initial_balances = recipients_balances(&client, mint);
            let (nft_mint, nft_token) = accounts_for_mint_nft(&client, &authority, &user);

            client
                .mint_nft(
                    chill_api::ID,
                    &authority,
                    &user,
                    mint,
                    user_token_account,
                    nft_mint,
                    nft_token,
                    mint_nft_args.clone(),
                )
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

            let config = client.config(chill_api::ID, mint).unwrap();
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
    let client = Client::init(RPC_URL);
    sequential_airdrop(&client, authority.pubkey()).unwrap();

    for _ in 0..2 {
        let mint = initialize(&client, &authority, None);
        let authority_token_account = create_token_account(&client, &authority, &authority, mint);

        for _ in 0..2 {
            let mint_nft_args = random_nft_args();
            let initial_balances = recipients_balances(&client, mint);
            let (nft_mint, nft_token) = accounts_for_mint_nft(&client, &authority, &authority);

            client
                .mint_nft(
                    chill_api::ID,
                    &authority,
                    &authority,
                    mint,
                    authority_token_account,
                    nft_mint,
                    nft_token,
                    mint_nft_args.clone(),
                )
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

            let config = client.config(chill_api::ID, mint).unwrap();
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
    let client = Client::init(RPC_URL);
    sequential_airdrop(&client, authority.pubkey()).unwrap();

    for _ in 0..2 {
        let mint = initialize(&client, &authority, Some(authority.pubkey()));
        let authority_token_account = client.associated_token_address(authority.pubkey(), mint);
        client
            .mint_to(&authority, mint, authority_token_account, TOKEN_AMOUNT)
            .unwrap();

        for _ in 0..2 {
            let mint_nft_args = random_nft_args();
            let initial_balances = recipients_balances(&client, mint);
            let (nft_mint, nft_token) = accounts_for_mint_nft(&client, &authority, &authority);

            let initial_authority_balance = client.token_balance(authority.pubkey(), mint).unwrap();

            client
                .mint_nft(
                    chill_api::ID,
                    &authority,
                    &authority,
                    mint,
                    authority_token_account,
                    nft_mint,
                    nft_token,
                    mint_nft_args.clone(),
                )
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

            let config = client.config(chill_api::ID, mint).unwrap();
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
