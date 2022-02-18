use crate::error::{CliError, Result};
use chill_api::{
    instruction::MintNftArgs,
    state::{Config, NftType, Recipient, UiFees},
};
use clap::{
    crate_description, crate_name, crate_version, value_t_or_exit, values_t_or_exit, App,
    AppSettings, Arg, ArgMatches, SubCommand,
};
use solana_clap_utils::{
    input_parsers::{pubkey_of, pubkeys_of},
    input_validators::{is_pubkey, is_valid_pubkey, is_valid_signer},
    keypair::signer_from_path,
};
use solana_sdk::{pubkey::Pubkey, signature::Signer};
use std::{error, fs, path::Path, str::FromStr};

const COMMAND_BALANCE: &str = "balance";
const COMMAND_INFO: &str = "info";
const COMMAND_INITIALIZE: &str = "initialize";
const COMMAND_MINT: &str = "mint";
const COMMAND_MINT_NFT: &str = "mint-nft";
const COMMAND_TRANSFER: &str = "transfer";

const AMOUNT: &str = "amount";
const DECIMALS: &str = "decimals";
const FEES: &str = "fees";
const FEES_CHARACTER: &str = "character";
const FEES_EMOTE: &str = "emote";
const FEES_ITEM: &str = "item";
const FEES_PET: &str = "pet";
const FEES_TILESET: &str = "tileset";
const MAINNET: &str = "mainnet";
const MINT: &str = "mint-address";
const MINT_SHARE: &str = "mint-share";
const NAME: &str = "name";
const SYMBOL: &str = "symbol";
const NFT_TYPE: &str = "type";
const OWNER: &str = "owner";
const PROGRAM_ID: &str = "program-id";
const RECIPIENT: &str = "recipient";
const SAVE_PATH: &str = "save-path";
const TRANSACTION_SHARE: &str = "transaction-share";
const URL: &str = "url";

pub enum CliCommand {
    Balance,
    Info,
    Initialize,
    Mint,
    MintNft,
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
    pub fn init() -> Self {
        let app = Self::build_app();
        Self {
            matches: app.get_matches(),
        }
    }

    fn build_app<'b, 'c>() -> App<'b, 'c> {
        let account_address = "ACCOUNT_ADDRESS";
        let account_address_help = concat!(
            "<ACCOUNT_ADDRESS> is one of:\n",
            "   * a base58-encoded public key\n",
            "   * a path to a keypair file\n",
            "   * a hyphen; signals a JSON-encoded keypair on stdin\n",
            "   * the 'ASK' keyword; to recover a keypair via its seed phrase\n",
            "   * a hardware wallet keypair URL (i.e. usb://ledger)\n\n",
            "<MINT_ADDRESS> is one of:\n",
            "   * a base58-encoded public key\n",
            "   * a path to a pubkey file"
        );

        let owner = Arg::with_name(OWNER)
            .long(OWNER)
            .short("o")
            .required(false)
            .takes_value(true)
            .value_name(account_address)
            .validator(is_valid_signer);

        let mint = Arg::with_name(MINT)
            .long(MINT)
            .short("a")
            .required(false)
            .takes_value(true)
            .value_name("MINT_ADDRESS")
            .validator(is_mint_pubkey)
            .help("Chill mint pubkey");

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

        let recipient = Arg::with_name(RECIPIENT)
            .takes_value(true)
            .value_name("RECIPIENT_ADDRESS")
            .validator(is_valid_signer)
            .help("Recipient's account address");

        let required_recipient = recipient.clone().required(true);

        let program_id = Arg::with_name(PROGRAM_ID)
            .long(PROGRAM_ID)
            .takes_value(true)
            .value_name("ADDRESS")
            .validator(is_valid_pubkey)
            .help("Program id of the Chill smart-contract");

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
            )
            .after_help(account_address_help);

        let balance_command = SubCommand::with_name(COMMAND_BALANCE)
            .args(&[mainnet.clone(), mint.clone(), owner.clone()])
            .about("Prints the balance of the token account")
            .after_help(account_address_help);

        let info_command = SubCommand::with_name(COMMAND_INFO)
            .args(&[mainnet.clone(), mint.clone(), program_id.clone()])
            .about("Prints the information about smart-contract state");

        let transfer_command = SubCommand::with_name(COMMAND_TRANSFER)
            .args(&[
                mainnet.clone(),
                mint.clone(),
                owner.clone(),
                required_recipient,
                amount_transfer,
            ])
            .about("Transfers a number of tokens to the destination address")
            .after_help(account_address_help);

