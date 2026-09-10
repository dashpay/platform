//! Read-only accessors on [`PlatformWalletManager`].

use std::sync::Arc;

use dashcore::{OutPoint, Txid};
use dpp::prelude::Identifier;
use key_wallet::account::AccountType;
use key_wallet::managed_account::address_pool::{
    AddressInfo, AddressPool, AddressPoolType, AddressState,
};
use key_wallet::managed_account::transaction_record::TransactionRecord;
use key_wallet::utxo::Utxo;
use key_wallet::WalletCoreBalance;

use crate::changeset::{PersistenceCapabilities, PlatformWalletPersistence};
use crate::manager::dashpay_sync::DashPaySyncManager;
use crate::manager::dpns_sync::DpnsSyncManager;
use crate::manager::identity_sync::IdentitySyncManager;
use crate::manager::platform_address_sync::PlatformAddressSyncManager;
#[cfg(feature = "shielded")]
use crate::manager::shielded_sync::ShieldedSyncManager;
use crate::spv::SpvRuntime;
use crate::wallet::platform_wallet::WalletId;
use crate::wallet::PlatformWallet;

/// Result of [`PlatformWalletManager::provider_masternode_txs_blocking`]:
/// the wallet's network (for base58 address encoding on the FFI side),
/// its retained provider special transactions with their confirmation
/// height and in-block position (for same-block ordering), a DML snapshot
/// (`proTxHash -> is_valid`, `None` when the
/// deterministic masternode list isn't available yet), and two
/// derive-and-compare ownership maps for the key kinds that live ONLY in
/// payloads (never as on-chain addresses):
///
///   * operator BLS public key (48 bytes) ⇒ derivation index, and
///   * platform node id (SHA256[..20] Tenderdash, 20 bytes) ⇒ derivation index.
///
/// Owner / voting keys ARE on-chain addresses, so their ownership is
/// resolved app-side against the persisted `PersistentCoreAddress` rows;
/// operator / platform keys can't be, so they're derived here from the
/// wallet's provider accounts and matched against each masternode's
/// payload key. Operator keys derive from the account xpub with no seed;
/// platform-node keys need the seed (resident wallets only — watch-only
/// yields an empty map, a documented follow-up).
pub type ProviderMasternodeTxs = (
    dashcore::Network,
    // Each tuple is `(block_height, in_block_position, tx)`. The position
    // orders same-block provider updates for the aggregation's latest-wins
    // (Core applies them in `block.vtx` order). Stamped during block
    // processing (rust-dashcore#891); 0 for legacy rows persisted before
    // the field existed.
    Vec<(u32, u32, dashcore::Transaction)>,
    Option<std::collections::HashMap<[u8; 32], bool>>,
    std::collections::HashMap<[u8; 48], u32>,
    std::collections::HashMap<[u8; 20], u32>,
);

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
/// Carries no `is_watch_only` or `custom_name`: upstream's
/// `ManagedCoreFundsAccount` / `ManagedCoreKeysAccount` have neither, and
/// watch-only is a wallet-level property (read off `Wallet.wallet_type`).
/// Add such fields here only if the upstream variants gain them.
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

/// One row of a wallet's UTXO inventory page — see [`wallet_utxos_page`].
///
/// Carries the owning account alongside the coin so a store that keys its
/// rows by account can file a healed row under the right one, and the
/// address the engine derived from the script so the store never has to
/// re-derive it. The flags are the engine's own (`Utxo` fields); a coin the
/// engine holds is by definition unspent from its point of view.
#[derive(Debug, Clone)]
pub struct WalletUtxoRow {
    pub account_type: AccountType,
    pub outpoint: OutPoint,
    pub value_duffs: u64,
    pub script_pubkey: Vec<u8>,
    /// Base58Check address of `script_pubkey`, as the engine holds it.
    pub address: String,
    pub height: u32,
    pub is_confirmed: bool,
    pub is_instantlocked: bool,
    pub is_coinbase: bool,
    pub is_locked: bool,
}

/// Cursor for [`wallet_utxos_page`]: the last row of the previous page.
/// The walk is ordered by `(AccountType, OutPoint)`, so an account is
/// exhausted before the next one starts and a cursor is exact — no row is
/// visited twice or skipped because a concurrent round inserted beside it.
pub type WalletUtxoCursor = (AccountType, OutPoint);

/// Page size [`wallet_utxos_page`] uses when the caller passes 0.
pub const WALLET_UTXO_PAGE_DEFAULT: usize = 512;
/// Largest page [`wallet_utxos_page`] returns — enforced here, not trusted
/// from the caller, because the inventory's size is chain-controlled
/// (anyone who knows a watched address can grow it with dust).
pub const WALLET_UTXO_PAGE_MAX: usize = 4096;

/// One store row the store asks the engine to classify — see
/// [`classify_outpoints`]. `account_type` and `script_pubkey` are the
/// store's own record of who owns the coin, which the verdict checks
/// against the engine's pools rather than trusting.
#[derive(Debug, Clone)]
pub struct OutpointOwnershipQuery {
    pub account_type: AccountType,
    pub outpoint: OutPoint,
    pub script_pubkey: Vec<u8>,
}

/// The engine's answer for one [`OutpointOwnershipQuery`].
///
/// Only [`Self::KnownUncredited`] is positive evidence a reconciler may act
/// on: the owning account recorded the funding transaction (its txid is in
/// the account's records or its finalized set), recognises the output's
/// script as its own, does not hold the coin, AND a funds account holds a
/// MINED record whose transaction spends the outpoint. The last condition
/// is what makes the answer durable. Absence from `utxos` alone is not:
/// `update_utxos` removes the inputs of a mempool spend that may never
/// confirm, and a conflict sweep releases a loser's other inputs without
/// reinserting their coins — both leave the coin absent with its funding
/// known, and both are states the store deliberately keeps restorable.
/// Everything else says nothing: `Unknown` covers those, a funding
/// transaction this session never processed (after a restart the finalized
/// set is empty), and a spender the engine never recorded at all (the
/// rust-dashcore#992 shape, which only the emit-time verdict can name);
/// `NotOwned` a script the account's pools do not monitor, which the engine
/// could never have credited in the first place.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutpointClass {
    /// The engine has no opinion.
    Unknown = 0,
    /// The coin is in a funds account's live `utxos`.
    Unspent = 1,
    /// The owning account knows the funding txid, owns the script, does
    /// not hold the coin, and a mined record spends the outpoint.
    KnownUncredited = 2,
    /// The owning account's pools do not monitor the script.
    NotOwned = 3,
}

