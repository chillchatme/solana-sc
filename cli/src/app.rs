use crate::{
    cli::{Cli, CliCommand},
    client::Client,
    error::{AppError, CliError, Result},
    pda,
};
use anchor_client::{
    solana_sdk::{
        native_token::sol_to_lamports,
        program_option::COption,
        pubkey::Pubkey,
        signature::{Keypair, Signature},
        signer::Signer,
    },
    Cluster,
};
use chill_nft::state::Fees;
use colored::Colorize;
use spl_token::native_mint;
use std::{fs, io::Write, path::Path, process::exit, rc::Rc};

pub struct App<'cli> {
    cli: Cli<'cli>,
    client: Client,
}

impl App<'_> {
    pub fn init() -> Self {
        let cli = Cli::init();
        let client = Client::init(&cli.rpc_url());

        App { cli, client }
    }

    fn on_error(&self, error: AppError) -> ! {
        println!("{}", error);
        exit(1);
    }

    fn try_to_airdrop(&self, address: Pubkey) -> Result<()> {
        if self.client.balance(address)? == 0 {
            if self.cli.cluster() == Cluster::Mainnet {
                println!("{}", "You have to top up your balance".red());
                exit(0);
            } else {
                self.client.airdrop(address, sol_to_lamports(1.0))?;
            }
        }

        Ok(())
    }

    fn save_mint(&self, mint: Pubkey) -> Result<()> {
        let save_path = self.cli.save_path();
        let mint = mint.to_string();

        std::fs::write(save_path, mint)
            .map_err(|_| CliError::CannotWriteToFile(save_path.to_owned()))?;

        let path = Path::new(save_path);
        let full_path = fs::canonicalize(path).unwrap();
        let full_path_str = full_path.as_os_str().to_str().unwrap();
        println!("{} \"{}\"", "Mint file:".cyan(), full_path_str);
        Ok(())
    }

    fn assert_mint_authority(&self, mint: Pubkey, authority: Pubkey) -> Result<()> {
        let mint_account = self.client.mint_account(mint)?;
        if mint_account.mint_authority != COption::Some(authority) {
            Err(CliError::AuthorityNotMatch(mint).into())
        } else {
            Ok(())
        }
    }

    fn get_mint(&self) -> Result<Pubkey> {
        match self.cli.mint()? {
            Some(mint) => Ok(mint),
            None => Err(CliError::MintNotSpecified.into()),
        }
    }

    fn get_or_create_mint(
        &self,
        authority: Rc<dyn Signer>,
        payer: Rc<dyn Signer>,
        decimals: u8,
    ) -> Result<Pubkey> {
        if let Some(mint) = self.cli.mint()? {
            self.assert_mint_authority(mint, authority.pubkey())?;
            return Ok(mint);
        }

        let save_path = self.cli.save_path();
        let path = Path::new(save_path);
        if path.exists() {
            let full_path = fs::canonicalize(path).unwrap();
            let full_path_str = full_path.as_os_str().to_str().unwrap();
            return Err(CliError::MintFileExists(full_path_str.to_owned()).into());
        }

        let mint = self.client.create_mint(authority, payer, decimals)?;
        println!("{} {}", "Mint:".cyan(), mint);

        self.save_mint(mint)?;
        Ok(mint)
    }

    fn print_signature(&self, signature: &Signature) {
        println!("{} {}", "Signature:".cyan(), signature);
    }

    fn print_balance(&self, address: Pubkey, mint: Pubkey) -> Result<()> {
        let balance = self.client.ui_token_balance(address, mint)?;
        println!("{} {} tokens", "Balance:".green().bold(), balance);

        Ok(())
    }

    fn print_info(&self, mint: Pubkey, program_id: Pubkey) -> Result<()> {
        let config = self.client.config(mint, program_id)?;
        let mint_account = self.client.mint_account(mint)?;

        println!(
            "{0} {1}",
            "Authority:".green().bold(),
            mint_account.mint_authority.unwrap()
        );

        let fees = config.fees.to_ui(mint_account.decimals);
        println!("\n{0}", "======= MINT FEES =======".cyan().bold());
        println!("{0:>10} {1}", "Character:".cyan(), fees.character);
        println!("{0:>10} {1}", "Pet:".cyan(), fees.pet);
        println!("{0:>10} {1}", "Emote:".cyan(), fees.emote);
        println!("{0:>10} {1}", "Tileset:".cyan(), fees.tileset);
        println!("{0:>10} {1}", "Item:".cyan(), fees.item);
        println!("{0:>10} {1}", "World:".cyan(), fees.world);

        let recipients = config.recipients;
        if !recipients.is_empty() {
            println!("\n{0}", "======= RECIPIENTS =======".bright_blue().bold());
            let recipients_info = recipients
                .iter()
                .map(|r| {
                    format!(
                        "{0} {1}\n{2} {3}%\n{4} {5}%\n\n",
                        "Address:".bright_blue(),
                        r.address,
                        "Mint share:".bright_blue(),
                        r.mint_share,
                        "Transaction share:".bright_blue(),
                        r.transaction_share
                    )
                })
                .collect::<String>();

            println!("{}", recipients_info.trim());
        }

        Ok(())
    }

    fn process_mint(&self) -> Result<()> {
        let primary_wallet = self.cli.primary_wallet()?;
        let payer = self.cli.payer()?;
        let recipient = self.cli.recipient();

        self.try_to_airdrop(payer.pubkey())?;

        let decimals = self.cli.decimals();
        let mint = self.get_or_create_mint(primary_wallet.clone(), payer.clone(), decimals)?;

        let token_account_pubkey =
            self.client
                .get_or_create_token_account(recipient, mint, payer.clone())?;

        let ui_amount = self.cli.ui_amount();
        let amount = spl_token::ui_amount_to_amount(ui_amount, decimals);

        self.client
            .mint_to(primary_wallet, payer, mint, token_account_pubkey, amount)?;

        self.print_balance(recipient, mint)
    }

    fn process_mint_nft(&self) -> Result<()> {
        let payer = self.cli.payer()?;
        let primary_wallet = self.cli.primary_wallet()?;
        let recipient = self.cli.recipient();
        let creator = self.cli.creator();

        self.try_to_airdrop(payer.pubkey())?;

        let mint_chill = self.get_mint()?;
        let args = self.cli.mint_args()?;
        let nft_type = self.cli.nft_type();
        let program_id = self.cli.nft_program_id();

        let (nft_mint, _nft_token) = self.client.create_mint_and_token_nft(
            primary_wallet.clone(),
            payer.clone(),
            recipient,
        )?;

        println!("{0} {1}", "NFT Mint:".green(), nft_mint);

        let signature = self.client.mint_nft(
            primary_wallet,
            payer,
            mint_chill,
            creator,
            nft_mint,
            nft_type,
            args,
            program_id,
        )?;

        self.print_signature(&signature);

        Ok(())
    }

    fn process_update_nft(&self) -> Result<()> {
        let payer = self.cli.payer()?;
        let primary_wallet = self.cli.primary_wallet()?;
        let nft_mint = self.get_mint()?;
        let args = self.cli.mint_args()?;
        let program_id = self.cli.nft_program_id();

        let signature =
            self.client
                .update_nft(payer, primary_wallet, nft_mint, args, program_id)?;

        self.print_signature(&signature);

        Ok(())
    }

    fn process_print_info(&self) -> Result<()> {
        let mint = self.get_mint()?;
        let program_id = self.cli.nft_program_id();
        self.print_info(mint, program_id)
    }

    fn process_print_balance(&self) -> Result<()> {
        let account = self.cli.account();
        let mint = self.get_mint()?;
        self.print_balance(account, mint)
    }

    fn process_transfer(&self) -> Result<()> {
        let primary_wallet = self.cli.primary_wallet()?;
        let payer = self.cli.payer()?;
        let mint = self.get_mint()?;

        let ui_amount = self.cli.ui_amount();
        let recipient = self.cli.recipient();

        if ui_amount == 0.0 {
            return Err(CliError::TransferZeroTokens.into());
        }

        let current_balance = self
            .client
            .ui_token_balance(primary_wallet.pubkey(), mint)?;

        if ui_amount > current_balance {
            return Err(CliError::InsufficientTokens(ui_amount, current_balance).into());
        }

        let mint_account = self.client.mint_account(mint)?;
        let decimals = mint_account.decimals;
        let amount = spl_token::ui_amount_to_amount(ui_amount, decimals);

        let primary_wallet_pubkey = primary_wallet.pubkey();
        let signature =
            self.client
                .transfer_tokens(primary_wallet, payer, mint, recipient, amount)?;

        self.print_signature(&signature);
        self.print_balance(primary_wallet_pubkey, mint)
    }

    pub fn process_nft_initialize(&self) -> Result<()> {
        let payer = self.cli.payer()?;
        let primary_wallet = self.cli.primary_wallet()?;
        let mint = self.get_mint()?;
        let program_id = self.cli.nft_program_id();

        self.assert_mint_authority(mint, primary_wallet.pubkey())?;

        let ui_fees = self.cli.fees();

        let recipients = self.cli.multiple_recipients()?;
        let mint_account = self.client.mint_account(mint)?;
        let fees = Fees::from_ui(ui_fees, mint_account.decimals);

        self.client
            .initialize(primary_wallet, payer, mint, fees, recipients, program_id)?;

        self.print_info(mint, program_id)
    }

    pub fn process_create_wallet(&self) -> Result<()> {
        let payer = self.cli.payer()?;
        let primary_wallet = self.cli.primary_wallet_pubkey();
        let account = self.cli.account();
        let program_id = self.cli.wallet_program_id();

        let proxy_wallet = pda::proxy_wallet(account, primary_wallet, program_id);

        println!("{} {}", "Wallet:".green(), proxy_wallet);

        let signature =
            self.client
                .create_wallet(payer, account, proxy_wallet, primary_wallet, program_id)?;

        self.print_signature(&signature);

        Ok(())
    }

    pub fn process_withdraw_lamports(&self) -> Result<()> {
        let account = self.cli.account();
        let authority = self.cli.authority()?;
        let payer = self.cli.payer()?;
        let primary_wallet = self.cli.primary_wallet_pubkey();
        let recipient = self.cli.recipient();
        let program_id = self.cli.wallet_program_id();

        let ui_amount = self.cli.ui_amount();
        let amount = spl_token::ui_amount_to_amount(ui_amount, native_mint::DECIMALS);

        let proxy_wallet = pda::proxy_wallet(account, primary_wallet, program_id);

        let signature = self.client.withdraw_lamports(
            payer,
            authority,
            proxy_wallet,
            recipient,
            amount,
            program_id,
        )?;

        self.print_signature(&signature);

        Ok(())
    }

    pub fn process_withdraw_ft(&self) -> Result<()> {
        let account = self.cli.account();
        let authority = self.cli.authority()?;
        let payer = self.cli.payer()?;
        let primary_wallet = self.cli.primary_wallet_pubkey();
        let recipient = self.cli.recipient();
        let mint = self.get_mint()?;
        let program_id = self.cli.wallet_program_id();

        let ui_amount = self.cli.ui_amount();
        let amount = spl_token::ui_amount_to_amount(ui_amount, native_mint::DECIMALS);

        let proxy_wallet = pda::proxy_wallet(account, primary_wallet, program_id);

        let signature = self.client.withdraw_ft(
            payer,
            authority,
            proxy_wallet,
            recipient,
            mint,
            amount,
            program_id,
        )?;

        self.print_signature(&signature);

        Ok(())
    }

    pub fn process_withdraw_nft(&self) -> Result<()> {
        let account = self.cli.account();
        let authority = self.cli.authority()?;
        let payer = self.cli.payer()?;
        let primary_wallet = self.cli.primary_wallet_pubkey();
        let recipient = self.cli.recipient();
        let mint = self.get_mint()?;
        let program_id = self.cli.wallet_program_id();

        let proxy_wallet = pda::proxy_wallet(account, primary_wallet, program_id);

        let signature = self.client.withdraw_nft(
            payer,
            authority,
            proxy_wallet,
            recipient,
            mint,
            program_id,
        )?;

        self.print_signature(&signature);

        Ok(())
    }

    pub fn process_staking_initialize(&self) -> Result<()> {
        let primary_wallet = self.cli.primary_wallet()?;
        let payer = self.cli.payer()?;
        let mint = self.get_mint()?;
        let start_time = self.cli.start_time();
        let end_time = self.cli.end_time();
        let min_stake_size_ui = self.cli.min_stake_size();
        let program_id = self.cli.staking_program_id();

        let mint_account = self.client.mint_account(mint)?;
        let decimals = mint_account.decimals;
        let min_stake_size = spl_token::ui_amount_to_amount(min_stake_size_ui, decimals);

        let staking_info = Keypair::new();
        let signature = self.client.staking_initialize(
            &staking_info,
            primary_wallet,
            payer,
            mint,
            start_time,
            end_time,
            min_stake_size,
            program_id,
        )?;

        let file_name = "staking_info.pubkey";
        let mut file = fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(file_name)?;

        writeln!(file, "{}", staking_info.pubkey())
            .map_err(|_| CliError::CannotWriteToFile(file_name.to_owned()))?;

        println!("{} {}", "StakingInfo:".green(), staking_info.pubkey());
        self.print_signature(&signature);

        Ok(())
    }

    pub fn process_staking_add_reward_tokens(&self) -> Result<()> {
        let primary_wallet = self.cli.primary_wallet()?;
        let payer = self.cli.payer()?;
        let mint = self.get_mint()?;
        let staking_info = self.cli.staking_info();
        let program_id = self.cli.staking_program_id();

        let mint_account = self.client.mint_account(mint)?;
        let decimals = mint_account.decimals;
        let ui_amount = self.cli.ui_amount();
        let amount = spl_token::ui_amount_to_amount(ui_amount, decimals);

        let signature = self.client.staking_add_token_reward(
            primary_wallet,
            payer,
            staking_info,
            mint,
            amount,
            program_id,
        )?;

        self.print_signature(&signature);

        Ok(())
    }

    pub fn run(&self) {
        let result = match self.cli.command() {
            CliCommand::Balance => self.process_print_balance(),
            CliCommand::Info => self.process_print_info(),
            CliCommand::Initialize => self.process_nft_initialize(),
            CliCommand::Mint => self.process_mint(),
            CliCommand::MintNft => self.process_mint_nft(),
            CliCommand::UpdateNft => self.process_update_nft(),
            CliCommand::Transfer => self.process_transfer(),
            CliCommand::CreateWallet => self.process_create_wallet(),
            CliCommand::WithdrawLamports => self.process_withdraw_lamports(),
            CliCommand::WithdrawFt => self.process_withdraw_ft(),
            CliCommand::WithdrawNft => self.process_withdraw_nft(),
            CliCommand::StakingInitialize => self.process_staking_initialize(),
            CliCommand::StakingAddRewardTokens => self.process_staking_add_reward_tokens(),
        };

        if let Err(error) = result {
            self.on_error(error);
        }
    }
}
