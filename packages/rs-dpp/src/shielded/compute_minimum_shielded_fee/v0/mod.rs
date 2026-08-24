use crate::fee::Credits;
use crate::shielded::{
    SHIELDED_UNSHIELD_ADDRESS_STORAGE_BYTES, SHIELDED_WITHDRAWAL_DOCUMENT_STORAGE_BYTES,
};
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

/// v0 of the shielded **compute-only** fee:
///
///   `compute_fee = proof_verification_fee + num_actions × processing_fee`
///
/// This is the ZK-compute portion of a shielded transition that GroveDB metering cannot see. It has
/// two parts: a per-bundle `proof_verification_fee` for the single Halo 2 proof, and a per-action
/// `processing_fee` that prices the MARGINAL verification work each additional action adds to the
/// bundle (a larger bundle is a larger circuit and a longer batch verification). For spend-bearing
/// transitions that marginal work includes the per-action RedPallas spend-auth signature
/// verification and nullifier check; output-only entry transitions (Shield / ShieldFromAssetLock)
/// do no spends or nullifier checks, but each output action still enlarges the proof and so carries
/// the same per-action processing charge.
///
/// It carries **no storage term**: storage is the real cost of the note/nullifier writes and is
/// metered separately by GroveDB.
///
/// All arithmetic is checked: an overflow (only reachable via pathological fee constants)
/// surfaces as `ProtocolError::Overflow` instead of silently wrapping.
pub fn compute_shielded_verification_fee_v0(
    num_actions: usize,
    platform_version: &PlatformVersion,
) -> Result<Credits, ProtocolError> {
    let constants = &platform_version
        .drive_abci
        .validation_and_processing
        .event_constants;

    let actions_fee = (num_actions as u64)
        .checked_mul(constants.shielded_per_action_processing_fee)
        .ok_or(ProtocolError::Overflow(
            "shielded compute actions fee overflow",
        ))?;
    constants
        .shielded_proof_verification_fee
        .checked_add(actions_fee)
        .ok_or(ProtocolError::Overflow("shielded compute fee overflow"))
}

/// v0 of the shielded minimum-fee formula:
///
///   `min_fee = compute_fee + num_actions × storage_fee_per_action`
///
/// where `compute_fee = proof_verification_fee + num_actions × processing_fee`
/// (see [`compute_shielded_verification_fee_v0`]) and
/// `storage_fee_per_action = shielded_storage_bytes_per_action × (disk + processing) credits/byte`,
/// with the byte allowance a versioned event constant beside the compute fees — so the compute
/// and storage components of the flat fee are calibrated independently per protocol version
/// (compute reserved for compute; the allowance sized to what the metering actually charges a
/// note/nullifier write under the GroveVersion in force).
///
/// This is the fee carved from the shielded **pool** by the pool-paid transitions
/// (ShieldedTransfer / Unshield / ShieldedWithdrawal), which cannot meter their writes against an
/// address balance and so must price a flat storage estimate into the carved fee. The transparent
/// `Shield` instead meters storage via GroveDB and only adds [`compute_shielded_verification_fee_v0`].
///
/// All arithmetic is checked: an overflow (only reachable via pathological fee constants)
/// surfaces as `ProtocolError::Overflow` instead of silently wrapping.
pub fn compute_minimum_shielded_fee_v0(
    num_actions: usize,
    platform_version: &PlatformVersion,
) -> Result<Credits, ProtocolError> {
    let storage = &platform_version.fee_version.storage;
    let constants = &platform_version
        .drive_abci
        .validation_and_processing
        .event_constants;

    let compute_fee = compute_shielded_verification_fee_v0(num_actions, platform_version)?;

    let per_byte_rate = storage
        .storage_disk_usage_credit_per_byte
        .checked_add(storage.storage_processing_credit_per_byte)
        .ok_or(ProtocolError::Overflow(
            "shielded storage per-byte rate overflow",
        ))?;
    let storage_fee_per_action = constants
        .shielded_storage_bytes_per_action
        .checked_mul(per_byte_rate)
        .ok_or(ProtocolError::Overflow(
            "shielded per-action storage fee overflow",
        ))?;
    let storage_fee = (num_actions as u64)
        .checked_mul(storage_fee_per_action)
        .ok_or(ProtocolError::Overflow(
            "shielded actions storage fee overflow",
        ))?;
    compute_fee
        .checked_add(storage_fee)
        .ok_or(ProtocolError::Overflow("shielded minimum fee overflow"))
}

