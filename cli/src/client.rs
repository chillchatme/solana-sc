use crate::{
    error::{CliError, Result},
    pda,
};
use anchor_client::{
    anchor_lang::AccountDeserialize,
    solana_client::{rpc_client::RpcClient, rpc_request::TokenAccountsFilter},
    solana_sdk::{
        commitment_config::CommitmentConfig,
        instruction::{AccountMeta, Instruction},
        program_pack::Pack,
        pubkey::Pubkey,
        rent::Rent,
        signature::{Keypair, Signature},
        signer::Signer,
        signers::Signers,
        system_instruction, system_program,
        sysvar::SysvarId,
        transaction::Transaction,
    },
    Client as AnchorClient, Cluster, Program,
};
use anchor_spl::associated_token;
use chill_nft::{
    self,
    state::{ChillNftMetadata, Config, Fees, NftType, Recipient, AUTHORITY_SHARE},
    utils::NftArgs,
};
use mpl_token_metadata::{
    state::{Creator, DataV2, Key, Metadata, TokenStandard, MAX_METADATA_LEN},
    utils::try_from_slice_checked,
};
use spl_associated_token_account::{create_associated_token_account, get_associated_token_address};
use spl_token::{
    amount_to_ui_amount, instruction as spl_instruction,
    state::{Account, Mint},
};
use std::{convert::TryInto, rc::Rc, str::FromStr};

pub struct Client {
    url: String,
    commitment: CommitmentConfig,
    rpc_client: RpcClient,
}

impl Client {
    pub fn init(url: &str) -> Self {
        let commitment = CommitmentConfig::confirmed();

        Self {
            url: url.to_string(),
            commitment,
            rpc_client: RpcClient::new_with_commitment(url, commitment),
        }
    }

    pub fn program(&self, payer: Rc<dyn Signer>, program_id: Pubkey) -> Result<Program> {
        let cluster = Cluster::from_str(&self.url)?;
        let anchor_client = AnchorClient::new_with_options(cluster, payer, self.commitment);
        Ok(anchor_client.program(program_id))
    }

    pub fn rpc(&self) -> RpcClient {
        RpcClient::new_with_commitment(&self.url, self.commitment)
    }

    fn run_transaction(
        &self,
        instructions: &[Instruction],
        payer: Pubkey,
        signers: &impl Signers,
    ) -> Result<Signature> {
        let blockhash = self.rpc_client.get_latest_blockhash()?;
        let transaction =
            Transaction::new_signed_with_payer(instructions, Some(&payer), signers, blockhash);
        self.rpc_client
            .send_and_confirm_transaction(&transaction)
            .map_err(|e| e.into())
    }

    pub fn airdrop(&self, address: Pubkey, lamports: u64) -> Result<()> {
        let signature = self.rpc_client.request_airdrop(&address, lamports)?;
        let blockhash = self.rpc_client.get_latest_blockhash()?;
        self.rpc_client
            .confirm_transaction_with_spinner(&signature, &blockhash, CommitmentConfig::confirmed())
            .map_err(|e| e.into())
    }

    pub fn balance(&self, address: Pubkey) -> Result<u64> {
        self.rpc_client.get_balance(&address).map_err(|e| e.into())
    }

    //
    // Accounts
    //

    pub fn account_data(&self, address: Pubkey) -> Result<Vec<u8>> {
        self.rpc_client
            .get_account_data(&address)
            .map_err(|e| e.into())
    }

    pub fn mint_account(&self, address: Pubkey) -> Result<Mint> {
        let data = self
            .rpc_client
            .get_account_data(&address)
            .map_err(|_| CliError::MintNotFound(address))?;
        let mint = Mint::unpack(&data).map_err(|_| CliError::AccountIsNotMint)?;
        Ok(mint)
    }

    pub fn token_account(&self, address: Pubkey) -> Result<Account> {
        let data = self
            .rpc_client
            .get_account_data(&address)
            .map_err(|_| CliError::TokenNotInitialized(address))?;

        let token_account = Account::unpack(&data).map_err(|_| CliError::AccountIsNotToken)?;

        Ok(token_account)
    }

