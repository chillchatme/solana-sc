use crate::error::{CliError, Result};
use clap::{
    crate_description, crate_name, crate_version, value_t_or_exit, App, AppSettings, Arg,
    ArgMatches, SubCommand,
};
use solana_clap_utils::{
    input_parsers::pubkey_of,
    input_validators::{is_pubkey, is_valid_pubkey, is_valid_signer},
    keypair::signer_from_path,
};
use solana_sdk::{pubkey::Pubkey, signer::Signer};
use std::{fs, path::Path, str::FromStr};

const COMMAND_BALANCE: &str = "balance";
const COMMAND_MINT: &str = "mint";
const COMMAND_TRANSFER: &str = "transfer";

const AMOUNT: &str = "amount";
const DECIMALS: &str = "decimals";
const MAINNET: &str = "mainnet";
const MINT: &str = "mint-address";
const OWNER: &str = "owner";
const RECEIVER: &str = "receiver";
const SAVE_PATH: &str = "save-path";

pub enum CliCommand {
    Mint,
    Balance,
    Transfer,
}

pub struct Cli<'a> {
    matches: ArgMatches<'a>,
}

fn is_mint_pubkey(string: String) -> core::result::Result<(), String> {
    if is_pubkey(&string).is_ok() {
        return Ok(());
    }

    if let Ok(pubkey_from_file) = fs::read_to_string(&string) {
        match is_pubkey(pubkey_from_file.trim()) {
            Ok(_) => return Ok(()),
            Err(_) => {
                return Err(format!(
                    "Cannot parse file '{0}'. File must contain a base58-encoded public key",
                    string
                ))
            }
        }
    }

    Err(format!(
        "Cannot parse '{0}' as a public key or path",
        string
    ))
}

impl<'a> Cli<'a> {
    fn build_app<'b, 'c>() -> App<'b, 'c> {
        let account_address = "ACCOUNT_ADDRESS";
        let account_address_help = concat!(
            "Account address is one of:\n",
            "   * a base58-encoded public key\n",
            "   * a path to a keypair file\n",
            "   * a hyphen; signals a JSON-encoded keypair on stdin\n",
            "   * the 'ASK' keyword; to recover a keypair via its seed phrase\n",
            "   * a hardware wallet keypair URL (i.e. usb://ledger)"
        );

        let owner = Arg::with_name(OWNER)
            .long(OWNER)
            .short("o")
            .required(false)
            .takes_value(true)
            .value_name(account_address)
            .validator(is_valid_signer)
            .help(account_address_help);

        let mint = Arg::with_name(MINT)
            .long(MINT)
            .short("a")
            .required(false)
            .takes_value(true)
            .value_name(account_address)
            .validator(is_mint_pubkey)
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
            .help("The path to the file where to put mint pubkey");

        let amount = Arg::with_name(AMOUNT)
            .required(true)
            .takes_value(true)
            .value_name("AMOUNT");

        let amount_mint = amount.clone().help("Amount of tokens to mint");
        let amount_transfer = amount.help("Amount of tokens to transfer");

        let decimals = Arg::with_name(DECIMALS)
            .long(DECIMALS)
            .short("d")
            .takes_value(true)
            .value_name("DECIMALS")
            .default_value("9")
            .help("Number of base 10 digits to the right of the decimal place");

        let mainnet = Arg::with_name(MAINNET)
            .long(MAINNET)
            .short("m")
            .takes_value(false)
            .help("Runs the command in the Mainnet");

        let receiver = Arg::with_name(RECEIVER)
            .required(true)
            .takes_value(true)
            .value_name("RECEIVER_ADDRESS")
            .validator(is_valid_pubkey)
            .help(account_address_help);

        let mint_command = SubCommand::with_name(COMMAND_MINT)
            .args(&[
                amount_mint,
                decimals,
                mainnet.clone(),
                mint.clone(),
                owner.clone(),
                save_path,
            ])
            .about(
                "Creates mint and token accounts, if they don't exist, and mint a number of tokens",
            );

        let balance_command = SubCommand::with_name(COMMAND_BALANCE)
            .args(&[mainnet.clone(), mint.clone(), owner.clone()])
            .about("Prints the balance of the token account");

        let transfer_command = SubCommand::with_name(COMMAND_TRANSFER)
            .args(&[mainnet, mint, owner, receiver, amount_transfer])
            .about("Transfers a number of tokens to the destination address");

        App::new(crate_name!())
            .about(crate_description!())
            .version(crate_version!())
            .subcommands(vec![mint_command, balance_command, transfer_command])
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
            (COMMAND_TRANSFER, Some(matcher)) => (COMMAND_TRANSFER, matcher),
            _ => unimplemented!(),
        }
    }

    pub fn command(&self) -> CliCommand {
        match self.get_matches().0 {
            COMMAND_MINT => CliCommand::Mint,
            COMMAND_BALANCE => CliCommand::Balance,
            COMMAND_TRANSFER => CliCommand::Transfer,
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
        matches
            .value_of(SAVE_PATH)
            .unwrap_or_else(|| self.default_mint_file())
    }

    pub fn receiver(&self) -> Pubkey {
        let matches = self.get_matches().1;
        pubkey_of(matches, RECEIVER).unwrap()
    }

    pub fn owner(&self) -> Option<Box<dyn Signer>> {
        let matches = self.get_matches().1;
        matches
            .value_of(OWNER)
            .and_then(|path| signer_from_path(matches, path, OWNER, &mut None).ok())
    }

    fn default_mint_file(&self) -> &str {
        if self.mainnet() {
            "mint.mainnet.pubkey"
        } else {
            "mint.devnet.pubkey"
        }
    }

    fn parse_mint(&self, mint: &str) -> Result<Option<Pubkey>> {
        let mint = mint.trim();
        if let Ok(pubkey) = Pubkey::from_str(mint) {
            return Ok(Some(pubkey));
        }

        if Path::new(mint).is_file() {
            let pubkey_from_file = fs::read_to_string(mint)?;
            let pubkey = Pubkey::from_str(pubkey_from_file.trim())
                .map_err(|e| CliError::CannotParseFile(mint.to_string(), e.to_string()))?;
            return Ok(Some(pubkey));
        }

        Ok(None)
    }

    pub fn mint(&self) -> Result<Option<Pubkey>> {
        let matches = self.get_matches().1;
        let default_mint_path = self.default_mint_file();
        let mint = matches.value_of(MINT).unwrap_or(default_mint_path);
        self.parse_mint(mint)
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
