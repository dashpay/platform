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
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;
use key_wallet::{AddressInfo, Network};
use parking_lot::RwLock;
use platform_wallet::changeset::{
    ClientStartState, ClientWalletStartState, Merge, PersistenceError, PlatformWalletChangeSet,
    PlatformWalletPersistence,
};
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet::wallet::{PerAccountPlatformAddressState, PerWalletPlatformAddressState};
use std::collections::BTreeMap;
use std::ffi::CString;
use std::os::raw::c_void;
use std::slice;

use crate::core_address_types::{
    AddressPoolTypeTagFFI, CoreAddressEntryFFI, PersistAccountAddressesFn,
};
use crate::core_wallet_types::{free_wallet_changeset_ffi, WalletChangeSetFFI};
use crate::identity_persistence::{
    free_identity_entry_ffi, free_identity_key_entry_ffi, IdentityEntryFFI, IdentityKeyEntryFFI,
    IdentityKeyRemovalFFI,
};
use crate::platform_address_types::AddressBalanceEntryFFI;
use crate::wallet_restore_types::{
    AccountSpecFFI, AccountTypeTagFFI, LoadWalletListFn, LoadWalletListFreeFn, PersistAccountFn,
    PersistWalletMetadataFn, StandardAccountTypeTagFFI, WalletRestoreEntryFFI,
};
use dpp::address_funds::PlatformAddress;

