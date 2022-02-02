use clap::{
    crate_description, crate_name, crate_version, value_t_or_exit, App, AppSettings, Arg,
    ArgMatches, SubCommand,
};
use solana_clap_utils::{input_validators::is_valid_signer, keypair::signer_from_path};
use solana_sdk::signer::Signer;

const COMMAND_INIT: &str = "init";

const AMOUNT: &str = "amount";
const DECIMALS: &str = "decimals";
const MAINNET: &str = "mainnet";
const OWNER: &str = "owner";

pub enum CliCommand {
    Init,
}

pub struct Cli<'a> {
    matches: ArgMatches<'a>,
}

impl<'a> Cli<'a> {
    fn build_app<'b, 'c>() -> App<'b, 'c> {
        let owner = Arg::with_name(OWNER)
            .long(OWNER)
            .short("o")
            .required(false)
            .takes_value(true)
            .value_name("ACCOUNT_ADDRESS")
            .validator(is_valid_signer)
            .help(concat!(
                "The account address of the owner. One of:\n",
                "   * a path to a keypair file\n",
                "   * a hyphen; signals a JSON-encoded keypair on stdin\n",
                "   * the 'ASK' keyword; to recover a keypair via its seed phrase\n",
                "   * a hardware wallet keypair URL (i.e. usb://ledger)",
            ));

        let amount = Arg::with_name(AMOUNT)
            .long(AMOUNT)
            .short("a")
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
            .help("Decimals of new mint");

        let mainnet = Arg::with_name(MAINNET)
            .long(MAINNET)
            .short("m")
            .takes_value(false)
            .help("Mint tokens to Mainnet");

        let init = SubCommand::with_name(COMMAND_INIT)
            .args(&[decimals, owner, amount, mainnet])
            .about("Create a new mint and token account with initial balance");

        App::new(crate_name!())
            .about(crate_description!())
            .version(crate_version!())
            .subcommands(vec![init])
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
            (COMMAND_INIT, Some(matcher)) => (COMMAND_INIT, matcher),
            _ => unimplemented!(),
        }
    }

    pub fn command(&self) -> CliCommand {
        match self.get_matches().0 {
            COMMAND_INIT => CliCommand::Init,
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

    pub fn owner(&self) -> Option<Box<dyn Signer>> {
        let matches = self.get_matches().1;
        matches
            .value_of(OWNER)
            .and_then(|path| signer_from_path(matches, path, OWNER, &mut None).ok())
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
