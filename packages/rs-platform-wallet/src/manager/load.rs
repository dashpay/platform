//! Hydrate a [`PlatformWalletManager`] from its persister.

use std::collections::BTreeMap;
use std::sync::Arc;

use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;

use crate::changeset::{ClientStartState, ClientWalletStartState, PlatformWalletPersistence};
use crate::error::PlatformWalletError;
use crate::manager::load_outcome::{CorruptKind, LoadOutcome, SkipReason};
use crate::wallet::core::WalletBalance;
use crate::wallet::identity::IdentityManager;
use crate::wallet::platform_wallet::{PlatformWalletInfo, WalletId};
use crate::wallet::PlatformWallet;

use super::PlatformWalletManager;

impl<P: PlatformWalletPersistence + 'static> PlatformWalletManager<P> {
    /// Restore every persisted wallet as a **watch-only** entry — no
    /// signing key material is derived here. The persister hands back a
    /// keyless reconstruction snapshot; each wallet is rebuilt via
    /// [`Wallet::new_watch_only`](key_wallet::wallet::Wallet::new_watch_only)
    /// from its [`AccountRegistrationEntry`](crate::changeset::AccountRegistrationEntry)
    /// manifest, the managed core state is restored, and the result is
    /// registered into the manager.
    ///
    /// Core state arrives as a full keyless snapshot
    /// ([`ClientWalletStartState::wallet_info`]) — consumed directly,
    /// preserving per-account UTXO/record attribution and exact pool
    /// contents — after its `wallet_id`/`network`/account-set are
    /// validated against the row.
    ///
    /// The load path never touches the seed, so it performs no wrong-seed
    /// check. Signing happens later, on demand, via the configured
    /// `MnemonicResolverHandle` (`rs-sdk-ffi`).
    ///
    /// # Skip vs hard-fail
    ///
    /// Returns a [`LoadOutcome`] describing the pass:
    /// [`Loaded`](LoadOutcome::Loaded) (all wallets loaded),
    /// [`Partial`](LoadOutcome::Partial) (some loaded, some skipped), or
    /// [`NoneUsable`](LoadOutcome::NoneUsable) (rows present, all skipped).
    ///
    /// - **Per-row decode failure** (empty manifest, malformed xpub,
    ///   snapshot/row mismatch, …): the wallet is **skipped** — never
    ///   inserted into `wallet_manager` / `self.wallets`, recorded in the
    ///   outcome's skip set with a structural
    ///   [`SkipReason::CorruptPersistedRow`], and
    ///   [`on_wallet_skipped_on_load`](crate::PlatformEventHandler::on_wallet_skipped_on_load)
    ///   fires on each registered handler. One bad row never aborts the
    ///   others; the call still returns `Ok`.
    /// - **Already present** (a repeat restore or a runtime-created
    ///   wallet, detected either at the pre-reconstruction idempotency
    ///   check or as `WalletExists` at insert): the wallet is **skipped**
    ///   with [`SkipReason::AlreadyRegistered`] and left untouched — kept
    ///   out of the rollback set so a later hard-fail never evicts it. A
    ///   second `load_from_persistor` therefore mutates no state and returns
    ///   `Ok(LoadOutcome)` — a repeat where every wallet is already
    ///   registered is a [`NoneUsable`](LoadOutcome::NoneUsable) no-op, not a
    ///   failure — and the caller can tell an already-present wallet from one
    ///   freshly loaded via [`loaded`](LoadOutcome::loaded) /
    ///   [`skipped`](LoadOutcome::skipped).
    /// - **Whole-load failure** (persister I/O, programmer error,
    ///   registering a persisted wallet in `WalletManager`):
    ///   `Err(_)` — every wallet inserted earlier in this pass is
    ///   rolled back. Skipped wallets never entered the maps so the
    ///   rollback path never sees them.
    ///
    /// Platform-address provider state is restored per wallet via
    /// [`initialize_from_persisted`](crate::wallet::platform_addresses::PlatformAddressWallet::initialize_from_persisted),
    /// or a fresh
    /// [`initialize`](crate::wallet::platform_addresses::PlatformAddressWallet::initialize)
    /// when the snapshot carries no slice for it.
    ///
    /// # Trust boundary
    ///
    /// The persisted account manifest is trusted as-is — it is **not**
    /// cryptographically bound to its `wallet_id` (see `build_watch_only_wallet`
    /// in `rehydrate`). A corrupted or tampered store can rebuild a wallet whose
    /// receive addresses derive from the wrong key under the original id;
    /// authenticating the manifest on load is a tracked storage-schema follow-up.
    pub async fn load_from_persistor(&self) -> Result<LoadOutcome, PlatformWalletError> {
        let ClientStartState {
            mut platform_addresses,
            wallets,
            skipped: persister_skipped,
            // Shielded restore happens lazily on `bind_shielded`,
            // not here — drop the snapshot at this entry point.
            #[cfg(feature = "shielded")]
                shielded: _,
        } = self.persister.load()?;

        let persister_dyn: Arc<dyn PlatformWalletPersistence> = Arc::clone(&self.persister) as _;

        // Transactional batch: every wallet inserted into
        // `wallet_manager` / `self.wallets` is tracked so a later hard
        // error walks back every prior insert. Skipped wallets never
        // enter either map, so the rollback path never sees them.
        let mut inserted_in_manager: Vec<WalletId> = Vec::new();
        let mut inserted_in_wallets: Vec<WalletId> = Vec::new();
        let mut load_error: Option<PlatformWalletError> = None;
        let mut loaded: Vec<WalletId> = Vec::new();
        let mut skipped: Vec<(WalletId, SkipReason)> = Vec::new();

        // Rows the persister rejected as corrupt before reconstruction
        // (e.g. a malformed xpub that aborts FFI decode) never reach the
        // rebuild loop below — fold them into the skip set and notify, so
        // one bad persisted row never blocks the batch.
        for (wallet_id, reason) in persister_skipped {
            self.event_manager
                .on_wallet_skipped_on_load(wallet_id, &reason);
            skipped.push((wallet_id, reason));
        }

        'load: for (expected_wallet_id, wallet_state) in wallets {
            let ClientWalletStartState {
                network,
                // The carried snapshot supplies its own sync metadata, so
                // the row's birth height is not needed on this path.
                birth_height: _,
                account_manifest,
                wallet_info,
                identity_manager,
                unused_asset_locks,
            } = wallet_state;

            // Idempotency, checked FIRST: a wallet already registered (a
            // prior load pass, or a runtime create) is not freshly loaded
            // by this pass — report it as an `AlreadyRegistered` skip so
            // the caller can tell it apart from a genuine load. Checking
            // before any reconstruction work matters — the rebuild below
            // derives eager gap windows (and possibly a deep discovery
            // scan), all of which the `WalletExists` arm at insert time
            // would only throw away.
            {
                let wm = self.wallet_manager.read().await;
                if wm.get_wallet(&expected_wallet_id).is_some() {
                    let reason = SkipReason::AlreadyRegistered;
                    self.event_manager
                        .on_wallet_skipped_on_load(expected_wallet_id, &reason);
                    skipped.push((expected_wallet_id, reason));
                    continue 'load;
                }
            }

            // Build the watch-only wallet from the keyless manifest. A
            // structural decode failure skips this row (per-row
            // resilience) — it never aborts the batch and never inserts
            // a degraded placeholder.
            let wallet = match super::rehydrate::build_watch_only_wallet(
                network,
                expected_wallet_id,
                &account_manifest,
            ) {
                Ok(w) => w,
                Err(kind) => {
                    let reason = SkipReason::CorruptPersistedRow { kind };
                    skipped.push((expected_wallet_id, reason.clone()));
                    self.event_manager
                        .on_wallet_skipped_on_load(expected_wallet_id, &reason);
                    continue 'load;
                }
            };

            // Full keyless snapshot carried by the persister: consume it
            // directly. This preserves per-account UTXO/record attribution,
            // the exact pool contents (derived-but-unused addresses stay in
            // the SPV watch set), and per-index used flags.
            let wallet_info = {
                let mut info = *wallet_info;
                // The snapshot must describe this row's wallet and its
                // account set must agree with the manifest that built the
                // watch-only wallet above. Either mismatch is a wrong-row
                // snapshot — skipped like any structural failure, kept
                // distinct from unreadable bytes.
                if info.wallet_id != expected_wallet_id
                    || info.network != network
                    || !snapshot_accounts_match_manifest(&info, &account_manifest)
                {
                    let reason = SkipReason::CorruptPersistedRow {
                        kind: CorruptKind::SnapshotIdentityMismatch,
                    };
                    skipped.push((expected_wallet_id, reason.clone()));
                    self.event_manager
                        .on_wallet_skipped_on_load(expected_wallet_id, &reason);
                    continue 'load;
                }
                // Recompute totals from the carried UTXO set so the
                // lock-free balance mirrored below can never drift from it
                // (no-silent-zero holds by recomputation).
                {
                    use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
                    info.update_balance();
                }
                info
            };

            // Flatten the (account → outpoint → lock) map.
            let mut tracked_asset_locks = BTreeMap::new();
            for (_account_index, account_locks) in unused_asset_locks {
                tracked_asset_locks.extend(account_locks);
            }

            let balance = Arc::new(WalletBalance::new());
            let core_balance = &wallet_info.balance;
            balance.set(
                core_balance.confirmed(),
                core_balance.unconfirmed(),
                core_balance.immature(),
                core_balance.locked(),
            );
            // Build the identity manager from the snapshot; public keys
            // and contacts are already reconstructed into it upstream by
            // the FFI persister (`build_wallet_identity_bucket`).
            let identity_manager = IdentityManager::from(identity_manager);
            let platform_info = PlatformWalletInfo {
                core_wallet: wallet_info,
                balance: Arc::clone(&balance),
                identity_manager,
                tracked_asset_locks,
            };

            let wallet_id = {
                let mut wm = self.wallet_manager.write().await;
                match wm.insert_wallet(wallet, platform_info) {
                    Ok(id) => id,
                    Err(key_wallet_manager::WalletError::WalletExists(_)) => {
                        // Idempotent restore, lost the insert race: a
                        // concurrent pass (or a runtime create) registered
                        // this wallet after our pre-reconstruction check.
                        // Re-registering must not abort the batch — record
                        // it as an `AlreadyRegistered` skip and continue. It
                        // was NOT inserted by this pass, so it stays out of
                        // the rollback set and a later hard-fail never evicts
                        // the pre-existing wallet.
                        let reason = SkipReason::AlreadyRegistered;
                        self.event_manager
                            .on_wallet_skipped_on_load(expected_wallet_id, &reason);
                        skipped.push((expected_wallet_id, reason));
                        continue 'load;
                    }
                    Err(e) => {
                        load_error = Some(PlatformWalletError::WalletCreation(format!(
                            "Failed to register persisted wallet in WalletManager: {}",
                            e
                        )));
                        break 'load;
                    }
                }
            };
            inserted_in_manager.push(wallet_id);

            let broadcaster = Arc::new(crate::broadcaster::SpvBroadcaster::new(Arc::clone(
                &self.spv_manager,
            )));
            let platform_wallet = PlatformWallet::new(
                Arc::clone(&self.sdk),
                wallet_id,
                Arc::clone(&self.wallet_manager),
                balance,
                Arc::clone(&self.lock_notify),
                Arc::clone(&persister_dyn),
                broadcaster,
            );

            if let Some(persisted) = platform_addresses.remove(&wallet_id) {
                if let Err(e) = platform_wallet
                    .platform()
                    .initialize_from_persisted(persisted)
                    .await
                {
                    load_error = Some(PlatformWalletError::WalletCreation(format!(
                        "Failed to restore platform address state: {}",
                        e
                    )));
                    break 'load;
                }
            } else {
                platform_wallet.platform().initialize().await;
            }

            let platform_wallet = Arc::new(platform_wallet);
            let mut wallets_guard = self.wallets.write().await;
            wallets_guard.insert(wallet_id, platform_wallet);
            drop(wallets_guard);
            inserted_in_wallets.push(wallet_id);
            loaded.push(wallet_id);
        }

        if let Some(err) = load_error {
            if !inserted_in_wallets.is_empty() {
                let mut wallets_guard = self.wallets.write().await;
                for id in &inserted_in_wallets {
                    wallets_guard.remove(id);
                }
            }
            if !inserted_in_manager.is_empty() {
                let mut wm = self.wallet_manager.write().await;
                for id in &inserted_in_manager {
                    if let Err(e) = wm.remove_wallet(id) {
                        tracing::warn!(
                            wallet_id = %hex::encode(id),
                            error = %e,
                            "rollback after load failure: remove_wallet failed"
                        );
                    }
                }
            }
            return Err(err);
        }

        Ok(LoadOutcome::from_parts(loaded, skipped))
    }
}

