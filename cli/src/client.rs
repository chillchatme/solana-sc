use crate::error::{CliError, Result};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::Instruction,
    native_token::lamports_to_sol,
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
use std::convert::TryInto;

pub struct Client {
    client: RpcClient,
}

impl Client {
    pub fn init(url: &str) -> Self {
        let client = RpcClient::new_with_commitment(url.to_owned(), CommitmentConfig::confirmed());
        Self { client }
    }

    pub fn balance(&self, owner: Pubkey) -> Result<u64> {
        self.client.get_balance(&owner).map_err(|e| e.into())
    }

    pub fn airdrop(&self, address: Pubkey, lamports: u64) -> Result<()> {
        let signature = self.client.request_airdrop(&address, lamports)?;
        let initial_balance = self.client.get_balance(&address)?;
        let blockhash = self.client.get_latest_blockhash()?;
        self.client.confirm_transaction_with_spinner(
            &signature,
            &blockhash,
            CommitmentConfig::confirmed(),
        )?;

        let new_balance = self.client.get_balance(&address)?;
        if initial_balance >= new_balance {
            return Err(CliError::CannotAirdrop(lamports_to_sol(lamports)).into());
        }

        Ok(())
    }

    pub fn token_account(&self, owner: Pubkey, mint: Pubkey) -> Result<Account> {
        let token_pubkey = get_associated_token_address(&owner, &mint);
        let data = self
            .client
            .get_account_data(&token_pubkey)
            .map_err(|_| CliError::TokenNotInitialized(owner, mint))?;

        Ok(Account::unpack(&data)?)
    }

    pub fn mint_account(&self, address: Pubkey) -> Result<Mint> {
        let data = self
            .client
            .get_account_data(&address)
            .map_err(|_| CliError::MintNotFound(address))?;
        Ok(Mint::unpack(&data)?)
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

    pub fn get_token_pubkey(&self, owner: Pubkey, mint: Pubkey) -> Result<Pubkey> {
        self.token_account(owner, mint)?;
        Ok(get_associated_token_address(&owner, &mint))
    }

    pub fn create_token_account(&self, owner: &dyn Signer, mint: Pubkey) -> Result<Pubkey> {
        let token_pubkey = get_associated_token_address(&owner.pubkey(), &mint);
        let ix = create_associated_token_account(&owner.pubkey(), &owner.pubkey(), &mint);
        self.run_transaction(&[ix], owner.pubkey(), &[owner])?;
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
