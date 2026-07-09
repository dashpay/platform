//! Read-only accessors on [`PlatformWalletManager`].

use std::sync::Arc;

use dashcore::{OutPoint, Txid};
use dpp::prelude::Identifier;
use key_wallet::account::AccountType;
use key_wallet::managed_account::address_pool::{AddressInfo, AddressPool, AddressPoolType};
use key_wallet::managed_account::transaction_record::TransactionRecord;
use key_wallet::utxo::Utxo;
use key_wallet::WalletCoreBalance;

use crate::changeset::PlatformWalletPersistence;
use crate::manager::dashpay_sync::DashPaySyncManager;
use crate::manager::identity_sync::IdentitySyncManager;
use crate::manager::platform_address_sync::PlatformAddressSyncManager;
#[cfg(feature = "shielded")]
use crate::manager::shielded_sync::ShieldedSyncManager;
use crate::spv::SpvRuntime;
use crate::wallet::platform_wallet::WalletId;
use crate::wallet::PlatformWallet;

use super::PlatformWalletManager;

/// Snapshot of [`PlatformAddressSyncManager`] tunables and last-event
/// counters, returned from
/// [`PlatformWalletManager::platform_address_sync_config_blocking`].
///
/// `last_event_wallet_count` was dropped — it aliased
/// `watch_list_size` (both read `wallets.len()`) and rendering it as
/// an independent observation in the explorer was misleading. If a
/// real per-event footprint metric ever lands on the sync manager,
/// add it back as a separate field sourced from there.
#[derive(Debug, Clone, Copy)]
pub struct PlatformAddressSyncConfigSnapshot {
    pub interval_seconds: u64,
    pub watch_list_size: usize,
    pub last_event_unix_seconds: u64,
}

/// One row of the account-balance snapshot returned by
/// [`PlatformWalletManager::account_balances_blocking`]. Named fields
/// rather than a positional tuple so adding the next field
/// (`pool_count`, `last_used_height`, …) doesn't ripple through every
/// destructuring site.
#[derive(Debug, Clone, Copy)]
pub struct AccountBalanceRow {
    pub account_type: AccountType,
    pub balance: WalletCoreBalance,
    pub keys_used: u32,
    pub keys_total: u32,
}

/// Snapshot of [`IdentitySyncManager`] tunables / queue depth, returned
/// from [`PlatformWalletManager::identity_sync_config_blocking`].
#[derive(Debug, Clone, Copy)]
pub struct IdentitySyncConfigSnapshot {
    pub interval_seconds: u64,
    pub queue_depth: usize,
}

/// Snapshot of the core SPV state for a single wallet, returned from
/// [`PlatformWalletManager::core_wallet_state_blocking`].
#[derive(Debug, Clone, Copy)]
pub struct CoreWalletStateSnapshot {
    pub synced_height: u32,
    pub last_processed_height: u32,
    pub monitor_revision: u64,
}

/// Snapshot of the identity-wallet scan state for a single wallet,
/// returned from
/// [`PlatformWalletManager::identity_wallet_state_blocking`].
///
/// `last_scanned_index` is sourced from
/// `IdentityManager::highest_registration_index`, which replaced the
/// old explicit `last_scanned_index` watermark — see the doc comment
/// on that accessor.
///
/// `scan_pending` is reserved for future use; the gap-limit scan now
/// resumes implicitly from `highest_registration_index + 1` rather
/// than carrying a flag on the manager, so this value is always
/// `false` today.
#[derive(Debug, Clone, Copy)]
pub struct IdentityWalletStateSnapshot {
    pub last_scanned_index: u32,
    pub scan_pending: bool,
}

/// Snapshot of the platform-address provider state for a single
/// wallet, returned from
/// [`PlatformWalletManager::platform_address_provider_state_blocking`].
#[derive(Debug, Clone, Copy)]
pub struct PlatformAddressProviderStateSnapshot {
    pub initialized: bool,
    pub accounts_watched: usize,
    pub found_count: usize,
    pub known_balances_count: usize,
    pub watermark_height: u32,
}

// `WalletInfoMetadataSnapshot` and `wallet_info_metadata_blocking`
// were removed: the diagnostic explorer's "PlatformWalletInfo Metadata"
// section duplicated `CoreWalletStateSnapshot` (heights/revision) and
// surfaced fields with no active populator (total_transactions is
// event-driven; first_loaded_at isn't stamped on this path; name /
// description are wallet-row labels, not part of the in-memory diag
// surface). Re-add only if a future caller needs the name/description
// specifically.

