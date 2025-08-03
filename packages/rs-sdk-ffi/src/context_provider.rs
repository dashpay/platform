//! Context Provider FFI bindings
//!
//! This module provides FFI bindings for configuring context providers,
//! allowing the Platform SDK to connect to Core SDK for proof verification.

use std::ffi::{c_char, CStr};
use std::sync::Arc;

use dash_sdk::dpp::data_contract::TokenConfiguration;
use dash_sdk::dpp::prelude::{CoreBlockHeight, DataContract, Identifier};
use dash_sdk::dpp::version::PlatformVersion;
use dash_sdk::error::ContextProviderError;
use drive_proof_verifier::ContextProvider;

use crate::context_callbacks::{CallbackContextProvider, ContextProviderCallbacks};
use crate::{DashSDKError, DashSDKErrorCode, FFIError};

/// Handle for Core SDK that can be passed to Platform SDK
/// This matches the definition from dash_spv_ffi.h
#[repr(C)]
pub struct CoreSDKHandle {
    pub client: *mut FFIDashSpvClient,
}

/// Opaque handle to a context provider
#[repr(C)]
pub struct ContextProviderHandle {
    _private: [u8; 0],
}

/// Internal wrapper for context provider
pub(crate) struct ContextProviderWrapper {
    provider: Arc<dyn ContextProvider>,
}

impl ContextProviderWrapper {
    pub fn new(provider: impl ContextProvider + 'static) -> Self {
        Self {
            provider: Arc::new(provider),
        }
    }

    pub fn provider(&self) -> Arc<dyn ContextProvider> {
        Arc::clone(&self.provider)
    }
}

/// Bridge context provider that delegates to Core SDK via callbacks
/// This is now deprecated in favor of CallbackContextProvider
#[deprecated(since = "2.0.0", note = "Use CallbackContextProvider instead")]
struct CoreBridgeContextProvider {
    client: *mut FFIDashSpvClient,
    _rpc_url: Option<String>,
    _rpc_user: Option<String>,
    _rpc_password: Option<String>,
}

// SAFETY: CoreBridgeContextProvider is Send if we ensure proper synchronization
unsafe impl Send for CoreBridgeContextProvider {}
unsafe impl Sync for CoreBridgeContextProvider {}

impl CoreBridgeContextProvider {
    fn new(
        client: *mut FFIDashSpvClient,
        rpc_url: Option<String>,
        rpc_user: Option<String>,
        rpc_password: Option<String>,
    ) -> Self {
        Self {
            client,
            _rpc_url: rpc_url,
            _rpc_user: rpc_user,
            _rpc_password: rpc_password,
        }
    }
}

// FFI Result type from Core SDK
#[repr(C)]
pub(crate) struct FFIResult {
    pub error_code: i32,
    pub error_message: *const c_char,
}

// FFI Client type from Core SDK
#[repr(C)]
pub(crate) struct FFIDashSpvClient {
    pub _opaque: [u8; 0],
}

// Note: Core SDK functions are now provided via callbacks instead of direct linking
// This allows Platform SDK to be built independently and linked at runtime

impl ContextProvider for CoreBridgeContextProvider {
    fn get_quorum_public_key(
        &self,
        _quorum_type: u32,
        _quorum_hash: [u8; 32],
        _core_chain_locked_height: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        // This implementation is deprecated - use CallbackContextProvider instead
        Err(ContextProviderError::Generic(
            "CoreBridgeContextProvider is deprecated. Use CallbackContextProvider with registered callbacks instead.".to_string()
        ))
    }

    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        // This implementation is deprecated - use CallbackContextProvider instead
        Err(ContextProviderError::Generic(
            "CoreBridgeContextProvider is deprecated. Use CallbackContextProvider with registered callbacks instead.".to_string()
        ))
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

/// Create a context provider from a Core SDK handle (DEPRECATED)
///
/// This function is deprecated. Use dash_sdk_context_provider_from_callbacks instead.
///
/// # Safety
/// - `core_handle` must be a valid Core SDK handle
/// - String parameters must be valid UTF-8 C strings or null
#[no_mangle]
#[deprecated(
    since = "2.0.0",
    note = "Use dash_sdk_context_provider_from_callbacks instead"
)]
pub unsafe extern "C" fn dash_sdk_context_provider_from_core(
    core_handle: *mut CoreSDKHandle,
    _core_rpc_url: *const c_char,
    _core_rpc_user: *const c_char,
    _core_rpc_password: *const c_char,
) -> *mut ContextProviderHandle {
    if core_handle.is_null() {
        return std::ptr::null_mut();
    }

    // Try to create from global callbacks if available
    if let Some(provider) = CallbackContextProvider::from_global() {
        let wrapper = Box::new(ContextProviderWrapper::new(provider));
        return Box::into_raw(wrapper) as *mut ContextProviderHandle;
    }

    // No callbacks registered - return null
    std::ptr::null_mut()
}

/// Create a context provider from callbacks
///
/// # Safety
/// - `callbacks` must contain valid function pointers
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_context_provider_from_callbacks(
    callbacks: *const ContextProviderCallbacks,
) -> *mut ContextProviderHandle {
    if callbacks.is_null() {
        return std::ptr::null_mut();
    }

    let callbacks = &*callbacks;
    let provider = CallbackContextProvider::new(ContextProviderCallbacks {
        core_handle: callbacks.core_handle,
        get_platform_activation_height: callbacks.get_platform_activation_height,
        get_quorum_public_key: callbacks.get_quorum_public_key,
    });

    let wrapper = Box::new(ContextProviderWrapper::new(provider));
    Box::into_raw(wrapper) as *mut ContextProviderHandle
}

/// Destroy a context provider handle
///
/// # Safety
/// - `handle` must be a valid context provider handle or null
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_context_provider_destroy(handle: *mut ContextProviderHandle) {
    if !handle.is_null() {
        let _ = Box::from_raw(handle as *mut ContextProviderWrapper);
    }
}