    pub fn metadata_account(&self, mint: Pubkey) -> Result<Metadata> {
        let metadata_pubkey = pda::metadata(mint);
        let data = self
            .rpc_client
            .get_account_data(&metadata_pubkey)
            .map_err(|_| CliError::MetadataNotFound(mint))?;

        try_from_slice_checked(&data, Key::MetadataV1, MAX_METADATA_LEN)
            .map_err(|_| CliError::AccountIsNotMetadata.into())
    }

    pub fn config(&self, mint: Pubkey) -> Result<Config> {
        let config_pubkey = pda::config(mint);

        let config_data = self
            .rpc_client
            .get_account_data(&config_pubkey)
            .map_err(|_| CliError::ConfigNotFound)?;

        Config::try_deserialize(&mut config_data.as_ref())
            .map_err(|_| CliError::ConfigDataError.into())
    }

    pub fn chill_metadata(&self, nft_mint: Pubkey) -> Result<ChillNftMetadata> {
        let chill_metadata_pubkey = pda::chill_metadata(nft_mint);
        let chill_metadata_data = self
            .rpc_client
            .get_account_data(&chill_metadata_pubkey)
            .map_err(|_| CliError::ChillMetadataNotFound)?;

        ChillNftMetadata::try_deserialize(&mut chill_metadata_data.as_ref())
            .map_err(|_| CliError::ChillMetadataDataError.into())
    }

    //
    // Mint & Token accounts functions
    //

    pub fn create_mint_and_token_nft(
        &self,
        primary_wallet: Rc<dyn Signer>,
        payer: Rc<dyn Signer>,
        recipient: Pubkey,
    ) -> Result<(Pubkey, Pubkey)> {
        let mint = Keypair::new();
        let token = get_associated_token_address(&recipient, &mint.pubkey());

        let space = Mint::LEN;
        let lamports = self
            .rpc_client
            .get_minimum_balance_for_rent_exemption(space)?;

        let ixs = &[
            system_instruction::create_account(
                &payer.pubkey(),
                &mint.pubkey(),
                lamports,
                space.try_into().unwrap(),
                &spl_token::ID,
            ),
            spl_instruction::initialize_mint(
                &spl_token::ID,
                &mint.pubkey(),
                &primary_wallet.pubkey(),
                None,
                0,
            )
            .unwrap(),
            create_associated_token_account(&payer.pubkey(), &recipient, &mint.pubkey()),
            spl_instruction::mint_to(
                &spl_token::ID,
                &mint.pubkey(),
                &token,
                &primary_wallet.pubkey(),
                &[],
                1,
            )
            .unwrap(),
        ];

        self.run_transaction(
            ixs,
            payer.pubkey(),
            &[&mint, payer.as_ref(), primary_wallet.as_ref()],
        )?;

        Ok((mint.pubkey(), token))
    }

    pub fn create_mint(
        &self,
        authority: Rc<dyn Signer>,
        payer: Rc<dyn Signer>,
        decimals: u8,
    ) -> Result<Pubkey> {
        let mint = Keypair::new();
        let space = Mint::LEN;
        let lamports = self
            .rpc_client
            .get_minimum_balance_for_rent_exemption(space)?;

        let ixs = &[
            system_instruction::create_account(
                &payer.pubkey(),
                &mint.pubkey(),
                lamports,
                space.try_into().unwrap(),
                &spl_token::ID,
            ),
            spl_instruction::initialize_mint(
                &spl_token::ID,
                &mint.pubkey(),
                &authority.pubkey(),
                None,
                decimals,
            )
            .unwrap(),
        ];
        self.run_transaction(ixs, payer.pubkey(), &[payer.as_ref(), &mint])?;

        Ok(mint.pubkey())
    }

    pub fn mint_to(
        &self,
        authority: Rc<dyn Signer>,
        payer: Rc<dyn Signer>,
        mint: Pubkey,
        token: Pubkey,
        amount: u64,
    ) -> Result<()> {
        let ix = spl_instruction::mint_to(
            &spl_token::ID,
            &mint,
            &token,
            &authority.pubkey(),
            &[],
            amount,
        )
        .unwrap();

        self.run_transaction(&[ix], payer.pubkey(), &[authority.as_ref(), payer.as_ref()])?;
        Ok(())
    }

