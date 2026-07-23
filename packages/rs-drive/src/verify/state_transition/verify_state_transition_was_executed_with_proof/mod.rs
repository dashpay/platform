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
        match platform_version
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
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_state_transition_was_executed_with_proof".to_string(),
                known_versions: vec![0],
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
}
