use crate::error::ChillNftError;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    borsh::try_from_slice_unchecked,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};
use spl_token::{amount_to_ui_amount, ui_amount_to_amount};

pub const AUTHORITY_SHARE: u8 = 2;
pub const CONFIG_SEED: &str = "config";
pub const CHILL_METADATA_SEED: &str = "chill-metadata";

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize)]
pub enum StateType {
    Uninitialized,
    Config,
    ChillNftMetadata,
}

impl StateType {
    pub const LEN: usize = 1;
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize)]
pub enum NftType {
    Character,
    Pet,
    Emote,
    Tileset,
    Item,
}

impl NftType {
    pub const LEN: usize = 1;
}
impl TryFrom<&str> for NftType {
    type Error = String;

    fn try_from(string: &str) -> Result<Self, Self::Error> {
        match string {
            "character" => Ok(NftType::Character),
            "pet" => Ok(NftType::Pet),
            "emote" => Ok(NftType::Emote),
            "tileset" => Ok(NftType::Tileset),
            "item" => Ok(NftType::Item),
            _ => Err("Wrong nft type".to_owned()),
        }
    }
}

#[repr(C)]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct UiFees {
    pub character: f64,
    pub pet: f64,
    pub emote: f64,
    pub tileset: f64,
    pub item: f64,
}

#[repr(C)]
#[derive(Clone, Debug, Default, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct Fees {
    pub character: u64,
    pub pet: u64,
    pub emote: u64,
    pub tileset: u64,
    pub item: u64,
}

impl Fees {
    pub const LEN: usize = 8 * 5;

    pub fn from_ui(ui_fees: UiFees, decimals: u8) -> Fees {
        Fees {
            character: ui_amount_to_amount(ui_fees.character, decimals),
            pet: ui_amount_to_amount(ui_fees.pet, decimals),
            emote: ui_amount_to_amount(ui_fees.emote, decimals),
            tileset: ui_amount_to_amount(ui_fees.tileset, decimals),
            item: ui_amount_to_amount(ui_fees.item, decimals),
        }
    }

    pub fn to_ui(&self, decimals: u8) -> UiFees {
        UiFees {
            character: amount_to_ui_amount(self.character, decimals),
            pet: amount_to_ui_amount(self.pet, decimals),
            emote: amount_to_ui_amount(self.emote, decimals),
            tileset: amount_to_ui_amount(self.tileset, decimals),
            item: amount_to_ui_amount(self.item, decimals),
        }
    }

    pub fn of(&self, nft_type: NftType) -> u64 {
        match nft_type {
            NftType::Character => self.character,
            NftType::Pet => self.pet,
            NftType::Emote => self.emote,
            NftType::Tileset => self.tileset,
            NftType::Item => self.item,
        }
    }
}

#[repr(C)]
#[derive(Clone, Debug, Default, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct Recipient {
    pub address: Pubkey,
    pub mint_share: u8,
    pub transaction_share: u8,
}

impl Recipient {
    pub const LEN: usize = 32 + 1 + 1;
}

#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct Config {
    state_type: StateType,
    pub mint: Pubkey,
    pub fees: Fees,
    pub recipients: Vec<Recipient>,
}

#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct ChillNftMetadata {
    state_type: StateType,
    pub nft_type: NftType,
}

impl Sealed for Config {}
impl Sealed for ChillNftMetadata {}

impl IsInitialized for Config {
    fn is_initialized(&self) -> bool {
        self.state_type == StateType::Config
    }
}

impl IsInitialized for ChillNftMetadata {
    fn is_initialized(&self) -> bool {
        self.state_type == StateType::ChillNftMetadata
    }
}

impl Pack for Config {
    const LEN: usize = StateType::LEN
        + 32
        + Fees::LEN
        + Self::VECTOR_PREFIX
        + Self::MAX_RECIPIENT_NUMBER * Recipient::LEN;

    fn pack_into_slice(&self, mut dst: &mut [u8]) {
        self.serialize(&mut dst).unwrap();
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        try_from_slice_unchecked(src).map_err(|e| e.into())
    }
}

