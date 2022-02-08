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

    pub fn owner_pubkey(&self) -> Option<Pubkey> {
        let matches = self.get_matches().1;
        matches
            .value_of(OWNER)
            .map(|_| pubkey_of(matches, OWNER).unwrap())
    }

    pub fn owner(&self) -> Result<Option<Box<dyn Signer>>> {
        let matches = self.get_matches().1;
        if matches.is_present(OWNER) {
            let owner = matches.value_of(OWNER).unwrap();
            let signer = signer_from_path(matches, owner, OWNER, &mut None)
                .map_err(|e| CliError::CannotGetOwner(e.to_string()))?;

            Ok(Some(signer))
        } else {
            Ok(None)
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::signature::{read_keypair_file, Keypair};

    const MINT_PATH: &str = "../localnet/mint.pubkey.localnet";
    const OWNER_PATH: &str = "../localnet/owner.json";
    const RECEIVER_PATH: &str = "../localnet/receiver.json";

    fn get_cli<'a>(args: &[&'a str]) -> Cli<'a> {
        let mut argv = vec![crate_name!()];
        argv.extend(args);

        let app = Cli::build_app();
        let matches = app.get_matches_from(argv);
        Cli { matches }
    }

    fn mint_pubkey() -> Pubkey {
        let pubkey_str = fs::read_to_string(MINT_PATH).unwrap();
        Pubkey::from_str(pubkey_str.trim()).unwrap()
    }

    fn owner() -> Keypair {
        read_keypair_file(OWNER_PATH).unwrap()
    }

    fn receiver() -> Keypair {
        read_keypair_file(RECEIVER_PATH).unwrap()
    }

    #[test]
    fn mint() {
        let amount = "1000";
        let cli = get_cli(&[COMMAND_MINT, amount]);

        assert!(!cli.mainnet());
        assert!(cli.default_mint_file().contains("devnet"));
        assert!(cli.mint().unwrap().is_none());
        assert_eq!(cli.decimals(), 9);
        assert_eq!(cli.owner().unwrap(), None);
        assert_eq!(cli.save_path(), cli.default_mint_file());
        assert_eq!(cli.ui_amount().to_string(), amount);

        let decimals = "0";
        let all_args_string = format!(
            "{0} {1} --{2} {3} --{4} {5} --{6} {7} --{8}",
            COMMAND_MINT, amount, OWNER, OWNER_PATH, DECIMALS, decimals, MINT, MINT_PATH, MAINNET
        );

        let args = all_args_string.split(' ').collect::<Vec<&str>>();
        let cli = get_cli(&args);

        assert!(cli.default_mint_file().contains("mainnet"));
        assert!(cli.mainnet());
        assert_eq!(cli.decimals().to_string(), decimals);
        assert_eq!(cli.mint().unwrap(), Some(mint_pubkey()));
        assert_eq!(cli.owner().unwrap().unwrap().pubkey(), owner().pubkey());
        assert_eq!(cli.save_path(), cli.default_mint_file());
        assert_eq!(cli.ui_amount().to_string(), amount);
    }

    #[test]
    fn balance() {
        let cli = get_cli(&[COMMAND_BALANCE]);

        assert!(!cli.mainnet());
        assert!(cli.mint().unwrap().is_none());
        assert_eq!(cli.owner().unwrap(), None);

        let args_with_mint_as_pubkey =
            format!("{0} --{1} {2}", COMMAND_BALANCE, MINT, mint_pubkey());

        let args = args_with_mint_as_pubkey.split(' ').collect::<Vec<&str>>();
        let cli = get_cli(&args);
        assert_eq!(cli.mint().unwrap(), Some(mint_pubkey()));

        let all_args_string = format!(
            "{0} --{1} {2} --{3} {4} --{5}",
            COMMAND_BALANCE, OWNER, OWNER_PATH, MINT, MINT_PATH, MAINNET
        );

        let args = all_args_string.split(' ').collect::<Vec<&str>>();
        let cli = get_cli(&args);

        assert!(cli.mainnet());
        assert_eq!(cli.mint().unwrap(), Some(mint_pubkey()));
        assert_eq!(cli.owner().unwrap().unwrap().pubkey(), owner().pubkey());
    }

    #[test]
    fn transfer() {
        let amount = "1000";
        let cli = get_cli(&[COMMAND_TRANSFER, RECEIVER_PATH, amount]);

        assert!(!cli.mainnet());
        assert!(cli.mint().unwrap().is_none());
        assert_eq!(cli.owner().unwrap(), None);
        assert_eq!(cli.receiver(), receiver().pubkey());
        assert_eq!(cli.ui_amount().to_string(), amount);

        let receiver_pubkey = receiver().pubkey().to_string();
        let cli = get_cli(&[COMMAND_TRANSFER, &receiver_pubkey, amount]);
        assert_eq!(cli.receiver(), receiver().pubkey());

        let all_args_string = format!(
            "{0} {1} {2} --{3} {4} --{5} {6} --{7}",
            COMMAND_TRANSFER, RECEIVER_PATH, amount, MINT, MINT_PATH, OWNER, OWNER_PATH, MAINNET
        );

        let args = all_args_string.split(' ').collect::<Vec<&str>>();
        let cli = get_cli(&args);

        assert!(cli.mainnet());
        assert_eq!(cli.mint().unwrap(), Some(mint_pubkey()));
        assert_eq!(cli.owner().unwrap().unwrap().pubkey(), owner().pubkey());
        assert_eq!(cli.receiver(), receiver().pubkey());
        assert_eq!(cli.ui_amount().to_string(), amount);
    }
}
