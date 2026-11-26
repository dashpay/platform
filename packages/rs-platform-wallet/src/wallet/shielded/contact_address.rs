//! The viewing keys a DashPay contact request's shielded address is derived from.
//!
//! The identity wallet sends contact requests and holds no shielded keys. The keys live
//! with the platform wallet. [`ContactShieldedAddresses`] is the view of them that the
//! two share, so the identity wallet can derive the address a request carries without
//! the platform wallet handing it one.

use super::AccountViewingKeys;
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::{RwLock, RwLockReadGuard};

/// The bound shielded viewing keys of a wallet. Cloning shares them; the default holds
/// no keys.
#[derive(Clone, Default)]
pub(crate) struct ContactShieldedAddresses {
    viewing_keys: Arc<RwLock<Option<BTreeMap<u32, AccountViewingKeys>>>>,
}

impl ContactShieldedAddresses {
    /// A view of the platform wallet's own key map.
    pub(crate) fn new(
        viewing_keys: Arc<RwLock<Option<BTreeMap<u32, AccountViewingKeys>>>>,
    ) -> Self {
        Self { viewing_keys }
    }

    /// The viewing keys of `account`, or `None` when the wallet is not bound or the
    /// account is not among the bound ones. The keys are read under the guard returned,
    /// so no bind replaces them while it lives: a bind takes the key map for writing.
    ///
    /// Having the keys of an account proves nothing about which identity it belongs to:
    /// the caller proves that before it gives away an address derived from them.
    pub(crate) async fn account_keys(
        &self,
        account: u32,
    ) -> Option<RwLockReadGuard<'_, AccountViewingKeys>> {
        RwLockReadGuard::try_map(self.viewing_keys.read().await, |keys| {
            keys.as_ref()?.get(&account)
        })
        .ok()
    }
}
