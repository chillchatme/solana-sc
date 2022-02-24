use crate::{
    pda,
    state::{Fees, NftType, Recipient},
};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program,
    sysvar::rent,
};

#[repr(C)]
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct InitializeArgs {
    pub fees: Fees,
    pub recipients: Vec<Recipient>,
}

#[repr(C)]
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct MintNftArgs {
    pub nft_type: NftType,
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub fees: u16, // 10000 = 100%
}

#[repr(C)]
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub enum ChillInstruction {
    /// Initialize
    ///
    /// Config - PDA of ("config", Mint, program_id)
    ///
    /// 0. [signer, writable] Authority
    /// 1. [writable] Config
    /// 2. [] Chill Mint account
    /// 3. [] System program
    Initialize(InitializeArgs),

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
    /// 10. [] Rent program
    /// 11. [] Spl token program
    /// 12. [] Token metadata program
    ///
    /// Optional
    ///
    /// 13. [writable] Recipient's Chill token account
    /// 14. ...
    MintNft(MintNftArgs),
}

pub fn initialize(
    program_id: Pubkey,
    authority: Pubkey,
    mint: Pubkey,
    args: InitializeArgs,
) -> Instruction {
    let config = pda::config(&mint, &program_id).0;
    Instruction::new_with_borsh(
        program_id,
        &ChillInstruction::Initialize(args),
        vec![
            AccountMeta::new(authority, true),
            AccountMeta::new(config, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
    )
}

#[allow(clippy::too_many_arguments)]
pub fn mint_nft(
    program_id: Pubkey,
    authority: Pubkey,
    user: Pubkey,
    mint: Pubkey,
    user_token_account: Pubkey,
    nft_mint: Pubkey,
    nft_token: Pubkey,
    recipients_token_accounts: &[Pubkey],
    args: MintNftArgs,
) -> Instruction {
    let config = pda::config(&mint, &program_id).0;
    let metadata = pda::metadata(&nft_mint);
    let master_edition = pda::master_edition(&nft_mint);

    let mut accounts = vec![
        AccountMeta::new_readonly(authority, true),
        AccountMeta::new(user, true),
        AccountMeta::new_readonly(config, false),
        AccountMeta::new_readonly(mint, false),
        AccountMeta::new(user_token_account, false),
        AccountMeta::new(nft_mint, false),
        AccountMeta::new(nft_token, false),
        AccountMeta::new(metadata, false),
        AccountMeta::new(master_edition, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(rent::ID, false),
        AccountMeta::new_readonly(spl_token::ID, false),
        AccountMeta::new_readonly(mpl_token_metadata::ID, false),
    ];

    let recipients = recipients_token_accounts
        .iter()
        .map(|recipient| AccountMeta::new(*recipient, false));

    accounts.extend(recipients);
    Instruction::new_with_borsh(program_id, &ChillInstruction::MintNft(args), accounts)
}
