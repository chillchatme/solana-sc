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
use spl_associated_token_account::get_associated_token_address;
use spl_token::amount_to_ui_amount;
use std::path::PathBuf;

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
        std::process::exit(1);
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
            .map_err(|_| CliError::CannotWriteToFile(keypair_filename))?;

        if !self.cli.mainnet() {
            self.client
                .airdrop(new_keypair.pubkey(), sol_to_lamports(1.0))?;
        }

        Ok(Box::new(new_keypair))
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
        std::fs::write(&save_path, mint.to_string())
            .map_err(|_| CliError::CannotWriteToFile(save_path))?;

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

    fn get_or_create_mint(&self, owner: &dyn Signer, decimals: u8) -> Result<Pubkey> {
        if let Some(mint) = self.cli.mint() {
            self.assert_mint_owner(mint, owner.pubkey())?;
            return Ok(mint);
        }

        let mint = self.client.create_mint(owner, decimals)?;
        self.save_mint(mint)?;
        Ok(mint)
    }

    fn print_balance(&self, token: Pubkey, decimals: u8) -> Result<()> {
        let token_account = self.client.token_account(token)?;
        let amount = token_account.amount;

        println!(
            "{} {} tokens",
            "Balance:".green(),
            amount_to_ui_amount(amount, decimals)
        );

        Ok(())
    }

    fn process_mint(&self) -> Result<()> {
        let owner = self.get_or_create_owner()?;
        let decimals = self.cli.decimals();
        let mint = self.get_or_create_mint(owner.as_ref(), decimals)?;
        let token = self.client.get_or_create_token(owner.as_ref(), mint)?;

        let ui_amount = self.cli.ui_amount();
        let amount = spl_token::ui_amount_to_amount(ui_amount, decimals);
        self.client.mint_to(owner.as_ref(), mint, token, amount)?;
        self.print_balance(token, decimals)
    }

    fn process_print_balance(&self) -> Result<()> {
        let owner = self.get_owner()?;
        let mint = self.cli.mint().unwrap();
        let token = get_associated_token_address(&owner.pubkey(), &mint);
        let mint_account = self.client.mint_account(mint)?;
        let decimals = mint_account.decimals;
        self.print_balance(token, decimals)
    }

    pub fn run(&self) {
        let result = match self.cli.command() {
            CliCommand::Mint => self.process_mint(),
            CliCommand::Balance => self.process_print_balance(),
        };

        if let Err(error) = result {
            self.on_error(error);
        }
    }
}
