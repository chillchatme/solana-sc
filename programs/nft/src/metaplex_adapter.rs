use anchor_lang::{
    prelude::{Pubkey, Result},
    solana_program::borsh::try_from_slice_unchecked,
};
use borsh::BorshDeserialize;
use mpl_token_metadata::{
    state::{Key, MAX_METADATA_LEN},
    utils::try_from_slice_checked,
};
use std::ops::Deref;

#[derive(Clone)]
pub struct TokenMetadataProgram;

impl anchor_lang::Id for TokenMetadataProgram {
    fn id() -> Pubkey {
        mpl_token_metadata::ID
    }
}

#[derive(Clone, BorshDeserialize)]
pub struct Metadata(mpl_token_metadata::state::Metadata);

impl Metadata {
    pub const LEN: usize = mpl_token_metadata::state::MAX_METADATA_LEN;
}

impl anchor_lang::AccountDeserialize for Metadata {
    fn try_deserialize_unchecked(buf: &mut &[u8]) -> Result<Self> {
        try_from_slice_checked(buf, Key::MetadataV1, MAX_METADATA_LEN).map_err(Into::into)
    }
}

impl anchor_lang::AccountSerialize for Metadata {}

impl anchor_lang::Owner for Metadata {
    fn owner() -> Pubkey {
        mpl_token_metadata::ID
    }
}

impl Deref for Metadata {
    type Target = mpl_token_metadata::state::Metadata;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone)]
pub struct MasterEdition(mpl_token_metadata::state::MasterEditionV2);

impl MasterEdition {
    pub const LEN: usize = mpl_token_metadata::state::MAX_MASTER_EDITION_LEN;
}

impl anchor_lang::AccountDeserialize for MasterEdition {
    fn try_deserialize_unchecked(buf: &mut &[u8]) -> Result<Self> {
        try_from_slice_unchecked(buf)
            .map(MasterEdition)
            .map_err(Into::into)
    }
}

impl anchor_lang::AccountSerialize for MasterEdition {}

impl anchor_lang::Owner for MasterEdition {
    fn owner() -> Pubkey {
        mpl_token_metadata::ID
    }
}

impl Deref for MasterEdition {
    type Target = mpl_token_metadata::state::MasterEditionV2;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