        //
        // Initialize
        //

        let multiple_recipients = Arg::with_name(RECIPIENT)
            .long(RECIPIENT)
            .short("r")
            .takes_value(true)
            .multiple(true)
            .max_values(Config::MAX_RECIPIENT_NUMBER as u64)
            .value_name("RECIPIENT_ADDRESS")
            .validator(is_valid_pubkey);

        let mint_share = Arg::with_name(MINT_SHARE)
            .long(MINT_SHARE)
            .short("s")
            .takes_value(true)
            .multiple(true)
            .value_name("SHARE")
            .max_values(Config::MAX_RECIPIENT_NUMBER as u64)
            .help("Percentage of fees for NFT minting in Chill tokens");

        let transaction_share = Arg::with_name(TRANSACTION_SHARE)
            .long(TRANSACTION_SHARE)
            .short("x")
            .takes_value(true)
            .multiple(true)
            .value_name("SHARE")
            .max_values(Config::MAX_RECIPIENT_NUMBER as u64)
            .help("Percentage of transaction fees in Chill tokens");

        let fees_value_name = "FEE";
        let fees_character = Arg::with_name(FEES_CHARACTER)
            .long(FEES_CHARACTER)
            .short("c")
            .required(true)
            .takes_value(true)
            .value_name(fees_value_name)
            .help("Fees for mint a character NFT");

        let fees_pet = Arg::with_name(FEES_PET)
            .long(FEES_PET)
            .short("p")
            .required(true)
            .takes_value(true)
            .value_name(fees_value_name)
            .help("Fees for mint a pet NFT");

        let fees_emote = Arg::with_name(FEES_EMOTE)
            .long(FEES_EMOTE)
            .short("e")
            .required(true)
            .takes_value(true)
            .value_name(fees_value_name)
            .help("Fees for mint an emote NFT");

        let fees_tileset = Arg::with_name(FEES_TILESET)
            .long(FEES_TILESET)
            .short("t")
            .required(true)
            .takes_value(true)
            .value_name(fees_value_name)
            .help("Fees for mint a tileset NFT");

        let fees_item = Arg::with_name(FEES_ITEM)
            .long(FEES_ITEM)
            .short("i")
            .required(true)
            .takes_value(true)
            .value_name(fees_value_name)
            .help("Fees for mint an item NFT");

        let initialize_command = SubCommand::with_name(COMMAND_INITIALIZE)
            .args(&[
                mainnet.clone(),
                mint.clone(),
                owner.clone(),
                program_id.clone(),
                multiple_recipients,
                mint_share,
                transaction_share,
                fees_character,
                fees_pet,
                fees_emote,
                fees_tileset,
                fees_item,
            ])
            .about("Initializes the Chill smart-contract")
            .after_help(account_address_help);

        //
        // MintNft
        //

        let nft_type = Arg::with_name(NFT_TYPE)
            .takes_value(true)
            .value_name("TYPE")
            .possible_values(&["character", "pet", "emote", "tileset", "item"])
            .required(true)
            .help("Nft type");

        let name = Arg::with_name(NAME)
            .takes_value(true)
            .value_name("NAME")
            .required(true)
            .help("Name of the NFT");

        let url = Arg::with_name(URL)
            .takes_value(true)
            .value_name("URL")
            .required(true)
            .help("URL to the a NFT image");

        let symbol = Arg::with_name(SYMBOL)
            .long(SYMBOL)
            .short("s")
            .takes_value(true)
            .value_name("SYMBOL")
            .default_value("CHILL")
            .help("Symbol of the NFT");

        let fees = Arg::with_name(FEES)
            .long(FEES)
            .short("f")
            .takes_value(true)
            .value_name("PERCENT")
            .default_value("2")
            .help("");

        let optional_recipient = recipient.long(RECIPIENT).short("r");

        let mint_nft_command = SubCommand::with_name(COMMAND_MINT_NFT)
            .args(&[
                nft_type,
                name,
                url,
                symbol,
                fees,
                owner,
                mint,
                optional_recipient,
                program_id,
                mainnet,
            ])
            .about("Creates a new NFT")
            .after_help(account_address_help);

        //
        // App
        //

