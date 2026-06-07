mod v0;

use crate::fee::Credits;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;
use v0::compute_minimum_shielded_fee_v0;
use v0::compute_shielded_compute_fee_v0;
use v0::compute_shielded_withdrawal_fee_v0;

/// Computes the minimum **flat** fee (in credits) for a pool-paid / asset-lock shielded
/// transition.
///
/// Dispatches on the platform-versioned `dpp.methods.compute_minimum_shielded_fee` so the
/// fee formula can evolve across protocol versions without breaking older ones.
///
/// This is the source of truth for the **flat** shielded fee charged by the four transitions whose
/// storage cannot be metered against an address balance — ShieldedTransfer, Unshield,
/// ShieldedWithdrawal, and ShieldFromAssetLock. For those, the SDK builders, the
/// unshield/withdrawal transformers (for the fee actually carved from the pool), and the consensus
/// gate `validate_minimum_shielded_fee` all call this function, so the carved fee and the validation
/// threshold can never drift.
///
/// The transparent `Shield` is the exception: it meters its note/nullifier storage via GroveDB and
/// adds only the COMPUTE portion via the sibling [`compute_shielded_compute_fee`] (which carries no
/// storage term). Both functions dispatch on the SAME version key, so the flat fee and the compute
/// fee always evolve together and cannot drift.
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

/// Computes the **ShieldedWithdrawal** fee (in credits): [`compute_minimum_shielded_fee`] PLUS the
/// flat storage cost of the Core withdrawal document a `ShieldedWithdrawal` inserts.
///
/// A `ShieldedWithdrawal` additionally writes a Core withdrawal document into the withdrawals
/// contract (the document plus its index entries — `AddWithdrawalDocument`), a real,
/// GroveDB-metered insert (≈110M credits, FLAT regardless of action count) that
/// [`compute_minimum_shielded_fee`] does NOT price. This function adds that document cost as a
/// flat `SHIELDED_WITHDRAWAL_DOCUMENT_STORAGE_BYTES`-byte storage component (priced at the same
/// per-byte storage rate the per-action note storage uses).
///
/// Used ONLY by `ShieldedWithdrawal`: its SDK builder, the withdrawal transformer (for the fee
/// carved from the pool), and the consensus gate `validate_minimum_shielded_fee` all call this
/// function, so the carved fee and the validation threshold can never drift. ShieldedTransfer,
/// Unshield, and the entry transitions keep using [`compute_minimum_shielded_fee`] /
/// [`compute_shielded_compute_fee`].
///
/// Dispatches on the SAME version key (`dpp.methods.compute_minimum_shielded_fee`) as
/// [`compute_minimum_shielded_fee`] so the two formulas evolve together across protocol versions.
///
/// # Parameters
/// - `num_actions` — number of Orchard actions in the bundle
/// - `platform_version` — protocol version (determines the formula version and fee constants)
pub fn compute_shielded_withdrawal_fee(
    num_actions: usize,
    platform_version: &PlatformVersion,
) -> Result<Credits, ProtocolError> {
    match platform_version.dpp.methods.compute_minimum_shielded_fee {
        0 => compute_shielded_withdrawal_fee_v0(num_actions, platform_version),
        version => Err(ProtocolError::UnknownVersionMismatch {
            method: "compute_shielded_withdrawal_fee".to_string(),
            known_versions: vec![0],
            received: version,
        }),
    }
}

/// Computes the **compute-only** shielded fee (in credits): the ZK-compute portion (Halo 2 proof
/// verification + per-action spend-auth/nullifier processing) that GroveDB metering cannot see.
///
/// Unlike [`compute_minimum_shielded_fee`] this carries **no storage term**. It is used by the
/// transparent `Shield`, which meters its note/nullifier storage writes via GroveDB and adds only
/// this compute fee on top (as the event's `additional_fixed_fee_cost`), so storage is never
/// double-counted.
///
/// Dispatches on the SAME version key (`dpp.methods.compute_minimum_shielded_fee`) as
/// [`compute_minimum_shielded_fee`] so the two formulas evolve together across protocol versions.
///
/// # Parameters
/// - `num_actions` — number of Orchard actions in the bundle
/// - `platform_version` — protocol version (determines the formula version and fee constants)
pub fn compute_shielded_compute_fee(
    num_actions: usize,
    platform_version: &PlatformVersion,
) -> Result<Credits, ProtocolError> {
    match platform_version.dpp.methods.compute_minimum_shielded_fee {
        0 => compute_shielded_compute_fee_v0(num_actions, platform_version),
        version => Err(ProtocolError::UnknownVersionMismatch {
            method: "compute_shielded_compute_fee".to_string(),
            known_versions: vec![0],
            received: version,
        }),
    }
}
