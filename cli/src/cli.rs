use crate::error::{CliError, Result};
use anchor_client::{
    solana_sdk::{pubkey::Pubkey, signature::Signer},
    Cluster,
};
use chill_nft::{
    state::{Config, NftType, Recipient, UiFees},
    utils::NftArgs,
};
use clap::{
    crate_description, crate_name, crate_version, value_t_or_exit, values_t_or_exit, App,
    AppSettings, Arg, ArgMatches, SubCommand,
};
use lazy_static::lazy_static;
use solana_clap_utils::{
    input_parsers::{pubkey_of, pubkeys_of, unix_timestamp_from_rfc3339_datetime},
    input_validators::{
        is_pubkey, is_pubkey_or_keypair, is_rfc3339_datetime, is_url_or_moniker, is_valid_signer,
        normalize_to_url_if_moniker,
    },
    keypair::signer_from_path,
};
use std::{error, fs, path::Path, rc::Rc, str::FromStr};

lazy_static! {
    pub static ref DEFAULT_KEYPAIR: Option<String> = {
        dirs::home_dir().map(|mut path| {
            path.extend(&[".config", "solana", "id.json"]);
            path.to_str().unwrap().to_string()
        })
    };
}

const COMMAND_BALANCE: &str = "balance";
const COMMAND_CREATE_WALLET: &str = "create-wallet";
const COMMAND_INFO: &str = "info";
const COMMAND_INITIALIZE: &str = "initialize";
const COMMAND_MINT: &str = "mint";
const COMMAND_MINT_NFT: &str = "mint-nft";
const COMMAND_TRANSFER: &str = "transfer";
const COMMAND_UPDATE_NFT: &str = "update-nft";
const COMMAND_WITHDRAW_FT: &str = "withdraw-ft";
const COMMAND_WITHDRAW_LAMPORTS: &str = "withdraw-lamports";
const COMMAND_WITHDRAW_NFT: &str = "withdraw-nft";

const COMMAND_STAKING: &str = "staking";
const COMMAND_ADD_REWARD_TOKENS: &str = "add-reward-tokens";
const COMMAND_STAKING_INITIALIZE: &str = "staking-initialize";
const COMMAND_STAKING_ADD_REWARD_TOKENS: &str = "staking-add-reward-tokens";

const ACCOUNT: &str = "account";
const AMOUNT: &str = "amount";
const AUTHORITY: &str = "authority";
const CREATOR: &str = "creator";
const DECIMALS: &str = "decimals";
const END_TIMESTAMP: &str = "end";
const FEES: &str = "fees";
const FEES_CHARACTER: &str = "character";
const FEES_EMOTE: &str = "emote";
const FEES_ITEM: &str = "item";
const FEES_PET: &str = "pet";
const FEES_TILESET: &str = "tileset";
const FEES_WORLD: &str = "world";
const MINT: &str = "mint-address";
const MINT_SHARE: &str = "mint-share";
const MIN_STAKE_SIZE: &str = "min-stake-size";
const NAME: &str = "name";
const NFT_TYPE: &str = "type";
const PAYER: &str = "payer";
const PRIMARY_WALLET: &str = "primary-wallet";
const RECIPIENT: &str = "recipient";
const RPC_URL: &str = "url";
const SAVE_PATH: &str = "save-path";
const STAKING_INFO: &str = "staking-info";
const START_TIMESTAMP: &str = "start";
const SYMBOL: &str = "symbol";
const TRANSACTION_SHARE: &str = "transaction-share";
const URI: &str = "uri";

pub enum CliCommand {
    Balance,
    CreateWallet,
    Info,
    Initialize,
    Mint,
    MintNft,
    StakingAddRewardTokens,
    StakingInitialize,
    Transfer,
    UpdateNft,
    WithdrawFt,
    WithdrawLamports,
    WithdrawNft,
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

        let mut primary_wallet = Arg::with_name(PRIMARY_WALLET)
            .long(PRIMARY_WALLET)
            .short("k")
            .takes_value(true)
            .value_name(account_address)
            .validator(is_valid_signer)
            .help("Primary wallet keypair");

        let mut payer = Arg::with_name(PAYER)
            .long(PAYER)
            .short("P")
            .takes_value(true)
            .value_name(account_address)
            .validator(is_valid_signer)
            .help("The account used to pay for the transactions");

