use wasm_bindgen::prelude::*;

use crate::buffer::Buffer;

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

/// Compute the platform sighash from an Orchard bundle commitment and extra data.
///
/// `sighash = SHA-256("DashPlatformSighash" || bundleCommitment || extraData)`
///
/// - For shield and shielded_transfer transitions, `extraData` should be empty.
/// - For unshield transitions, `extraData` = `outputAddress || amount (LE u64)`.
/// - For shielded withdrawal transitions, `extraData` = `outputScript || amount (LE u64)`.
///
/// @param {Buffer} bundleCommitment - 32-byte Orchard bundle commitment (BLAKE2b-256 per ZIP-244)
/// @param {Buffer} extraData - Transparent field binding (empty for shielded-only transitions)
/// @returns {Buffer} 32-byte SHA-256 sighash
#[wasm_bindgen(js_name = computePlatformSighash)]
pub fn compute_platform_sighash_wasm(
    bundle_commitment: &[u8],
    extra_data: &[u8],
) -> Result<Buffer, JsValue> {
    if bundle_commitment.len() != 32 {
        return Err(JsValue::from_str(&format!(
            "bundleCommitment must be exactly 32 bytes, got {}",
            bundle_commitment.len()
        )));
    }
    let commitment: &[u8; 32] = bundle_commitment.try_into().unwrap();
    let result = dpp::shielded::compute_platform_sighash(commitment, extra_data);
    Ok(Buffer::from_bytes(&result))
}
