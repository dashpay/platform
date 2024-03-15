//! Verifies the execution of a state transition using a provided proof.
//!
use crate::drive::verify::RootHash;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::data_contract::DataContract;
use dpp::identifier::Identifier;
use dpp::identity::TimestampMillis;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::state_transition::StateTransition;
use dpp::version::PlatformVersion;
use std::sync::Arc;

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
        block_time: TimestampMillis,
        proof: &[u8],
        known_contracts_provider_fn: &impl Fn(&Identifier) -> Result<Option<Arc<DataContract>>, Error>,
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
                block_time,
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