        let mut account = Arg::with_name(ACCOUNT)
            .long(ACCOUNT)
            .short("a")
            .takes_value(true)
            .value_name(account_address)
            .validator(is_pubkey_or_keypair)
            .help("Pubkey or keypair of the desired account");

        let mut authority = Arg::with_name(AUTHORITY)
            .long(AUTHORITY)
            .short("A")
            .takes_value(true)
            .value_name(account_address)
            .validator(is_valid_signer)
            .help("Proxy wallet authority");

        let mut recipient = Arg::with_name(RECIPIENT)
            .long(RECIPIENT)
            .short("r")
            .takes_value(true)
            .value_name(account_address)
            .validator(is_pubkey_or_keypair)
            .help("An account that will receive tokens");

        if let Some(ref file) = *DEFAULT_KEYPAIR {
            account = account.required(false).default_value(file);
            authority = authority.required(false).default_value(file);
            payer = payer.required(false).default_value(file);
            primary_wallet = primary_wallet.required(false).default_value(file);
            recipient = recipient.required(false).default_value(file);
        } else {
            account = account.required(true);
            authority = authority.required(true);
            payer = payer.required(true);
            primary_wallet = primary_wallet.required(true);
            recipient = recipient.required(true);
        }

        let required_mint = Arg::with_name(MINT)
            .required(true)
            .takes_value(true)
            .value_name("MINT_ADDRESS")
            .validator(is_mint_pubkey)
            .help("Mint pubkey");

        let mint = required_mint.clone().required(false).short("m").long(MINT);

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

        let rpc = Arg::with_name(RPC_URL)
            .long(RPC_URL)
            .short("u")
            .global(true)
            .takes_value(true)
            .validator(is_url_or_moniker)
            .default_value("devnet")
            .help("URL for Solana's JSON RPC or moniker (or their first letter)");

        let mint_command = SubCommand::with_name(COMMAND_MINT)
            .args(&[
                amount_mint,
                decimals,
                mint.clone(),
                recipient.clone(),
                primary_wallet.clone(),
                payer.clone(),
                save_path,
            ])
            .about(
                "Creates mint and token accounts, if they don't exist, and mint a number of tokens",
            )
            .after_help(account_address_help);

        let balance_command = SubCommand::with_name(COMMAND_BALANCE)
            .args(&[mint.clone(), account.clone()])
            .about("Prints the balance of the token account")
            .after_help(account_address_help);

        let info_command = SubCommand::with_name(COMMAND_INFO)
            .args(&[mint.clone()])
            .about("Prints the information about smart-contract state");