/// Whether the snapshot's account set matches the row's account manifest.
///
/// A **self-consistency** check between two pieces of the same persisted
/// row, not an authenticity guard: the manifest is the account-set oracle
/// used to build the watch-only wallet, and a snapshot carrying a
/// different set of account types is internally inconsistent with it and
/// must not be consumed. It does not attest that the manifest itself is
/// genuine — that trust boundary lives in `build_watch_only_wallet`.
///
/// The manifest is enumerated from `Wallet::all_accounts` (ECDSA-only:
/// carries `PlatformPayment`, omits the BLS `ProviderOperatorKeys` /
/// EdDSA `ProviderPlatformKeys`); the snapshot from
/// `ManagedWalletInfo::all_managed_accounts` (the mirror: carries the
/// BLS/EdDSA provider keys, omits `PlatformPayment`). Comparison is
/// restricted to the families both enumerations can carry so this known
/// asymmetry never rejects a legitimate snapshot.
fn snapshot_accounts_match_manifest(
    info: &ManagedWalletInfo,
    manifest: &[crate::changeset::AccountRegistrationEntry],
) -> bool {
    use key_wallet::account::AccountType;
    use std::collections::BTreeSet;

    fn comparable(t: &AccountType) -> bool {
        !matches!(
            t,
            AccountType::ProviderOperatorKeys
                | AccountType::ProviderPlatformKeys
                | AccountType::PlatformPayment { .. }
        )
    }

    let manifest_types: BTreeSet<AccountType> = manifest
        .iter()
        .map(|e| e.account_type)
        .filter(comparable)
        .collect();
    let snapshot_types: BTreeSet<AccountType> = info
        .all_managed_accounts()
        .iter()
        .map(|a| a.managed_account_type().to_account_type())
        .filter(comparable)
        .collect();
    manifest_types == snapshot_types
}

