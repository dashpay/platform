//! Read and persist a managed identity's current Platform credit balance.

use dash_sdk::platform::Fetch;
use dash_sdk::query_types::IdentityBalance;
use dpp::identity::accessors::{IdentityGettersV0, IdentitySettersV0};
use dpp::prelude::Identifier;

use crate::error::PlatformWalletError;
use crate::BlockTime;

use super::IdentityWallet;

impl IdentityWallet {
    /// Fetch the balance directly from Platform and persist it for this wallet.
    ///
    /// Uses the dedicated balance query rather than the local identity snapshot.
    /// No keys, signatures, or state transitions are required. Network failure
    /// leaves the previous balance untouched; persistence failures are returned.
    pub async fn refresh_identity_balance(
        &self,
        identity_id: &Identifier,
    ) -> Result<u64, PlatformWalletError> {
        {
            let wm = self.wallet_manager.read().await;
            let info = wm.get_wallet_info(&self.wallet_id).ok_or_else(|| {
                PlatformWalletError::WalletNotFound("Wallet info not found".to_string())
            })?;
            if info.identity_manager.identity(identity_id).is_none() {
                return Err(PlatformWalletError::IdentityNotFound(*identity_id));
            }
        }

        let (balance, metadata) =
            IdentityBalance::fetch_with_metadata(&self.sdk, *identity_id, None).await?;
        let balance = balance.ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;

        let current_balance = {
            let mut wm = self.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&self.wallet_id).ok_or_else(|| {
                PlatformWalletError::WalletNotFound("Wallet info not found".to_string())
            })?;
            // Recheck after the network await: never recreate a removed identity.
            let managed = info
                .identity_manager
                .managed_identity_mut(identity_id)
                .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;

            // A slower, overlapping query must not overwrite a newer response.
            if managed
                .last_updated_balance_block_time
                .is_none_or(|previous| metadata.height >= previous.height)
            {
                managed.identity.set_balance(balance);
                managed.last_updated_balance_block_time = Some(BlockTime::new(
                    metadata.height,
                    metadata.core_chain_locked_height,
                    metadata.time_ms,
                ));
                self.persister
                    .store(managed.snapshot_changeset().into())
                    .map_err(|e| PlatformWalletError::Persistence(e.to_string()))?;
            }
            managed.identity.balance()
        };

