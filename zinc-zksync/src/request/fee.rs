//!
//! The contract resource `fee` PUT request.
//!

use std::iter::IntoIterator;

use serde::Deserialize;
use serde::Serialize;
use serde_json::Value as JsonValue;

use zksync::Network;
use zksync_types::Address;

use crate::transaction::Transaction;

///
/// The contract resource `fee` PUT request query.
///
#[derive(Debug, Deserialize)]
pub struct Query {
    /// The contract ETH address.
    pub address: Address,
    /// The name of the queried method.
    pub method: String,
    /// The network where the contract resides.
    pub network: Network,
}

impl Query {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(address: Address, method: String, network: Network) -> Self {
        Self {
            address,
            method,
            network,
        }
    }
}

impl IntoIterator for Query {
    type Item = (&'static str, String);

    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        vec![
            (
                "address",
                serde_json::to_string(&self.address)
                    .expect(zinc_const::panic::DATA_CONVERSION)
                    .replace("\"", ""),
            ),
            ("method", self.method),
            ("network", self.network.to_string()),
        ]
        .into_iter()
    }
}

///
/// The contract resource `call` POST request body.
///
#[derive(Debug, Serialize, Deserialize)]
pub struct Body {
    /// The JSON method input.
    pub arguments: JsonValue,
    /// The signed transaction which must be sent directly to zkSync.
    pub transaction: Vec<Transaction>,
}

impl Body {
    ///
    /// A shortcut constructor.
    ///
    pub fn new(arguments: JsonValue, transaction: Vec<Transaction>) -> Self {
        Self {
            arguments,
            transaction,
        }
    }
}