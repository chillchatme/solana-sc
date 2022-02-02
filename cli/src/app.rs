use crate::{
    cli::{Cli, CliCommand},
    client::Client,
    error::{AppError, CliError, Result},
};
use solana_sdk::{
    native_token::sol_to_lamports,
    pubkey::{read_pubkey_file, write_pubkey_file, Pubkey},
    signature::{read_keypair_file, write_keypair_file, Keypair},
    signer::Signer,
};
use std::path::Path;

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

    fn on_error(error: AppError) -> ! {
        println!("{}", error);
        std::process::exit(1);
    }

    fn get_or_create_default_keypair(&self) -> Result<Box<dyn Signer>> {
        let mut keypair_file = dirs::home_dir().unwrap();
        keypair_file.push(".config");
        keypair_file.push("solana");
        keypair_file.push("id.json");

        let keypair_filename = keypair_file.clone().into_os_string().into_string().unwrap();

        if keypair_file.is_file() {
            let keypair = read_keypair_file(keypair_file)
                .map_err(|_| CliError::CannotParseFile(keypair_filename))?;
            return Ok(Box::new(keypair));
        }

        let new_keypair = Keypair::new();
        write_keypair_file(&new_keypair, keypair_file)
            .map_err(|_| CliError::CannotWriteToFile(keypair_filename))?;

        if !self.cli.mainnet() {
            self.client
                .airdrop(new_keypair.pubkey(), sol_to_lamports(5.0))?;
        }

        Ok(Box::new(new_keypair))
    }

    fn get_pubkey_from_file(&self, file_name: &str) -> Result<Pubkey> {
        let mint = read_pubkey_file(file_name)
            .map_err(|_| CliError::CannotParseFile(file_name.to_owned()))?;
        Ok(mint)
    }

    fn save_mint(&self, mint: Pubkey, file_name: &str) -> Result<()> {
        write_pubkey_file(file_name, mint)
            .map_err(|_| CliError::CannotWriteToFile(file_name.to_owned()))?;
        Ok(())
    }

    fn process_init(&self) -> Result<()> {
        let file_name = "mint.pubkey";
        let ui_amount = self.cli.ui_amount();
        let decimals = self.cli.decimals();
        let owner = match self.cli.owner() {
            Some(owner) => owner,
            None => self.get_or_create_default_keypair()?,
        };

        let mint;
        if !Path::new(file_name).is_file() {
            mint = self.client.create_mint(owner.as_ref(), decimals)?;
            self.save_mint(mint, file_name)?;
        } else {
            mint = self.get_pubkey_from_file(file_name)?;
        }

        if self.client.is_token_exists(owner.pubkey(), mint) {
            return Err(CliError::AlreadyInitialized.into());
        }

        let token = self.client.create_token(owner.as_ref(), mint)?;
        let amount = spl_token::ui_amount_to_amount(ui_amount, decimals);
        self.client.mint_to(owner.as_ref(), mint, token, amount)?;

        Ok(())
    }

    pub fn run(&self) {
        let result = match self.cli.command() {
            CliCommand::Mint => self.process_init(),
        };

        if let Err(error) = result {
            Self::on_error(error);
        }
    }
}
