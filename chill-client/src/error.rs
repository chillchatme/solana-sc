use solana_client::client_error::ClientError as RpcClientError;
use solana_sdk::pubkey::Pubkey;
use thiserror::Error;

#[derive(Debug)]
pub enum ClientError {
    RpcError(RpcClientError),
    Custom(CustomClientError),
}

#[derive(Debug, Error)]
pub enum CustomClientError {
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

    #[error("Data cannot be parsed as config")]
    ConfigDataError,

    #[error("Config account not found. Initialize it with \"initialize\" command")]
    ConfigNotFound,

    #[error("Not enoght tokens to transfer. Expected {0}, found {1}")]
    NotEnoughTokens(f64, f64),
}

impl From<RpcClientError> for ClientError {
    fn from(error: RpcClientError) -> Self {
        ClientError::RpcError(error)
    }
}

impl From<CustomClientError> for ClientError {
    fn from(error: CustomClientError) -> Self {
        ClientError::Custom(error)
    }
}

pub type Result<T> = core::result::Result<T, ClientError>;
