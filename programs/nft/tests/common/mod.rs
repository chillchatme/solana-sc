use chill_nft::{
    instruction::MintNftArgs,
    state::{Config, Fees, NftType, Recipient},
};
use rand::{prelude::SliceRandom, Rng};
use solana_sdk::{signature::Keypair, signer::Signer};

pub mod client;

pub fn random_fees() -> Fees {
    let mut rng = rand::thread_rng();
    Fees {
        character: rng.gen_range(0..10),
        pet: rng.gen_range(0..10),
        emote: rng.gen_range(0..10),
        tileset: rng.gen_range(0..10),
        item: rng.gen_range(0..10),
    }
}

pub fn random_recipients() -> Vec<Recipient> {
    let mut rng = rand::thread_rng();
    let amount = rng.gen_range(0..=Config::MAX_RECIPIENT_NUMBER);

    let mut recipients = Vec::with_capacity(amount);
    let mut total_mint_share = 100;
    let mut total_transaction_share = 100;

    for _ in 0..amount {
        let mint_share = rng.gen_range(0..total_mint_share);
        let transaction_share = rng.gen_range(0..total_transaction_share);
        let recipient = Recipient {
            address: Keypair::new().pubkey(),
            mint_share,
            transaction_share,
        };
        recipients.push(recipient);
        total_mint_share -= mint_share;
        total_transaction_share -= transaction_share;
    }

    if !recipients.is_empty() {
        let last = recipients.last_mut().unwrap();
        last.mint_share += total_mint_share;
        last.transaction_share += total_transaction_share;
    }

    recipients
}

pub fn random_nft_args() -> MintNftArgs {
    let nft_types = &[
        NftType::Character,
        NftType::Pet,
        NftType::Emote,
        NftType::Tileset,
        NftType::Item,
    ];

    let mut rng = rand::thread_rng();
    let nft_type = nft_types.choose(&mut rng).unwrap();

    MintNftArgs {
        nft_type: *nft_type,
        name: format!("NAME_{0}", rng.gen_range(0..100)),
        symbol: format!("SYM_{0}", rng.gen_range(0..100)),
        url: format!("https://arweave.com/{0}", Keypair::new().pubkey()),
        fees: rng.gen_range(0..=10000),
    }
}
