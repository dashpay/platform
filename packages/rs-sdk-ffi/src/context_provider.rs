//! Context Provider FFI bindings
//!
//! This module provides FFI bindings for configuring context providers,
//! allowing the Platform SDK to connect to Core SDK for proof verification.

use std::sync::Arc;

use dash_sdk::dpp::data_contract::TokenConfiguration;
use dash_sdk::dpp::prelude::{CoreBlockHeight, DataContract, Identifier};
use dash_sdk::dpp::version::PlatformVersion;
use dash_sdk::error::ContextProviderError;
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
/// Adapter wrapping any [`ContextProvider`] as an opaque
/// [`ContextProviderHandle`] for the SDK. Public so sibling FFI crates
/// (e.g. `platform-wallet-ffi`) can install a native Rust provider.
pub struct ContextProviderWrapper {
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

/// Context provider that serves the security-critical quorum public key (and
/// platform activation height) from `quorum`, while delegating data-contract
/// and token-configuration lookups to `aux`.
///
/// Installing an SPV quorum provider must not lose the SDK's ability to resolve
/// contracts and token configurations: those feed proof verification (e.g.
/// token perpetual-distribution verification calls `get_token_configuration`
/// before checking the Tenderdash signature) and the SPV provider cannot supply
/// them. This composite keeps quorum verification on SPV-synced state while
/// contract/token resolution stays on the retained trusted provider.
pub struct CompositeContextProvider {
    quorum: Arc<dyn ContextProvider>,
    aux: Arc<dyn ContextProvider>,
}

impl CompositeContextProvider {
    pub fn new(quorum: Arc<dyn ContextProvider>, aux: Arc<dyn ContextProvider>) -> Self {
        Self { quorum, aux }
    }
}

impl ContextProvider for CompositeContextProvider {
    fn get_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        core_chain_locked_height: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        self.quorum
            .get_quorum_public_key(quorum_type, quorum_hash, core_chain_locked_height)
    }

    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        self.quorum.get_platform_activation_height()
    }

    fn get_data_contract(
        &self,
        id: &Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        self.aux.get_data_contract(id, platform_version)
    }

    fn get_token_configuration(
        &self,
        token_id: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
        self.aux.get_token_configuration(token_id)
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
