//! The viewing keys a DashPay contact request's shielded address is derived from.
//!
//! The identity wallet sends contact requests and holds no shielded keys. The keys live
//! with the platform wallet. [`ContactShieldedAddresses`] is the view of them that the
//! two share, so the identity wallet can derive the address a request carries without
//! the platform wallet handing it one.

use super::AccountViewingKeys;
use grovedb_commitment_tree::PaymentAddress;
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

    /// Whether the raw 43-byte `address` belongs to one of the bound accounts, at any
    /// diversifier index and in either scope (an account's internal addresses are its
    /// own too). `false` while the wallet is not bound: with no viewing key there is
    /// nothing to compare with. `false` for bytes that are not an Orchard address.
    pub(crate) async fn is_own_address(&self, address: &[u8; 43]) -> bool {
        let Some(address) =
            Option::<PaymentAddress>::from(PaymentAddress::from_raw_address_bytes(address))
        else {
            return false;
        };
        self.viewing_keys.read().await.as_ref().is_some_and(|keys| {
            keys.values()
                .any(|keys| keys.full_viewing_key.scope_for_address(&address).is_some())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use grovedb_commitment_tree::{FullViewingKey, Scope, SpendingKey};

    fn viewing_key(seed: u8) -> FullViewingKey {
        let spending_key = SpendingKey::from_bytes([seed; 32]).expect("a valid spending key");
        FullViewingKey::from(&spending_key)
    }

    #[tokio::test]
    async fn should_tell_an_address_of_a_bound_account_from_another_wallets() {
        let ours = viewing_key(7);
        let theirs = viewing_key(8);
        let own_address = ours
            .address_at(5u32, Scope::External)
            .to_raw_address_bytes();
        let own_internal_address = ours
            .address_at(5u32, Scope::Internal)
            .to_raw_address_bytes();
        let other_address = theirs
            .address_at(5u32, Scope::External)
            .to_raw_address_bytes();

        let unbound = ContactShieldedAddresses::default();
        assert!(!unbound.is_own_address(&own_address).await);

        let bound = ContactShieldedAddresses::new(Arc::new(RwLock::new(Some(BTreeMap::from([(
            0,
            AccountViewingKeys::from_full_viewing_key(ours),
        )])))));
        assert!(bound.is_own_address(&own_address).await);
        assert!(bound.is_own_address(&own_internal_address).await);
        assert!(!bound.is_own_address(&other_address).await);
    }
}