        App::new(crate_name!())
            .about(crate_description!())
            .version(crate_version!())
            .subcommands(vec![
                balance_command,
                info_command,
                initialize_command,
                mint_command,
                mint_nft_command,
                transfer_command,
            ])
            .setting(AppSettings::SubcommandRequiredElseHelp)
    }

    fn get_matches(&self) -> (&'static str, &ArgMatches<'a>) {
        match self.matches.subcommand() {
            (COMMAND_BALANCE, Some(matcher)) => (COMMAND_BALANCE, matcher),
            (COMMAND_INFO, Some(matcher)) => (COMMAND_INFO, matcher),
            (COMMAND_INITIALIZE, Some(matcher)) => (COMMAND_INITIALIZE, matcher),
            (COMMAND_MINT, Some(matcher)) => (COMMAND_MINT, matcher),
            (COMMAND_MINT_NFT, Some(matcher)) => (COMMAND_MINT_NFT, matcher),
            (COMMAND_TRANSFER, Some(matcher)) => (COMMAND_TRANSFER, matcher),
            _ => unimplemented!(),
        }
    }

    pub fn command(&self) -> CliCommand {
        match self.get_matches().0 {
            COMMAND_BALANCE => CliCommand::Balance,
            COMMAND_INFO => CliCommand::Info,
            COMMAND_INITIALIZE => CliCommand::Initialize,
            COMMAND_MINT => CliCommand::Mint,
            COMMAND_MINT_NFT => CliCommand::MintNft,
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

    pub fn mint_args(&self) -> Result<MintNftArgs> {
        let matches = self.get_matches().1;
        let ui_fees = value_t_or_exit!(matches, FEES, f32);
        if !(0.0..=100.0).contains(&ui_fees) {
            return Err(CliError::FeesOutOfRange.into());
        }

        let fees = (ui_fees * 100.0).trunc() as u16;
        let name = matches.value_of(NAME).unwrap().to_owned();
        let symbol = matches.value_of(SYMBOL).unwrap().to_owned();
        let url = matches.value_of(URL).unwrap().to_owned();
        let nft_type_str = matches.value_of(NFT_TYPE).unwrap();
        let nft_type = NftType::try_from(nft_type_str).unwrap();

        Ok(MintNftArgs {
            nft_type,
            name,
            symbol,
            url,
            fees,
        })
    }

    fn get_signer(
        &self,
        key: &str,
    ) -> core::result::Result<Option<Box<dyn Signer>>, Box<dyn error::Error>> {
        let matches = self.get_matches().1;
        if matches.is_present(key) {
            let owner = matches.value_of(key).unwrap();
            let signer = signer_from_path(matches, owner, key, &mut None)?;
            Ok(Some(signer))
        } else {
            Ok(None)
        }
    }

    fn get_pubkey(&self, key: &str) -> Option<Pubkey> {
        let matches = self.get_matches().1;
        matches
            .value_of(key)
            .map(|_| pubkey_of(matches, key).unwrap())
    }

    pub fn recipient_pubkey(&self) -> Option<Pubkey> {
        self.get_pubkey(RECIPIENT)
    }

    pub fn recipient(&self) -> Result<Option<Box<dyn Signer>>> {
        self.get_signer(RECIPIENT)
            .map_err(|e| CliError::CannotGetRecipient(e.to_string()).into())
    }

    pub fn owner_pubkey(&self) -> Option<Pubkey> {
        self.get_pubkey(OWNER)
    }

    pub fn owner(&self) -> Result<Option<Box<dyn Signer>>> {
        self.get_signer(OWNER)
            .map_err(|e| CliError::CannotGetOwner(e.to_string()).into())
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

    pub fn fees(&self) -> UiFees {
        let matches = self.get_matches().1;
        UiFees {
            character: value_t_or_exit!(matches, FEES_CHARACTER, f64),
            pet: value_t_or_exit!(matches, FEES_PET, f64),
            emote: value_t_or_exit!(matches, FEES_EMOTE, f64),
            tileset: value_t_or_exit!(matches, FEES_TILESET, f64),
            item: value_t_or_exit!(matches, FEES_ITEM, f64),
        }
    }

    pub fn multiple_recipients(&self) -> Result<Vec<Recipient>> {
        let matches = self.get_matches().1;
        if !matches.is_present(RECIPIENT) {
            return Ok(Vec::new());
        }

        let recipients_pubkeys = pubkeys_of(matches, RECIPIENT).unwrap();
        let recipients_number = recipients_pubkeys.len();
        let mint_shares;
        let transaction_shares;

        if recipients_number == 1 {
            mint_shares = vec![100];
            transaction_shares = vec![100];
        } else {
            mint_shares = values_t_or_exit!(matches, MINT_SHARE, u8);
            transaction_shares = values_t_or_exit!(matches, TRANSACTION_SHARE, u8);
        }

        if mint_shares.len() != recipients_number || transaction_shares.len() != recipients_number {
            return Err(CliError::NotEnoughShares.into());
        }

        Ok(recipients_pubkeys
            .iter()
            .zip(mint_shares)
            .zip(transaction_shares)
            .map(|((pubkey, mint_share), transaction_share)| Recipient {
                address: *pubkey,
                mint_share,
                transaction_share,
            })
            .collect())
    }

    pub fn program_id(&self) -> Pubkey {
        let matches = self.get_matches().1;
        pubkey_of(matches, PROGRAM_ID).unwrap_or(chill_api::ID)
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
    use rand::{prelude::SliceRandom, thread_rng, Rng};
    use solana_sdk::signature::{read_keypair_file, Keypair};

    const MINT_PATH: &str = "../localnet/mint.pubkey.localnet";
    const OWNER_PATH: &str = "../localnet/owner.json";
    const RECIPIENT_PATH: &str = "../localnet/recipient.json";

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

    fn recipient() -> Keypair {
        read_keypair_file(RECIPIENT_PATH).unwrap()
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
        let cli = get_cli(&[COMMAND_TRANSFER, RECIPIENT_PATH, amount]);

        assert!(!cli.mainnet());
        assert!(cli.mint().unwrap().is_none());
        assert_eq!(cli.owner().unwrap(), None);
        assert_eq!(cli.recipient_pubkey(), Some(recipient().pubkey()));
        assert_eq!(cli.ui_amount().to_string(), amount);

        let recipient_pubkey = recipient().pubkey().to_string();
        let cli = get_cli(&[COMMAND_TRANSFER, &recipient_pubkey, amount]);
        assert_eq!(cli.recipient_pubkey(), Some(recipient().pubkey()));

        let all_args_string = format!(
            "{0} {1} {2} --{3} {4} --{5} {6} --{7}",
            COMMAND_TRANSFER, RECIPIENT_PATH, amount, MINT, MINT_PATH, OWNER, OWNER_PATH, MAINNET
        );

        let args = all_args_string.split(' ').collect::<Vec<&str>>();
        let cli = get_cli(&args);

        assert!(cli.mainnet());
        assert_eq!(cli.mint().unwrap(), Some(mint_pubkey()));
        assert_eq!(cli.owner().unwrap().unwrap().pubkey(), owner().pubkey());
        assert_eq!(cli.recipient_pubkey(), Some(recipient().pubkey()));
        assert_eq!(cli.ui_amount().to_string(), amount);
    }

    #[test]
    fn initialize() {
        let mut rng = thread_rng();
        let fees = UiFees {
            character: rng.gen(),
            pet: rng.gen(),
            emote: rng.gen(),
            tileset: rng.gen(),
            item: rng.gen(),
        };

        let recipient = Recipient {
            address: Keypair::new().pubkey(),
            mint_share: 100,
            transaction_share: 100,
        };

        let args_str = format!(
            "{0} --{1} {2} --{3} {4} --{5} {6} --{7} {8} --{9} {10} --{11} {12}",
            COMMAND_INITIALIZE,
            FEES_CHARACTER,
            fees.character,
            FEES_EMOTE,
            fees.emote,
            FEES_ITEM,
            fees.item,
            FEES_PET,
            fees.pet,
            FEES_TILESET,
            fees.tileset,
            RECIPIENT,
            recipient.address
        );

        let args = args_str.split(' ').collect::<Vec<_>>();
        let cli = get_cli(&args);

        assert!(!cli.mainnet());
        assert!(cli.mint().unwrap().is_none());
        assert_eq!(cli.owner().unwrap(), None);
        assert_eq!(cli.multiple_recipients().unwrap(), vec![recipient]);
        assert_eq!(cli.fees(), fees);

        let recipients = (0..Config::MAX_RECIPIENT_NUMBER)
            .map(|_| Recipient {
                address: Keypair::new().pubkey(),
                mint_share: rng.gen_range(0..=100),
                transaction_share: rng.gen_range(0..=100),
            })
            .collect::<Vec<_>>();

        let mut args_str = format!(
            "{0} --{1} {2} --{3} {4} --{5} {6} --{7} {8} --{9} {10} --{11} --{12} {13} --{14} {15}",
            COMMAND_INITIALIZE,
            FEES_CHARACTER,
            fees.character,
            FEES_EMOTE,
            fees.emote,
            FEES_ITEM,
            fees.item,
            FEES_PET,
            fees.pet,
            FEES_TILESET,
            fees.tileset,
            MAINNET,
            MINT,
            MINT_PATH,
            OWNER,
            OWNER_PATH
        );

        for recipient in recipients.iter() {
            let recipient_args = format!(
                " --{0} {1} --{2} {3} --{4} {5}",
                RECIPIENT,
                recipient.address,
                MINT_SHARE,
                recipient.mint_share,
                TRANSACTION_SHARE,
                recipient.transaction_share
            );

            args_str.push_str(&recipient_args);
        }

        let args = args_str.split(' ').collect::<Vec<_>>();
        let cli = get_cli(&args);

        assert!(cli.mainnet());
        assert_eq!(cli.mint().unwrap(), Some(mint_pubkey()));
        assert_eq!(cli.owner().unwrap().unwrap().pubkey(), owner().pubkey());
        assert_eq!(cli.multiple_recipients().unwrap(), recipients);
        assert_eq!(cli.fees(), fees);
    }

    #[test]
    fn mint_nft() {
        let nft_types = &["character", "pet", "emote", "tileset", "item"];

        let mut rng = thread_rng();
        let nft_type = nft_types.choose(&mut rng).unwrap();
        let fees = rng.gen_range(0..=10000);
        let ui_fees = fees as f32 / 100.0;

        let mint_args = MintNftArgs {
            nft_type: NftType::try_from(*nft_type).unwrap(),
            name: format!("NAME_{0}", rng.gen_range(0..100)),
            symbol: format!("SYM_{0}", rng.gen_range(0..100)),
            url: format!("https://arweave.com/{0}", Keypair::new().pubkey()),
            fees,
        };

        let args_str = format!(
            "{0} {1} {2} {3}",
            COMMAND_MINT_NFT, nft_type, mint_args.name, mint_args.url,
        );
        let args = args_str.split(' ').collect::<Vec<_>>();
        let cli = get_cli(&args);
        let cli_mint_args = cli.mint_args().unwrap();

        assert!(!cli.mainnet());
        assert!(cli.mint().unwrap().is_none());
        assert_eq!(cli.owner().unwrap(), None);
        assert_eq!(cli.program_id(), chill_api::ID);
        assert_eq!(cli.recipient().unwrap(), None);
        assert_eq!(cli.recipient_pubkey(), None);
        assert_eq!(cli_mint_args.name, mint_args.name);
        assert_eq!(cli_mint_args.url, mint_args.url);
        assert_eq!(cli_mint_args.nft_type, mint_args.nft_type);

        let args_str = format!(
            "{0} {1} {2} {3} --{4} {5} --{6} {7}",
            COMMAND_MINT_NFT,
            nft_type,
            mint_args.name,
            mint_args.url,
            RECIPIENT,
            recipient().pubkey(),
            FEES,
            101
        );

        let args = args_str.split(' ').collect::<Vec<_>>();
        let cli = get_cli(&args);
        assert!(cli.recipient().is_err());
        assert!(cli.mint_args().is_err());
        assert_eq!(cli.recipient_pubkey(), Some(recipient().pubkey()));

        let new_program_id = Keypair::new().pubkey();
        let args_str = format!(
            "{0} {1} {2} {3} --{4} {5} --{6} {7} --{8} {9} --{10} {11} --{12} {13} --{14} {15} --{16}",
            COMMAND_MINT_NFT,
            nft_type,
            mint_args.name,
            mint_args.url,
            SYMBOL,
            mint_args.symbol,
            FEES,
            ui_fees,
            OWNER,
            OWNER_PATH,
            MINT,
            MINT_PATH,
            RECIPIENT,
            RECIPIENT_PATH,
            PROGRAM_ID,
            new_program_id,
            MAINNET
        );

        let args = args_str.split(' ').collect::<Vec<_>>();
        let cli = get_cli(&args);
        let cli_mint_args = cli.mint_args().unwrap();

        assert!(cli.mainnet());
        assert_eq!(cli_mint_args, mint_args);
        assert_eq!(cli.mint().unwrap(), Some(mint_pubkey()));
        assert_eq!(cli.owner().unwrap().unwrap().pubkey(), owner().pubkey());
        assert_eq!(
            cli.recipient().unwrap().unwrap().pubkey(),
            recipient().pubkey()
        );
        assert_eq!(cli.recipient_pubkey(), Some(recipient().pubkey()));
        assert_eq!(cli.program_id(), new_program_id);
    }
}
