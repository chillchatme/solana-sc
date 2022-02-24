use num_derive::FromPrimitive;
use solana_program::{
    decode_error::DecodeError,
    msg,
    program_error::{PrintProgramError, ProgramError},
};
use thiserror::Error;

#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum ChillApiError {
    #[error("Sum of all recipient shares must equal 100")]
    InvalidShares,

    #[error("Exceeded the maximum number of recipients")]
    MaximumRecipientsNumberExceeded,
}

impl PrintProgramError for ChillApiError {
    fn print<E>(&self) {
        msg!(&self.to_string());
    }
}

impl From<ChillApiError> for ProgramError {
    fn from(e: ChillApiError) -> Self {
        ProgramError::Custom(10_000 + e as u32)
    }
}

impl<T> DecodeError<T> for ChillApiError {
    fn type_of() -> &'static str {
        "ChillApiError"
    }
}
