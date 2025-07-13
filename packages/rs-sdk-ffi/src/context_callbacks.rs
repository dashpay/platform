//! Context Provider Callbacks for decoupling Platform SDK from Core SDK
//!
//! This module provides function pointer types that allow Platform SDK to call
//! Core SDK functionality without direct compile-time dependencies.

use std::ffi::c_char;
use std::os::raw::c_void;
use std::sync::Arc;
use once_cell::sync::OnceCell;
use std::sync::RwLock;

use drive_proof_verifier::ContextProvider;
use dash_sdk::error::ContextProviderError;
use dash_sdk::dpp::data_contract::TokenConfiguration;
use dash_sdk::dpp::prelude::{DataContract, Identifier, CoreBlockHeight};
use dash_sdk::dpp::version::PlatformVersion;

/// Result type for FFI callbacks
#[repr(C)]
pub struct CallbackResult {
    pub success: bool,
    pub error_code: i32,
    pub error_message: *const c_char,
}

/// Function pointer type for getting platform activation height
pub type GetPlatformActivationHeightFn = unsafe extern "C" fn(
    handle: *mut c_void,
    out_height: *mut u32,
) -> CallbackResult;

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
pub unsafe fn set_global_callbacks(callbacks: ContextProviderCallbacks) -> Result<(), &'static str> {
    let storage = GLOBAL_CALLBACKS.get_or_init(|| RwLock::new(None));
    let mut guard = storage.write().map_err(|_| "Failed to acquire write lock")?;
    *guard = Some(callbacks);
    Ok(())
}

/// Get global context provider callbacks
pub fn get_global_callbacks() -> Option<ContextProviderCallbacks> {
    GLOBAL_CALLBACKS.get()
        .and_then(|storage| storage.read().ok())
        .and_then(|guard| guard.as_ref().map(|cb| ContextProviderCallbacks {
            core_handle: cb.core_handle,
            get_platform_activation_height: cb.get_platform_activation_height,
            get_quorum_public_key: cb.get_quorum_public_key,
        }))
}

/// Context provider implementation using callbacks
pub struct CallbackContextProvider {
    callbacks: ContextProviderCallbacks,
}

impl CallbackContextProvider {
    /// Create a new callback-based context provider
    pub fn new(callbacks: ContextProviderCallbacks) -> Self {
        Self { callbacks }
    }

    /// Create from global callbacks if available
    pub fn from_global() -> Option<Self> {
        get_global_callbacks().map(|callbacks| Self::new(callbacks))
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
                    format!("Failed to get quorum public key: error code {}", result.error_code)
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
            let result = callback(
                self.callbacks.core_handle,
                &mut height,
            );

            if result.success {
                Ok(height)
            } else {
                let error_msg = if result.error_message.is_null() {
                    format!("Failed to get platform activation height: error code {}", result.error_code)
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