#[cfg(test)]
mod rollback_tests {
    //! In-crate because constructing the mid-batch hard-fail needs
    //! `PerAccountPlatformAddressState` + `bimap::BiBTreeMap`, which are
    //! not reachable from the external integration-test crate.

    use std::sync::Arc;

    use bimap::BiBTreeMap;
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;
    use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
    use key_wallet::wallet::Wallet;

    use crate::changeset::{
        AccountRegistrationEntry, ClientStartState, ClientWalletStartState, PersistenceError,
        PlatformWalletChangeSet, PlatformWalletPersistence,
    };
    use crate::error::PlatformWalletError;
    use crate::events::{EventHandler, PlatformEventHandler};
    use crate::wallet::platform_wallet::WalletId;
    use crate::wallet::{PerAccountPlatformAddressState, PerWalletPlatformAddressState};
    use crate::{PlatformAddressSyncStartState, PlatformWalletManager};

    fn slice(seed: [u8; 64]) -> (WalletId, ClientWalletStartState) {
        let wallet = Wallet::from_seed_bytes(
            seed,
            key_wallet::Network::Testnet,
            WalletAccountCreationOptions::Default,
        )
        .unwrap();
        let id = wallet.compute_wallet_id();
        let account_manifest: Vec<AccountRegistrationEntry> = wallet
            .accounts
            .all_accounts()
            .into_iter()
            .map(|a| AccountRegistrationEntry {
                account_type: a.account_type,
                account_xpub: a.account_xpub,
            })
            .collect();
        let info = ManagedWalletInfo::from_wallet(&wallet, 1);
        (
            id,
            ClientWalletStartState {
                network: key_wallet::Network::Testnet,
                birth_height: 1,
                account_manifest,
                wallet_info: Box::new(info),
                identity_manager: Default::default(),
                unused_asset_locks: Default::default(),
            },
        )
    }

