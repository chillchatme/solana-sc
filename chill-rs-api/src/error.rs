use num_derive::FromPrimitive;
use solana_program::{
    decode_error::DecodeError,
    msg,
    program_error::{PrintProgramError, ProgramError},
};
use thiserror::Error;

#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum ChillProgramError {
    #[error("Account is already initialized")]
    AccountAlreadyInitialized,

    #[error("Wrong authority")]
    WrongAuthority,

    #[error("Wrong recipients list")]
    WrongRecipientsList,

    #[error("Chill metadata account has wrong pubkey")]
    ChillMetadataWrongPubkey,

    #[error("Config has wrong pubkey")]
    ConfigHasWrongPubkey,

    #[error("Config is already initialized")]
    ConfigAlreadyInitialized,

    #[error("Sum of all recipient shares must equal 100")]
    InvalidShares,

    #[error("Exceeded the maximum number of recipients")]
    MaximumRecipientsNumberExceeded,

    #[error("Mint has another authority")]
    MintHasAnotherAuthority,

    #[error("Token account has another mint")]
    TokenHasAnotherMint,

    #[error("Token account has another owner")]
    TokenHasAnotherOwner,
}

impl PrintProgramError for ChillProgramError {
    fn print<E>(&self) {
        msg!(&self.to_string());
    }
}

impl From<ChillProgramError> for ProgramError {
    fn from(e: ChillProgramError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl<T> DecodeError<T> for ChillProgramError {
    fn type_of() -> &'static str {
        "ChillProgramError"
    }
}
