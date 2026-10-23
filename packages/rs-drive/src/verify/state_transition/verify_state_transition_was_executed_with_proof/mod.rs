//! Verifies the execution of a state transition using a provided proof.
//!
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::ContractLookupFn;
use crate::verify::RootHash;
use dpp::block::block_info::BlockInfo;
use dpp::state_transition::proof_result::StateTransitionProofOutcome;
use dpp::state_transition::StateTransition;
use dpp::version::PlatformVersion;

mod v0;
mod v1;

impl Drive {
    /// Verifies a state transition against a provided proof, returning the
    /// verified result tagged with the guarantee the proof establishes.
    ///
    /// For most transition types the proof binds the execution of this
    /// specific transition and the outcome is
    /// [`StateTransitionProofOutcome::ExecutionProved`]. For the transition
    /// families whose proof format cannot establish request-specific
    /// completion (balance top-ups, credit transfers and withdrawals,
    /// address funds movements, shields, and no-history token
    /// burn/mint/transfer), the proof only authenticates the affected keys'
    /// state at the committed block, and the outcome is
    /// [`StateTransitionProofOutcome::AffectedState`]: a height-pinned
    /// snapshot (keys derived from the transition, values as of the proof's
    /// block), **not** evidence that the transition executed.
    ///
    /// From protocol version 14 a document batch's proof also carries the
    /// credit balance of the batch's owner, read from the same state as the
    /// document; it is a snapshot at the proof's block whatever the outcome's
    /// guarantee, and `None` for a proof made at an earlier version.
    ///
    /// # Parameters
    ///
    /// - `state_transition`: A reference to the `StateTransition` that needs to be verified.
    /// - `proof`: A byte slice representing the cryptographic proof to be verified.
    /// - `known_contracts_provider_fn`: A lookup returning known `DataContract`s by identifier.
    /// - `platform_version`: A reference to the `PlatformVersion` which dictates the verification method to be used.
    ///
    /// # Returns
    ///
    /// A `Result` containing either:
    /// - On success: a tuple of `RootHash` and `StateTransitionProofOutcome`, where `RootHash` is the root hash of the
    ///   proof and `StateTransitionProofOutcome` carries the verified result tagged with its guarantee.
    /// - On failure: an `Error` encapsulating the reason for failure, such as proof corruption or a database query error.
    ///
    /// # Errors
    ///
    /// This function can return an `Error` in several cases, including but not limited to:
    /// - The proof is invalid or corrupted.
    /// - The verification process for the given platform version is not implemented.
    /// - The database query required for the verification fails.
    ///
    pub fn verify_state_transition_was_executed_with_proof(
        state_transition: &StateTransition,
        block_info: &BlockInfo,
        proof: &[u8],
        known_contracts_provider_fn: &ContractLookupFn,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, StateTransitionProofOutcome), Error> {
        Self::verify_state_transition_was_executed_with_proof_for_query_version(
            state_transition,
            block_info,
            proof,
            known_contracts_provider_fn,
            platform_version,
            platform_version,
        )
    }

    /// Verifies the producer's execution-proof layout using the caller's verifier rules.
    ///
    /// `query_platform_version` selects whether the query includes an owner balance.
    /// All proof validation and decoding still use `platform_version`. The caller
    /// must authenticate the producer version together with the resulting root.
    ///
    /// The generation is chosen by the producer's version, so the generations of this
    /// method may differ only in the layout the producer proved. A stricter check
    /// belongs under the caller's `platform_version`, which an older producer cannot
    /// lower.
    ///
    /// # Parameters
    ///
    /// - `state_transition`, `block_info`, `proof`, `known_contracts_provider_fn`: as for
    ///   [`Self::verify_state_transition_was_executed_with_proof`].
    /// - `query_platform_version`: the version the producer signed, which selects the
    ///   proof layout.
    /// - `platform_version`: the caller's version, which selects every other rule. Its
    ///   generation of this method must be one this build knows.
    ///
    /// # Returns
    ///
    /// The root hash of the proof and the verified outcome, as for
    /// [`Self::verify_state_transition_was_executed_with_proof`]. An unknown generation
    /// in either version is reported as [`DriveError::UnknownVersionMismatch`].
    pub fn verify_state_transition_was_executed_with_proof_for_query_version(
        state_transition: &StateTransition,
        block_info: &BlockInfo,
        proof: &[u8],
        known_contracts_provider_fn: &ContractLookupFn,
        query_platform_version: &PlatformVersion,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, StateTransitionProofOutcome), Error> {
        let verifier_version = platform_version
            .drive
            .methods
            .verify
            .state_transition
            .verify_state_transition_was_executed_with_proof;
        if !matches!(verifier_version, 0 | 1) {
            return Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_state_transition_was_executed_with_proof".to_string(),
                known_versions: vec![0, 1],
                received: verifier_version,
            }));
        }
        match query_platform_version
            .drive
            .methods
            .verify
            .state_transition
            .verify_state_transition_was_executed_with_proof
        {
            0 => Drive::verify_state_transition_was_executed_with_proof_v0(
                state_transition,
                block_info,
                proof,
                known_contracts_provider_fn,
                platform_version,
            ),
            1 => Drive::verify_state_transition_was_executed_with_proof_v1(
                state_transition,
                block_info,
                proof,
                known_contracts_provider_fn,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_state_transition_was_executed_with_proof".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::state_transition::state_transitions::identity::identity_credit_transfer_transition::IdentityCreditTransferTransition;
    use dpp::state_transition::state_transitions::identity::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;
    use dpp::version::PlatformVersion;
    #[test]
    fn test_verify_state_transition_was_executed_with_proof_unknown_version_mismatch() {
        let mut platform_version = PlatformVersion::latest().clone();
        platform_version
            .drive
            .methods
            .verify
            .state_transition
            .verify_state_transition_was_executed_with_proof = 255;

        let st = StateTransition::IdentityCreditTransfer(IdentityCreditTransferTransition::V0(
            IdentityCreditTransferTransitionV0::default(),
        ));
        let block_info = BlockInfo::default();
        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        let result = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &block_info,
            &[],
            known_contracts_provider_fn,
            &platform_version,
        );

        assert!(
            matches!(
                result,
                Err(Error::Drive(DriveError::UnknownVersionMismatch { .. }))
            ),
            "expected UnknownVersionMismatch, got {:?}",
            result,
        );
    }

    /// The producer's version only picks the layout: a verifier generation this build does
    /// not know is refused even when the producer's generation is known, and the reverse.
    #[test]
    fn should_reject_unknown_verifier_or_producer_generation() {
        let mut unknown = PlatformVersion::latest().clone();
        unknown
            .drive
            .methods
            .verify
            .state_transition
            .verify_state_transition_was_executed_with_proof = 255;
        let known = PlatformVersion::latest();

        let st = StateTransition::IdentityCreditTransfer(IdentityCreditTransferTransition::V0(
            IdentityCreditTransferTransitionV0::default(),
        ));
        let block_info = BlockInfo::default();
        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);

        for (query_platform_version, platform_version) in [(known, &unknown), (&unknown, known)] {
            let result = Drive::verify_state_transition_was_executed_with_proof_for_query_version(
                &st,
                &block_info,
                &[],
                known_contracts_provider_fn,
                query_platform_version,
                platform_version,
            );

            assert!(
                matches!(
                    result,
                    Err(Error::Drive(DriveError::UnknownVersionMismatch {
                        received: 255,
                        ..
                    }))
                ),
                "expected UnknownVersionMismatch, got {:?}",
                result,
            );
        }
    }
}
