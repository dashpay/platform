use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Handle type for FFI objects
pub type Handle = u64;

/// Null handle constant
pub const NULL_HANDLE: Handle = 0;

/// Global handle counter
static NEXT_HANDLE: AtomicU64 = AtomicU64::new(1);

/// Generate next unique handle
pub fn next_handle() -> Handle {
    NEXT_HANDLE.fetch_add(1, Ordering::SeqCst)
}

/// Handle storage for a specific type
pub struct HandleStorage<T> {
    items: RwLock<HashMap<Handle, T>>,
}

impl<T> HandleStorage<T> {
    pub fn new() -> Self {
        Self {
            items: RwLock::new(HashMap::new()),
        }
    }

    pub fn insert(&self, item: T) -> Handle {
        let handle = next_handle();
        self.items.write().insert(handle, item);
        handle
    }

    pub fn get(
        &self,
        handle: Handle,
    ) -> Option<parking_lot::RwLockReadGuard<'_, HashMap<Handle, T>>> {
        let guard = self.items.read();
        if guard.contains_key(&handle) {
            Some(guard)
        } else {
            None
        }
    }

    pub fn get_mut(
        &self,
        handle: Handle,
    ) -> Option<parking_lot::RwLockWriteGuard<'_, HashMap<Handle, T>>> {
        let guard = self.items.write();
        if guard.contains_key(&handle) {
            Some(guard)
        } else {
            None
        }
    }

    pub fn remove(&self, handle: Handle) -> Option<T> {
        self.items.write().remove(&handle)
    }

    pub fn with_item<F, R>(&self, handle: Handle, f: F) -> Option<R>
    where
        F: FnOnce(&T) -> R,
    {
        let guard = self.items.read();
        guard.get(&handle).map(f)
    }

    pub fn with_item_mut<F, R>(&self, handle: Handle, f: F) -> Option<R>
    where
        F: FnOnce(&mut T) -> R,
    {
        let mut guard = self.items.write();
        guard.get_mut(&handle).map(f)
    }
}

impl<T> Default for HandleStorage<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Storage for PlatformWalletInfo handles
pub static WALLET_INFO_STORAGE: Lazy<HandleStorage<platform_wallet::PlatformWalletInfo>> =
    Lazy::new(HandleStorage::new);

/// Storage for IdentityManager handles
pub static IDENTITY_MANAGER_STORAGE: Lazy<HandleStorage<platform_wallet::IdentityManager>> =
    Lazy::new(HandleStorage::new);

/// Storage for ManagedIdentity handles
pub static MANAGED_IDENTITY_STORAGE: Lazy<HandleStorage<platform_wallet::ManagedIdentity>> =
    Lazy::new(HandleStorage::new);

/// Storage for CoreWallet handles.
///
/// `CoreWallet` is generic over the broadcaster type; the FFI pins
/// it to `SpvBroadcaster` — the only production broadcaster.
pub static CORE_WALLET_STORAGE: Lazy<
    HandleStorage<platform_wallet::CoreWallet<platform_wallet::broadcaster::SpvBroadcaster>>,
> = Lazy::new(HandleStorage::new);

/// Storage for AssetLockManager handles (pinned to `SpvBroadcaster`).
pub static ASSET_LOCK_MANAGER_STORAGE: Lazy<
    HandleStorage<
        std::sync::Arc<
            platform_wallet::AssetLockManager<platform_wallet::broadcaster::SpvBroadcaster>,
        >,
    >,
> = Lazy::new(HandleStorage::new);

/// Storage for PlatformAddressWallet handles
pub static PLATFORM_ADDRESS_WALLET_STORAGE: Lazy<
    HandleStorage<platform_wallet::wallet::platform_addresses::PlatformAddressWallet>,
> = Lazy::new(HandleStorage::new);

