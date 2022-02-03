use crate::error::{CliError, Result};
use clap::{
    crate_description, crate_name, crate_version, value_t_or_exit, App, AppSettings, Arg,
    ArgMatches, SubCommand,
};
use solana_clap_utils::{input_validators::is_valid_signer, keypair::signer_from_path};
use solana_sdk::{pubkey::Pubkey, signer::Signer};
use std::{fs, path::Path, str::FromStr};

const DEFAULT_MINT_PATH: &str = "mint.pubkey";

const COMMAND_MINT: &str = "mint";
const COMMAND_BALANCE: &str = "balance";

const AMOUNT: &str = "amount";
const DECIMALS: &str = "decimals";
const MAINNET: &str = "mainnet";
const MINT: &str = "mint-address";
const OWNER: &str = "owner";
const SAVE_PATH: &str = "save-path";

pub enum CliCommand {
    Mint,
    Balance,
}

pub struct Cli<'a> {
    matches: ArgMatches<'a>,
}

fn is_pubkey(string: String) -> core::result::Result<(), String> {
    if Pubkey::from_str(&string).is_ok() {
        return Ok(());
    }

    if let Ok(pubkey) = fs::read_to_string(&string) {
        match Pubkey::from_str(pubkey.trim()) {
            Ok(_) => return Ok(()),
            Err(_) => {
                return Err(format!(
                    "Cannot parse file '{0}'. File must contain a base58 encoded public key",
                    string
                ))
            }
        }
    }

    if string == DEFAULT_MINT_PATH {
        return Err("Specify mint address if exists or create a new one".to_owned());
    }

    Err(format!(
        "Cannot parse '{0}' as a public key or path",
        string
    ))
}

impl<'a> Cli<'a> {
    fn build_app<'b, 'c>() -> App<'b, 'c> {
        let account_address = "ACCOUNT_ADDRESS";
        let owner = Arg::with_name(OWNER)
            .long(OWNER)
            .short("o")
            .required(false)
            .takes_value(true)
            .value_name(account_address)
            .validator(is_valid_signer)
            .help(concat!(
                "The account address of the owner. One of:\n",
                "   * a base58-encoded public key\n",
                "   * a path to a keypair file\n",
                "   * a hyphen; signals a JSON-encoded keypair on stdin\n",
                "   * the 'ASK' keyword; to recover a keypair via its seed phrase\n",
                "   * a hardware wallet keypair URL (i.e. usb://ledger)"
            ));

        let mint = Arg::with_name(MINT)
            .long(MINT)
            .short("a")
            .required(false)
            .takes_value(true)
            .value_name(account_address)
            .validator(is_pubkey)
            .help(concat!(
                "The mint account address. One of:\n",
                "   * a base58-encoded public key\n",
                "   * a path to a pubkey file"
            ));

        let save_path = Arg::with_name(SAVE_PATH)
            .long(SAVE_PATH)
            .short("p")
            .required(false)
            .takes_value(true)
            .value_name("PATH")
            .default_value(DEFAULT_MINT_PATH)
            .help("The path to the file where to put mint pubkey");

        let amount = Arg::with_name(AMOUNT)
            .required(true)
            .takes_value(true)
            .value_name("AMOUNT")
            .help("Amount of tokens to mint");

        let decimals = Arg::with_name(DECIMALS)
            .long(DECIMALS)
            .short("d")
            .takes_value(true)
            .value_name("DECIMALS")
            .default_value("9")
            .help("Decimals of the new mint");

        let mainnet = Arg::with_name(MAINNET)
            .long(MAINNET)
            .short("m")
            .takes_value(false)
            .help("Run command in Mainnet");

        let mint_command = SubCommand::with_name(COMMAND_MINT)
            .args(&[
                amount,
                decimals,
                mainnet.clone(),
                mint.clone(),
                owner.clone(),
                save_path,
            ])
            .about("Create a mint and token account if they don't exist, and mint <AMOUNT> tokens");

        let balance_command = SubCommand::with_name(COMMAND_BALANCE)
            .args(&[mainnet, mint, owner])
            .about("Print the balance of the token account");

        App::new(crate_name!())
            .about(crate_description!())
            .version(crate_version!())
            .subcommands(vec![mint_command, balance_command])
            .setting(AppSettings::SubcommandRequiredElseHelp)
    }

    pub fn init() -> Self {
        let app = Self::build_app();
        Self {
            matches: app.get_matches(),
        }
    }

    fn get_matches(&self) -> (&'static str, &ArgMatches<'a>) {
        match self.matches.subcommand() {
            (COMMAND_MINT, Some(matcher)) => (COMMAND_MINT, matcher),
            (COMMAND_BALANCE, Some(matcher)) => (COMMAND_BALANCE, matcher),
            _ => unimplemented!(),
        }
    }

    pub fn command(&self) -> CliCommand {
        match self.get_matches().0 {
            COMMAND_MINT => CliCommand::Mint,
            COMMAND_BALANCE => CliCommand::Balance,
            _ => unimplemented!(),
        }
    }

    pub fn ui_amount(&self) -> f64 {
        let matches = self.get_matches().1;
        value_t_or_exit!(matches, AMOUNT, f64)
    }

    pub fn decimals(&self) -> u8 {
        let matches = self.get_matches().1;
        value_t_or_exit!(matches, DECIMALS, u8)
    }

    pub fn save_path(&self) -> &str {
        let matches = self.get_matches().1;
        matches.value_of(SAVE_PATH).unwrap()
    }

    pub fn owner(&self) -> Option<Box<dyn Signer>> {
        let matches = self.get_matches().1;
        matches
            .value_of(OWNER)
            .and_then(|path| signer_from_path(matches, path, OWNER, &mut None).ok())
    }

    fn parse_mint(&self, mint: &str) -> Result<Pubkey> {
        let mint_pubkey;
        if Path::new(mint).is_file() {
            let pubkey_string = fs::read_to_string(mint)?;
            mint_pubkey = Pubkey::from_str(pubkey_string.trim())?;
        } else {
            mint_pubkey = Pubkey::from_str(mint.trim())?;
        }
        Ok(mint_pubkey)
    }

    pub fn mint_with_default(&self) -> Result<Pubkey> {
        let matches = self.get_matches().1;
        let mint = matches.value_of(MINT).unwrap_or(DEFAULT_MINT_PATH);
        self.parse_mint(mint)
    }

    pub fn mint(&self) -> Result<Pubkey> {
        let matches = self.get_matches().1;
        match matches.value_of(MINT) {
            Some(mint) => self.parse_mint(mint),
            None => self
                .parse_mint(DEFAULT_MINT_PATH)
                .map_err(|_| CliError::MintNotSpecified.into()),
        }
    }

    pub fn mainnet(&self) -> bool {
        let matches = self.get_matches().1;
        matches.is_present(MAINNET)
    }

    pub fn rpc_url(&self) -> &'static str {
        if self.mainnet() {
            "https://api.mainnet-beta.solana.com"
        } else {
            "https://api.devnet.solana.com"
        }
    }
}
