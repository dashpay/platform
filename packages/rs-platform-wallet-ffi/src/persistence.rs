//! FFI callback-based implementation of PlatformWalletPersistence.
//!
//! Changesets are kept in-memory as Rust objects. When specific sub-changeset
//! data is available (e.g., address balances), it is sent across FFI in
//! C-compatible structs so the caller can persist it incrementally (e.g., via
//! SwiftData on iOS).

use bincode::config;
use key_wallet::account::account_collection::AccountCollection;
use key_wallet::account::{Account, AccountType, StandardAccountType};
use key_wallet::bip32::DerivationPath;
use key_wallet::bip32::ExtendedPubKey;
use key_wallet::derivation_bls_bip32::ExtendedBLSPubKey;
use key_wallet::derivation_slip10::ExtendedEd25519PubKey;
use key_wallet::managed_account::address_pool::{AddressPool, AddressPoolType, PublicKeyType};
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;
use key_wallet::AddressInfo;
use parking_lot::RwLock;
use std::str::FromStr;

use crate::types::{FFINetwork, Network};
use platform_wallet::changeset::{
    rebuild_provider_key_account, AccountAddressPoolEntry, AccountRegistrationEntry,
    ClientStartState, ClientWalletStartState, Merge, PersistenceError, PlatformWalletChangeSet,
    PlatformWalletPersistence, ProviderKeyAccountEntry, ProviderKeyExtendedPubKey,
};
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet::wallet::{PerAccountPlatformAddressState, PerWalletPlatformAddressState};
use std::collections::BTreeMap;
use std::ffi::CString;
use std::os::raw::c_void;
use std::slice;

