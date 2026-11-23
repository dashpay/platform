//! Removing one identity from the device's live wallets
//! ([`PlatformWalletManager::remove_identity_from_device`]).

use std::sync::Arc;

use dpp::prelude::Identifier;

use crate::changeset::PlatformWalletPersistence;
use crate::error::PlatformWalletError;

use super::PlatformWalletManager;

impl<P: PlatformWalletPersistence + 'static> PlatformWalletManager<P> {
    /// Remove `identity_id` from every loaded wallet that manages it, so the
    /// running wallets stop syncing it and no later round stores it again.
    /// The host then deletes its own copy of the identity (rows and stored
    /// keys); nothing is broadcast and the identity stays on Platform.
    ///
    /// Every wallet holding the identity is checked with
    /// [`IdentityManager::check_removable`](crate::IdentityManager::check_removable)
    /// before any is changed, all under one `wallet_manager` write lock, so a
    /// refusal (a contact request in flight or unconfirmed, a journaled private
    /// sender, private payments not restored yet) removes nothing anywhere. A
    /// holder registered in `wallet_manager` but not yet published (still being
    /// loaded) refuses the whole removal the same way, as `WalletNotFound`.
    /// Accepted, each holding wallet removes it through its own persister,
    /// which emits the removal to the host, and the identity's token-sync row
    /// is dropped.
    ///
    /// Returns `Ok(true)` when at least one wallet managed the identity and
    /// `Ok(false)` when none did (an identity the host loaded on its own),
    /// so the host learns from Rust, not from its own rows, whether the live
    /// wallet was involved.
    pub async fn remove_identity_from_device(
        &self,
        identity_id: &Identifier,
    ) -> Result<bool, PlatformWalletError> {
        let removed = {
            let mut wm = self.wallet_manager.write().await;
            // Holders come from the wallet infos the write lock guards, so a
            // wallet registered in `wallet_manager` but not yet published to
            // `wallets` is not missed. Each needs its own persister, from the
            // published map read under the same lock.
            let published = self.wallets.load_full();
            let mut holders = Vec::new();
            for (wallet_id, info) in wm.get_all_wallet_infos() {
                if info.identity_manager.identity(identity_id).is_none() {
                    continue;
                }
                let wallet = published.get(wallet_id).ok_or_else(|| {
                    PlatformWalletError::WalletNotFound(format!(
                        "{} is still being loaded; retry the identity removal",
                        hex::encode(wallet_id)
                    ))
                })?;
                info.identity_manager
                    .check_removable(identity_id, wallet.persister())?;
                holders.push((*wallet_id, Arc::clone(wallet)));
            }
            for (wallet_id, wallet) in &holders {
                if let Some(info) = wm.get_wallet_info_mut(wallet_id) {
                    info.identity_manager
                        .remove_identity(identity_id, wallet.persister())?;
                }
            }
            !holders.is_empty()
        };
        if removed {
            self.identity_sync_manager
                .unregister_identity(identity_id)
                .await;
        }
        Ok(removed)
    }
}

#[cfg(test)]
mod tests {
    use crate::error::PlatformWalletError;
    use crate::test_support::test_platform_wallet_manager;
    use crate::wallet::persister::{NoPlatformPersistence, WalletPersister};
    use dpp::identity::v0::IdentityV0;
    use dpp::identity::Identity;
    use dpp::prelude::Identifier;
    use std::collections::BTreeMap;
    use std::sync::Arc;

    fn identity(byte: u8) -> Identity {
        Identity::V0(IdentityV0 {
            id: Identifier::from([byte; 32]),
            public_keys: BTreeMap::new(),
            balance: 0,
            revision: 0,
        })
    }

    #[tokio::test]
    async fn should_remove_a_managed_identity_and_report_an_unmanaged_one() {
        let (manager, wallet_id) = test_platform_wallet_manager().await;
        let persister = WalletPersister::new(wallet_id, Arc::new(NoPlatformPersistence));
        {
            let mut wm = manager.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("wallet info");
            info.identity_manager
                .add_identity(identity(1), 0, wallet_id, &persister)
                .expect("add identity");
        }
        let id = Identifier::from([1u8; 32]);

        assert!(manager.remove_identity_from_device(&id).await.unwrap());
        {
            let wm = manager.wallet_manager.read().await;
            let info = wm.get_wallet_info(&wallet_id).expect("wallet info");
            assert!(info.identity_manager.identity(&id).is_none());
        }
        // Removing again, or an identity no wallet holds, changes nothing.
        assert!(!manager.remove_identity_from_device(&id).await.unwrap());
        assert!(!manager
            .remove_identity_from_device(&Identifier::from([9u8; 32]))
            .await
            .unwrap());
    }

    #[tokio::test]
    async fn should_refuse_and_keep_an_identity_whose_private_payments_are_held() {
        let (manager, wallet_id) = test_platform_wallet_manager().await;
        let persister = WalletPersister::new(wallet_id, Arc::new(NoPlatformPersistence));
        let id = Identifier::from([1u8; 32]);
        {
            let mut wm = manager.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("wallet info");
            info.identity_manager
                .add_identity(identity(1), 0, wallet_id, &persister)
                .expect("add identity");
            info.identity_manager
                .managed_identity_mut(&id)
                .expect("managed identity")
                .hold_for_retained_private_payment_marker();
        }

        let refusal = manager.remove_identity_from_device(&id).await;

        assert!(matches!(
            refusal,
            Err(PlatformWalletError::PrivateContactPaymentsNotRestored)
        ));
        let wm = manager.wallet_manager.read().await;
        let info = wm.get_wallet_info(&wallet_id).expect("wallet info");
        assert!(info.identity_manager.identity(&id).is_some());
    }
}