impl Pack for ChillNftMetadata {
    const LEN: usize = StateType::LEN + NftType::LEN;

    fn pack_into_slice(&self, mut dst: &mut [u8]) {
        self.serialize(&mut dst).unwrap();
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        try_from_slice_unchecked(src).map_err(|e| e.into())
    }
}

impl Config {
    const VECTOR_PREFIX: usize = 4;

    pub const MAX_RECIPIENT_NUMBER: usize = 3;

    pub fn check_recipients(recipients: &[Recipient]) -> Result<(), ChillNftError> {
        if recipients.len() > Self::MAX_RECIPIENT_NUMBER {
            return Err(ChillNftError::MaximumRecipientsNumberExceeded);
        }

        if !recipients.is_empty() {
            let mint_share_sum = recipients.iter().map(|r| r.mint_share).sum::<u8>();
            let transaction_share_sum = recipients.iter().map(|r| r.mint_share).sum::<u8>();
            if mint_share_sum != 100 || transaction_share_sum != 100 {
                return Err(ChillNftError::InvalidShares);
            }
        }

        Ok(())
    }

    pub fn new(
        mint: &Pubkey,
        fees: Fees,
        recipients: Vec<Recipient>,
    ) -> Result<Self, ChillNftError> {
        Self::check_recipients(&recipients)?;

        Ok(Self {
            state_type: StateType::Config,
            mint: *mint,
            fees,
            recipients,
        })
    }
}

impl ChillNftMetadata {
    pub fn new(nft_type: NftType) -> Self {
        Self {
            state_type: StateType::ChillNftMetadata,
            nft_type,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            state_type: StateType::Config,
            mint: Pubkey::default(),
            fees: Fees::default(),
            recipients: Vec::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use solana_program::borsh::try_from_slice_unchecked;
    use solana_sdk::{signature::Keypair, signer::Signer};

    use super::*;

    fn get_recipients(number: u8) -> Vec<Recipient> {
        if number == 0 {
            return Vec::new();
        }

        let mut remaining_share = 100;
        let share = 100 / number - 1;

        let mut recipients = Vec::with_capacity(number.into());
        for _ in 1..number {
            let recipient = Recipient {
                address: Keypair::new().pubkey(),
                mint_share: share,
                transaction_share: share,
            };

            assert_eq!(recipient.try_to_vec().unwrap().len(), Recipient::LEN);

            recipients.push(recipient);
            remaining_share -= share;
        }

        let last_recipient = Recipient {
            address: Keypair::new().pubkey(),
            mint_share: remaining_share,
            transaction_share: remaining_share,
        };

        recipients.push(last_recipient);
        recipients
    }

    #[test]
    fn config() {
        let mint = Keypair::new().pubkey();
        let fees = Fees::default();

        assert_eq!(fees.try_to_vec().unwrap().len(), Fees::LEN);

        for i in 0..=Config::MAX_RECIPIENT_NUMBER {
            let mut recipients = get_recipients(i as u8);
            let config = Config::new(&mint, fees.clone(), recipients.clone()).unwrap();

            let mut buffer = [0; Config::LEN];
            config.serialize(&mut buffer.as_mut()).unwrap();

            let unpacked_config = try_from_slice_unchecked(&buffer).unwrap();
            assert_eq!(config, unpacked_config);

            if i == Config::MAX_RECIPIENT_NUMBER {
                assert_eq!(config.try_to_vec().unwrap().len(), Config::LEN);
            } else {
                assert!(config.try_to_vec().unwrap().len() <= Config::LEN);
            }

            if !recipients.is_empty() {
                recipients[0].mint_share -= 1;
                let config_wrong_share = Config::new(&mint, fees.clone(), recipients);
                assert!(config_wrong_share.is_err());
            }
        }

        let recipients = get_recipients(Config::MAX_RECIPIENT_NUMBER as u8 + 1);
        let config_overflow = Config::new(&mint, fees, recipients);
        assert!(config_overflow.is_err());
    }
}