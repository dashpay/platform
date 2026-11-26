mod v0;
mod v1;

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
            1 => self.prove_state_transition_v1(state_transition, transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_state_transition".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }

    /// Whether the proof [`Drive::prove_state_transition`] makes of this state transition at
    /// this platform version is of a fixed set of keys, so that making it costs a bounded
    /// amount of work whatever the state holds. Each version of the prover answers for what it
    /// proves, so a version that changes a kind's query says here whether it is still bounded.
    ///
    /// Only the kinds a caller relies on are declared: `false` does not mean a kind's proof
    /// is unbounded.
    pub fn state_transition_proof_has_fixed_keys(
        state_transition: &StateTransition,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        match platform_version.drive.methods.prove.prove_state_transition {
            // Version 1 proves these kinds as version 0 does
            0 | 1 => Ok(Self::state_transition_proof_has_fixed_keys_v0(
                state_transition,
            )),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "state_transition_proof_has_fixed_keys".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::Drive;
    use dpp::state_transition::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;
    use dpp::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
    use dpp::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0;
    use dpp::state_transition::identity_update_transition::IdentityUpdateTransition;
    use dpp::state_transition::StateTransition;
    use dpp::version::PLATFORM_VERSIONS;

    #[test]
    fn should_declare_a_credit_transfer_proof_of_fixed_keys_and_an_identity_update_not() {
        let transfer: StateTransition =
            IdentityCreditTransferTransition::V0(IdentityCreditTransferTransitionV0::default())
                .into();
        // An identity update proves every key of the identity
        let update: StateTransition =
            IdentityUpdateTransition::V0(IdentityUpdateTransitionV0::default()).into();

        for platform_version in PLATFORM_VERSIONS {
            assert_eq!(
                Drive::state_transition_proof_has_fixed_keys(&transfer, platform_version).ok(),
                Some(true)
            );
            assert_eq!(
                Drive::state_transition_proof_has_fixed_keys(&update, platform_version).ok(),
                Some(false)
            );
        }
    }
}
