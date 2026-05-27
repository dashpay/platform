//! Context Provider Callbacks for decoupling Platform SDK from Core SDK
//!
//! This module provides function pointer types that allow Platform SDK to call
//! Core SDK functionality without direct compile-time dependencies.

use once_cell::sync::OnceCell;
use std::ffi::c_char;
use std::os::raw::c_void;
use std::sync::Arc;
use std::sync::RwLock;

use dash_sdk::dpp::data_contract::TokenConfiguration;
use dash_sdk::dpp::prelude::{CoreBlockHeight, DataContract, Identifier};
use dash_sdk::dpp::version::PlatformVersion;
use dash_sdk::error::ContextProviderError;
use drive_proof_verifier::ContextProvider;

use crate::context_provider::CoreSDKHandle;

/// Result type for FFI callbacks
#[repr(C)]
pub struct CallbackResult {
    pub success: bool,
    pub error_code: i32,
    pub error_message: *const c_char,
}

/// Function pointer type for getting platform activation height
pub type GetPlatformActivationHeightFn =
    unsafe extern "C" fn(handle: *mut c_void, out_height: *mut u32) -> CallbackResult;

/// Function pointer type for getting quorum public key
pub type GetQuorumPublicKeyFn = unsafe extern "C" fn(
    handle: *mut c_void,
    quorum_type: u32,
    quorum_hash: *const u8,
    core_chain_locked_height: u32,
    out_pubkey: *mut u8,
) -> CallbackResult;

/// Container for context provider callbacks
#[repr(C)]
pub struct ContextProviderCallbacks {
    /// Handle to the Core SDK instance
    pub core_handle: *mut c_void,
    /// Function to get platform activation height
    pub get_platform_activation_height: GetPlatformActivationHeightFn,
    /// Function to get quorum public key
    pub get_quorum_public_key: GetQuorumPublicKeyFn,
}

// SAFETY: The callbacks are function pointers and the handle is only used within those callbacks
unsafe impl Send for ContextProviderCallbacks {}
unsafe impl Sync for ContextProviderCallbacks {}

/// Global callbacks storage
static GLOBAL_CALLBACKS: OnceCell<RwLock<Option<ContextProviderCallbacks>>> = OnceCell::new();

/// Initialize global callbacks storage
pub fn init_global_callbacks() {
    let _ = GLOBAL_CALLBACKS.set(RwLock::new(None));
}

/// Set global context provider callbacks
///
/// # Safety
/// The callbacks must remain valid for the lifetime of the SDK
pub unsafe fn set_global_callbacks(
    callbacks: ContextProviderCallbacks,
) -> Result<(), &'static str> {
    let storage = GLOBAL_CALLBACKS.get_or_init(|| RwLock::new(None));
    let mut guard = storage
        .write()
        .map_err(|_| "Failed to acquire write lock")?;
    *guard = Some(callbacks);
    Ok(())
}

/// Get global context provider callbacks
pub fn get_global_callbacks() -> Option<ContextProviderCallbacks> {
    GLOBAL_CALLBACKS
        .get()
        .and_then(|storage| storage.read().ok())
        .and_then(|guard| {
            guard.as_ref().map(|cb| ContextProviderCallbacks {
                core_handle: cb.core_handle,
                get_platform_activation_height: cb.get_platform_activation_height,
                get_quorum_public_key: cb.get_quorum_public_key,
            })
        })
}

/// Context provider implementation using callbacks
pub struct CallbackContextProvider {
    callbacks: ContextProviderCallbacks,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallbackContextProviderCreateError {
    NullCoreSdkHandle,
    NullCoreClientHandle,
    MissingGlobalCallbacks,
}

impl CallbackContextProvider {
    /// Create a new callback-based context provider
    pub fn new(callbacks: ContextProviderCallbacks) -> Self {
        Self { callbacks }
    }