/// One row of the tracked-asset-lock list, returned from
/// [`PlatformWalletManager::tracked_asset_locks_blocking`].
#[derive(Debug, Clone, Copy)]
pub struct TrackedAssetLockSnapshot {
    pub outpoint: OutPoint,
    /// 0 = `AssetLockBuilder` index funding type variant; project the
    /// upstream `AssetLockFundingType` enum into a u8 lazily — see
    /// [`asset_lock_funding_type_to_u8`].
    pub lock_type: u8,
    /// 0=Built, 1=Broadcast, 2=InstantSendLocked, 3=ChainLocked.
    pub status: u8,
    pub registration_index: u32,
    pub instant_lock_present: bool,
    pub chain_lock_height: u32,
}

/// Snapshot of the per-account metadata for a single account.
///
/// `is_watch_only` and `custom_name` were dropped after upstream
/// removed both from `ManagedCoreFundsAccount` / `ManagedCoreKeysAccount`.
/// Watch-only is now a wallet-level property (read off `Wallet.wallet_type`)
/// and `AccountMetadata` no longer exists. Re-add fields here only if
/// the upstream variants gain them again.
#[derive(Debug, Clone, Copy)]
pub struct AccountMetadataSnapshot {
    pub total_transactions: u64,
    pub total_utxos: u64,
    pub monitor_revision: u64,
}

/// Snapshot of one address-pool slot for the per-account drill-down.
#[derive(Debug, Clone)]
pub struct AccountAddressPoolSnapshot {
    /// 0=External, 1=Internal, 2=Absent, 3=AbsentHardened.
    pub pool_type: u8,
    pub gap_limit: u32,
    /// `i64`-encoded so `-1` cleanly signals "no addresses used yet"
    /// without needing a side-channel. Fits inside the FFI surface
    /// without splitting the field.
    pub last_used_index: i64,
    pub addresses: Vec<AccountAddressInfoSnapshot>,
}

/// Snapshot of a single derived address inside an
/// [`AccountAddressPoolSnapshot`].
#[derive(Debug, Clone)]
pub struct AccountAddressInfoSnapshot {
    /// 20-byte HASH160 of the derived public key (i.e. the P2PKH
    /// payload). Sourced from the address's `script_pubkey`.
    pub pubkey_hash: [u8; 20],
    pub address_index: u32,
    pub is_used: bool,
    /// Encoded address as the user would see it (Base58check P2PKH for
    /// every account variant the explorer surfaces today). Built from
    /// `AddressInfo.address.to_string()`.
    pub address: String,
    /// Raw bytes of the public key that derived this address — empty
    /// when `AddressInfo.public_key` is `None` (e.g. address-only
    /// pools that don't carry the derived key). Variant info (ECDSA /
    /// EdDSA / BLS) is not surfaced separately; the bytes are typed
    /// implicitly by the owning account variant.
    pub public_key_bytes: Vec<u8>,
}

/// Snapshot of one UTXO row inside an account.
#[derive(Debug, Clone)]
pub struct AccountUtxoSnapshot {
    pub outpoint: OutPoint,
    pub value_duffs: u64,
    pub script_pubkey: Vec<u8>,
    pub height: u32,
    pub is_locked: bool,
}

/// Snapshot of one transaction row inside an account.
#[derive(Debug, Clone, Copy)]
pub struct AccountTransactionSnapshot {
    pub txid: Txid,
    pub height: u32,
    pub timestamp: u64,
    pub value_delta_duffs: i64,
    pub fee_duffs: u64,
    pub is_coinbase: bool,
}

/// One row of the wallet-bound identity list (registration index +
/// identity id) returned from
/// [`PlatformWalletManager::identity_manager_wallet_identities_blocking`].
#[derive(Debug, Clone, Copy)]
pub struct WalletIdentityRowSnapshot {
    pub registration_index: u32,
    pub identity_id: [u8; 32],
}

/// One row of the DAPI address ban-list snapshot, returned from
/// [`PlatformWalletManager::address_ban_info_blocking`].
///
/// A platform-wallet-owned mirror of the SDK's `AddressBanInfo` with
/// `banned_until` already projected to a millisecond Unix timestamp so
/// the FFI layer can marshal it without depending on `chrono`.
#[derive(Debug, Clone)]
pub struct AddressBanInfoSnapshot {
    /// The DAPI node URI.
    pub uri: String,
    /// Whether the address is currently effectively banned (banned at
    /// least once and the ban period has not yet expired).
    pub banned: bool,
    /// Total number of times the address has been banned.
    pub ban_count: usize,
    /// Unix-epoch millisecond timestamp until which the address is
    /// banned, or `None` if there is no active ban window.
    pub banned_until_ms: Option<i64>,
    /// Human-readable reason for the most recent ban, if recorded.
    pub reason: Option<String>,
}

