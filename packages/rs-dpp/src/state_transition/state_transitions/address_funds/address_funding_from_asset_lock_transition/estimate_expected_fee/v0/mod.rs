use crate::fee::Credits;
use crate::state_transition::address_funding_from_asset_lock_transition::AddressFundingFromAssetLockTransition;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

impl AddressFundingFromAssetLockTransition {
    /// v0 expected-fee formula:
    ///
    /// `base + per_input × input_count + per_output × max(output_count, 1)`
    ///
    /// - `base` covers the fixed execution work of a minimal funding: the
    ///   asset-lock outpoint write, proof validation, the one-time-key
    ///   ECDSA verification, and signable-bytes hashing. A typical-size
    ///   hashing allowance is folded in (the signable bytes embed the full
    ///   asset-lock proof, unknown before the L1 build, but the term is
    ///   ~0.3% of the total — a length parameter would buy nothing).
    /// - `per_output` covers the metered balance/nonce write of a (new)
    ///   platform address entry.
    /// - `per_input` covers the nonce/balance retrieval, witness signature
    ///   verification and hashing, and the update of an existing entry.
    ///
    /// `output_count` is clamped to at least 1, mirroring the consensus
    /// floor's shape (a funding always credits at least the remainder
    /// output). Checked arithmetic — overflow fails closed.
    pub(super) fn estimate_expected_fee_v0(
        input_count: usize,
        output_count: usize,
        platform_version: &PlatformVersion,
    ) -> Result<Credits, ProtocolError> {
        let min_fees = &platform_version.fee_version.state_transition_min_fees;
        let inputs_fee = min_fees
            .address_funding_expected_fee_per_input
            .checked_mul(input_count as u64)
            .ok_or(ProtocolError::Overflow(
                "address funding expected fee: per-input overflow",
            ))?;
        let outputs_fee = min_fees
            .address_funding_expected_fee_per_output
            .checked_mul(output_count.max(1) as u64)
            .ok_or(ProtocolError::Overflow(
                "address funding expected fee: per-output overflow",
            ))?;
        min_fees
            .address_funding_expected_base_fee
            .checked_add(inputs_fee)
            .and_then(|sum| sum.checked_add(outputs_fee))
            .ok_or(ProtocolError::Overflow(
                "address funding expected fee: total overflow",
            ))
    }
}