/// C callback vtable for wallet persistence.
///
/// General-purpose notifications (`on_store_fn`, `on_flush_fn`) plus
/// typed callbacks that send incremental data across FFI for the caller
/// to persist in their preferred storage backend.
#[repr(C)]
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
    /// Called once per account when the account is added to a wallet.
    /// Caller should upsert keyed by `(wallet_id, account spec)`. See
    /// [`PersistAccountFn`].
    pub on_persist_account_fn: Option<PersistAccountFn>,
    /// Invoked on [`FFIPersister::load`] to pull the persisted wallet
    /// list back into Rust for watch-only reconstruction. See
    /// [`LoadWalletListFn`] for the allocation / lifetime contract.
    pub on_load_wallet_list_fn: Option<LoadWalletListFn>,
    /// Paired free callback for `on_load_wallet_list_fn`. See
    /// [`LoadWalletListFreeFn`].
    pub on_load_wallet_list_free_fn: Option<LoadWalletListFreeFn>,
    /// Called once per wallet at registration with network tag and
    /// birth height. See [`PersistWalletMetadataFn`].
    pub on_persist_wallet_metadata_fn: Option<PersistWalletMetadataFn>,
    /// Called per account whenever its address pool content changes
    /// (initial population, pool extension, `used` flip). See
    /// [`PersistAccountAddressesFn`].
    pub on_persist_account_addresses_fn: Option<PersistAccountAddressesFn>,
    /// Called with an `IdentityChangeSet` slice — scalar-only
    /// identity upserts (id / balance / revision / label / status /
    /// wallet_id / identity_index) and identity-id removals. Swift
    /// handlers map upserts onto `PersistentIdentity` rows and
    /// removals onto tombstones. `primary_identity_ptr` is `null`
    /// when the changeset doesn't touch primary selection;
    /// `has_last_scanned_index` gates `last_scanned_index` the same
    /// way.
    pub on_persist_identities_fn: Option<
        unsafe extern "C" fn(
            context: *mut c_void,
            wallet_id: *const u8,
            upserts_ptr: *const IdentityEntryFFI,
            upserts_count: usize,
            removed_ptr: *const [u8; 32],
            removed_count: usize,
            primary_identity_ptr: *const [u8; 32],
            has_last_scanned_index: bool,
            last_scanned_index: u32,
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

        // Send identity scalar changeset — upserts, removals, primary
        // selection, and the gap-limit watermark. Swift handler maps
        // these onto `PersistentIdentity` row upserts / tombstones.
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
                let primary_buf = id_cs.primary_identity.map(|id| id.to_buffer());
                let primary_ptr: *const [u8; 32] = primary_buf
                    .as_ref()
                    .map(|b| b as *const [u8; 32])
                    .unwrap_or(std::ptr::null());
                let (has_scan, scan_idx) = match id_cs.last_scanned_index {
                    Some(v) => (true, v),
                    None => (false, 0),
                };
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
                        primary_ptr,
                        has_scan,
                        scan_idx,
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

    fn store_account(
        &self,
        wallet_id: WalletId,
        account_type: &AccountType,
        account_xpub: &ExtendedPubKey,
    ) -> Result<(), PersistenceError> {
        let Some(cb) = self.callbacks.on_persist_account_fn else {
            return Ok(());
        };
        let xpub_bytes = bincode::encode_to_vec(account_xpub, config::standard())
            .map_err(|e| format!("failed to encode account xpub: {}", e))?;
        let spec = build_account_spec_ffi(account_type, &xpub_bytes);
        let result = unsafe { cb(self.callbacks.context, wallet_id.as_ptr(), &spec) };
        if result != 0 {
            return Err(format!(
                "Persistence account callback returned error code {}",
                result
            )
            .into());
        }
        Ok(())
    }

    fn store_account_addresses(
        &self,
        wallet_id: WalletId,
        account_type: &AccountType,
        pool_type: AddressPoolType,
        addresses: &[AddressInfo],
    ) -> Result<(), PersistenceError> {
        let Some(cb) = self.callbacks.on_persist_account_addresses_fn else {
            return Ok(());
        };
        if addresses.is_empty() {
            return Ok(());
        }

        let pool_tag = match pool_type {
            AddressPoolType::External => AddressPoolTypeTagFFI::External,
            AddressPoolType::Internal => AddressPoolTypeTagFFI::Internal,
            AddressPoolType::Absent => AddressPoolTypeTagFFI::Absent,
            AddressPoolType::AbsentHardened => AddressPoolTypeTagFFI::AbsentHardened,
        } as u8;

        // Whether the address pool belongs to a DIP-17 PlatformPayment
        // account. The addresses themselves are the same (P2PKH / P2SH
        // hashes derived from the wallet), but Platform Payment
        // addresses are rendered as DIP-0018 bech32m (`dash1…` /
        // `tdash1…`) rather than the base58check Core form.
        let is_platform_payment = matches!(account_type, AccountType::PlatformPayment { .. });

        // Build owned CStrings for every (address, path) pair so they
        // outlive the callback window. `entries` borrows the pointers.
        let mut owned_strings: Vec<CString> = Vec::with_capacity(addresses.len() * 2);
        let mut entries: Vec<CoreAddressEntryFFI> = Vec::with_capacity(addresses.len());
        for info in addresses {
            // Pick the right display encoding based on whether this
            // address belongs to a PlatformPayment pool. If the
            // `PlatformAddress` conversion fails (only supports P2PKH
            // and P2SH), fall back to the base58check form so the
            // address is still surfaced to the caller.
            let rendered_address = if is_platform_payment {
                let network = *info.address.network();
                let converted: Result<PlatformAddress, _> =
                    PlatformAddress::try_from(info.address.clone());
                converted
                    .map(|p| p.to_bech32m_string(network))
                    .unwrap_or_else(|_| info.address.to_string())
            } else {
                info.address.to_string()
            };
            let address_c = CString::new(rendered_address)
                .map_err(|e| format!("address contained NUL byte: {}", e))?;
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

            entries.push(CoreAddressEntryFFI {
                public_key,
                has_public_key,
                pool_type_tag: pool_tag,
                address_index: info.index,
                is_used: info.used,
                balance: info.balance,
                address_base58: address_ptr,
                derivation_path: path_ptr,
            });
        }

        // Identify the account to Swift using the same flat-spec shape
        // `on_persist_account_fn` uses (minus the per-account xpub —
        // irrelevant for the address-write path).
        let empty_xpub: &[u8] = &[];
        let spec = build_account_spec_ffi(account_type, empty_xpub);

        let result = unsafe {
            cb(
                self.callbacks.context,
                wallet_id.as_ptr(),
                &spec,
                entries.as_ptr(),
                entries.len(),
            )
        };
        // Force `owned_strings` to live until after the callback.
        drop(owned_strings);

        if result != 0 {
            return Err(format!(
                "Persistence account_addresses callback returned error code {}",
                result
            )
            .into());
        }
        Ok(())
    }

    fn store_wallet_metadata(
        &self,
        wallet_id: WalletId,
        network: Network,
        birth_height: u32,
    ) -> Result<(), PersistenceError> {
        let Some(cb) = self.callbacks.on_persist_wallet_metadata_fn else {
            return Ok(());
        };
        let network_tag = network_tag_for(network);
        let result = unsafe {
            cb(
                self.callbacks.context,
                wallet_id.as_ptr(),
                network_tag,
                birth_height,
            )
        };
        if result != 0 {
            return Err(format!(
                "Persistence wallet_metadata callback returned error code {}",
                result
            )
            .into());
        }
        Ok(())
    }
}

