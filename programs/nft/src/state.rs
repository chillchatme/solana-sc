use anchor_lang::prelude::*;
use spl_token::{amount_to_ui_amount, ui_amount_to_amount};

pub const DESCRIMINATOR_LEN: usize = 8;
pub const VECTOR_PREFIX_LEN: usize = 4;
pub const AUTHORITY_SHARE: u8 = 2;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct UiFees {
    pub character: f64,
    pub pet: f64,
    pub emote: f64,
    pub tileset: f64,
    pub item: f64,
    pub world: f64,
}

#[repr(C)]
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct Fees {
    pub character: u64,
    pub pet: u64,
    pub emote: u64,
    pub tileset: u64,
    pub item: u64,
    pub world: u64,
}

impl Fees {
    pub const LEN: usize = 8 + 8 + 8 + 8 + 8 + 8;

    pub fn from_ui(ui_fees: UiFees, decimals: u8) -> Fees {
        Fees {
            character: ui_amount_to_amount(ui_fees.character, decimals),
            pet: ui_amount_to_amount(ui_fees.pet, decimals),
            emote: ui_amount_to_amount(ui_fees.emote, decimals),
            tileset: ui_amount_to_amount(ui_fees.tileset, decimals),
            item: ui_amount_to_amount(ui_fees.item, decimals),
            world: ui_amount_to_amount(ui_fees.world, decimals),
        }
    }

    pub fn to_ui(&self, decimals: u8) -> UiFees {
        UiFees {
            character: amount_to_ui_amount(self.character, decimals),
            pet: amount_to_ui_amount(self.pet, decimals),
            emote: amount_to_ui_amount(self.emote, decimals),
            tileset: amount_to_ui_amount(self.tileset, decimals),
            item: amount_to_ui_amount(self.item, decimals),
            world: amount_to_ui_amount(self.world, decimals),
        }
    }

    pub fn of(&self, nft_type: NftType) -> u64 {
        match nft_type {
            NftType::Character => self.character,
            NftType::Pet => self.pet,
            NftType::Emote => self.emote,
            NftType::Tileset => self.tileset,
            NftType::Item => self.item,
            NftType::World => self.world,
        }
    }
}

#[repr(C)]
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct Recipient {
    pub address: Pubkey,
    pub mint_share: u8,
    pub transaction_share: u8,
}

impl Recipient {
    pub const LEN: usize = 32 + 1 + 1;
}

#[account]
pub struct Config {
    pub bump: u8,
    pub primary_wallet: Pubkey,
    pub mint: Pubkey,
    pub fees: Fees,
    pub recipients: Vec<Recipient>,
}

impl Config {
    pub const MAX_RECIPIENT_NUMBER: usize = 3;

    pub const LEN: usize = DESCRIMINATOR_LEN
        + 1
        + 32
        + 32
        + Fees::LEN
        + VECTOR_PREFIX_LEN
        + Self::MAX_RECIPIENT_NUMBER * Recipient::LEN;

    pub const SEED: &'static [u8] = b"config";
}

#[repr(u8)]
#[derive(AnchorSerialize, AnchorDeserialize, Copy, Clone, Debug)]
pub enum NftType {
    Character,
    Pet,
    Emote,
    Tileset,
    Item,
    World,
}

impl NftType {
    pub const LEN: usize = 1;
}

impl TryFrom<&str> for NftType {
    type Error = String;

    fn try_from(string: &str) -> core::result::Result<Self, Self::Error> {
        match string {
            "character" => Ok(NftType::Character),
            "pet" => Ok(NftType::Pet),
            "emote" => Ok(NftType::Emote),
            "tileset" => Ok(NftType::Tileset),
            "item" => Ok(NftType::Item),
            "world" => Ok(NftType::World),
            _ => Err("Wrong nft type".to_owned()),
        }
    }
}

impl TryFrom<u8> for NftType {
    type Error = String;

    fn try_from(number: u8) -> core::result::Result<Self, Self::Error> {
        const CHARACTER: u8 = NftType::Character as u8;
        const PET: u8 = NftType::Pet as u8;
        const EMOTE: u8 = NftType::Emote as u8;
        const TILESET: u8 = NftType::Tileset as u8;
        const ITEM: u8 = NftType::Item as u8;
        const WORLD: u8 = NftType::World as u8;

        match number {
            CHARACTER => Ok(NftType::Character),
            PET => Ok(NftType::Pet),
            EMOTE => Ok(NftType::Emote),
            TILESET => Ok(NftType::Tileset),
            ITEM => Ok(NftType::Item),
            WORLD => Ok(NftType::World),
            _ => Err("Wrong nft type".to_owned()),
        }
    }
}

#[allow(clippy::from_over_into)]
impl Into<u8> for NftType {
    fn into(self) -> u8 {
        self as u8
    }
}

#[account]
pub struct ChillNftMetadata {
    pub bump: u8,
    pub nft_type: NftType,
}

impl ChillNftMetadata {
    pub const LEN: usize = DESCRIMINATOR_LEN + 1 + NftType::LEN;

    pub const SEED: &'static [u8] = b"chill-metadata";
}
