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
    /// Core state comes in one of two shapes, per wallet:
    /// - a full keyless snapshot
    ///   ([`ClientWalletStartState::core_wallet_info`]) — consumed
    ///   directly, preserving per-account UTXO/record attribution and
    ///   exact pool contents (the FFI/iOS persister); or
    /// - the keyless projection
    ///   ([`core_state`](ClientWalletStartState::core_state) +
    ///   [`used_core_addresses`](ClientWalletStartState::used_core_addresses)),
    ///   replayed onto a fresh skeleton via
    ///   [`apply_persisted_core_state`](super::rehydrate::apply_persisted_core_state)
    ///   (persisters that cannot reconstruct the snapshot).
    ///
    /// The load path never touches the seed, so it performs no wrong-seed
    /// check. Signing happens later, on demand, via the configured
    /// `MnemonicResolverHandle` (`rs-sdk-ffi`).
    ///
    /// # Skip vs hard-fail
    ///
    /// - **Per-row decode/projection failure** (empty manifest, malformed
    ///   xpub, duplicate `account_type`, …): the wallet is **skipped** —
    ///   never inserted into `wallet_manager` / `self.wallets`, recorded
    ///   in [`LoadOutcome::skipped`] with a structural
    ///   [`SkipReason::CorruptPersistedRow`], and
    ///   [`on_wallet_skipped_on_load`](crate::PlatformEventHandler::on_wallet_skipped_on_load)
    ///   is called on each registered handler. One bad row
    ///   never aborts the others; the call still returns `Ok`.
    /// - **Whole-load failure** (persister I/O, programmer error, the
    ///   no-silent-zero topology check in
    ///   [`apply_persisted_core_state`](super::rehydrate::apply_persisted_core_state)):
    ///   `Err(_)` — every wallet inserted earlier in this pass is
    ///   rolled back. Skipped wallets never entered the maps so the
    ///   rollback path never sees them.
    /// - **Already present** (`WalletExists` from `insert_wallet`, e.g. a
    ///   repeat restore or a runtime-created wallet): treated as
    ///   already-satisfied — counted as loaded, left untouched, and kept
    ///   out of the rollback set so a later hard-fail never evicts it. A
    ///   second `load_from_persistor` is therefore idempotent.
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
        } = self.persister.load().map_err(|e| {
            PlatformWalletError::WalletCreation(format!(
                "Failed to load persisted client state: {}",
                e
            ))
        })?;

        let persister_dyn: Arc<dyn PlatformWalletPersistence> = Arc::clone(&self.persister) as _;

        // Transactional batch: every wallet inserted into
        // `wallet_manager` / `self.wallets` is tracked so a later hard
        // error walks back every prior insert. Skipped wallets never
        // enter either map, so the rollback path never sees them.
        let mut inserted_in_manager: Vec<WalletId> = Vec::new();
        let mut inserted_in_wallets: Vec<WalletId> = Vec::new();
        let mut load_error: Option<PlatformWalletError> = None;
        let mut outcome = LoadOutcome::default();

        // Rows the persister rejected as corrupt before reconstruction
        // (e.g. a malformed xpub that aborts FFI decode) never reach the
        // rebuild loop below — fold them into the skip set and notify, so
        // one bad persisted row never blocks the batch.
        for (wallet_id, reason) in persister_skipped {
            self.event_manager
                .on_wallet_skipped_on_load(wallet_id, &reason);
            outcome.skipped.push((wallet_id, reason));
        }

        'load: for (expected_wallet_id, wallet_state) in wallets {
            let ClientWalletStartState {
                network,
                birth_height,
                account_manifest,
                core_wallet_info,
                core_state,
                identity_manager,
                unused_asset_locks,
                contacts,
                identity_keys,
                used_core_addresses,
            } = wallet_state;

            // Idempotency, checked FIRST: a wallet already registered (a
            // prior load pass, or a runtime create) is already-satisfied.
            // Checking before any reconstruction work matters — the
            // rebuild below derives eager gap windows (and possibly a
            // deep discovery scan), all of which the `WalletExists` arm
            // at insert time would only throw away.
            {
                let wm = self.wallet_manager.read().await;
                if wm.get_wallet(&expected_wallet_id).is_some() {
                    outcome.loaded.push(expected_wallet_id);
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
                    outcome.skipped.push((expected_wallet_id, reason.clone()));
                    self.event_manager
                        .on_wallet_skipped_on_load(expected_wallet_id, &reason);
                    continue 'load;
                }
            };

            let wallet_info = match core_wallet_info {
                // Full keyless snapshot carried by the persister (the
                // FFI/iOS path): consume it directly. This preserves
                // per-account UTXO/record attribution, the exact pool
                // contents (derived-but-unused addresses stay in the SPV
                // watch set), and per-index used flags — none of which
                // the projection replay below can reconstruct — and
                // skips a second eager gap-window derivation.
                Some(info) => {
                    let mut info = *info;
                    // The snapshot must describe this row's wallet; a
                    // mismatch is a corrupt row, skipped like any other
                    // structural failure.
                    if info.wallet_id != expected_wallet_id || info.network != network {
                        let reason = SkipReason::CorruptPersistedRow {
                            kind: CorruptKind::DecodeError(format!(
                                "managed-info snapshot (wallet {}, network {:?}) does not \
                                 match its row (wallet {}, network {:?})",
                                hex::encode(info.wallet_id),
                                info.network,
                                hex::encode(expected_wallet_id),
                                network,
                            )),
                        };
                        outcome.skipped.push((expected_wallet_id, reason.clone()));
                        self.event_manager
                            .on_wallet_skipped_on_load(expected_wallet_id, &reason);
                        continue 'load;
                    }
                    // Recompute totals from the carried UTXO set so the
                    // lock-free balance mirrored below can never drift
                    // from it (no-silent-zero holds by recomputation).
                    {
                        use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
                        info.update_balance();
                    }
                    info
                }
                // No snapshot (native/SQLite persister until
                // dashpay/platform#3968): mint the managed-info skeleton
                // from the watch-only wallet, then replay the keyless
                // projection (UTXOs, sync watermarks, used addresses). A
                // wallet with persisted UTXOs but no funds account
                // hard-fails here rather than reconstructing a silent
                // zero balance.
                None => {
                    let mut wallet_info = ManagedWalletInfo::from_wallet(&wallet, birth_height);
                    if let Err(e) = super::rehydrate::apply_persisted_core_state(
                        &mut wallet_info,
                        &account_manifest,
                        &core_state,
                        &used_core_addresses,
                    ) {
                        load_error = Some(e);
                        break 'load;
                    }
                    wallet_info
                }
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
            // Build the identity manager from the (id, balance,
            // revision) skeleton, then layer the persisted PUBLIC
            // contacts + identity keys onto it — the same routing the
            // runtime changeset-replay path uses.
            let mut identity_manager = IdentityManager::from(identity_manager);
            identity_manager.apply_contacts_and_keys(contacts, identity_keys, network);
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
                        // Idempotent restore: a prior `load_from_persistor`
                        // (or a runtime create) already registered this
                        // wallet. Re-registering must not abort the batch —
                        // treat it as already-satisfied: record it as loaded
                        // and continue. It was NOT inserted by this pass, so
                        // it stays out of the rollback set and a later
                        // hard-fail never evicts the pre-existing wallet.
                        outcome.loaded.push(expected_wallet_id);
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
            outcome.loaded.push(wallet_id);
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
                    let _ = wm.remove_wallet(id);
                }
            }
            return Err(err);
        }

        Ok(outcome)
    }
}
