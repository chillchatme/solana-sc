use chill_nft::error::ChillNftError;
use colored::Colorize;
use solana_client::{
    client_error::{ClientError, ClientErrorKind},
    rpc_request::{RpcError, RpcResponseErrorData},
    rpc_response::RpcSimulateTransactionResult,
};
use solana_sdk::{
    program_error::ProgramError,
    pubkey::{ParsePubkeyError, Pubkey},
};
use std::fmt::Display;
use thiserror::Error;

#[derive(Debug)]
pub enum AppError {
    InternalError(anyhow::Error),
    ClientError(ClientError),
}

#[derive(Error, Debug)]
pub enum CliError {
    #[error("Data cannot be parsed as a mint account")]
    AccountIsNotMint,

    #[error("Data cannot be parsed as a metadata account")]
    AccountIsNotMetadata,

    #[error("Data cannot be parsed as a token account")]
    AccountIsNotToken,

    #[error("Metadata for mint '{0}' not found")]
    MetadataNotFound(Pubkey),

    #[error("Mint '{0}' not found. Please specify the correct mint address with '--mint-address' argument")]
    MintNotFound(Pubkey),

    #[error("Token account '{0}' is not initialized")]
    TokenNotInitialized(Pubkey),

    #[error("Data cannot be parsed as chill metadata")]
    ChillMetadataDataError,

    #[error("Data cannot be parsed as config")]
    ConfigDataError,

    #[error("Chill metadata account not found")]
    ChillMetadataNotFound,

    #[error("Config account not found. Initialize it with \"initialize\" command")]
    ConfigNotFound,

    #[error("Not enoght tokens to transfer. Expected {0}, found {1}")]
    NotEnoughTokens(f64, f64),

    #[error("Fees must be from 0 to 100")]
    FeesOutOfRange,

    #[error("Cannot parse pubkey from the file '{0}' - {1}")]
    CannotParseFile(String, String),

    #[error("Cannot write data to the file '{0}'")]
    CannotWriteToFile(String),

    #[error("Cannot get authority: {0}")]
    CannotGetAuthority(String),

    #[error("Cannot get recipient: {0}")]
    CannotGetRecipient(String),

    #[error("Insufficient tokens amount. Expected at least {0} tokens, found {1} tokens")]
    InsufficientTokens(f64, f64),

    #[error("Cannot overwrite existing file \"{0}\"")]
    MintFileExists(String),

    #[error("Please specify a mint address with '--mint-address' argument")]
    MintNotSpecified,

    #[error("Authority account not found. Please specify the path to existing keypair with '--authority' argument")]
    AuthorityNotFound,

    #[error("Mint '{0}' has another authority")]
    AuthorityNotMatch(Pubkey),

    #[error("Cannot transfer zero tokens")]
    TransferZeroTokens,

    #[error("Specify shares for all recipients")]
    NotEnoughShares,
}

impl std::error::Error for AppError {}

impl From<CliError> for AppError {
    fn from(error: CliError) -> Self {
        AppError::InternalError(error.into())
    }
}

impl From<ProgramError> for AppError {
    fn from(error: ProgramError) -> Self {
        AppError::InternalError(error.into())
    }
}

impl From<std::io::Error> for AppError {
    fn from(error: std::io::Error) -> Self {
        AppError::InternalError(error.into())
    }
}

impl From<ClientError> for AppError {
    fn from(error: ClientError) -> Self {
        AppError::ClientError(error)
    }
}

impl From<ParsePubkeyError> for AppError {
    fn from(error: ParsePubkeyError) -> Self {
        AppError::InternalError(error.into())
    }
}

impl From<ChillNftError> for AppError {
    fn from(error: ChillNftError) -> Self {
        AppError::InternalError(error.into())
    }
}

impl AppError {
    fn client_logs(&self) -> Option<&Vec<String>> {
        match self {
            AppError::ClientError(ClientError {
                kind:
                    ClientErrorKind::RpcError(RpcError::RpcResponseError {
                        data:
                            RpcResponseErrorData::SendTransactionPreflightFailure(
                                RpcSimulateTransactionResult {
                                    logs: Some(ref logs),
                                    ..
                                },
                            ),
                        ..
                    }),
                ..
            }) => Some(logs),
            _ => None,
        }
    }
}

impl Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::InternalError(e) => {
                write!(f, "{} {}", "Error:".red(), e)
            }
            AppError::ClientError(e) => {
                write!(f, "{} {}", "Error:".red(), e)?;
                if let Some(logs) = self.client_logs() {
                    if !logs.is_empty() {
                        writeln!(f, "\n{}", "[LOGS]".cyan())?;
                        for log in logs.iter().take(logs.len() - 1) {
                            writeln!(f, "{}", log)?;
                        }
                        write!(f, "{}", logs.last().unwrap())?;
                    }
                }
                Ok(())
            }
        }
    }
}

pub type Result<T> = core::result::Result<T, AppError>;
