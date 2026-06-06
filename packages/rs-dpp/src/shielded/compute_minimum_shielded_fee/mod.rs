mod v0;

use crate::fee::Credits;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;
use v0::compute_minimum_shielded_fee_v0;

/// Computes the minimum fee (in credits) for a shielded state transition.
///
/// Dispatches on the platform-versioned `dpp.methods.compute_minimum_shielded_fee` so the
/// fee formula can evolve across protocol versions without breaking older ones.
///
/// This is the **single source of truth** for the shielded fee formula: the SDK builders,
/// the unshield/withdrawal transformers (for the fee actually carved from the pool), and the
/// consensus gate `validate_minimum_shielded_fee` all call it, so the carved fee and the
/// validation threshold can never drift.
///
/// # Parameters
/// - `num_actions` — number of Orchard actions in the bundle
/// - `platform_version` — protocol version (determines the formula version and fee constants)
pub fn compute_minimum_shielded_fee(
    num_actions: usize,
    platform_version: &PlatformVersion,
) -> Result<Credits, ProtocolError> {
    match platform_version.dpp.methods.compute_minimum_shielded_fee {
        0 => compute_minimum_shielded_fee_v0(num_actions, platform_version),
        version => Err(ProtocolError::UnknownVersionMismatch {
            method: "compute_minimum_shielded_fee".to_string(),
            known_versions: vec![0],
            received: version,
        }),
    }
}
