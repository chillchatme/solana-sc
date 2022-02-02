use std::convert::TryInto;

use crate::error::Result;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::Instruction,
    program_pack::Pack,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
    signers::Signers,
    system_instruction,
    transaction::Transaction,
};
use spl_associated_token_account::{create_associated_token_account, get_associated_token_address};
use spl_token::{
    instruction::{initialize_mint, mint_to},
    state::{Account, Mint},
};

pub struct Client {
    client: RpcClient,
}

impl Client {
    pub fn init(url: &str) -> Self {
        let client = RpcClient::new_with_commitment(url.to_owned(), CommitmentConfig::confirmed());
        Self { client }
    }

    pub fn airdrop(&self, address: Pubkey, lamports: u64) -> Result<()> {
        println!("address {}, lamports {}", address, lamports);
        let signature = self.client.request_airdrop(&address, lamports)?;
        loop {
            let confirmed = self
                .client
                .confirm_transaction_with_commitment(&signature, CommitmentConfig::confirmed())?;
            if confirmed.value {
                break;
            }
        }

        Ok(())
    }

    pub fn is_token_exists(&self, owner: Pubkey, mint: Pubkey) -> bool {
        let token_pubkey = get_associated_token_address(&owner, &mint);
        match self.client.get_account_data(&token_pubkey) {
            Ok(ref data) => Account::unpack(data).is_ok(),
            Err(_) => false,
        }
    }

    fn run_transaction(
        &self,
        instructions: &[Instruction],
        payer: Pubkey,
        signers: &impl Signers,
    ) -> Result<Signature> {
        let blockhash = self.client.get_latest_blockhash()?;
        let transaction =
            Transaction::new_signed_with_payer(instructions, Some(&payer), signers, blockhash);
        self.client
            .send_and_confirm_transaction(&transaction)
            .map_err(|e| e.into())
    }

    pub fn create_mint(&self, owner: &dyn Signer, decimals: u8) -> Result<Pubkey> {
        let mint = Keypair::new();
        let space = Mint::LEN;
        let lamports = self.client.get_minimum_balance_for_rent_exemption(space)?;

        let ixs = &[
            system_instruction::create_account(
                &owner.pubkey(),
                &mint.pubkey(),
                lamports,
                space.try_into().unwrap(),
                &spl_token::ID,
            ),
            initialize_mint(
                &spl_token::ID,
                &mint.pubkey(),
                &owner.pubkey(),
                None,
                decimals,
            )
            .unwrap(),
        ];

        self.run_transaction(ixs, owner.pubkey(), &[owner, &mint])?;
        Ok(mint.pubkey())
    }

    pub fn create_token(&self, owner: &dyn Signer, mint: Pubkey) -> Result<Pubkey> {
        let ix = create_associated_token_account(&owner.pubkey(), &owner.pubkey(), &mint);
        self.run_transaction(&[ix], owner.pubkey(), &[owner])?;
        let token_pubkey = get_associated_token_address(&owner.pubkey(), &mint);
        Ok(token_pubkey)
    }

    pub fn mint_to(
        &self,
        owner: &dyn Signer,
        mint: Pubkey,
        token: Pubkey,
        amount: u64,
    ) -> Result<()> {
        let ix = mint_to(&spl_token::ID, &mint, &token, &owner.pubkey(), &[], amount)?;
        self.run_transaction(&[ix], owner.pubkey(), &[owner])?;
        Ok(())
    }
}
