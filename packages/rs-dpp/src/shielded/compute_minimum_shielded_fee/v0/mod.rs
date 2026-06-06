use crate::fee::Credits;
use crate::shielded::SHIELDED_STORAGE_BYTES_PER_ACTION;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

/// v0 of the shielded minimum-fee formula:
///
///   `min_fee = proof_verification_fee + num_actions × (processing_fee + storage_fee)`
///
/// where `storage_fee = SHIELDED_STORAGE_BYTES_PER_ACTION × (disk + processing) credits/byte`.
///
/// All arithmetic is checked: an overflow (only reachable via pathological fee constants)
/// surfaces as `ProtocolError::Overflow` instead of silently wrapping.
pub fn compute_minimum_shielded_fee_v0(
    num_actions: usize,
    platform_version: &PlatformVersion,
) -> Result<Credits, ProtocolError> {
    let constants = &platform_version
        .drive_abci
        .validation_and_processing
        .event_constants;
    let storage = &platform_version.fee_version.storage;

    let per_byte_rate = storage
        .storage_disk_usage_credit_per_byte
        .checked_add(storage.storage_processing_credit_per_byte)
        .ok_or(ProtocolError::Overflow(
            "shielded storage per-byte rate overflow",
        ))?;
    let storage_fee = SHIELDED_STORAGE_BYTES_PER_ACTION
        .checked_mul(per_byte_rate)
        .ok_or(ProtocolError::Overflow(
            "shielded per-action storage fee overflow",
        ))?;
    let per_action = constants
        .shielded_per_action_processing_fee
        .checked_add(storage_fee)
        .ok_or(ProtocolError::Overflow("shielded per-action fee overflow"))?;
    let actions_fee = (num_actions as u64)
        .checked_mul(per_action)
        .ok_or(ProtocolError::Overflow("shielded actions fee overflow"))?;
    constants
        .shielded_proof_verification_fee
        .checked_add(actions_fee)
        .ok_or(ProtocolError::Overflow("shielded minimum fee overflow"))
}