        self.persister
            .flush()
            .map_err(|e| PlatformWalletError::Persistence(e.to_string()))?;
        Ok(current_balance)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::changeset::{
        ClientStartState, IdentityEntry, PersistenceError, PlatformWalletChangeSet,
        PlatformWalletPersistence,
    };
    use crate::events::{EventHandler, PlatformEventHandler};
    use crate::wallet::identity::IdentityManager;
    use dpp::identity::{v0::IdentityV0, Identity};
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};

    const OLD_BALANCE: u64 = 2_818_262_560;
    const AFTER_DPNS: u64 = 2_743_797_100;

    #[derive(Default)]
    struct BalancePersister {
        queued: Mutex<Vec<([u8; 32], IdentityEntry)>>,
        committed: Mutex<Vec<([u8; 32], IdentityEntry)>>,
        fail_flush: AtomicBool,
    }

    impl PlatformWalletPersistence for BalancePersister {
        fn store(
            &self,
            wallet_id: [u8; 32],
            changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            if let Some(identities) = changeset.identities {
                self.queued.lock().unwrap().extend(
                    identities
                        .identities
                        .into_values()
                        .map(|entry| (wallet_id, entry)),
                );
            }
            Ok(())
        }

        fn flush(&self, _: [u8; 32]) -> Result<(), PersistenceError> {
            if self.fail_flush.load(Ordering::SeqCst) {
                return Err(PersistenceError::backend("injected flush failure"));
            }
            self.committed
                .lock()
                .unwrap()
                .extend(self.queued.lock().unwrap().drain(..));
            Ok(())
        }

        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            Ok(ClientStartState::default())
        }
    }

    struct NoopEvents;
    impl EventHandler for NoopEvents {}
    impl PlatformEventHandler for NoopEvents {}

    async fn fixture(balance: Option<u64>) -> (IdentityWallet, Identifier, Arc<BalancePersister>) {
        let id = Identifier::from([0xAA; 32]);
        let mut sdk = dash_sdk::SdkBuilder::new_mock().build().unwrap();
        sdk.mock()
            .expect_fetch::<IdentityBalance, Identifier>(id, balance)
            .await
            .unwrap();
        let backend = Arc::new(BalancePersister::default());
        let manager =
            crate::PlatformWalletManager::new(Arc::new(sdk), backend.clone(), Arc::new(NoopEvents));
        let wallet = manager
            .create_wallet_from_seed_bytes(
                key_wallet::Network::Testnet,
                &[42; 64],
                WalletAccountCreationOptions::None,
                Some(0),
            )
            .await
            .unwrap();
        let iw = wallet.identity().clone();
        {
            let mut wm = iw.wallet_manager.write().await;
            wm.get_wallet_info_mut(&iw.wallet_id)
                .unwrap()
                .identity_manager
                .add_identity(
                    Identity::V0(IdentityV0 {
                        id,
                        public_keys: Default::default(),
                        balance: OLD_BALANCE,
                        revision: 7,
                    }),
                    0,
                    iw.wallet_id,
                    &iw.persister,
                )
                .unwrap();
        }
        backend.queued.lock().unwrap().clear();
        backend.committed.lock().unwrap().clear();
        (iw, id, backend)
    }

    async fn local_balance(iw: &IdentityWallet, id: &Identifier) -> u64 {
        iw.wallet_manager
            .read()
            .await
            .get_wallet_info(&iw.wallet_id)
            .unwrap()
            .identity_manager
            .identity(id)
            .unwrap()
            .identity
            .balance()
    }

    #[tokio::test]
    async fn should_fetch_balance_after_dpns_and_flush_a_reloadable_wallet_snapshot() {
        let (iw, id, backend) = fixture(Some(AFTER_DPNS)).await;
        assert_eq!(iw.refresh_identity_balance(&id).await.unwrap(), AFTER_DPNS);
        assert_eq!(local_balance(&iw, &id).await, AFTER_DPNS);
        assert!(backend.queued.lock().unwrap().is_empty());
        let committed = backend.committed.lock().unwrap();
        assert_eq!(committed.len(), 1);
        let (wallet_id, entry) = &committed[0];
        assert_eq!(*wallet_id, iw.wallet_id);
        assert_eq!(entry.wallet_id, Some(iw.wallet_id));
        assert_eq!(entry.revision, 7);
        let mut reloaded = IdentityManager::new();
        reloaded.apply_identity_entry(entry.clone());
        assert_eq!(
            reloaded.identity(&id).unwrap().identity.balance(),
            AFTER_DPNS
        );
    }

    #[tokio::test]
    async fn should_persist_a_real_zero_balance() {
        let (iw, id, backend) = fixture(Some(0)).await;
        assert_eq!(iw.refresh_identity_balance(&id).await.unwrap(), 0);
        assert_eq!(backend.committed.lock().unwrap()[0].1.balance, 0);
    }

    #[tokio::test]
    async fn should_preserve_balance_when_platform_has_no_balance() {
        let (iw, id, backend) = fixture(None).await;
        assert!(iw.refresh_identity_balance(&id).await.is_err());
        assert_eq!(local_balance(&iw, &id).await, OLD_BALANCE);
        assert!(backend.committed.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn should_reject_identity_outside_this_wallet_without_changing_balance() {
        let (iw, id, backend) = fixture(Some(AFTER_DPNS)).await;
        let other = Identifier::from([0xBB; 32]);
        assert!(matches!(iw.refresh_identity_balance(&other).await,
            Err(PlatformWalletError::IdentityNotFound(found)) if found == other));
        assert_eq!(local_balance(&iw, &id).await, OLD_BALANCE);
        assert!(backend.committed.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn should_not_overwrite_a_more_recent_balance_response() {
        let (iw, id, backend) = fixture(Some(AFTER_DPNS)).await;
        {
            let mut wm = iw.wallet_manager.write().await;
            let managed = wm
                .get_wallet_info_mut(&iw.wallet_id)
                .unwrap()
                .identity_manager
                .managed_identity_mut(&id)
                .unwrap();
            // Mock responses have height 0; this state came from a newer block.
            managed.last_updated_balance_block_time = Some(BlockTime::new(1, 1, 1));
        }
        assert_eq!(iw.refresh_identity_balance(&id).await.unwrap(), OLD_BALANCE);
        assert!(backend.committed.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn should_report_persistence_failure_instead_of_claiming_a_durable_refresh() {
        let (iw, id, backend) = fixture(Some(AFTER_DPNS)).await;
        backend.fail_flush.store(true, Ordering::SeqCst);
        assert!(matches!(
            iw.refresh_identity_balance(&id).await,
            Err(PlatformWalletError::Persistence(_))
        ));
        assert!(backend.committed.lock().unwrap().is_empty());
        assert_eq!(backend.queued.lock().unwrap()[0].1.balance, AFTER_DPNS);
    }
}
