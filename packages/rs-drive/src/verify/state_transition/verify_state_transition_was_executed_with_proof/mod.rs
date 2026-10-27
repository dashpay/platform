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
    /// Expects the layout a producer at `platform_version` proves. To verify a proof
    /// made at another version, use
    /// [`Self::verify_state_transition_was_executed_with_proof_for_query_version`].
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
    /// The caller's `platform_version` selects the generation of this method and every
    /// rule inside it, so a producer's version cannot lower them. `query_platform_version`
    /// selects only the layout the producer proved: whether the proof of an owned,
    /// fee-paying transition carries the owner's credit balance, which the producer's
    /// `prove_state_transition` generation decides. The caller must authenticate the
    /// producer version together with the resulting root.
    ///
    /// # Parameters
    ///
    /// - `state_transition`, `block_info`, `proof`, `known_contracts_provider_fn`: as for
    ///   [`Self::verify_state_transition_was_executed_with_proof`].
    /// - `query_platform_version`: the version the producer signed, which selects the
    ///   proof layout.
    /// - `platform_version`: the caller's version, which selects every other rule.
    ///
    /// # Returns
    ///
    /// The root hash of the proof and the verified outcome, as for
    /// [`Self::verify_state_transition_was_executed_with_proof`]. A generation this build
    /// does not know, of this method in the caller's version or of
    /// `prove_state_transition` in the producer's, is reported as
    /// [`DriveError::UnknownVersionMismatch`] naming that method.
    pub fn verify_state_transition_was_executed_with_proof_for_query_version(
        state_transition: &StateTransition,
        block_info: &BlockInfo,
        proof: &[u8],
        known_contracts_provider_fn: &ContractLookupFn,
        query_platform_version: &PlatformVersion,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, StateTransitionProofOutcome), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .state_transition
            .verify_state_transition_was_executed_with_proof
        {
            // Generations 0 and 1 run the same checks. The layout comes from the
            // producer's `prove_state_transition` generation, so either one verifies a
            // proof from any producer whose prover generation this build knows; an
            // unknown prover generation is refused. Protocol versions 1 to 13 select
            // verifier and prover generation 0, and protocol version 14 selects 1 for
            // both, so a proof checked at the version that made it runs the same body
            // with the layout that version's verifier generation named.
            0 | 1 => Drive::verify_state_transition_was_executed_with_proof_v0(
                state_transition,
                block_info,
                proof,
                known_contracts_provider_fn,
                Self::proof_carries_owner_balance(query_platform_version)?,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_state_transition_was_executed_with_proof".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }

    /// Whether a producer at `query_platform_version` proves an owned, fee-paying
    /// transition together with its owner's credit balance: generation 1 of
    /// `prove_state_transition` does, generation 0 does not.
    fn proof_carries_owner_balance(
        query_platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        match query_platform_version
            .drive
            .methods
            .prove
            .prove_state_transition
        {
            0 => Ok(false),
            1 => Ok(true),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_state_transition".to_string(),
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

    fn verify_empty_credit_transfer_proof(
        query_platform_version: &PlatformVersion,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, StateTransitionProofOutcome), Error> {
        let st = StateTransition::IdentityCreditTransfer(IdentityCreditTransferTransition::V0(
            IdentityCreditTransferTransitionV0::default(),
        ));
        let known_contracts_provider_fn: &ContractLookupFn = &|_id| Ok(None);
        Drive::verify_state_transition_was_executed_with_proof_for_query_version(
            &st,
            &BlockInfo::default(),
            &[],
            known_contracts_provider_fn,
            query_platform_version,
            platform_version,
        )
    }

    /// The caller's generation of this verifier selects the checks and the producer's
    /// prover generation the layout, so an unknown generation of either is refused,
    /// naming the method whose generation this build does not know.
    #[test]
    fn should_reject_unknown_verifier_or_producer_generation() {
        let mut unknown_verifier = PlatformVersion::latest().clone();
        unknown_verifier
            .drive
            .methods
            .verify
            .state_transition
            .verify_state_transition_was_executed_with_proof = 255;
        let mut unknown_prover = PlatformVersion::latest().clone();
        unknown_prover.drive.methods.prove.prove_state_transition = 255;
        let known = PlatformVersion::latest();

        for (query_platform_version, platform_version, unknown_method) in [
            (
                known,
                &unknown_verifier,
                "verify_state_transition_was_executed_with_proof",
            ),
            (&unknown_prover, known, "prove_state_transition"),
        ] {
            let result =
                verify_empty_credit_transfer_proof(query_platform_version, platform_version);

            assert!(
                matches!(
                    &result,
                    Err(Error::Drive(DriveError::UnknownVersionMismatch {
                        method,
                        received: 255,
                        ..
                    })) if method == unknown_method
                ),
                "expected UnknownVersionMismatch for {unknown_method}, got {:?}",
                result,
            );
        }
    }

    /// Which checks run is the verifier's decision: the generation of this verifier
    /// in the producer's table is never read, so a producer cannot select an older
    /// or weaker one.
    #[test]
    fn should_not_read_the_producers_verifier_generation() {
        let mut producer = PlatformVersion::latest().clone();
        producer
            .drive
            .methods
            .verify
            .state_transition
            .verify_state_transition_was_executed_with_proof = 255;
        let verifier = PlatformVersion::latest();

        let result = verify_empty_credit_transfer_proof(&producer, verifier);
        assert!(
            !matches!(
                result,
                Err(Error::Drive(DriveError::UnknownVersionMismatch { .. }))
            ),
            "the producer's verifier generation must not be read, got {:?}",
            result,
        );
        assert_eq!(
            format!("{:?}", result),
            format!(
                "{:?}",
                verify_empty_credit_transfer_proof(verifier, verifier)
            ),
        );
    }
}
