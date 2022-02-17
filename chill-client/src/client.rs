use crate::error::{CustomClientError, Result};
use chill_api::{
    self,
    instruction::{self, InitializeArgs, MintNftArgs},
    pda,
    state::{Config, Fees, Recipient},
};
use solana_client::{rpc_client::RpcClient, rpc_request::TokenAccountsFilter};
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
    amount_to_ui_amount, instruction as spl_instruction,
    state::{Account, Mint},
};
use std::{convert::TryInto, str::FromStr};

pub struct Client {
    client: RpcClient,
}

impl Client {
    pub fn init(url: &str) -> Self {
        let client = RpcClient::new_with_commitment(url.to_owned(), CommitmentConfig::confirmed());
        Self { client }
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

    pub fn airdrop(&self, address: Pubkey, lamports: u64) -> Result<()> {
        let signature = self.client.request_airdrop(&address, lamports)?;
        let blockhash = self.client.get_latest_blockhash()?;
        self.client
            .confirm_transaction_with_spinner(&signature, &blockhash, CommitmentConfig::confirmed())
            .map_err(|e| e.into())
    }

    pub fn balance(&self, owner: Pubkey) -> Result<u64> {
        self.client.get_balance(&owner).map_err(|e| e.into())
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
            spl_instruction::initialize_mint(
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

    pub fn associated_token_address(&self, owner: Pubkey, mint: Pubkey) -> Pubkey {
        get_associated_token_address(&owner, &mint)
    }

    pub fn create_token_account(
        &self,
        payer: &dyn Signer,
        owner: Pubkey,
        mint: Pubkey,
    ) -> Result<Pubkey> {
        let token_pubkey = self.associated_token_address(owner, mint);
        let ix = create_associated_token_account(&payer.pubkey(), &owner, &mint);
        self.run_transaction(&[ix], payer.pubkey(), &[payer])?;
        Ok(token_pubkey)
    }

    pub fn account_data(&self, address: Pubkey) -> Result<Vec<u8>> {
        self.client.get_account_data(&address).map_err(|e| e.into())
    }

    pub fn mint_account(&self, address: Pubkey) -> Result<Mint> {
        let data = self
            .client
            .get_account_data(&address)
            .map_err(|_| CustomClientError::MintNotFound(address))?;
        let mint = Mint::unpack(&data).map_err(|_| CustomClientError::DataIsNotMint)?;
        Ok(mint)
    }

    pub fn mint_to(
        &self,
        owner: &dyn Signer,
        mint: Pubkey,
        token: Pubkey,
        amount: u64,
    ) -> Result<()> {
        let ix =
            spl_instruction::mint_to(&spl_token::ID, &mint, &token, &owner.pubkey(), &[], amount)
                .unwrap();

        self.run_transaction(&[ix], owner.pubkey(), &[owner])?;
        Ok(())
    }

    pub fn transfer_tokens(
        &self,
        owner: &dyn Signer,
        mint: Pubkey,
        recipient_token_account: Pubkey,
        amount: u64,
    ) -> Result<Signature> {
        let owner_token_pubkey = self.associated_token_address(owner.pubkey(), mint);
        let ix = spl_token::instruction::transfer(
            &spl_token::ID,
            &owner_token_pubkey,
            &recipient_token_account,
            &owner.pubkey(),
            &[],
            amount,
        )
        .unwrap();

        self.run_transaction(&[ix], owner.pubkey(), &[owner])
    }

    pub fn token_account(&self, address: Pubkey) -> Result<Account> {
        let data = self
            .client
            .get_account_data(&address)
            .map_err(|_| CustomClientError::TokenNotInitialized(address))?;

        let token_account =
            Account::unpack(&data).map_err(|_| CustomClientError::DataIsNotTokenAccount)?;

        Ok(token_account)
    }

    pub fn token_balance(&self, owner: Pubkey, mint: Pubkey) -> Result<u64> {
        let filter = TokenAccountsFilter::Mint(mint);
        let token_accounts = self.client.get_token_accounts_by_owner(&owner, filter)?;
        let addresses = token_accounts
            .iter()
            .map(|t| Pubkey::from_str(&t.pubkey).unwrap());

        let mut balance = 0;
        for address in addresses {
            let token_account = self.token_account(address)?;
            balance += token_account.amount;
        }

        Ok(balance)
    }

    pub fn ui_token_balance(&self, owner: Pubkey, mint: Pubkey) -> Result<f64> {
        let token_balance = self.token_balance(owner, mint)?;
        let mint = self.mint_account(mint)?;
        let decimals = mint.decimals;
        Ok(amount_to_ui_amount(token_balance, decimals))
    }

    pub fn find_token_account(&self, address: Pubkey, mint: Pubkey) -> Result<Option<Pubkey>> {
        let filter = TokenAccountsFilter::Mint(mint);
        let token_accounts = self.client.get_token_accounts_by_owner(&address, filter)?;
        if token_accounts.is_empty() {
            return Ok(None);
        }

        let associated_token_pubkey = self.associated_token_address(address, mint);
        let associated_token_string = associated_token_pubkey.to_string();
        let associated_token_exists = token_accounts
            .iter()
            .any(|t| t.pubkey == associated_token_string);

        if associated_token_exists {
            return Ok(Some(associated_token_pubkey));
        }

        let first_token_pubkey = Pubkey::from_str(&token_accounts[0].pubkey).unwrap();
        Ok(Some(first_token_pubkey))
    }

    pub fn config(&self, program_id: Pubkey, mint: Pubkey) -> Result<Config> {
        let config_pubkey = pda::config(&mint, &program_id).0;
        let config_data = self
            .client
            .get_account_data(&config_pubkey)
            .map_err(|_| CustomClientError::ConfigNotFound)?;

        Config::unpack(&config_data).map_err(|_| CustomClientError::ConfigDataError.into())
    }

    pub fn initialize(
        &self,
        program_id: Pubkey,
        owner: &dyn Signer,
        mint: Pubkey,
        fees: Fees,
        recipients: Vec<Recipient>,
    ) -> Result<Signature> {
        let args = InitializeArgs { fees, recipients };
        let ix = instruction::initialize(program_id, owner.pubkey(), mint, args);
        self.run_transaction(&[ix], owner.pubkey(), &[owner])
    }

    #[allow(clippy::too_many_arguments)]
    pub fn mint_nft(
        &self,
        program_id: Pubkey,
        owner: &dyn Signer,
        user: &dyn Signer,
        mint_chill: Pubkey,
        user_token_account: Pubkey,
        nft_mint: Pubkey,
        nft_token: Pubkey,
        args: MintNftArgs,
    ) -> Result<Signature> {
        let config = self.config(program_id, mint_chill)?;

        let mut recipients_token_accounts = Vec::with_capacity(config.recipients.len());
        for recipient in config.recipients {
            match self.find_token_account(recipient.address, mint_chill)? {
                Some(token_address) => recipients_token_accounts.push(token_address),
                None => {
                    let token_address =
                        self.create_token_account(user, recipient.address, mint_chill)?;
                    recipients_token_accounts.push(token_address);
                }
            };
        }

        let ix = instruction::mint_nft(
            program_id,
            owner.pubkey(),
            user.pubkey(),
            mint_chill,
            user_token_account,
            nft_mint,
            nft_token,
            &recipients_token_accounts,
            args,
        );

        self.run_transaction(&[ix], user.pubkey(), &[owner, user])
    }
}