/// Reverse of [`network_from_tag`] — keeps the discriminant in sync
/// with `platform_wallet_manager_create_wallet_from_seed` (0 = Mainnet,
/// 1 = Testnet, 2 = Devnet, 3 = Regtest).
fn network_tag_for(network: Network) -> u8 {
    match network {
        Network::Mainnet => 0,
        Network::Testnet => 1,
        Network::Devnet => 2,
        Network::Regtest => 3,
        // Future variants: fall through to Testnet as the least-bad
        // default. Adding a variant here is trivial.
        _ => 1,
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
        type_tag: AccountTypeTagFFI::Standard,
        standard_tag: StandardAccountTypeTagFFI::Bip44,
        index: 0,
        registration_index: 0,
        key_class: 0,
        user_identity_id: [0u8; 32],
        friend_identity_id: [0u8; 32],
        account_xpub_bytes: xpub_bytes.as_ptr(),
        account_xpub_bytes_len: xpub_bytes.len(),
    };
    match account_type {
        AccountType::Standard {
            index,
            standard_account_type,
        } => {
            spec.type_tag = AccountTypeTagFFI::Standard;
            spec.standard_tag = match standard_account_type {
                StandardAccountType::BIP44Account => StandardAccountTypeTagFFI::Bip44,
                StandardAccountType::BIP32Account => StandardAccountTypeTagFFI::Bip32,
            };
            spec.index = *index;
        }
        AccountType::CoinJoin { index } => {
            spec.type_tag = AccountTypeTagFFI::CoinJoin;
            spec.index = *index;
        }
        AccountType::IdentityRegistration => {
            spec.type_tag = AccountTypeTagFFI::IdentityRegistration;
        }
        AccountType::IdentityTopUp { registration_index } => {
            spec.type_tag = AccountTypeTagFFI::IdentityTopUp;
            spec.registration_index = *registration_index;
        }
        AccountType::IdentityTopUpNotBoundToIdentity => {
            spec.type_tag = AccountTypeTagFFI::IdentityTopUpNotBoundToIdentity;
        }
        AccountType::IdentityInvitation => {
            spec.type_tag = AccountTypeTagFFI::IdentityInvitation;
        }
        AccountType::AssetLockAddressTopUp => {
            spec.type_tag = AccountTypeTagFFI::AssetLockAddressTopUp;
        }
        AccountType::AssetLockShieldedAddressTopUp => {
            spec.type_tag = AccountTypeTagFFI::AssetLockShieldedAddressTopUp;
        }
        AccountType::ProviderVotingKeys => {
            spec.type_tag = AccountTypeTagFFI::ProviderVotingKeys;
        }
        AccountType::ProviderOwnerKeys => {
            spec.type_tag = AccountTypeTagFFI::ProviderOwnerKeys;
        }
        AccountType::ProviderOperatorKeys => {
            spec.type_tag = AccountTypeTagFFI::ProviderOperatorKeys;
        }
        AccountType::ProviderPlatformKeys => {
            spec.type_tag = AccountTypeTagFFI::ProviderPlatformKeys;
        }
        AccountType::DashpayReceivingFunds {
            index,
            user_identity_id,
            friend_identity_id,
        } => {
            spec.type_tag = AccountTypeTagFFI::DashpayReceivingFunds;
            spec.index = *index;
            spec.user_identity_id = *user_identity_id;
            spec.friend_identity_id = *friend_identity_id;
        }
        AccountType::DashpayExternalAccount {
            index,
            user_identity_id,
            friend_identity_id,
        } => {
            spec.type_tag = AccountTypeTagFFI::DashpayExternalAccount;
            spec.index = *index;
            spec.user_identity_id = *user_identity_id;
            spec.friend_identity_id = *friend_identity_id;
        }
        AccountType::PlatformPayment { account, key_class } => {
            spec.type_tag = AccountTypeTagFFI::PlatformPayment;
            spec.index = *account;
            spec.key_class = *key_class;
        }
        AccountType::IdentityAuthenticationEcdsa { identity_index } => {
            spec.type_tag = AccountTypeTagFFI::IdentityAuthenticationEcdsa;
            spec.index = *identity_index;
        }
        AccountType::IdentityAuthenticationBls { identity_index } => {
            spec.type_tag = AccountTypeTagFFI::IdentityAuthenticationBls;
            spec.index = *identity_index;
        }
    }
    spec
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

/// Reconstruct a watch-only [`Wallet`] + matching start-state bucket
/// from a single `WalletRestoreEntryFFI`.
fn build_wallet_start_state(
    entry: &WalletRestoreEntryFFI,
) -> Result<
    (
        ClientWalletStartState,
        Option<platform_wallet::PlatformAddressSyncStartState>,
    ),
    PersistenceError,
> {
    let network = network_from_tag(entry.network)?;

    // Build the per-account collection from the typed spec array.
    let mut accounts = AccountCollection::new();
    let specs: &[AccountSpecFFI] = if entry.accounts.is_null() || entry.accounts_count == 0 {
        &[]
    } else {
        unsafe { slice::from_raw_parts(entry.accounts, entry.accounts_count) }
    };
    for spec in specs {
        let account_type = account_type_from_spec(spec)?;
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

    // Watch-only wallet via the new unit-variant constructor — takes
    // the wallet_id directly (no recomputation from a root xpub we
    // don't store anymore). Signing ops error out until a follow-up
    // unlock path builds a signing wallet from the mnemonic.
    let wallet = Wallet::new_watch_only(network, entry.wallet_id, accounts);

    let wallet_info = ManagedWalletInfo::from_wallet(&wallet);

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
    for persisted in platform_balance_entries {
        if persisted.address.address_type != 0 {
            return Err("only P2PKH platform address persistence is supported".into());
        }

        let account_state = per_account
            .get_mut(&persisted.account_index)
            .ok_or_else(|| {
                format!(
                    "persisted platform address references unknown account {}",
                    persisted.account_index
                )
            })?;
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

    let wallet_state = ClientWalletStartState {
        wallet,
        wallet_info,
        identity_manager: Default::default(),
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

fn network_from_tag(tag: u8) -> Result<Network, PersistenceError> {
    match tag {
        0 => Ok(Network::Mainnet),
        1 => Ok(Network::Testnet),
        2 => Ok(Network::Devnet),
        3 => Ok(Network::Regtest),
        other => Err(format!("unknown network tag {}", other).into()),
    }
}

fn account_type_from_spec(spec: &AccountSpecFFI) -> Result<AccountType, PersistenceError> {
    Ok(match spec.type_tag {
        AccountTypeTagFFI::Standard => {
            let standard_account_type = match spec.standard_tag {
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
        AccountTypeTagFFI::IdentityAuthenticationEcdsa => {
            AccountType::IdentityAuthenticationEcdsa {
                identity_index: spec.index,
            }
        }
        AccountTypeTagFFI::IdentityAuthenticationBls => AccountType::IdentityAuthenticationBls {
            identity_index: spec.index,
        },
    })
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
