//! Orchard cryptographic FFI functions for iOS.
//!
//! Provides key derivation, bundle building, and note decryption functions
//! that perform all Orchard cryptography internally and return JSON results.
//! This enables iOS to build shielded bundles without needing to construct
//! the complex `DashSDKOrchardBundleParams` externally.

mod address;
mod bundle_build;
mod decrypt;

pub use address::*;
pub use bundle_build::*;
pub use decrypt::*;

use std::sync::OnceLock;

use dash_sdk::dpp::shielded::builder::OrchardProver;
use dash_sdk::grovedb_commitment_tree::ProvingKey;

use crate::DashSDKResult;

/// Global cached proving key. Building the Halo 2 proving key takes ~30 seconds
/// on first call, so we cache it for the lifetime of the process.
static PROVING_KEY: OnceLock<ProvingKey> = OnceLock::new();

/// Cached prover that wraps the global `OnceLock` proving key.
pub(crate) struct CachedProver;

impl OrchardProver for CachedProver {
    fn proving_key(&self) -> &ProvingKey {
        PROVING_KEY.get_or_init(ProvingKey::build)
    }
}

/// Pre-build and cache the Halo 2 proving key (~30s on first call, instant after).
///
/// Call this on a background thread at app startup so that subsequent bundle
/// building calls do not incur the proving key generation latency.
///
/// # Returns
/// `DashSDKResult` with no data on success.
///
/// # Safety
/// This function has no pointer parameters and is safe to call from any thread.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_shielded_warmup_proving_key() -> DashSDKResult {
    let _ = PROVING_KEY.get_or_init(ProvingKey::build);
    DashSDKResult::success(std::ptr::null_mut())
}