/// v0 of the shielded **withdrawal** fee formula:
///
///   `withdrawal_fee = compute_minimum_shielded_fee_v0(num_actions)
///                     + SHIELDED_WITHDRAWAL_DOCUMENT_STORAGE_BYTES × (disk + processing) credits/byte`
///
/// This is [`compute_minimum_shielded_fee_v0`] PLUS one flat storage component for the Core
/// withdrawal document a `ShieldedWithdrawal` inserts (the document plus its
/// withdrawals-contract index entries — `AddWithdrawalDocument`). That document insert has a
/// real, GroveDB-metered cost of ≈110M credits that is FLAT regardless of action count and is
/// NOT priced by `compute_minimum_shielded_fee_v0` (which only covers per-action note/nullifier
/// storage and the per-bundle ZK compute). Pricing it as `SHIELDED_WITHDRAWAL_DOCUMENT_STORAGE_BYTES`
/// effective bytes at the SAME per-byte storage rate the per-action note storage uses keeps the
/// document write covered (so `execute_event/v0`'s `storage = min(real_storage, fee)` booking
/// split never zeroes the proposer's processing reward) and lets the component track the storage
/// rate as it evolves.
///
/// This fee is used ONLY by `ShieldedWithdrawal`. The other pool-paid transitions
/// (ShieldedTransfer / Unshield) and the entry transitions keep using
/// [`compute_minimum_shielded_fee_v0`] / [`compute_shielded_verification_fee_v0`].
///
/// All arithmetic is checked: an overflow (only reachable via pathological fee constants)
/// surfaces as `ProtocolError::Overflow` instead of silently wrapping. The `per_byte_rate` is
/// computed exactly as in [`compute_minimum_shielded_fee_v0`].
pub fn compute_shielded_withdrawal_fee_v0(
    num_actions: usize,
    platform_version: &PlatformVersion,
) -> Result<Credits, ProtocolError> {
    let storage = &platform_version.fee_version.storage;

    let base_fee = compute_minimum_shielded_fee_v0(num_actions, platform_version)?;

    let per_byte_rate = storage
        .storage_disk_usage_credit_per_byte
        .checked_add(storage.storage_processing_credit_per_byte)
        .ok_or(ProtocolError::Overflow(
            "shielded storage per-byte rate overflow",
        ))?;
    let document_storage_fee = SHIELDED_WITHDRAWAL_DOCUMENT_STORAGE_BYTES
        .checked_mul(per_byte_rate)
        .ok_or(ProtocolError::Overflow(
            "shielded withdrawal document storage fee overflow",
        ))?;
    base_fee
        .checked_add(document_storage_fee)
        .ok_or(ProtocolError::Overflow("shielded withdrawal fee overflow"))
}

/// v0 of the shielded **unshield** fee formula:
///
///   `unshield_fee = compute_minimum_shielded_fee_v0(num_actions)
///                   + SHIELDED_UNSHIELD_ADDRESS_STORAGE_BYTES × (disk + processing) credits/byte`
///
/// This is [`compute_minimum_shielded_fee_v0`] PLUS one flat storage component for the single
/// `AddBalanceToAddress` write an `Unshield` performs, crediting the net
/// (`unshielding_amount − fee`) to the output platform address. That address write has a real,
/// GroveDB-metered cost of ≈6.24M credits that is FLAT regardless of action count and is NOT
/// priced by `compute_minimum_shielded_fee_v0` (which only covers per-action note/nullifier storage
/// and the per-bundle ZK compute). Pricing it as `SHIELDED_UNSHIELD_ADDRESS_STORAGE_BYTES`
/// effective bytes at the SAME per-byte storage rate the per-action note storage uses keeps the
/// address write covered (so `execute_event/v0`'s `storage = min(real_storage, fee)` booking split
/// never zeroes the proposer's processing reward) and lets the component track the storage rate as
/// it evolves.
///
/// This fee is used ONLY by `Unshield`. The other pool-paid transitions (ShieldedTransfer /
/// ShieldedWithdrawal) and the entry transitions keep using their own formulas
/// ([`compute_minimum_shielded_fee_v0`] / [`compute_shielded_withdrawal_fee_v0`] /
/// [`compute_shielded_verification_fee_v0`]).
///
/// All arithmetic is checked: an overflow (only reachable via pathological fee constants)
/// surfaces as `ProtocolError::Overflow` instead of silently wrapping. The `per_byte_rate` is
/// computed exactly as in [`compute_minimum_shielded_fee_v0`].
pub fn compute_shielded_unshield_fee_v0(
    num_actions: usize,
    platform_version: &PlatformVersion,
) -> Result<Credits, ProtocolError> {
    let storage = &platform_version.fee_version.storage;

    let base_fee = compute_minimum_shielded_fee_v0(num_actions, platform_version)?;

    let per_byte_rate = storage
        .storage_disk_usage_credit_per_byte
        .checked_add(storage.storage_processing_credit_per_byte)
        .ok_or(ProtocolError::Overflow(
            "shielded storage per-byte rate overflow",
        ))?;
    let address_storage_fee = SHIELDED_UNSHIELD_ADDRESS_STORAGE_BYTES
        .checked_mul(per_byte_rate)
        .ok_or(ProtocolError::Overflow(
            "shielded unshield address storage fee overflow",
        ))?;
    base_fee
        .checked_add(address_storage_fee)
        .ok_or(ProtocolError::Overflow("shielded unshield fee overflow"))
}