    /// Create from global callbacks if available
    pub fn from_global() -> Option<Self> {
        get_global_callbacks().map(Self::new)
    }

    /// Create a callback-based provider using the globally registered callbacks
    /// but replacing the callback handle with the provided Core SDK client handle.
    ///
    /// # Safety
    /// `core_sdk_handle` must either be null or a valid, dereferenceable
    /// pointer to a `CoreSDKHandle` for the duration of the call. The
    /// inner `client` pointer is not dereferenced here; it is only stored
    /// and later passed back to the registered callbacks, which are
    /// responsible for validating it.
    pub unsafe fn from_core_sdk_handle(
        core_sdk_handle: *mut CoreSDKHandle,
    ) -> Result<Self, CallbackContextProviderCreateError> {
        if core_sdk_handle.is_null() {
            return Err(CallbackContextProviderCreateError::NullCoreSdkHandle);
        }

        let core_sdk_handle = unsafe { &*core_sdk_handle };
        if core_sdk_handle.client.is_null() {
            return Err(CallbackContextProviderCreateError::NullCoreClientHandle);
        }

        let mut callbacks = get_global_callbacks()
            .ok_or(CallbackContextProviderCreateError::MissingGlobalCallbacks)?;
        callbacks.core_handle = core_sdk_handle.client;

        Ok(Self::new(callbacks))
    }
}

// SAFETY: CallbackContextProvider only contains function pointers and a handle
unsafe impl Send for CallbackContextProvider {}
unsafe impl Sync for CallbackContextProvider {}

impl ContextProvider for CallbackContextProvider {
    fn get_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        core_chain_locked_height: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        let callback = self.callbacks.get_quorum_public_key;

        unsafe {
            let mut public_key = [0u8; 48];

            let result = callback(
                self.callbacks.core_handle,
                quorum_type,
                quorum_hash.as_ptr(),
                core_chain_locked_height,
                public_key.as_mut_ptr(),
            );

            if result.success {
                Ok(public_key)
            } else {
                let error_msg = if result.error_message.is_null() {
                    format!(
                        "Failed to get quorum public key: error code {}",
                        result.error_code
                    )
                } else {
                    let c_str = std::ffi::CStr::from_ptr(result.error_message);
                    c_str.to_string_lossy().into_owned()
                };
                Err(ContextProviderError::Generic(error_msg))
            }
        }
    }

    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        let callback = self.callbacks.get_platform_activation_height;

        unsafe {
            let mut height = 0u32;
            let result = callback(self.callbacks.core_handle, &mut height);

            if result.success {
                Ok(height)
            } else {
                let error_msg = if result.error_message.is_null() {
                    format!(
                        "Failed to get platform activation height: error code {}",
                        result.error_code
                    )
                } else {
                    let c_str = std::ffi::CStr::from_ptr(result.error_message);
                    c_str.to_string_lossy().into_owned()
                };
                Err(ContextProviderError::Generic(error_msg))
            }
        }
    }

    fn get_data_contract(
        &self,
        _data_contract_id: &Identifier,
        _platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        // TODO: Implement when Core SDK supports data contract retrieval
        Ok(None)
    }

    fn get_token_configuration(
        &self,
        _token_id: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
        // TODO: Implement when Core SDK supports token configuration retrieval
        Ok(None)
    }
}

/// Test-only utilities for callers (including other test modules in this
/// crate) that need to mutate the process-wide `GLOBAL_CALLBACKS` without
/// races. Wrapped in `cfg(test)` so it never ships in release builds.
#[cfg(test)]
pub(crate) mod test_support {
    use super::*;
    use std::sync::{Mutex, MutexGuard};

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    /// Guard returned by `lock_global_callbacks_for_test`. Holds a
    /// crate-wide test mutex so that tests touching `GLOBAL_CALLBACKS`
    /// are serialized, and restores the previously installed callbacks
    /// (or `None`) on drop so that subsequent tests start clean.
    pub struct GlobalCallbacksTestGuard {
        _lock: MutexGuard<'static, ()>,
        previous: Option<ContextProviderCallbacks>,
    }

