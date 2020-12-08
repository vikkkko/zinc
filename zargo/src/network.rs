//!
//! The Zargo network type wrapper.
//!

use std::fmt;

///
/// The zkSync SDK network wrapper.
///
#[derive(Debug, Clone, Copy)]
pub struct Network {
    /// The zkSync type.
    inner: zksync::Network,
}

impl Network {
    ///
    /// Returns the address for the Zandbox in the specified network.
    ///
    pub fn try_into_url(self) -> Result<String, zksync::Network> {
        match self.inner {
            zksync::Network::Rinkeby => Ok("https://rinkeby2-zandbox.zksync.dev".to_owned()),
            zksync::Network::Localhost => Ok("http://localhost:4001".to_owned()),
            zksync::Network::Ropsten => Ok("http://47.241.13.124:4001".to_owned()),
            another => Err(another),
        }
    }
}

impl From<zksync::Network> for Network {
    fn from(inner: zksync::Network) -> Self {
        Self { inner }
    }
}

impl Into<zksync::Network> for Network {
    fn into(self) -> zksync::Network {
        self.inner
    }
}

impl fmt::Display for Network {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner)
    }
}
