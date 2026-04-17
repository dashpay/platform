//! FFI callback-based implementation of PlatformWalletPersistence.
//!
//! Changesets are kept in-memory as Rust objects. When specific sub-changeset
//! data is available (e.g., address balances), it is sent across FFI in
//! C-compatible structs so the caller can persist it incrementally (e.g., via
//! SwiftData on iOS).

use parking_lot::RwLock;
use platform_wallet::changeset::{Merge, PlatformWalletChangeSet, PlatformWalletPersistence};
use platform_wallet::wallet::platform_wallet::WalletId;
use std::collections::BTreeMap;
use std::os::raw::c_void;

use crate::core_wallet_types::{free_wallet_changeset_ffi, WalletChangeSetFFI};
use crate::platform_address_types::AddressBalanceEntryFFI;

/// C callback vtable for wallet persistence.
///
/// General-purpose notifications (`on_store_fn`, `on_flush_fn`) plus
/// typed callbacks that send incremental data across FFI for the caller
/// to persist in their preferred storage backend.
#[repr(C)]
pub struct PersistenceCallbacks {
    /// Opaque context pointer passed to all callbacks.
    pub context: *mut c_void,
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
}

// SAFETY: The context pointer is managed by the FFI caller who must ensure
// thread safety.
unsafe impl Send for PersistenceCallbacks {}
unsafe impl Sync for PersistenceCallbacks {}

/// In-memory persister that accumulates changesets and notifies via callbacks.
pub(crate) struct FFIPersister {
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
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Send incremental address balance updates before merging.
        if let Some(ref addr_cs) = changeset.platform_addresses {
            if let Some(cb) = self.callbacks.on_persist_address_balances_fn {
                let entries: Vec<AddressBalanceEntryFFI> = addr_cs
                    .addresses
                    .iter()
                    .map(|entry| AddressBalanceEntryFFI {
                        address: entry.address.into(),
                        balance: entry.funds.balance,
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
                    }
                }
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

    fn flush(&self, wallet_id: WalletId) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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

    fn load(
        &self,
        wallet_id: WalletId,
    ) -> Result<PlatformWalletChangeSet, Box<dyn std::error::Error + Send + Sync>> {
        // Return any pending changeset, or empty default.
        let pending = self.pending.read();
        Ok(pending.get(&wallet_id).cloned().unwrap_or_default())
    }
}