    impl Drop for GlobalCallbacksTestGuard {
        fn drop(&mut self) {
            let storage = GLOBAL_CALLBACKS.get_or_init(|| RwLock::new(None));
            if let Ok(mut guard) = storage.write() {
                *guard = self.previous.take();
            }
        }
    }

    /// Acquire exclusive access to the global callback storage for the
    /// duration of a test, snapshotting any previously installed callbacks
    /// so they can be restored on guard drop.
    pub fn lock_global_callbacks_for_test() -> GlobalCallbacksTestGuard {
        let lock = TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let previous = get_global_callbacks();
        GlobalCallbacksTestGuard {
            _lock: lock,
            previous,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static LAST_CORE_HANDLE: AtomicUsize = AtomicUsize::new(0);

    extern "C" fn get_height_cb(handle: *mut c_void, out: *mut u32) -> CallbackResult {
        LAST_CORE_HANDLE.store(handle as usize, Ordering::SeqCst);
        unsafe {
            if !out.is_null() {
                *out = 42;
            }
        }
        CallbackResult {
            success: true,
            error_code: 0,
            error_message: std::ptr::null(),
        }
    }

    extern "C" fn get_quorum_pk_cb(
        _handle: *mut c_void,
        _quorum_type: u32,
        _quorum_hash: *const u8,
        _core_chain_locked_height: u32,
        out: *mut u8,
    ) -> CallbackResult {
        unsafe {
            if !out.is_null() {
                std::ptr::write_bytes(out, 0, 48);
            }
        }
        CallbackResult {
            success: true,
            error_code: 0,
            error_message: std::ptr::null(),
        }
    }

    #[test]
    fn from_core_sdk_handle_uses_client_handle_instead_of_global_handle() {
        let _guard = test_support::lock_global_callbacks_for_test();

        let global_handle = std::ptr::dangling_mut::<c_void>();
        let core_client_handle = std::ptr::without_provenance_mut::<c_void>(0x1234usize);

        unsafe {
            set_global_callbacks(ContextProviderCallbacks {
                core_handle: global_handle,
                get_platform_activation_height: get_height_cb,
                get_quorum_public_key: get_quorum_pk_cb,
            })
            .expect("global callbacks should be set");
        }

        let mut core_sdk_handle = CoreSDKHandle {
            client: core_client_handle,
        };

        let provider = unsafe {
            CallbackContextProvider::from_core_sdk_handle(&mut core_sdk_handle)
                .expect("provider should be created from core SDK handle")
        };

        let height = provider
            .get_platform_activation_height()
            .expect("callback should succeed");

        assert_eq!(height, 42);
        assert_eq!(
            LAST_CORE_HANDLE.load(Ordering::SeqCst),
            core_client_handle as usize
        );
        assert_ne!(
            LAST_CORE_HANDLE.load(Ordering::SeqCst),
            global_handle as usize
        );
    }

    #[test]
    fn from_core_sdk_handle_rejects_null_client_handle() {
        let _guard = test_support::lock_global_callbacks_for_test();

        unsafe {
            set_global_callbacks(ContextProviderCallbacks {
                core_handle: std::ptr::dangling_mut::<c_void>(),
                get_platform_activation_height: get_height_cb,
                get_quorum_public_key: get_quorum_pk_cb,
            })
            .expect("global callbacks should be set");
        }

        let mut core_sdk_handle = CoreSDKHandle {
            client: std::ptr::null_mut(),
        };

        let err =
            match unsafe { CallbackContextProvider::from_core_sdk_handle(&mut core_sdk_handle) } {
                Ok(_) => panic!("null client handles must be rejected"),
                Err(err) => err,
            };

        assert_eq!(
            err,
            CallbackContextProviderCreateError::NullCoreClientHandle
        );
    }
}
