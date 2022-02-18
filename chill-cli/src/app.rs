use crate::{
    cli::{Cli, CliCommand},
    error::{AppError, CliError, Result},
};
use chill_api::state::{Config, Fees};
use chill_client::client::Client;
use colored::Colorize;
use solana_sdk::{
    native_token::sol_to_lamports,
    program_option::COption,
    pubkey::Pubkey,
    signature::{read_keypair_file, write_keypair_file, Keypair, Signature},
    signer::Signer,
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::exit,
};

pub struct App<'cli> {
    cli: Cli<'cli>,
    client: Client,
}

impl App<'_> {
    pub fn init() -> Self {
        let cli = Cli::init();
        let client = Client::init(cli.rpc_url());
        App { cli, client }
    }

    fn on_error(&self, error: AppError) -> ! {
        println!("{}", error);
        exit(1);
    }

    fn default_keypair_path(&self) -> PathBuf {
        let mut keypair_path = dirs::home_dir().unwrap();
        keypair_path.push(".config");
        keypair_path.push("solana");
        keypair_path.push("id.json");
        keypair_path
    }

    fn get_default_keypair(&self) -> Result<Box<dyn Signer>> {
        let keypair_path = self.default_keypair_path();
        let keypair_filename = keypair_path.clone().into_os_string().into_string().unwrap();

        if keypair_path.is_file() {
            let keypair = read_keypair_file(keypair_path)
                .map_err(|e| CliError::CannotParseFile(keypair_filename, e.to_string()))?;
            Ok(Box::new(keypair))
        } else {
            Err(CliError::OwnerNotFound.into())
        }
    }

    fn get_or_create_default_keypair(&self) -> Result<Box<dyn Signer>> {
        if let Ok(keypair) = self.get_default_keypair() {
            return Ok(keypair);
        }

        let keypair_path = self.default_keypair_path();
        let keypair_filename = keypair_path.clone().into_os_string().into_string().unwrap();
        let new_keypair = Keypair::new();
        write_keypair_file(&new_keypair, keypair_path)
            .map_err(|_| CliError::CannotWriteToFile(keypair_filename.clone()))?;

        println!("{0} \"{1}\"", "Keypair file:".yellow(), keypair_filename);

        Ok(Box::new(new_keypair))
    }

    fn try_to_airdrop(&self, owner: Pubkey) -> Result<()> {
        if self.client.balance(owner)? == 0 {
            if self.cli.mainnet() {
                println!("{}", "You have to top up your balance".red());
                exit(0);
            } else {
                self.client.airdrop(owner, sol_to_lamports(1.0))?;
            }
        }

        Ok(())
    }

    fn get_owner_pubkey(&self) -> Result<Pubkey> {
        match self.cli.owner_pubkey() {
            Some(owner) => Ok(owner),
            None => Ok(self.get_default_keypair()?.pubkey()),
        }
    }

    fn get_owner(&self) -> Result<Box<dyn Signer>> {
        match self.cli.owner()? {
            Some(owner) => Ok(owner),
            None => self.get_default_keypair(),
        }
    }

    fn get_or_create_owner(&self) -> Result<Box<dyn Signer>> {
        match self.cli.owner()? {
            Some(owner) => Ok(owner),
            None => self.get_or_create_default_keypair(),
        }
    }

    fn save_mint(&self, mint: Pubkey) -> Result<()> {
        let save_path = self.cli.save_path();

        std::fs::write(save_path, mint.to_string())
            .map_err(|_| CliError::CannotWriteToFile(save_path.to_owned()))?;

        let path = Path::new(save_path);
        let full_path = fs::canonicalize(path).unwrap();
        let full_path_str = full_path.as_os_str().to_str().unwrap();
        println!("{0} \"{1}\"", "Mint file:".cyan(), full_path_str);
        Ok(())
    }

    fn assert_mint_owner(&self, mint: Pubkey, owner: Pubkey) -> Result<()> {
        let mint_account = self.client.mint_account(mint)?;
        if mint_account.mint_authority != COption::Some(owner) {
            Err(CliError::OwnerNotMatch(mint).into())
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

    fn get_or_create_mint(&self, owner: &dyn Signer, decimals: u8) -> Result<Pubkey> {
        if let Some(mint) = self.cli.mint()? {
            self.assert_mint_owner(mint, owner.pubkey())?;
            return Ok(mint);
        }

        let save_path = self.cli.save_path();
        let path = Path::new(save_path);
        if path.exists() {
            let full_path = fs::canonicalize(path).unwrap();
            let full_path_str = full_path.as_os_str().to_str().unwrap();
            return Err(CliError::MintFileExists(full_path_str.to_owned()).into());
        }

        let mint = self.client.create_mint(owner, decimals)?;
        println!("{0} {1}", "Mint:".cyan(), mint);

        self.save_mint(mint)?;
        Ok(mint)
    }

    fn print_signature(&self, signature: &Signature) {
        println!("{} {}", "Signature:".cyan(), signature);
    }

    fn print_balance(&self, owner: Pubkey, mint: Pubkey) -> Result<()> {
        let balance = self.client.ui_token_balance(owner, mint)?;
        println!("{} {} tokens", "Balance:".green().bold(), balance);

        Ok(())
    }

    fn print_info(&self, program_id: Pubkey, mint: Pubkey) -> Result<()> {
        let config = self.client.config(program_id, mint)?;
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
        let owner = self.get_or_create_owner()?;
        self.try_to_airdrop(owner.pubkey())?;

        let decimals = self.cli.decimals();
        let mint = self.get_or_create_mint(owner.as_ref(), decimals)?;
        let token =
            self.client
                .get_or_create_token_account(owner.as_ref(), owner.pubkey(), mint)?;

        let ui_amount = self.cli.ui_amount();
        let amount = spl_token::ui_amount_to_amount(ui_amount, decimals);
        self.client.mint_to(owner.as_ref(), mint, token, amount)?;

        self.print_balance(owner.pubkey(), mint)
    }

    fn process_mint_nft(&self) -> Result<()> {
        let owner = self.get_owner()?;

        let recipient_signer = self.cli.recipient();
        let nft_recipient = match recipient_signer {
            Ok(Some(ref signer)) => signer,
            _ => &owner,
        };

        let mint_chill = self.get_mint()?;
        let recipient_token_account = self.client.get_or_create_token_account(
            nft_recipient.as_ref(),
            nft_recipient.pubkey(),
            mint_chill,
        )?;

        let program_id = self.cli.program_id();
        let args = self.cli.mint_args()?;

        let (nft_mint, nft_token) = self
            .client
            .create_mint_and_token_nft(owner.as_ref(), nft_recipient.as_ref())?;

        println!("{0} {1}", "NFT Mint:".green(), nft_mint);
        let signature = self.client.mint_nft(
            program_id,
            owner.as_ref(),
            nft_recipient.as_ref(),
            mint_chill,
            recipient_token_account,
            nft_mint,
            nft_token,
            args,
        )?;
        self.print_signature(&signature);

        let recipient_pubkey_opt = self.cli.recipient_pubkey();
        if recipient_pubkey_opt.is_none() {
            return Ok(());
        }

        let recipient_pubkey = recipient_pubkey_opt.unwrap();
        if recipient_pubkey != nft_recipient.pubkey() {
            println!("\n{} '{}'", "Transfer NFT to".green(), recipient_pubkey);
            let signature =
                self.client
                    .transfer_tokens(owner.as_ref(), nft_mint, recipient_pubkey, 1)?;
            self.print_signature(&signature);
        }

        Ok(())
    }

    fn process_print_info(&self) -> Result<()> {
        let mint = self.get_mint()?;
        let program_id = self.cli.program_id();
        self.print_info(program_id, mint)
    }

    fn process_print_balance(&self) -> Result<()> {
        let owner = self.get_owner_pubkey()?;
        let mint = self.get_mint()?;
        self.print_balance(owner, mint)
    }

    fn process_transfer(&self) -> Result<()> {
        let owner = self.get_owner()?;
        let mint = self.get_mint()?;

        let ui_amount = self.cli.ui_amount();
        let recipient = self.cli.recipient_pubkey().unwrap();

        if ui_amount == 0.0 {
            return Err(CliError::TransferZeroTokens.into());
        }

        let current_balance = self.client.ui_token_balance(owner.pubkey(), mint)?;
        if ui_amount > current_balance {
            return Err(CliError::InsufficientTokens(ui_amount, current_balance).into());
        }

        let mint_account = self.client.mint_account(mint)?;
        let decimals = mint_account.decimals;
        let amount = spl_token::ui_amount_to_amount(ui_amount, decimals);

        let signature = self
            .client
            .transfer_tokens(owner.as_ref(), mint, recipient, amount)?;

        self.print_signature(&signature);
        self.print_balance(owner.pubkey(), mint)
    }

    pub fn initialize(&self) -> Result<()> {
        let owner = self.get_owner()?;
        let mint = self.get_mint()?;
        self.assert_mint_owner(mint, owner.pubkey())?;

        let ui_fees = self.cli.fees();
        let program_id = self.cli.program_id();

        let recipients = self.cli.multiple_recipients()?;
        Config::check_recipients(&recipients)?;

        let mint_account = self.client.mint_account(mint)?;
        let fees = Fees::from_ui(ui_fees, mint_account.decimals);

        self.client
            .initialize(program_id, owner.as_ref(), mint, fees, recipients)?;
        self.print_info(program_id, mint)
    }

    pub fn run(&self) {
        let result = match self.cli.command() {
            CliCommand::Balance => self.process_print_balance(),
            CliCommand::Info => self.process_print_info(),
            CliCommand::Initialize => self.initialize(),
            CliCommand::Mint => self.process_mint(),
            CliCommand::MintNft => self.process_mint_nft(),
            CliCommand::Transfer => self.process_transfer(),
        };

        if let Err(error) = result {
            self.on_error(error);
        }
    }
}