/// v0 of the shielded **identity-create** fee formula:
///
///   `identity_create_fee = compute_minimum_shielded_fee_v0(num_actions)
///       + identity_create_base_cost + num_keys × identity_key_in_creation_cost`
///
/// This is [`compute_minimum_shielded_fee_v0`] (the per-action note/nullifier storage estimate +
/// the per-bundle ZK compute) PLUS the consensus identity-create cost floor for the `AddNewIdentity`
/// write an `IdentityCreateFromShieldedPool` performs (the identity record + balance + revision + N
/// keys). Rather than a bespoke storage-byte estimate, this reuses the SAME
/// `identity_create_base_cost` + `identity_key_in_creation_cost` constants
/// (`platform_version.fee_version.state_transition_min_fees`) that the non-shielded
/// `IdentityCreate` / `IdentityCreateFromAddresses` transitions use in their
/// `StateTransitionEstimatedFeeValidation::calculate_min_required_fee` — one source of truth for
/// the cost of creating an identity, so the shielded predictor cannot drift from the consensus
/// minimum the create is actually subject to. Like those constants, it grows with the key count.
///
/// This function is NOT the authoritative consensus fee (execution meters the real GroveDB cost of
/// the identity write against the new identity's balance and adds only the compute fee on top). It
/// is the **client-side predictor** — so a client can size its bundle and pick a denomination that
/// covers the fee — and the **cheap floor** the `denomination >= min_fee` gate uses to reject
/// obviously-underfunded denominations before metering. If the later metered affordability check
/// inside `validate_fees_of_event` finds `denomination < total_fee`, execution returns
/// `IdentityInsufficientBalanceError` through the standard unpaid-rejection path (the spend is not
/// finalized and no nullifier is consumed). Only the unique-public-key-hash collision branch in
/// state validation uses the fallback-address-minus-penalty path — the same residual-risk window the
/// non-shielded identity-create predictor relies on by using this floor. (In practice the smallest
/// legal denomination, 10^10 credits, far exceeds the max-key floor, so neither rejection arises for
/// well-formed transitions.)
///
/// All arithmetic is checked: an overflow (only reachable via pathological fee constants or key
/// counts) surfaces as `ProtocolError::Overflow` instead of silently wrapping.
pub fn compute_shielded_identity_create_fee_v0(
    num_actions: usize,
    num_keys: usize,
    platform_version: &PlatformVersion,
) -> Result<Credits, ProtocolError> {
    let min_fees = &platform_version.fee_version.state_transition_min_fees;

    let base_fee = compute_minimum_shielded_fee_v0(num_actions, platform_version)?;

    let keys_fee = min_fees
        .identity_key_in_creation_cost
        .checked_mul(num_keys as u64)
        .ok_or(ProtocolError::Overflow(
            "shielded identity create per-key fee overflow",
        ))?;
    let identity_create_floor = min_fees
        .identity_create_base_cost
        .checked_add(keys_fee)
        .ok_or(ProtocolError::Overflow(
            "shielded identity create floor overflow",
        ))?;
    base_fee
        .checked_add(identity_create_floor)
        .ok_or(ProtocolError::Overflow(
            "shielded identity create fee overflow",
        ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `compute_minimum_shielded_fee_v0` must equal
    /// `proof + num_actions × (processing + allowance×rate)` with every term read from the
    /// version's own constant tables, and must decompose as
    /// `compute_fee + num_actions × storage_allowance` — the component split the pool-paid
    /// booking and the fee-floor tests rely on.
    #[test]
    fn compute_minimum_shielded_fee_v0_equals_historical_formula() {
        let platform_version = PlatformVersion::latest();
        let constants = &platform_version
            .drive_abci
            .validation_and_processing
            .event_constants;
        let storage = &platform_version.fee_version.storage;
        let per_byte_rate =
            storage.storage_disk_usage_credit_per_byte + storage.storage_processing_credit_per_byte;

        for num_actions in [0usize, 1, 2, 5, 16] {
            // Structure: proof + num_actions × (processing + allowance×rate)
            let per_action = constants.shielded_per_action_processing_fee
                + constants.shielded_storage_bytes_per_action * per_byte_rate;
            let historical =
                constants.shielded_proof_verification_fee + (num_actions as u64) * per_action;

            let refactored = compute_minimum_shielded_fee_v0(num_actions, platform_version)
                .expect("minimum shielded fee");
            assert_eq!(
                refactored, historical,
                "refactored minimum fee must match historical formula for {num_actions} actions"
            );

            // And min_fee == compute_fee + num_actions × storage_estimate.
            let compute_fee = compute_shielded_verification_fee_v0(num_actions, platform_version)
                .expect("compute fee");
            assert_eq!(
                refactored,
                compute_fee
                    + (num_actions as u64)
                        * constants.shielded_storage_bytes_per_action
                        * per_byte_rate,
                "minimum fee must equal compute fee plus the per-action storage estimate"
            );
        }
    }

    /// Independent boundary goldens across the protocol-14 rebalance: the released protocol-12
    /// and protocol-13 tables must keep producing the shipped 162,851,200-credit two-action fee
    /// byte-for-byte (100M proof + 2 × 22M processing + 2 × 344 B × 27,400), and protocol 14
    /// must produce exactly 114,140,000 (40M proof + 2 × 22M + 2 × 550 B × 27,400). Hardcoded
    /// on purpose — deriving the expectation from the same table field the implementation
    /// reads would pass even if a released table were accidentally given the new allowance.
    #[test]
    fn minimum_shielded_fee_changes_only_at_protocol_14() {
        for protocol_version in [12, 13] {
            let platform_version = PlatformVersion::get(protocol_version)
                .expect("released shielded protocol version should exist");
            assert_eq!(
                compute_minimum_shielded_fee_v0(2, platform_version)
                    .expect("released minimum shielded fee"),
                162_851_200,
                "protocol {protocol_version} must keep the shipped two-action fee byte-for-byte"
            );
        }
        let platform_version = PlatformVersion::get(14).expect("protocol version 14 exists");
        assert_eq!(
            compute_minimum_shielded_fee_v0(2, platform_version).expect("minimum shielded fee"),
            114_140_000,
            "protocol 14 must price a two-action bundle at the rebalanced constants"
        );
    }

    /// Pin the exact relationship between the ShieldedWithdrawal fee and the base shielded fee:
    /// the withdrawal fee MUST be `compute_minimum_shielded_fee_v0(n)` plus exactly one flat
    /// `SHIELDED_WITHDRAWAL_DOCUMENT_STORAGE_BYTES × per_byte_rate` document component (the same
    /// `per_byte_rate = disk + processing` the per-action note storage uses), independent of `n`.
    /// This locks the document cost as a flat add-on so it cannot silently drift from the base
    /// fee or accidentally scale with the action count.
    #[test]
    fn compute_shielded_withdrawal_fee_v0_equals_base_plus_flat_document_cost() {
        let platform_version = PlatformVersion::latest();
        let storage = &platform_version.fee_version.storage;
        let per_byte_rate =
            storage.storage_disk_usage_credit_per_byte + storage.storage_processing_credit_per_byte;
        let document_cost = SHIELDED_WITHDRAWAL_DOCUMENT_STORAGE_BYTES * per_byte_rate;

        for num_actions in [0usize, 1, 2, 5, 16] {
            let base = compute_minimum_shielded_fee_v0(num_actions, platform_version)
                .expect("minimum shielded fee");
            let withdrawal = compute_shielded_withdrawal_fee_v0(num_actions, platform_version)
                .expect("withdrawal shielded fee");
            assert_eq!(
                withdrawal,
                base + document_cost,
                "withdrawal fee must equal the base minimum fee plus the flat \
                 {SHIELDED_WITHDRAWAL_DOCUMENT_STORAGE_BYTES}-byte document storage cost for \
                 {num_actions} actions"
            );
            // The document component is flat: the delta over the base must not depend on n.
            assert_eq!(
                withdrawal - base,
                document_cost,
                "the withdrawal-document component must be flat (independent of action count)"
            );
        }
    }

    /// The identity-create fee MUST equal the base shielded fee plus the consensus identity-create
    /// floor `identity_create_base_cost + num_keys × identity_key_in_creation_cost`, and it MUST grow
    /// strictly with the key count (a larger key set is a larger `AddNewIdentity` write). This pins
    /// the formula to the SAME constants the non-shielded `IdentityCreate` predictor uses, so the
    /// `denomination >= min_fee` gate stays aligned with the consensus minimum and cannot drift into
    /// a second, divergent calibration.
    #[test]
    fn compute_shielded_identity_create_fee_v0_scales_with_keys() {
        let platform_version = PlatformVersion::latest();
        let min_fees = &platform_version.fee_version.state_transition_min_fees;

        for num_actions in [1usize, 2, 5] {
            let base = compute_minimum_shielded_fee_v0(num_actions, platform_version)
                .expect("minimum shielded fee");
            let mut previous = None;
            for num_keys in [1usize, 2, 5, 10] {
                let fee = compute_shielded_identity_create_fee_v0(
                    num_actions,
                    num_keys,
                    platform_version,
                )
                .expect("identity create fee");
                let expected_floor = min_fees.identity_create_base_cost
                    + num_keys as u64 * min_fees.identity_key_in_creation_cost;
                assert_eq!(
                    fee,
                    base + expected_floor,
                    "identity create fee must equal base + identity_create_base_cost + \
                     num_keys×identity_key_in_creation_cost"
                );
                if let Some(prev) = previous {
                    assert!(
                        fee > prev,
                        "identity create fee must grow strictly with the key count"
                    );
                }
                previous = Some(fee);
            }
        }
    }

    /// Pin the exact relationship between the Unshield fee and the base shielded fee:
    /// the unshield fee MUST be `compute_minimum_shielded_fee_v0(n)` plus exactly one flat
    /// `SHIELDED_UNSHIELD_ADDRESS_STORAGE_BYTES × per_byte_rate` address-write component (the same
    /// `per_byte_rate = disk + processing` the per-action note storage uses), independent of `n`.
    /// This locks the address-write cost as a flat add-on so it cannot silently drift from the base
    /// fee or accidentally scale with the action count.
    #[test]
    fn compute_shielded_unshield_fee_v0_equals_base_plus_flat_address_cost() {
        let platform_version = PlatformVersion::latest();
        let storage = &platform_version.fee_version.storage;
        let per_byte_rate =
            storage.storage_disk_usage_credit_per_byte + storage.storage_processing_credit_per_byte;
        let address_cost = SHIELDED_UNSHIELD_ADDRESS_STORAGE_BYTES * per_byte_rate;

        for num_actions in [0usize, 1, 2, 5, 16] {
            let base = compute_minimum_shielded_fee_v0(num_actions, platform_version)
                .expect("minimum shielded fee");
            let unshield = compute_shielded_unshield_fee_v0(num_actions, platform_version)
                .expect("unshield shielded fee");
            assert_eq!(
                unshield,
                base + address_cost,
                "unshield fee must equal the base minimum fee plus the flat \
                 {SHIELDED_UNSHIELD_ADDRESS_STORAGE_BYTES}-byte address-write storage cost for \
                 {num_actions} actions"
            );
            // The address-write component is flat: the delta over the base must not depend on n.
            assert_eq!(
                unshield - base,
                address_cost,
                "the unshield address-write component must be flat (independent of action count)"
            );
        }
    }
}
