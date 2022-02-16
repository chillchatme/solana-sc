use chill::utils::pda;
use chill_cli::client::Client;
use common::{random_fees, random_recipients, RPC_URL};
use solana_sdk::signature::{Keypair, Signer};

mod common;

#[test]
fn initialize() {
    let authority = Keypair::new();
    let client = Client::init(RPC_URL);
    let lamports = 1_000_000_000;

    client.airdrop(authority.pubkey(), lamports).unwrap();
    let mint = client.create_mint(&authority, 9).unwrap();

    pda::config(&mint, &chill::id());
    let fees = random_fees();
    let recipients = random_recipients();

    // assert!(client
    //     .initialize(
    //         chill::ID,
    //         &authority,
    //         mint,
    //         fees.clone(),
    //         recipients.clone()
    //     )
    //     .is_ok());

    let config = client.config(chill::ID, mint).unwrap();
    assert_eq!(config.fees, fees);
    assert_eq!(config.recipients, recipients);
    assert_eq!(config.mint, mint);

    let fees = random_fees();
    let recipients = random_recipients();

    // Already initialized
    assert!(client
        .initialize(chill::ID, &authority, mint, fees, recipients)
        .is_err());
}
