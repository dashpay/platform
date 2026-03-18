//! DashPay wallet for contact requests and payments.

use std::sync::Arc;

use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;
use tokio::sync::RwLock;

use crate::wallet::identity::IdentityManager;

/// DashPay wallet providing contact request and payment functionality.
///
/// Shares the same `identity_manager` Arc as `IdentityWallet`.
#[derive(Clone)]
pub struct DashPayWallet {
    pub(crate) sdk: dash_sdk::Sdk,
    pub(crate) wallet: Arc<RwLock<Wallet>>,
    pub(crate) wallet_info: Arc<RwLock<ManagedWalletInfo>>,
    pub(crate) identity_manager: Arc<RwLock<IdentityManager>>,
}

impl std::fmt::Debug for DashPayWallet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DashPayWallet").finish()
    }
}
