//! Read and persist a managed identity's current Platform credit balance.

use dash_sdk::platform::Fetch;
use dash_sdk::query_types::IdentityBalance;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::prelude::Identifier;

use crate::error::PlatformWalletError;
use crate::wallet::identity::state::managed_identity::ManagedIdentity;
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
            if info
                .identity_manager
                .wallet_identity(&self.wallet_id, identity_id)
                .is_none()
            {
                return Err(PlatformWalletError::IdentityNotFound(*identity_id));
            }
        }

        let (balance, metadata) =
            IdentityBalance::fetch_with_metadata(&self.sdk, *identity_id, None).await?;
        let balance = balance.ok_or(PlatformWalletError::IdentityBalanceUnavailable(
            *identity_id,
        ))?;

        let current_balance = {
            let mut wm = self.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&self.wallet_id).ok_or_else(|| {
                PlatformWalletError::WalletNotFound("Wallet info not found".to_string())
            })?;
            // Recheck after the network await: never recreate a removed identity.
            let managed = info
                .identity_manager
                .wallet_identity_mut(&self.wallet_id, identity_id)
                .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;

            self.persist_refreshed_balance(
                managed,
                balance,
                BlockTime::new(
                    metadata.height,
                    metadata.core_chain_locked_height,
                    metadata.time_ms,
                ),
            )?;
            managed.identity.balance()
        };

        Ok(current_balance)
    }

    // The manager write lock must cover persistence and publication together.
    fn persist_refreshed_balance(
        &self,
        managed: &mut ManagedIdentity,
        balance: u64,
        block_time: BlockTime,
    ) -> Result<(), PlatformWalletError> {
        managed.persist_refreshed_balance(balance, block_time, &self.persister)
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
        fail_store: AtomicBool,
        transient_store: AtomicBool,
        store_reissuable: AtomicBool,
        commits_inline: AtomicBool,
        flush_count: std::sync::atomic::AtomicUsize,
    }

    impl PlatformWalletPersistence for BalancePersister {
        fn store_transient_is_reissuable(&self) -> bool {
            self.store_reissuable.load(Ordering::SeqCst)
        }

        fn store_commits_inline(&self) -> bool {
            self.commits_inline.load(Ordering::SeqCst)
        }
        fn store(
            &self,
            wallet_id: [u8; 32],
            changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            if self.transient_store.load(Ordering::SeqCst) {
                return Err(PersistenceError::backend_with_kind(
                    crate::changeset::PersistenceErrorKind::Transient,
                    "busy",
                ));
            }
            if self.fail_store.load(Ordering::SeqCst) {
                return Err(PersistenceError::backend("injected store failure"));
            }
            if let Some(identities) = changeset.identities {
                self.queued.lock().unwrap().extend(
                    identities
                        .identities
                        .into_values()
                        .map(|entry| (wallet_id, entry)),
                );
            }
            if self.store_commits_inline() {
                self.committed
                    .lock()
                    .unwrap()
                    .extend(self.queued.lock().unwrap().drain(..));
            }
            Ok(())
        }

        fn flush(&self, _: [u8; 32]) -> Result<(), PersistenceError> {
            self.flush_count.fetch_add(1, Ordering::SeqCst);
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
        assert!(
            matches!(iw.refresh_identity_balance(&id).await, Err(PlatformWalletError::IdentityBalanceUnavailable(found)) if found == id)
        );
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
        assert_eq!(local_balance(&iw, &id).await, OLD_BALANCE);
    }

    #[tokio::test]
    async fn should_keep_confirmed_transaction_balance_when_query_is_older_or_equal() {
        for proof_height in [0, 1] {
            let (iw, id, backend) = fixture(Some(OLD_BALANCE)).await;
            {
                let mut wm = iw.wallet_manager.write().await;
                let managed = wm
                    .get_wallet_info_mut(&iw.wallet_id)
                    .unwrap()
                    .identity_manager
                    .wallet_identity_mut(&iw.wallet_id, &id)
                    .unwrap();
                managed.persist_confirmed_balance(
                    AFTER_DPNS,
                    BlockTime::new(proof_height, 42, 1000),
                    &iw.persister,
                );
            }
            let before = backend.flush_count.load(Ordering::SeqCst);
            assert_eq!(iw.refresh_identity_balance(&id).await.unwrap(), AFTER_DPNS);
            assert_eq!(backend.flush_count.load(Ordering::SeqCst), before);
            assert_eq!(backend.committed.lock().unwrap().len(), 1);
            assert_eq!(backend.committed.lock().unwrap()[0].1.balance, AFTER_DPNS);
        }
    }

    #[tokio::test]
    async fn should_not_publish_balance_or_watermark_when_store_fails() {
        let (iw, id, backend) = fixture(Some(AFTER_DPNS)).await;
        backend.fail_store.store(true, Ordering::SeqCst);
        assert!(matches!(
            iw.refresh_identity_balance(&id).await,
            Err(PlatformWalletError::PersisterStore(_))
        ));
        assert_eq!(local_balance(&iw, &id).await, OLD_BALANCE);
        assert!(iw
            .wallet_manager
            .read()
            .await
            .get_wallet_info(&iw.wallet_id)
            .unwrap()
            .identity_manager
            .identity(&id)
            .unwrap()
            .last_updated_balance_block_time
            .is_none());
        backend.fail_store.store(false, Ordering::SeqCst);
        assert_eq!(iw.refresh_identity_balance(&id).await.unwrap(), AFTER_DPNS);
        assert_eq!(backend.committed.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn should_not_flush_an_inline_commit() {
        let (iw, id, backend) = fixture(Some(AFTER_DPNS)).await;
        backend.commits_inline.store(true, Ordering::SeqCst);
        backend.fail_flush.store(true, Ordering::SeqCst);
        assert_eq!(iw.refresh_identity_balance(&id).await.unwrap(), AFTER_DPNS);
        assert_eq!(backend.committed.lock().unwrap().len(), 1);
        assert_eq!(backend.flush_count.load(Ordering::SeqCst), 0);
    }
    #[tokio::test]
    async fn should_reject_an_observed_identity_without_fetching_or_persisting() {
        let (iw, id, backend) = fixture(Some(AFTER_DPNS)).await;
        {
            let mut wm = iw.wallet_manager.write().await;
            let manager = &mut wm
                .get_wallet_info_mut(&iw.wallet_id)
                .unwrap()
                .identity_manager;
            let identity = manager.remove_identity(&id, &iw.persister).unwrap();
            manager
                .add_out_of_wallet_identity(identity, &iw.persister)
                .unwrap();
        }
        backend.queued.lock().unwrap().clear();
        assert!(matches!(iw.refresh_identity_balance(&id).await,
            Err(PlatformWalletError::IdentityNotFound(found)) if found == id));
        assert_eq!(local_balance(&iw, &id).await, OLD_BALANCE);
        assert!(backend.queued.lock().unwrap().is_empty());
        assert_eq!(backend.flush_count.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn should_retry_the_newer_snapshot_before_accepting_an_older_response() {
        let (iw, id, backend) = fixture(Some(AFTER_DPNS)).await;
        let mut wm = iw.wallet_manager.write().await;
        let managed = wm
            .get_wallet_info_mut(&iw.wallet_id)
            .unwrap()
            .identity_manager
            .wallet_identity_mut(&iw.wallet_id, &id)
            .unwrap();
        backend.fail_store.store(true, Ordering::SeqCst);
        assert!(iw
            .persist_refreshed_balance(managed, 100, BlockTime::new(10, 10, 10))
            .is_err());
        assert_eq!(managed.identity.balance(), OLD_BALANCE);
        assert!(managed.last_updated_balance_block_time.is_none());
        backend.fail_store.store(false, Ordering::SeqCst);
        iw.persist_refreshed_balance(managed, 200, BlockTime::new(9, 9, 9))
            .unwrap();
        assert_eq!(managed.identity.balance(), 100);
        let committed = backend.committed.lock().unwrap();
        assert_eq!(committed.len(), 1);
        assert_eq!(committed[0].1.balance, 100);
        assert_eq!(
            committed[0]
                .1
                .last_updated_balance_block_time
                .unwrap()
                .height,
            10
        );
    }
    #[tokio::test]
    async fn should_reject_a_query_newer_than_the_last_refresh_but_older_than_a_transaction() {
        let (iw, id, backend) = fixture(Some(AFTER_DPNS)).await;
        let mut wm = iw.wallet_manager.write().await;
        let managed = wm
            .get_wallet_info_mut(&iw.wallet_id)
            .unwrap()
            .identity_manager
            .wallet_identity_mut(&iw.wallet_id, &id)
            .unwrap();
        iw.persist_refreshed_balance(managed, 300, BlockTime::new(8, 8, 8))
            .unwrap();
        managed.persist_confirmed_balance(100, BlockTime::new(10, 42, 1000), &iw.persister);
        for height in [9, 10] {
            iw.persist_refreshed_balance(
                managed,
                200,
                BlockTime::new(height, height as u32, height),
            )
            .unwrap();
            assert_eq!(managed.identity.balance(), 100);
        }
        assert_eq!(backend.committed.lock().unwrap().len(), 2);
        iw.persist_refreshed_balance(managed, 50, BlockTime::new(11, 11, 11))
            .unwrap();
        assert_eq!(managed.identity.balance(), 50);
        assert_eq!(backend.committed.lock().unwrap().len(), 3);
    }
    #[tokio::test]
    async fn should_preserve_store_failure_kind_with_backend_retry_guarantee() {
        use crate::changeset::PersistenceErrorKind;
        for reissuable in [false, true] {
            let (iw, id, backend) = fixture(Some(AFTER_DPNS)).await;
            backend.transient_store.store(true, Ordering::SeqCst);
            backend.store_reissuable.store(reissuable, Ordering::SeqCst);
            let error = iw.refresh_identity_balance(&id).await.unwrap_err();
            let PlatformWalletError::PersisterStore(source) = error else {
                panic!("lost typed store failure")
            };
            assert_eq!(
                source.kind(),
                Some(if reissuable {
                    PersistenceErrorKind::Transient
                } else {
                    PersistenceErrorKind::Fatal
                })
            );
            assert_eq!(local_balance(&iw, &id).await, OLD_BALANCE);
        }
    }
    fn reload_balance(backend: &BalancePersister, id: &Identifier) -> (u64, Option<BlockTime>) {
        let mut reloaded = IdentityManager::new();
        for (_, entry) in backend.committed.lock().unwrap().iter() {
            reloaded.apply_identity_entry(entry.clone());
        }
        let managed = reloaded.identity(id).unwrap();
        (
            managed.identity.balance(),
            managed.last_updated_balance_block_time,
        )
    }

    #[tokio::test]
    async fn should_retry_failed_height_ten_flush_before_height_nine_and_reload_ten() {
        let (iw, id, backend) = fixture(Some(AFTER_DPNS)).await;
        let mut wm = iw.wallet_manager.write().await;
        let managed = wm
            .get_wallet_info_mut(&iw.wallet_id)
            .unwrap()
            .identity_manager
            .wallet_identity_mut(&iw.wallet_id, &id)
            .unwrap();
        let newer = BlockTime::new(10, 42, 1000);
        backend.fail_flush.store(true, Ordering::SeqCst);
        assert!(iw.persist_refreshed_balance(managed, 100, newer).is_err());
        assert_eq!(managed.identity.balance(), OLD_BALANCE);
        assert!(iw
            .persist_refreshed_balance(managed, 200, BlockTime::new(9, 41, 900))
            .is_err());
        assert!(backend
            .queued
            .lock()
            .unwrap()
            .iter()
            .all(|(_, entry)| entry.balance == 100));
        backend.fail_flush.store(false, Ordering::SeqCst);
        iw.persist_refreshed_balance(managed, 200, BlockTime::new(9, 41, 900))
            .unwrap();
        assert_eq!(managed.identity.balance(), 100);
        assert_eq!(reload_balance(&backend, &id), (100, Some(newer)));
        assert!(backend.queued.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn should_carry_pending_balance_through_an_unrelated_scalar_write() {
        let (iw, id, backend) = fixture(Some(AFTER_DPNS)).await;
        let mut wm = iw.wallet_manager.write().await;
        let managed = wm
            .get_wallet_info_mut(&iw.wallet_id)
            .unwrap()
            .identity_manager
            .wallet_identity_mut(&iw.wallet_id, &id)
            .unwrap();
        let newer = BlockTime::new(10, 42, 1000);
        backend.fail_flush.store(true, Ordering::SeqCst);
        assert!(iw.persist_refreshed_balance(managed, 100, newer).is_err());
        // This used to queue the published OLD_BALANCE behind the height-10 row.
        managed.update_keys_sync_block_time(BlockTime::new(11, 43, 1100), &iw.persister);
        backend.fail_flush.store(false, Ordering::SeqCst);
        iw.persister.flush().unwrap();
        assert_eq!(reload_balance(&backend, &id), (100, Some(newer)));
        iw.persist_refreshed_balance(managed, 200, BlockTime::new(9, 41, 900))
            .unwrap();
        assert_eq!(managed.identity.balance(), 100);
        assert_eq!(
            managed.last_synced_keys_block_time,
            Some(BlockTime::new(11, 43, 1100))
        );
    }

    #[tokio::test]
    async fn should_retry_failed_confirmed_balance_store_before_equal_or_older_refresh() {
        for refresh_height in [9, 10] {
            for inline in [false, true] {
                let (iw, id, backend) = fixture(Some(AFTER_DPNS)).await;
                let mut wm = iw.wallet_manager.write().await;
                let managed = wm
                    .get_wallet_info_mut(&iw.wallet_id)
                    .unwrap()
                    .identity_manager
                    .wallet_identity_mut(&iw.wallet_id, &id)
                    .unwrap();
                let confirmed = BlockTime::new(10, 42, 1000);
                backend.commits_inline.store(inline, Ordering::SeqCst);
                backend.fail_store.store(true, Ordering::SeqCst);
                assert_eq!(
                    managed.persist_confirmed_balance(100, confirmed, &iw.persister),
                    100
                );
                assert_eq!(managed.last_updated_balance_block_time, Some(confirmed));
                assert!(!managed.needs_balance_update(1050, 100));
                assert!(backend.committed.lock().unwrap().is_empty());
                backend.fail_store.store(false, Ordering::SeqCst);
                iw.persist_refreshed_balance(managed, 200, BlockTime::new(refresh_height, 41, 900))
                    .unwrap();
                assert_eq!(reload_balance(&backend, &id), (100, Some(confirmed)));
            }
        }
    }

    #[tokio::test]
    async fn should_keep_one_snapshot_per_height_and_return_the_retained_balance() {
        let (iw, id, backend) = fixture(Some(AFTER_DPNS)).await;
        let mut wm = iw.wallet_manager.write().await;
        let managed = wm
            .get_wallet_info_mut(&iw.wallet_id)
            .unwrap()
            .identity_manager
            .wallet_identity_mut(&iw.wallet_id, &id)
            .unwrap();
        let confirmed = BlockTime::new(10, 42, 1000);
        assert_eq!(
            managed.persist_confirmed_balance(100, confirmed, &iw.persister),
            100
        );
        for height in [9, 10] {
            // A delayed result cannot replace the height-pinned state, return a
            // different balance to the host, or queue an obsolete snapshot.
            assert_eq!(
                managed.persist_confirmed_balance(
                    200,
                    BlockTime::new(height, 41, 900),
                    &iw.persister
                ),
                100
            );
        }
        assert_eq!(backend.committed.lock().unwrap().len(), 1);
        assert_eq!(reload_balance(&backend, &id), (100, Some(confirmed)));
    }

    #[tokio::test]
    async fn should_compare_query_and_transaction_proofs_on_the_same_chain_height() {
        let (iw, id, backend) = fixture(Some(AFTER_DPNS)).await;
        let mut wm = iw.wallet_manager.write().await;
        let managed = wm
            .get_wallet_info_mut(&iw.wallet_id)
            .unwrap()
            .identity_manager
            .wallet_identity_mut(&iw.wallet_id, &id)
            .unwrap();
        let query = BlockTime::new(1000, 42, 1000);
        iw.persist_refreshed_balance(managed, 100, query).unwrap();
        assert_eq!(
            managed.persist_confirmed_balance(200, BlockTime::new(999, 41, 900), &iw.persister),
            100
        );
        assert_eq!(backend.committed.lock().unwrap().len(), 1);
        assert_eq!(reload_balance(&backend, &id), (100, Some(query)));
    }

    #[tokio::test]
    async fn should_replace_a_failed_query_write_with_a_newer_transaction_snapshot() {
        let (iw, id, backend) = fixture(Some(AFTER_DPNS)).await;
        let mut wm = iw.wallet_manager.write().await;
        let managed = wm
            .get_wallet_info_mut(&iw.wallet_id)
            .unwrap()
            .identity_manager
            .wallet_identity_mut(&iw.wallet_id, &id)
            .unwrap();
        backend.fail_flush.store(true, Ordering::SeqCst);
        assert!(iw
            .persist_refreshed_balance(managed, 200, BlockTime::new(9, 41, 900))
            .is_err());
        backend.fail_flush.store(false, Ordering::SeqCst);
        let confirmed = BlockTime::new(10, 42, 1000);
        assert_eq!(
            managed.persist_confirmed_balance(100, confirmed, &iw.persister),
            100
        );
        assert_eq!(reload_balance(&backend, &id), (100, Some(confirmed)));
        let count = backend.committed.lock().unwrap().len();
        iw.persist_refreshed_balance(managed, 200, BlockTime::new(9, 41, 900))
            .unwrap();
        assert_eq!(backend.committed.lock().unwrap().len(), count);
    }
    #[tokio::test]
    async fn should_publish_a_confirmation_matching_a_failed_query_without_losing_retry() {
        let (iw, id, backend) = fixture(Some(AFTER_DPNS)).await;
        let mut wm = iw.wallet_manager.write().await;
        let managed = wm
            .get_wallet_info_mut(&iw.wallet_id)
            .unwrap()
            .identity_manager
            .wallet_identity_mut(&iw.wallet_id, &id)
            .unwrap();
        let confirmed = BlockTime::new(10, 42, 1000);
        backend.fail_store.store(true, Ordering::SeqCst);
        assert!(iw
            .persist_refreshed_balance(managed, 100, confirmed)
            .is_err());
        assert_eq!(managed.identity.balance(), OLD_BALANCE);
        assert_eq!(
            managed.persist_confirmed_balance(100, confirmed, &iw.persister),
            100
        );
        assert_eq!(managed.last_updated_balance_block_time, Some(confirmed));
        assert!(backend.committed.lock().unwrap().is_empty());
        backend.fail_store.store(false, Ordering::SeqCst);
        iw.persist_refreshed_balance(managed, 200, BlockTime::new(9, 41, 900))
            .unwrap();
        assert_eq!(reload_balance(&backend, &id), (100, Some(confirmed)));
    }

    #[tokio::test]
    async fn should_preserve_balance_watermark_and_pending_write_when_replay_revision_is_rejected()
    {
        let (iw, id, backend) = fixture(Some(AFTER_DPNS)).await;
        let mut wm = iw.wallet_manager.write().await;
        let manager = &mut wm
            .get_wallet_info_mut(&iw.wallet_id)
            .unwrap()
            .identity_manager;
        let managed = manager.wallet_identity_mut(&iw.wallet_id, &id).unwrap();
        let previous = BlockTime::new(9, 41, 900);
        managed.last_updated_balance_block_time = Some(previous);
        let pending = BlockTime::new(10, 42, 1000);
        backend.fail_store.store(true, Ordering::SeqCst);
        assert!(iw.persist_refreshed_balance(managed, 100, pending).is_err());
        let mut entry = IdentityEntry::from_managed(managed);
        entry.revision = 6;
        entry.balance = 200;
        entry.last_updated_balance_block_time = Some(BlockTime::new(20, 50, 2000));
        manager.apply_identity_entry(entry);
        let managed = manager.wallet_identity_mut(&iw.wallet_id, &id).unwrap();
        assert_eq!(managed.identity.balance(), OLD_BALANCE);
        assert_eq!(managed.identity.revision(), 7);
        assert_eq!(managed.last_updated_balance_block_time, Some(previous));
        assert_eq!(
            managed.balance_snapshot_for_persistence(),
            (100, Some(pending))
        );
        backend.fail_store.store(false, Ordering::SeqCst);
        managed.retry_pending_balance(&iw.persister).unwrap();
        assert_eq!(reload_balance(&backend, &id), (100, Some(pending)));
        // The rejected entry must not poison the gate for a later proven read.
        let next = BlockTime::new(20, 50, 2000);
        iw.persist_refreshed_balance(managed, 150, next).unwrap();
        assert_eq!(reload_balance(&backend, &id), (150, Some(next)));
    }

    #[tokio::test]
    async fn should_clear_superseded_pending_balance_when_replaying_an_accepted_entry() {
        for revision in [7, 8] {
            for watermark in [
                None,
                Some(BlockTime::new(9, 41, 900)),
                Some(BlockTime::new(11, 43, 1100)),
            ] {
                let (iw, id, backend) = fixture(Some(AFTER_DPNS)).await;
                let mut wm = iw.wallet_manager.write().await;
                let manager = &mut wm
                    .get_wallet_info_mut(&iw.wallet_id)
                    .unwrap()
                    .identity_manager;
                let managed = manager.wallet_identity_mut(&iw.wallet_id, &id).unwrap();
                backend.fail_store.store(true, Ordering::SeqCst);
                assert!(iw
                    .persist_refreshed_balance(managed, 100, BlockTime::new(10, 42, 1000))
                    .is_err());
                let mut entry = IdentityEntry::from_managed(managed);
                entry.revision = revision;
                entry.balance = 200;
                entry.last_updated_balance_block_time = watermark;
                manager.apply_identity_entry(entry);
                let managed = manager.wallet_identity_mut(&iw.wallet_id, &id).unwrap();
                assert_eq!(managed.identity.balance(), 200);
                assert_eq!(managed.identity.revision(), revision);
                assert_eq!(managed.last_updated_balance_block_time, watermark);
                assert_eq!(managed.balance_snapshot_for_persistence(), (200, watermark));
                backend.fail_store.store(false, Ordering::SeqCst);
                managed.retry_pending_balance(&iw.persister).unwrap();
                assert!(backend.queued.lock().unwrap().is_empty());
                assert!(backend.committed.lock().unwrap().is_empty());
                managed.update_keys_sync_block_time(BlockTime::new(30, 60, 3000), &iw.persister);
                iw.persister.flush().unwrap();
                assert_eq!(reload_balance(&backend, &id), (200, watermark));
            }
        }
    }
}
