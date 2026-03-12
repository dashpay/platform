pub mod shield_from_asset_lock_transition;
pub mod shield_transition;
pub mod shielded_transfer_transition;
pub mod shielded_withdrawal_transition;
pub mod unshield_transition;

pub use shield_from_asset_lock_transition::ShieldFromAssetLockTransitionWasm;
pub use shield_transition::ShieldTransitionWasm;
pub use shielded_transfer_transition::ShieldedTransferTransitionWasm;
pub use shielded_withdrawal_transition::ShieldedWithdrawalTransitionWasm;
pub use unshield_transition::UnshieldTransitionWasm;

use crate::error::WasmDppResult;
use wasm_bindgen::prelude::wasm_bindgen;

/// Compute the platform sighash from an Orchard bundle commitment and extra data.
///
/// `sighash = SHA-256("DashPlatformSighash" || bundleCommitment || extraData)`
///
/// - For shield and shielded_transfer transitions, `extraData` should be empty.
/// - For unshield transitions, `extraData` = serialized `outputAddress` bytes.
/// - For shielded withdrawal transitions, `extraData` = `outputScript` bytes.
#[wasm_bindgen(js_name = computePlatformSighash)]
pub fn compute_platform_sighash_wasm(
    bundle_commitment: &[u8],
    extra_data: &[u8],
) -> WasmDppResult<Vec<u8>> {
    if bundle_commitment.len() != 32 {
        return Err(crate::error::WasmDppError::invalid_argument(&format!(
            "bundleCommitment must be exactly 32 bytes, got {}",
            bundle_commitment.len()
        )));
    }
    let commitment: &[u8; 32] = bundle_commitment.try_into().expect("checked length above");
    let result = dpp::shielded::compute_platform_sighash(commitment, extra_data);
    Ok(result.to_vec())
}
