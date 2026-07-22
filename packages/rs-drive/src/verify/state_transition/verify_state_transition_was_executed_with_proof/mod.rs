//! Verifies the execution of a state transition using a provided proof.
//!
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::ContractLookupFn;
use crate::verify::RootHash;
use dpp::block::block_info::BlockInfo;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::state_transition::StateTransition;
use dpp::version::PlatformVersion;

mod v0;

impl Drive {
    /// Verifies the execution of a state transition using a provided proof.
    ///
    /// This method checks if the state transition has been executed and is included in the proof.
    /// It supports different versions of the verification process, which are handled based on the
    /// platform version specified.
    ///
    /// # Parameters
    ///
    /// - `state_transition`: A reference to the `StateTransition` that needs to be verified.
    /// - `proof`: A byte slice representing the cryptographic proof to be verified.
    /// - `known_contracts`: A `HashMap` mapping `Identifier`s to references of `DataContract`s that are known.
    /// - `platform_version`: A reference to the `PlatformVersion` which dictates the verification method to be used.
    ///
    /// # Returns
    ///
    /// A `Result` containing either:
    /// - On success: a tuple of `RootHash` and `StateTransitionProofResult`, where `RootHash` is the root hash of the
    ///   proof and `StateTransitionProofResult` contains the result of the proof verification.
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
    ) -> Result<(RootHash, StateTransitionProofResult), Error> {
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
                v0::StateTransitionProofSemantics::ExecutionEvidence,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_state_transition_was_executed_with_proof".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Verifies the state a state transition affects, using a provided proof.
    ///
    /// This behaves exactly like
    /// [`verify_state_transition_was_executed_with_proof`](Self::verify_state_transition_was_executed_with_proof)
    /// for transition types whose proof can be bound to execution. For the
    /// transition families where the current proof format cannot establish
    /// request-specific completion (balance top-ups, credit transfers and
    /// withdrawals, address funds movements, shields, and no-history token
    /// burn/mint/transfer), it returns a verified snapshot of the affected
    /// state instead of failing: the queried keys are derived from the
    /// transition, and the values are authenticated as of the proof's block.
    ///
    /// # Important
    ///
    /// A snapshot result is **not** evidence that the transition executed.
    /// Callers must treat these results as height-pinned state snapshots
    /// (e.g. for balance reconciliation), never as execution confirmation.
    pub fn verify_state_transition_affected_state_with_proof(
        state_transition: &StateTransition,
        block_info: &BlockInfo,
        proof: &[u8],
        known_contracts_provider_fn: &ContractLookupFn,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, StateTransitionProofResult), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .state_transition
            .verify_state_transition_affected_state_with_proof
        {
            0 => Drive::verify_state_transition_was_executed_with_proof_v0(
                state_transition,
                block_info,
                proof,
                known_contracts_provider_fn,
                v0::StateTransitionProofSemantics::AllowAffectedState,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_state_transition_affected_state_with_proof".to_string(),
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