/// FFI-owned manager plus the optional lease that connects its running SPV
/// runtime to an SDK's fixed SPV-capable context provider.
pub struct PlatformWalletManagerHandle {
    manager: platform_wallet::PlatformWalletManager<crate::persistence::FFIPersister>,
    spv_source: Option<Arc<platform_wallet::spv_context_provider::SpvQuorumSource>>,
    spv_source_controller: Option<Arc<rs_sdk_ffi::SpvSourceController>>,
    spv_source_lease: parking_lot::Mutex<Option<rs_sdk_ffi::SpvSourceLease>>,
}

impl PlatformWalletManagerHandle {
    pub(crate) fn new(
        manager: platform_wallet::PlatformWalletManager<crate::persistence::FFIPersister>,
        spv_source: Option<Arc<platform_wallet::spv_context_provider::SpvQuorumSource>>,
        spv_source_controller: Option<Arc<rs_sdk_ffi::SpvSourceController>>,
    ) -> Self {
        Self {
            manager,
            spv_source,
            spv_source_controller,
            spv_source_lease: parking_lot::Mutex::new(None),
        }
    }

    pub(crate) fn acquire_spv_source(&self) -> Result<(), String> {
        let Some(controller) = &self.spv_source_controller else {
            return Ok(());
        };
        let source = self
            .spv_source
            .as_ref()
            .ok_or_else(|| "SPV source controller has no quorum source".to_string())?;
        let readiness_source = Arc::clone(source);
        let quorum_source: Arc<dyn dash_sdk::platform::ContextProvider> =
            Arc::clone(source) as Arc<dyn dash_sdk::platform::ContextProvider>;
        let lease = controller
            .acquire(quorum_source, Arc::new(move || readiness_source.is_ready()))
            .map_err(str::to_string)?;
        *self.spv_source_lease.lock() = Some(lease);
        Ok(())
    }

    pub(crate) fn release_spv_source(&self) {
        let _ = self.spv_source_lease.lock().take();
    }
}

impl Deref for PlatformWalletManagerHandle {
    type Target = platform_wallet::PlatformWalletManager<crate::persistence::FFIPersister>;

    fn deref(&self) -> &Self::Target {
        &self.manager
    }
}

/// Storage for PlatformWalletManager handles.
///
/// The manager is generic over the persister type; the FFI binds it
/// to the callback-based [`FFIPersister`](crate::persistence::FFIPersister).
pub static PLATFORM_WALLET_MANAGER_STORAGE: Lazy<HandleStorage<PlatformWalletManagerHandle>> =
    Lazy::new(HandleStorage::new);

/// Storage for PlatformWallet handles
pub static PLATFORM_WALLET_STORAGE: Lazy<
    HandleStorage<std::sync::Arc<platform_wallet::PlatformWallet>>,
> = Lazy::new(HandleStorage::new);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_next_handle_unique() {
        let h1 = next_handle();
        let h2 = next_handle();
        let h3 = next_handle();

        assert_ne!(h1, h2);
        assert_ne!(h2, h3);
        assert_ne!(h1, h3);
    }

    #[test]
    fn test_handle_storage_insert_get() {
        let storage = HandleStorage::<String>::new();
        let handle = storage.insert("test".to_string());

        assert_ne!(handle, NULL_HANDLE);

        let result = storage.with_item(handle, |item| item.clone());
        assert_eq!(result, Some("test".to_string()));
    }

    #[test]
    fn test_handle_storage_remove() {
        let storage = HandleStorage::<String>::new();
        let handle = storage.insert("test".to_string());

        let removed = storage.remove(handle);
        assert_eq!(removed, Some("test".to_string()));

        let result = storage.with_item(handle, |item| item.clone());
        assert_eq!(result, None);
    }

    #[test]
    fn test_handle_storage_get_invalid() {
        let storage = HandleStorage::<String>::new();
        let result = storage.with_item(9999, |item| item.clone());
        assert_eq!(result, None);
    }

    #[test]
    fn test_handle_storage_with_item_mut() {
        let storage = HandleStorage::<String>::new();
        let handle = storage.insert("test".to_string());

        storage.with_item_mut(handle, |item| {
            item.push_str("_modified");
        });

        let result = storage.with_item(handle, |item| item.clone());
        assert_eq!(result, Some("test_modified".to_string()));
    }
}
