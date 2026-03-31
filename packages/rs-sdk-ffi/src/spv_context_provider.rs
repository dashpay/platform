//! SPV-based Context Provider (FFI bridge)
//!
//! Implements `ContextProvider` by calling the `dash-spv-ffi` crate's
//! FFI functions directly in Rust (same binary, no actual FFI boundary
//! crossing), avoiding a round-trip through Swift callbacks.
//!
//! # Migration path
//!
//! The long-term replacement is [`platform_wallet::spv_context_provider::SpvContextProvider`],
//! which holds an `Arc<RwLock<MasternodeListEngine>>` directly and has zero FFI
//! involvement. To use it, `dash-spv-ffi` needs to expose a public accessor for
//! the `MasternodeListEngine` inside `FFIDashSpvClient` (its `inner` field is
//! currently `pub(crate)`). Once that accessor exists, this bridge module can be
//! removed and the pure-Rust provider used instead.

use std::os::raw::c_void;
use std::sync::Arc;

use dash_sdk::dpp::data_contract::TokenConfiguration;
use dash_sdk::dpp::prelude::{CoreBlockHeight, DataContract, Identifier};
use dash_sdk::dpp::version::PlatformVersion;
use dash_sdk::error::ContextProviderError;
use drive_proof_verifier::ContextProvider;

/// Context provider backed by an SPV client's synced masternode data.
///
/// Calls `ffi_dash_spv_get_quorum_public_key` and
/// `ffi_dash_spv_get_platform_activation_height` from the `dash-spv-ffi` crate
/// directly, without crossing the FFI boundary into Swift.
pub struct SpvContextProvider {
    /// Raw pointer to the `FFIDashSpvClient`. The SPV client must outlive this provider.
    spv_client: *mut c_void,
}

// SAFETY: The pointer is only used inside the FFI calls which are themselves thread-safe
// (the SPV client uses internal locking).
unsafe impl Send for SpvContextProvider {}
unsafe impl Sync for SpvContextProvider {}

impl SpvContextProvider {
    /// Create a new SPV context provider.
    ///
    /// # Safety
    /// `spv_client` must be a valid `*mut FFIDashSpvClient` that outlives this provider.
    pub unsafe fn new(spv_client: *mut c_void) -> Self {
        Self { spv_client }
    }
}

impl ContextProvider for SpvContextProvider {
    fn get_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        core_chain_locked_height: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        let client = self.spv_client as *mut dash_spv_ffi::FFIDashSpvClient;

        let mut public_key = [0u8; 48];

        let result = unsafe {
            dash_spv_ffi::ffi_dash_spv_get_quorum_public_key(
                client,
                quorum_type,
                quorum_hash.as_ptr(),
                core_chain_locked_height,
                public_key.as_mut_ptr(),
                48,
            )
        };

        if result.error_code == 0 {
            Ok(public_key)
        } else {
            let error_msg = if result.error_message.is_null() {
                format!(
                    "SPV quorum key lookup failed: error code {}",
                    result.error_code
                )
            } else {
                let c_str = unsafe { std::ffi::CStr::from_ptr(result.error_message) };
                c_str.to_string_lossy().into_owned()
            };
            Err(ContextProviderError::Generic(error_msg))
        }
    }

    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        let client = self.spv_client as *mut dash_spv_ffi::FFIDashSpvClient;

        let mut height = 0u32;

        let result = unsafe {
            dash_spv_ffi::ffi_dash_spv_get_platform_activation_height(client, &mut height)
        };

        if result.error_code == 0 {
            Ok(height)
        } else {
            let error_msg = if result.error_message.is_null() {
                format!(
                    "SPV platform activation height lookup failed: error code {}",
                    result.error_code
                )
            } else {
                let c_str = unsafe { std::ffi::CStr::from_ptr(result.error_message) };
                c_str.to_string_lossy().into_owned()
            };
            Err(ContextProviderError::Generic(error_msg))
        }
    }

    fn get_data_contract(
        &self,
        _data_contract_id: &Identifier,
        _platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        Ok(None)
    }

    fn get_token_configuration(
        &self,
        _token_id: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
        Ok(None)
    }
}
