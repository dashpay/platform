//! Identity wallet for managing Platform identities.

use std::sync::Arc;

use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;
use tokio::sync::RwLock;

use crate::identity_manager::IdentityManager;

/// Identity wallet providing identity management functionality.
#[derive(Clone)]
pub struct IdentityWallet {
    pub(crate) sdk: dash_sdk::Sdk,
    pub(crate) wallet: Arc<RwLock<Wallet>>,
    pub(crate) wallet_info: Arc<RwLock<ManagedWalletInfo>>,
    pub(crate) identity_manager: Arc<RwLock<IdentityManager>>,
}

impl std::fmt::Debug for IdentityWallet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IdentityWallet").finish()
    }
}
