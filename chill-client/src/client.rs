use crate::error::{CustomClientError, Result};
use chill_api::{
    self,
    instruction::{self, InitializeArgs, MintNftArgs},
    pda,
    state::{Config, Fees, Recipient, AUTHORITY_SHARE},
};
use mpl_token_metadata::{
    state::{Creator, DataV2, Key, Metadata, TokenStandard, MAX_METADATA_LEN},
    utils::try_from_slice_checked,
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

    //
    // Accounts
    //

    pub fn account_data(&self, address: Pubkey) -> Result<Vec<u8>> {
        self.client.get_account_data(&address).map_err(|e| e.into())
    }

    pub fn mint_account(&self, address: Pubkey) -> Result<Mint> {
        let data = self
            .client
            .get_account_data(&address)
            .map_err(|_| CustomClientError::MintNotFound(address))?;
        let mint = Mint::unpack(&data).map_err(|_| CustomClientError::AccountIsNotMint)?;
        Ok(mint)
    }

    pub fn token_account(&self, address: Pubkey) -> Result<Account> {
        let data = self
            .client
            .get_account_data(&address)
            .map_err(|_| CustomClientError::TokenNotInitialized(address))?;

        let token_account =
            Account::unpack(&data).map_err(|_| CustomClientError::AccountIsNotToken)?;

        Ok(token_account)
    }

    pub fn metadata_account(&self, mint: Pubkey) -> Result<Metadata> {
        let metadata_pubkey = pda::metadata(&mint);
        let data = self
            .client
            .get_account_data(&metadata_pubkey)
            .map_err(|_| CustomClientError::MetadataNotFound(mint))?;

        try_from_slice_checked(&data, Key::MetadataV1, MAX_METADATA_LEN)
            .map_err(|_| CustomClientError::AccountIsNotMetadata.into())
    }

    pub fn config(&self, program_id: Pubkey, mint: Pubkey) -> Result<Config> {
        let config_pubkey = pda::config(&mint, &program_id).0;
        let config_data = self
            .client
            .get_account_data(&config_pubkey)
            .map_err(|_| CustomClientError::ConfigNotFound)?;

        Config::unpack(&config_data).map_err(|_| CustomClientError::ConfigDataError.into())
    }

    //
    // Mint & Token accounts functions
    //

    pub fn create_mint_and_token_nft(
        &self,
        owner: &dyn Signer,
        recipient: &dyn Signer,
    ) -> Result<(Pubkey, Pubkey)> {
        let mint = Keypair::new();
        let token = get_associated_token_address(&recipient.pubkey(), &mint.pubkey());

        let space = Mint::LEN;
        let lamports = self.client.get_minimum_balance_for_rent_exemption(space)?;
        let ixs = &[
            system_instruction::create_account(
                &recipient.pubkey(),
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
                0,
            )
            .unwrap(),
            create_associated_token_account(
                &recipient.pubkey(),
                &recipient.pubkey(),
                &mint.pubkey(),
            ),
            spl_instruction::mint_to(
                &spl_token::ID,
                &mint.pubkey(),
                &token,
                &owner.pubkey(),
                &[],
                1,
            )
            .unwrap(),
        ];

        self.run_transaction(ixs, recipient.pubkey(), &[&mint, recipient, owner])?;
        Ok((mint.pubkey(), token))
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

    pub fn get_or_create_token_account(
        &self,
        payer: &dyn Signer,
        owner: Pubkey,
        mint: Pubkey,
    ) -> Result<Pubkey> {
        if let Some(found_token_pubkey) = self.find_token_account(owner, mint)? {
            return Ok(found_token_pubkey);
        }

        let token_pubkey = get_associated_token_address(&owner, &mint);
        let ix = create_associated_token_account(&payer.pubkey(), &owner, &mint);
        self.run_transaction(&[ix], payer.pubkey(), &[payer])?;
        Ok(token_pubkey)
    }

    pub fn find_token_account(&self, address: Pubkey, mint: Pubkey) -> Result<Option<Pubkey>> {
        let filter = TokenAccountsFilter::Mint(mint);
        let token_accounts = self.client.get_token_accounts_by_owner(&address, filter)?;
        if token_accounts.is_empty() {
            return Ok(None);
        }

        let associated_token_pubkey = get_associated_token_address(&address, &mint);
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

    pub fn transfer_tokens(
        &self,
        owner: &dyn Signer,
        mint: Pubkey,
        recipient: Pubkey,
        amount: u64,
    ) -> Result<Signature> {
        let current_balance = self.token_balance(owner.pubkey(), mint)?;
        if amount > current_balance {
            let decimals = self.mint_account(mint)?.decimals;
            let expected = amount_to_ui_amount(amount, decimals);
            let found = amount_to_ui_amount(current_balance, decimals);
            return Err(CustomClientError::NotEnoughTokens(expected, found).into());
        }

        let owner_token_pubkey = get_associated_token_address(&owner.pubkey(), &mint);
        let recipient_token_account = self.get_or_create_token_account(owner, recipient, mint)?;
        let mut ixs = Vec::new();

        if let Some(ix) = self.try_set_primary_sale_and_update_creators_ix(owner, mint, recipient) {
            ixs.push(ix);
        }

        ixs.push(
            spl_token::instruction::transfer(
                &spl_token::ID,
                &owner_token_pubkey,
                &recipient_token_account,
                &owner.pubkey(),
                &[],
                amount,
            )
            .unwrap(),
        );

        self.run_transaction(&ixs, owner.pubkey(), &[owner])
    }

    fn try_set_primary_sale_and_update_creators_ix(
        &self,
        owner: &dyn Signer,
        nft_mint: Pubkey,
        recipient: Pubkey,
    ) -> Option<Instruction> {
        let metadata_result = self.metadata_account(nft_mint);

        if metadata_result.is_err() {
            return None;
        }

        let metadata = metadata_result.unwrap();
        if metadata.token_standard != Some(TokenStandard::NonFungible)
            || owner.pubkey() == recipient
            || metadata.update_authority != owner.pubkey()
            || metadata.primary_sale_happened
        {
            return None;
        }

        let creators = Some(vec![
            Creator {
                address: owner.pubkey(),
                verified: true,
                share: AUTHORITY_SHARE,
            },
            Creator {
                address: recipient,
                verified: false,
                share: 100 - AUTHORITY_SHARE,
            },
        ]);

        let data = DataV2 {
            name: metadata.data.name,
            symbol: metadata.data.symbol,
            uri: metadata.data.uri,
            seller_fee_basis_points: metadata.data.seller_fee_basis_points,
            creators,
            collection: metadata.collection,
            uses: metadata.uses,
        };

        Some(
            mpl_token_metadata::instruction::update_metadata_accounts_v2(
                mpl_token_metadata::ID,
                pda::metadata(&nft_mint),
                owner.pubkey(),
                None,
                Some(data),
                Some(true),
                None,
            ),
        )
    }

    //
    // Program instructions
    //

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
                        self.get_or_create_token_account(user, recipient.address, mint_chill)?;
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

#[cfg(test)]
mod tests {
    use chill_api::state::NftType;

    use super::*;

    #[test]
    fn transfer_updates_nft() {
        let client = Client::init("http://localhost:8899");
        let owner = Keypair::new();
        client.airdrop(owner.pubkey(), 1_000_000_000).unwrap();

        let program_id = chill_api::ID;
        let mint_chill = client.create_mint(&owner, 9).unwrap();
        let (nft_mint, nft_token) = client.create_mint_and_token_nft(&owner, &owner).unwrap();
        let owner_chill_account = client
            .get_or_create_token_account(&owner, owner.pubkey(), mint_chill)
            .unwrap();

        let recipients = Vec::new();
        let fees = Fees::default();
        client
            .initialize(program_id, &owner, mint_chill, fees, recipients)
            .unwrap();

        let args = MintNftArgs {
            nft_type: NftType::Pet,
            name: "Name".to_owned(),
            symbol: "Symbol".to_owned(),
            url: "Url".to_owned(),
            fees: 0,
        };

        client
            .mint_nft(
                program_id,
                &owner,
                &owner,
                mint_chill,
                owner_chill_account,
                nft_mint,
                nft_token,
                args,
            )
            .unwrap();

        let owner_token_pubkey = get_associated_token_address(&owner.pubkey(), &nft_mint);
        let owner_token_account = client.token_account(owner_token_pubkey).unwrap();
        assert_eq!(owner_token_account.amount, 1);

        let metadata = client.metadata_account(nft_mint).unwrap();
        let creators = metadata.data.creators.unwrap();
        assert!(!metadata.primary_sale_happened);
        assert_eq!(creators.len(), 1);
        assert_eq!(creators[0].address, owner.pubkey());

        let recipient = Keypair::new();
        client.airdrop(recipient.pubkey(), 1_000_000_000).unwrap();

        client
            .transfer_tokens(&owner, nft_mint, recipient.pubkey(), 1)
            .unwrap();

        let owner_token_account = client.token_account(owner_token_pubkey).unwrap();
        assert_eq!(owner_token_account.amount, 0);

        let recipient_token_pubkey = get_associated_token_address(&recipient.pubkey(), &nft_mint);

        let recipient_token_account = client.token_account(recipient_token_pubkey).unwrap();
        assert_eq!(recipient_token_account.amount, 1);

        let metadata = client.metadata_account(nft_mint).unwrap();
        let creators = metadata.data.creators.unwrap();
        assert!(metadata.primary_sale_happened);
        assert_eq!(creators.len(), 2);
        assert_eq!(creators[0].address, owner.pubkey());
        assert_eq!(creators[0].share, AUTHORITY_SHARE);
        assert_eq!(creators[1].address, recipient.pubkey());
        assert_eq!(creators[1].share, 100 - AUTHORITY_SHARE);

        let new_recipient = Keypair::new();
        client
            .transfer_tokens(&recipient, nft_mint, new_recipient.pubkey(), 1)
            .unwrap();

        let recipient_token_account = client.token_account(recipient_token_pubkey).unwrap();
        assert_eq!(recipient_token_account.amount, 0);

        let new_recipient_token_pubkey =
            get_associated_token_address(&new_recipient.pubkey(), &nft_mint);

        let new_recipient_token_account = client.token_account(new_recipient_token_pubkey).unwrap();
        assert_eq!(new_recipient_token_account.amount, 1);

        let metadata = client.metadata_account(nft_mint).unwrap();
        assert!(metadata.primary_sale_happened);
        assert_eq!(metadata.data.creators.unwrap(), creators);
    }
}
