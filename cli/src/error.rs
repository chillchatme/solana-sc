use anchor_client::{
    solana_client::{
        client_error::{ClientError, ClientErrorKind},
        rpc_request::{RpcError, RpcResponseErrorData},
        rpc_response::RpcSimulateTransactionResult,
    },
    solana_sdk::{
        program_error::ProgramError,
        pubkey::{ParsePubkeyError, Pubkey},
    },
    ClientError as AnchorClientError,
};
use colored::Colorize;
use std::fmt::Display;
use thiserror::Error;

#[derive(Debug)]
pub enum AppError {
    InternalError(anyhow::Error),
    ClientError(ClientError),
    AnchorClientError(AnchorClientError),
}

impl std::convert::From<std::fmt::Error> for AppError {
    fn from(e: std::fmt::Error) -> Self {
        AppError::InternalError(anyhow::Error::new(e))
    }
}

impl std::convert::From<clap::Error> for AppError {
    fn from(e: clap::Error) -> Self {
        AppError::InternalError(anyhow::Error::new(e))
    }
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

    #[error("Cannot get primary wallet: {0}")]
    CannotGetPrimaryWallet(String),

    #[error("Cannot get payer: {0}")]
    CannotGetPayer(String),

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

    #[error("Token account not found for {0}")]
    TokenAccountNotFound(Pubkey),

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

impl From<AnchorClientError> for AppError {
    fn from(error: AnchorClientError) -> Self {
        AppError::AnchorClientError(error)
    }
}

impl From<ParsePubkeyError> for AppError {
    fn from(error: ParsePubkeyError) -> Self {
        AppError::InternalError(error.into())
    }
}

impl From<anyhow::Error> for AppError {
    fn from(error: anyhow::Error) -> Self {
        AppError::InternalError(error)
    }
}

fn extract_logs(client_error: &ClientError) -> Option<Vec<String>> {
    match client_error {
        ClientError {
            kind:
                ClientErrorKind::RpcError(RpcError::RpcResponseError {
                    data:
                        RpcResponseErrorData::SendTransactionPreflightFailure(
                            RpcSimulateTransactionResult {
                                logs: Some(logs), ..
                            },
                        ),
                    ..
                }),
            ..
        } => Some(logs.clone()),
        _ => None,
    }
}

impl Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let logs;
        match &self {
            AppError::InternalError(e) => {
                write!(f, "{} {}", "error:".red(), e)?;
                logs = None;
            }
            AppError::AnchorClientError(e) => {
                write!(f, "{} {}", "error:".red().bold(), e)?;
                match e {
                    AnchorClientError::SolanaClientError(client_error) => {
                        logs = extract_logs(client_error);
                    }
                    _ => logs = None,
                }
            }
            AppError::ClientError(e) => {
                write!(f, "{} {}", "error:".red().bold(), e)?;
                logs = extract_logs(e);
            }
        }

        if let Some(logs) = logs {
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

pub type Result<T> = core::result::Result<T, AppError>;