    pub fn get_or_create_token_account(
        &self,
        owner: Pubkey,
        mint: Pubkey,
        payer: Rc<dyn Signer>,
    ) -> Result<Pubkey> {
        if let Some(found_token_pubkey) = self.find_token_address(owner, mint)? {
            return Ok(found_token_pubkey);
        }

        let token_pubkey = get_associated_token_address(&owner, &mint);
        let ix = create_associated_token_account(&payer.pubkey(), &owner, &mint);
        self.run_transaction(&[ix], payer.pubkey(), &[payer.as_ref()])?;
        Ok(token_pubkey)
    }

    pub fn find_token_address(&self, address: Pubkey, mint: Pubkey) -> Result<Option<Pubkey>> {
        let filter = TokenAccountsFilter::Mint(mint);
        let token_accounts = self
            .rpc_client
            .get_token_accounts_by_owner(&address, filter)?;

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
        let token_accounts = self
            .rpc_client
            .get_token_accounts_by_owner(&owner, filter)?;
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
        from: Rc<dyn Signer>,
        payer: Rc<dyn Signer>,
        mint: Pubkey,
        recipient: Pubkey,
        amount: u64,
    ) -> Result<Signature> {
        let current_balance = self.token_balance(from.pubkey(), mint)?;
        if amount > current_balance {
            let decimals = self.mint_account(mint)?.decimals;
            let expected = amount_to_ui_amount(amount, decimals);
            let found = amount_to_ui_amount(current_balance, decimals);
            return Err(CliError::NotEnoughTokens(expected, found).into());
        }

        let authority_token_pubkey = self.find_token_address(from.pubkey(), mint)?.unwrap();
        let recipient_token_account =
            self.get_or_create_token_account(recipient, mint, payer.clone())?;

        let mut ixs = Vec::new();
        if let Some(ix) =
            self.try_set_primary_sale_and_update_creators_ix(from.clone(), mint, recipient)
        {
            ixs.push(ix);
        }

        ixs.push(
            spl_token::instruction::transfer(
                &spl_token::ID,
                &authority_token_pubkey,
                &recipient_token_account,
                &from.pubkey(),
                &[],
                amount,
            )
            .unwrap(),
        );

        self.run_transaction(&ixs, payer.pubkey(), &[from.as_ref(), payer.as_ref()])
    }

