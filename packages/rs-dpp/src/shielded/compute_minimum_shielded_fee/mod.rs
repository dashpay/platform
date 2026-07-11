mod v0;

use crate::fee::Credits;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;
use v0::compute_minimum_shielded_fee_v0;
use v0::compute_shielded_identity_create_fee_v0;
use v0::compute_shielded_unshield_fee_v0;
use v0::compute_shielded_verification_fee_v0;
use v0::compute_shielded_withdrawal_fee_v0;

/// Computes the minimum **flat** fee (in credits) for a pool-paid / asset-lock shielded
/// transition.
///
/// Dispatches on the platform-versioned `dpp.methods.compute_minimum_shielded_fee` so the
/// fee formula can evolve across protocol versions without breaking older ones.
///
/// This is the **base** flat shielded fee for the pool-paid / asset-lock transitions whose storage
/// cannot be metered against an address balance. ShieldedTransfer and ShieldFromAssetLock charge
/// exactly this base (ShieldFromAssetLock adds the asset-lock base cost on the asset-lock side); the
/// other two pool-paid transitions add one flat per-transition storage component on top of this
/// base — Unshield via [`compute_shielded_unshield_fee`] (the `AddBalanceToAddress` output write)
/// and ShieldedWithdrawal via [`compute_shielded_withdrawal_fee`] (the Core withdrawal document).
/// For each transition, its SDK builder, its transformer (for the fee actually carved from the
/// pool), and the consensus gate `validate_minimum_shielded_fee` all call the SAME one of these
/// functions, so the carved fee and the validation threshold can never drift.
///
/// The transparent `Shield` is the exception: it meters its note/nullifier storage via GroveDB and
/// adds only the COMPUTE portion via the sibling [`compute_shielded_verification_fee`] (which carries no
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
/// function, so the carved fee and the validation threshold can never drift. ShieldedTransfer keeps
/// using [`compute_minimum_shielded_fee`], Unshield uses [`compute_shielded_unshield_fee`], and the
/// entry transitions use [`compute_minimum_shielded_fee`] / [`compute_shielded_verification_fee`].
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

/// Computes the **Unshield** fee (in credits): [`compute_minimum_shielded_fee`] PLUS the flat
/// storage cost of the single `AddBalanceToAddress` write an `Unshield` performs.
///
/// An `Unshield` additionally credits the net (`unshielding_amount − fee`) to the output platform
/// address via `AddBalanceToAddress`, a real, GroveDB-metered write (≈6.24M credits, FLAT
/// regardless of action count) that [`compute_minimum_shielded_fee`] does NOT price. This function
/// adds that address cost as a flat `SHIELDED_UNSHIELD_ADDRESS_STORAGE_BYTES`-byte storage
/// component (priced at the same per-byte storage rate the per-action note storage uses).
///
/// Used ONLY by `Unshield`: its SDK builder, the unshield transformer (for the fee carved from the
/// pool), and the consensus gate `validate_minimum_shielded_fee` all call this function, so the
/// carved fee and the validation threshold can never drift. ShieldedTransfer, ShieldedWithdrawal,
/// and the entry transitions keep using [`compute_minimum_shielded_fee`] /
/// [`compute_shielded_withdrawal_fee`] / [`compute_shielded_verification_fee`].
///
/// Dispatches on the SAME version key (`dpp.methods.compute_minimum_shielded_fee`) as
/// [`compute_minimum_shielded_fee`] so the two formulas evolve together across protocol versions.
///
/// # Parameters
/// - `num_actions` — number of Orchard actions in the bundle
/// - `platform_version` — protocol version (determines the formula version and fee constants)
pub fn compute_shielded_unshield_fee(
    num_actions: usize,
    platform_version: &PlatformVersion,
) -> Result<Credits, ProtocolError> {
    match platform_version.dpp.methods.compute_minimum_shielded_fee {
        0 => compute_shielded_unshield_fee_v0(num_actions, platform_version),
        version => Err(ProtocolError::UnknownVersionMismatch {
            method: "compute_shielded_unshield_fee".to_string(),
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
pub fn compute_shielded_verification_fee(
    num_actions: usize,
    platform_version: &PlatformVersion,
) -> Result<Credits, ProtocolError> {
    match platform_version.dpp.methods.compute_minimum_shielded_fee {
        0 => compute_shielded_verification_fee_v0(num_actions, platform_version),
        version => Err(ProtocolError::UnknownVersionMismatch {
            method: "compute_shielded_verification_fee".to_string(),
            known_versions: vec![0],
            received: version,
        }),
    }
}

/// Computes the **IdentityCreateFromShieldedPool** fee (in credits): [`compute_minimum_shielded_fee`]
/// PLUS the variable storage cost of the `AddNewIdentity` write (identity record + balance +
/// revision + N key subtrees), which scales with the number of public keys.
///
/// Unlike the flat per-transition components of [`compute_shielded_unshield_fee`] /
/// [`compute_shielded_withdrawal_fee`], the identity write grows monotonically with the key count.
/// This is the **client-side predictor** + the **cheap floor** the `denomination >= min_fee` gate
/// uses; the authoritative consensus fee is METERED by GroveDB at execution (the transition's
/// `ExecutionEvent` meters its ops and adds only the compute fee via `additional_fixed_fee_cost`).
///
/// Dispatches on the SAME version key (`dpp.methods.compute_minimum_shielded_fee`) as
/// [`compute_minimum_shielded_fee`] so the formulas evolve together across protocol versions.
///
/// # Parameters
/// - `num_actions` — number of Orchard actions in the bundle
/// - `num_keys` — number of public keys the new identity is created with
/// - `platform_version` — protocol version (determines the formula version and fee constants)
pub fn compute_shielded_identity_create_fee(
    num_actions: usize,
    num_keys: usize,
    platform_version: &PlatformVersion,
) -> Result<Credits, ProtocolError> {
    match platform_version.dpp.methods.compute_minimum_shielded_fee {
        0 => compute_shielded_identity_create_fee_v0(num_actions, num_keys, platform_version),
        version => Err(ProtocolError::UnknownVersionMismatch {
            method: "compute_shielded_identity_create_fee".to_string(),
            known_versions: vec![0],
            received: version,
        }),
    }
}
