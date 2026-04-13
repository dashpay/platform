//! FFI callback-based implementation of PlatformWalletPersistence.
//!
//! Since `PlatformWalletChangeSet` does not implement `Serialize`/`Deserialize`,
//! changesets are kept in-memory as Rust objects. The FFI callbacks notify the
//! caller that a store/flush/load occurred, and the caller can use the
//! notification to trigger their own persistence logic (e.g., writing a marker
//! file). The actual changeset data stays on the Rust side.

use parking_lot::RwLock;
use platform_wallet::changeset::{Merge, PlatformWalletChangeSet, PlatformWalletPersistence};
use platform_wallet::wallet::platform_wallet::WalletId;
use std::collections::BTreeMap;
use std::os::raw::c_void;

/// C callback vtable for wallet persistence notifications.
///
/// These callbacks notify the FFI caller when persistence events occur.
/// The actual changeset data is managed internally by the Rust side.
#[repr(C)]
pub struct PersistenceCallbacks {
    /// Opaque context pointer passed to all callbacks.
    pub context: *mut c_void,
    /// Called when a changeset is stored. The caller can use this as a
    /// signal that data has changed and needs to be written to disk.
    /// Returns 0 on success, non-zero on error.
    pub on_store_fn:
        Option<unsafe extern "C" fn(context: *mut c_void, wallet_id: *const u8) -> i32>,
    /// Called when flush is requested. Returns 0 on success.
    pub on_flush_fn:
        Option<unsafe extern "C" fn(context: *mut c_void, wallet_id: *const u8) -> i32>,
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
