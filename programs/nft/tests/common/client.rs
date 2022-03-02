use chill_nft::{
    state::{ChillNftMetadata, Config},
    utils::pda,
};
use lazy_static::lazy_static;
use mpl_token_metadata::{
    state::{Key, Metadata, MAX_METADATA_LEN},
    utils::try_from_slice_checked,
};
use solana_client::{client_error::Result, rpc_client::RpcClient};
use solana_program::{
    instruction::Instruction, program_pack::Pack, pubkey::Pubkey, system_instruction,
};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::{Keypair, Signature},
    signer::Signer,
    signers::Signers,
    transaction::Transaction,
};
use spl_associated_token_account::{create_associated_token_account, get_associated_token_address};
use spl_token::state::{Account, Mint};
use std::sync::Mutex;

pub const RPC_URL: &str = "https://api.devnet.solana.com";

lazy_static! {
    static ref TEST_MUTEX: Mutex<()> = Mutex::new(());
}

pub struct Client {
    rpc_client: RpcClient,
}

impl Client {
    pub fn new() -> Self {
        let rpc_client = RpcClient::new_with_commitment(RPC_URL, CommitmentConfig::confirmed());
        Self { rpc_client }
    }

    pub fn airdrop(&self, address: Pubkey, lamports: u64) -> Result<()> {
        let blockhash = self.rpc_client.get_latest_blockhash()?;
        let _guard;

        if RPC_URL.contains("localhost") {
            _guard = None;
        } else {
            _guard = Some(TEST_MUTEX.lock().unwrap());
        }

        let signature = self.rpc_client.request_airdrop(&address, lamports)?;
        self.rpc_client.confirm_transaction_with_spinner(
            &signature,
            &blockhash,
            CommitmentConfig::confirmed(),
        )
    }

    pub fn token_balance(&self, owner: Pubkey, mint: Pubkey) -> Result<u64> {
        let token_pubkey = get_associated_token_address(&owner, &mint);
        let token_data = self.rpc_client.get_account_data(&token_pubkey)?;
        Ok(Account::unpack(&token_data).unwrap().amount)
    }

    pub fn config(&self, mint: Pubkey) -> Result<Config> {
        let config_pubkey = pda::config(&mint, &chill_nft::ID).0;
        let config_data = self.rpc_client.get_account_data(&config_pubkey)?;
        Ok(Config::unpack(&config_data).unwrap())
    }

    pub fn metadata(&self, mint: Pubkey) -> Result<Metadata> {
        let metadata_pubkey = pda::metadata(&mint);
        let data = self.rpc_client.get_account_data(&metadata_pubkey)?;
        Ok(try_from_slice_checked(&data, Key::MetadataV1, MAX_METADATA_LEN).unwrap())
    }

    pub fn chill_metadata(&self, nft_mint: Pubkey) -> Result<ChillNftMetadata> {
        let chill_metadata_pubkey = pda::chill_metadata(&nft_mint, &chill_nft::ID).0;
        let chill_metadata_data = self.rpc_client.get_account_data(&chill_metadata_pubkey)?;
        Ok(ChillNftMetadata::unpack(&chill_metadata_data).unwrap())
    }

    pub fn run_transaction(
        &self,
        instructions: &[Instruction],
        payer: Pubkey,
        signers: &impl Signers,
    ) -> Result<Signature> {
        let blockhash = self.rpc_client.get_latest_blockhash()?;
        let transaction =
            Transaction::new_signed_with_payer(instructions, Some(&payer), signers, blockhash);

        self.rpc_client.send_and_confirm_transaction(&transaction)
    }

    pub fn create_mint(&self, authority: &Keypair, decimals: u8) -> Result<Pubkey> {
        let mint = Keypair::new();
        let space = Mint::LEN;
        let lamports = self
            .rpc_client
            .get_minimum_balance_for_rent_exemption(space)?;

        let ixs = &[
            system_instruction::create_account(
                &authority.pubkey(),
                &mint.pubkey(),
                lamports,
                space.try_into().unwrap(),
                &spl_token::ID,
            ),
            spl_token::instruction::initialize_mint(
                &spl_token::ID,
                &mint.pubkey(),
                &authority.pubkey(),
                None,
                decimals,
            )
            .unwrap(),
        ];

        self.run_transaction(ixs, authority.pubkey(), &[authority, &mint])?;
        Ok(mint.pubkey())
    }

    pub fn mint_to(
        &self,
        authority: &Keypair,
        mint: Pubkey,
        token_account: Pubkey,
        amount: u64,
    ) -> Result<Signature> {
        let ix = spl_token::instruction::mint_to(
            &spl_token::ID,
            &mint,
            &token_account,
            &authority.pubkey(),
            &[],
            amount,
        )
        .unwrap();

        self.run_transaction(&[ix], authority.pubkey(), &[authority])
    }

    pub fn create_token_account(
        &self,
        payer: &Keypair,
        owner: Pubkey,
        mint: Pubkey,
    ) -> Result<Pubkey> {
        let ix = create_associated_token_account(&payer.pubkey(), &owner, &mint);
        self.run_transaction(&[ix], payer.pubkey(), &[payer])?;
        Ok(get_associated_token_address(&owner, &mint))
    }
}