    fn try_set_primary_sale_and_update_creators_ix(
        &self,
        authority: Rc<dyn Signer>,
        nft_mint: Pubkey,
        recipient: Pubkey,
    ) -> Option<Instruction> {
        let metadata_result = self.metadata_account(nft_mint);

        if metadata_result.is_err() {
            return None;
        }

        let metadata = metadata_result.unwrap();
        if metadata.token_standard != Some(TokenStandard::NonFungible)
            || authority.pubkey() == recipient
            || metadata.update_authority != authority.pubkey()
            || metadata.primary_sale_happened
        {
            return None;
        }

        let creators = Some(vec![
            Creator {
                address: authority.pubkey(),
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
                pda::metadata(nft_mint),
                authority.pubkey(),
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
        primary_wallet: Rc<dyn Signer>,
        payer: Rc<dyn Signer>,
        chill_mint: Pubkey,
        fees: Fees,
        recipients: Vec<Recipient>,
    ) -> Result<Signature> {
        let program = self.program(payer.clone(), chill_nft::ID)?;
        let config = pda::config(chill_mint);

        program
            .request()
            .args(chill_nft::instruction::Initialize { fees, recipients })
            .accounts(chill_nft::accounts::Initialize {
                primary_wallet: primary_wallet.pubkey(),
                payer: payer.pubkey(),
                config,
                chill_mint,
                system_program: system_program::id(),
            })
            .send()
            .map_err(Into::into)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn mint_nft(
        &self,
        primary_wallet: Rc<dyn Signer>,
        payer: Rc<dyn Signer>,
        chill_mint: Pubkey,
        creator: Option<Pubkey>,
        nft_mint: Pubkey,
        nft_type: NftType,
        args: NftArgs,
    ) -> Result<Signature> {
        let config = self.config(chill_mint)?;
        let mut recipients_token_accounts = Vec::with_capacity(config.recipients.len());
        for recipient in config.recipients {
            match self.find_token_address(recipient.address, chill_mint)? {
                Some(token_address) => recipients_token_accounts.push(AccountMeta {
                    pubkey: token_address,
                    is_signer: false,
                    is_writable: true,
                }),
                None => {
                    let token_address = self.get_or_create_token_account(
                        recipient.address,
                        chill_mint,
                        payer.clone(),
                    )?;

                    recipients_token_accounts.push(AccountMeta {
                        pubkey: token_address,
                        is_signer: false,
                        is_writable: true,
                    });
                }
            };
        }

        let program = self.program(payer.clone(), chill_nft::ID)?;
        let config_pubkey = pda::config(chill_mint);

        let nft_metadata = pda::metadata(nft_mint);
        let nft_master_edition = pda::master_edition(nft_mint);
        let nft_chill_metadata = pda::chill_metadata(nft_mint);

        let primary_wallet_token = self
            .find_token_address(primary_wallet.pubkey(), chill_mint)?
            .ok_or_else(|| CliError::TokenAccountNotFound(primary_wallet.pubkey()))?;

        program
            .request()
            .args(chill_nft::instruction::MintNft {
                nft_type,
                args,
                creator,
            })
            .accounts(chill_nft::accounts::MintNft {
                primary_wallet: primary_wallet.pubkey(),
                payer: payer.pubkey(),
                chill_payer: primary_wallet.pubkey(),
                chill_payer_token_account: primary_wallet_token,
                config: config_pubkey,
                chill_mint,
                nft_mint,
                nft_metadata,
                nft_master_edition,
                nft_chill_metadata,
                rent: Rent::id(),
                system_program: system_program::ID,
                token_program: spl_token::ID,
                token_metadata_program: mpl_token_metadata::ID,
            })
            .accounts(recipients_token_accounts)
            .signer(primary_wallet.as_ref())
            .send()
            .map_err(Into::into)
    }

    pub fn update_nft(
        &self,
        payer: Rc<dyn Signer>,
        primary_wallet: Rc<dyn Signer>,
        nft_mint: Pubkey,
        args: NftArgs,
    ) -> Result<Signature> {
        let program = self.program(payer.clone(), chill_nft::ID)?;
        let nft_metadata = pda::metadata(nft_mint);

        program
            .request()
            .args(chill_nft::instruction::UpdateNft { args })
            .accounts(chill_nft::accounts::UpdateNft {
                primary_wallet: primary_wallet.pubkey(),
                nft_metadata,
                token_metadata_program: mpl_token_metadata::ID,
            })
            .signer(primary_wallet.as_ref())
            .send()
            .map_err(Into::into)
    }

    pub fn create_wallet(
        &self,
        payer: Rc<dyn Signer>,
        account: Pubkey,
        proxy_wallet: Pubkey,
        primary_wallet: Pubkey,
    ) -> Result<Signature> {
        let program = self.program(payer.clone(), chill_wallet::ID)?;

        program
            .request()
            .args(chill_wallet::instruction::CreateWallet)
            .accounts(chill_wallet::accounts::CreateWallet {
                primary_wallet,
                user: account,
                payer: payer.pubkey(),
                proxy_wallet,
                system_program: system_program::ID,
            })
            .send()
            .map_err(Into::into)
    }

    pub fn withdraw_lamports(
        &self,
        payer: Rc<dyn Signer>,
        authority: Rc<dyn Signer>,
        proxy_wallet: Pubkey,
        recipient: Pubkey,
        amount: u64,
    ) -> Result<Signature> {
        let program = self.program(payer.clone(), chill_wallet::ID)?;

        program
            .request()
            .args(chill_wallet::instruction::WithdrawLamports { amount })
            .accounts(chill_wallet::accounts::WithdrawLamports {
                authority: authority.pubkey(),
                proxy_wallet,
                receiver: recipient,
            })
            .signer(authority.as_ref())
            .send()
            .map_err(Into::into)
    }

    pub fn withdraw_ft(
        &self,
        payer: Rc<dyn Signer>,
        authority: Rc<dyn Signer>,
        proxy_wallet: Pubkey,
        recipient: Pubkey,
        mint: Pubkey,
        amount: u64,
    ) -> Result<Signature> {
        let program = self.program(payer.clone(), chill_wallet::ID)?;

        let proxy_wallet_token_account = self
            .find_token_address(proxy_wallet, mint)?
            .ok_or(CliError::TokenAccountNotFound(proxy_wallet))?;

        let receiver_token_account = self.get_or_create_token_account(recipient, mint, payer)?;

        program
            .request()
            .args(chill_wallet::instruction::WithdrawFt { amount })
            .accounts(chill_wallet::accounts::WithdrawFt {
                authority: authority.pubkey(),
                proxy_wallet,
                mint,
                proxy_wallet_token_account,
                receiver_token_account,
                token_program: spl_token::ID,
            })
            .signer(authority.as_ref())
            .send()
            .map_err(Into::into)
    }

    pub fn withdraw_nft(
        &self,
        payer: Rc<dyn Signer>,
        authority: Rc<dyn Signer>,
        proxy_wallet: Pubkey,
        recipient: Pubkey,
        nft_mint: Pubkey,
    ) -> Result<Signature> {
        let program = self.program(payer.clone(), chill_wallet::ID)?;

        let proxy_wallet_token_account = self
            .find_token_address(proxy_wallet, nft_mint)?
            .ok_or(CliError::TokenAccountNotFound(proxy_wallet))?;

        let receiver_token_account =
            self.get_or_create_token_account(recipient, nft_mint, payer)?;

        program
            .request()
            .args(chill_wallet::instruction::WithdrawNft)
            .accounts(chill_wallet::accounts::WithdrawNft {
                authority: authority.pubkey(),
                proxy_wallet,
                nft_mint,
                proxy_wallet_token_account,
                receiver_token_account,
                token_program: spl_token::ID,
            })
            .signer(authority.as_ref())
            .send()
            .map_err(Into::into)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn staking_initialize(
        &self,
        staking_info: &Keypair,
        primary_wallet: Rc<dyn Signer>,
        payer: Rc<dyn Signer>,
        mint: Pubkey,
        start_time: u64,
        end_time: u64,
        min_stake_size: u64,
    ) -> Result<Signature> {
        let program_id = chill_staking::ID;
        let program = self.program(payer.clone(), program_id)?;

        let staking_token_authority = pda::staking_token_authority(staking_info.pubkey());
        let staking_token_account = get_associated_token_address(&staking_token_authority, &mint);

        let args = chill_staking::InitializeArgs {
            start_time,
            end_time,
            min_stake_size,
        };

        program
            .request()
            .args(chill_staking::instruction::Initialize { args })
            .accounts(chill_staking::accounts::Initialize {
                primary_wallet: primary_wallet.pubkey(),
                payer: payer.pubkey(),
                staking_info: staking_info.pubkey(),
                staking_token_authority,
                staking_token_account,
                mint,
                system_program: system_program::ID,
                rent: Rent::id(),
                token_program: spl_token::ID,
                associated_token_program: associated_token::ID,
            })
            .signer(primary_wallet.as_ref())
            .signer(staking_info)
            .send()
            .map_err(Into::into)
    }

    pub fn staking_add_token_reward(
        &self,
        primary_wallet: Rc<dyn Signer>,
        payer: Rc<dyn Signer>,
        staking_info: Pubkey,
        mint: Pubkey,
        amount: u64,
    ) -> Result<Signature> {
        let program_id = chill_staking::ID;
        let program = self.program(payer.clone(), program_id)?;

        let primary_wallet_token_account = self
            .find_token_address(primary_wallet.pubkey(), mint)?
            .ok_or_else(|| CliError::TokenAccountNotFound(primary_wallet.pubkey()))?;

        let staking_token_authority = pda::staking_token_authority(staking_info);
        let staking_token_account = get_associated_token_address(&staking_token_authority, &mint);

        program
            .request()
            .args(chill_staking::instruction::AddRewardTokens { amount })
            .accounts(chill_staking::accounts::AddRewardTokens {
                primary_wallet: primary_wallet.pubkey(),
                token_account_authority: primary_wallet.pubkey(),
                token_account: primary_wallet_token_account,
                staking_info,
                staking_token_authority,
                staking_token_account,
                token_program: spl_token::ID,
            })
            .signer(primary_wallet.as_ref())
            .send()
            .map_err(Into::into)
    }
}
