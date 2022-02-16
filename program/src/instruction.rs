use crate::{
    state::{Fees, Recipient},
    utils::pda,
};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program,
};

#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize)]
pub enum ChillInstruction {
    /// Initialize
    ///
    /// Config - PDA of ("config", Mint, program_id)
    ///
    /// 0. [signer, writable] Authority
    /// 1. [writable] Config
    /// 2. [] Chill Mint account
    /// 3. [] System program
    Initialize {
        fees: Fees,
        recipients: Vec<Recipient>,
    },

    /// MintNft
    ///
    /// 0. [signer] Authority
    /// 1. [signer, writable] User (Payer)
    /// 2. [] Config
    /// 3. [] Chill Mint account
    /// 4. [writable] User's Chill token account
    /// 5. [writable] NFT Mint account
    /// 6. [writable] NFT Token account
    /// 7. [writable] NFT Metadata account
    /// 8. [writable] NFT MasterEdition account
    /// 9. [] System program
    /// 10. [] Spl token program
    /// 11. [] Token metadata program
    ///
    /// Optional
    ///
    /// 12. [writable] Recipient's Chill token account
    /// 13. ...
    MintNft,
}

pub fn initialize(
    program_id: Pubkey,
    authority: Pubkey,
    mint: Pubkey,
    fees: Fees,
    recipients: Vec<Recipient>,
) -> Instruction {
    let config_pubkey = pda::config(&mint, &program_id).0;
    Instruction::new_with_borsh(
        program_id,
        &ChillInstruction::Initialize { fees, recipients },
        vec![
            AccountMeta::new(authority, true),
            AccountMeta::new(config_pubkey, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
    )
}