impl<P: PlatformWalletPersistence + 'static> PlatformWalletManager<P> {
    /// The SDK instance.
    pub fn sdk(&self) -> &dash_sdk::Sdk {
        &self.sdk
    }

    /// Snapshot of every DAPI address' ban state, including the reason
    /// each address was banned (when recorded).
    ///
    /// Delegates to the SDK's `address_ban_info`, projecting the
    /// `chrono` timestamp into a Unix-epoch millisecond `i64` so the
    /// FFI layer can marshal it without a `chrono` dependency. This is
    /// a pure read — no async, no lock contention on the wallet manager.
    pub fn address_ban_info_blocking(&self) -> Vec<AddressBanInfoSnapshot> {
        self.sdk
            .address_ban_info()
            .into_iter()
            .map(|info| AddressBanInfoSnapshot {
                uri: info.uri,
                banned: info.banned,
                ban_count: info.ban_count,
                banned_until_ms: info.banned_until.map(|t| t.timestamp_millis()),
                reason: info.reason,
            })
            .collect()
    }

    /// Access the SPV runtime for sync control.
    pub fn spv(&self) -> &SpvRuntime {
        &self.spv_manager
    }

    /// Clone the `Arc<SpvRuntime>` so callers (e.g. FFI) can invoke
    /// [`SpvRuntime::spawn_run_loop`] which takes `&Arc<Self>`.
    pub fn spv_arc(&self) -> Arc<SpvRuntime> {
        Arc::clone(&self.spv_manager)
    }

    /// Access the platform-address sync coordinator.
    pub fn platform_address_sync(&self) -> &PlatformAddressSyncManager {
        &self.platform_address_sync_manager
    }

    /// Clone the `Arc<PlatformAddressSyncManager>` so callers (e.g. FFI)
    /// can invoke [`PlatformAddressSyncManager::start`] which takes
    /// `&Arc<Self>`.
    pub fn platform_address_sync_arc(&self) -> Arc<PlatformAddressSyncManager> {
        Arc::clone(&self.platform_address_sync_manager)
    }

    /// Access the per-identity token state sync coordinator.
    pub fn identity_sync(&self) -> &IdentitySyncManager<P> {
        &self.identity_sync_manager
    }

    /// Clone the `Arc<IdentitySyncManager<P>>` so callers (e.g. FFI)
    /// can invoke [`IdentitySyncManager::start`] which takes
    /// `&Arc<Self>`.
    pub fn identity_sync_arc(&self) -> Arc<IdentitySyncManager<P>> {
        Arc::clone(&self.identity_sync_manager)
    }

    /// Access the recurring DashPay (contact-request + profile) sync
    /// coordinator.
    pub fn dashpay_sync(&self) -> &DashPaySyncManager {
        &self.dashpay_sync_manager
    }

    /// Clone the `Arc<DashPaySyncManager>` so callers (e.g. FFI) can
    /// invoke [`DashPaySyncManager::start`] which takes `&Arc<Self>`.
    pub fn dashpay_sync_arc(&self) -> Arc<DashPaySyncManager> {
        Arc::clone(&self.dashpay_sync_manager)
    }

    /// Access the shielded sync coordinator.
    #[cfg(feature = "shielded")]
    pub fn shielded_sync(&self) -> &ShieldedSyncManager {
        &self.shielded_sync_manager
    }

    /// Clone the `Arc<ShieldedSyncManager>` so callers (e.g. FFI)
    /// can invoke [`ShieldedSyncManager::start`] which takes
    /// `&Arc<Self>`.
    #[cfg(feature = "shielded")]
    pub fn shielded_sync_arc(&self) -> Arc<ShieldedSyncManager> {
        Arc::clone(&self.shielded_sync_manager)
    }

    /// Get a clone of a wallet by its ID.
    pub async fn get_wallet(&self, wallet_id: &WalletId) -> Option<Arc<PlatformWallet>> {
        let wallets = self.wallets.read().await;
        wallets.get(wallet_id).cloned()
    }

    /// List all wallet IDs.
    pub async fn wallet_ids(&self) -> Vec<WalletId> {
        let wallets = self.wallets.read().await;
        wallets.keys().copied().collect()
    }

    /// Read per-account balance + key-usage snapshots for a wallet.
    ///
    /// Returns one [`AccountBalanceSnapshot`] per managed account: the
    /// wallet's `AccountType`, the live `WalletCoreBalance` (zero on
    /// keys-only variants by construction), and (`keys_used`,
    /// `keys_total`) totals across the account's address pools.
    /// Funds variants and keys variants both expose pools the same
    /// way, so the count is meaningful in both directions — the
    /// explorer surfaces it as the headline number on keys-only rows
    /// where balance has no semantic content.
    ///
    /// Uses `blocking_read` on the wallet manager lock; safe from
    /// non-async FFI context but must NOT be called from within a
    /// tokio async task.
    pub fn account_balances_blocking(&self, wallet_id: &WalletId) -> Vec<AccountBalanceRow> {
        let wm = self.wallet_manager.blocking_read();
        let Some(info) = wm.get_wallet_info(wallet_id) else {
            return Vec::new();
        };
        info.core_wallet
            .accounts
            .all_accounts()
            .iter()
            .map(|account| {
                // Balance lives on the funds-bearing variant only;
                // keys-only accounts (identity, asset-lock, provider)
                // never carry UTXOs.
                let balance = account.as_funds().map(|a| a.balance).unwrap_or_default();
                // Walk every pool on the account, sum
                // `used` + total entries. Cheap — pools are bounded by
                // the gap limit.
                let (keys_used, keys_total) = account
                    .managed_account_type()
                    .address_pools()
                    .iter()
                    .fold((0u32, 0u32), |(used, total), pool| {
                        let pool_used =
                            pool.addresses.values().filter(|info| info.used).count() as u32;
                        let pool_total = pool.addresses.len() as u32;
                        (used + pool_used, total + pool_total)
                    });
                AccountBalanceRow {
                    account_type: account.managed_account_type().to_account_type(),
                    balance,
                    keys_used,
                    keys_total,
                }
            })
            .collect()
    }

    // -----------------------------------------------------------------
    // Phase 2 — Manager-level diagnostic snapshots
    // -----------------------------------------------------------------

    /// Atomic snapshot of every wallet id currently registered on the
    /// manager. Cheap (`Arc<RwLock>` read + `BTreeMap` key clone).
    pub fn list_wallet_ids_blocking(&self) -> Vec<WalletId> {
        let wallets = self.wallets.blocking_read();
        wallets.keys().copied().collect()
    }

    /// Snapshot of [`PlatformAddressSyncManager`] tunables and last-
    /// pass timestamp. `watch_list_size` is `wallets.len()` — every
    /// registered wallet participates in each pass since the sync
    /// manager doesn't keep a separate watch list.
    pub fn platform_address_sync_config_blocking(&self) -> PlatformAddressSyncConfigSnapshot {
        let wallets = self.wallets.blocking_read();
        let count = wallets.len();
        drop(wallets);
        let interval = self.platform_address_sync_manager.interval();
        let last = self
            .platform_address_sync_manager
            .last_sync_unix_seconds()
            .unwrap_or(0);
        PlatformAddressSyncConfigSnapshot {
            interval_seconds: interval.as_secs().max(1),
            watch_list_size: count,
            last_event_unix_seconds: last,
        }
    }

    /// Snapshot of [`IdentitySyncManager`] tunables and queue depth.
    /// `queue_depth` reports the number of identities currently in the
    /// per-identity registry (i.e. the number of identities the next
    /// pass would touch). The manager doesn't expose a sync method to
    /// read the registry without an `await`, so we use the
    /// `interval_secs` getter and a coarse "is_running" probe.
    pub fn identity_sync_config_blocking(&self) -> IdentitySyncConfigSnapshot {
        let interval = self.identity_sync_manager.interval();
        // The registry behind `IdentitySyncManager.state` is async-only
        // (`tokio::sync::RwLock`). Use `blocking_read` on the registry
        // through a helper on the manager — since the registry itself
        // isn't exposed, fall back to "0" until a sync getter is
        // added. This is intentionally a TODO surface, not a guess.
        let queue_depth = self
            .identity_sync_manager
            .try_queue_depth()
            .unwrap_or_default();
        IdentitySyncConfigSnapshot {
            interval_seconds: interval.as_secs().max(1),
            queue_depth,
        }
    }

    // -----------------------------------------------------------------
    // Phase 3 — Per-wallet state
    // -----------------------------------------------------------------

    /// Snapshot of the core wallet's SPV bookkeeping for a single
    /// wallet. `monitor_revision` is the max across every account on
    /// the wallet — the max picks up the most recent address-set
    /// mutation the bloom-filter rebuilder cares about.
    pub fn core_wallet_state_blocking(
        &self,
        wallet_id: &WalletId,
    ) -> Option<CoreWalletStateSnapshot> {
        let wm = self.wallet_manager.blocking_read();
        let info = wm.get_wallet_info(wallet_id)?;
        let monitor_revision = info
            .core_wallet
            .accounts
            .all_accounts()
            .iter()
            .map(|a| a.monitor_revision())
            .max()
            .unwrap_or(0);
        Some(CoreWalletStateSnapshot {
            synced_height: info.core_wallet.metadata.synced_height,
            last_processed_height: info.core_wallet.metadata.last_processed_height,
            monitor_revision,
        })
    }

    /// Rewind a single wallet's SPV filter-scan checkpoint
    /// (`synced_height`) to `from_height`, arming an organic filter
    /// rescan.
    ///
    /// This is the write half of the same mechanism `reconcile_dashpay_rescan`
    /// uses for historical-contact backfill. It mutates the *shared*
    /// `wallet_manager` (`Arc<RwLock<..>>`) that the running `DashSpvClient`
    /// holds a clone of — so the change is observed by the live filter-sync
    /// loop: on its next tick `FiltersManager` sees this wallet in
    /// `wallets_behind(committed_height)`, calls `reset_for_rescan()`, rewinds
    /// its committed height to this wallet's `synced_height`, and re-downloads /
    /// re-matches compact filters from there.
    ///
    /// Unlike the `WalletInterface::update_wallet_synced_height` trait method
    /// (which is forward-only and silently ignores a lower value), this writes
    /// `core_wallet.update_synced_height` directly, which is an unconditional
    /// set — so a **rewind** actually takes effect. A `from_height` at or above
    /// the current checkpoint is written verbatim but arms no rescan: the
    /// filter loop only rescans wallets strictly *behind* the committed height,
    /// so a forward/equal set is a harmless no-op for the rescan purpose.
    ///
    /// `synced_height` may regress here: that is safe because it is the
    /// filter-scan checkpoint, decoupled from the monotonic
    /// `last_processed_height`, and every persisted sync cursor is
    /// monotonic-max guarded (see `reconcile_dashpay_rescan`'s note), so a
    /// transient rewind cannot corrupt state or persist a lower cursor.
    ///
    /// The rewound checkpoint lives in the in-memory `WalletManager`; it is not
    /// itself persisted by this call, and that is fine for the feature: a
    /// rescan completes in-session, and if the process dies mid-rescan the
    /// wallet is simply still behind, so the next `start` re-arms the same
    /// backfill from the persisted high-water. Requires SPV running for an
    /// immediate effect; otherwise it takes effect when SPV next starts and its
    /// filter loop first ticks.
    ///
    /// Returns `false` when no wallet matches `wallet_id`.
    pub fn spv_rescan_filters_blocking(
        &self,
        wallet_id: &WalletId,
        from_height: u32,
    ) -> bool {
        use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;

        let mut wm = self.wallet_manager.blocking_write();
        let Some(info) = wm.get_wallet_info_mut(wallet_id) else {
            return false;
        };
        info.core_wallet.update_synced_height(from_height);
        tracing::info!(
            wallet_id = %hex::encode(wallet_id),
            from_height,
            "SPV rescan: rewound wallet synced_height to arm a filter rescan"
        );
        true
    }

    /// Snapshot of identity-wallet scan state for a single wallet.
    /// See [`IdentityWalletStateSnapshot`] for the field doc and the
    /// upstream renaming history (the legacy `last_scanned_index`
    /// watermark was replaced with `highest_registration_index`).
    pub fn identity_wallet_state_blocking(
        &self,
        wallet_id: &WalletId,
    ) -> Option<IdentityWalletStateSnapshot> {
        let wm = self.wallet_manager.blocking_read();
        let info = wm.get_wallet_info(wallet_id)?;
        let last_scanned_index = info
            .identity_manager
            .highest_registration_index(wallet_id)
            .unwrap_or(0);
        Some(IdentityWalletStateSnapshot {
            last_scanned_index,
            // TODO(diagnostic): plumb a real `scan_pending` flag from
            // the discovery scan once the gap-limit walker carries
            // one. The watermark-only model can't express it.
            scan_pending: false,
        })
    }

    /// Snapshot of the unified [`PlatformPaymentAddressProvider`]
    /// state for a single wallet. Returns
    /// `initialized = false` (with zeroed counters) if the provider
    /// hasn't been built yet.
    ///
    /// `accounts_watched` counts platform payment accounts on this
    /// wallet that the provider tracks; `found_count` and
    /// `known_balances_count` aggregate across those accounts. The
    /// provider stores `found` / `addresses` per account, so both are
    /// summed.
    ///
    /// Acquires the provider's `RwLock` via `blocking_read` — must
    /// not be called from inside a tokio async task.
    pub fn platform_address_provider_state_blocking(
        &self,
        wallet_id: &WalletId,
    ) -> Option<PlatformAddressProviderStateSnapshot> {
        let wallets = self.wallets.blocking_read();
        let wallet = wallets.get(wallet_id)?.clone();
        drop(wallets);
        let provider_lock = wallet.platform().provider_for_diagnostics();
        let guard = provider_lock.blocking_read();
        let Some(provider) = guard.as_ref() else {
            return Some(PlatformAddressProviderStateSnapshot {
                initialized: false,
                accounts_watched: 0,
                found_count: 0,
                known_balances_count: 0,
                watermark_height: 0,
            });
        };
        let (accounts_watched, found_count, known_balances_count) =
            provider.diagnostic_counts(wallet_id);
        Some(PlatformAddressProviderStateSnapshot {
            initialized: true,
            accounts_watched,
            found_count,
            known_balances_count,
            watermark_height: provider.diagnostic_sync_height_u32(),
        })
    }

    // -----------------------------------------------------------------
    // Phase 4 — Wallet metadata + floating state
    // -----------------------------------------------------------------

    /// Snapshot of the wallet's tracked-asset-lock list. Reads the
    /// `info.tracked_asset_locks` map once under the lock.
    pub fn tracked_asset_locks_blocking(
        &self,
        wallet_id: &WalletId,
    ) -> Vec<TrackedAssetLockSnapshot> {
        let wm = self.wallet_manager.blocking_read();
        let Some(info) = wm.get_wallet_info(wallet_id) else {
            return Vec::new();
        };
        info.tracked_asset_locks
            .values()
            .map(|lock| {
                use crate::wallet::asset_lock::tracked::AssetLockStatus;
                let status: u8 = match &lock.status {
                    AssetLockStatus::Built => 0,
                    AssetLockStatus::Broadcast => 1,
                    AssetLockStatus::InstantSendLocked => 2,
                    AssetLockStatus::ChainLocked => 3,
                    AssetLockStatus::Consumed => 4,
                };
                let (instant_lock_present, chain_lock_height) = match &lock.proof {
                    Some(dpp::prelude::AssetLockProof::Instant(_)) => (true, 0u32),
                    Some(dpp::prelude::AssetLockProof::Chain(c)) => {
                        (false, c.core_chain_locked_height)
                    }
                    None => (false, 0u32),
                };
                TrackedAssetLockSnapshot {
                    outpoint: lock.out_point,
                    lock_type: asset_lock_funding_type_to_u8(&lock.funding_type),
                    status,
                    registration_index: lock.identity_index,
                    instant_lock_present,
                    chain_lock_height,
                }
            })
            .collect()
    }

    /// Snapshot of the wallet's InstantSend lock txid set. Returns
    /// the txids in `HashSet` iteration order (non-deterministic
    /// between runs, deterministic within a run while the set is
    /// untouched).
    pub fn instant_send_locks_blocking(&self, wallet_id: &WalletId) -> Vec<Txid> {
        let wm = self.wallet_manager.blocking_read();
        let Some(info) = wm.get_wallet_info(wallet_id) else {
            return Vec::new();
        };
        info.core_wallet
            .instant_send_locks()
            .iter()
            .copied()
            .collect()
    }

    // -----------------------------------------------------------------
    // Phase 5 — Per-account drill-down
    // -----------------------------------------------------------------

    /// Snapshot of the per-account metadata for one account.
    ///
    /// `target` is matched against the canonical `AccountType` projected
    /// from each `ManagedCoreAccount.managed_account_type` — same
    /// equality the changeset / persistence path uses.
    pub fn account_metadata_blocking(
        &self,
        wallet_id: &WalletId,
        target: &AccountType,
    ) -> Option<AccountMetadataSnapshot> {
        let wm = self.wallet_manager.blocking_read();
        let info = wm.get_wallet_info(wallet_id)?;
        let accounts = info.core_wallet.accounts.all_accounts();
        let account = accounts
            .iter()
            .find(|a| &a.managed_account_type().to_account_type() == target)?;
        // Funds-only fields (`utxos`) live on the funds variant; the
        // ref-enum delegates the rest. `transactions()` returns an
        // empty map when `keep-finalized-transactions` is off (the
        // default — tx history is event-driven), so
        // `total_transactions` reads 0 in production builds. Both
        // behaviors are intentional.
        let funds = account.as_funds();
        Some(AccountMetadataSnapshot {
            // `transactions()` is empty when
            // `keep-finalized-transactions` is off (the default — tx
            // history is event-driven), so `total_transactions` reads
            // 0 in production builds.
            total_transactions: account.transactions().len() as u64,
            total_utxos: funds.map(|a| a.utxos.len() as u64).unwrap_or(0),
            monitor_revision: account.monitor_revision(),
        })
    }

    /// Snapshot of the address pools for one account. Each pool
    /// carries every derived address; pools are returned in the
    /// order [`crate`]: `address_pools()` exposes them, which is
    /// `[external, internal]` for `Standard` and a single pool for
    /// every other variant.
    pub fn account_address_pools_blocking(
        &self,
        wallet_id: &WalletId,
        target: &AccountType,
    ) -> Vec<AccountAddressPoolSnapshot> {
        let wm = self.wallet_manager.blocking_read();
        let Some(info) = wm.get_wallet_info(wallet_id) else {
            return Vec::new();
        };
        let accounts = info.core_wallet.accounts.all_accounts();
        let Some(account) = accounts
            .iter()
            .find(|a| &a.managed_account_type().to_account_type() == target)
        else {
            return Vec::new();
        };
        account
            .managed_account_type()
            .address_pools()
            .iter()
            .map(|pool| pool_snapshot(pool))
            .collect()
    }

    /// Snapshot of every UTXO row on one account.
    pub fn account_utxos_blocking(
        &self,
        wallet_id: &WalletId,
        target: &AccountType,
    ) -> Vec<AccountUtxoSnapshot> {
        let wm = self.wallet_manager.blocking_read();
        let Some(info) = wm.get_wallet_info(wallet_id) else {
            return Vec::new();
        };
        let accounts = info.core_wallet.accounts.all_accounts();
        let Some(account) = accounts
            .iter()
            .find(|a| &a.managed_account_type().to_account_type() == target)
        else {
            return Vec::new();
        };
        // UTXOs only exist on the funds variant. Keys-only accounts
        // (identity / asset-lock / provider) never carry UTXOs by
        // construction, so an empty list is the correct snapshot.
        let Some(funds) = account.as_funds() else {
            return Vec::new();
        };
        funds
            .utxos
            .values()
            .map(|utxo: &Utxo| AccountUtxoSnapshot {
                outpoint: utxo.outpoint,
                value_duffs: utxo.txout.value,
                script_pubkey: utxo.txout.script_pubkey.as_bytes().to_vec(),
                height: utxo.height,
                is_locked: utxo.is_locked,
            })
            .collect()
    }

    // -----------------------------------------------------------------
    // Phase 6 — Per-account transactions
    // -----------------------------------------------------------------

    /// Paginated snapshot of an account's transaction list.
    ///
    /// `page_offset` skips the first `page_offset` records;
    /// `page_limit == 0` means "no limit", any other value caps the
    /// returned slice at `page_limit` rows. Records iterate in
    /// `BTreeMap<Txid, _>` order — deterministic but not
    /// chronological.
    pub fn account_transactions_blocking(
        &self,
        wallet_id: &WalletId,
        target: &AccountType,
        page_offset: usize,
        page_limit: usize,
    ) -> Vec<AccountTransactionSnapshot> {
        let wm = self.wallet_manager.blocking_read();
        let Some(info) = wm.get_wallet_info(wallet_id) else {
            return Vec::new();
        };
        let accounts = info.core_wallet.accounts.all_accounts();
        let Some(account) = accounts
            .iter()
            .find(|a| &a.managed_account_type().to_account_type() == target)
        else {
            return Vec::new();
        };
        // `transactions()` returns an empty map when
        // `keep-finalized-transactions` is disabled — the default. Tx
        // history is delivered through the event channel, not stored
        // in-memory, so a paged readout here is effectively a debug
        // surface for builds that flip the feature on. The snapshot
        // type carries the txid as a field of its own, so we walk
        // values only.
        let iter = account.transactions().values().skip(page_offset);
        let take = if page_limit == 0 {
            usize::MAX
        } else {
            page_limit
        };
        iter.take(take).map(tx_record_snapshot).collect()
    }

    // -----------------------------------------------------------------
    // Phase 7 — Identity manager structure
    // -----------------------------------------------------------------

    /// Snapshot of the wallet's `out_of_wallet_identities` keys
    /// (i.e. observed but un-owned identities the manager tracks).
    /// Reading the per-identity drill-down still goes through the
    /// existing `get_managed_identity` FFI.
    pub fn identity_manager_out_of_wallet_ids_blocking(
        &self,
        wallet_id: &WalletId,
    ) -> Vec<Identifier> {
        let wm = self.wallet_manager.blocking_read();
        let Some(info) = wm.get_wallet_info(wallet_id) else {
            return Vec::new();
        };
        info.identity_manager
            .out_of_wallet_identities
            .keys()
            .copied()
            .collect()
    }

    /// Ordered list of `(registration_index, identity_id)` rows for
    /// a single wallet. `registration_index` is the inner-bucket key,
    /// so the rows come out in BIP-9 index order.
    pub fn identity_manager_wallet_identities_blocking(
        &self,
        wallet_id: &WalletId,
    ) -> Vec<WalletIdentityRowSnapshot> {
        let wm = self.wallet_manager.blocking_read();
        let Some(info) = wm.get_wallet_info(wallet_id) else {
            return Vec::new();
        };
        let Some(inner) = info.identity_manager.wallet_identities.get(wallet_id) else {
            return Vec::new();
        };
        inner
            .iter()
            .map(|(reg_idx, managed)| {
                use dpp::identity::accessors::IdentityGettersV0;
                WalletIdentityRowSnapshot {
                    registration_index: *reg_idx,
                    identity_id: managed.identity.id().to_buffer(),
                }
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Helper conversions used by the snapshot accessors.
// ---------------------------------------------------------------------------

/// Project upstream `AssetLockFundingType` into the diagnostic FFI's
/// stable `lock_type: u8`. Variant order pinned to upstream
/// declaration order.
fn asset_lock_funding_type_to_u8(
    ty: &key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType,
) -> u8 {
    use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
    match ty {
        AssetLockFundingType::IdentityRegistration => 0,
        AssetLockFundingType::IdentityTopUp => 1,
        AssetLockFundingType::IdentityTopUpNotBound => 2,
        AssetLockFundingType::IdentityInvitation => 3,
        AssetLockFundingType::AssetLockAddressTopUp => 4,
        AssetLockFundingType::AssetLockShieldedAddressTopUp => 5,
    }
}

fn pool_snapshot(pool: &AddressPool) -> AccountAddressPoolSnapshot {
    let pool_type: u8 = match pool.pool_type {
        AddressPoolType::External => 0,
        AddressPoolType::Internal => 1,
        AddressPoolType::Absent => 2,
        AddressPoolType::AbsentHardened => 3,
    };
    let last_used_index: i64 = pool.highest_used.map(|i| i as i64).unwrap_or(-1);
    let addresses = pool.addresses.values().map(addr_info_snapshot).collect();
    AccountAddressPoolSnapshot {
        pool_type,
        gap_limit: pool.gap_limit,
        last_used_index,
        addresses,
    }
}

fn addr_info_snapshot(info: &AddressInfo) -> AccountAddressInfoSnapshot {
    // The address pool stores `script_pubkey` directly. P2PKH is the
    // dominant shape here, so pull the 20-byte HASH160 out via
    // `p2pkh_public_key_hash_bytes`. Non-P2PKH script types simply
    // surface zeroed bytes — the diagnostic surface stays a flat
    // `[u8; 20]` either way.
    let mut pubkey_hash = [0u8; 20];
    if let Some(bytes) = info.script_pubkey.p2pkh_public_key_hash_bytes() {
        if bytes.len() == 20 {
            pubkey_hash.copy_from_slice(bytes);
        }
    }
    // Pull the encoded address + raw public-key bytes for the explorer
    // to display. `info.public_key` is `None` on pools that store only
    // the script_pubkey without retaining the derivation source, so an
    // empty `Vec` is the correct shape there.
    let address = info.address.to_string();
    let public_key_bytes = match &info.public_key {
        Some(key_wallet::managed_account::address_pool::PublicKeyType::ECDSA(b))
        | Some(key_wallet::managed_account::address_pool::PublicKeyType::EdDSA(b))
        | Some(key_wallet::managed_account::address_pool::PublicKeyType::BLS(b)) => b.clone(),
        None => Vec::new(),
    };
    AccountAddressInfoSnapshot {
        pubkey_hash,
        address_index: info.index,
        is_used: info.used,
        address,
        public_key_bytes,
    }
}

fn tx_record_snapshot(rec: &TransactionRecord) -> AccountTransactionSnapshot {
    use key_wallet::transaction_checking::TransactionContext;
    let (height, timestamp) = match &rec.context {
        TransactionContext::Mempool | TransactionContext::InstantSend(_) => (0u32, 0u64),
        TransactionContext::InBlock(bi) => (bi.height(), bi.timestamp() as u64),
        TransactionContext::InChainLockedBlock(bi) => (bi.height(), bi.timestamp() as u64),
    };
    AccountTransactionSnapshot {
        txid: rec.txid,
        height,
        timestamp,
        value_delta_duffs: rec.net_amount,
        fee_duffs: rec.fee.unwrap_or(0),
        is_coinbase: rec.transaction.is_coin_base(),
    }
}
