//! Context Provider FFI bindings
//!
//! This module provides FFI bindings for configuring context providers,
//! allowing the Platform SDK to connect to Core SDK for proof verification.

use std::ffi::c_char;
use std::sync::Arc;

use drive_proof_verifier::ContextProvider;

use crate::context_callbacks::{CallbackContextProvider, ContextProviderCallbacks};

/// Handle for Core SDK that can be passed to Platform SDK
/// This matches the definition from dash_spv_ffi.h
#[repr(C)]
pub struct CoreSDKHandle {
    pub client: *mut std::ffi::c_void,
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

// Note: Core SDK FFI types are opaque to rs-sdk-ffi and referenced via raw pointers.

// Note: Core SDK functions are now provided via callbacks instead of direct linking
// This allows Platform SDK to be built independently and linked at runtime

// Note: The deprecated CoreBridgeContextProvider has been removed.

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
