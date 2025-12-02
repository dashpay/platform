use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::identity_update::advanced_structure::v0::IdentityUpdateStateTransitionIdentityAndSignaturesValidationV0;
use dpp::identity::PartialIdentity;
use dpp::prelude::ConsensusValidationResult;
use dpp::serialization::Signable;
use dpp::state_transition::StateTransition;
use dpp::version::PlatformVersion;
use drive::state_transition_action::StateTransitionAction;

/// A trait for validating state transitions within a blockchain.
/// The advanced structure validation should always happen in a block
/// and not in check_tx
pub(crate) trait StateTransitionAdvancedStructureValidationV0 {
    /// Validates the structure of a transaction by checking its basic elements.
    ///
    /// # Arguments
    ///
    /// * `platform` - A reference to the platform state ref.
    /// * `platform_version` - The platform version.
    ///
    /// # Returns
    ///
    /// * `Result<SimpleConsensusValidationResult, Error>` - A result with either a SimpleConsensusValidationResult or an Error.
    fn validate_advanced_structure(
        &self,
        identity: &PartialIdentity,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;

    /// True if the state transition has advanced structure validation.
    /// This structure validation makes users pay if there is a failure
    fn has_advanced_structure_validation_without_state(&self) -> bool;
}

impl StateTransitionAdvancedStructureValidationV0 for StateTransition {
    fn validate_advanced_structure(
        &self,
        identity: &PartialIdentity,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        match self {
            StateTransition::IdentityUpdate(st) => {
                match platform_version
                    .drive_abci
                    .validation_and_processing
                    .state_transitions
                    .identity_update_state_transition
                    .advanced_structure
                {
                    Some(0) => {
                        let signable_bytes: Vec<u8> = self.signable_bytes()?;
                        st.validate_identity_update_state_transition_signatures_v0(
                            signable_bytes,
                            identity,
                            execution_context,
                        )
                    }
                    Some(version) => {
                        Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                            method: "identity update transition: validate_advanced_structure"
                                .to_string(),
                            known_versions: vec![0],
                            received: version,
                        }))
                    }
                    None => Err(Error::Execution(ExecutionError::VersionNotActive {
                        method: "identity update transition: validate_advanced_structure"
                            .to_string(),
                        known_versions: vec![0],
                    })),
                }
            }
            StateTransition::DataContractCreate(st) => {
                st.validate_advanced_structure(identity, execution_context, platform_version)
            }
            _ => Ok(ConsensusValidationResult::<StateTransitionAction>::new()),
        }
    }

    fn has_advanced_structure_validation_without_state(&self) -> bool {
        matches!(
            self,
            StateTransition::IdentityUpdate(_)
                | StateTransition::DataContractCreate(_)
                | StateTransition::IdentityCreateFromAddresses(_)
        )
    }
}
