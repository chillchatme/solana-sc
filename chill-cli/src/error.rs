use chill_client::error::ClientError;
use colored::Colorize;
use solana_client::{
    client_error::{ClientError as RpcClientError, ClientErrorKind},
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
    RpcError(RpcClientError),
}

#[derive(Error, Debug)]
pub enum CliError {
    #[error("Cannot parse pubkey from the file '{0}' - {1}")]
    CannotParseFile(String, String),

    #[error("Cannot write data to the file '{0}'")]
    CannotWriteToFile(String),

    #[error("Cannot get owner: {0}")]
    CannotGetOwner(String),

    #[error("Insufficient tokens amount. Expected at least {0} tokens, found {1} tokens")]
    InsufficientTokens(f64, f64),

    #[error("Cannot overwrite existing file \"{0}\"")]
    MintFileExists(String),

    #[error("Please specify a mint address with '--mint-address' argument")]
    MintNotSpecified,

    #[error("Owner account not found. Please specify the path to existing keypair with '--owner' argument")]
    OwnerNotFound,

    #[error("Mint '{0}' has another owner")]
    OwnerNotMatch(Pubkey),

    #[error("Cannot transfer zero tokens")]
    TransferZeroTokens,
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

impl From<ParsePubkeyError> for AppError {
    fn from(error: ParsePubkeyError) -> Self {
        AppError::InternalError(error.into())
    }
}

impl From<ClientError> for AppError {
    fn from(error: ClientError) -> Self {
        match error {
            ClientError::RpcError(e) => AppError::RpcError(e),
            ClientError::Custom(e) => AppError::InternalError(e.into()),
        }
    }
}

impl AppError {
    fn client_logs(&self) -> Option<&Vec<String>> {
        match self {
            AppError::RpcError(RpcClientError {
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
            AppError::RpcError(e) => {
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
