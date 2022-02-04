use crate::{
    cli::{Cli, CliCommand},
    client::Client,
    error::{AppError, CliError, Result},
};
use colored::Colorize;
use solana_sdk::{
    native_token::sol_to_lamports,
    program_option::COption,
    pubkey::Pubkey,
    signature::{read_keypair_file, write_keypair_file, Keypair},
    signer::Signer,
};
use std::{fs, path::PathBuf, process::exit};

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
                .map_err(|_| CliError::CannotParseFile(keypair_filename))?;
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

    fn check_balance(&self, owner: Pubkey) -> Result<()> {
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

    fn get_owner(&self) -> Result<Box<dyn Signer>> {
        match self.cli.owner() {
            Some(owner) => Ok(owner),
            None => self.get_default_keypair(),
        }
    }

    fn get_or_create_owner(&self) -> Result<Box<dyn Signer>> {
        match self.cli.owner() {
            Some(owner) => Ok(owner),
            None => self.get_or_create_default_keypair(),
        }
    }

    fn save_mint(&self, mint: Pubkey) -> Result<()> {
        let save_path = self.cli.save_path();
        std::fs::write(save_path, mint.to_string())
            .map_err(|_| CliError::CannotWriteToFile(save_path.to_owned()))?;

        let path = std::path::Path::new(save_path);
        let full_path = fs::canonicalize(path).unwrap();

        println!(
            "{0} \"{1}\"",
            "Mint file:".cyan(),
            full_path.as_os_str().to_str().unwrap()
        );
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

    fn get_mint(&self, owner: Pubkey) -> Result<Pubkey> {
        let mint = self.cli.mint()?;
        self.assert_mint_owner(mint, owner)?;
        Ok(mint)
    }

    fn get_or_create_mint(&self, owner: &dyn Signer, decimals: u8) -> Result<Pubkey> {
        if let Ok(mint) = self.cli.mint_with_default() {
            self.assert_mint_owner(mint, owner.pubkey())?;
            return Ok(mint);
        }

        let mint = self.client.create_mint(owner, decimals)?;
        println!("{0} {1}", "Mint:".cyan(), mint);

        self.save_mint(mint)?;
        Ok(mint)
    }

    fn get_or_create_token_account(&self, owner: &dyn Signer, mint: Pubkey) -> Result<Pubkey> {
        if let Ok(token) = self.client.get_token_pubkey(owner.pubkey(), mint) {
            return Ok(token);
        }

        let token = self
            .client
            .create_token_account(owner, owner.pubkey(), mint)?;

        Ok(token)
    }

    fn print_balance(&self, owner: Pubkey, mint: Pubkey) -> Result<()> {
        let balance = self.client.token_balance(owner, mint)?;
        println!("{} {} tokens", "Balance:".green().bold(), balance);

        Ok(())
    }

    fn process_mint(&self) -> Result<()> {
        let owner = self.get_or_create_owner()?;
        self.check_balance(owner.pubkey())?;

        let decimals = self.cli.decimals();
        let mint = self.get_or_create_mint(owner.as_ref(), decimals)?;
        let token = self.get_or_create_token_account(owner.as_ref(), mint)?;

        let ui_amount = self.cli.ui_amount();
        let amount = spl_token::ui_amount_to_amount(ui_amount, decimals);
        self.client.mint_to(owner.as_ref(), mint, token, amount)?;

        self.print_balance(owner.pubkey(), mint)
    }

    fn process_print_balance(&self) -> Result<()> {
        let owner = self.get_owner()?;
        let mint = self.get_mint(owner.pubkey())?;
        self.print_balance(owner.pubkey(), mint)
    }

    fn process_transfer(&self) -> Result<()> {
        let owner = self.get_owner()?;
        let mint = self.get_mint(owner.pubkey())?;
        let ui_amount = self.cli.ui_amount();
        let receiver = self.cli.receiver();

        let receiver_token_account = match self.client.find_token_account(receiver, mint)? {
            Some(token_account) => token_account,
            None => self
                .client
                .create_token_account(owner.as_ref(), receiver, mint)?,
        };

        let current_balance = self.client.token_balance(owner.pubkey(), mint)?;
        if ui_amount > current_balance {
            return Err(CliError::InsufficientTokens(ui_amount, current_balance).into());
        }

        let mint_account = self.client.mint_account(mint)?;
        let decimals = mint_account.decimals;
        let amount = spl_token::ui_amount_to_amount(ui_amount, decimals);

        let signature =
            self.client
                .transfer_tokens(owner.as_ref(), mint, receiver_token_account, amount)?;

        println!("{} {}", "Signature:".cyan(), signature);
        self.print_balance(owner.pubkey(), mint)
    }

    pub fn run(&self) {
        let result = match self.cli.command() {
            CliCommand::Mint => self.process_mint(),
            CliCommand::Balance => self.process_print_balance(),
            CliCommand::Transfer => self.process_transfer(),
        };

        if let Err(error) = result {
            self.on_error(error);
        }
    }
}
