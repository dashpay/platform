pub mod address_witness;
pub mod identity_create_from_shielded_pool_transition;
pub mod orchard_action;
pub mod shield_from_asset_lock_transition;
pub mod shield_transition;
pub mod shielded_transfer_transition;
pub mod shielded_withdrawal_transition;
pub mod unshield_transition;

pub use address_witness::{AddressWitnessWasm, input_witnesses_from_js_options};
pub use identity_create_from_shielded_pool_transition::IdentityCreateFromShieldedPoolTransitionWasm;
pub use orchard_action::{SerializedOrchardActionWasm, actions_from_js_options};
pub use shield_from_asset_lock_transition::ShieldFromAssetLockTransitionWasm;
pub use shield_transition::ShieldTransitionWasm;
pub use shielded_transfer_transition::ShieldedTransferTransitionWasm;
pub use shielded_withdrawal_transition::ShieldedWithdrawalTransitionWasm;
pub use unshield_transition::UnshieldTransitionWasm;

use crate::error::WasmDppResult;
use crate::utils::try_vec_to_fixed_bytes;
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
    bundle_commitment: Vec<u8>,
    extra_data: &[u8],
) -> WasmDppResult<Vec<u8>> {
    let commitment: [u8; 32] = try_vec_to_fixed_bytes(bundle_commitment, "bundleCommitment")?;
    let result = dpp::shielded::compute_platform_sighash(&commitment, extra_data);
    Ok(result.to_vec())
}