use crate::asset_lock_persistence::{
    build_asset_lock_entries, outpoint_to_bytes, AssetLockEntryFFI,
};
use crate::contact_persistence::{
    free_contact_requests_ffi, ContactIgnoredSenderFFI, ContactRequestFFI, ContactRequestRemovalFFI,
};
use crate::core_address_types::{AddressPoolTypeTagFFI, CoreAddressEntryFFI};
use crate::core_wallet_types::{free_wallet_changeset_ffi, WalletChangeSetFFI};
use crate::identity_persistence::{
    free_identity_entry_ffi, free_identity_key_entry_ffi, IdentityEntryFFI, IdentityKeyEntryFFI,
    IdentityKeyRemovalFFI,
};
use crate::platform_address_types::AddressBalanceEntryFFI;
use crate::token_persistence::{TokenBalanceRemovalFFI, TokenBalanceUpsertFFI};
use crate::wallet_registration_persistence::AccountAddressPoolFFI;
use crate::wallet_restore_types::{
    AccountSpecFFI, AccountTypeTagFFI, ContactProfileRestoreEntryFFI, IdentityKeyRestoreFFI,
    IdentityRestoreEntryFFI, LoadWalletListFreeFn, PaymentRestoreEntryFFI,
    ProviderPlatformNodeKeyFFI, StandardAccountTypeTagFFI, UnresolvedAssetLockTxRecordFFI,
    UtxoRestoreEntryFFI, WalletRestoreEntryFFI,
};
use dpp::address_funds::PlatformAddress;
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::v0::IdentityV0;
use dpp::identity::{Identity, IdentityPublicKey, KeyID, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::BinaryData;
use dpp::prelude::Identifier;
use platform_wallet::{DpnsNameInfo, IdentityManagerStartState, IdentityStatus, ManagedIdentity};
use std::ffi::CStr;

/// C callback vtable for wallet persistence.
///
/// General-purpose notifications (`on_store_fn`, `on_flush_fn`) plus
/// typed callbacks that send incremental data across FFI for the caller
/// to persist in their preferred storage backend.
#[repr(C)]
#[allow(clippy::type_complexity)]
pub struct PersistenceCallbacks {
    /// Opaque context pointer passed to all callbacks.
    pub context: *mut c_void,
    /// Fired once at the top of every [`FFIPersister::store`] call,
    /// before any per-kind sub-callback runs. Clients use this as a
    /// hook to open a transaction / begin a batch / snapshot context
    /// state; paired with `on_changeset_end_fn`. Return value is
    /// advisory — a non-zero result is logged but does NOT abort the
    /// round.
    pub on_changeset_begin_fn:
        Option<unsafe extern "C" fn(context: *mut c_void, wallet_id: *const u8) -> i32>,
    /// Fired once at the bottom of every [`FFIPersister::store`]
    /// call, after every per-kind sub-callback has run. `success`
    /// is `true` iff every per-kind callback returned 0; `false`
    /// otherwise. Clients use this to commit the round's
    /// accumulated writes (success → flush / save) or roll them
    /// back (failure → discard), making each Rust `store()` a
    /// single atomic transaction from the client's point of view.
    ///
    /// Returns 0 on success. A **non-zero** return means the commit
    /// itself failed (e.g. the atomic `save()` threw and the staged
    /// writes were rolled back); `store()` then returns `Err` so the
    /// caller does not advance state against data that never reached
    /// durable storage. (Unlike `on_changeset_begin_fn`, this return
    /// is honored, not advisory.)
    pub on_changeset_end_fn: Option<
        unsafe extern "C" fn(context: *mut c_void, wallet_id: *const u8, success: bool) -> i32,
    >,
    /// Called when a changeset is stored. Returns 0 on success.
    pub on_store_fn:
        Option<unsafe extern "C" fn(context: *mut c_void, wallet_id: *const u8) -> i32>,
    /// Called when flush is requested. Returns 0 on success.
    pub on_flush_fn:
        Option<unsafe extern "C" fn(context: *mut c_void, wallet_id: *const u8) -> i32>,
    /// Called with incremental address balance updates. The entries array
    /// contains only addresses whose balance changed. The pointer is valid
    /// only for the duration of the callback.
    pub on_persist_address_balances_fn: Option<
        unsafe extern "C" fn(
            context: *mut c_void,
            wallet_id: *const u8,
            entries: *const AddressBalanceEntryFFI,
            count: usize,
        ) -> i32,
    >,
    /// Called with core wallet changeset (accounts, transactions, UTXOs,
    /// chain state, balance deltas). The pointer is valid only for the
    /// duration of the callback.
    pub on_persist_wallet_changeset_fn: Option<
        unsafe extern "C" fn(
            context: *mut c_void,
            wallet_id: *const u8,
            changeset: *const WalletChangeSetFFI,
        ) -> i32,
    >,
    /// Called with updated sync state after each sync round.
    /// Allows the caller to persist the watermark so incremental
    /// sync resumes from this point on next app launch.
    pub on_persist_sync_state_fn: Option<
        unsafe extern "C" fn(
            context: *mut c_void,
            wallet_id: *const u8,
            sync_height: u64,
            sync_timestamp: u64,
            last_known_recent_block: u64,
        ) -> i32,
    >,
    /// Called once per registration round with the array of accounts
    /// being persisted. Each entry is the same flat
    /// [`AccountSpecFFI`] shape the load callback returns, so the
    /// receiver matches by `(type_tag, index, registration_index,
    /// key_class, user_identity_id, friend_identity_id, standard_tag)`
    /// and writes one row per spec. The pointer + every nested
    /// `account_xpub_bytes` buffer are Rust-owned and live for the
    /// callback window only — Swift must copy the bytes before the
    /// call returns.
    ///
    /// Returns 0 on success. A non-zero return flips the round's
    /// `success` flag to `false` so [`Self::on_changeset_end_fn`]
    /// receives the rollback signal.
    pub on_persist_account_registrations_fn: Option<
        unsafe extern "C" fn(
            context: *mut c_void,
            wallet_id: *const u8,
            specs: *const AccountSpecFFI,
            count: usize,
        ) -> i32,
    >,
    /// Invoked on [`FFIPersister::load`] to pull the persisted wallet
    /// list back into Rust for external-signable reconstruction.
    /// (The function name still reads "watch-only" in older docs; the
    /// reconstructed `Wallet` is built via
    /// `Wallet::new_external_signable` so the signer surface routes
    /// back to the host's keychain.)
    ///
    /// Implementations must set `*out_entries` to a Swift-allocated
    /// array of `WalletRestoreEntryFFI` and `*out_count` to the
    /// length. The allocation is freed by the caller via
    /// `on_load_wallet_list_free_fn` once Rust has consumed it.
    /// Returns 0 on success, non-zero on failure; on failure Rust
    /// does not call the free callback.
    pub on_load_wallet_list_fn: Option<
        unsafe extern "C" fn(
            context: *mut c_void,
            out_entries: *mut *const WalletRestoreEntryFFI,
            out_count: *mut usize,
        ) -> i32,
    >,
    /// Paired free callback for `on_load_wallet_list_fn`. See
    /// [`LoadWalletListFreeFn`] for the allocation / lifetime contract.
    pub on_load_wallet_list_free_fn: Option<
        unsafe extern "C" fn(
            context: *mut c_void,
            entries: *const WalletRestoreEntryFFI,
            count: usize,
        ),
    >,
    /// Called once per registration round with the wallet's
    /// network tag, network-independent group id + birth height.
    /// `network` uses the same discriminant as
    /// `WalletRestoreEntryFFI.network` (0 = Mainnet, 1 = Testnet,
    /// 2 = Devnet, 3 = Regtest). `wallet_group_id` points to 32
    /// readable bytes (same shape as `wallet_id`) — the
    /// NETWORK-INDEPENDENT id shared by every network's wallet derived
    /// from the same seed, so a consumer can group a seed's
    /// sibling-network rows by it (the per-network `wallet_id` differs
    /// per network for the same seed). For watch-only /
    /// external-signable wallets it equals `wallet_id` (a group of
    /// one). `birth_height` is the best estimate of the block at which
    /// the wallet started; zero means "scan from genesis / unknown".
    ///
    /// Returns 0 on success. A non-zero return flips the round's
    /// `success` flag to `false` so [`Self::on_changeset_end_fn`]
    /// receives the rollback signal.
    pub on_persist_wallet_metadata_fn: Option<
        unsafe extern "C" fn(
            context: *mut c_void,
            wallet_id: *const u8,
            network: FFINetwork,
            wallet_group_id: *const u8,
            birth_height: u32,
        ) -> i32,
    >,
    /// Called once per registration round with the array of address
    /// pool snapshots. Each [`AccountAddressPoolFFI`] entry carries
    /// the owning account spec (matched against the
    /// [`Self::on_persist_account_registrations_fn`] entry that wrote
    /// the row), the pool-type discriminant, and a contiguous slice
    /// of [`CoreAddressEntryFFI`] rows for the pool. All pointers
    /// (the entry array, every nested address slice, every nested
    /// c-string) are Rust-owned and valid only for the callback
    /// window — Swift must copy strings before returning.
    ///
    /// Returns 0 on success. A non-zero return flips the round's
    /// `success` flag to `false` so [`Self::on_changeset_end_fn`]
    /// receives the rollback signal.
    pub on_persist_account_address_pools_fn: Option<
        unsafe extern "C" fn(
            context: *mut c_void,
            wallet_id: *const u8,
            pools: *const AccountAddressPoolFFI,
            count: usize,
        ) -> i32,
    >,
    /// Called with an `IdentityChangeSet` slice — scalar-only
    /// identity upserts (id / balance / revision / status /
    /// wallet_id / identity_index) and identity-id removals. Swift
    /// handlers map upserts onto `PersistentIdentity` rows and
    /// removals onto tombstones.
    pub on_persist_identities_fn: Option<
        unsafe extern "C" fn(
            context: *mut c_void,
            wallet_id: *const u8,
            upserts_ptr: *const IdentityEntryFFI,
            upserts_count: usize,
            removed_ptr: *const [u8; 32],
            removed_count: usize,
        ) -> i32,
    >,
    /// Called with an `IdentityKeysChangeSet` slice — per-key
    /// upserts (public key + optional private-key material) and
    /// `(identity_id, key_id)` removals. Swift handlers map upserts
    /// onto `PersistentPublicKey` rows and removals onto row deletes.
    pub on_persist_identity_keys_fn: Option<
        unsafe extern "C" fn(
            context: *mut c_void,
            wallet_id: *const u8,
            upserts_ptr: *const IdentityKeyEntryFFI,
            upserts_count: usize,
            removed_ptr: *const IdentityKeyRemovalFFI,
            removed_count: usize,
        ) -> i32,
    >,
    /// Called with a `TokenBalanceChangeSet` slice — `(identity_id,
    /// token_id) -> balance` upserts and `(identity_id, token_id)`
    /// tombstones. Swift maps upserts onto `PersistentTokenBalance`
    /// rows keyed by `(tokenId, identityId)` and removes rows for
    /// every tombstone. The watch list itself is no longer
    /// changeset-replicated — it lives in the
    /// [`platform_wallet::IdentitySyncManager`] in-memory cache and
    /// is rehydrated from the SwiftData `PersistentTokenBalance`
    /// rows on app start.
    pub on_persist_token_balances_fn: Option<
        unsafe extern "C" fn(
            context: *mut c_void,
            wallet_id: *const u8,
            upserts_ptr: *const TokenBalanceUpsertFFI,
            upserts_count: usize,
            removed_ptr: *const TokenBalanceRemovalFFI,
            removed_count: usize,
        ) -> i32,
    >,
    /// Called with a flat `ContactChangeSet` projection — sent /
    /// incoming / established contact requests in `upserts`, parallel
    /// sent / incoming removal tombstone arrays, plus an `ignored`
    /// per-sender ignore-delta array keyed `(owner, sender)` (each row's
    /// `is_ignored` bit says persist vs delete — ignore vs un-ignore).
    ///
    /// `ContactChangeSet` is a top-level (not per-identity)
    /// changeset, but the callback is still wallet-scoped via
    /// `wallet_id` so the Swift handler can resolve the network for
    /// the rows it persists.
    ///
    /// The `established` map is projected as **two** rows per entry
    /// (one with `is_outgoing == true`, one with `is_outgoing ==
    /// false`) covering the underlying outgoing+incoming
    /// `ContactRequest` pair on `EstablishedContact`. The auto-
    /// establishment contract on the Rust side drops any matching
    /// pending entries when the contact is established (no separate
    /// tombstone is emitted), so the Swift unique constraint upserts
    /// these rows in place over any prior pending row for the same
    /// `(owner, contact, direction)`.
    pub on_persist_contacts_fn: Option<
        unsafe extern "C" fn(
            context: *mut c_void,
            wallet_id: *const u8,
            upserts_ptr: *const ContactRequestFFI,
            upserts_count: usize,
            removed_sent_ptr: *const ContactRequestRemovalFFI,
            removed_sent_count: usize,
            removed_incoming_ptr: *const ContactRequestRemovalFFI,
            removed_incoming_count: usize,
            ignored_ptr: *const ContactIgnoredSenderFFI,
            ignored_count: usize,
        ) -> i32,
    >,
    // ── Shielded (Orchard) persistence ─────────────────────────────────
    //
    // These four `on_persist_shielded_*` callbacks fire from
    // `FFIPersister::store` whenever a `ShieldedChangeSet` arrives
    // from `ShieldedWallet`. The matching `on_load_shielded_*`
    // callbacks fire once on `FFIPersister::load` to rehydrate the
    // in-memory `SubwalletState`s before the first sync pass. The
    // `wallet_id` carried inside each entry scopes the row by
    // wallet; the outer `wallet_id` argument on the `store`
    // callback identifies the wallet the changeset originated from
    // (always identical to every entry's nested `wallet_id`).
    /// Per-subwallet decrypted notes upserts.
    #[cfg(feature = "shielded")]
    pub on_persist_shielded_notes_fn: Option<
        unsafe extern "C" fn(
            context: *mut c_void,
            wallet_id: *const u8,
            entries: *const crate::shielded_persistence::ShieldedNoteFFI,
            count: usize,
        ) -> i32,
    >,
    /// Per-subwallet nullifier-spent observations.
    #[cfg(feature = "shielded")]
    pub on_persist_shielded_nullifiers_spent_fn: Option<
        unsafe extern "C" fn(
            context: *mut c_void,
            wallet_id: *const u8,
            entries: *const crate::shielded_persistence::ShieldedNullifierSpentFFI,
            count: usize,
        ) -> i32,
    >,
    /// Per-subwallet outgoing (sent) note upserts, recovered via OVK.
    /// Append-only send history keyed by `(wallet_id, account_index,
    /// cmx)`; no spend / nullifier state.
    #[cfg(feature = "shielded")]
    pub on_persist_shielded_outgoing_notes_fn: Option<
        unsafe extern "C" fn(
            context: *mut c_void,
            wallet_id: *const u8,
            entries: *const crate::shielded_persistence::ShieldedOutgoingNoteFFI,
            count: usize,
        ) -> i32,
    >,
    /// Per-subwallet sync watermark advances.
    #[cfg(feature = "shielded")]
    pub on_persist_shielded_synced_indices_fn: Option<
        unsafe extern "C" fn(
            context: *mut c_void,
            wallet_id: *const u8,
            entries: *const crate::shielded_persistence::ShieldedSyncedIndexFFI,
            count: usize,
        ) -> i32,
    >,
    /// Persist a batch of derived activity-log entries. The host upserts
    /// each by `(wallet_id, account_index, entry_id)` — `entry_id` alone
    /// is not globally unique across accounts (see [`ShieldedActivityFFI`]).
    /// Pending→Confirmed/Failed flips and scan-kind refinements re-emit
    /// the same tuple. Mirrors the other `on_persist_shielded_*`
    /// callbacks.
    #[cfg(feature = "shielded")]
    pub on_persist_shielded_activity_fn: Option<
        unsafe extern "C" fn(
            context: *mut c_void,
            wallet_id: *const u8,
            entries: *const crate::shielded_persistence::ShieldedActivityFFI,
            count: usize,
        ) -> i32,
    >,
    /// Restore-on-load: every persisted shielded note. Host
    /// allocates the array; Rust calls the matching free
    /// callback after copying. Same lifetime contract as
    /// `on_load_wallet_list_fn`. Inlined here (rather than via
    /// the `OnLoadShieldedNotesFn` type alias) so cbindgen sees
    /// the full signature and emits the referenced struct
    /// definitions in the generated header.
    #[cfg(feature = "shielded")]
    pub on_load_shielded_notes_fn: Option<
        unsafe extern "C" fn(
            context: *mut c_void,
            out_entries: *mut *const crate::shielded_persistence::ShieldedNoteRestoreFFI,
            out_count: *mut usize,
        ) -> i32,
    >,
    #[cfg(feature = "shielded")]
    pub on_load_shielded_notes_free_fn: Option<
        unsafe extern "C" fn(
            context: *mut c_void,
            entries: *const crate::shielded_persistence::ShieldedNoteRestoreFFI,
            count: usize,
        ),
    >,
    /// Restore-on-load: every persisted outgoing (sent) note. Same
    /// host-allocates / Rust-frees lifetime contract as
    /// `on_load_shielded_notes_fn`. Inlined (rather than via a type
    /// alias) so cbindgen emits the referenced struct in the header.
    #[cfg(feature = "shielded")]
    pub on_load_shielded_outgoing_notes_fn: Option<
        unsafe extern "C" fn(
            context: *mut c_void,
            out_entries: *mut *const crate::shielded_persistence::ShieldedOutgoingNoteRestoreFFI,
            out_count: *mut usize,
        ) -> i32,
    >,
    #[cfg(feature = "shielded")]
    pub on_load_shielded_outgoing_notes_free_fn: Option<
        unsafe extern "C" fn(
            context: *mut c_void,
            entries: *const crate::shielded_persistence::ShieldedOutgoingNoteRestoreFFI,
            count: usize,
        ),
    >,
    /// Restore-on-load: every per-subwallet sync state.
    #[cfg(feature = "shielded")]
    pub on_load_shielded_sync_states_fn: Option<
        unsafe extern "C" fn(
            context: *mut c_void,
            out_entries: *mut *const crate::shielded_persistence::ShieldedSubwalletSyncStateFFI,
            out_count: *mut usize,
        ) -> i32,
    >,
    #[cfg(feature = "shielded")]
    pub on_load_shielded_sync_states_free_fn: Option<
        unsafe extern "C" fn(
            context: *mut c_void,
            entries: *const crate::shielded_persistence::ShieldedSubwalletSyncStateFFI,
            count: usize,
        ),
    >,
    /// Restore-on-load: every persisted activity-log entry. Same
    /// host-allocates / Rust-frees lifetime contract as
    /// `on_load_shielded_notes_fn`. Inlined so cbindgen emits the
    /// referenced struct in the header.
    #[cfg(feature = "shielded")]
    pub on_load_shielded_activity_fn: Option<
        unsafe extern "C" fn(
            context: *mut c_void,
            out_entries: *mut *const crate::shielded_persistence::ShieldedActivityRestoreFFI,
            out_count: *mut usize,
        ) -> i32,
    >,
    #[cfg(feature = "shielded")]
    pub on_load_shielded_activity_free_fn: Option<
        unsafe extern "C" fn(
            context: *mut c_void,
            entries: *const crate::shielded_persistence::ShieldedActivityRestoreFFI,
            count: usize,
        ),
    >,
    /// Look up a single core transaction record by `txid` for the
    /// asset-lock proof flow's persister fallback.
    ///
    /// With upstream's `keep-finalized-transactions` Cargo feature OFF
    /// (the default), chain-locked records are evicted from the
    /// in-memory `transactions()` map and only their txids retained in
    /// `finalized_txids` for dedup. The asset-lock proof flow needs to
    /// recover the chain-lock height to construct a
    /// `ChainAssetLockProof`; the persister has the record (it
    /// received it on the chain-lock-transition `store` call before
    /// eviction) and answers this lookup.
    ///
    /// Output contract:
    /// - Set `*out_found = true` when a row exists for `txid`. Set
    ///   `*out_context_kind` to the row's actual context (0=Mempool,
    ///   1=InstantSend, 2=InBlock, 3=InChainLockedBlock). For
    ///   context kinds 2 and 3, populate `out_block_height`,
    ///   `out_block_hash` (32 bytes), and `out_block_timestamp` from
    ///   the row's block info; the Rust side ignores those fields
    ///   for kinds 0 and 1.
    /// - Hand back the row's raw transaction bytes via
    ///   `*out_tx_bytes` + `*out_tx_bytes_len`. The buffer is
    ///   caller-allocated and must remain valid until the Rust side
    ///   invokes [`Self::on_get_core_tx_record_free_fn`]. Set
    ///   `*out_tx_bytes = null` + `*out_tx_bytes_len = 0` if the
    ///   row exists but the persister never stored the bytes (the
    ///   Rust side will surface `None` rather than synthesize a
    ///   placeholder).
    /// - Set `*out_found = false` when no row exists for `txid`.
    /// - Return `0` on a successful lookup (whether found or not).
    ///   Non-zero values are treated as a transient backend failure
    ///   by the Rust side and surfaced as `None` (no error
    ///   propagation through the proof flow).
    ///
    /// The Rust side faithfully reconstructs the
    /// [`TransactionContext`](key_wallet::transaction_checking::TransactionContext)
    /// from `*out_context_kind` and decodes the tx bytes into a real
    /// [`dashcore::Transaction`]. InstantSend rows are reported back
    /// with the kind tag but not currently consumed (the persister
    /// doesn't store the IS-lock blob), so for an IS hit the Rust
    /// side surfaces `None` to the proof flow — same outcome as a
    /// miss. The proof flow then falls through to its existing
    /// SPV-event-driven wait path, which is what would have happened
    /// without the fallback at all.
    pub on_get_core_tx_record_fn: Option<
        unsafe extern "C" fn(
            context: *mut c_void,
            wallet_id: *const u8,
            txid: *const u8,
            out_context_kind: *mut u8,
            out_block_height: *mut u32,
            out_block_hash: *mut u8,
            out_block_timestamp: *mut u32,
            out_tx_bytes: *mut *const u8,
            out_tx_bytes_len: *mut usize,
            out_found: *mut bool,
        ) -> i32,
    >,
    /// Paired free callback for the tx-bytes buffer returned by
    /// [`Self::on_get_core_tx_record_fn`]. The Rust side invokes
    /// this with the same `(tx_bytes, tx_bytes_len)` pair the lookup
    /// callback wrote into the output pointers, exactly once per
    /// hit. Implementations should release the buffer (e.g.
    /// `UnsafeMutablePointer<UInt8>.deallocate()` on the Swift
    /// side).
    pub on_get_core_tx_record_free_fn: Option<
        unsafe extern "C" fn(context: *mut c_void, tx_bytes: *const u8, tx_bytes_len: usize),
    >,
    /// Called with an `AssetLockChangeSet` slice — upserts on the
    /// tracked-asset-lock store and outpoint tombstones. Swift maps
    /// upserts onto `PersistentAssetLock` rows keyed by the 36-byte
    /// outpoint (`txid || vout_le`) and deletes rows for every
    /// removal. The `transaction_bytes` / `proof_bytes` slices inside
    /// each [`AssetLockEntryFFI`] are Rust-owned and valid only for
    /// the callback window — Swift must copy them before returning.
    ///
    /// Returns 0 on success. A non-zero return flips the round's
    /// `success` flag to `false` so [`Self::on_changeset_end_fn`]
    /// receives the rollback signal.
    pub on_persist_asset_locks_fn: Option<
        unsafe extern "C" fn(
            context: *mut c_void,
            wallet_id: *const u8,
            upserts_ptr: *const AssetLockEntryFFI,
            upserts_count: usize,
            removed_ptr: *const [u8; 36],
            removed_count: usize,
        ) -> i32,
    >,
}

// SAFETY: The context pointer is managed by the FFI caller who must ensure
// thread safety.
unsafe impl Send for PersistenceCallbacks {}
unsafe impl Sync for PersistenceCallbacks {}

impl Default for PersistenceCallbacks {
    fn default() -> Self {
        Self {
            context: std::ptr::null_mut(),
            on_changeset_begin_fn: None,
            on_changeset_end_fn: None,
            on_store_fn: None,
            on_flush_fn: None,
            on_persist_address_balances_fn: None,
            on_persist_wallet_changeset_fn: None,
            on_persist_asset_locks_fn: None,
            on_persist_sync_state_fn: None,
            on_persist_account_registrations_fn: None,
            on_load_wallet_list_fn: None,
            on_load_wallet_list_free_fn: None,
            on_persist_wallet_metadata_fn: None,
            on_persist_account_address_pools_fn: None,
            on_persist_identities_fn: None,
            on_persist_identity_keys_fn: None,
            on_persist_token_balances_fn: None,
            on_persist_contacts_fn: None,
            on_get_core_tx_record_fn: None,
            on_get_core_tx_record_free_fn: None,
            #[cfg(feature = "shielded")]
            on_persist_shielded_notes_fn: None,
            #[cfg(feature = "shielded")]
            on_persist_shielded_nullifiers_spent_fn: None,
            #[cfg(feature = "shielded")]
            on_persist_shielded_outgoing_notes_fn: None,
            #[cfg(feature = "shielded")]
            on_persist_shielded_synced_indices_fn: None,
            #[cfg(feature = "shielded")]
            on_persist_shielded_activity_fn: None,
            #[cfg(feature = "shielded")]
            on_load_shielded_notes_fn: None,
            #[cfg(feature = "shielded")]
            on_load_shielded_notes_free_fn: None,
            #[cfg(feature = "shielded")]
            on_load_shielded_outgoing_notes_fn: None,
            #[cfg(feature = "shielded")]
            on_load_shielded_outgoing_notes_free_fn: None,
            #[cfg(feature = "shielded")]
            on_load_shielded_sync_states_fn: None,
            #[cfg(feature = "shielded")]
            on_load_shielded_sync_states_free_fn: None,
            #[cfg(feature = "shielded")]
            on_load_shielded_activity_fn: None,
            #[cfg(feature = "shielded")]
            on_load_shielded_activity_free_fn: None,
        }
    }
}

/// In-memory persister that accumulates changesets and notifies via callbacks.
pub struct FFIPersister {
    callbacks: PersistenceCallbacks,
    pending: RwLock<BTreeMap<WalletId, PlatformWalletChangeSet>>,
}

impl FFIPersister {
    pub fn new(callbacks: PersistenceCallbacks) -> Self {
        Self {
            callbacks,
            pending: RwLock::new(BTreeMap::new()),
        }
    }
}

impl PlatformWalletPersistence for FFIPersister {
    fn store(
        &self,
        wallet_id: WalletId,
        changeset: PlatformWalletChangeSet,
    ) -> Result<(), PersistenceError> {
        // Bracket the whole per-kind callback sequence with a
        // begin/end pair so clients (Swift, etc.) can treat the
        // round as a single atomic transaction: begin opens a
        // batch, each per-kind callback mutates staged state, and
        // end either commits (`success == true`) or rolls back
        // (`success == false`). `success` flips to false on any
        // sub-callback returning non-zero; the result is advisory
        // for clients (Swift uses it to decide save vs rollback).
        if let Some(cb) = self.callbacks.on_changeset_begin_fn {
            let result = unsafe { cb(self.callbacks.context, wallet_id.as_ptr()) };
            if result != 0 {
                eprintln!("Changeset-begin callback returned error code {}", result);
            }
        }
        let mut round_success = true;

        // Wallet-registration metadata. Fires at most once per round
        // (registration emits the entry; subsequent rounds carry
        // `wallet_metadata: None` so no callback fires).
        if let Some(meta) = changeset.wallet_metadata.as_ref() {
            if let Some(cb) = self.callbacks.on_persist_wallet_metadata_fn {
                let result = unsafe {
                    cb(
                        self.callbacks.context,
                        wallet_id.as_ptr(),
                        meta.network.into(),
                        meta.wallet_group_id.as_ptr(),
                        meta.birth_height,
                    )
                };
                if result != 0 {
                    eprintln!(
                        "Wallet metadata persistence callback returned error code {}",
                        result
                    );
                    round_success = false;
                }
            }
        }

        // Per-account registration entries. The `_xpub_bytes_storage`
        // Vec keeps the bincoded xpub buffers alive for the callback
        // window — `AccountSpecFFI.account_xpub_bytes` borrows into
        // it. Same lifetime discipline the prior dedicated callback
        // used.
        if !changeset.account_registrations.is_empty()
            || !changeset.provider_key_account_registrations.is_empty()
        {
            if let Some(cb) = self.callbacks.on_persist_account_registrations_fn {
                match build_account_specs_for_callback(
                    &changeset.account_registrations,
                    &changeset.provider_key_account_registrations,
                ) {
                    Ok((specs, _xpub_bytes_storage, _derived_keys_storage)) => {
                        let result = unsafe {
                            cb(
                                self.callbacks.context,
                                wallet_id.as_ptr(),
                                specs.as_ptr(),
                                specs.len(),
                            )
                        };
                        // Force the spec / byte buffers / derived-key
                        // buffers to live until after the callback even
                        // though their drop happens on scope exit anyway.
                        drop(specs);
                        drop(_xpub_bytes_storage);
                        drop(_derived_keys_storage);
                        if result != 0 {
                            eprintln!(
                                "Account registrations persistence callback returned error code {}",
                                result
                            );
                            round_success = false;
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to encode account registration specs: {}", e);
                        round_success = false;
                    }
                }
            }
        }

        // Per-account address-pool snapshots. The `_string_storage`
        // Vec keeps every owned `CString` alive for the callback
        // window; `_address_storage` keeps every per-pool
        // `Vec<CoreAddressEntryFFI>` alive (each pool holds pointers
        // into a sibling string buffer); `_pools` is the heap-array
        // the callback iterates over.
        if !changeset.account_address_pools.is_empty() {
            if let Some(cb) = self.callbacks.on_persist_account_address_pools_fn {
                match build_address_pools_for_callback(&changeset.account_address_pools) {
                    Ok((pools, _address_storage, _string_storage)) => {
                        let result = unsafe {
                            cb(
                                self.callbacks.context,
                                wallet_id.as_ptr(),
                                pools.as_ptr(),
                                pools.len(),
                            )
                        };
                        drop(pools);
                        drop(_address_storage);
                        drop(_string_storage);
                        if result != 0 {
                            eprintln!(
                                "Account address pools persistence callback returned error code {}",
                                result
                            );
                            round_success = false;
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to encode account address pool entries: {}", e);
                        round_success = false;
                    }
                }
            }
        }

        // Send incremental address balance updates before merging.
        if let Some(ref addr_cs) = changeset.platform_addresses {
            if let Some(cb) = self.callbacks.on_persist_address_balances_fn {
                let entries: Vec<AddressBalanceEntryFFI> = addr_cs
                    .addresses
                    .iter()
                    .map(|entry| AddressBalanceEntryFFI {
                        address: entry.address.into(),
                        balance: entry.funds.balance,
                        nonce: entry.funds.nonce,
                        account_index: entry.account_index,
                        address_index: entry.address_index,
                        as_of_height: entry.funds.as_of_height,
                    })
                    .collect();
                if !entries.is_empty() {
                    let result = unsafe {
                        cb(
                            self.callbacks.context,
                            wallet_id.as_ptr(),
                            entries.as_ptr(),
                            entries.len(),
                        )
                    };
                    if result != 0 {
                        eprintln!(
                            "Address balance persistence callback returned error code {}",
                            result
                        );
                        round_success = false;
                    }
                }
            }
        }

        // Send core wallet changeset (accounts, transactions, UTXOs).
        if let Some(ref core_cs) = changeset.core {
            // Fan out gap-limit-extension addresses BEFORE the wallet
            // changeset itself: the changeset's UTXOs reference these
            // addresses, and the Swift-side `upsertUtxo`'s
            // `coreAddress` link lookup is keyed on the address row
            // existing. Emitting the pool snapshot first means a
            // brand-new TXO landing on a freshly-derived address
            // finds the matching `PersistentCoreAddress` in the same
            // changeset round and the cascade-delete chain stays
            // intact. Reuses `on_persist_account_address_pools_fn`
            // so the Swift side handles both registration-time emit
            // and event-time emit through `persistAccountAddresses`
            // — single Swift code path covers both.
            if !core_cs.addresses_derived.is_empty() {
                if let Some(cb) = self.callbacks.on_persist_account_address_pools_fn {
                    match build_address_pools_from_derived(&core_cs.addresses_derived) {
                        Ok((pools, _address_storage, _string_storage)) => {
                            let result = unsafe {
                                cb(
                                    self.callbacks.context,
                                    wallet_id.as_ptr(),
                                    pools.as_ptr(),
                                    pools.len(),
                                )
                            };
                            drop(pools);
                            drop(_address_storage);
                            drop(_string_storage);
                            if result != 0 {
                                eprintln!(
                                    "Derived-address persistence callback returned error code {}",
                                    result
                                );
                                round_success = false;
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to encode derived address pool entries: {}", e);
                            round_success = false;
                        }
                    }
                }
            }

            // Fan out used-flag flips AFTER the derived-address emit:
            // a tx can land on a freshly-derived address in the same
            // round, and the Swift-side `persistAccountAddresses`
            // overwrites `isUsed` with whatever the latest emit says —
            // derived-first (`is_used: false`) then marked-used
            // (`is_used: true`) leaves the row correctly flipped.
            // These entries carry the authoritative post-mark
            // `AddressInfo` from the wallet's pools (see
            // `CoreChangeSet::addresses_marked_used`), so reusing the
            // whole-pool snapshot encoder is exact, not approximate.
            if !core_cs.addresses_marked_used.is_empty() {
                if let Some(cb) = self.callbacks.on_persist_account_address_pools_fn {
                    let entries =
                        group_marked_used_into_pool_entries(&core_cs.addresses_marked_used);
                    match build_address_pools_for_callback(&entries) {
                        Ok((pools, _address_storage, _string_storage)) => {
                            let result = unsafe {
                                cb(
                                    self.callbacks.context,
                                    wallet_id.as_ptr(),
                                    pools.as_ptr(),
                                    pools.len(),
                                )
                            };
                            drop(pools);
                            drop(_address_storage);
                            drop(_string_storage);
                            if result != 0 {
                                eprintln!(
                                    "Marked-used address persistence callback returned error code {}",
                                    result
                                );
                                round_success = false;
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to encode marked-used address pool entries: {}", e);
                            round_success = false;
                        }
                    }
                }
            }

            if let Some(cb) = self.callbacks.on_persist_wallet_changeset_fn {
                let ffi_cs = WalletChangeSetFFI::from_changeset(core_cs);
                let result = unsafe { cb(self.callbacks.context, wallet_id.as_ptr(), &ffi_cs) };
                unsafe { free_wallet_changeset_ffi(&ffi_cs) };
                if result != 0 {
                    eprintln!(
                        "Wallet changeset persistence callback returned error code {}",
                        result
                    );
                    round_success = false;
                }
            }
        }

        // Send identity scalar changeset — upserts and removals.
        // Swift handler maps these onto `PersistentIdentity` row
        // upserts / tombstones.
        if let Some(ref id_cs) = changeset.identities {
            if let Some(cb) = self.callbacks.on_persist_identities_fn {
                // Build heap-allocated mirrors. Scoped so the Vec
                // drops only after the free-loop runs even if the
                // callback invocation panics.
                let mut upserts: Vec<IdentityEntryFFI> = id_cs
                    .identities
                    .values()
                    .map(IdentityEntryFFI::from_entry)
                    .collect();
                let removed: Vec<[u8; 32]> =
                    id_cs.removed.iter().map(|id| id.to_buffer()).collect();
                let result = unsafe {
                    cb(
                        self.callbacks.context,
                        wallet_id.as_ptr(),
                        upserts.as_ptr(),
                        upserts.len(),
                        if removed.is_empty() {
                            std::ptr::null()
                        } else {
                            removed.as_ptr()
                        },
                        removed.len(),
                    )
                };
                // Release every heap-allocated field on the FFI
                // entries before the Vec drops its storage.
                for entry in upserts.iter_mut() {
                    unsafe { free_identity_entry_ffi(entry) };
                }
                if result != 0 {
                    eprintln!(
                        "Identity changeset persistence callback returned error code {}",
                        result
                    );
                    round_success = false;
                }
            }
        }

        // Send identity-keys changeset — per-key upserts +
        // `(identity_id, key_id)` removals. Maps onto Swift's
        // `PersistentPublicKey` rows.
        if let Some(ref keys_cs) = changeset.identity_keys {
            if let Some(cb) = self.callbacks.on_persist_identity_keys_fn {
                let mut upserts: Vec<IdentityKeyEntryFFI> = keys_cs
                    .upserts
                    .values()
                    .map(IdentityKeyEntryFFI::from_entry)
                    .collect();
                let removed: Vec<IdentityKeyRemovalFFI> = keys_cs
                    .removed
                    .iter()
                    .map(|(id, key_id)| IdentityKeyRemovalFFI {
                        identity_id: id.to_buffer(),
                        key_id: *key_id,
                    })
                    .collect();
                let result = unsafe {
                    cb(
                        self.callbacks.context,
                        wallet_id.as_ptr(),
                        upserts.as_ptr(),
                        upserts.len(),
                        if removed.is_empty() {
                            std::ptr::null()
                        } else {
                            removed.as_ptr()
                        },
                        removed.len(),
                    )
                };
                for entry in upserts.iter_mut() {
                    unsafe { free_identity_key_entry_ffi(entry) };
                }
                if result != 0 {
                    eprintln!(
                        "Identity keys changeset persistence callback returned error code {}",
                        result
                    );
                    round_success = false;
                }
            }
        }

        // Send token-balance changeset — `(identity_id, token_id) ->
        // balance` upserts and `(identity_id, token_id)` tombstones.
        // Maps onto Swift's `PersistentTokenBalance` rows.
        if let Some(ref tb_cs) = changeset.token_balances {
            if let Some(cb) = self.callbacks.on_persist_token_balances_fn {
                let upserts: Vec<TokenBalanceUpsertFFI> = tb_cs
                    .balances
                    .iter()
                    .map(|((iid, tid), amount)| TokenBalanceUpsertFFI {
                        identity_id: iid.to_buffer(),
                        token_id: tid.to_buffer(),
                        balance: *amount,
                    })
                    .collect();
                let removals: Vec<TokenBalanceRemovalFFI> = tb_cs
                    .removed_balances
                    .iter()
                    .map(|(iid, tid)| TokenBalanceRemovalFFI {
                        identity_id: iid.to_buffer(),
                        token_id: tid.to_buffer(),
                    })
                    .collect();
                if !upserts.is_empty() || !removals.is_empty() {
                    let result = unsafe {
                        cb(
                            self.callbacks.context,
                            wallet_id.as_ptr(),
                            if upserts.is_empty() {
                                std::ptr::null()
                            } else {
                                upserts.as_ptr()
                            },
                            upserts.len(),
                            if removals.is_empty() {
                                std::ptr::null()
                            } else {
                                removals.as_ptr()
                            },
                            removals.len(),
                        )
                    };
                    if result != 0 {
                        eprintln!(
                            "Token balance persistence callback returned error code {}",
                            result
                        );
                        round_success = false;
                    }
                }
            }
        }

        // Send asset-lock changeset — tracked-lock upserts (one row
        // per credit output, addressed by outpoint) and outpoint
        // tombstones (consumed-by-registration removals). Maps onto
        // Swift's `PersistentAssetLock` rows.
        if let Some(ref al_cs) = changeset.asset_locks {
            if let Some(cb) = self.callbacks.on_persist_asset_locks_fn {
                let upsert_refs: Vec<&platform_wallet::changeset::AssetLockEntry> =
                    al_cs.asset_locks.values().collect();
                let (upserts, _storage) = build_asset_lock_entries(&upsert_refs);
                let removed: Vec<[u8; 36]> = al_cs.removed.iter().map(outpoint_to_bytes).collect();
                if !upserts.is_empty() || !removed.is_empty() {
                    let result = unsafe {
                        cb(
                            self.callbacks.context,
                            wallet_id.as_ptr(),
                            if upserts.is_empty() {
                                std::ptr::null()
                            } else {
                                upserts.as_ptr()
                            },
                            upserts.len(),
                            if removed.is_empty() {
                                std::ptr::null()
                            } else {
                                removed.as_ptr()
                            },
                            removed.len(),
                        )
                    };
                    // Pin both byte-buffer storage (`_storage`) and
                    // the FFI Vec until after the callback so the
                    // pointers stay valid through the C call.
                    drop(upserts);
                    drop(_storage);
                    if result != 0 {
                        eprintln!(
                            "Asset lock persistence callback returned error code {}",
                            result
                        );
                        round_success = false;
                    }
                }
            }
        }

        // Send DashPay contact-request changeset.
        //
        // The flat upsert array is built by walking every source
        // bucket on the changeset:
        //   - `sent_requests`     ⇒ one outgoing row per entry
        //   - `incoming_requests` ⇒ one incoming row per entry
        //   - `established`       ⇒ two rows per entry (the underlying
        //     outgoing + incoming `ContactRequest` on
        //     `EstablishedContact`) so the Swift uniqueness key
        //     `(network, owner, contact, is_outgoing)` upserts both
        //     directions cleanly. The auto-establishment contract on
        //     the Rust side drops any matching `sent_requests` /
        //     `incoming_requests` entry when promoting to established,
        //     so this projection never produces a duplicate row in a
        //     single round.
        //
        // Removal arrays mirror the changeset's two tombstone fields
        // 1:1 — Swift deletes rows by `(owner, contact, is_outgoing)`
        // with the direction implied by which bucket they came from.
        if let Some(ref contacts_cs) = changeset.contacts {
            if let Some(cb) = self.callbacks.on_persist_contacts_fn {
                let mut upserts: Vec<ContactRequestFFI> = Vec::with_capacity(
                    contacts_cs.sent_requests.len()
                        + contacts_cs.incoming_requests.len()
                        + contacts_cs.established.len() * 2,
                );
                for (key, entry) in &contacts_cs.sent_requests {
                    upserts.push(ContactRequestFFI::from_outgoing(
                        key.owner_id.to_buffer(),
                        key.recipient_id.to_buffer(),
                        &entry.request,
                    ));
                }
                for (key, entry) in &contacts_cs.incoming_requests {
                    upserts.push(ContactRequestFFI::from_incoming(
                        key.owner_id.to_buffer(),
                        key.sender_id.to_buffer(),
                        &entry.request,
                    ));
                }
                for (key, established) in &contacts_cs.established {
                    // Replicate the relationship's broken-channel flag
                    // and owner-private metadata (alias/note/hidden —
                    // contactInfo, M3) onto BOTH the outgoing and
                    // incoming row — they are properties of the
                    // established pair, not of one direction, so the
                    // Swift handler persists them on each
                    // `(owner, contact, is_outgoing)` row.
                    upserts.push(ContactRequestFFI::from_established_outgoing(
                        key.owner_id.to_buffer(),
                        key.recipient_id.to_buffer(),
                        &established.outgoing_request,
                        established.payment_channel_broken,
                        established.alias.as_deref(),
                        established.note.as_deref(),
                        established.is_hidden,
                        &established.accepted_accounts,
                    ));
                    upserts.push(ContactRequestFFI::from_established_incoming(
                        key.owner_id.to_buffer(),
                        key.recipient_id.to_buffer(),
                        &established.incoming_request,
                        established.payment_channel_broken,
                        established.alias.as_deref(),
                        established.note.as_deref(),
                        established.is_hidden,
                        // Direction-specific: the contact's account label
                        // rides only the incoming row.
                        established.contact_account_label.as_deref(),
                        &established.accepted_accounts,
                    ));
                }
                let removed_sent: Vec<ContactRequestRemovalFFI> = contacts_cs
                    .removed_sent
                    .iter()
                    .map(|key| ContactRequestRemovalFFI {
                        owner_id: key.owner_id.to_buffer(),
                        contact_id: key.recipient_id.to_buffer(),
                    })
                    .collect();
                let removed_incoming: Vec<ContactRequestRemovalFFI> = contacts_cs
                    .removed_incoming
                    .iter()
                    .map(|key| ContactRequestRemovalFFI {
                        owner_id: key.owner_id.to_buffer(),
                        contact_id: key.sender_id.to_buffer(),
                    })
                    .collect();
                // Per-sender ignore deltas, keyed `(owner, sender)`. The
                // `ignored` set projects to rows with `is_ignored == true`
                // (persist the ignored-sender row); the `unignored` set to
                // rows with `is_ignored == false` (delete it). Both ride a
                // single array so the host applies a mixed delta in one
                // callback.
                let ignored: Vec<ContactIgnoredSenderFFI> =
                    contacts_cs
                        .ignored
                        .iter()
                        .map(|(owner, sender)| ContactIgnoredSenderFFI::new(owner, sender, true))
                        .chain(contacts_cs.unignored.iter().map(|(owner, sender)| {
                            ContactIgnoredSenderFFI::new(owner, sender, false)
                        }))
                        .collect();
                if !upserts.is_empty()
                    || !removed_sent.is_empty()
                    || !removed_incoming.is_empty()
                    || !ignored.is_empty()
                {
                    let result = unsafe {
                        cb(
                            self.callbacks.context,
                            wallet_id.as_ptr(),
                            if upserts.is_empty() {
                                std::ptr::null()
                            } else {
                                upserts.as_ptr()
                            },
                            upserts.len(),
                            if removed_sent.is_empty() {
                                std::ptr::null()
                            } else {
                                removed_sent.as_ptr()
                            },
                            removed_sent.len(),
                            if removed_incoming.is_empty() {
                                std::ptr::null()
                            } else {
                                removed_incoming.as_ptr()
                            },
                            removed_incoming.len(),
                            if ignored.is_empty() {
                                std::ptr::null()
                            } else {
                                ignored.as_ptr()
                            },
                            ignored.len(),
                        )
                    };
                    // Release every heap-allocated payload before the
                    // outer Vec drops its storage.
                    if !upserts.is_empty() {
                        unsafe { free_contact_requests_ffi(upserts.as_mut_ptr(), upserts.len()) };
                    }
                    if result != 0 {
                        eprintln!(
                            "Contact persistence callback returned error code {}",
                            result
                        );
                        round_success = false;
                    }
                }
            }
        }

        // Send sync state updates.
        if let Some(ref addr_cs) = changeset.platform_addresses {
            if let Some(cb) = self.callbacks.on_persist_sync_state_fn {
                let height = addr_cs.sync_height.unwrap_or(0);
                let timestamp = addr_cs.sync_timestamp.unwrap_or(0);
                let recent = addr_cs.last_known_recent_block.unwrap_or(0);
                if height > 0 || timestamp > 0 || recent > 0 {
                    let result = unsafe {
                        cb(
                            self.callbacks.context,
                            wallet_id.as_ptr(),
                            height,
                            timestamp,
                            recent,
                        )
                    };
                    if result != 0 {
                        eprintln!(
                            "Sync state persistence callback returned error code {}",
                            result
                        );
                        round_success = false;
                    }
                }
            }
        }

        // Shielded changeset (Orchard): four flat callback batches
        // mirroring the four `ShieldedChangeSet` fields. Notes
        // first so a follow-up `mark_spent` for the same nullifier
        // upserts onto an existing row instead of falling on
        // missing-row floor.
        #[cfg(feature = "shielded")]
        if let Some(ref shielded_cs) = changeset.shielded {
            use crate::shielded_persistence::*;

            // 1) notes_saved
            if !shielded_cs.notes_saved.is_empty() {
                if let Some(cb) = self.callbacks.on_persist_shielded_notes_fn {
                    // Flatten the per-subwallet map into a single
                    // contiguous Vec so the callback gets one
                    // `entries: *const ShieldedNoteFFI` slice. The
                    // host copies `note_data` bytes during the call.
                    let entries: Vec<ShieldedNoteFFI> = shielded_cs
                        .notes_saved
                        .iter()
                        .flat_map(|(id, notes)| {
                            notes.iter().map(|n| ShieldedNoteFFI {
                                wallet_id: id.wallet_id,
                                account_index: id.account_index,
                                position: n.position,
                                cmx: n.cmx,
                                nullifier: n.nullifier,
                                block_height: n.block_height,
                                is_spent: u8::from(n.is_spent),
                                value: n.value,
                                note_data_ptr: n.note_data.as_ptr(),
                                note_data_len: n.note_data.len(),
                            })
                        })
                        .collect();
                    let result = unsafe {
                        cb(
                            self.callbacks.context,
                            wallet_id.as_ptr(),
                            entries.as_ptr(),
                            entries.len(),
                        )
                    };
                    if result != 0 {
                        eprintln!(
                            "Shielded notes persistence callback returned error code {}",
                            result
                        );
                        round_success = false;
                    }
                }
            }

            // 2) nullifiers_spent
            if !shielded_cs.nullifiers_spent.is_empty() {
                if let Some(cb) = self.callbacks.on_persist_shielded_nullifiers_spent_fn {
                    let entries: Vec<ShieldedNullifierSpentFFI> = shielded_cs
                        .nullifiers_spent
                        .iter()
                        .flat_map(|(id, nfs)| {
                            nfs.iter().map(|nf| ShieldedNullifierSpentFFI {
                                wallet_id: id.wallet_id,
                                account_index: id.account_index,
                                nullifier: *nf,
                            })
                        })
                        .collect();
                    let result = unsafe {
                        cb(
                            self.callbacks.context,
                            wallet_id.as_ptr(),
                            entries.as_ptr(),
                            entries.len(),
                        )
                    };
                    if result != 0 {
                        eprintln!(
                            "Shielded nullifier-spent persistence callback returned error code {}",
                            result
                        );
                        round_success = false;
                    }
                }
            }

            // 3) outgoing_notes (OVK-recovered send history). Each
            //    `memo_ptr` borrows into the changeset's owned `memo`
            //    Vec, which lives for the whole `store()` call, so the
            //    pointer stays valid for the callback window (same
            //    discipline as `note_data_ptr` above).
            if !shielded_cs.outgoing_notes.is_empty() {
                if let Some(cb) = self.callbacks.on_persist_shielded_outgoing_notes_fn {
                    let entries: Vec<ShieldedOutgoingNoteFFI> = shielded_cs
                        .outgoing_notes
                        .iter()
                        .flat_map(|(id, notes)| {
                            notes.iter().filter_map(|n| {
                                // `recipient` is a 43-byte raw Orchard address
                                // stored as a `Vec` (serde-derive only covers
                                // arrays <= 32). It is always exactly 43 bytes
                                // from OVK recovery; reject (skip + warn) a
                                // malformed row rather than silently zero-padding
                                // it into a wrong address.
                                let recipient: [u8; 43] = match n.recipient.as_slice().try_into() {
                                    Ok(r) => r,
                                    Err(_) => {
                                        tracing::warn!(
                                            recipient_len = n.recipient.len(),
                                            "skipping outgoing-note persist row: \
                                                 recipient is not the expected 43 bytes"
                                        );
                                        return None;
                                    }
                                };
                                Some(ShieldedOutgoingNoteFFI {
                                    wallet_id: id.wallet_id,
                                    account_index: id.account_index,
                                    cmx: n.cmx,
                                    recipient,
                                    value: n.value,
                                    block_height: n.block_height,
                                    memo_ptr: n.memo.as_ptr(),
                                    memo_len: n.memo.len(),
                                })
                            })
                        })
                        .collect();
                    let result = unsafe {
                        cb(
                            self.callbacks.context,
                            wallet_id.as_ptr(),
                            entries.as_ptr(),
                            entries.len(),
                        )
                    };
                    if result != 0 {
                        eprintln!(
                            "Shielded outgoing-notes persistence callback returned error code {}",
                            result
                        );
                        round_success = false;
                    }
                }
            }

            // 4) synced_indices
            if !shielded_cs.synced_indices.is_empty() {
                if let Some(cb) = self.callbacks.on_persist_shielded_synced_indices_fn {
                    let entries: Vec<ShieldedSyncedIndexFFI> = shielded_cs
                        .synced_indices
                        .iter()
                        .map(|(id, &idx)| ShieldedSyncedIndexFFI {
                            wallet_id: id.wallet_id,
                            account_index: id.account_index,
                            last_synced_index: idx,
                        })
                        .collect();
                    let result = unsafe {
                        cb(
                            self.callbacks.context,
                            wallet_id.as_ptr(),
                            entries.as_ptr(),
                            entries.len(),
                        )
                    };
                    if result != 0 {
                        eprintln!(
                            "Shielded synced-index persistence callback returned error code {}",
                            result
                        );
                        round_success = false;
                    }
                }
            }

            // 5) activity entries (derived activity log). The variable-
            //    length fields (counterparty / memo / cmx + nullifier
            //    arrays) borrow into `backing`, a Vec of owned byte
            //    buffers that outlives the callback — same pointer-validity
            //    discipline as `note_data_ptr` / `memo_ptr` above. The
            //    host upserts by `(wallet_id, account_index, entry_id)`.
            if !shielded_cs.activity_entries.is_empty() {
                if let Some(cb) = self.callbacks.on_persist_shielded_activity_fn {
                    // One pass pairs each entry with its owned cmx /
                    // nullifier buffers STRUCTURALLY (same tuple), so the
                    // pointer-into-backing invariant can't silently
                    // mis-pair if either side is ever filtered or
                    // reordered. The buffers live in `rows` (immutable
                    // once built) for the whole callback window; an inner
                    // `Vec<u8>`'s heap allocation is stable even as the
                    // outer Vec grows.
                    let rows: Vec<(
                        &platform_wallet::wallet::shielded::SubwalletId,
                        &platform_wallet::wallet::shielded::ShieldedActivityEntry,
                        Vec<u8>,
                        Vec<u8>,
                    )> = shielded_cs
                        .activity_entries
                        .iter()
                        .flat_map(|(id, entries)| entries.iter().map(move |e| (id, e)))
                        .map(|(id, e)| {
                            let mut cmx_buf = Vec::with_capacity(e.note_cmxs.len() * 32);
                            for c in &e.note_cmxs {
                                cmx_buf.extend_from_slice(c);
                            }
                            let mut nf_buf = Vec::with_capacity(e.spent_nullifiers.len() * 32);
                            for n in &e.spent_nullifiers {
                                nf_buf.extend_from_slice(n);
                            }
                            (id, e, cmx_buf, nf_buf)
                        })
                        .collect();
                    let entries: Vec<ShieldedActivityFFI> = rows
                        .iter()
                        .map(|(id, e, cmx_buf, nf_buf)| {
                            let (identity_id, has_identity_id) = match &e.kind {
                                platform_wallet::wallet::shielded::ShieldedActivityKind::IdentityCreate {
                                    identity_id,
                                } => (*identity_id, 1u8),
                                _ => ([0u8; 32], 0u8),
                            };
                            let (counterparty_ptr, counterparty_len) = match &e.counterparty {
                                Some(c) if !c.is_empty() => (c.as_ptr(), c.len()),
                                _ => (std::ptr::null(), 0),
                            };
                            let (memo_ptr, memo_len) = match &e.memo {
                                Some(m) if !m.is_empty() => (m.as_ptr(), m.len()),
                                _ => (std::ptr::null(), 0),
                            };
                            ShieldedActivityFFI {
                                wallet_id: id.wallet_id,
                                account_index: id.account_index,
                                entry_id: e.id,
                                kind_tag: e.kind.tag(),
                                direction: activity_direction_tag(&e.direction),
                                status: activity_status_tag(&e.status),
                                amount: e.amount,
                                fee: e.fee.unwrap_or(0),
                                has_fee: u8::from(e.fee.is_some()),
                                block_height: e.block_height.unwrap_or(0),
                                has_block_height: u8::from(e.block_height.is_some()),
                                created_at_ms: e.created_at_ms,
                                identity_id,
                                has_identity_id,
                                counterparty_ptr,
                                counterparty_len,
                                memo_ptr,
                                memo_len,
                                // Match the documented "valid or null"
                                // contract (and the counterparty/memo
                                // siblings): an empty Vec's `as_ptr()` is
                                // a dangling non-null sentinel, so emit a
                                // real null when there's nothing to point
                                // at.
                                note_cmxs_ptr: if cmx_buf.is_empty() {
                                    std::ptr::null()
                                } else {
                                    cmx_buf.as_ptr()
                                },
                                note_cmxs_count: cmx_buf.len() / 32,
                                spent_nullifiers_ptr: if nf_buf.is_empty() {
                                    std::ptr::null()
                                } else {
                                    nf_buf.as_ptr()
                                },
                                spent_nullifiers_count: nf_buf.len() / 32,
                            }
                        })
                        .collect();
                    let result = unsafe {
                        cb(
                            self.callbacks.context,
                            wallet_id.as_ptr(),
                            entries.as_ptr(),
                            entries.len(),
                        )
                    };
                    if result != 0 {
                        eprintln!(
                            "Shielded activity persistence callback returned error code {}",
                            result
                        );
                        round_success = false;
                    }
                    // `rows` and `entries` drop here, after the callback
                    // has copied everything it needs.
                    drop(rows);
                }
            }
        }

        // Close the round. Clients use this to commit (if
        // `round_success == true`) or roll back (otherwise) the
        // staged writes accumulated across the per-kind callbacks
        // above, making the whole store() call a single atomic
        // transaction from their perspective.
        if let Some(cb) = self.callbacks.on_changeset_end_fn {
            let result = unsafe { cb(self.callbacks.context, wallet_id.as_ptr(), round_success) };
            if result != 0 {
                eprintln!("Changeset-end callback returned error code {}", result);
                // The end callback is where the client COMMITS the round (e.g.
                // the SwiftData atomic `save()`). A non-zero return means the
                // commit failed and the staged writes were rolled back — the
                // round never reached durable storage. Treat it as a
                // persistence failure so `store()` returns `Err` and the caller
                // does NOT advance / clear its in-memory state (pending queues,
                // cleared drain entries, ignored-sender deltas) against data
                // that was dropped. Otherwise the failure is silent and the
                // dropped writes resurface or are lost with no signal.
                round_success = false;
            }
        }

        if !round_success {
            return Err(PersistenceError::backend(
                "one or more persistence callbacks failed; changeset was rolled back",
            ));
        }

        // Merge into pending changesets. No secret rides the changeset any
        // more — the client derives identity keys on demand from the Keychain
        // seed at the breadcrumb path, so nothing here needs scrubbing.
        let mut pending = self.pending.write();
        pending
            .entry(wallet_id)
            .and_modify(|existing| existing.merge(changeset.clone()))
            .or_insert(changeset);

        // Notify caller.
        if let Some(cb) = self.callbacks.on_store_fn {
            let result = unsafe { cb(self.callbacks.context, wallet_id.as_ptr()) };
            if result != 0 {
                return Err(PersistenceError::backend(format!(
                    "Persistence store callback returned error code {}",
                    result
                )));
            }
        }

        Ok(())
    }

    fn flush(&self, wallet_id: WalletId) -> Result<(), PersistenceError> {
        // TODO: deferred — FFI callback failures are classified as
        // `Fatal` (no transient-retry signal across the C ABI), and
        // trailing-byte validation on decoded FFI payloads is not yet
        // applied here. Both are tracked for a follow-up; no behavior
        // change in this change.
        // Notify caller.
        if let Some(cb) = self.callbacks.on_flush_fn {
            let result = unsafe { cb(self.callbacks.context, wallet_id.as_ptr()) };
            if result != 0 {
                return Err(PersistenceError::backend(format!(
                    "Persistence flush callback returned error code {}",
                    result
                )));
            }
        }

        // Clear pending after successful flush notification.
        let mut pending = self.pending.write();
        pending.remove(&wallet_id);

        Ok(())
    }

    fn load(&self) -> Result<ClientStartState, PersistenceError> {
        // If Swift hasn't wired up `on_load_wallet_list_fn` there's
        // nothing to restore — treat as a fresh client.
        let Some(load_cb) = self.callbacks.on_load_wallet_list_fn else {
            return Ok(ClientStartState::default());
        };

        // Swift allocates the entries array (and every nested byte
        // buffer); we read it under a `Guard` that calls the matching
        // free callback on drop, so failures inside the loop don't
        // leak.
        let mut entries_ptr: *const WalletRestoreEntryFFI = std::ptr::null();
        let mut count: usize = 0;
        let rc = unsafe { load_cb(self.callbacks.context, &mut entries_ptr, &mut count) };
        if rc != 0 {
            return Err(PersistenceError::backend(format!(
                "on_load_wallet_list_fn returned error code {}",
                rc
            )));
        }
        let _guard = LoadGuard {
            context: self.callbacks.context,
            free_fn: self.callbacks.on_load_wallet_list_free_fn,
            entries: entries_ptr,
            count,
        };

        let mut out = ClientStartState::default();
        if entries_ptr.is_null() || count == 0 {
            return Ok(out);
        }

        // SAFETY: Swift guarantees `entries_ptr` points to `count`
        // contiguous `WalletRestoreEntryFFI` values for the lifetime
        // of the callback window. `_guard` ensures the free callback
        // fires before we leave this function.
        let entries = unsafe { slice::from_raw_parts(entries_ptr, count) };
        for entry in entries {
            let (wallet_state, platform_address_state) = build_wallet_start_state(entry)?;
            out.wallets.insert(entry.wallet_id, wallet_state);
            if let Some(platform_address_state) = platform_address_state {
                out.platform_addresses
                    .insert(entry.wallet_id, platform_address_state);
            }
        }

        // Restore shielded sub-wallet state if the host has wired
        // up the optional callbacks. Notes and per-subwallet sync
        // states travel separately so the host can populate them
        // from independent SwiftData fetch descriptors. Both arms
        // walk the same `(wallet_id, account_index)` key space and
        // funnel into a single `SubwalletId` map on
        // `ClientStartState.shielded`.
        #[cfg(feature = "shielded")]
        {
            use crate::shielded_persistence::*;
            use platform_wallet::changeset::{ShieldedSubwalletStartState, ShieldedSyncStartState};
            use platform_wallet::wallet::shielded::{
                ShieldedActivityEntry, ShieldedActivityKind, ShieldedActivityStatus,
                ShieldedDirection, ShieldedNote, ShieldedOutgoingNote, SubwalletId,
            };

            let mut shielded_state = ShieldedSyncStartState::default();

            // Fail fast on a half-wired callback pair: a loader without
            // its matching free callback leaks the host-allocated buffer
            // on every successful load (the guard's `Drop` is a no-op
            // when `free_fn` is `None`).
            if self.callbacks.on_load_shielded_notes_fn.is_some()
                != self.callbacks.on_load_shielded_notes_free_fn.is_some()
            {
                return Err(PersistenceError::backend(
                    "on_load_shielded_notes_fn and on_load_shielded_notes_free_fn must be \
                     provided together",
                ));
            }
            if self.callbacks.on_load_shielded_sync_states_fn.is_some()
                != self
                    .callbacks
                    .on_load_shielded_sync_states_free_fn
                    .is_some()
            {
                return Err(PersistenceError::backend(
                    "on_load_shielded_sync_states_fn and on_load_shielded_sync_states_free_fn \
                     must be provided together",
                ));
            }
            if self.callbacks.on_load_shielded_outgoing_notes_fn.is_some()
                != self
                    .callbacks
                    .on_load_shielded_outgoing_notes_free_fn
                    .is_some()
            {
                return Err(PersistenceError::backend(
                    "on_load_shielded_outgoing_notes_fn and \
                     on_load_shielded_outgoing_notes_free_fn must be provided together",
                ));
            }

            // 1) notes
            if let Some(load_notes) = self.callbacks.on_load_shielded_notes_fn {
                let mut notes_ptr: *const ShieldedNoteRestoreFFI = std::ptr::null();
                let mut notes_count: usize = 0;
                let rc =
                    unsafe { load_notes(self.callbacks.context, &mut notes_ptr, &mut notes_count) };
                if rc != 0 {
                    return Err(PersistenceError::backend(format!(
                        "on_load_shielded_notes_fn returned error code {}",
                        rc
                    )));
                }
                struct NotesGuard {
                    context: *mut c_void,
                    free_fn: Option<
                        unsafe extern "C" fn(
                            context: *mut c_void,
                            entries: *const ShieldedNoteRestoreFFI,
                            count: usize,
                        ),
                    >,
                    entries: *const ShieldedNoteRestoreFFI,
                    count: usize,
                }
                impl Drop for NotesGuard {
                    fn drop(&mut self) {
                        if let Some(free_fn) = self.free_fn {
                            unsafe { free_fn(self.context, self.entries, self.count) };
                        }
                    }
                }
                let _notes_guard = NotesGuard {
                    context: self.callbacks.context,
                    free_fn: self.callbacks.on_load_shielded_notes_free_fn,
                    entries: notes_ptr,
                    count: notes_count,
                };
                if !notes_ptr.is_null() && notes_count > 0 {
                    let slice = unsafe { slice::from_raw_parts(notes_ptr, notes_count) };
                    for ffi in slice {
                        if ffi.note_data_ptr.is_null() || ffi.note_data_len == 0 {
                            continue;
                        }
                        let note_data = unsafe {
                            std::slice::from_raw_parts(ffi.note_data_ptr, ffi.note_data_len)
                                .to_vec()
                        };
                        let id = SubwalletId::new(ffi.wallet_id, ffi.account_index);
                        let entry = shielded_state
                            .per_subwallet
                            .entry(id)
                            .or_insert_with(ShieldedSubwalletStartState::default);
                        entry.notes.push(ShieldedNote {
                            position: ffi.position,
                            cmx: ffi.cmx,
                            nullifier: ffi.nullifier,
                            block_height: ffi.block_height,
                            is_spent: ffi.is_spent != 0,
                            value: ffi.value,
                            note_data,
                        });
                    }
                }
            }

            // 2) outgoing (sent) notes recovered via OVK
            if let Some(load_outgoing) = self.callbacks.on_load_shielded_outgoing_notes_fn {
                let mut out_ptr: *const ShieldedOutgoingNoteRestoreFFI = std::ptr::null();
                let mut out_count: usize = 0;
                let rc =
                    unsafe { load_outgoing(self.callbacks.context, &mut out_ptr, &mut out_count) };
                if rc != 0 {
                    return Err(PersistenceError::backend(format!(
                        "on_load_shielded_outgoing_notes_fn returned error code {}",
                        rc
                    )));
                }
                struct OutgoingGuard {
                    context: *mut c_void,
                    free_fn: Option<
                        unsafe extern "C" fn(
                            context: *mut c_void,
                            entries: *const ShieldedOutgoingNoteRestoreFFI,
                            count: usize,
                        ),
                    >,
                    entries: *const ShieldedOutgoingNoteRestoreFFI,
                    count: usize,
                }
                impl Drop for OutgoingGuard {
                    fn drop(&mut self) {
                        if let Some(free_fn) = self.free_fn {
                            unsafe { free_fn(self.context, self.entries, self.count) };
                        }
                    }
                }
                let _outgoing_guard = OutgoingGuard {
                    context: self.callbacks.context,
                    free_fn: self.callbacks.on_load_shielded_outgoing_notes_free_fn,
                    entries: out_ptr,
                    count: out_count,
                };
                if !out_ptr.is_null() && out_count > 0 {
                    let slice = unsafe { slice::from_raw_parts(out_ptr, out_count) };
                    for ffi in slice {
                        let memo = if ffi.memo_ptr.is_null() || ffi.memo_len == 0 {
                            Vec::new()
                        } else {
                            unsafe {
                                std::slice::from_raw_parts(ffi.memo_ptr, ffi.memo_len).to_vec()
                            }
                        };
                        let id = SubwalletId::new(ffi.wallet_id, ffi.account_index);
                        let entry = shielded_state
                            .per_subwallet
                            .entry(id)
                            .or_insert_with(ShieldedSubwalletStartState::default);
                        entry.outgoing_notes.push(ShieldedOutgoingNote {
                            cmx: ffi.cmx,
                            recipient: ffi.recipient.to_vec(),
                            value: ffi.value,
                            memo,
                            block_height: ffi.block_height,
                        });
                    }
                }
            }

            // 3) per-subwallet sync states
            if let Some(load_states) = self.callbacks.on_load_shielded_sync_states_fn {
                let mut states_ptr: *const ShieldedSubwalletSyncStateFFI = std::ptr::null();
                let mut states_count: usize = 0;
                let rc = unsafe {
                    load_states(self.callbacks.context, &mut states_ptr, &mut states_count)
                };
                if rc != 0 {
                    return Err(PersistenceError::backend(format!(
                        "on_load_shielded_sync_states_fn returned error code {}",
                        rc
                    )));
                }
                struct StatesGuard {
                    context: *mut c_void,
                    free_fn: Option<
                        unsafe extern "C" fn(
                            context: *mut c_void,
                            entries: *const ShieldedSubwalletSyncStateFFI,
                            count: usize,
                        ),
                    >,
                    entries: *const ShieldedSubwalletSyncStateFFI,
                    count: usize,
                }
                impl Drop for StatesGuard {
                    fn drop(&mut self) {
                        if let Some(free_fn) = self.free_fn {
                            unsafe { free_fn(self.context, self.entries, self.count) };
                        }
                    }
                }
                let _states_guard = StatesGuard {
                    context: self.callbacks.context,
                    free_fn: self.callbacks.on_load_shielded_sync_states_free_fn,
                    entries: states_ptr,
                    count: states_count,
                };
                if !states_ptr.is_null() && states_count > 0 {
                    let slice = unsafe { slice::from_raw_parts(states_ptr, states_count) };
                    for ffi in slice {
                        let id = SubwalletId::new(ffi.wallet_id, ffi.account_index);
                        let entry = shielded_state
                            .per_subwallet
                            .entry(id)
                            .or_insert_with(ShieldedSubwalletStartState::default);
                        entry.last_synced_index = ffi.last_synced_index;
                    }
                }
            }

            // 4) derived activity entries
            if self.callbacks.on_load_shielded_activity_fn.is_some()
                != self.callbacks.on_load_shielded_activity_free_fn.is_some()
            {
                return Err(PersistenceError::backend(
                    "on_load_shielded_activity_fn and on_load_shielded_activity_free_fn must be \
                     provided together",
                ));
            }
            if let Some(load_activity) = self.callbacks.on_load_shielded_activity_fn {
                let mut act_ptr: *const ShieldedActivityRestoreFFI = std::ptr::null();
                let mut act_count: usize = 0;
                let rc =
                    unsafe { load_activity(self.callbacks.context, &mut act_ptr, &mut act_count) };
                if rc != 0 {
                    return Err(PersistenceError::backend(format!(
                        "on_load_shielded_activity_fn returned error code {}",
                        rc
                    )));
                }
                struct ActivityGuard {
                    context: *mut c_void,
                    free_fn: Option<
                        unsafe extern "C" fn(
                            context: *mut c_void,
                            entries: *const ShieldedActivityRestoreFFI,
                            count: usize,
                        ),
                    >,
                    entries: *const ShieldedActivityRestoreFFI,
                    count: usize,
                }
                impl Drop for ActivityGuard {
                    fn drop(&mut self) {
                        if let Some(free_fn) = self.free_fn {
                            unsafe { free_fn(self.context, self.entries, self.count) };
                        }
                    }
                }
                let _activity_guard = ActivityGuard {
                    context: self.callbacks.context,
                    free_fn: self.callbacks.on_load_shielded_activity_free_fn,
                    entries: act_ptr,
                    count: act_count,
                };
                if !act_ptr.is_null() && act_count > 0 {
                    let slice = unsafe { slice::from_raw_parts(act_ptr, act_count) };
                    for ffi in slice {
                        let kind = match ffi.kind_tag {
                            0 => ShieldedActivityKind::Shield,
                            1 => ShieldedActivityKind::ShieldFromAssetLock,
                            2 => ShieldedActivityKind::Received,
                            3 => ShieldedActivityKind::Sent,
                            4 => ShieldedActivityKind::Unshield,
                            5 => ShieldedActivityKind::Withdrawal,
                            6 => ShieldedActivityKind::IdentityCreate {
                                identity_id: ffi.identity_id,
                            },
                            // 7 and any unknown tag fall back to the
                            // residual — a forward-compat tag we don't yet
                            // model still loads as an opaque spend rather
                            // than getting dropped.
                            _ => ShieldedActivityKind::ShieldedSpend,
                        };
                        let direction = match ffi.direction {
                            0 => ShieldedDirection::In,
                            1 => ShieldedDirection::Out,
                            2 => ShieldedDirection::SelfTransfer,
                            other => {
                                // Unlike kind_tag (whose residual
                                // `ShieldedSpend` variant is a designed
                                // forward-compat catch-all), direction has
                                // no "unknown" bucket — make a corrupted /
                                // future byte loud instead of silently
                                // reading as a real classification.
                                tracing::warn!(
                                    direction = other,
                                    "unknown shielded-activity direction byte on load; folding to SelfTransfer"
                                );
                                ShieldedDirection::SelfTransfer
                            }
                        };
                        let status = match ffi.status {
                            0 => ShieldedActivityStatus::Pending,
                            1 => ShieldedActivityStatus::Confirmed,
                            2 => ShieldedActivityStatus::Failed,
                            other => {
                                // Failed is materially distinct from
                                // Pending/Confirmed — never let a stray
                                // byte silently mark an operation failed.
                                tracing::warn!(
                                    status = other,
                                    "unknown shielded-activity status byte on load; folding to Pending so a scan can re-confirm it"
                                );
                                ShieldedActivityStatus::Pending
                            }
                        };
                        let counterparty = if ffi.counterparty_ptr.is_null()
                            || ffi.counterparty_len == 0
                        {
                            None
                        } else {
                            Some(unsafe {
                                slice::from_raw_parts(ffi.counterparty_ptr, ffi.counterparty_len)
                                    .to_vec()
                            })
                        };
                        let memo = if ffi.memo_ptr.is_null() || ffi.memo_len == 0 {
                            None
                        } else {
                            Some(unsafe {
                                slice::from_raw_parts(ffi.memo_ptr, ffi.memo_len).to_vec()
                            })
                        };
                        let note_cmxs =
                            unsafe { decode_cmx_array(ffi.note_cmxs_ptr, ffi.note_cmxs_count) };
                        let spent_nullifiers = unsafe {
                            decode_cmx_array(ffi.spent_nullifiers_ptr, ffi.spent_nullifiers_count)
                        };

                        let id = SubwalletId::new(ffi.wallet_id, ffi.account_index);
                        let entry = shielded_state
                            .per_subwallet
                            .entry(id)
                            .or_insert_with(ShieldedSubwalletStartState::default);
                        entry.activity.push(ShieldedActivityEntry {
                            id: ffi.entry_id,
                            kind,
                            direction,
                            amount: ffi.amount,
                            fee: if ffi.has_fee != 0 {
                                Some(ffi.fee)
                            } else {
                                None
                            },
                            counterparty,
                            memo,
                            block_height: if ffi.has_block_height != 0 {
                                Some(ffi.block_height)
                            } else {
                                None
                            },
                            status,
                            created_at_ms: ffi.created_at_ms,
                            note_cmxs,
                            spent_nullifiers,
                        });
                    }
                }
            }

            out.shielded = shielded_state;
        }

        Ok(out)
    }

    /// Provider node keys live in the host's separate persistence path, which has no read-back callback.
    fn provider_node_keys(
        &self,
        _wallet_id: WalletId,
    ) -> Result<Vec<platform_wallet::changeset::ProviderPlatformNodePubKey>, PersistenceError> {
        Err(PersistenceError::backend(
            "the FFI backend does not yet support reading back persisted provider node keys; \
             the host owns this data through its own separate persistence path",
        ))
    }

    /// Look up a transaction record by `txid` via the
    /// `on_get_core_tx_record_fn` callback and reconstruct the
    /// [`TransactionRecord`] for the asset-lock proof flow.
    ///
    /// The proof-flow callers in
    /// `platform-wallet/src/wallet/asset_lock/sync/` only read
    /// `record.context`, `record.height()`, and (for site 4)
    /// `record.transaction.txid`. The Swift side hands back the
    /// row's actual context kind plus the raw transaction bytes, so
    /// `txid` / `context` / `transaction` are all reliable. Other
    /// fields (`account_type`, `transaction_type`, `direction`,
    /// `input_details`, `output_details`, `net_amount`, `fee`,
    /// `label`) are best-effort placeholders per the trait field
    /// contract; see
    /// [`PlatformWalletPersistence::get_core_tx_record`].
    ///
    /// The InstantSend variant requires an
    /// [`InstantLock`](dashcore::ephemerealdata::instant_lock::InstantLock)
    /// blob that the persister doesn't currently store, so for an IS
    /// hit we surface `None` (treat as miss) and let the proof
    /// flow's existing SPV-event-driven wait path complete the
    /// proof.
    ///
    /// Returns `Ok(None)` when the callback is unset, when the
    /// callback reports `out_found = false`, when the callback
    /// returns a non-zero result code (treated as a transient backend
    /// failure per the trait contract — surfaced as `None` rather
    /// than propagating), when the callback hands back a null /
    /// empty tx-bytes buffer, when the bytes don't decode as a
    /// `dashcore::Transaction`, or for an IS hit (see above).
    fn get_core_tx_record(
        &self,
        wallet_id: WalletId,
        txid: &dashcore::Txid,
    ) -> Result<
        Option<key_wallet::managed_account::transaction_record::TransactionRecord>,
        PersistenceError,
    > {
        use dashcore::consensus::Decodable;
        use dashcore::hashes::Hash;
        use key_wallet::account::{AccountType, StandardAccountType};
        use key_wallet::managed_account::transaction_record::{
            TransactionDirection, TransactionRecord,
        };
        use key_wallet::transaction_checking::{BlockInfo, TransactionContext, TransactionType};

        let Some(get_cb) = self.callbacks.on_get_core_tx_record_fn else {
            return Ok(None);
        };

        let txid_bytes: [u8; 32] = *txid.as_byte_array();
        let mut context_kind: u8 = 0;
        let mut block_height: u32 = 0;
        let mut block_hash: [u8; 32] = [0u8; 32];
        let mut block_timestamp: u32 = 0;
        let mut tx_bytes_ptr: *const u8 = std::ptr::null();
        let mut tx_bytes_len: usize = 0;
        let mut found: bool = false;

        // SAFETY: All output pointers reference Rust-owned stack
        // locals that outlive the callback invocation. `wallet_id`
        // and `txid` are fixed-size byte arrays.
        let rc = unsafe {
            get_cb(
                self.callbacks.context,
                wallet_id.as_ptr(),
                txid_bytes.as_ptr(),
                &mut context_kind,
                &mut block_height,
                block_hash.as_mut_ptr(),
                &mut block_timestamp,
                &mut tx_bytes_ptr,
                &mut tx_bytes_len,
                &mut found,
            )
        };

        // RAII guard so the tx-bytes free callback fires on every
        // exit path past this point — early returns for unknown
        // context kinds, decode failures, and the IS-skip case all
        // correctly hand the buffer back to Swift.
        struct TxBytesGuard<'a> {
            ptr: *const u8,
            len: usize,
            free_fn: Option<
                unsafe extern "C" fn(
                    context: *mut c_void,
                    tx_bytes: *const u8,
                    tx_bytes_len: usize,
                ),
            >,
            ctx: *mut c_void,
            _marker: std::marker::PhantomData<&'a ()>,
        }
        impl<'a> Drop for TxBytesGuard<'a> {
            fn drop(&mut self) {
                if let (Some(free), false) = (self.free_fn, self.ptr.is_null()) {
                    // SAFETY: ptr+len match the values the lookup
                    // callback wrote; Swift owns the allocation
                    // until this free fires.
                    unsafe { free(self.ctx, self.ptr, self.len) };
                }
            }
        }
        let _bytes_guard = TxBytesGuard {
            ptr: tx_bytes_ptr,
            len: tx_bytes_len,
            free_fn: self.callbacks.on_get_core_tx_record_free_fn,
            ctx: self.callbacks.context,
            _marker: std::marker::PhantomData,
        };

        if rc != 0 {
            tracing::debug!(
                txid = %txid,
                rc,
                "on_get_core_tx_record_fn returned a non-zero result; \
                 treating as miss"
            );
            return Ok(None);
        }
        if !found {
            return Ok(None);
        }

        let context = match context_kind {
            0 => TransactionContext::Mempool,
            1 => {
                // InstantSend requires the IS-lock blob, which the
                // persister doesn't currently store. Treat as miss
                // so the proof flow's SPV wait path completes the
                // proof from the live event stream.
                return Ok(None);
            }
            2 => TransactionContext::InBlock(BlockInfo::new(
                block_height,
                dashcore::BlockHash::from_byte_array(block_hash),
                block_timestamp,
            )),
            3 => TransactionContext::InChainLockedBlock(BlockInfo::new(
                block_height,
                dashcore::BlockHash::from_byte_array(block_hash),
                block_timestamp,
            )),
            unknown => {
                tracing::debug!(
                    txid = %txid,
                    unknown,
                    "on_get_core_tx_record_fn returned an unknown \
                     context kind; treating as miss"
                );
                return Ok(None);
            }
        };

        if tx_bytes_ptr.is_null() || tx_bytes_len == 0 {
            tracing::debug!(
                txid = %txid,
                "on_get_core_tx_record_fn reported a hit but no tx \
                 bytes; treating as miss"
            );
            return Ok(None);
        }
        // SAFETY: Swift guarantees `tx_bytes_ptr` points to
        // `tx_bytes_len` valid bytes for the duration of the
        // callback window — `_bytes_guard` keeps that window open
        // until this function returns.
        let tx_slice = unsafe { slice::from_raw_parts(tx_bytes_ptr, tx_bytes_len) };
        let transaction = match dashcore::blockdata::transaction::Transaction::consensus_decode(
            &mut &tx_slice[..],
        ) {
            Ok(tx) => tx,
            Err(err) => {
                tracing::debug!(
                    txid = %txid,
                    error = %err,
                    "on_get_core_tx_record_fn returned undecodable \
                     tx bytes; treating as miss"
                );
                return Ok(None);
            }
        };

        Ok(Some(TransactionRecord {
            transaction,
            txid: *txid,
            account_type: AccountType::Standard {
                index: 0,
                standard_account_type: StandardAccountType::BIP44Account,
            },
            context,
            transaction_type: TransactionType::Standard,
            direction: TransactionDirection::Internal,
            input_details: Vec::new(),
            output_details: Vec::new(),
            net_amount: 0,
            fee: None,
            label: String::new(),
        }))
    }
}

/// Decode `count` contiguous 32-byte commitments / nullifiers from a
/// host buffer into `Vec<[u8; 32]>`. A null pointer, a zero count, or a
/// `count` whose byte length overflows / exceeds `isize::MAX` (the
/// `slice::from_raw_parts` bound) yields an empty vec — the buffer is
/// Rust-written on persist and host-round-tripped on load, so an
/// out-of-range count means a corrupt row, and the linkage is dropped
/// rather than read past the buffer.
///
/// # Safety
/// When non-null, `ptr` must point to at least `count * 32` valid
/// bytes for the duration of the call.
#[cfg(feature = "shielded")]
unsafe fn decode_cmx_array(ptr: *const u8, count: usize) -> Vec<[u8; 32]> {
    if ptr.is_null() || count == 0 {
        return Vec::new();
    }
    // `count` is host-supplied: guard the multiplication so a corrupt
    // row degrades to a dropped linkage instead of an overflowed length
    // handed to `from_raw_parts` (UB). Also enforce `from_raw_parts`'
    // documented bound that the slice length must not exceed
    // `isize::MAX` bytes.
    let Some(byte_len) = count
        .checked_mul(32)
        .filter(|&len| len <= isize::MAX as usize)
    else {
        tracing::warn!(
            count,
            "shielded activity linkage count overflows on load; dropping linkage bytes"
        );
        return Vec::new();
    };
    let bytes = slice::from_raw_parts(ptr, byte_len);
    bytes
        .chunks_exact(32)
        .filter_map(|c| <[u8; 32]>::try_from(c).ok())
        .collect()
}

/// Discriminant byte for a `ShieldedDirection` (FFI: 0 In, 1 Out, 2 Self).
#[cfg(feature = "shielded")]
fn activity_direction_tag(d: &platform_wallet::wallet::shielded::ShieldedDirection) -> u8 {
    use platform_wallet::wallet::shielded::ShieldedDirection::*;
    match d {
        In => 0,
        Out => 1,
        SelfTransfer => 2,
    }
}

/// Discriminant byte for a `ShieldedActivityStatus` (FFI: 0 Pending,
/// 1 Confirmed, 2 Failed).
#[cfg(feature = "shielded")]
fn activity_status_tag(s: &platform_wallet::wallet::shielded::ShieldedActivityStatus) -> u8 {
    use platform_wallet::wallet::shielded::ShieldedActivityStatus::*;
    match s {
        Pending => 0,
        Confirmed => 1,
        Failed => 2,
    }
}

/// [`AccountSpecFFI`] layout.
///
/// The returned struct borrows `xpub_bytes` — caller must keep the
/// slice alive for the duration of the FFI callback.
fn build_account_spec_ffi(account_type: &AccountType, xpub_bytes: &[u8]) -> AccountSpecFFI {
    // Default zeroed payload; the match below fills in only the
    // fields relevant to the active variant. Fields for other
    // variants stay at their zero value and are ignored on the
    // receiving side per the struct docs.
    let mut spec = AccountSpecFFI {
        type_tag: AccountTypeTagFFI::Standard as u8,
        standard_tag: StandardAccountTypeTagFFI::Bip44 as u8,
        index: 0,
        registration_index: 0,
        key_class: 0,
        user_identity_id: [0u8; 32],
        friend_identity_id: [0u8; 32],
        account_xpub_bytes: xpub_bytes.as_ptr(),
        account_xpub_bytes_len: xpub_bytes.len(),
        // Set by `build_account_specs_for_callback` for the
        // `ProviderPlatformKeys` entry; null/0 for every other account.
        derived_platform_node_keys: std::ptr::null(),
        derived_platform_node_keys_count: 0,
    };
    // The producer side casts each `AccountTypeTagFFI` /
    // `StandardAccountTypeTagFFI` variant to `u8` because both fields
    // are now FFI-typed as plain `u8` (see the field comments on
    // `AccountSpecFFI`). The consumer validates the byte via
    // `try_from_u8` before any `match`.
    match account_type {
        AccountType::Standard {
            index,
            standard_account_type,
        } => {
            spec.type_tag = AccountTypeTagFFI::Standard as u8;
            spec.standard_tag = match standard_account_type {
                StandardAccountType::BIP44Account => StandardAccountTypeTagFFI::Bip44 as u8,
                StandardAccountType::BIP32Account => StandardAccountTypeTagFFI::Bip32 as u8,
            };
            spec.index = *index;
        }
        AccountType::CoinJoin { index } => {
            spec.type_tag = AccountTypeTagFFI::CoinJoin as u8;
            spec.index = *index;
        }
        AccountType::IdentityRegistration => {
            spec.type_tag = AccountTypeTagFFI::IdentityRegistration as u8;
        }
        AccountType::IdentityTopUp { registration_index } => {
            spec.type_tag = AccountTypeTagFFI::IdentityTopUp as u8;
            spec.registration_index = *registration_index;
        }
        AccountType::IdentityTopUpNotBoundToIdentity => {
            spec.type_tag = AccountTypeTagFFI::IdentityTopUpNotBoundToIdentity as u8;
        }
        AccountType::IdentityInvitation => {
            spec.type_tag = AccountTypeTagFFI::IdentityInvitation as u8;
        }
        AccountType::AssetLockAddressTopUp => {
            spec.type_tag = AccountTypeTagFFI::AssetLockAddressTopUp as u8;
        }
        AccountType::AssetLockShieldedAddressTopUp => {
            spec.type_tag = AccountTypeTagFFI::AssetLockShieldedAddressTopUp as u8;
        }
        AccountType::ProviderVotingKeys => {
            spec.type_tag = AccountTypeTagFFI::ProviderVotingKeys as u8;
        }
        AccountType::ProviderOwnerKeys => {
            spec.type_tag = AccountTypeTagFFI::ProviderOwnerKeys as u8;
        }
        AccountType::ProviderOperatorKeys => {
            spec.type_tag = AccountTypeTagFFI::ProviderOperatorKeys as u8;
        }
        AccountType::ProviderPlatformKeys => {
            spec.type_tag = AccountTypeTagFFI::ProviderPlatformKeys as u8;
        }
        AccountType::DashpayReceivingFunds {
            index,
            user_identity_id,
            friend_identity_id,
        } => {
            spec.type_tag = AccountTypeTagFFI::DashpayReceivingFunds as u8;
            spec.index = *index;
            spec.user_identity_id = *user_identity_id;
            spec.friend_identity_id = *friend_identity_id;
        }
        AccountType::DashpayExternalAccount {
            index,
            user_identity_id,
            friend_identity_id,
        } => {
            spec.type_tag = AccountTypeTagFFI::DashpayExternalAccount as u8;
            spec.index = *index;
            spec.user_identity_id = *user_identity_id;
            spec.friend_identity_id = *friend_identity_id;
        }
        AccountType::PlatformPayment { account, key_class } => {
            spec.type_tag = AccountTypeTagFFI::PlatformPayment as u8;
            spec.index = *account;
            spec.key_class = *key_class;
        } // TODO(events): the `IdentityAuthenticationEcdsa` /
          // `IdentityAuthenticationBls` upstream `AccountType` variants
          // were removed when identity-key derivation moved off the
          // wallet-account model. The FFI ABI still exposes the matching
          // `AccountTypeTagFFI` tags for backwards compatibility, but
          // there's no upstream variant to map them to right now. If a
          // wallet record arrives with an identity-auth derivation path,
          // the bridge surface needs new entry points for the new
          // identity-key shape — until then no upstream `AccountType`
          // value can produce these tags, so the match is exhaustive
          // without explicit branches.
    }
    spec
}

/// Build the `Vec<AccountSpecFFI>` array for
/// `on_persist_account_registrations_fn` plus the parallel storage each
/// spec borrows into:
/// 1. `Vec<Vec<u8>>` — bincoded xpub byte buffers
///    (`account_xpub_bytes`).
/// 2. `Vec<Vec<ProviderPlatformNodeKeyFFI>>` — one inner Vec per
///    provider entry holding its pre-derived platform-node keys
///    (`derived_platform_node_keys`); empty for the BLS operator entry
///    and for every ECDSA account.
///
/// All three share lifetime — the caller must keep them alive until
/// after the callback returns.
#[allow(clippy::type_complexity)]
fn build_account_specs_for_callback(
    entries: &[AccountRegistrationEntry],
    provider_entries: &[ProviderKeyAccountEntry],
) -> Result<
    (
        Vec<AccountSpecFFI>,
        Vec<Vec<u8>>,
        Vec<Vec<ProviderPlatformNodeKeyFFI>>,
    ),
    String,
> {
    // Pre-encode every extended public key once so each spec slot can
    // borrow the pointer + length without a self-referential lifetime
    // trick. ECDSA accounts encode their secp256k1 `ExtendedPubKey`;
    // provider key accounts (BLS operator / EdDSA platform node)
    // encode their own-curve extended public key into the same slot —
    // the `type_tag` disambiguates the decode on the restore side.
    let mut xpub_buffers: Vec<Vec<u8>> = Vec::with_capacity(entries.len() + provider_entries.len());
    for entry in entries {
        let bytes = bincode::encode_to_vec(entry.account_xpub, config::standard())
            .map_err(|e| format!("failed to encode account xpub: {}", e))?;
        xpub_buffers.push(bytes);
    }
    for entry in provider_entries {
        let bytes = match &entry.extended_public_key {
            ProviderKeyExtendedPubKey::Bls(key) => bincode::encode_to_vec(key, config::standard())
                .map_err(|e| format!("failed to encode provider BLS xpub: {}", e))?,
            ProviderKeyExtendedPubKey::EdDSA(key) => {
                bincode::encode_to_vec(key, config::standard())
                    .map_err(|e| format!("failed to encode provider EdDSA xpub: {}", e))?
            }
        };
        xpub_buffers.push(bytes);
    }

    // Pre-derived platform-node key storage, index-aligned to
    // `provider_entries`. Built to completion BEFORE any spec borrows a
    // pointer into it so the inner Vecs never move under a live pointer.
    let derived_storage: Vec<Vec<ProviderPlatformNodeKeyFFI>> = provider_entries
        .iter()
        .map(|entry| {
            entry
                .derived_platform_node_keys
                .iter()
                .map(|k| ProviderPlatformNodeKeyFFI {
                    index: k.index,
                    public_key: k.public_key,
                    node_id: k.node_id,
                })
                .collect()
        })
        .collect();

    let mut specs: Vec<AccountSpecFFI> = Vec::with_capacity(xpub_buffers.len());
    let mut idx = 0;
    for entry in entries {
        specs.push(build_account_spec_ffi(
            &entry.account_type,
            &xpub_buffers[idx],
        ));
        idx += 1;
    }
    for (p_idx, entry) in provider_entries.iter().enumerate() {
        let mut spec = build_account_spec_ffi(&entry.account_type, &xpub_buffers[idx]);
        // Point at the pre-built (stable) derived-key storage for this
        // provider entry. Empty for the BLS operator account, so its
        // spec keeps the null/0 default from `build_account_spec_ffi`.
        let rows = &derived_storage[p_idx];
        if !rows.is_empty() {
            spec.derived_platform_node_keys = rows.as_ptr();
            spec.derived_platform_node_keys_count = rows.len();
        }
        specs.push(spec);
        idx += 1;
    }
    Ok((specs, xpub_buffers, derived_storage))
}

/// Build the `Vec<AccountAddressPoolFFI>` array for
/// `on_persist_account_address_pools_fn`.
///
/// Returns three parallel Vecs whose lifetimes are tied together:
/// 1. `Vec<AccountAddressPoolFFI>` — the heap-array the callback
///    iterates over. Each entry's `addresses_ptr` borrows into one
///    of the inner Vecs from (2).
/// 2. `Vec<Vec<CoreAddressEntryFFI>>` — one inner Vec per pool,
///    holding the pool's address entries. Each entry's c-string
///    pointers borrow into (3).
/// 3. `Vec<CString>` — owned c-string storage for every (address,
///    derivation_path) pair across all pools.
///
/// Caller must keep all three alive until after the FFI callback
/// returns. Mirrors the lifetime discipline the prior dedicated
/// `store_account_addresses` impl used; same forgiveness on
/// PlatformAddress conversion failures (falls back to base58check).
#[allow(clippy::type_complexity)]
fn build_address_pools_for_callback(
    entries: &[AccountAddressPoolEntry],
) -> Result<
    (
        Vec<AccountAddressPoolFFI>,
        Vec<Vec<CoreAddressEntryFFI>>,
        Vec<CString>,
    ),
    String,
> {
    // Owned string pool — every (address, path) c-string borrowed by
    // every CoreAddressEntryFFI lives in this Vec until callback end.
    let mut owned_strings: Vec<CString> = Vec::new();
    // Per-pool address-entry storage. Indexed parallel to the
    // returned `pools` Vec; pool i's `addresses_ptr` points at
    // `address_storage[i].as_ptr()`.
    let mut address_storage: Vec<Vec<CoreAddressEntryFFI>> = Vec::with_capacity(entries.len());
    let mut pools: Vec<AccountAddressPoolFFI> = Vec::with_capacity(entries.len());

    for entry in entries {
        let pool_tag = match entry.pool_type {
            AddressPoolType::External => AddressPoolTypeTagFFI::External,
            AddressPoolType::Internal => AddressPoolTypeTagFFI::Internal,
            AddressPoolType::Absent => AddressPoolTypeTagFFI::Absent,
            AddressPoolType::AbsentHardened => AddressPoolTypeTagFFI::AbsentHardened,
        } as u8;

        let is_platform_payment = matches!(entry.account_type, AccountType::PlatformPayment { .. });

        let mut pool_entries: Vec<CoreAddressEntryFFI> = Vec::with_capacity(entry.addresses.len());
        for info in &entry.addresses {
            let entry_ffi = build_core_address_entry_ffi(
                info,
                pool_tag,
                is_platform_payment,
                &mut owned_strings,
            )?;
            pool_entries.push(entry_ffi);
        }

        // Account spec borrows an empty xpub slice — the
        // address-pool callback receiver does not need the xpub
        // (it matches by the same identifier subset
        // `on_persist_account_registrations_fn` uses).
        let empty_xpub: &[u8] = &[];
        let spec = build_account_spec_ffi(&entry.account_type, empty_xpub);

        // Build the FFI struct after the inner Vec is finalized so
        // the pointer is stable.
        let addresses_ptr = pool_entries.as_ptr();
        let addresses_count = pool_entries.len();
        address_storage.push(pool_entries);

        pools.push(AccountAddressPoolFFI {
            account: spec,
            pool_type_tag: pool_tag,
            addresses_ptr,
            addresses_count,
        });
    }

    Ok((pools, address_storage, owned_strings))
}

/// Build a single `CoreAddressEntryFFI` from an `AddressInfo`,
/// pushing the owned (address, path) c-strings into `owned_strings`
/// so they outlive the callback window.
///
/// Recover the network an `Address` renders for. `Address` no longer exposes its network
/// directly (the prefix is shared across testnet/devnet and legacy-regtest), but for the
/// bech32m platform-payment addresses rendered here the prefix is decisive, so probing
/// yields the correct HRP (mainnet `ds`, testnet/devnet `tb`, regtest `dsrt`).
fn address_display_network(address: &dashcore::Address) -> dashcore::Network {
    let unchecked = address.as_unchecked();
    if unchecked.is_valid_for_network(dashcore::Network::Mainnet) {
        dashcore::Network::Mainnet
    } else if unchecked.is_valid_for_network(dashcore::Network::Testnet) {
        dashcore::Network::Testnet
    } else {
        dashcore::Network::Regtest
    }
}

fn build_core_address_entry_ffi(
    info: &AddressInfo,
    pool_type_tag: u8,
    is_platform_payment: bool,
    owned_strings: &mut Vec<CString>,
) -> Result<CoreAddressEntryFFI, String> {
    // Pick the right display encoding. PlatformPayment pools render
    // as DIP-0018 bech32m; everything else uses base58check. If the
    // PlatformAddress conversion fails (only P2PKH / P2SH supported)
    // fall back to base58check so the address still surfaces.
    let rendered_address = if is_platform_payment {
        let network = address_display_network(&info.address);
        let converted: Result<PlatformAddress, _> = PlatformAddress::try_from(info.address.clone());
        converted
            .map(|p| p.to_bech32m_string(network))
            .unwrap_or_else(|_| info.address.to_string())
    } else {
        info.address.to_string()
    };
    let address_c =
        CString::new(rendered_address).map_err(|e| format!("address contained NUL byte: {}", e))?;
    let path_c = CString::new(info.path.to_string())
        .map_err(|e| format!("derivation path contained NUL byte: {}", e))?;
    let address_ptr = address_c.as_ptr();
    let path_ptr = path_c.as_ptr();
    owned_strings.push(address_c);
    owned_strings.push(path_c);

    let mut public_key = [0u8; 33];
    let has_public_key = match &info.public_key {
        Some(PublicKeyType::ECDSA(bytes)) if bytes.len() == 33 => {
            public_key.copy_from_slice(bytes);
            true
        }
        _ => false,
    };

    Ok(CoreAddressEntryFFI {
        public_key,
        has_public_key,
        pool_type_tag,
        address_index: info.index,
        is_used: info.used,
        balance: info.balance,
        address_base58: address_ptr,
        derivation_path: path_ptr,
    })
}

/// Reverse of [`build_core_address_entry_ffi`]: rebuild an
/// `AddressInfo` from a Swift-supplied `CoreAddressEntryFFI` on the
/// wallet-load path.
///
/// # Safety
/// `entry.address_base58` / `entry.derivation_path` must be valid
/// NUL-terminated C strings for the duration of the call.
unsafe fn address_info_from_ffi(
    entry: &CoreAddressEntryFFI,
    network: Network,
) -> Result<AddressInfo, String> {
    if entry.address_base58.is_null() {
        return Err("CoreAddressEntryFFI.address_base58 is null".to_string());
    }
    if entry.derivation_path.is_null() {
        return Err("CoreAddressEntryFFI.derivation_path is null".to_string());
    }
    let address_str = CStr::from_ptr(entry.address_base58)
        .to_str()
        .map_err(|e| format!("address_base58 not UTF-8: {}", e))?;
    let parsed = dashcore::Address::from_str(address_str)
        .map_err(|e| format!("failed to parse address '{}': {}", address_str, e))?
        .require_network(network)
        .map_err(|e| format!("address '{}' not on {:?}: {}", address_str, network, e))?;
    let script_pubkey = parsed.script_pubkey();
    // Re-tag with the wallet's exact network. Devnet (and regtest)
    // share testnet's base58 prefixes, so `require_network` only
    // VALIDATES the parse — the returned value keeps the as-parsed
    // (Testnet) tag. `Address` equality and hashing include the
    // network, and every runtime lookup key is built via
    // `Address::from_script(script, wallet_network)`, so a
    // Testnet-tagged restored key silently misses the pool's
    // address-keyed maps (`get_address_info`) on a devnet wallet.
    // The observable failure: outputs paying restored addresses are
    // counted (`contains_script_pub_key` is script-keyed and hits)
    // but never credited as UTXOs — a restored wallet permanently
    // loses change returned by its own transactions. Rebuild from
    // the script so the restored key is identical to runtime keys.
    let address = dashcore::Address::from_script(&script_pubkey, network).map_err(|e| {
        format!(
            "failed to rebuild address '{}' from its script for {:?}: {}",
            address_str, network, e
        )
    })?;
    let path_str = CStr::from_ptr(entry.derivation_path)
        .to_str()
        .map_err(|e| format!("derivation_path not UTF-8: {}", e))?;
    let path = DerivationPath::from_str(path_str)
        .map_err(|e| format!("failed to parse derivation path '{}': {}", path_str, e))?;
    let public_key = if entry.has_public_key {
        Some(PublicKeyType::ECDSA(entry.public_key.to_vec()))
    } else {
        None
    };
    Ok(AddressInfo {
        address,
        script_pubkey,
        public_key,
        index: entry.address_index,
        path,
        used: entry.is_used,
        generated_at: 0,
        used_at: if entry.is_used { Some(0) } else { None },
        tx_count: 0,
        total_received: 0,
        total_sent: 0,
        balance: entry.balance,
        label: None,
        metadata: std::collections::BTreeMap::new(),
    })
}

/// Merge persisted `AddressInfo` rows into a managed account's
/// `AddressPool`. Upsert semantics: a persisted row overwrites the
/// gap-limit default `ManagedWalletInfo::from_wallet` pre-derived at
/// the same index, and the reverse-lookup maps + `highest_*`
/// watermarks are extended to cover indices past that default
/// gap window.
fn restore_address_pool(pool: &mut AddressPool, infos: Vec<AddressInfo>) {
    for info in infos {
        let idx = info.index;
        pool.address_index.insert(info.address.clone(), idx);
        pool.script_pubkey_index
            .insert(info.script_pubkey.clone(), idx);
        pool.highest_generated = Some(pool.highest_generated.map_or(idx, |h| h.max(idx)));
        if info.used {
            pool.used_indices.insert(idx);
            pool.highest_used = Some(pool.highest_used.map_or(idx, |h| h.max(idx)));
        }
        pool.addresses.insert(idx, info);
    }
}

/// Bucket a slice of upstream-emitted `DerivedAddress` entries into the
/// same `AccountAddressPoolFFI` shape `build_address_pools_for_callback`
/// produces, so the event-driven gap-limit-extension flow can fan out
/// through the existing `on_persist_account_address_pools_fn` pipeline
/// rather than introducing a parallel callback.
///
/// Each upstream entry already carries `(account_type, pool_type,
/// derivation_index, address, public_key)`; we group on
/// `(account_type, pool_type)` so a single block that pushed the
/// gap-limit boundary on multiple pools (e.g. an internal change
/// receive that extends Internal AND a separate external receive that
/// extends External) emits one pool snapshot per pool variant.
///
/// `derivation_path` is computed deterministically per-entry by
/// [`platform_wallet::derivation_path_string_for_derived_address`]
/// from the same `(account_type, pool_type, derivation_index)`
/// triple the pool itself uses at derive time. Falls back to an
/// empty string only when the account-level path can't be resolved
/// (`AccountType` variants whose `derivation_path` errors); the
/// address string remains the authoritative join key in that case.
#[allow(clippy::type_complexity)]
fn build_address_pools_from_derived(
    derived: &[platform_wallet::DerivedAddress],
) -> Result<
    (
        Vec<AccountAddressPoolFFI>,
        Vec<Vec<CoreAddressEntryFFI>>,
        Vec<CString>,
    ),
    String,
> {
    use std::collections::BTreeMap;
    if derived.is_empty() {
        return Ok((Vec::new(), Vec::new(), Vec::new()));
    }

    // Bucket key: (account_type, pool_type). Preserve arrival order
    // within a bucket — the upstream `project_derived_addresses`
    // already deduped by `(account_type, pool_type, derivation_index)`,
    // so two entries in the same bucket here always have distinct
    // indices.
    let mut buckets: BTreeMap<(usize, AddressPoolType), Vec<&platform_wallet::DerivedAddress>> =
        BTreeMap::new();
    // We can't use AccountType as the BTreeMap key directly (no `Ord`
    // upstream), so keep an account-type index per bucket and look it
    // up by the discriminant order entries arrive in.
    let mut account_types: Vec<AccountType> = Vec::new();
    let mut account_type_to_idx: Vec<(AccountType, usize)> = Vec::new();
    for d in derived {
        let idx = account_type_to_idx
            .iter()
            .find(|(at, _)| *at == d.account_type)
            .map(|(_, i)| *i)
            .unwrap_or_else(|| {
                let i = account_types.len();
                account_types.push(d.account_type);
                account_type_to_idx.push((d.account_type, i));
                i
            });
        buckets.entry((idx, d.pool_type)).or_default().push(d);
    }

    let mut owned_strings: Vec<CString> = Vec::new();
    let mut address_storage: Vec<Vec<CoreAddressEntryFFI>> = Vec::with_capacity(buckets.len());
    let mut pools: Vec<AccountAddressPoolFFI> = Vec::with_capacity(buckets.len());

    for ((account_idx, pool_type), bucket) in buckets {
        let account_type = account_types[account_idx];
        let pool_tag = match pool_type {
            AddressPoolType::External => AddressPoolTypeTagFFI::External,
            AddressPoolType::Internal => AddressPoolTypeTagFFI::Internal,
            AddressPoolType::Absent => AddressPoolTypeTagFFI::Absent,
            AddressPoolType::AbsentHardened => AddressPoolTypeTagFFI::AbsentHardened,
        } as u8;
        let is_platform_payment = matches!(account_type, AccountType::PlatformPayment { .. });

        let mut pool_entries: Vec<CoreAddressEntryFFI> = Vec::with_capacity(bucket.len());
        for d in bucket {
            // Re-render the address. PlatformPayment uses DIP-0018
            // bech32m; everything else base58check (matching
            // `build_core_address_entry_ffi`'s logic).
            let rendered_address = if is_platform_payment {
                let network = address_display_network(&d.address);
                let converted: Result<PlatformAddress, _> =
                    PlatformAddress::try_from(d.address.clone());
                converted
                    .map(|p| p.to_bech32m_string(network))
                    .unwrap_or_else(|_| d.address.to_string())
            } else {
                d.address.to_string()
            };
            let address_c = CString::new(rendered_address)
                .map_err(|e| format!("derived address contained NUL byte: {}", e))?;
            // Render the BIP32 derivation path via the
            // platform-wallet helper. Path-shape decisions are
            // protocol-aware and live next to other key-derivation
            // logic in the non-FFI crate; the FFI shim's job is
            // only to marshal the resulting string into the C ABI.
            // Falls back to an empty string for non-Standard
            // variants whose account-level path doesn't render —
            // the address string remains the authoritative join
            // key on the persister side regardless.
            let path_str =
                platform_wallet::derivation_path_string_for_derived_address(d).unwrap_or_default();
            let path_c = CString::new(path_str)
                .map_err(|e| format!("derivation path contained NUL byte: {}", e))?;
            let address_ptr = address_c.as_ptr();
            let path_ptr = path_c.as_ptr();
            owned_strings.push(address_c);
            owned_strings.push(path_c);

            pool_entries.push(CoreAddressEntryFFI {
                // Upstream `DerivedAddress::public_key` is now a
                // `dashcore::PublicKey`; compressed serialization
                // is the 33-byte form our FFI expects.
                public_key: d.public_key.inner.serialize(),
                has_public_key: true,
                pool_type_tag: pool_tag,
                address_index: d.derivation_index,
                // Newly-derived addresses haven't been seen in any
                // tx yet (they came from gap-limit extension,
                // not from observing the address as used). The
                // upstream `mark_address_used` flow that triggered
                // this derivation marks the OLD address that got
                // matched, not these new ones; their `is_used`
                // stays false until SPV later observes a tx paying
                // to one of them.
                is_used: false,
                balance: 0,
                address_base58: address_ptr,
                derivation_path: path_ptr,
            });
        }

        let empty_xpub: &[u8] = &[];
        let spec = build_account_spec_ffi(&account_type, empty_xpub);
        let addresses_ptr = pool_entries.as_ptr();
        let addresses_count = pool_entries.len();
        address_storage.push(pool_entries);

        pools.push(AccountAddressPoolFFI {
            account: spec,
            pool_type_tag: pool_tag,
            addresses_ptr,
            addresses_count,
        });
    }

    Ok((pools, address_storage, owned_strings))
}

/// Bucket the changeset's marked-used address entries into
/// [`AccountAddressPoolEntry`] values so the used-flag flip rides the
/// same `build_address_pools_for_callback` →
/// `on_persist_account_address_pools_fn` pipeline the registration
/// snapshot and derived-address emits already use — one Swift code
/// path (`persistAccountAddresses`) covers all three.
///
/// Grouping key is `(account_type, pool_type)`, mirroring
/// `build_address_pools_from_derived`. Each entry's `AddressInfo` is
/// the authoritative post-mark pool snapshot the bridge captured
/// (`used == true`), so no field synthesis happens here.
fn group_marked_used_into_pool_entries(
    marked: &[key_wallet::transaction_checking::DerivedAddressInfo],
) -> Vec<AccountAddressPoolEntry> {
    let mut entries: Vec<AccountAddressPoolEntry> = Vec::new();
    for d in marked {
        if let Some(bucket) = entries
            .iter_mut()
            .find(|e| e.account_type == d.account_type && e.pool_type == d.pool_type)
        {
            bucket.addresses.push(d.info.clone());
        } else {
            entries.push(AccountAddressPoolEntry {
                account_type: d.account_type,
                pool_type: d.pool_type,
                addresses: vec![d.info.clone()],
            });
        }
    }
    entries
}

/// RAII drop-guard that invokes the paired free callback on exit, so
/// any error path through `FFIPersister::load` still returns memory
/// to Swift.
struct LoadGuard {
    context: *mut c_void,
    free_fn: Option<LoadWalletListFreeFn>,
    entries: *const WalletRestoreEntryFFI,
    count: usize,
}

impl Drop for LoadGuard {
    fn drop(&mut self) {
        if self.entries.is_null() || self.count == 0 {
            return;
        }
        if let Some(free_fn) = self.free_fn {
            unsafe { free_fn(self.context, self.entries, self.count) };
        }
    }
}

/// Reconstruct an external-signable [`Wallet`] + matching start-state
/// bucket from a single `WalletRestoreEntryFFI`. The mnemonic / seed
/// stays in the host's keychain; signing requests route back through
/// the configured signer surface (see
/// `Wallet::new_external_signable`). Earlier revisions of this code
/// path produced a `WatchOnly` wallet — that has been replaced.
fn build_wallet_start_state(
    entry: &WalletRestoreEntryFFI,
) -> Result<
    (
        ClientWalletStartState,
        Option<platform_wallet::PlatformAddressSyncStartState>,
    ),
    PersistenceError,
> {
    let network: Network = entry.network.into();

    // Build the per-account collection from the typed spec array.
    let mut accounts = AccountCollection::new();
    let specs: &[AccountSpecFFI] = if entry.accounts.is_null() || entry.accounts_count == 0 {
        &[]
    } else {
        unsafe { slice::from_raw_parts(entry.accounts, entry.accounts_count) }
    };
    for spec in specs {
        // Skip-and-continue on legacy `IdentityAuthentication{Ecdsa,Bls}`
        // rows — those `AccountTypeTagFFI` discriminants are still ABI-
        // valid but their upstream `AccountType` variants were removed,
        // so `account_type_from_spec` deliberately returns `Err` for
        // them. Propagating that with `?` would abort the entire
        // `load()` (every wallet, every launch) the moment a single
        // such row exists in SwiftData. Treating it as recoverable
        // snapshot drift matches how the UTXO loop a few lines below
        // handles the same failure mode.
        //
        // Only the *legacy* tag bytes (15 / 16) are skip-and-continue;
        // real validation errors (out-of-range bytes from a corrupt
        // SwiftData row) propagate so the corruption surfaces rather
        // than silently under-restoring accounts.
        let account_type = match account_type_from_spec(spec) {
            Ok(t) => t,
            Err(e) => {
                if is_legacy_removed_account_tag(spec.type_tag) {
                    tracing::warn!(
                        wallet_id = %hex::encode(entry.wallet_id),
                        type_tag = spec.type_tag,
                        "load: skipping legacy IdentityAuthentication account tag"
                    );
                    continue;
                }
                return Err(e);
            }
        };
        let xpub_bytes =
            unsafe { slice_from_raw(spec.account_xpub_bytes, spec.account_xpub_bytes_len) };

        // Provider key-material accounts (BLS operator keys / EdDSA
        // platform node keys) carry a non-secp256k1 extended public key in
        // the same `account_xpub_bytes` slot; `account_type` discriminates
        // the decode. The rebuild itself is the shared helper every backend's
        // restore path uses (the SQLite backend calls it too).
        let provider_key = match account_type {
            AccountType::ProviderOperatorKeys => {
                let (bls_pubkey, _): (ExtendedBLSPubKey, usize) =
                    bincode::decode_from_slice(xpub_bytes, config::standard()).map_err(|e| {
                        PersistenceError::backend(format!(
                            "failed to decode provider BLS xpub: {}",
                            e
                        ))
                    })?;
                Some(ProviderKeyExtendedPubKey::Bls(bls_pubkey))
            }
            AccountType::ProviderPlatformKeys => {
                let (ed_pubkey, _): (ExtendedEd25519PubKey, usize) =
                    bincode::decode_from_slice(xpub_bytes, config::standard()).map_err(|e| {
                        PersistenceError::backend(format!(
                            "failed to decode provider EdDSA xpub: {}",
                            e
                        ))
                    })?;
                Some(ProviderKeyExtendedPubKey::EdDSA(ed_pubkey))
            }
            _ => None,
        };
        if let Some(key) = provider_key {
            rebuild_provider_key_account(
                &mut accounts,
                entry.wallet_id,
                network,
                account_type,
                &key,
            )
            .map_err(|e| {
                PersistenceError::backend(format!("provider key account rebuild failed: {e}"))
            })?;
            continue;
        }

        let (account_xpub, _): (ExtendedPubKey, usize) =
            bincode::decode_from_slice(xpub_bytes, config::standard()).map_err(|e| {
                PersistenceError::backend(format!("failed to decode account xpub: {}", e))
            })?;
        let account =
            Account::from_xpub(Some(entry.wallet_id), account_type, account_xpub, network)
                .map_err(|e| {
                    PersistenceError::backend(format!("Account::from_xpub failed: {:?}", e))
                })?;
        accounts.insert(account).map_err(|e| {
            PersistenceError::backend(format!("AccountCollection::insert failed: {}", e))
        })?;
    }

    // External-signable wallet — the mnemonic / seed lives in the
    // iOS Keychain, not in this Rust handle. Signing requests route
    // back to the host through the configured signer surface; the
    // host fetches the mnemonic from the Keychain on demand. The
    // wallet_id is passed in directly (no recomputation from a root
    // xpub the snapshot doesn't carry).
    let wallet = Wallet::new_external_signable(network, entry.wallet_id, accounts);

    // Stamp the persisted core-chain sync metadata onto the rebuilt
    // managed-info. `from_wallet` seeds `synced_height` and
    // `last_processed_height` to `birth_height - 1`; we then override
    // with the values Swift actually persisted, treating zero as
    // "unknown" so we don't clobber the seeded default for fresh /
    // never-synced wallets.
    let mut wallet_info = ManagedWalletInfo::from_wallet(&wallet, entry.birth_height);
    if entry.synced_height > 0 {
        wallet_info.metadata.synced_height = entry.synced_height;
    }
    if entry.last_processed_height > 0 {
        wallet_info.metadata.last_processed_height = entry.last_processed_height;
    }
    if entry.last_synced > 0 {
        wallet_info.metadata.last_synced = Some(entry.last_synced);
    }

    // Persisted `last_applied_chain_lock` — bincode-decoded from the
    // bytes Swift handed back. Restoring this before the wallet
    // enters the manager means the asset-lock-resume CL-from-metadata
    // fallback (`proof.rs`) can fire immediately at app launch on
    // any tracked lock whose funding block height is `<= cl.block_height`,
    // without waiting for SPV to re-apply a fresh CL. SPV persists
    // its own `best_chainlock` independently; this is the symmetric
    // wallet-side restore.
    //
    // Decode failure is treated as miss: malformed bytes here are
    // either a serialisation-shape regression in upstream `ChainLock`
    // or a corrupted SwiftData row — neither is recoverable in-flight,
    // so log at `warn` (so the operator sees it) and continue with
    // `metadata.last_applied_chain_lock = None`. The next fresh
    // chainlock arrival will overwrite the field with a valid value
    // and the failure window for the metadata fallback is the SPV
    // catch-up latency, same as if the column had been empty.
    if !entry.last_applied_chain_lock_bytes.is_null() && entry.last_applied_chain_lock_bytes_len > 0
    {
        let cl_slice = unsafe {
            slice::from_raw_parts(
                entry.last_applied_chain_lock_bytes,
                entry.last_applied_chain_lock_bytes_len,
            )
        };
        match dpp::bincode::decode_from_slice::<dashcore::ephemerealdata::chain_lock::ChainLock, _>(
            cl_slice,
            dpp::bincode::config::standard(),
        ) {
            Ok((cl, _)) => {
                wallet_info.metadata.last_applied_chain_lock = Some(cl);
            }
            Err(e) => {
                tracing::warn!(
                    wallet_id = %hex::encode(entry.wallet_id),
                    error = %e,
                    "load: failed to decode persisted last_applied_chain_lock; \
                     metadata.last_applied_chain_lock left as None — the \
                     next fresh CLSig will repopulate"
                );
            }
        }
    }

    // Persisted core address pools → funds-bearing managed accounts.
    // `ManagedWalletInfo::from_wallet` only pre-derives the gap-limit
    // window from each account xpub; addresses past that window — and
    // every `used` flag — come from this snapshot. Without it a
    // restored wallet can hold a UTXO whose address the signer can't
    // map back to a derivation path, breaking core-to-core spends.
    {
        use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
        let pool_entries: &[AccountAddressPoolFFI] =
            if entry.core_address_pools.is_null() || entry.core_address_pools_count == 0 {
                &[]
            } else {
                unsafe {
                    slice::from_raw_parts(entry.core_address_pools, entry.core_address_pools_count)
                }
            };
        let mut pools_routed = 0usize;
        let mut pools_dropped = 0usize;
        for pool_ffi in pool_entries {
            let account_type = match account_type_from_spec(&pool_ffi.account) {
                Ok(t) => t,
                Err(e) => {
                    if is_legacy_removed_account_tag(pool_ffi.account.type_tag) {
                        pools_dropped += 1;
                        continue;
                    }
                    return Err(e);
                }
            };
            let pool_type = match pool_ffi.pool_type_tag {
                0 => AddressPoolType::External,
                1 => AddressPoolType::Internal,
                2 => AddressPoolType::Absent,
                3 => AddressPoolType::AbsentHardened,
                other => {
                    pools_dropped += 1;
                    tracing::warn!(
                        wallet_id = %hex::encode(entry.wallet_id),
                        pool_type_tag = other,
                        "load: skipping persisted address pool with invalid pool_type_tag"
                    );
                    continue;
                }
            };
            let funds = match account_type {
                AccountType::Standard {
                    index,
                    standard_account_type: StandardAccountType::BIP44Account,
                } => wallet_info.accounts.standard_bip44_accounts.get_mut(&index),
                AccountType::Standard {
                    index,
                    standard_account_type: StandardAccountType::BIP32Account,
                } => wallet_info.accounts.standard_bip32_accounts.get_mut(&index),
                AccountType::CoinJoin { index } => {
                    wallet_info.accounts.coinjoin_accounts.get_mut(&index)
                }
                AccountType::DashpayReceivingFunds {
                    index,
                    user_identity_id,
                    friend_identity_id,
                } => wallet_info.accounts.dashpay_receival_accounts.get_mut(
                    &key_wallet::account::account_collection::DashpayAccountKey {
                        index,
                        user_identity_id,
                        friend_identity_id,
                    },
                ),
                AccountType::DashpayExternalAccount {
                    index,
                    user_identity_id,
                    friend_identity_id,
                } => wallet_info.accounts.dashpay_external_accounts.get_mut(
                    &key_wallet::account::account_collection::DashpayAccountKey {
                        index,
                        user_identity_id,
                        friend_identity_id,
                    },
                ),
                _ => None,
            };
            let Some(funds_account) = funds else {
                pools_dropped += 1;
                tracing::warn!(
                    wallet_id = %hex::encode(entry.wallet_id),
                    ?account_type,
                    "load: skipping persisted address pool with no matching funds account"
                );
                continue;
            };
            let rows: &[CoreAddressEntryFFI] = if pool_ffi.addresses_ptr.is_null()
                || pool_ffi.addresses_count == 0
            {
                &[]
            } else {
                unsafe { slice::from_raw_parts(pool_ffi.addresses_ptr, pool_ffi.addresses_count) }
            };
            let mut infos: Vec<AddressInfo> = Vec::with_capacity(rows.len());
            for row in rows {
                match unsafe { address_info_from_ffi(row, network) } {
                    Ok(info) => infos.push(info),
                    Err(e) => {
                        pools_dropped += 1;
                        tracing::warn!(
                            wallet_id = %hex::encode(entry.wallet_id),
                            error = %e,
                            "load: skipping un-decodable persisted address row"
                        );
                    }
                }
            }
            let mut managed_pools = funds_account.managed_account_type_mut().address_pools_mut();
            match managed_pools.iter_mut().find(|p| p.pool_type == pool_type) {
                Some(pool) => {
                    pools_routed += infos.len();
                    restore_address_pool(pool, infos);
                }
                None => {
                    pools_dropped += 1;
                    tracing::warn!(
                        wallet_id = %hex::encode(entry.wallet_id),
                        ?pool_type,
                        "load: persisted address pool has no matching managed pool"
                    );
                }
            }
        }
        if pools_dropped > 0 {
            tracing::warn!(
                wallet_id = %hex::encode(entry.wallet_id),
                pools_routed,
                pools_dropped,
                "load: persisted address-pool restore completed with skipped rows"
            );
        }
    }

    // Persisted unspent UTXOs → funds-bearing accounts. Keys-only and
    // PlatformPayment variants are skipped: the former never carry
    // UTXOs, the latter route through `PlatformAddressSyncStartState`.
    // Each row is mapped from `(prev_txid, vout, script_pubkey,
    // value, height, flags)` into the target account's `utxos` map.
    let utxo_entries: &[UtxoRestoreEntryFFI] = if entry.utxos.is_null() || entry.utxos_count == 0 {
        &[]
    } else {
        unsafe { slice::from_raw_parts(entry.utxos, entry.utxos_count) }
    };
    // Track each skip reason separately so a non-zero `dropped` value
    // is debuggable without a native trace. The four categories have
    // very different operational meanings — corruption (bad txid /
    // unrenderable script), legitimate drift (no matching account),
    // and ABI-only-present-tag (unmappable type for keys-only / legacy
    // identity-auth rows). Each emits a `tracing::warn!` so a host
    // running a subscriber sees the breakdown in real time.
    let mut routed = 0usize;
    let mut dropped_account_type = 0usize;
    let mut dropped_bad_txid = 0usize;
    let mut dropped_bad_script = 0usize;
    let mut dropped_no_account = 0usize;
    for u in utxo_entries {
        // Bring `Hash` into scope locally so `Txid::from_slice` is
        // available — matches the pattern used elsewhere in this
        // crate (see e.g. asset_lock/sync.rs).
        use dashcore::hashes::Hash;
        let script_bytes = unsafe { slice_from_raw(u.script_pubkey, u.script_pubkey_len) };
        // Build the AccountType via the same helper the per-spec path
        // uses, repackaged as an `AccountSpecFFI` so we can reuse
        // `account_type_from_spec` (it ignores `account_xpub_bytes`).
        let spec = AccountSpecFFI {
            type_tag: u.type_tag,
            standard_tag: u.standard_tag,
            index: u.account_index,
            registration_index: u.registration_index,
            key_class: u.key_class,
            user_identity_id: u.user_identity_id,
            friend_identity_id: u.friend_identity_id,
            account_xpub_bytes: std::ptr::null(),
            account_xpub_bytes_len: 0,
            // Irrelevant to `account_type_from_spec` routing.
            derived_platform_node_keys: std::ptr::null(),
            derived_platform_node_keys_count: 0,
        };
        // Skip-and-continue is correct ONLY for the legacy
        // `IdentityAuthentication{Ecdsa,Bls}` tag bytes (15 / 16)
        // whose upstream `AccountType` variants were removed. Real
        // validation errors (out-of-range bytes from a corrupt
        // SwiftData row, etc.) propagate so the corruption surfaces
        // rather than silently under-restoring the UTXO set.
        let account_type = match account_type_from_spec(&spec) {
            Ok(t) => t,
            Err(e) => {
                if is_legacy_removed_account_tag(u.type_tag) {
                    dropped_account_type += 1;
                    tracing::warn!(
                        wallet_id = %hex::encode(entry.wallet_id),
                        type_tag = u.type_tag,
                        "load: skipping persisted UTXO on legacy IdentityAuthentication tag"
                    );
                    continue;
                }
                return Err(e);
            }
        };
        let Ok(txid) = dashcore::Txid::from_slice(&u.prev_txid) else {
            dropped_bad_txid += 1;
            tracing::warn!(
                wallet_id = %hex::encode(entry.wallet_id),
                "load: skipping persisted UTXO with malformed txid bytes"
            );
            continue;
        };
        let outpoint = dashcore::OutPoint { txid, vout: u.vout };
        let script_pubkey = dashcore::ScriptBuf::from_bytes(script_bytes.to_vec());
        let Ok(address) = dashcore::Address::from_script(&script_pubkey, network) else {
            dropped_bad_script += 1;
            tracing::warn!(
                wallet_id = %hex::encode(entry.wallet_id),
                txid = %txid,
                vout = u.vout,
                "load: skipping persisted UTXO with un-decodable script_pubkey"
            );
            continue;
        };
        let txout = dashcore::TxOut {
            value: u.value_duffs,
            script_pubkey,
        };
        let utxo = key_wallet::Utxo {
            outpoint,
            txout,
            address,
            height: u.height,
            is_coinbase: u.is_coinbase,
            is_confirmed: u.is_confirmed,
            is_instantlocked: u.is_instantlocked,
            is_locked: u.is_locked,
            // `is_trusted` is a runtime-only flag derived from the
            // tx graph (we created it ourselves and it pays back to
            // us). Recompute on the next SPV pass; default to false.
            is_trusted: false,
        };
        // Route into the target funds-bearing account. Match on the
        // resolved `AccountType` and look up the right map field. Keys
        // and Platform variants are intentionally no-ops.
        let target_funds = match account_type {
            AccountType::Standard {
                index,
                standard_account_type: StandardAccountType::BIP44Account,
            } => wallet_info.accounts.standard_bip44_accounts.get_mut(&index),
            AccountType::Standard {
                index,
                standard_account_type: StandardAccountType::BIP32Account,
            } => wallet_info.accounts.standard_bip32_accounts.get_mut(&index),
            AccountType::CoinJoin { index } => {
                wallet_info.accounts.coinjoin_accounts.get_mut(&index)
            }
            AccountType::DashpayReceivingFunds {
                index,
                user_identity_id,
                friend_identity_id,
            } => wallet_info.accounts.dashpay_receival_accounts.get_mut(
                &key_wallet::account::account_collection::DashpayAccountKey {
                    index,
                    user_identity_id,
                    friend_identity_id,
                },
            ),
            AccountType::DashpayExternalAccount {
                index,
                user_identity_id,
                friend_identity_id,
            } => wallet_info.accounts.dashpay_external_accounts.get_mut(
                &key_wallet::account::account_collection::DashpayAccountKey {
                    index,
                    user_identity_id,
                    friend_identity_id,
                },
            ),
            _ => None,
        };
        if let Some(funds_account) = target_funds {
            funds_account.utxos.insert(utxo.outpoint, utxo);
            routed += 1;
        } else {
            dropped_no_account += 1;
            tracing::warn!(
                wallet_id = %hex::encode(entry.wallet_id),
                ?account_type,
                "load: skipping persisted UTXO with no matching funds account in snapshot"
            );
        }
    }
    let dropped = dropped_account_type + dropped_bad_txid + dropped_bad_script + dropped_no_account;
    if dropped > 0 {
        // Surface a single rollup line so operators see the totals
        // even with `tracing` set to ERROR-only (the per-row warns
        // above are the breakdown).
        tracing::warn!(
            wallet_id = %hex::encode(entry.wallet_id),
            routed,
            dropped,
            dropped_account_type,
            dropped_bad_txid,
            dropped_bad_script,
            dropped_no_account,
            "load: persisted UTXO restore completed with skipped rows"
        );
    }

    // Recompute balances from the freshly-loaded UTXO set. Raw
    // `account.utxos.insert` bypasses the normal `record_transaction`
    // path that keeps the per-account `balance` field in sync, so
    // the per-account confirmed/unconfirmed/immature/locked totals
    // and the wallet-level rollup stay zero unless we tell the info
    // to reread them. `update_balance` walks every funds account
    // and recomputes from `utxos` against the wallet's
    // `metadata.synced_height` (passed through to
    // `ManagedCoreFundsAccount::update_balance` as the
    // `last_processed_height` parameter — that's the maturity
    // baseline upstream uses; the parameter naming is historical),
    // then sums into `wallet_info.balance`. The lock-free
    // `Arc<WalletBalance>` the UI reads is mirrored in
    // `manager::load::load_from_persistor` (`WalletBalance::set` is
    // `pub(crate)` to platform-wallet).
    if routed > 0 {
        wallet_info.update_balance();
    }

    // Selectively repopulate the in-memory `transactions()` map with
    // the funding-tx records of any tracked asset locks still at
    // `statusRaw < 2` (Built / Broadcast). The wallet's load path
    // otherwise leaves `transactions()` empty for these accounts —
    // most tx history is consumed reactively from SwiftData rather
    // than the in-memory map, so the bulk-restore cost isn't
    // justified. But asset locks waiting on a proof are the one
    // exception: `WalletManager::apply_chain_lock` walks the in-
    // memory `transactions()` map looking for records to promote
    // from `InBlock` to `InChainLockedBlock`. If the funding tx
    // isn't present at the moment the next CLSig fires, the
    // promotion silently drops on the floor and the asset lock
    // stays stuck at `Broadcast` indefinitely.
    //
    // Re-injecting these specific records is the minimum surface
    // that lets the existing event-driven cascade do its job: at
    // the next chain-lock event the bridge's Fix-1
    // `chain_lock_promotions` projection picks them up, Swift
    // flips their `PersistentTransaction.context` to `3`, and the
    // `AssetLockManager::resolve_status_with_in_memory` path
    // builds a `ChainAssetLockProof` from the row's block info.
    //
    // Each restored record is synthetic: `input_details` /
    // `output_details` are empty (we don't have the per-account
    // role classification at load time), `net_amount` is zero, and
    // `account_type` reflects the funding BIP44 slot. None of
    // those fields are read by the chain-lock cascade — only
    // `context` and `height()` matter for the
    // `apply_chain_lock` → bridge → persister loop.
    let unresolved_recs: &[UnresolvedAssetLockTxRecordFFI] =
        if entry.unresolved_asset_lock_tx_records.is_null()
            || entry.unresolved_asset_lock_tx_records_count == 0
        {
            &[]
        } else {
            unsafe {
                slice::from_raw_parts(
                    entry.unresolved_asset_lock_tx_records,
                    entry.unresolved_asset_lock_tx_records_count,
                )
            }
        };
    if !unresolved_recs.is_empty() {
        let stats = restore_unresolved_asset_lock_tx_records(&mut wallet_info, unresolved_recs)?;
        if stats.restored > 0 || stats.dropped() > 0 {
            tracing::info!(
                wallet_id = %hex::encode(entry.wallet_id),
                restored = stats.restored,
                dropped_decode = stats.dropped_decode,
                dropped_no_account = stats.dropped_no_account,
                "load: unresolved-asset-lock tx-record restore complete"
            );
        }
    }

    // TODO: this per-account reconstruction mirrors the SQLite backend's
    // `platform_addrs::build_per_account`. Deferred dedup — once a shared
    // helper crate hosts the reconstruction, both backends should call it
    // instead of keeping parallel copies.
    let mut per_account = PerWalletPlatformAddressState::new();
    for (&account_key, account) in &wallet.accounts.platform_payment_accounts {
        per_account.entry(account_key.account).or_insert_with(|| {
            PerAccountPlatformAddressState::from_persisted(
                account.account_xpub,
                Default::default(),
                Default::default(),
            )
        });
    }

    let platform_balance_entries: &[AddressBalanceEntryFFI] = if entry
        .platform_address_balances
        .is_null()
        || entry.platform_address_balances_count == 0
    {
        &[]
    } else {
        unsafe {
            slice::from_raw_parts(
                entry.platform_address_balances,
                entry.platform_address_balances_count,
            )
        }
    };
    let mut dropped_unknown_account = 0usize;
    let mut dropped_unsupported_address_type = 0usize;
    for persisted in platform_balance_entries {
        if persisted.address.address_type != 0 {
            // Non-P2PKH rows aren't supported on the persistence path
            // yet. Skip rather than abort the whole load — the next
            // platform-address sync will repopulate from authoritative
            // state.
            dropped_unsupported_address_type += 1;
            tracing::warn!(
                wallet_id = %hex::encode(entry.wallet_id),
                address_type = persisted.address.address_type,
                account_index = persisted.account_index,
                "load: skipping persisted platform-address row with unsupported address_type"
            );
            continue;
        }

        // `per_account` is built only from the reconstructed wallet's
        // platform-payment account map; the cached
        // `platform_address_balances` slice can include rows whose
        // referenced account didn't make it into the snapshot
        // (deleted, not-yet-hydrated, stale cache). Skip-and-warn so
        // a single drift row doesn't abort the whole `load()` — the
        // sync coordinator will recompute on the next pass.
        let Some(account_state) = per_account.get_mut(&persisted.account_index) else {
            dropped_unknown_account += 1;
            tracing::warn!(
                wallet_id = %hex::encode(entry.wallet_id),
                account_index = persisted.account_index,
                address_index = persisted.address_index,
                "load: skipping persisted platform-address row referencing unknown account"
            );
            continue;
        };
        let p2pkh = key_wallet::PlatformP2PKHAddress::new(persisted.address.hash);
        account_state.insert_persisted_entry(
            persisted.address_index,
            p2pkh,
            dash_sdk::platform::address_sync::AddressFunds {
                nonce: persisted.nonce,
                balance: persisted.balance,
                // Height pin round-trip: rows persisted before the pin
                // existed load as 0 ("unknown provenance") and yield to
                // the first pinned absolute — the self-healing path.
                as_of_height: persisted.as_of_height,
            },
        );
    }
    if dropped_unknown_account > 0 || dropped_unsupported_address_type > 0 {
        tracing::warn!(
            wallet_id = %hex::encode(entry.wallet_id),
            dropped_unknown_account,
            dropped_unsupported_address_type,
            "load: persisted platform-address rows skipped during restore"
        );
    }

    // Per-wallet identities go straight into the wallet_identities
    // sub-map keyed by registration index. Out-of-wallet identities
    // are not surfaced here — there's no SwiftData path for them
    // today (PersistentIdentity always links to a wallet) — so the
    // out-of-wallet bucket starts empty and is populated only via
    // runtime DPNS resolution / observation.
    let bucket = build_wallet_identity_bucket(entry)?;
    let mut wallet_identities = BTreeMap::new();
    if !bucket.is_empty() {
        wallet_identities.insert(entry.wallet_id, bucket);
    }
    let identity_manager = IdentityManagerStartState {
        out_of_wallet_identities: BTreeMap::new(),
        wallet_identities,
    };

    // Rehydrate tracked asset-locks (built / broadcast / IS-locked
    // / chain-locked credit outputs awaiting registration). These
    // rows are persisted by `on_persist_asset_locks_fn` whenever the
    // asset-lock manager flushes a status change, and the Swift load
    // path hands them back here so an in-flight registration that
    // was interrupted by an app kill can resume from the latest
    // status without rebroadcasting.
    let unused_asset_locks = build_unused_asset_locks(entry)?;

    let wallet_state = ClientWalletStartState {
        wallet,
        wallet_info,
        identity_manager,
        unused_asset_locks,
    };

    let platform_address_state = if per_account.is_empty()
        && entry.platform_sync_height == 0
        && entry.platform_sync_timestamp == 0
        && entry.platform_last_known_recent_block == 0
    {
        None
    } else {
        Some(platform_wallet::PlatformAddressSyncStartState {
            per_account,
            sync_height: entry.platform_sync_height,
            sync_timestamp: entry.platform_sync_timestamp,
            last_known_recent_block: entry.platform_last_known_recent_block,
        })
    };

    Ok((wallet_state, platform_address_state))
}

/// Translate the `IdentityRestoreEntryFFI` slice carried on a wallet
/// entry into the wallet-bucket portion of an
/// [`IdentityManagerStartState`].
///
/// Every entry on a `WalletRestoreEntryFFI` is wallet-owned by
/// definition, so the returned map is shaped for direct insertion
/// into `wallet_identities[entry.wallet_id]`. Out-of-wallet identities
/// (no associated wallet) come from a separate path that today simply
/// doesn't exist in SwiftData — see the report observation.
///
/// The DPP `Identity` is reconstructed from the persisted scalars via
/// the `IdentityV0` shape — same approach
/// [`apply_identity_entry`](platform_wallet::IdentityManager::apply_identity_entry)
/// uses on the changeset replay path. Public keys are now pulled in
/// from the `keys` array on each `IdentityRestoreEntryFFI` (assembled
/// from the per-identity `PersistentPublicKey` rows on the Swift
/// side), so the restored `Identity.public_keys` map is populated at
/// load time. An identity with no persisted keys (e.g. an in-flight
/// registration whose key-persist round hasn't completed) loads with
/// an empty map and gets refreshed on the next sync round —
/// degraded-but-usable for that narrow case.
/// Rebuild the `unused_asset_locks` map carried on
/// [`ClientWalletStartState`] from the `tracked_asset_locks` slice the
/// Swift load callback hands back. Mirrors the encoding used by
/// [`crate::asset_lock_persistence::build_asset_lock_entries`]:
///
/// - `out_point` is 32-byte raw txid + 4-byte little-endian vout.
/// - `transaction_bytes` is consensus-encoded.
/// - `proof_bytes` is bincode-encoded (`dpp::bincode::config::standard()`).
///   `null` / 0 length means "no proof yet" (statuses Built / Broadcast).
///
/// A malformed entry returns `Err(PersistenceError)` so the caller
/// surfaces the load failure rather than dropping a partially-rebuilt
/// state silently. Empty / null `tracked_asset_locks` yields an empty
/// map (same as the legacy hardcoded path).
fn build_unused_asset_locks(
    entry: &WalletRestoreEntryFFI,
) -> Result<
    BTreeMap<u32, BTreeMap<dashcore::OutPoint, platform_wallet::TrackedAssetLock>>,
    PersistenceError,
> {
    use dashcore::hashes::Hash;

    let specs: &[AssetLockEntryFFI] = if entry.tracked_asset_locks.is_null()
        || entry.tracked_asset_locks_count == 0
    {
        &[]
    } else {
        // SAFETY: Swift guarantees the pointer + count form a valid
        // slice for the callback window; this function runs inside
        // that window (called from `build_wallet_start_state` invoked
        // by `FFIPersister::load`).
        unsafe { slice::from_raw_parts(entry.tracked_asset_locks, entry.tracked_asset_locks_count) }
    };

    let mut map: BTreeMap<u32, BTreeMap<dashcore::OutPoint, platform_wallet::TrackedAssetLock>> =
        BTreeMap::new();
    for spec in specs {
        // Decode the outpoint: 32-byte raw txid + 4-byte LE vout.
        let txid = dashcore::Txid::from_slice(&spec.out_point[..32]).map_err(|e| {
            PersistenceError::backend(format!(
                "tracked asset lock: invalid txid in outpoint: {}",
                e
            ))
        })?;
        let vout_bytes: [u8; 4] = spec.out_point[32..]
            .try_into()
            .expect("4-byte slice from 36-byte array");
        let vout = u32::from_le_bytes(vout_bytes);
        let out_point = dashcore::OutPoint { txid, vout };

        // Decode the consensus-encoded transaction.
        if spec.transaction_bytes.is_null() || spec.transaction_bytes_len == 0 {
            return Err(PersistenceError::backend(
                "tracked asset lock: empty transaction bytes",
            ));
        }
        // SAFETY: Swift guarantees the buffer is valid for the
        // callback window. We immediately decode + clone out of it,
        // so the lifetime concern is satisfied.
        let tx_bytes =
            unsafe { slice::from_raw_parts(spec.transaction_bytes, spec.transaction_bytes_len) };
        let transaction: dashcore::Transaction = dashcore::consensus::deserialize(tx_bytes)
            .map_err(|e| {
                PersistenceError::backend(format!(
                    "tracked asset lock: failed to decode transaction: {}",
                    e
                ))
            })?;

        // Decode the optional bincode-encoded proof.
        let proof: Option<dpp::prelude::AssetLockProof> = if spec.proof_bytes.is_null()
            || spec.proof_bytes_len == 0
        {
            None
        } else {
            // SAFETY: Same lifetime contract as `transaction_bytes`.
            let proof_bytes =
                unsafe { slice::from_raw_parts(spec.proof_bytes, spec.proof_bytes_len) };
            let (proof, _) = dpp::bincode::decode_from_slice::<dpp::prelude::AssetLockProof, _>(
                proof_bytes,
                config::standard(),
            )
            .map_err(|e| {
                PersistenceError::backend(format!(
                    "tracked asset lock: failed to decode proof: {}",
                    e
                ))
            })?;
            Some(proof)
        };

        let funding_type = funding_type_from_u8(spec.funding_type)?;
        let status = status_from_u8(spec.status)?;

        // Skip `Consumed` rows. The Swift persister keeps them for
        // historical UI lookups (transactions list → locked amount),
        // but the in-memory `tracked_asset_locks` map is for
        // still-actionable locks only — a consumed lock has no proof
        // worth waiting on and adding it back to memory at every
        // load would defeat the point of marking it terminal.
        if matches!(status, platform_wallet::AssetLockStatus::Consumed) {
            continue;
        }

        let tracked = platform_wallet::TrackedAssetLock {
            out_point,
            transaction,
            account_index: spec.account_index,
            funding_type,
            identity_index: spec.identity_index,
            amount: spec.amount_duffs,
            status,
            proof,
        };
        map.entry(spec.account_index)
            .or_default()
            .insert(out_point, tracked);
    }

    Ok(map)
}

fn funding_type_from_u8(
    b: u8,
) -> Result<
    key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType,
    PersistenceError,
> {
    use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
    Ok(match b {
        0 => AssetLockFundingType::IdentityRegistration,
        1 => AssetLockFundingType::IdentityTopUp,
        2 => AssetLockFundingType::IdentityTopUpNotBound,
        3 => AssetLockFundingType::IdentityInvitation,
        4 => AssetLockFundingType::AssetLockAddressTopUp,
        5 => AssetLockFundingType::AssetLockShieldedAddressTopUp,
        other => {
            return Err(PersistenceError::backend(format!(
                "tracked asset lock: unknown funding_type discriminant {}",
                other
            )))
        }
    })
}

fn status_from_u8(b: u8) -> Result<platform_wallet::AssetLockStatus, PersistenceError> {
    use platform_wallet::AssetLockStatus;
    Ok(match b {
        0 => AssetLockStatus::Built,
        1 => AssetLockStatus::Broadcast,
        2 => AssetLockStatus::InstantSendLocked,
        3 => AssetLockStatus::ChainLocked,
        4 => AssetLockStatus::Consumed,
        other => {
            return Err(PersistenceError::backend(format!(
                "tracked asset lock: unknown status discriminant {}",
                other
            )))
        }
    })
}

fn build_wallet_identity_bucket(
    entry: &WalletRestoreEntryFFI,
) -> Result<BTreeMap<u32, ManagedIdentity>, PersistenceError> {
    let identity_specs: &[IdentityRestoreEntryFFI] =
        if entry.identities.is_null() || entry.identities_count == 0 {
            &[]
        } else {
            unsafe { slice::from_raw_parts(entry.identities, entry.identities_count) }
        };

    let mut bucket: BTreeMap<u32, ManagedIdentity> = BTreeMap::new();

    for spec in identity_specs {
        let identifier = Identifier::from(spec.identity_id);
        let public_keys = unsafe { build_identity_public_keys(spec) };
        let identity = Identity::V0(IdentityV0 {
            id: identifier,
            public_keys,
            balance: spec.balance,
            revision: spec.revision,
        });
        let status = identity_status_from_tag(spec.status);
        let dpns_names = unsafe {
            c_string_array_to_vec(spec.dpns_names, spec.dpns_names_count)
                .into_iter()
                .map(|label| DpnsNameInfo {
                    label,
                    acquired_at: None,
                })
                .collect::<Vec<_>>()
        };
        let contested_dpns_names = unsafe {
            c_string_array_to_vec(spec.contested_dpns_names, spec.contested_dpns_names_count)
        };

        let mut managed = ManagedIdentity::new(identity, spec.identity_index);
        managed.status = status;
        managed.wallet_id = Some(entry.wallet_id);
        managed.dpns_names = dpns_names;
        managed.contested_dpns_names = contested_dpns_names;
        unsafe { restore_dashpay_contacts(spec, &identifier, &mut managed) };
        unsafe { restore_dashpay_payments(spec, &mut managed) };
        unsafe { restore_dashpay_ignored(spec, &mut managed) };
        unsafe { restore_contact_profiles(spec, &mut managed) };
        bucket.insert(spec.identity_index, managed);
    }

    Ok(bucket)
}

/// Rebuild the per-identity DashPay payment history (`dashpay_payments`)
/// from the persisted SwiftData rows at load (H1).
///
/// Without this the in-memory map starts empty and only *Received*
/// entries are re-derived from UTXOs by the reconcile sweep, so *Sent*
/// entries (with their user memos) vanish from the authoritative model
/// on every relaunch. Direct map inserts, NO persister round — the rows
/// ARE the persisted state.
///
/// # Safety
///
/// `spec.payments` must be either null or point at `spec.payments_count`
/// valid `PaymentRestoreEntryFFI` rows whose `txid`/`memo` c-strings
/// Swift owns for the duration of the load callback.
unsafe fn restore_dashpay_payments(spec: &IdentityRestoreEntryFFI, managed: &mut ManagedIdentity) {
    if spec.payments.is_null() || spec.payments_count == 0 {
        return;
    }
    let rows = slice::from_raw_parts(spec.payments, spec.payments_count);
    apply_payment_rows(rows, managed);
}

/// Rebuild the per-identity ignored-sender set (`ignored_senders`) from
/// the persisted rows at load.
///
/// Without this the ignore set starts empty on every relaunch, so a
/// previously-ignored sender's still-on-platform immutable
/// `contactRequest` documents re-ingest on the next sync sweep and the
/// ignored sender resurfaces. Direct set inserts, NO persister round —
/// the rows ARE the persisted state. Much simpler than the contact-row
/// restore: each row is a bare 32-byte sender id (the host only persists
/// senders that are currently ignored, so un-ignored ones simply don't
/// appear here).
///
/// # Safety
///
/// `spec.ignored_senders` must be either null or point at
/// `spec.ignored_senders_count` valid `[u8; 32]` id arrays.
unsafe fn restore_dashpay_ignored(spec: &IdentityRestoreEntryFFI, managed: &mut ManagedIdentity) {
    if spec.ignored_senders.is_null() || spec.ignored_senders_count == 0 {
        return;
    }
    let rows = slice::from_raw_parts(spec.ignored_senders, spec.ignored_senders_count);
    apply_ignored_rows(rows, managed);
}

/// Fold a slice of 32-byte sender ids into the managed identity's ignored set.
/// Split out from [`restore_dashpay_ignored`] so the decode is
/// unit-testable without a full `IdentityRestoreEntryFFI`.
fn apply_ignored_rows(rows: &[[u8; 32]], managed: &mut ManagedIdentity) {
    for row in rows {
        managed.apply_ignored_sender(Identifier::from(*row));
    }
}

/// Fold a slice of [`PaymentRestoreEntryFFI`] rows into
/// the managed identity's payments map. Split out from [`restore_dashpay_payments`]
/// so the discriminant mapping + c-string decode is unit-testable
/// without a full `IdentityRestoreEntryFFI`.
///
/// # Safety
/// Each row's `txid`/`memo` pointers must be null or point at valid
/// NUL-terminated c-strings for the call's duration.
unsafe fn apply_payment_rows(rows: &[PaymentRestoreEntryFFI], managed: &mut ManagedIdentity) {
    use platform_wallet::wallet::identity::{PaymentDirection, PaymentEntry, PaymentStatus};

    for row in rows {
        if row.txid.is_null() {
            continue;
        }
        let txid = match std::ffi::CStr::from_ptr(row.txid).to_str() {
            Ok(s) => s.to_string(),
            Err(_) => continue,
        };
        let direction = match row.direction_raw {
            0 => PaymentDirection::Sent,
            1 => PaymentDirection::Received,
            other => {
                tracing::warn!(
                    direction = other,
                    "skipping payment row with unknown direction"
                );
                continue;
            }
        };
        let status = match row.status_raw {
            0 => PaymentStatus::Pending,
            1 => PaymentStatus::Confirmed,
            2 => PaymentStatus::Failed,
            other => {
                tracing::warn!(status = other, "skipping payment row with unknown status");
                continue;
            }
        };
        let memo = if row.memo.is_null() {
            None
        } else {
            CStr::from_ptr(row.memo).to_str().ok().map(str::to_string)
        };
        managed.dashpay_payments_mut().insert(
            txid,
            PaymentEntry {
                counterparty_id: Identifier::from(row.counterparty_id),
                amount_duffs: row.amount_duffs,
                memo,
                direction,
                status,
            },
        );
    }
}

/// Rebuild the per-identity cached **contact** profiles
/// (`contact_profiles`) from the persisted SwiftData rows at load.
///
/// Without this the contact-profile cache starts empty on every
/// relaunch, so the requests/contacts UI shows raw identity ids until
/// the next profile sweep re-fetches every contact — a visible
/// cold-start flicker plus write amplification. Direct map inserts, NO
/// persister round — the rows ARE the persisted state. Only **present**
/// profiles are persisted, so every restored entry rebuilds as
/// `ContactProfileEntry { profile: Some(..), checked_at_ms }`; the
/// confirmed-absent negative cache rebuilds on the next sweep.
///
/// # Safety
///
/// `spec.contact_profiles` must be either null or point at
/// `spec.contact_profiles_count` valid [`ContactProfileRestoreEntryFFI`]
/// rows whose four c-strings Swift owns for the duration of the load
/// callback.
unsafe fn restore_contact_profiles(spec: &IdentityRestoreEntryFFI, managed: &mut ManagedIdentity) {
    if spec.contact_profiles.is_null() || spec.contact_profiles_count == 0 {
        return;
    }
    let rows = slice::from_raw_parts(spec.contact_profiles, spec.contact_profiles_count);
    apply_contact_profile_rows(rows, managed);
}

/// Maximum cached `avatarUrl` length — mirrors the
/// `MAX_AVATAR_URL_LEN` gate `platform-wallet`'s profile fetch applies
/// before caching (DIP-15's 2048-char cap).
const MAX_AVATAR_URL_LEN: usize = 2048;

/// Defensive re-validation of a cached `avatarUrl` at restore. The
/// fetch path already dropped non-`https://` / over-length URLs before
/// caching ([`platform_wallet`]'s `is_valid_avatar_url`), but the URL is
/// attacker-controlled public data and the UI will load it, so we
/// re-apply the same `https://`-only, length-capped rule on the way back
/// in. A URL that fails is dropped to `None` (the rest of the profile is
/// still restored) rather than discarding the whole row.
fn is_valid_avatar_url(url: &str) -> bool {
    !url.is_empty() && url.len() <= MAX_AVATAR_URL_LEN && url.starts_with("https://")
}

/// Fold a slice of [`ContactProfileRestoreEntryFFI`] rows into
/// the managed identity's contact-profile cache. Split out from
/// [`restore_contact_profiles`] so the c-string decode + avatar-url
/// re-validation is unit-testable without a full
/// [`IdentityRestoreEntryFFI`].
///
/// # Safety
/// Each row's four string pointers must be null or point at valid
/// NUL-terminated c-strings for the call's duration.
unsafe fn apply_contact_profile_rows(
    rows: &[ContactProfileRestoreEntryFFI],
    managed: &mut ManagedIdentity,
) {
    use platform_wallet::{ContactProfileEntry, DashPayProfile};

    let opt_string = |ptr: *const std::os::raw::c_char| -> Option<String> {
        if ptr.is_null() {
            None
        } else {
            CStr::from_ptr(ptr).to_str().ok().map(str::to_string)
        }
    };

    for row in rows {
        let avatar_hash = if row.avatar_hash_present {
            Some(row.avatar_hash)
        } else {
            None
        };
        let avatar_fingerprint = if row.avatar_fingerprint_present {
            Some(row.avatar_fingerprint)
        } else {
            None
        };
        // Re-validate the public, attacker-controlled avatar URL; drop
        // just the URL field (keep the rest of the profile) if it no
        // longer passes the `https://` / length rule.
        let avatar_url = opt_string(row.avatar_url).filter(|u| is_valid_avatar_url(u));

        managed.dashpay_contact_profiles_mut().insert(
            Identifier::from(row.contact_id),
            ContactProfileEntry {
                profile: Some(DashPayProfile {
                    display_name: opt_string(row.display_name),
                    bio: opt_string(row.bio),
                    avatar_url,
                    avatar_hash,
                    avatar_fingerprint,
                    public_message: opt_string(row.public_message),
                }),
                checked_at_ms: row.checked_at_ms,
            },
        );
    }
}

/// Rebuild the per-identity DashPay contact state from the SwiftData
/// contact rows the load callback hands back: pending sent / incoming
/// requests, and established contacts (a pair of rows per contact —
/// one per direction) with their owner-private metadata
/// (alias / note / hidden — contactInfo, M3) and broken-channel flag.
///
/// Direct map inserts, NO persister rounds — this runs inside `load()`
/// and the rows ARE the persisted state. Without this restore,
/// contacts only re-derive from chain on the first sync sweep, which
/// (a) leaves the Contacts UI empty on offline launches and (b) wipes
/// contactInfo metadata during the DIP-15 deferred-publish window:
/// the re-establish round emitted `alias = None` over the SwiftData
/// rows (the M3 part-3 relaunch-durability gap).
///
/// # Safety
///
/// `spec.contacts` must be either null or point at
/// `spec.contacts_count` valid `ContactRequestFFI` rows whose byte
/// buffers and strings Swift owns for the duration of the load
/// callback.
unsafe fn restore_dashpay_contacts(
    spec: &IdentityRestoreEntryFFI,
    owner_id: &Identifier,
    managed: &mut ManagedIdentity,
) {
    if spec.contacts.is_null() || spec.contacts_count == 0 {
        return;
    }
    let rows = slice::from_raw_parts(spec.contacts, spec.contacts_count);
    apply_contact_rows(rows, owner_id, managed);
}

/// Pair the per-direction [`ContactRequestFFI`] rows back into the
/// `ManagedIdentity`'s `sent` / `incoming` / `established` contact maps.
/// Extracted from [`restore_dashpay_contacts`] so the FFI→Rust decode is
/// unit-testable against rows built by the persist constructors (the other
/// restore families have the same `apply_*_rows` seam). The persist side is
/// tested field-by-field; this is the read-back half — where a dropped
/// optional or a swapped key index would otherwise be invisible.
///
/// # Safety
/// Each row's `*_ptr`/`*_len` byte buffers and C strings must be valid for
/// the call (Swift owns them during load; tests own them via the persist
/// constructors).
unsafe fn apply_contact_rows(
    rows: &[ContactRequestFFI],
    owner_id: &Identifier,
    managed: &mut ManagedIdentity,
) {
    use platform_wallet::{ContactRequest, EstablishedContact};

    /// Per-contact accumulator while pairing the direction rows.
    #[derive(Default)]
    struct PairAccumulator {
        outgoing: Option<ContactRequest>,
        incoming: Option<ContactRequest>,
        payment_channel_broken: bool,
        alias: Option<String>,
        note: Option<String>,
        is_hidden: bool,
        contact_account_label: Option<String>,
        accepted_accounts: Vec<u32>,
    }

    let opt_string = |ptr: *const std::os::raw::c_char| -> Option<String> {
        if ptr.is_null() {
            None
        } else {
            Some(std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned())
        }
    };
    let opt_bytes = |ptr: *const u8, len: usize| -> Option<Vec<u8>> {
        if ptr.is_null() || len == 0 {
            None
        } else {
            Some(slice::from_raw_parts(ptr, len).to_vec())
        }
    };
    let u32s = |ptr: *const u32, len: usize| -> Vec<u32> {
        if ptr.is_null() || len == 0 {
            Vec::new()
        } else {
            slice::from_raw_parts(ptr, len).to_vec()
        }
    };

    let mut by_contact: BTreeMap<[u8; 32], PairAccumulator> = BTreeMap::new();
    for row in rows {
        let contact_id = Identifier::from(row.contact_id);
        let (sender_id, recipient_id) = if row.is_outgoing {
            (*owner_id, contact_id)
        } else {
            (contact_id, *owner_id)
        };
        let mut request = ContactRequest::new(
            sender_id,
            recipient_id,
            row.sender_key_index,
            row.recipient_key_index,
            row.account_reference,
            opt_bytes(row.encrypted_public_key, row.encrypted_public_key_len).unwrap_or_default(),
            row.core_height_created_at,
            row.created_at,
        );
        request.encrypted_account_label =
            opt_bytes(row.encrypted_account_label, row.encrypted_account_label_len);
        request.auto_accept_proof = opt_bytes(row.auto_accept_proof, row.auto_accept_proof_len);

        let acc = by_contact.entry(row.contact_id).or_default();
        if row.is_outgoing {
            acc.outgoing = Some(request);
        } else {
            acc.incoming = Some(request);
            // The contact's account label is direction-specific — it rides
            // ONLY the incoming row, so take it from that row (never the
            // outgoing one, which may carry a label we sent).
            acc.contact_account_label = opt_string(row.contact_account_label);
        }
        // Relationship-level properties are replicated onto both rows
        // by the persist projection; OR / first-non-null is the safe
        // re-fold.
        acc.payment_channel_broken |= row.payment_channel_broken;
        acc.is_hidden |= row.is_hidden;
        if acc.alias.is_none() {
            acc.alias = opt_string(row.alias);
        }
        if acc.note.is_none() {
            acc.note = opt_string(row.note);
        }
        // Relationship-level, replicated onto both rows — take the first
        // non-empty projection.
        if acc.accepted_accounts.is_empty() {
            acc.accepted_accounts = u32s(row.accepted_accounts, row.accepted_accounts_len);
        }
    }

    for (contact_id_bytes, acc) in by_contact {
        let contact_id = Identifier::from(contact_id_bytes);
        match (acc.outgoing, acc.incoming) {
            (Some(outgoing), Some(incoming)) => {
                let mut contact = EstablishedContact::new(contact_id, outgoing, incoming);
                contact.alias = acc.alias;
                contact.note = acc.note;
                contact.is_hidden = acc.is_hidden;
                contact.payment_channel_broken = acc.payment_channel_broken;
                contact.contact_account_label = acc.contact_account_label;
                contact.accepted_accounts = acc.accepted_accounts;
                managed.apply_established_contact(contact);
            }
            (Some(outgoing), None) => {
                managed.apply_sent_contact_request(outgoing);
            }
            (None, Some(incoming)) => {
                managed.apply_incoming_contact_request(incoming);
            }
            (None, None) => unreachable!("accumulator entries always hold at least one row"),
        }
    }
}

/// Translate the `keys` array hanging off an `IdentityRestoreEntryFFI`
/// into a `BTreeMap<KeyID, IdentityPublicKey>` ready to drop into
/// `IdentityV0.public_keys`.
///
/// Rows whose `key_type`, `purpose` or `security_level` discriminant
/// doesn't decode are skipped silently (forward-compatibility with
/// future enum variants on the Rust side); rows with null `data`
/// pointers or zero `data_len` are likewise skipped — neither is
/// recoverable, and the only consequence of skipping is the
/// auth-key-gate fallback to "fetch from chain on next sync".
///
/// # Safety
///
/// `spec.keys` must be either null or point at `spec.keys_count`
/// valid `IdentityKeyRestoreFFI` rows for the duration of the load
/// callback. Each row's `data` pointer must be either null or point
/// at `data_len` bytes Swift owns for the same window.
unsafe fn build_identity_public_keys(
    spec: &IdentityRestoreEntryFFI,
) -> BTreeMap<KeyID, IdentityPublicKey> {
    use dpp::identity::identity_public_key::contract_bounds::ContractBounds;
    let mut map: BTreeMap<KeyID, IdentityPublicKey> = BTreeMap::new();
    if spec.keys.is_null() || spec.keys_count == 0 {
        return map;
    }
    let rows: &[IdentityKeyRestoreFFI] = slice::from_raw_parts(spec.keys, spec.keys_count);
    for row in rows {
        let Ok(key_type) = KeyType::try_from(row.key_type) else {
            continue;
        };
        let Ok(purpose) = Purpose::try_from(row.purpose) else {
            continue;
        };
        let Ok(security_level) = SecurityLevel::try_from(row.security_level) else {
            continue;
        };
        if row.data.is_null() || row.data_len == 0 {
            continue;
        }
        let bytes: Vec<u8> = slice::from_raw_parts(row.data, row.data_len).to_vec();

        // Reconstruct the ContractBounds variant from the kind tag
        // + id + optional doc-type C-string trio. Mirrors the
        // encoding in `IdentityKeyEntryFFI::from_entry`. A kind=2
        // row with a null doc-type pointer is an FFI-side
        // inconsistency (the writer is supposed to demote to
        // kind=1 in that case — see identity_persistence.rs); we
        // demote it here too rather than fabricating an empty doc-
        // type name. Invalid kind tags load as unbounded so a
        // forward-compatible writer doesn't lock us out.
        let contract_bounds: Option<ContractBounds> = match row.contract_bounds_kind {
            0 => None,
            1 => Some(ContractBounds::SingleContract {
                id: row.contract_bounds_id.into(),
            }),
            2 => {
                if row.contract_bounds_document_type.is_null() {
                    Some(ContractBounds::SingleContract {
                        id: row.contract_bounds_id.into(),
                    })
                } else {
                    match CStr::from_ptr(row.contract_bounds_document_type).to_str() {
                        Ok(name) => Some(ContractBounds::SingleContractDocumentType {
                            id: row.contract_bounds_id.into(),
                            document_type_name: name.to_string(),
                        }),
                        Err(_) => Some(ContractBounds::SingleContract {
                            id: row.contract_bounds_id.into(),
                        }),
                    }
                }
            }
            _ => None,
        };

        let pk = IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: row.key_id,
            purpose,
            security_level,
            contract_bounds,
            key_type,
            read_only: row.read_only,
            data: BinaryData::new(bytes),
            disabled_at: None,
        });
        map.insert(row.key_id, pk);
    }
    map
}

/// Decode a flat `*const *const c_char` array into a `Vec<String>`.
///
/// Used for the per-identity DPNS / contested-DPNS label arrays. The
/// outer pointer + count come from Swift; each inner pointer is a
/// NUL-terminated UTF-8 c-string. Inner entries that are null or
/// invalid UTF-8 are skipped (same forgiveness as
/// [`c_string_to_option`]).
///
/// # Safety
///
/// `ptr` must be either null or point at `count` valid `*const c_char`
/// pointers Swift owns for the duration of the load callback. Each
/// non-null inner pointer must reference NUL-terminated UTF-8 bytes.
unsafe fn c_string_array_to_vec(
    ptr: *const *const std::os::raw::c_char,
    count: usize,
) -> Vec<String> {
    if ptr.is_null() || count == 0 {
        return Vec::new();
    }
    let raw = slice::from_raw_parts(ptr, count);
    raw.iter()
        .filter_map(|inner| {
            if inner.is_null() {
                None
            } else {
                CStr::from_ptr(*inner).to_str().ok().map(str::to_owned)
            }
        })
        .collect()
}

/// Map the [`IdentityRestoreEntryFFI::status`] discriminant onto
/// `IdentityStatus`. Mirrors the encoding in
/// [`crate::identity_persistence::status_discriminant`]; unknown
/// discriminants degrade to `IdentityStatus::Unknown` rather than
/// erroring, so a forward-compatible Swift writer doesn't lock
/// rollback.
fn identity_status_from_tag(tag: u8) -> IdentityStatus {
    match tag {
        1 => IdentityStatus::PendingCreation,
        2 => IdentityStatus::Active,
        3 => IdentityStatus::FailedCreation,
        4 => IdentityStatus::NotFound,
        _ => IdentityStatus::Unknown,
    }
}

fn account_type_from_spec(spec: &AccountSpecFFI) -> Result<AccountType, PersistenceError> {
    // Validate the foreign byte before matching — `spec.type_tag` and
    // `spec.standard_tag` are now plain `u8` on the FFI surface
    // (previously typed as `repr(u8)` enum fields, which would have
    // been UB for out-of-range bytes from a corrupt SwiftData row /
    // forward-versioned tag / malformed host buffer).
    let type_tag = AccountTypeTagFFI::try_from_u8(spec.type_tag).ok_or_else(|| {
        PersistenceError::backend(format!(
            "AccountSpecFFI carries unknown type_tag byte {} (out of declared range)",
            spec.type_tag
        ))
    })?;
    Ok(match type_tag {
        AccountTypeTagFFI::Standard => {
            let standard_tag = StandardAccountTypeTagFFI::try_from_u8(spec.standard_tag)
                .ok_or_else(|| {
                    PersistenceError::backend(format!(
                        "AccountSpecFFI(Standard) carries unknown standard_tag byte {}",
                        spec.standard_tag
                    ))
                })?;
            let standard_account_type = match standard_tag {
                StandardAccountTypeTagFFI::Bip44 => StandardAccountType::BIP44Account,
                StandardAccountTypeTagFFI::Bip32 => StandardAccountType::BIP32Account,
            };
            AccountType::Standard {
                index: spec.index,
                standard_account_type,
            }
        }
        AccountTypeTagFFI::CoinJoin => AccountType::CoinJoin { index: spec.index },
        AccountTypeTagFFI::IdentityRegistration => AccountType::IdentityRegistration,
        AccountTypeTagFFI::IdentityTopUp => AccountType::IdentityTopUp {
            registration_index: spec.registration_index,
        },
        AccountTypeTagFFI::IdentityTopUpNotBoundToIdentity => {
            AccountType::IdentityTopUpNotBoundToIdentity
        }
        AccountTypeTagFFI::IdentityInvitation => AccountType::IdentityInvitation,
        AccountTypeTagFFI::AssetLockAddressTopUp => AccountType::AssetLockAddressTopUp,
        AccountTypeTagFFI::AssetLockShieldedAddressTopUp => {
            AccountType::AssetLockShieldedAddressTopUp
        }
        AccountTypeTagFFI::ProviderVotingKeys => AccountType::ProviderVotingKeys,
        AccountTypeTagFFI::ProviderOwnerKeys => AccountType::ProviderOwnerKeys,
        AccountTypeTagFFI::ProviderOperatorKeys => AccountType::ProviderOperatorKeys,
        AccountTypeTagFFI::ProviderPlatformKeys => AccountType::ProviderPlatformKeys,
        AccountTypeTagFFI::DashpayReceivingFunds => AccountType::DashpayReceivingFunds {
            index: spec.index,
            user_identity_id: spec.user_identity_id,
            friend_identity_id: spec.friend_identity_id,
        },
        AccountTypeTagFFI::DashpayExternalAccount => AccountType::DashpayExternalAccount {
            index: spec.index,
            user_identity_id: spec.user_identity_id,
            friend_identity_id: spec.friend_identity_id,
        },
        AccountTypeTagFFI::PlatformPayment => AccountType::PlatformPayment {
            account: spec.index,
            key_class: spec.key_class,
        },
        // TODO(events): the upstream `AccountType::IdentityAuthentication*`
        // variants were removed in the event-bus refactor. The FFI ABI
        // still surfaces the tags for backwards compatibility, so a
        // Swift caller passing one back through the load path needs a
        // new mapping target. Until the identity-key derivation moves
        // off `AccountType` entirely, fail loudly so we don't silently
        // pretend a record is restorable when its derivation context
        // is gone.
        AccountTypeTagFFI::IdentityAuthenticationEcdsa
        | AccountTypeTagFFI::IdentityAuthenticationBls => {
            return Err(PersistenceError::backend(format!(
                "AccountTypeTagFFI {:?} is no longer mappable to a key-wallet AccountType after the upstream event-bus refactor (TODO(events))",
                type_tag
            )));
        }
    })
}

/// Returns `true` for the ABI-only `IdentityAuthentication{Ecdsa,Bls}`
/// tag bytes whose upstream `AccountType` variants were removed
/// (TODO(events)). These are the only tags `account_type_from_spec`
/// deliberately returns `Err` for while still being valid
/// discriminants — callers use this predicate to distinguish
/// "recoverable drift" (warn + continue) from "real corruption /
/// out-of-range byte" (propagate the error).
fn is_legacy_removed_account_tag(type_tag: u8) -> bool {
    type_tag == AccountTypeTagFFI::IdentityAuthenticationEcdsa as u8
        || type_tag == AccountTypeTagFFI::IdentityAuthenticationBls as u8
}

/// Read `len` bytes from a Swift-owned pointer as a `&[u8]`.
///
/// # Safety
///
/// `ptr` must point to at least `len` valid bytes for the duration of
/// the callback. Caller holds the callback window open via
/// `LoadGuard`.
unsafe fn slice_from_raw<'a>(ptr: *const u8, len: usize) -> &'a [u8] {
    if ptr.is_null() || len == 0 {
        &[]
    } else {
        slice::from_raw_parts(ptr, len)
    }
}

/// Per-call statistics for [`restore_unresolved_asset_lock_tx_records`].
/// Pulled out as a struct so the caller logs a single rollup line and
/// the unit tests can assert on the breakdown without ad-hoc tuples.
#[derive(Debug, Default, PartialEq, Eq)]
struct UnresolvedRestoreStats {
    restored: usize,
    dropped_decode: usize,
    dropped_no_account: usize,
}

impl UnresolvedRestoreStats {
    fn dropped(&self) -> usize {
        self.dropped_decode + self.dropped_no_account
    }
}

/// Project a slice of [`UnresolvedAssetLockTxRecordFFI`] rows onto the
/// in-memory `transactions()` maps of the matching
/// `standard_bip44_accounts[account_index]` slots on the rebuilt
/// `ManagedWalletInfo`.
///
/// See the call site in [`build_wallet_start_state`] for the design
/// rationale on WHY this exists at all (selective bulk-restore for
/// the chain-lock cascade path). This helper is the pure
/// computational core, separated so a Rust unit test can exercise it
/// without standing up an entire `WalletRestoreEntryFFI`.
///
/// Returns an `Err` only for non-recoverable corruption (malformed
/// `block_hash`). Per-row decode failures and no-matching-account
/// rows are counted into `UnresolvedRestoreStats` so the caller can
/// emit a single rollup log line.
fn restore_unresolved_asset_lock_tx_records(
    wallet_info: &mut ManagedWalletInfo,
    records: &[UnresolvedAssetLockTxRecordFFI],
) -> Result<UnresolvedRestoreStats, PersistenceError> {
    use dashcore::hashes::Hash;
    use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
    use key_wallet::managed_account::transaction_record::{
        TransactionDirection, TransactionRecord,
    };
    use key_wallet::transaction_checking::{BlockInfo, TransactionContext, TransactionType};

    let mut stats = UnresolvedRestoreStats::default();
    for rec in records {
        let tx_bytes = unsafe { slice_from_raw(rec.tx_bytes, rec.tx_bytes_len) };
        let tx: dashcore::Transaction = match dashcore::consensus::encode::deserialize(tx_bytes) {
            Ok(t) => t,
            Err(e) => {
                stats.dropped_decode += 1;
                tracing::warn!(
                    account_index = rec.account_index,
                    error = %e,
                    "load: skipping unresolved-asset-lock tx record with undecodable bytes"
                );
                continue;
            }
        };

        // Only the two confirmed contexts are reconstructible from
        // the persisted scalars; `0` / `1` either have no block
        // info to project or need an IS-lock signature blob we
        // don't carry. Treat them as `Mempool` — defensive code
        // for an edge that shouldn't occur in practice (an asset
        // lock at `Built` / `Broadcast` has by definition not yet
        // observed IS-lock or block confirmation).
        let context = match rec.context_raw {
            2 => {
                let block_hash = dashcore::BlockHash::from_slice(&rec.block_hash).map_err(|e| {
                    PersistenceError::backend(format!(
                        "load: malformed block_hash on unresolved asset-lock tx record: {}",
                        e
                    ))
                })?;
                TransactionContext::InBlock(BlockInfo::new(
                    rec.block_height,
                    block_hash,
                    rec.block_timestamp as u32,
                ))
            }
            3 => {
                let block_hash = dashcore::BlockHash::from_slice(&rec.block_hash).map_err(|e| {
                    PersistenceError::backend(format!(
                        "load: malformed block_hash on unresolved asset-lock tx record: {}",
                        e
                    ))
                })?;
                TransactionContext::InChainLockedBlock(BlockInfo::new(
                    rec.block_height,
                    block_hash,
                    rec.block_timestamp as u32,
                ))
            }
            _ => TransactionContext::Mempool,
        };

        // Asset-lock txs are funded from a BIP44 account; that's
        // the only account map the asset-lock recovery flow
        // consults (`recover_asset_lock_blocking` reads
        // `info.core_wallet.accounts.standard_bip44_accounts.get(
        // &account_index)...transactions().get(&out_point.txid)`),
        // so restoration goes through the same map. Records for
        // other variants would never be reached by that lookup.
        let Some(account) = wallet_info
            .accounts
            .standard_bip44_accounts
            .get_mut(&rec.account_index)
        else {
            stats.dropped_no_account += 1;
            tracing::warn!(
                account_index = rec.account_index,
                "load: dropping unresolved-asset-lock tx record — no matching BIP44 account"
            );
            continue;
        };

        let account_type = account.managed_account_type().to_account_type();
        let record = TransactionRecord::new(
            tx,
            account_type,
            context,
            // Funding transactions ARE asset locks by definition —
            // the upstream router classifies them via the
            // `AssetLockPayloadType` special-tx payload. Use the
            // same tag here so any downstream code keying off
            // `transaction_type` sees the canonical value.
            TransactionType::AssetLock,
            // The funding flow always starts from our own UTXOs
            // and writes one credit output to ourselves; per
            // `TransactionDirection::Internal`'s docstring, a
            // self-transfer with no outputs to external addresses
            // is "Internal". `wait_for_proof` doesn't read
            // direction; this is just the most-correct tag.
            TransactionDirection::Internal,
            Vec::new(),
            Vec::new(),
            0,
        );
        account.transactions_mut().insert(record.txid, record);
        stats.restored += 1;
    }
    Ok(stats)
}

#[cfg(test)]
mod tests {
    //! Unit tests for the load-side helpers. Focused on the
    //! restoration loops that don't need the full FFI plumbing —
    //! exercising the in-memory mutation against synthetic input.

    use super::*;
    use dashcore::blockdata::transaction::txin::TxIn;
    use dashcore::blockdata::transaction::txout::TxOut;
    use dashcore::blockdata::transaction::Transaction;
    use dashcore::consensus::encode::serialize;
    use dashcore::secp256k1::Secp256k1;
    use dashcore::{Network, ScriptBuf};
    use key_wallet::account::{Account, AccountType, StandardAccountType};
    use key_wallet::bip32::{ExtendedPrivKey, ExtendedPubKey};
    use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
    use key_wallet::mnemonic::{Language, Mnemonic};
    use key_wallet::wallet::Wallet;

    /// Regression: restored pool addresses must be tagged with the
    /// WALLET's network, not the network the base58 string parses as.
    /// Devnet shares testnet's base58 prefixes, so a devnet wallet's
    /// persisted "y…" address parses as Testnet, and `require_network`
    /// only validates — it keeps the as-parsed tag. `Address` equality
    /// includes the network, so a Testnet-tagged restored key misses
    /// every runtime lookup built via `Address::from_script(script,
    /// Devnet)`: outputs paying restored addresses were matched
    /// (script-keyed `contains_script_pub_key` hits) but never credited
    /// as UTXOs (`get_address_info` missed) — a restored devnet wallet
    /// permanently lost the change of its own transactions.
    #[test]
    fn restored_address_info_is_tagged_with_wallet_network() {
        use std::ffi::CString;
        // A valid testnet-prefixed (0x8C, "y…") P2PKH address, as a
        // devnet wallet persists them.
        let addr = "yMqShkrgjTRuReBGFpQr7FozEF1QcNBBYA";
        let addr_c = CString::new(addr).unwrap();
        let path_c = CString::new("m/44'/1'/0'/1/0").unwrap();
        let entry = CoreAddressEntryFFI {
            public_key: [0u8; 33],
            has_public_key: false,
            pool_type_tag: AddressPoolTypeTagFFI::Internal as u8,
            address_index: 0,
            is_used: false,
            balance: 0,
            address_base58: addr_c.as_ptr(),
            derivation_path: path_c.as_ptr(),
        };
        let info = unsafe { address_info_from_ffi(&entry, Network::Devnet) }
            .expect("restore must accept a testnet-prefixed string on devnet");
        let runtime_key = dashcore::Address::from_script(&info.script_pubkey, Network::Devnet)
            .expect("p2pkh script must convert back to an address");
        assert_eq!(
            info.address, runtime_key,
            "restored address must be identical (network tag included) to the \
             runtime `from_script` lookup key"
        );
    }

    /// Pins the contract that an `InBlock` unresolved-asset-lock row
    /// projects onto the matching BIP44 account's in-memory
    /// `transactions()` map with the correct context — the precise
    /// invariant the chain-lock cascade depends on at the next CLSig
    /// after a wallet restart.
    #[test]
    fn restore_unresolved_records_inserts_inblock_record() {
        let mut wallet_info = test_managed_wallet_info_with_bip44(0);
        let tx = synthetic_minimal_tx();
        let txid = tx.txid();
        let tx_bytes = serialize(&tx);

        // Build a single `InBlock` row pointing at account 0.
        let mut tx_buf: Vec<u8> = tx_bytes.clone();
        let block_hash = [0x42u8; 32];
        let rec = UnresolvedAssetLockTxRecordFFI {
            account_index: 0,
            tx_bytes: tx_buf.as_mut_ptr(),
            tx_bytes_len: tx_buf.len(),
            context_raw: 2,
            block_height: 1475917,
            block_hash,
            block_timestamp: 1700000000,
            first_seen: 1699999000,
        };

        let stats = restore_unresolved_asset_lock_tx_records(&mut wallet_info, &[rec])
            .expect("restoration should not return an error for a well-formed row");
        assert_eq!(
            stats,
            UnresolvedRestoreStats {
                restored: 1,
                dropped_decode: 0,
                dropped_no_account: 0
            }
        );

        let account = wallet_info
            .accounts
            .standard_bip44_accounts
            .get(&0)
            .expect("BIP44 account 0 must exist on the synthetic wallet");
        let restored = account
            .transactions()
            .get(&txid)
            .expect("restored record must be in the in-memory map");
        match &restored.context {
            key_wallet::transaction_checking::TransactionContext::InBlock(info) => {
                assert_eq!(info.height(), 1475917);
                let actual_hash = info.block_hash();
                let actual_hash_bytes: &[u8; 32] = actual_hash.as_ref();
                assert_eq!(actual_hash_bytes, &block_hash);
            }
            other => panic!("expected InBlock context, got {:?}", other),
        }

        // Keep the buffer alive until after the read so the
        // `tx_bytes` pointer remains valid for the duration of
        // `restore_unresolved_asset_lock_tx_records`. The function
        // copies the decoded `Transaction` into the record, so the
        // original buffer can drop after the call returns — but we
        // keep it alive explicitly here to make the lifetime
        // contract obvious for reviewers.
        drop(tx_buf);
    }

    /// Pins that a no-matching-account row is counted but doesn't
    /// abort the load — important so a stray persisted row from a
    /// pruned account doesn't poison wallet load.
    #[test]
    fn restore_unresolved_records_skips_missing_account() {
        let mut wallet_info = test_managed_wallet_info_with_bip44(0);
        let tx = synthetic_minimal_tx();
        let mut tx_buf: Vec<u8> = serialize(&tx);

        let rec = UnresolvedAssetLockTxRecordFFI {
            account_index: 99, // not present
            tx_bytes: tx_buf.as_mut_ptr(),
            tx_bytes_len: tx_buf.len(),
            context_raw: 2,
            block_height: 1,
            block_hash: [0u8; 32],
            block_timestamp: 0,
            first_seen: 0,
        };

        let stats = restore_unresolved_asset_lock_tx_records(&mut wallet_info, &[rec])
            .expect("missing-account is a recoverable drop, not an error");
        assert_eq!(stats.restored, 0);
        assert_eq!(stats.dropped_no_account, 1);
        drop(tx_buf);
    }

    /// Helper: build a `ManagedWalletInfo` with a single BIP44
    /// account at `index`. Uses a hard-coded valid xpub so the
    /// construction is deterministic; the test cares about the
    /// `transactions()` map structure, not key material.
    fn test_managed_wallet_info_with_bip44(index: u32) -> ManagedWalletInfo {
        // Derive a valid testnet xpub from the canonical
        // `abandon × 11 about` BIP-39 vector so the construction is
        // reproducible and doesn't depend on a hand-typed
        // base58-checked string. Same pattern the upstream
        // `account_collection_test.rs` uses.
        let mnemonic = Mnemonic::from_phrase(
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
            Language::English,
        )
        .expect("static BIP-39 vector must parse");
        let seed = mnemonic.to_seed("");
        let master = ExtendedPrivKey::new_master(Network::Testnet, &seed)
            .expect("master derivation must succeed");
        let secp = Secp256k1::new();
        let xpub = ExtendedPubKey::from_priv(&secp, &master);
        let account = Account::from_xpub(
            None,
            AccountType::Standard {
                index,
                standard_account_type: StandardAccountType::BIP44Account,
            },
            xpub,
            Network::Testnet,
        )
        .expect("Account::from_xpub on a valid xpub must succeed");
        let mut accounts = key_wallet::AccountCollection::new();
        accounts
            .insert(account)
            .expect("inserting the single account must succeed");
        let wallet = Wallet::new_external_signable(Network::Testnet, [0u8; 32], accounts);
        ManagedWalletInfo::from_wallet(&wallet, 0)
    }

    /// `account_xpub` must survive the persist→restore byte round-trip — it is
    /// the key the seed-binding self-check (`PlatformWallet::verify_seed_binds`)
    /// later compares the resolver-derived xpub against, so a corrupted restore
    /// would silently make a correct seed fail to bind. This drives the exact
    /// production chain: the store side bincode-encodes the account xpub
    /// (`build_account_specs_for_callback`), and the restore side decodes it from
    /// `AccountSpecFFI.account_xpub_bytes` and rebuilds the account via
    /// `Account::from_xpub` (`build_wallet_start_state`). Pins that both ends use
    /// the same bincode config and that `Account::from_xpub` preserves the xpub.
    /// (Asserted here at the FFI persister layer, where the bytes round-trip
    /// actually happens — `load_from_persistor` itself only sees decoded structs.)
    #[test]
    fn account_xpub_survives_persist_restore_round_trip() {
        let mnemonic = Mnemonic::from_phrase(
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
            Language::English,
        )
        .expect("static BIP-39 vector must parse");
        let seed = mnemonic.to_seed("");
        let wallet = Wallet::from_seed_bytes(
            seed,
            Network::Testnet,
            key_wallet::wallet::initialization::WalletAccountCreationOptions::Default,
        )
        .expect("seeded wallet");
        let wallet_id = wallet.wallet_id;
        let source = wallet
            .get_bip44_account(0)
            .expect("a Default-created wallet has BIP44 account 0");
        let account_type = source.account_type;
        let expected_xpub = source.account_xpub;

        // Store side: encode the xpub exactly as the callback producer does.
        let xpub_bytes =
            bincode::encode_to_vec(expected_xpub, config::standard()).expect("encode account xpub");
        // The C struct the host hands back on restore.
        let spec = build_account_spec_ffi(&account_type, &xpub_bytes);

        // Restore side: reconstruct the account type + decode the xpub exactly as
        // `build_wallet_start_state` does, then rebuild via `Account::from_xpub`.
        let restored_type =
            account_type_from_spec(&spec).expect("account type tag round-trips through the spec");
        let raw = unsafe { slice_from_raw(spec.account_xpub_bytes, spec.account_xpub_bytes_len) };
        let (decoded_xpub, _): (ExtendedPubKey, usize) =
            bincode::decode_from_slice(raw, config::standard()).expect("decode account xpub");
        assert_eq!(
            decoded_xpub, expected_xpub,
            "the bincode round-trip must preserve the account xpub byte-for-byte"
        );
        let restored = Account::from_xpub(
            Some(wallet_id),
            restored_type,
            decoded_xpub,
            Network::Testnet,
        )
        .expect("Account::from_xpub on the restored xpub must succeed");
        assert_eq!(
            restored.account_xpub, expected_xpub,
            "the restored account's xpub must equal the original — the key verify_seed_binds binds against"
        );
    }

    /// Helper: a minimum valid consensus-encodable transaction —
    /// version 1, one synthetic input, one zero-value output. The
    /// restoration helper only cares that the bytes round-trip
    /// through `consensus::encode::deserialize`; the tx's semantic
    /// validity is irrelevant.
    fn synthetic_minimal_tx() -> Transaction {
        Transaction {
            version: 1,
            lock_time: 0,
            input: vec![TxIn {
                previous_output: dashcore::OutPoint::null(),
                script_sig: ScriptBuf::new(),
                sequence: 0xffffffff,
                witness: dashcore::Witness::default(),
            }],
            output: vec![TxOut {
                value: 0,
                script_pubkey: ScriptBuf::new(),
            }],
            special_transaction_payload: None,
        }
    }

    /// **H1 — DashPay payment history is restored at load.**
    /// The fold must rebuild `dashpay_payments` (Sent AND Received, with
    /// memos) from the persisted rows, mapping the direction/status
    /// discriminants and decoding the c-strings. Without this restore step
    /// there is no payment restore at all, so the in-memory map starts empty
    /// and Sent entries vanish on relaunch.
    #[test]
    fn restore_payments_fold_rebuilds_sent_and_received() {
        use platform_wallet::wallet::identity::{PaymentDirection, PaymentStatus};

        let owner = IdentityV0 {
            id: Identifier::from([0xAA; 32]),
            public_keys: std::collections::BTreeMap::new(),
            balance: 0,
            revision: 0,
        };
        let mut managed = ManagedIdentity::new(Identity::V0(owner), 0);

        // Keep the CStrings alive for the duration of the call.
        let sent_txid = std::ffi::CString::new("aa".repeat(32)).unwrap();
        let sent_memo = std::ffi::CString::new("lunch").unwrap();
        let recv_txid = std::ffi::CString::new("bb".repeat(32)).unwrap();

        let rows = [
            PaymentRestoreEntryFFI {
                txid: sent_txid.as_ptr(),
                counterparty_id: [0xBB; 32],
                amount_duffs: 1_000_000,
                direction_raw: 0, // Sent
                status_raw: 0,    // Pending
                memo: sent_memo.as_ptr(),
            },
            PaymentRestoreEntryFFI {
                txid: recv_txid.as_ptr(),
                counterparty_id: [0xCC; 32],
                amount_duffs: 500_000,
                direction_raw: 1, // Received
                status_raw: 1,    // Confirmed
                memo: std::ptr::null(),
            },
        ];

        unsafe { apply_payment_rows(&rows, &mut managed) };

        assert_eq!(managed.dashpay().payments.len(), 2);
        let sent = managed
            .dashpay()
            .payments
            .get(&"aa".repeat(32))
            .expect("sent entry restored");
        assert_eq!(sent.direction, PaymentDirection::Sent);
        assert_eq!(sent.status, PaymentStatus::Pending);
        assert_eq!(sent.amount_duffs, 1_000_000);
        assert_eq!(sent.memo.as_deref(), Some("lunch"));
        assert_eq!(sent.counterparty_id, Identifier::from([0xBB; 32]));

        let recv = managed
            .dashpay()
            .payments
            .get(&"bb".repeat(32))
            .expect("received entry restored");
        assert_eq!(recv.direction, PaymentDirection::Received);
        assert_eq!(recv.status, PaymentStatus::Confirmed);
        assert!(recv.memo.is_none());

        // An unknown discriminant is skipped, not panicked.
        let bad_txid = std::ffi::CString::new("cc".repeat(32)).unwrap();
        let bad = [PaymentRestoreEntryFFI {
            txid: bad_txid.as_ptr(),
            counterparty_id: [0xDD; 32],
            amount_duffs: 1,
            direction_raw: 9,
            status_raw: 0,
            memo: std::ptr::null(),
        }];
        unsafe { apply_payment_rows(&bad, &mut managed) };
        assert_eq!(
            managed.dashpay().payments.len(),
            2,
            "a row with an unknown direction must be skipped, not inserted"
        );
    }

    /// **Cached contact profiles are restored at load.**
    /// The fold must rebuild `contact_profiles` (keyed by the contact's
    /// identity id) from the persisted rows, decoding the c-strings and
    /// the `_present`-gated avatar hash / fingerprint, and re-validating
    /// the public avatar URL. Without this restore step there is no
    /// contact-profile restore at all, so the cache starts empty on
    /// relaunch and the requests/contacts UI shows raw ids until the next
    /// sweep re-fetches every contact.
    #[test]
    fn restore_contact_profiles_fold_rebuilds_cache() {
        use crate::wallet_restore_types::ContactProfileRestoreEntryFFI;

        let owner = IdentityV0 {
            id: Identifier::from([0xAA; 32]),
            public_keys: std::collections::BTreeMap::new(),
            balance: 0,
            revision: 0,
        };
        let mut managed = ManagedIdentity::new(Identity::V0(owner), 0);

        // Keep the CStrings alive for the duration of the call.
        let display_name = std::ffi::CString::new("Alice").unwrap();
        let public_message = std::ffi::CString::new("gm").unwrap();
        let good_url = std::ffi::CString::new("https://example.com/a.png").unwrap();
        // A non-https URL: must be dropped to None, but the rest of the
        // profile (display name) must still be restored.
        let bad_url = std::ffi::CString::new("http://evil.example/track.gif").unwrap();
        let other_name = std::ffi::CString::new("Bob").unwrap();

        let rows = [
            ContactProfileRestoreEntryFFI {
                contact_id: [0xBB; 32],
                display_name: display_name.as_ptr(),
                bio: std::ptr::null(),
                avatar_url: good_url.as_ptr(),
                avatar_hash: [0x11; 32],
                avatar_hash_present: true,
                avatar_fingerprint: [0x22; 8],
                avatar_fingerprint_present: true,
                public_message: public_message.as_ptr(),
                checked_at_ms: 1_700_000_000_000,
            },
            ContactProfileRestoreEntryFFI {
                contact_id: [0xCC; 32],
                display_name: other_name.as_ptr(),
                bio: std::ptr::null(),
                avatar_url: bad_url.as_ptr(),
                avatar_hash: [0u8; 32],
                avatar_hash_present: false,
                avatar_fingerprint: [0u8; 8],
                avatar_fingerprint_present: false,
                public_message: std::ptr::null(),
                checked_at_ms: 1_700_000_000_001,
            },
        ];

        unsafe { apply_contact_profile_rows(&rows, &mut managed) };

        assert_eq!(managed.dashpay().contact_profiles.len(), 2);

        let alice = managed
            .dashpay()
            .contact_profiles
            .get(&Identifier::from([0xBB; 32]))
            .expect("alice contact profile restored");
        assert_eq!(alice.checked_at_ms, 1_700_000_000_000);
        let alice_profile = alice.profile.as_ref().expect("present profile");
        assert_eq!(alice_profile.display_name.as_deref(), Some("Alice"));
        assert_eq!(alice_profile.public_message.as_deref(), Some("gm"));
        assert_eq!(
            alice_profile.avatar_url.as_deref(),
            Some("https://example.com/a.png")
        );
        assert_eq!(alice_profile.avatar_hash, Some([0x11; 32]));
        assert_eq!(alice_profile.avatar_fingerprint, Some([0x22; 8]));
        assert!(alice_profile.bio.is_none());

        let bob = managed
            .dashpay()
            .contact_profiles
            .get(&Identifier::from([0xCC; 32]))
            .expect("bob contact profile restored");
        let bob_profile = bob.profile.as_ref().expect("present profile");
        assert_eq!(bob_profile.display_name.as_deref(), Some("Bob"));
        // The non-https avatar URL is dropped on the way back in; the
        // rest of the profile survives.
        assert!(
            bob_profile.avatar_url.is_none(),
            "a non-https avatar URL must be dropped at restore"
        );
        assert!(bob_profile.avatar_hash.is_none());
        assert!(bob_profile.avatar_fingerprint.is_none());
    }

    /// Regression: ignored senders must be restored at load so a
    /// previously-ignored sender does NOT resurface on relaunch.
    ///
    /// A fresh `ManagedIdentity` ignores nothing — that empty set is
    /// exactly the post-relaunch state in which the still-on-platform
    /// immutable `contactRequest`s re-ingest on the next sweep. Before
    /// `restore_dashpay_ignored`/`apply_ignored_rows` existed, the load
    /// path rebuilt contacts + payments but left this set empty; this test
    /// pins that the ignored senders are now rehydrated, and that the
    /// suppression is per-sender (a bumped-`accountReference` request from
    /// the same sender is STILL suppressed).
    #[test]
    fn restore_ignored_rows_rebuilds_ignore_set() {
        let owner = IdentityV0 {
            id: Identifier::from([0xAA; 32]),
            public_keys: std::collections::BTreeMap::new(),
            balance: 0,
            revision: 0,
        };
        let mut managed = ManagedIdentity::new(Identity::V0(owner), 0);

        // Post-relaunch precondition: nothing is ignored yet.
        assert!(!managed.is_sender_ignored(&Identifier::from([0xBB; 32])));

        let rows: [[u8; 32]; 2] = [[0xBB; 32], [0xDD; 32]];

        apply_ignored_rows(&rows, &mut managed);

        assert_eq!(managed.dashpay().ignored_senders().len(), 2);
        assert!(managed.is_sender_ignored(&Identifier::from([0xBB; 32])));
        assert!(managed.is_sender_ignored(&Identifier::from([0xDD; 32])));
        // Per-sender suppression: the ignored sender is suppressed
        // regardless of accountReference (no per-ref discrimination).
        assert!(!managed.is_sender_ignored(&Identifier::from([0xEE; 32])));
    }

    /// Persist→restore round-trip for the contact maps. The persist
    /// projection (`from_outgoing`/`from_incoming`/`from_established_*`) is
    /// tested field-by-field, but the FFI→Rust read-back (`apply_contact_rows`)
    /// was untested — so a dropped optional or a swapped key index on restore
    /// would be invisible (the exact bug class that shipped on the parse side).
    /// This builds rows via the SAME persist constructors the live path uses,
    /// decodes them, and asserts field-by-field — including the
    /// direction-specific `contact_account_label` (incoming-row only) and
    /// distinct sender ≠ recipient key indices (a swap is otherwise invisible,
    /// both `u32`).
    #[test]
    fn apply_contact_rows_round_trips_all_fields() {
        use platform_wallet::ContactRequest;

        let owner = Identifier::from([0x11; 32]);
        let sent_c = Identifier::from([0xA1; 32]);
        let in_c = Identifier::from([0xA2; 32]);
        let estab_c = Identifier::from([0xA3; 32]);

        let sent_req = ContactRequest::new(owner, sent_c, 3, 4, 11, vec![1u8; 96], 100, 1000);
        let in_req = ContactRequest::new(in_c, owner, 5, 6, 22, vec![2u8; 96], 101, 1001);
        let mut estab_out = ContactRequest::new(owner, estab_c, 7, 8, 33, vec![3u8; 96], 102, 1002);
        estab_out.auto_accept_proof = Some(vec![9u8; 40]);
        let mut estab_in = ContactRequest::new(estab_c, owner, 9, 10, 44, vec![4u8; 96], 103, 1003);
        estab_in.encrypted_account_label = Some(vec![0x2au8; 48]);
        estab_in.auto_accept_proof = Some(vec![8u8; 38]);

        let mut rows = vec![
            ContactRequestFFI::from_outgoing(owner.to_buffer(), sent_c.to_buffer(), &sent_req),
            ContactRequestFFI::from_incoming(owner.to_buffer(), in_c.to_buffer(), &in_req),
            ContactRequestFFI::from_established_outgoing(
                owner.to_buffer(),
                estab_c.to_buffer(),
                &estab_out,
                true,
                Some("ally"),
                Some("a note"),
                true,
                &[7, 42],
            ),
            ContactRequestFFI::from_established_incoming(
                owner.to_buffer(),
                estab_c.to_buffer(),
                &estab_in,
                true,
                Some("ally"),
                Some("a note"),
                true,
                Some("Main wallet"),
                &[7, 42],
            ),
        ];

        let identity_v0 = IdentityV0 {
            id: owner,
            public_keys: std::collections::BTreeMap::new(),
            balance: 0,
            revision: 0,
        };
        let mut managed = ManagedIdentity::new(Identity::V0(identity_v0), 0);

        unsafe { apply_contact_rows(&rows, &owner, &mut managed) };

        let s = managed
            .dashpay()
            .sent_contact_requests()
            .get(&sent_c)
            .expect("pending sent request restored");
        assert_eq!(
            (
                s.sender_key_index,
                s.recipient_key_index,
                s.account_reference
            ),
            (3, 4, 11)
        );
        let i = managed
            .dashpay()
            .incoming_contact_requests()
            .get(&in_c)
            .expect("pending incoming request restored");
        assert_eq!(
            (
                i.sender_key_index,
                i.recipient_key_index,
                i.account_reference
            ),
            (5, 6, 22)
        );

        let e = managed
            .dashpay()
            .established_contacts()
            .get(&estab_c)
            .expect("established contact restored");
        assert_eq!(e.outgoing_request.auto_accept_proof, Some(vec![9u8; 40]));
        assert_eq!(
            e.incoming_request.encrypted_account_label,
            Some(vec![0x2au8; 48]),
            "the incoming encrypted label must survive read-back (the shipped-bug class)"
        );
        assert_eq!(e.incoming_request.auto_accept_proof, Some(vec![8u8; 38]));
        assert_eq!(e.alias.as_deref(), Some("ally"));
        assert_eq!(e.note.as_deref(), Some("a note"));
        assert!(e.is_hidden);
        assert!(e.payment_channel_broken);
        assert_eq!(
            e.contact_account_label.as_deref(),
            Some("Main wallet"),
            "contact_account_label must restore from the incoming row only"
        );
        assert_eq!(
            e.accepted_accounts,
            vec![7, 42],
            "accepted_accounts must round-trip through the FFI rows (matching the SQLite backend)"
        );
        // Key indices restored without a swap (incoming sender=9, recipient=10).
        assert_eq!(
            (
                e.incoming_request.sender_key_index,
                e.incoming_request.recipient_key_index
            ),
            (9, 10)
        );

        unsafe { free_contact_requests_ffi(rows.as_mut_ptr(), rows.len()) };
    }

    /// Stub one marked-used entry at `(account_type, pool_type, index)`
    /// for the grouping test. Only the grouping key and the `used`
    /// flag matter here.
    fn stub_marked_used(
        account_type: AccountType,
        pool_type: AddressPoolType,
        index: u32,
    ) -> key_wallet::transaction_checking::DerivedAddressInfo {
        use key_wallet::bip32::{ChildNumber, DerivationPath};
        // Compressed secp256k1 generator point — a well-known valid key.
        const TEST_PUBKEY_G: [u8; 33] = [
            0x02, 0x79, 0xbe, 0x66, 0x7e, 0xf9, 0xdc, 0xbb, 0xac, 0x55, 0xa0, 0x62, 0x95, 0xce,
            0x87, 0x0b, 0x07, 0x02, 0x9b, 0xfc, 0xdb, 0x2d, 0xce, 0x28, 0xd9, 0x59, 0xf2, 0x81,
            0x5b, 0x16, 0xf8, 0x17, 0x98,
        ];
        let pubkey =
            dashcore::PublicKey::from_slice(&TEST_PUBKEY_G).expect("generator point is valid");
        let address = dashcore::Address::p2pkh(&pubkey, Network::Testnet);
        let script_pubkey = address.script_pubkey();
        let path = DerivationPath::from(vec![
            ChildNumber::from_normal_idx(0).expect("valid child number"),
            ChildNumber::from_normal_idx(index).expect("valid child number"),
        ]);
        key_wallet::transaction_checking::DerivedAddressInfo {
            account_type,
            pool_type,
            info: AddressInfo {
                address,
                script_pubkey,
                public_key: Some(PublicKeyType::ECDSA(TEST_PUBKEY_G.to_vec())),
                index,
                path,
                used: true,
                generated_at: 0,
                used_at: None,
                tx_count: 0,
                total_received: 0,
                total_sent: 0,
                balance: 0,
                label: None,
                metadata: BTreeMap::new(),
            },
        }
    }

    /// Marked-used entries bucket into one `AccountAddressPoolEntry`
    /// per `(account_type, pool_type)` pair — the shape
    /// `build_address_pools_for_callback` expects — and every emitted
    /// address keeps `used == true` so the Swift persister flips the
    /// row instead of resetting it.
    #[test]
    fn marked_used_entries_group_per_account_and_pool() {
        let bip44 = AccountType::Standard {
            index: 0,
            standard_account_type: StandardAccountType::BIP44Account,
        };
        let owner_keys = AccountType::ProviderOwnerKeys;

        let marked = vec![
            stub_marked_used(bip44, AddressPoolType::External, 0),
            stub_marked_used(bip44, AddressPoolType::External, 3),
            stub_marked_used(bip44, AddressPoolType::Internal, 1),
            stub_marked_used(owner_keys, AddressPoolType::Absent, 0),
        ];

        let entries = group_marked_used_into_pool_entries(&marked);
        assert_eq!(entries.len(), 3, "one bucket per (account, pool) pair");

        let bip44_external = entries
            .iter()
            .find(|e| e.account_type == bip44 && e.pool_type == AddressPoolType::External)
            .expect("bip44 external bucket");
        assert_eq!(bip44_external.addresses.len(), 2);

        let owner_bucket = entries
            .iter()
            .find(|e| e.account_type == owner_keys)
            .expect("provider owner keys bucket");
        assert_eq!(owner_bucket.pool_type, AddressPoolType::Absent);
        assert_eq!(owner_bucket.addresses.len(), 1);
        assert!(
            entries
                .iter()
                .flat_map(|e| e.addresses.iter())
                .all(|a| a.used),
            "every emitted marked-used address must carry used == true"
        );
    }
}
