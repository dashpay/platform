//! The viewing keys a DashPay contact request's shielded address is derived from.
//!
//! The identity wallet sends contact requests and holds no shielded keys. The keys, and
//! the record of which identity's shielded account was proven against the seed in this
//! session, live with the platform wallet. [`ContactShieldedAddresses`] is the view of
//! both that the two share, so the identity wallet can derive the address a request
//! carries without the platform wallet handing it one.

use super::tips::PreparedTipAccount;
use super::AccountViewingKeys;
use dpp::prelude::Identifier;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, PoisonError};
use tokio::sync::RwLock;

/// The bound shielded viewing keys of a wallet and its seed-proven identity accounts.
/// Cloning shares both; the default holds no keys and no proven account.
#[derive(Clone, Default)]
pub(crate) struct ContactShieldedAddresses {
    viewing_keys: Arc<RwLock<Option<BTreeMap<u32, AccountViewingKeys>>>>,
    prepared_accounts: Arc<Mutex<BTreeMap<Identifier, PreparedTipAccount>>>,
}

impl ContactShieldedAddresses {
    /// A view of the platform wallet's own maps.
    pub(crate) fn new(
        viewing_keys: Arc<RwLock<Option<BTreeMap<u32, AccountViewingKeys>>>>,
        prepared_accounts: Arc<Mutex<BTreeMap<Identifier, PreparedTipAccount>>>,
    ) -> Self {
        Self {
            viewing_keys,
            prepared_accounts,
        }
    }

    /// The viewing keys of `account`, the shielded account `identity_id` holds now, when
    /// it is the account prepared for that identity in this session and the preparation
    /// stands. `None` otherwise.
    ///
    /// A recorded identity index is not proof of the account: hosts restore a placeholder
    /// index, and discovery can move an identity. Preparation proves the index with the
    /// seed, so only a prepared account's keys derive an address that is given away.
    pub(crate) async fn proven_account_keys(
        &self,
        identity_id: &Identifier,
        account: u32,
    ) -> Option<AccountViewingKeys> {
        let standing = self
            .prepared_accounts
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(identity_id)
            .and_then(PreparedTipAccount::standing_account);
        if standing != Some(account) {
            return None;
        }
        self.viewing_keys
            .read()
            .await
            .as_ref()?
            .get(&account)
            .cloned()
    }
}
