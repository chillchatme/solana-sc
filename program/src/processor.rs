use crate::{
    instruction::ChillInstruction,
    processor::{initialize::process_initialize, mint_nft::process_mint_nft},
};
use borsh::BorshDeserialize;
use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, msg, pubkey::Pubkey};

pub mod initialize;
pub mod mint_nft;

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {
    let instruction = ChillInstruction::try_from_slice(data)?;
    match instruction {
        ChillInstruction::Initialize { fees, recipients } => {
            msg!("Instruction: Initialize");
            process_initialize(program_id, accounts, fees, recipients)
        }
        ChillInstruction::MintNft => {
            msg!("Instruction: MintNft");
            process_mint_nft(program_id, accounts)
        }
    }
}
