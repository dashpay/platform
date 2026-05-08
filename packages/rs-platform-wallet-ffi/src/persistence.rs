//! FFI callback-based implementation of PlatformWalletPersistence.
//!
//! Changesets are kept in-memory as Rust objects. When specific sub-changeset
//! data is available (e.g., address balances), it is sent across FFI in
//! C-compatible structs so the caller can persist it incrementally (e.g., via
//! SwiftData on iOS).

use bincode::config;
use key_wallet::account::account_collection::AccountCollection;
use key_wallet::account::{Account, AccountType, StandardAccountType};
use key_wallet::bip32::ExtendedPubKey;
use key_wallet::managed_account::address_pool::{AddressPoolType, PublicKeyType};
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;
use key_wallet::AddressInfo;
use parking_lot::RwLock;

use crate::types::{FFINetwork, Network};
use platform_wallet::changeset::{
    AccountAddressPoolEntry, AccountRegistrationEntry, ClientStartState, ClientWalletStartState,
    Merge, PersistenceError, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet::wallet::{PerAccountPlatformAddressState, PerWalletPlatformAddressState};
use std::collections::BTreeMap;
use std::ffi::CString;
use std::os::raw::c_void;
use std::slice;

use crate::contact_persistence::{
    free_contact_requests_ffi, ContactRequestFFI, ContactRequestRemovalFFI,
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
    AccountSpecFFI, AccountTypeTagFFI, IdentityKeyRestoreFFI, IdentityRestoreEntryFFI,
    LoadWalletListFreeFn, StandardAccountTypeTagFFI, UtxoRestoreEntryFFI, WalletRestoreEntryFFI,
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
    /// network tag + birth height. `network` uses the same
    /// discriminant as `WalletRestoreEntryFFI.network` (0 = Mainnet,
    /// 1 = Testnet, 2 = Devnet, 3 = Regtest). `birth_height` is the
    /// best estimate of the block at which the wallet started; zero
    /// means "scan from genesis / unknown".
    ///
    /// Returns 0 on success. A non-zero return flips the round's
    /// `success` flag to `false` so [`Self::on_changeset_end_fn`]
    /// receives the rollback signal.
    pub on_persist_wallet_metadata_fn: Option<
        unsafe extern "C" fn(
            context: *mut c_void,
            wallet_id: *const u8,
            network: FFINetwork,
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
    /// incoming / established contact requests in `upserts`, plus
    /// parallel sent / incoming tombstone arrays.
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
        ) -> i32,
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
}

// SAFETY: The context pointer is managed by the FFI caller who must ensure
// thread safety.
unsafe impl Send for PersistenceCallbacks {}
unsafe impl Sync for PersistenceCallbacks {}

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
        if !changeset.account_registrations.is_empty() {
            if let Some(cb) = self.callbacks.on_persist_account_registrations_fn {
                let entries = &changeset.account_registrations;
                match build_account_specs_for_callback(entries) {
                    Ok((specs, _xpub_bytes_storage)) => {
                        let result = unsafe {
                            cb(
                                self.callbacks.context,
                                wallet_id.as_ptr(),
                                specs.as_ptr(),
                                specs.len(),
                            )
                        };
                        // Force the spec / byte buffers to live
                        // until after the callback even though
                        // their drop happens on scope exit anyway.
                        drop(specs);
                        drop(_xpub_bytes_storage);
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
                    upserts.push(ContactRequestFFI::from_outgoing(
                        key.owner_id.to_buffer(),
                        key.recipient_id.to_buffer(),
                        &established.outgoing_request,
                    ));
                    upserts.push(ContactRequestFFI::from_incoming(
                        key.owner_id.to_buffer(),
                        key.recipient_id.to_buffer(),
                        &established.incoming_request,
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
                if !upserts.is_empty() || !removed_sent.is_empty() || !removed_incoming.is_empty() {
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

        // Close the round. Clients use this to commit (if
        // `round_success == true`) or roll back (otherwise) the
        // staged writes accumulated across the per-kind callbacks
        // above, making the whole store() call a single atomic
        // transaction from their perspective.
        if let Some(cb) = self.callbacks.on_changeset_end_fn {
            let result = unsafe { cb(self.callbacks.context, wallet_id.as_ptr(), round_success) };
            if result != 0 {
                eprintln!("Changeset-end callback returned error code {}", result);
            }
        }

        if !round_success {
            return Err(
                "one or more persistence callbacks failed; changeset was rolled back"
                    .to_string()
                    .into(),
            );
        }

        // Merge into pending changesets.
        let mut pending = self.pending.write();
        pending
            .entry(wallet_id)
            .and_modify(|existing| existing.merge(changeset.clone()))
            .or_insert(changeset);

        // Notify caller.
        if let Some(cb) = self.callbacks.on_store_fn {
            let result = unsafe { cb(self.callbacks.context, wallet_id.as_ptr()) };
            if result != 0 {
                return Err(
                    format!("Persistence store callback returned error code {}", result).into(),
                );
            }
        }

        Ok(())
    }

    fn flush(&self, wallet_id: WalletId) -> Result<(), PersistenceError> {
        // Notify caller.
        if let Some(cb) = self.callbacks.on_flush_fn {
            let result = unsafe { cb(self.callbacks.context, wallet_id.as_ptr()) };
            if result != 0 {
                return Err(
                    format!("Persistence flush callback returned error code {}", result).into(),
                );
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
            return Err(format!("on_load_wallet_list_fn returned error code {}", rc).into());
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
        Ok(out)
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

/// Flatten an `AccountType` + encoded xpub into the C-flat
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
/// `on_persist_account_registrations_fn` plus the parallel
/// `Vec<Vec<u8>>` of bincoded xpub byte buffers each spec borrows
/// from. The two Vecs share lifetime — caller drops both after the
/// callback returns.
fn build_account_specs_for_callback(
    entries: &[AccountRegistrationEntry],
) -> Result<(Vec<AccountSpecFFI>, Vec<Vec<u8>>), String> {
    // Pre-encode every xpub once so the spec slot can borrow the
    // pointer + length without a self-referential lifetime trick.
    let xpub_buffers: Vec<Vec<u8>> = entries
        .iter()
        .map(|entry| {
            bincode::encode_to_vec(entry.account_xpub, config::standard())
                .map_err(|e| format!("failed to encode account xpub: {}", e))
        })
        .collect::<Result<_, _>>()?;
    let specs: Vec<AccountSpecFFI> = entries
        .iter()
        .zip(xpub_buffers.iter())
        .map(|(entry, bytes)| build_account_spec_ffi(&entry.account_type, bytes))
        .collect();
    Ok((specs, xpub_buffers))
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
        let network = *info.address.network();
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
                let network = *d.address.network();
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
                public_key: d.public_key,
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
        let (account_xpub, _): (ExtendedPubKey, usize) =
            bincode::decode_from_slice(xpub_bytes, config::standard())
                .map_err(|e| format!("failed to decode account xpub: {}", e))?;
        let account =
            Account::from_xpub(Some(entry.wallet_id), account_type, account_xpub, network)
                .map_err(|e| format!("Account::from_xpub failed: {:?}", e))?;
        accounts
            .insert(account)
            .map_err(|e| format!("AccountCollection::insert failed: {}", e))?;
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

    let wallet_state = ClientWalletStartState {
        wallet,
        wallet_info,
        identity_manager,
        unused_asset_locks: BTreeMap::new(),
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
/// an empty map and gets refreshed on the next sync round — same
/// degraded-but-usable behaviour as before this change for that
/// narrow case.
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
        bucket.insert(spec.identity_index, managed);
    }

    Ok(bucket)
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
        let pk = IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: row.key_id,
            purpose,
            security_level,
            contract_bounds: None,
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
        PersistenceError::Backend(format!(
            "AccountSpecFFI carries unknown type_tag byte {} (out of declared range)",
            spec.type_tag
        ))
    })?;
    Ok(match type_tag {
        AccountTypeTagFFI::Standard => {
            let standard_tag = StandardAccountTypeTagFFI::try_from_u8(spec.standard_tag)
                .ok_or_else(|| {
                    PersistenceError::Backend(format!(
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
            return Err(PersistenceError::Backend(format!(
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
