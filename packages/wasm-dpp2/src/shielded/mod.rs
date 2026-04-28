pub mod address_witness;
pub mod orchard_action;
pub mod shield_from_asset_lock_transition;
pub mod shield_transition;
pub mod shielded_transfer_transition;
pub mod shielded_withdrawal_transition;
pub mod unshield_transition;

pub use address_witness::{AddressWitnessWasm, input_witnesses_from_js_options};
pub use orchard_action::{SerializedOrchardActionWasm, actions_from_js_options};
pub use shield_from_asset_lock_transition::ShieldFromAssetLockTransitionWasm;
pub use shield_transition::ShieldTransitionWasm;
pub use shielded_transfer_transition::ShieldedTransferTransitionWasm;
pub use shielded_withdrawal_transition::ShieldedWithdrawalTransitionWasm;
pub use unshield_transition::UnshieldTransitionWasm;

use crate::error::WasmDppResult;
use crate::utils::try_vec_to_fixed_bytes;
use wasm_bindgen::prelude::wasm_bindgen;

/// Maximum on-the-wire size of an Orchard / Halo2 proof. Real proofs are well
/// under 10 KB; the cap is generous to absorb future circuit changes while
/// keeping a malicious caller from triggering GB-scale allocations through
/// `serde_wasm_bindgen::from_value` before downstream validation runs.
pub(crate) const MAX_HALO2_PROOF_BYTES: usize = 64 * 1024;

/// Maximum on-the-wire size of a Bitcoin Core output script. Standard scripts
/// fit in well under this; the cap is the Bitcoin protocol's overall script
/// size ceiling and prevents memory-exhaustion at the JS / WASM boundary.
pub(crate) const MAX_CORE_SCRIPT_BYTES: usize = 10_000;

/// Length of a recoverable ECDSA signature (64-byte signature + 1 recovery byte).
/// Constructors accept an empty `signature` (transitions are typically built
/// unsigned then signed in a later step) but reject anything longer than this.
pub(crate) const MAX_RECOVERABLE_ECDSA_SIGNATURE_BYTES: usize = 65;

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