impl OutpointClass {
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

/// One page of `wallet_id`'s UTXO inventory across every funds account, in
/// `(AccountType, OutPoint)` order, starting strictly after `after`.
/// Returns the rows and whether more follow. `limit` is clamped to
/// `1..=WALLET_UTXO_PAGE_MAX`, with 0 meaning [`WALLET_UTXO_PAGE_DEFAULT`].
/// An unknown wallet is an empty terminal page.
///
/// A UTXO set that moves between pages (a round landing mid-walk) can drop
/// a row out of ONE walk or repeat one; both are benign for the insert-only,
/// idempotent store reconcile this serves, which re-runs on a cadence.
/// Whether `account_type` is a contact's watch-only chain
/// (`DashpayExternalAccount`): coins there belong to the contact, so the
/// inventory omits them and the classifier has no verdict for them.
pub fn is_watch_only_contact(account_type: &AccountType) -> bool {
    matches!(account_type, AccountType::DashpayExternalAccount { .. })
}

pub fn wallet_utxos_page(
    wm: &key_wallet_manager::WalletManager<crate::wallet::platform_wallet::PlatformWalletInfo>,
    wallet_id: &WalletId,
    after: Option<&WalletUtxoCursor>,
    limit: usize,
) -> (Vec<WalletUtxoRow>, bool) {
    use std::ops::Bound;

    let limit = if limit == 0 {
        WALLET_UTXO_PAGE_DEFAULT
    } else {
        limit.min(WALLET_UTXO_PAGE_MAX)
    };
    let Some(info) = wm.get_wallet_info(wallet_id) else {
        return (Vec::new(), false);
    };
    let mut accounts: Vec<(
        AccountType,
        &key_wallet::managed_account::ManagedCoreFundsAccount,
    )> = info
        .core_wallet
        .accounts
        .all_accounts()
        .iter()
        .filter_map(|a| {
            a.as_funds()
                .map(|funds| (a.managed_account_type().to_account_type(), funds))
        })
        // A contact's watch-only chain is not this wallet's money: its
        // coins never enter the inventory, so no store ever heals them in
        // as the user's. Decided here, not by the store, so a renumbered
        // tag or a new watch-only account type cannot repoint the gate.
        .filter(|(account_type, _)| !is_watch_only_contact(account_type))
        .collect();
    accounts.sort_by(|a, b| a.0.cmp(&b.0));

    let mut rows = Vec::with_capacity(limit);
    let mut has_more = false;
    'accounts: for (account_type, funds) in accounts {
        let start = match after {
            Some((cursor_account, cursor_outpoint)) => match account_type.cmp(cursor_account) {
                std::cmp::Ordering::Less => continue,
                std::cmp::Ordering::Equal => Bound::Excluded(*cursor_outpoint),
                std::cmp::Ordering::Greater => Bound::Unbounded,
            },
            None => Bound::Unbounded,
        };
        for (outpoint, utxo) in funds.utxos.range((start, Bound::Unbounded)) {
            if rows.len() == limit {
                has_more = true;
                break 'accounts;
            }
            rows.push(WalletUtxoRow {
                account_type,
                outpoint: *outpoint,
                value_duffs: utxo.txout.value,
                script_pubkey: utxo.txout.script_pubkey.as_bytes().to_vec(),
                address: utxo.address.to_string(),
                height: utxo.height,
                is_confirmed: utxo.is_confirmed,
                is_instantlocked: utxo.is_instantlocked,
                is_coinbase: utxo.is_coinbase,
                is_locked: utxo.is_locked,
            });
        }
    }
    (rows, has_more)
}

