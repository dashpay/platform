//! Platform wallet using [SimpleSigner].
use dpp::identity::signer::Signer;
use simple_signer::signer::SimpleSigner;

use crate::wallet::PlatformWallet;

/// Platform wallet using [SimpleSigner].
pub struct PlatformSignerWallet {
    /// Signer used to sign Platform transactions.
    pub signer: SimpleSigner,
}

impl PlatformSignerWallet {
    /// Create new Platform wallet using [SimpleSigner].
    pub fn new() -> Self {
        Self {
            signer: SimpleSigner::default(),
        }
    }
}

impl Default for PlatformSignerWallet {
    fn default() -> Self {
        Self::new()
    }
}

impl From<SimpleSigner> for PlatformSignerWallet {
    fn from(signer: SimpleSigner) -> Self {
        Self { signer }
    }
}

impl PlatformWallet for PlatformSignerWallet {
    fn signer(&self) -> &dyn Signer {
        &self.signer
    }
}
