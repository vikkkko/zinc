//!
//! The contract resource POST `call` error.
//!

use std::fmt;

use actix_web::http::StatusCode;
use actix_web::ResponseError;

use zinc_build::ValueError as BuildValueError;
use zinc_vm::RuntimeError;
use zinc_zksync::TransactionError;

///
/// The contract resource POST `call` error.
///
#[derive(Debug)]
pub enum Error {
    /// The contract with the specified address is not found in the server cache.
    ContractNotFound(String),
    /// The contract with the specified address is locked.
    ContractLocked(String),
    /// The specified method does not exist in the contract.
    MethodNotFound(String),
    /// The immutable method must be called via the `query` endpoint.
    MethodIsImmutable(String),
    /// Invalid contract method arguments.
    InvalidInput(BuildValueError),
    /// The contract method input transaction is invalid.
    Transaction(TransactionError),
    /// Token with such identifier cannot be resolved by zkSync.
    TokenNotFound(String),

    /// The virtual machine contract method runtime error.
    RuntimeError(RuntimeError),
    /// The PostgreSQL database error.
    Database(sqlx::Error),
    /// The ZkSync server client error.
    ZkSyncClient(zksync::error::ClientError),
    /// The ZkSync server signer error.
    ZkSyncSigner(zksync_eth_signer::error::SignerError),
    /// The ZkSync transfer errors.
    TransferFailure(String),
}

impl From<TransactionError> for Error {
    fn from(inner: TransactionError) -> Self {
        Self::Transaction(inner)
    }
}

impl From<sqlx::Error> for Error {
    fn from(inner: sqlx::Error) -> Self {
        Self::Database(inner)
    }
}

impl From<zksync::error::ClientError> for Error {
    fn from(inner: zksync::error::ClientError) -> Self {
        Self::ZkSyncClient(inner)
    }
}

impl From<zksync_eth_signer::error::SignerError> for Error {
    fn from(inner: zksync_eth_signer::error::SignerError) -> Self {
        Self::ZkSyncSigner(inner)
    }
}

impl ResponseError for Error {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::ContractNotFound(..) => StatusCode::NOT_FOUND,
            Self::ContractLocked(..) => StatusCode::UNPROCESSABLE_ENTITY,
            Self::MethodNotFound(..) => StatusCode::BAD_REQUEST,
            Self::MethodIsImmutable(..) => StatusCode::BAD_REQUEST,
            Self::InvalidInput(..) => StatusCode::BAD_REQUEST,
            Self::Transaction(..) => StatusCode::BAD_REQUEST,
            Self::TokenNotFound(..) => StatusCode::UNPROCESSABLE_ENTITY,

            Self::RuntimeError(..) => StatusCode::UNPROCESSABLE_ENTITY,
            Self::Database(..) => StatusCode::SERVICE_UNAVAILABLE,
            Self::ZkSyncClient(..) => StatusCode::SERVICE_UNAVAILABLE,
            Self::ZkSyncSigner(..) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::TransferFailure { .. } => StatusCode::UNPROCESSABLE_ENTITY,
        }
    }
}

impl serde::Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.to_string().as_str())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let error = match self {
            Self::ContractNotFound(address) => {
                format!("Contract with address {} not found", address)
            }
            Self::ContractLocked(address) => format!("Contract with address {} is locked", address),
            Self::MethodNotFound(name) => format!("Method `{}` not found", name),
            Self::MethodIsImmutable(name) => {
                format!("Method `{}` is immutable: use 'query' instead", name)
            }
            Self::InvalidInput(inner) => format!("Input: {}", inner),
            Self::Transaction(inner) => format!("Transaction: {}", inner),
            Self::TokenNotFound(token_id) => {
                format!("Token with identifier `{}` cannot be resolved", token_id)
            }

            Self::RuntimeError(inner) => format!("Runtime: {:?}", inner),
            Self::Database(inner) => format!("Database: {:?}", inner),
            Self::ZkSyncClient(inner) => format!("ZkSync: {:?}", inner),
            Self::ZkSyncSigner(inner) => format!("ZkSync: {:?}", inner),
            Self::TransferFailure(inner) => format!("Transfer failure: {}", inner),
        };

        log::warn!("{}", error);
        write!(f, "{}", error)
    }
}