    /// Two healthy wallets plus a platform-address restore state for the
    /// second that references an account index the wallet does not have
    /// as a platform-payment account, so `initialize_from_persisted`
    /// hard-fails mid-batch. Rebuilt fresh on every `load` — the platform
    /// address state types are not `Clone`.
    struct RollbackPersister {
        healthy_seed: [u8; 64],
        fail_seed: [u8; 64],
    }

    impl PlatformWalletPersistence for RollbackPersister {
        fn store(&self, _: WalletId, _: PlatformWalletChangeSet) -> Result<(), PersistenceError> {
            Ok(())
        }
        fn flush(&self, _: WalletId) -> Result<(), PersistenceError> {
            Ok(())
        }
        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            let mut st = ClientStartState::default();
            let (healthy_id, healthy) = slice(self.healthy_seed);
            let (fail_id, fail) = slice(self.fail_seed);
            let bogus_xpub = fail.account_manifest[0].account_xpub;
            st.wallets.insert(healthy_id, healthy);
            st.wallets.insert(fail_id, fail);

            // A per-account restore entry keyed to an account index the
            // wallet has no platform-payment account for: `from_persisted`
            // returns `AddressSync`, which the manager treats as a hard
            // failure (not a per-row skip).
            let mut per_account = PerWalletPlatformAddressState::new();
            per_account.insert(
                9_999,
                PerAccountPlatformAddressState::from_persisted(
                    bogus_xpub,
                    BiBTreeMap::new(),
                    std::collections::BTreeMap::new(),
                ),
            );
            st.platform_addresses.insert(
                fail_id,
                PlatformAddressSyncStartState {
                    per_account,
                    sync_height: 0,
                    sync_timestamp: 0,
                    last_known_recent_block: 0,
                },
            );
            Ok(st)
        }
    }

    struct NoopHandler;
    impl EventHandler for NoopHandler {}
    impl PlatformEventHandler for NoopHandler {}

    /// A hard failure part-way through a batch must roll back every wallet
    /// already inserted this pass — from BOTH `self.wallets` and
    /// `wallet_manager` — and surface as `Err`. Nothing asserted this
    /// before.
    #[tokio::test]
    async fn hard_fail_rolls_back_already_loaded_healthy_wallet() {
        // Order the seeds so the healthy wallet sorts BEFORE the failing
        // one (wallets load in BTreeMap key order), guaranteeing the
        // healthy wallet is fully inserted before the failure aborts.
        let (id_a, _) = slice([0xA1; 64]);
        let (id_b, _) = slice([0xB2; 64]);
        let (healthy_seed, fail_seed) = if id_a < id_b {
            ([0xA1; 64], [0xB2; 64])
        } else {
            ([0xB2; 64], [0xA1; 64])
        };
        let (healthy_id, _) = slice(healthy_seed);

        let persister = Arc::new(RollbackPersister {
            healthy_seed,
            fail_seed,
        });
        let sdk = Arc::new(dash_sdk::Sdk::new_mock());
        let handler: Arc<dyn PlatformEventHandler> = Arc::new(NoopHandler);
        let mgr = Arc::new(PlatformWalletManager::new(sdk, persister, handler));

        let err = mgr
            .load_from_persistor()
            .await
            .expect_err("a mid-batch hard failure must surface as Err");
        assert!(
            matches!(err, PlatformWalletError::WalletCreation(_)),
            "a platform-address restore failure is a WalletCreation hard error, got {err:?}"
        );

        // The healthy wallet was fully inserted, then rolled back on the
        // failure — gone from self.wallets...
        assert!(
            mgr.get_wallet(&healthy_id).await.is_none(),
            "the already-loaded healthy wallet must be evicted from self.wallets"
        );
        assert!(
            mgr.wallet_ids().await.is_empty(),
            "self.wallets must be fully rolled back"
        );
        // ...and from wallet_manager (read via the blocking accessor).
        let rows = {
            let mgr = Arc::clone(&mgr);
            tokio::task::spawn_blocking(move || mgr.account_balances_blocking(&healthy_id))
                .await
                .unwrap()
        };
        assert!(
            rows.is_empty(),
            "the already-loaded healthy wallet must be evicted from wallet_manager too"
        );
    }
}
