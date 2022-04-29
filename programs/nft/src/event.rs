use crate::state::NftType;
use anchor_lang::prelude::*;

#[event]
pub struct MintNft {
    pub mint: Pubkey,
    pub nft_type: NftType,
}

#[event]
pub struct UpdateNft {
    pub mint: Pubkey,
}
