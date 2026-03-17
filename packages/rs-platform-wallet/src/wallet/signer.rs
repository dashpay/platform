//! Signer for identity operations using wallet-derived keys.

use std::sync::Arc;

use key_wallet::wallet::Wallet;
use tokio::sync::RwLock;

/// A signer that uses wallet-derived keys to sign identity state transitions.
pub struct IdentitySigner {
    wallet: Arc<RwLock<Wallet>>,
    identity_index: u32,
}

impl IdentitySigner {
    /// Create a new IdentitySigner for a specific identity index.
    pub(crate) fn new(wallet: Arc<RwLock<Wallet>>, identity_index: u32) -> Self {
        Self {
            wallet,
            identity_index,
        }
    }

    /// Get the identity index this signer is associated with.
    #[allow(dead_code)]
    pub fn identity_index(&self) -> u32 {
        self.identity_index
    }

    /// Get a reference to the wallet.
    #[allow(dead_code)]
    pub fn wallet(&self) -> &Arc<RwLock<Wallet>> {
        &self.wallet
    }
}

impl std::fmt::Debug for IdentitySigner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IdentitySigner")
            .field("identity_index", &self.identity_index)
            .finish()
    }
}
