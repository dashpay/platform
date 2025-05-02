mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

use crate::error::proof::ProofError;
use dpp::state_transition::StateTransition;
use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

pub type ProofCreationResult<TData> = ValidationResult<TData, ProofError>;

impl Drive {
    /// This function calls the versioned `prove_state_transition`
    /// function based on the version provided in the `PlatformVersion` parameter. It panics if the
    /// version doesn't match any existing versioned functions.
    ///
    /// # Parameters
    /// - `state_transition`: The [StateTransition] object for which we want to generate a proof.
    /// - `transaction`: An optional grovedb transaction.
    /// - `platform_version`: A reference to the [PlatformVersion] object that specifies the version of
    ///   the function to call.
    ///
    /// # Returns
    /// Returns a `Result` with a `Vec<u8>` containing the proof data if the function succeeds,
    /// or an `Error` if the function fails.
    pub fn prove_state_transition(
        &self,
        state_transition: &StateTransition,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ProofCreationResult<Vec<u8>>, Error> {
        match platform_version.drive.methods.prove.prove_state_transition {
            0 => self.prove_state_transition_v0(state_transition, transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_state_transition".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