/// Classify each query's outpoint for `wallet_id` — see [`OutpointClass`]
/// for the verdicts and the one a reconciler may act on. Positional:
/// `result[i]` answers `queries[i]`. An unknown wallet answers `Unknown`
/// for every query. Cost is `queries × funds accounts`, never the size of
/// the inventory.
pub fn classify_outpoints(
    wm: &key_wallet_manager::WalletManager<crate::wallet::platform_wallet::PlatformWalletInfo>,
    wallet_id: &WalletId,
    queries: &[OutpointOwnershipQuery],
) -> Vec<OutpointClass> {
    use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;

    let Some(info) = wm.get_wallet_info(wallet_id) else {
        return vec![OutpointClass::Unknown; queries.len()];
    };
    let accounts: Vec<(
        AccountType,
        &key_wallet::managed_account::ManagedCoreFundsAccount,
    )> = info
        .core_wallet
        .accounts
        .all_accounts()
        .iter()
        .filter_map(|a| {
            a.as_funds()
                .map(|funds| (a.managed_account_type().to_account_type(), funds))
        })
        .collect();

    // Durable spend evidence: the inputs of every MINED record in any funds
    // account. A mempool spend, an IS-locked spend or a released loser input
    // leaves a coin absent from `utxos` too, and none of those is a verdict.
    let mined_spends: std::collections::HashSet<OutPoint> = accounts
        .iter()
        .flat_map(|(_, funds)| funds.transactions().values())
        .filter(|record| record.context.block_info().is_some())
        .flat_map(|record| {
            record
                .transaction
                .input
                .iter()
                .map(|input| input.previous_output)
        })
        .collect();
    queries
        .iter()
        .map(|query| {
            // Unspent wins outright: a coin the engine holds is a coin,
            // whichever account the store filed it under.
            if accounts
                .iter()
                .any(|(_, funds)| funds.utxos.contains_key(&query.outpoint))
            {
                return OutpointClass::Unspent;
            }
            // A contact's watch-only chain gets no verdict at all: its
            // coins are the contact's to spend, never this wallet's to flip.
            if is_watch_only_contact(&query.account_type) {
                return OutpointClass::Unknown;
            }
            let Some((_, owner)) = accounts
                .iter()
                .find(|(account_type, _)| *account_type == query.account_type)
            else {
                return OutpointClass::Unknown;
            };
            let script = dashcore::ScriptBuf::from_bytes(query.script_pubkey.clone());
            if !owner.contains_script_pub_key(&script) {
                return OutpointClass::NotOwned;
            }
            let txid = &query.outpoint.txid;
            let funding_known = owner.has_transaction(txid) || owner.transaction_is_finalized(txid);
            if funding_known && mined_spends.contains(&query.outpoint) {
                OutpointClass::KnownUncredited
            } else {
                OutpointClass::Unknown
            }
        })
        .collect()
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
    /// Persistence contracts attested by this manager's configured backend.
    ///
    /// The value is safe to query before wallets are loaded or created and is
    /// immutable for the manager lifetime because the persistence backend is.
    pub fn persistence_capabilities(&self) -> PersistenceCapabilities {
        self.persister.persistence_capabilities()
    }

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
    /// Shared handle to the Platform SDK, for work that outlives a borrow
    /// of the manager (e.g. a locate run on a worker thread).
    pub fn sdk_arc(&self) -> Arc<dash_sdk::Sdk> {
        Arc::clone(&self.sdk)
    }

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

    /// Access the recurring DPNS username-marketplace sync coordinator.
    pub fn dpns_sync(&self) -> &DpnsSyncManager {
        &self.dpns_sync_manager
    }

    /// Clone the `Arc<DpnsSyncManager>` so callers (e.g. FFI) can invoke
    /// [`DpnsSyncManager::start`] which takes `&Arc<Self>`.
    pub fn dpns_sync_arc(&self) -> Arc<DpnsSyncManager> {
        Arc::clone(&self.dpns_sync_manager)
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
    ///
    /// The lookup is wait-free since the map became an `ArcSwap`, so this
    /// suspends at no point; it delegates to the synchronous twin and keeps
    /// its `async` signature for source compatibility with existing callers.
    pub async fn get_wallet(&self, wallet_id: &WalletId) -> Option<Arc<PlatformWallet>> {
        self.get_wallet_blocking(wallet_id)
    }

    /// Synchronous twin of [`Self::get_wallet`] for FFI entry points that
    /// need to clone the `Arc<PlatformWallet>` out before doing network work
    /// outside the handle-storage guard.
    ///
    /// Named `_blocking` for the callers it serves, not for what it does: the
    /// wallets map is an `ArcSwap`, so this load is wait-free and cannot block
    /// or panic inside a runtime the way the previous `blocking_read` could.
    pub fn get_wallet_blocking(&self, wallet_id: &WalletId) -> Option<Arc<PlatformWallet>> {
        self.wallets.load().get(wallet_id).cloned()
    }

    /// List all wallet IDs.
    ///
    /// Wait-free like [`Self::get_wallet`]; delegates to the synchronous
    /// twin and keeps its `async` signature for source compatibility.
    pub async fn wallet_ids(&self) -> Vec<WalletId> {
        self.list_wallet_ids_blocking()
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
        let last_processed_height = info.core_wallet.metadata.last_processed_height;
        info.core_wallet
            .accounts
            .all_accounts()
            .iter()
            .map(|account| {
                // Balance lives on the funds-bearing variant only;
                // keys-only accounts (identity, asset-lock, provider)
                // never carry UTXOs.
                //
                // Computed FRESH from the account's UTXO set — NOT the cached
                // `a.balance` field. The cache refreshes only when transaction
                // processing runs `update_balance()`, and a self-authored
                // asset-lock spend can leave it stale long after the UTXO set
                // (which coin selection reads) has moved on. Deriving from the
                // same source selection uses makes disagreement impossible;
                // the fold is bounded by the account's UTXO count.
                let balance = account
                    .as_funds()
                    .map(|a| computed_core_balance(a, last_processed_height))
                    .unwrap_or_default();
                // Walk every pool on the account, sum
                // `used` + total entries. Cheap — pools are bounded by
                // the gap limit.
                let (keys_used, keys_total) = account
                    .managed_account_type()
                    .address_pools()
                    .iter()
                    .fold((0u32, 0u32), |(used, total), pool| {
                        // "used" counts only funded addresses; a `Reserved`
                        // address is handed out but not yet used.
                        let pool_used = pool
                            .addresses
                            .values()
                            .filter(|info| matches!(info.state, AddressState::Used))
                            .count() as u32;
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
    /// manager. Cheap (wait-free `ArcSwap` load + `BTreeMap` key clone).
    pub fn list_wallet_ids_blocking(&self) -> Vec<WalletId> {
        self.wallets.load().keys().copied().collect()
    }

    /// Network a registered wallet belongs to, or `None` when the id is
    /// unknown.
    ///
    /// Exists for FFI callers that hold a manager handle and a wallet id but
    /// no wallet handle, and need the network before they can build the
    /// per-call key material a wallet operation requires (resolving a master
    /// xpriv, constructing a contact-crypto provider). Blocking and cheap: one
    /// `RwLock` read, no I/O.
    pub fn wallet_network_blocking(&self, wallet_id: &WalletId) -> Option<key_wallet::Network> {
        let wm = self.wallet_manager.blocking_read();
        Some(wm.get_wallet_info(wallet_id)?.core_wallet.network())
    }

    /// Snapshot of [`PlatformAddressSyncManager`] tunables and last-
    /// pass timestamp. `watch_list_size` is `wallets.len()` — every
    /// registered wallet participates in each pass since the sync
    /// manager doesn't keep a separate watch list.
    pub fn platform_address_sync_config_blocking(&self) -> PlatformAddressSyncConfigSnapshot {
        let count = self.wallets.load().len();
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
    /// (which is forward-only and silently ignores a lower value), this calls
    /// the core wallet's unconditional setter only after verifying that
    /// `from_height` is strictly below the current checkpoint. Equal/forward
    /// requests leave the checkpoint untouched; this API can never advance it.
    ///
    /// `synced_height` may regress here: that is safe because it is the
    /// filter-scan checkpoint, decoupled from the monotonic
    /// `last_processed_height`, and every persisted sync cursor is
    /// monotonic-max guarded (see `reconcile_dashpay_rescan`'s note), so a
    /// transient rewind cannot corrupt state or persist a lower cursor.
    ///
    /// The rewound checkpoint lives only in the in-memory `WalletManager`; this
    /// call does not persist it. If the process dies before the rescan finishes,
    /// the host must issue this request again after restart. Requires SPV
    /// running for an immediate effect; otherwise it takes effect when SPV next
    /// starts in the same process and its filter loop first ticks.
    ///
    /// Returns `false` when no wallet matches `wallet_id`.
    pub fn spv_rescan_filters_blocking(&self, wallet_id: &WalletId, from_height: u32) -> bool {
        use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;

        let mut wm = self.wallet_manager.blocking_write();
        let Some(info) = wm.get_wallet_info_mut(wallet_id) else {
            return false;
        };
        let current_height = info.core_wallet.metadata.synced_height;
        if from_height >= current_height {
            tracing::debug!(
                wallet_id = %hex::encode(wallet_id),
                from_height,
                current_height,
                "SPV rescan: ignored non-rewind checkpoint request"
            );
            return true;
        }
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
        let wallet = self.wallets.load().get(wallet_id)?.clone();
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
                    AssetLockStatus::RecoveredFromChain => 5,
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

    /// The shared wallet-manager lock, for callers that must take the
    /// read lock OUTSIDE another guard — an FFI entry point holds the
    /// handle registry's read guard only for the duration of its closure,
    /// and waiting on this lock inside that closure would stall
    /// `platform_wallet_manager_destroy` (a registry write) and, through
    /// parking_lot's writer preference, every other registry reader.
    pub fn wallet_manager_arc(
        &self,
    ) -> Arc<
        tokio::sync::RwLock<
            key_wallet_manager::WalletManager<crate::wallet::platform_wallet::PlatformWalletInfo>,
        >,
    > {
        Arc::clone(&self.wallet_manager)
    }

    /// [`wallet_utxos_page`] under this manager's read lock. Blocking;
    /// call from a thread that may park, never from a runtime worker.
    pub fn wallet_utxos_page_blocking(
        &self,
        wallet_id: &WalletId,
        after: Option<&WalletUtxoCursor>,
        limit: usize,
    ) -> (Vec<WalletUtxoRow>, bool) {
        let wm = self.wallet_manager.blocking_read();
        wallet_utxos_page(&wm, wallet_id, after, limit)
    }

    /// [`classify_outpoints`] under this manager's read lock. Blocking;
    /// call from a thread that may park, never from a runtime worker.
    pub fn classify_outpoints_blocking(
        &self,
        wallet_id: &WalletId,
        queries: &[OutpointOwnershipQuery],
    ) -> Vec<OutpointClass> {
        let wm = self.wallet_manager.blocking_read();
        classify_outpoints(&wm, wallet_id, queries)
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

    /// Provider special transactions (ProRegTx / ProUpServTx / ProUpRegTx
    /// / ProUpRevTx) across all of a wallet's accounts, deduplicated by
    /// txid, each paired with its confirmation height (0 when
    /// unconfirmed). The source for masternode aggregation.
    ///
    /// rust-dashcore #876 retains provider-payload records on the
    /// provider-key accounts (owner / voting / operator / platform) past
    /// chainlock finalization even with `keep-finalized-transactions` off
    /// (the mobile default), so — unlike `account_transactions_blocking`
    /// above — this is populated in every feature configuration. Deduped
    /// by txid because one ProRegTx matches both the owner- and
    /// voting-key accounts, so its record is retained on each.
    ///
    /// Caveat: records evicted *before* the #876 bump aren't resident
    /// until a filter rescan re-matches them, so on an existing install
    /// the set fills in after a Rescan.
    ///
    /// Returns `None` only when the wallet id isn't managed (an empty vec
    /// means "no provider txs yet"). The wallet's `Network` rides along so
    /// the FFI can encode owner / voting key hashes to base58 addresses,
    /// plus a DML snapshot (`proTxHash -> is_valid`, `None` when the list
    /// isn't available yet) so the FFI can derive Active / Inactive /
    /// Retired / Unknown status.
    pub fn provider_masternode_txs_blocking(
        &self,
        wallet_id: &WalletId,
    ) -> Option<ProviderMasternodeTxs> {
        use key_wallet::managed_account::address_pool::PublicKeyType;
        use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
        // Default provider-key pre-derivation / scan window.
        const PROVIDER_KEY_WINDOW: u32 = 20;

        // Scope the wallet-manager read lock so it's released before we
        // acquire the SPV client / engine locks for the DML snapshot — the
        // two never nest. Inside this scope we also read the managed
        // provider pools (`info.core_wallet` is a `ManagedWalletInfo`).
        let (network, txs, operator_scan_max, platform_index) = {
            let wm = self.wallet_manager.blocking_read();
            let info = wm.get_wallet_info(wallet_id)?;
            let network = info.core_wallet.network();

            let mut by_txid: std::collections::BTreeMap<
                dashcore::Txid,
                (u32, u32, dashcore::Transaction),
            > = std::collections::BTreeMap::new();

            for account in info.core_wallet.accounts.all_accounts().iter() {
                for record in account.transactions().values() {
                    if record.transaction.special_transaction_payload.is_none() {
                        continue;
                    }
                    let height = record.context.block_info().map(|b| b.height()).unwrap_or(0);
                    // In-block position for same-height tie-breaking in the
                    // aggregation (Core resolves same-block provider updates in
                    // `block.vtx` order). Stamped by block processing since
                    // rust-dashcore#891 and round-tripped through persistence;
                    // `None` only for legacy rows persisted before the field
                    // existed, which fall back to 0 (feed order).
                    let position = record
                        .context
                        .block_info()
                        .and_then(|b| b.position())
                        .unwrap_or(0);
                    by_txid
                        .entry(record.txid)
                        .or_insert_with(|| (height, position, record.transaction.clone()));
                }
            }

            // How far the operator (BLS) pool extends — its
            // `highest_generated` watermark, which #882 gap-extension and
            // the pool restore can push past the default window — so the
            // seedless derive-and-compare scan below covers beyond-window
            // keys, not a hardcoded 20. Floored at the default window.
            let operator_scan_max = info
                .core_wallet
                .accounts
                .provider_operator_keys
                .as_ref()
                .and_then(|acct| {
                    acct.managed_account_type()
                        .address_pools()
                        .iter()
                        .filter_map(|p| p.highest_generated)
                        .max()
                })
                .map(|h| h.saturating_add(1))
                .unwrap_or(PROVIDER_KEY_WINDOW)
                .max(PROVIDER_KEY_WINDOW);

            // Platform-node ownership: Ed25519/SLIP-10 is hardened-only, so
            // these keys can't be re-derived seedlessly. Read the wallet's
            // own platform-node public keys straight from the managed
            // pool's typed entries (populated at registration and rehydrated
            // from the persisted batch on restore) and recompute the
            // Tenderdash node id (SHA256[..20], rust-dashcore #884) — never
            // trusting a persisted hash160-era id.
            let mut platform_index: std::collections::HashMap<[u8; 20], u32> =
                std::collections::HashMap::new();
            if let Some(acct) = info.core_wallet.accounts.provider_platform_keys.as_ref() {
                for pool in acct.managed_account_type().address_pools() {
                    for entry in pool.addresses.values() {
                        if let Some(PublicKeyType::EdDSA(pk)) = &entry.public_key {
                            if let Ok(pk32) = <[u8; 32]>::try_from(pk.as_slice()) {
                                let node_id =
                                    dashcore::PlatformNodeId::from_ed25519_public_key(&pk32)
                                        .to_byte_array();
                                platform_index.insert(node_id, entry.index);
                            }
                        }
                    }
                }
            }

            (
                network,
                by_txid.into_values().collect::<Vec<_>>(),
                operator_scan_max,
                platform_index,
            )
        };

        let dml = self.spv().masternode_validity_snapshot_blocking();

        // Operator (BLS) ownership: derive both serializations for every
        // index the pool covers (`0..operator_scan_max`). Operator public
        // keys derive from the account xpub with no seed, so this works for
        // resident and restored external-signable wallets alike. A v1
        // ProRegTx carries the key in LEGACY serialization and a v2 in
        // MODERN, so both forms are indexed under the same index (distinct
        // byte strings for the same G1 point — no collision).
        let mut operator_index: std::collections::HashMap<[u8; 48], u32> =
            std::collections::HashMap::new();
        // Clone the `Arc<PlatformWallet>` out of the map snapshot before
        // deriving (the derive calls take the wallet's own state lock).
        let platform_wallet = self.wallets.load().get(wallet_id).cloned();
        if let Some(platform_wallet) = platform_wallet {
            use crate::wallet::provider_key_at_index::ProviderKeyKind;
            for index in 0..operator_scan_max {
                match platform_wallet.derive_provider_key_at_index(
                    ProviderKeyKind::Operator,
                    index,
                    None,
                    false,
                ) {
                    Ok(key) => {
                        if let Ok(bytes) = <[u8; 48]>::try_from(key.public_key_bytes.as_slice()) {
                            operator_index.insert(bytes, index);
                        }
                        if let Some(legacy) = key
                            .legacy_public_key_bytes
                            .as_deref()
                            .and_then(|b| <[u8; 48]>::try_from(b).ok())
                        {
                            operator_index.insert(legacy, index);
                        }
                    }
                    // First failure ⇒ no operator account (or unavailable) ⇒ stop.
                    Err(_) => break,
                }
            }
        }

        Some((network, txs, dml, operator_index, platform_index))
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
        is_used: matches!(info.state, AddressState::Used),
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

#[cfg(test)]
mod spv_rescan_tests {
    use std::sync::Arc;

    use key_wallet::mnemonic::Mnemonic;
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;
    use key_wallet::Network;

    use crate::changeset::{
        ClientStartState, PersistenceError, PlatformWalletChangeSet, PlatformWalletPersistence,
    };
    use crate::events::{EventHandler, PlatformEventHandler};
    use crate::wallet::platform_wallet::WalletId;
    use crate::PlatformWalletManager;

    const TEST_MNEMONIC: &str = "abandon abandon abandon abandon abandon abandon \
         abandon abandon abandon abandon abandon about";

    pub(super) struct NoopPersister;

    impl PlatformWalletPersistence for NoopPersister {
        fn store(
            &self,
            _wallet_id: WalletId,
            _changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            Ok(())
        }

        fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
            Ok(())
        }

        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            Ok(ClientStartState::default())
        }
    }

    pub(super) struct NoopEventHandler;
    impl EventHandler for NoopEventHandler {}
    impl PlatformEventHandler for NoopEventHandler {}

    #[tokio::test]
    async fn spv_rescan_only_rewinds_known_wallets() {
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let event_handler: Arc<dyn PlatformEventHandler> = Arc::new(NoopEventHandler);
        let manager = Arc::new(PlatformWalletManager::new(
            sdk,
            Arc::new(NoopPersister),
            event_handler,
        ));
        let mnemonic = Mnemonic::from_phrase(TEST_MNEMONIC).expect("valid mnemonic");
        let wallet = manager
            .create_wallet_from_seed_bytes(
                Network::Testnet,
                &mnemonic.to_seed(""),
                WalletAccountCreationOptions::Default,
                Some(100),
            )
            .await
            .expect("wallet registration");
        let wallet_id = wallet.wallet_id();

        tokio::task::spawn_blocking(move || {
            let initial_height = manager
                .core_wallet_state_blocking(&wallet_id)
                .expect("known wallet")
                .synced_height;
            let rewound_height = initial_height - 20;
            assert!(manager.spv_rescan_filters_blocking(&wallet_id, rewound_height));
            assert_eq!(
                manager
                    .core_wallet_state_blocking(&wallet_id)
                    .expect("known wallet")
                    .synced_height,
                rewound_height
            );

            // Equal and forward requests are successful no-ops; neither may
            // advance the filter checkpoint.
            assert!(manager.spv_rescan_filters_blocking(&wallet_id, rewound_height));
            assert!(manager.spv_rescan_filters_blocking(&wallet_id, initial_height + 20));
            assert_eq!(
                manager
                    .core_wallet_state_blocking(&wallet_id)
                    .expect("known wallet")
                    .synced_height,
                rewound_height
            );

            assert!(!manager.spv_rescan_filters_blocking(&[0xFF; 32], 40));
        })
        .await
        .expect("blocking accessor task");
    }
}

/// Read-only [`WalletCoreBalance`] over an account's live UTXO set, with the
/// exact bucket rules of `ManagedCoreFundsAccount::update_balance` (which
/// requires `&mut self` and mutates the cache, so it cannot serve a
/// read-path): locked, else immature, else confirmed when in a block /
/// InstantSend-locked / trusted change, else unconfirmed.
fn computed_core_balance(
    account: &key_wallet::managed_account::ManagedCoreFundsAccount,
    last_processed_height: u32,
) -> key_wallet::wallet::balance::WalletCoreBalance {
    let mut confirmed = 0u64;
    let mut unconfirmed = 0u64;
    let mut immature = 0u64;
    let mut locked = 0u64;
    for utxo in account.utxos.values() {
        let value = utxo.txout.value;
        if utxo.is_locked {
            locked += value;
        } else if !utxo.is_mature(last_processed_height) {
            immature += value;
        } else if utxo.is_confirmed || utxo.is_instantlocked || utxo.is_trusted {
            confirmed += value;
        } else {
            unconfirmed += value;
        }
    }
    key_wallet::wallet::balance::WalletCoreBalance::new(confirmed, unconfirmed, immature, locked)
}

#[cfg(test)]
mod computed_balance_tests {
    use super::spv_rescan_tests::{NoopEventHandler, NoopPersister};
    use super::*;
    use key_wallet::account::StandardAccountType;
    use key_wallet_manager::WalletManager;
    use tokio::sync::RwLock;

    use crate::events::PlatformEventHandler;
    use crate::wallet::platform_wallet::PlatformWalletInfo;

    /// Buckets of every account row the accessor returns, folded into one
    /// `(confirmed, unconfirmed, immature, locked)` tuple. Only the funded
    /// account carries UTXOs, so the fold IS that account's figure — and
    /// it stays meaningful once the account is drained to nothing.
    fn folded_buckets(rows: &[AccountBalanceRow]) -> (u64, u64, u64, u64) {
        rows.iter().fold((0, 0, 0, 0), |(c, u, i, l), row| {
            (
                c + row.balance.confirmed(),
                u + row.balance.unconfirmed(),
                i + row.balance.immature(),
                l + row.balance.locked(),
            )
        })
    }

    /// A manager whose wallet-manager IS the funded fixture's, so the
    /// production accessor reads the very account the test mutates.
    /// `account_balances_blocking` takes the manager, not a bare
    /// `WalletManager`, and there is no constructor that adopts one — so
    /// the fixture's value is moved into the freshly built manager's slot.
    async fn manager_over_funded_fixture(
        funded: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    ) -> Arc<PlatformWalletManager<NoopPersister>> {
        let sdk = Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk"));
        let event_handler: Arc<dyn PlatformEventHandler> = Arc::new(NoopEventHandler);
        let manager = Arc::new(PlatformWalletManager::new(
            sdk,
            Arc::new(NoopPersister),
            event_handler,
        ));
        let adopted = std::mem::replace(
            &mut *funded.write().await,
            WalletManager::<PlatformWalletInfo>::new(key_wallet::Network::Testnet),
        );
        *manager.wallet_manager.write().await = adopted;
        manager
    }

    /// `account_balances_blocking` uses `blocking_read`, so it may only be
    /// called off the async runtime's worker.
    async fn account_buckets(
        manager: &Arc<PlatformWalletManager<NoopPersister>>,
        wallet_id: WalletId,
    ) -> (u64, u64, u64, u64) {
        let manager = Arc::clone(manager);
        tokio::task::spawn_blocking(move || {
            folded_buckets(&manager.account_balances_blocking(&wallet_id))
        })
        .await
        .expect("blocking accessor task")
    }

    /// The per-account figure the explorer/FFI reads must come from the
    /// LIVE UTXO set, not the cached `balance` field: a self-authored
    /// asset-lock spend can leave the cache stale long after selection —
    /// which reads the UTXO set — has moved on.
    ///
    /// Driven through `account_balances_blocking`, the accessor production
    /// actually calls, and in three steps because "reports the live truth"
    /// is more than "reports zero": it must first REPRODUCE a freshly
    /// updated non-empty balance bucket for bucket (an implementation
    /// returning `WalletCoreBalance::default()` passes an empty-set-only
    /// test), then track a live re-classification the cache has not seen,
    /// then track removal.
    #[tokio::test]
    async fn account_balances_blocking_ignores_the_stale_cache() {
        let (funded, wallet_id, _balance, _signer) =
            crate::test_support::funded_wallet_manager_with_outputs(
                StandardAccountType::BIP44Account,
                &[7_000_000, 3_000_000],
            )
            .await;
        let manager = manager_over_funded_fixture(funded).await;

        // 1. Agreement on a funded account. The accessor's fold and the
        //    cache are two implementations of the same bucket rules; if
        //    they disagree here, every later assertion is meaningless.
        let cached = {
            let mut wm = manager.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("wallet");
            let height = info.core_wallet.metadata.last_processed_height;
            let account = info
                .core_wallet
                .accounts
                .standard_bip44_accounts
                .get_mut(&0)
                .expect("bip44 account 0");
            account.update_balance(height);
            account.balance
        };
        assert_eq!(cached.total(), 10_000_000, "fixture must be funded");
        assert_eq!(
            account_buckets(&manager, wallet_id).await,
            (
                cached.confirmed(),
                cached.unconfirmed(),
                cached.immature(),
                cached.locked()
            ),
            "the accessor must reproduce a freshly updated non-empty balance, bucket for bucket"
        );

        // 2. Re-classify one UTXO WITHOUT refreshing the cache. The
        //    accessor must move its value confirmed → locked live; a
        //    read of the cached `balance` field cannot.
        let locked_value = {
            let mut wm = manager.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("wallet");
            let account = info
                .core_wallet
                .accounts
                .standard_bip44_accounts
                .get_mut(&0)
                .expect("bip44 account 0");
            let first = *account.utxos.keys().next().expect("funded utxo");
            let utxo = account.utxos.get_mut(&first).expect("funded utxo");
            utxo.is_locked = true;
            assert_eq!(
                account.balance, cached,
                "precondition: the cache must still hold the pre-lock figure"
            );
            utxo.txout.value
        };
        assert_eq!(
            account_buckets(&manager, wallet_id).await,
            (
                cached.confirmed() - locked_value,
                cached.unconfirmed(),
                cached.immature(),
                locked_value
            ),
            "the accessor must see the live lock: value out of confirmed, into locked"
        );

        // 3. Remove every UTXO — the shape an unprocessed self-spend
        //    (the asset-lock drain) leaves behind.
        {
            let mut wm = manager.wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet_id).expect("wallet");
            let account = info
                .core_wallet
                .accounts
                .standard_bip44_accounts
                .get_mut(&0)
                .expect("bip44 account 0");
            account.utxos.clear();
            assert_eq!(
                account.balance.total(),
                cached.total(),
                "precondition: the cache must still hold the stale figure"
            );
        }
        assert_eq!(
            account_buckets(&manager, wallet_id).await,
            (0, 0, 0, 0),
            "the accessor must see the live (empty) UTXO set"
        );
    }

    /// Bucket-policy companion to
    /// [`account_balances_blocking_ignores_the_stale_cache`], asserted
    /// directly on the fold: `computed_core_balance` duplicates
    /// `ManagedCoreFundsAccount::update_balance`'s classification rules,
    /// and nothing in the type system keeps the two in step.
    #[tokio::test]
    async fn computed_core_balance_matches_update_balance_bucket_for_bucket() {
        let (wallet_manager, wallet_id, _balance, _signer) =
            crate::test_support::funded_wallet_manager_with_outputs(
                StandardAccountType::BIP44Account,
                &[7_000_000, 3_000_000],
            )
            .await;

        let mut wm = wallet_manager.write().await;
        let info = wm.get_wallet_info_mut(&wallet_id).expect("wallet");
        let height = info.core_wallet.metadata.last_processed_height;
        let account = info
            .core_wallet
            .accounts
            .standard_bip44_accounts
            .get_mut(&0)
            .expect("bip44 account 0");

        account.update_balance(height);
        let funded = account.balance;
        assert_eq!(funded.total(), 10_000_000, "fixture must be funded");
        assert_eq!(
            computed_core_balance(account, height),
            funded,
            "the fold must reproduce a freshly updated non-empty balance, bucket for bucket"
        );

        let first = *account.utxos.keys().next().expect("funded utxo");
        let locked_value = {
            let utxo = account.utxos.get_mut(&first).expect("funded utxo");
            utxo.is_locked = true;
            utxo.txout.value
        };
        let live = computed_core_balance(account, height);
        assert_eq!(
            live.locked(),
            locked_value,
            "the locked UTXO must be bucketed as locked"
        );
        assert_eq!(
            live.confirmed(),
            funded.confirmed() - locked_value,
            "and must have left the confirmed bucket"
        );
        assert_eq!(
            live.total(),
            funded.total(),
            "locking moves value between buckets, it does not destroy it"
        );

        account.utxos.clear();
        assert_eq!(
            computed_core_balance(account, height).total(),
            0,
            "the fold must see the live (empty) UTXO set"
        );
    }
}

#[cfg(test)]
mod txo_inventory_tests {
    //! Coverage for [`wallet_utxos_page`] and [`classify_outpoints`] — the
    //! two engine reads a store reconcile is built on. Drives a real
    //! `ManagedWalletInfo` through `check_core_transaction` in the exact
    //! arrival orders that produce each classification.

    use std::collections::BTreeMap;
    use std::sync::Arc;

    use dashcore::hashes::Hash;
    use dashcore::{BlockHash, OutPoint, ScriptBuf, Transaction, TxIn, TxOut, Txid, Witness};
    use key_wallet::account::{AccountType, StandardAccountType};
    use key_wallet::test_utils::TestWalletContext;
    use key_wallet::transaction_checking::{BlockInfo, TransactionContext};
    use key_wallet_manager::WalletManager;

    use super::{
        classify_outpoints, wallet_utxos_page, OutpointClass, OutpointOwnershipQuery,
        WALLET_UTXO_PAGE_MAX,
    };
    use crate::wallet::core::WalletGeneration;
    use crate::wallet::identity::IdentityManager;
    use crate::wallet::platform_wallet::{PlatformWalletInfo, WalletId};

    fn bip44_account_0() -> AccountType {
        AccountType::Standard {
            index: 0,
            standard_account_type: StandardAccountType::BIP44Account,
        }
    }

    fn in_block(height: u32) -> TransactionContext {
        TransactionContext::InBlock(BlockInfo::new(
            height,
            BlockHash::from_slice(&[6u8; 32]).expect("valid block hash"),
            1_234_567_890,
        ))
    }

    fn input(previous_output: OutPoint) -> TxIn {
        TxIn {
            previous_output,
            script_sig: ScriptBuf::new(),
            sequence: 0xffffffff,
            witness: Witness::new(),
        }
    }

    fn funding(script_pubkey: ScriptBuf, seed: u8, value: u64) -> Transaction {
        Transaction {
            version: 2,
            lock_time: 0,
            input: vec![input(OutPoint {
                txid: Txid::from_slice(&[seed; 32]).expect("valid txid"),
                vout: 0,
            })],
            output: vec![TxOut {
                value,
                script_pubkey,
            }],
            special_transaction_payload: None,
        }
    }

    /// The rust-dashcore#992 shape: one input, one zero-value `OP_RETURN`.
    fn collateral_burn(coin: OutPoint) -> Transaction {
        Transaction {
            version: 2,
            lock_time: 0,
            input: vec![input(coin)],
            output: vec![TxOut {
                value: 0,
                script_pubkey: dashcore::blockdata::script::Builder::new()
                    .push_opcode(dashcore::opcodes::all::OP_RETURN)
                    .into_script(),
            }],
            special_transaction_payload: None,
        }
    }

    fn foreign_script() -> ScriptBuf {
        const TEST_PUBKEY_G: [u8; 33] = [
            0x02, 0x79, 0xbe, 0x66, 0x7e, 0xf9, 0xdc, 0xbb, 0xac, 0x55, 0xa0, 0x62, 0x95, 0xce,
            0x87, 0x0b, 0x07, 0x02, 0x9b, 0xfc, 0xdb, 0x2d, 0xce, 0x28, 0xd9, 0x59, 0xf2, 0x81,
            0x5b, 0x16, 0xf8, 0x17, 0x98,
        ];
        let pubkey =
            dashcore::PublicKey::from_slice(&TEST_PUBKEY_G).expect("generator point is valid");
        dashcore::Address::p2pkh(&pubkey, key_wallet::Network::Testnet).script_pubkey()
    }

    fn manager_with(ctx: TestWalletContext) -> (WalletManager<PlatformWalletInfo>, WalletId) {
        let info = PlatformWalletInfo {
            core_wallet: ctx.managed_wallet,
            generation: Arc::new(WalletGeneration::new()),
            identity_manager: IdentityManager::new(),
            tracked_asset_locks: BTreeMap::new(),
            dpns_name_states: BTreeMap::new(),
            observed_input_conflicts: Default::default(),
        };
        let mut wm = WalletManager::<PlatformWalletInfo>::new(dashcore::Network::Testnet);
        let wallet_id = wm.insert_wallet(ctx.wallet, info).expect("insert wallet");
        (wm, wallet_id)
    }

    fn query(
        account_type: AccountType,
        outpoint: OutPoint,
        script: &ScriptBuf,
    ) -> OutpointOwnershipQuery {
        OutpointOwnershipQuery {
            account_type,
            outpoint,
            script_pubkey: script.as_bytes().to_vec(),
        }
    }

    #[tokio::test]
    async fn pages_walk_every_coin_once_in_order_and_terminate() {
        let mut ctx = TestWalletContext::new_random();
        let script = ctx.receive_address.script_pubkey();
        let address = ctx.receive_address.to_string();
        let mut coins = Vec::new();
        for (seed, value) in [(11u8, 1_000u64), (12, 2_000), (13, 3_000)] {
            let tx = funding(script.clone(), seed, value);
            assert!(
                ctx.check_transaction(&tx, in_block(100_000 + seed as u32))
                    .await
                    .is_relevant
            );
            coins.push(OutPoint {
                txid: tx.txid(),
                vout: 0,
            });
        }
        let (wm, wallet_id) = manager_with(ctx);

        let mut walked = Vec::new();
        let mut cursor = None;
        let mut pages = 0;
        loop {
            let (rows, has_more) = wallet_utxos_page(&wm, &wallet_id, cursor.as_ref(), 2);
            pages += 1;
            for row in &rows {
                assert_eq!(row.account_type, bip44_account_0());
                assert_eq!(row.address, address);
                assert_eq!(row.script_pubkey, script.as_bytes());
                assert!(row.is_confirmed);
                assert!(row.height >= 100_011);
                walked.push(row.outpoint);
            }
            match rows.last() {
                Some(last) if has_more => cursor = Some((last.account_type, last.outpoint)),
                _ => break,
            }
        }
        assert_eq!(pages, 2, "three coins at two per page");
        let mut expected = coins.clone();
        expected.sort();
        assert_eq!(walked, expected, "every coin once, in outpoint order");

        // Limit 0 means the default page; an oversized limit is clamped.
        let (all, more) = wallet_utxos_page(&wm, &wallet_id, None, 0);
        assert_eq!(all.len(), 3);
        assert!(!more);
        let (all, _) = wallet_utxos_page(&wm, &wallet_id, None, WALLET_UTXO_PAGE_MAX * 4);
        assert_eq!(all.len(), 3);
        // An unknown wallet is an empty terminal page.
        let (none, more) = wallet_utxos_page(&wm, &[0xEEu8; 32], None, 10);
        assert!(none.is_empty());
        assert!(!more);
    }

    /// The four answers, each from the arrival order that produces it.
    /// `KnownUncredited` needs a mined spender on record: a coin funded and
    /// then burned while held. The field case — a collateral burn processed
    /// before its funding is never recorded, the funding is, the coin is
    /// absent — is `Unknown` here: no record spends it, so absence is not
    /// durable evidence (the emit-time verdict covers that shape). A coin
    /// spent only in the mempool is `Unknown` too: `update_utxos` removed
    /// it, but the spend may never confirm and the store keeps it
    /// restorable.
    #[tokio::test]
    async fn classifies_unspent_known_uncredited_not_owned_and_unknown() {
        let mut ctx = TestWalletContext::new_random();
        let script = ctx.receive_address.script_pubkey();

        // A coin the engine holds.
        let held = funding(script.clone(), 21, 5_000);
        assert!(
            ctx.check_transaction(&held, in_block(100_000))
                .await
                .is_relevant
        );
        let held_coin = OutPoint {
            txid: held.txid(),
            vout: 0,
        };

        // The #992 shape: burn first (irrelevant, unrecorded), funding after.
        let burned = funding(script.clone(), 22, 19_549);
        let burned_coin = OutPoint {
            txid: burned.txid(),
            vout: 0,
        };
        assert!(
            !ctx.check_transaction(&collateral_burn(burned_coin), in_block(100_002))
                .await
                .is_relevant
        );
        assert!(
            ctx.check_transaction(&burned, in_block(100_001))
                .await
                .is_relevant
        );

        // A coin spent the ordinary way: funded, then burned while held.
        let spent = funding(script.clone(), 23, 7_000);
        let spent_coin = OutPoint {
            txid: spent.txid(),
            vout: 0,
        };
        assert!(
            ctx.check_transaction(&spent, in_block(100_003))
                .await
                .is_relevant
        );
        assert!(
            ctx.check_transaction(&collateral_burn(spent_coin), in_block(100_004))
                .await
                .is_relevant
        );

        // A coin spent only in the mempool: absent from `utxos`, funding
        // known, spender unconfirmed.
        let mempool_spent = funding(script.clone(), 24, 9_000);
        let mempool_spent_coin = OutPoint {
            txid: mempool_spent.txid(),
            vout: 0,
        };
        assert!(
            ctx.check_transaction(&mempool_spent, in_block(100_005))
                .await
                .is_relevant
        );
        assert!(
            ctx.check_transaction(
                &collateral_burn(mempool_spent_coin),
                key_wallet::transaction_checking::TransactionContext::Mempool
            )
            .await
            .is_relevant
        );
        let (wm, wallet_id) = manager_with(ctx);
        let never_seen = OutPoint {
            txid: Txid::from_slice(&[0x99u8; 32]).expect("valid txid"),
            vout: 0,
        };
        let queries = vec![
            query(bip44_account_0(), held_coin, &script),
            query(bip44_account_0(), burned_coin, &script),
            query(bip44_account_0(), spent_coin, &script),
            query(bip44_account_0(), mempool_spent_coin, &script),
            query(bip44_account_0(), burned_coin, &foreign_script()),
            query(bip44_account_0(), never_seen, &script),
            // The right coin filed under the wrong account: the CoinJoin
            // account exists but its pools never monitored a BIP44 script,
            // so ownership fails before the txid is even consulted.
            query(AccountType::CoinJoin { index: 0 }, burned_coin, &script),
            // A coin filed under an account the wallet does not have at all.
            query(AccountType::CoinJoin { index: 7 }, burned_coin, &script),
        ];
        let classes = classify_outpoints(&wm, &wallet_id, &queries);
        assert_eq!(
            classes,
            vec![
                OutpointClass::Unspent,
                OutpointClass::Unknown,
                OutpointClass::KnownUncredited,
                OutpointClass::Unknown,
                OutpointClass::NotOwned,
                OutpointClass::Unknown,
                OutpointClass::NotOwned,
                OutpointClass::Unknown,
            ]
        );
        // An unknown wallet has no opinion about anything.
        assert_eq!(
            classify_outpoints(&wm, &[0xEEu8; 32], &queries[..2]),
            vec![OutpointClass::Unknown, OutpointClass::Unknown]
        );
        assert!(classify_outpoints(&wm, &wallet_id, &[]).is_empty());
    }
}
