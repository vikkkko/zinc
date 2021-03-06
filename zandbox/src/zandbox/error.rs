//!
//! The Zandbox server daemon binary error.
//!

use std::io;

use failure::Fail;

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "Invalid network: {}", _0)]
    InvalidNetwork(String),
    #[fail(display = "Database: {}", _0)]
    Database(sqlx::Error),
    #[fail(display = "ZkSync client: {}", _0)]
    ZkSyncClient(zksync::error::ClientError),
    #[fail(display = "server binding: {}", _0)]
    ServerBinding(io::Error),
    #[fail(display = "server runtime: {}", _0)]
    ServerRuntime(io::Error),
}

impl From<sqlx::Error> for Error {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}

impl From<zksync::error::ClientError> for Error {
    fn from(inner: zksync::error::ClientError) -> Self {
        Self::ZkSyncClient(inner)
    }
}