        let transfer_command = SubCommand::with_name(COMMAND_TRANSFER)
            .args(&[
                recipient.clone(),
                amount_transfer.clone(),
                mint.clone(),
                primary_wallet.clone(),
                payer.clone(),
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
            .validator(is_pubkey_or_keypair);

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

        let fees_world = Arg::with_name(FEES_WORLD)
            .long(FEES_WORLD)
            .short("w")
            .required(true)
            .takes_value(true)
            .value_name(fees_value_name)
            .help("Fees for mint a world NFT");

        let initialize_command = SubCommand::with_name(COMMAND_INITIALIZE)
            .args(&[
                mint.clone(),
                primary_wallet.clone(),
                payer.clone(),
                multiple_recipients,
                mint_share,
                transaction_share,
                fees_character,
                fees_pet,
                fees_emote,
                fees_tileset,
                fees_item,
                fees_world,
            ])
            .about("Initializes the Chill smart-contract")
            .after_help(account_address_help);

        //
        // MintNft
        //

        let nft_type = Arg::with_name(NFT_TYPE)
            .takes_value(true)
            .value_name("TYPE")
            .possible_values(&["character", "pet", "emote", "tileset", "item", "world"])
            .required(true)
            .help("Nft type");

        let name = Arg::with_name(NAME)
            .takes_value(true)
            .value_name("NAME")
            .required(true)
            .help("Name of the NFT");

        let uri = Arg::with_name(URI)
            .takes_value(true)
            .value_name("URI")
            .required(true)
            .help("URI to the a NFT image");

        let symbol = Arg::with_name(SYMBOL)
            .long(SYMBOL)
            .short("s")
            .takes_value(true)
            .value_name("SYMBOL")
            .default_value("CHILL")
            .help("Symbol of the NFT");

        let creator = Arg::with_name(CREATOR)
            .long(CREATOR)
            .short("c")
            .takes_value(true)
            .value_name(account_address)
            .validator(is_pubkey_or_keypair)
            .help("An account that will appear in the creators list");

        let fees = Arg::with_name(FEES)
            .long(FEES)
            .short("f")
            .takes_value(true)
            .value_name("PERCENT")
            .default_value("2");

        let mint_nft_command = SubCommand::with_name(COMMAND_MINT_NFT)
            .args(&[
                fees.clone(),
                mint.clone(),
                nft_type,
                name.clone(),
                creator,
                payer.clone(),
                recipient.clone(),
                primary_wallet.clone(),
                symbol.clone(),
                uri.clone(),
            ])
            .about("Creates a new NFT")
            .after_help(account_address_help);

        //
        // UpdateNft
        //

        let update_nft_command = SubCommand::with_name(COMMAND_UPDATE_NFT)
            .args(&[
                fees.clone(),
                required_mint.clone(),
                name,
                payer.clone(),
                primary_wallet.clone(),
                symbol,
                uri,
            ])
            .about("Updates an NFT metadata")
            .after_help(account_address_help);

        //
        // Proxy wallets
        //

        let create_wallet_command = SubCommand::with_name(COMMAND_CREATE_WALLET)
            .args(&[primary_wallet.clone(), account.clone(), payer.clone()])
            .about("Creates a proxy wallet")
            .after_help(account_address_help);

        let withdraw_lamports_command = SubCommand::with_name(COMMAND_WITHDRAW_LAMPORTS)
            .args(&[
                account.clone(),
                amount_transfer.clone(),
                authority.clone(),
                payer.clone(),
                primary_wallet.clone(),
                recipient.clone(),
            ])
            .about("Withdraws lamports from proxy wallet")
            .after_help(account_address_help);

        let withdraw_ft_command = SubCommand::with_name(COMMAND_WITHDRAW_FT)
            .args(&[
                account.clone(),
                amount_transfer.clone(),
                authority.clone(),
                payer.clone(),
                primary_wallet.clone(),
                recipient.clone(),
                mint.clone(),
            ])
            .about("Withdraws FT from proxy wallet")
            .after_help(account_address_help);

        let withdraw_nft_command = SubCommand::with_name(COMMAND_WITHDRAW_NFT)
            .args(&[
                account.clone(),
                authority.clone(),
                payer.clone(),
                primary_wallet.clone(),
                recipient.clone(),
                required_mint.clone(),
            ])
            .about("Withdraws NFT from proxy wallet")
            .after_help(account_address_help);

        //
        // Staking
        //

        let start_timestamp = Arg::with_name(START_TIMESTAMP)
            .long(START_TIMESTAMP)
            .short("s")
            .required(true)
            .takes_value(true)
            .value_name("TIMESTAMP")
            .validator(is_rfc3339_datetime)
            .help("Staking start time");

        let end_timestamp = Arg::with_name(END_TIMESTAMP)
            .long(END_TIMESTAMP)
            .short("e")
            .required(true)
            .takes_value(true)
            .value_name("TIMESTAMP")
            .validator(is_rfc3339_datetime)
            .help("Staking end time");

        let staking_info = Arg::with_name(STAKING_INFO)
            .required(true)
            .takes_value(true)
            .value_name("PUBKEY")
            .validator(is_pubkey)
            .help("StakingInfo pubkey");

        let min_stake_size = Arg::with_name(MIN_STAKE_SIZE)
            .long(MIN_STAKE_SIZE)
            .required(true)
            .takes_value(true)
            .value_name("SOL")
            .default_value("0")
            .help("Minimum stake size");

        let staking_initialize_command = SubCommand::with_name(COMMAND_INITIALIZE)
            .args(&[
                primary_wallet.clone(),
                mint.clone(),
                payer.clone(),
                min_stake_size,
                start_timestamp,
                end_timestamp,
            ])
            .about("Initializes staking")
            .after_help(account_address_help);

        let staking_add_reward_tokens = SubCommand::with_name(COMMAND_ADD_REWARD_TOKENS)
            .args(&[primary_wallet, mint, payer, amount_transfer, staking_info])
            .about("Adds reward tokens to staking")
            .after_help(account_address_help);

        let staking_command = SubCommand::with_name(COMMAND_STAKING)
            .about("Manages staking")
            .setting(AppSettings::SubcommandRequiredElseHelp)
            .subcommands(vec![staking_initialize_command, staking_add_reward_tokens]);

        App::new(crate_name!())
            .about(crate_description!())
            .version(crate_version!())
            .arg(rpc)
            .subcommands(vec![
                staking_command,
                balance_command,
                info_command,
                initialize_command,
                mint_command,
                mint_nft_command,
                update_nft_command,
                transfer_command,
                create_wallet_command,
                withdraw_lamports_command,
                withdraw_ft_command,
                withdraw_nft_command,
            ])
            .setting(AppSettings::SubcommandRequiredElseHelp)
    }

    fn get_matches(&self) -> (&'static str, &ArgMatches<'a>) {
        match self.matches.subcommand() {
            (COMMAND_BALANCE, Some(matcher)) => (COMMAND_BALANCE, matcher),
            (COMMAND_CREATE_WALLET, Some(matcher)) => (COMMAND_CREATE_WALLET, matcher),
            (COMMAND_INFO, Some(matcher)) => (COMMAND_INFO, matcher),
            (COMMAND_INITIALIZE, Some(matcher)) => (COMMAND_INITIALIZE, matcher),
            (COMMAND_MINT, Some(matcher)) => (COMMAND_MINT, matcher),
            (COMMAND_MINT_NFT, Some(matcher)) => (COMMAND_MINT_NFT, matcher),
            (COMMAND_UPDATE_NFT, Some(matcher)) => (COMMAND_UPDATE_NFT, matcher),
            (COMMAND_TRANSFER, Some(matcher)) => (COMMAND_TRANSFER, matcher),
            (COMMAND_WITHDRAW_FT, Some(matcher)) => (COMMAND_WITHDRAW_FT, matcher),
            (COMMAND_WITHDRAW_LAMPORTS, Some(matcher)) => (COMMAND_WITHDRAW_LAMPORTS, matcher),
            (COMMAND_WITHDRAW_NFT, Some(matcher)) => (COMMAND_WITHDRAW_NFT, matcher),
            (COMMAND_STAKING, Some(matcher)) => match matcher.subcommand() {
                (COMMAND_INITIALIZE, Some(matcher)) => (COMMAND_STAKING_INITIALIZE, matcher),
                (COMMAND_ADD_REWARD_TOKENS, Some(matcher)) => {
                    (COMMAND_STAKING_ADD_REWARD_TOKENS, matcher)
                }
                _ => unimplemented!(),
            },
            _ => unimplemented!(),
        }
    }

    pub fn command(&self) -> CliCommand {
        match self.get_matches().0 {
            COMMAND_BALANCE => CliCommand::Balance,
            COMMAND_CREATE_WALLET => CliCommand::CreateWallet,
            COMMAND_INFO => CliCommand::Info,
            COMMAND_INITIALIZE => CliCommand::Initialize,
            COMMAND_MINT => CliCommand::Mint,
            COMMAND_MINT_NFT => CliCommand::MintNft,
            COMMAND_STAKING_ADD_REWARD_TOKENS => CliCommand::StakingAddRewardTokens,
            COMMAND_STAKING_INITIALIZE => CliCommand::StakingInitialize,
            COMMAND_TRANSFER => CliCommand::Transfer,
            COMMAND_UPDATE_NFT => CliCommand::UpdateNft,
            COMMAND_WITHDRAW_FT => CliCommand::WithdrawFt,
            COMMAND_WITHDRAW_LAMPORTS => CliCommand::WithdrawLamports,
            COMMAND_WITHDRAW_NFT => CliCommand::WithdrawNft,
            _ => unimplemented!(),
        }
    }

    fn timestamp(&self, name: &str) -> u64 {
        let matches = self.get_matches().1;
        unix_timestamp_from_rfc3339_datetime(matches, name)
            .unwrap()
            .try_into()
            .unwrap()
    }

    pub fn start_time(&self) -> u64 {
        self.timestamp(START_TIMESTAMP)
    }

    pub fn end_time(&self) -> u64 {
        self.timestamp(END_TIMESTAMP)
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

    pub fn nft_type(&self) -> NftType {
        let matches = self.get_matches().1;
        let nft_type_str = matches.value_of(NFT_TYPE).unwrap();
        NftType::try_from(nft_type_str).unwrap()
    }

    pub fn mint_args(&self) -> Result<NftArgs> {
        let matches = self.get_matches().1;
        let ui_fees = value_t_or_exit!(matches, FEES, f32);
        if !(0.0..=100.0).contains(&ui_fees) {
            return Err(CliError::FeesOutOfRange.into());
        }

        let fees = (ui_fees * 100.0).round() as u16;
        let name = matches.value_of(NAME).unwrap().to_owned();
        let symbol = matches.value_of(SYMBOL).unwrap().to_owned();
        let uri = matches.value_of(URI).unwrap().to_owned();

        Ok(NftArgs {
            name,
            symbol,
            uri,
            fees,
        })
    }

    fn get_signer(&self, key: &str) -> core::result::Result<Rc<dyn Signer>, Box<dyn error::Error>> {
        let matches = self.get_matches().1;
        let signer_path = matches.value_of(key).unwrap();
        signer_from_path(matches, signer_path, key, &mut None).map(Rc::from)
    }

    fn get_pubkey(&self, key: &str) -> Pubkey {
        let matches = self.get_matches().1;
        pubkey_of(matches, key).unwrap()
    }

    pub fn account(&self) -> Pubkey {
        self.get_pubkey(ACCOUNT)
    }

    pub fn recipient(&self) -> Pubkey {
        self.get_pubkey(RECIPIENT)
    }

    pub fn creator(&self) -> Option<Pubkey> {
        let matches = self.get_matches().1;
        if !matches.is_present(CREATOR) {
            return None;
        }
        Some(pubkey_of(matches, RECIPIENT).unwrap())
    }

    pub fn primary_wallet_pubkey(&self) -> Pubkey {
        self.get_pubkey(PRIMARY_WALLET)
    }

    pub fn primary_wallet(&self) -> Result<Rc<dyn Signer>> {
        self.get_signer(PRIMARY_WALLET)
            .map_err(|e| CliError::CannotGetPrimaryWallet(e.to_string()).into())
    }

    pub fn payer(&self) -> Result<Rc<dyn Signer>> {
        self.get_signer(PAYER)
            .map_err(|e| CliError::CannotGetPayer(e.to_string()).into())
    }

    pub fn authority(&self) -> Result<Rc<dyn Signer>> {
        self.get_signer(AUTHORITY)
            .map_err(|e| CliError::CannotGetAuthority(e.to_string()).into())
    }

    pub fn min_stake_size(&self) -> f64 {
        let matches = self.get_matches().1;
        value_t_or_exit!(matches, MIN_STAKE_SIZE, f64)
    }

    pub fn staking_info(&self) -> Pubkey {
        self.get_pubkey(STAKING_INFO)
    }

    fn default_mint_file(&self) -> &str {
        match self.cluster() {
            Cluster::Testnet => "mint.testnet.pubkey",
            Cluster::Mainnet => "mint.mainnet.pubkey",
            Cluster::Devnet => "mint.devnet.pubkey",
            Cluster::Localnet => "mint.localnet.pubkey",
            Cluster::Debug => "mint.debug.pubkey",
            Cluster::Custom(_, _) => "mint.url.pubkey",
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

    pub fn fees(&self) -> UiFees {
        let matches = self.get_matches().1;
        UiFees {
            character: value_t_or_exit!(matches, FEES_CHARACTER, f64),
            pet: value_t_or_exit!(matches, FEES_PET, f64),
            emote: value_t_or_exit!(matches, FEES_EMOTE, f64),
            tileset: value_t_or_exit!(matches, FEES_TILESET, f64),
            item: value_t_or_exit!(matches, FEES_ITEM, f64),
            world: value_t_or_exit!(matches, FEES_WORLD, f64),
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

    pub fn cluster(&self) -> Cluster {
        let matches = self.get_matches().1;
        let cluster = matches.value_of(RPC_URL).unwrap();
        Cluster::from_str(cluster).unwrap()
    }

    pub fn rpc_url(&self) -> String {
        let matches = self.get_matches().1;
        let url_or_moniker = matches.value_of(RPC_URL).unwrap();
        normalize_to_url_if_moniker(url_or_moniker)
    }
}
