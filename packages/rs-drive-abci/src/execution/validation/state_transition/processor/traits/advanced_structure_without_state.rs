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
            StateTransition::IdentityUpdate(_) | StateTransition::DataContractCreate(_)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn make_data_contract_create_st() -> StateTransition {
        use dpp::tests::fixtures::get_data_contract_fixture;
        use platform_version::TryIntoPlatformVersioned;
        let platform_version = platform_version::version::PlatformVersion::latest();
        let created_data_contract =
            get_data_contract_fixture(None, 1, platform_version.protocol_version);
        let transition: dpp::state_transition::data_contract_create_transition::DataContractCreateTransition =
            created_data_contract.try_into_platform_versioned(platform_version).unwrap();
        transition.into()
    }

    fn make_data_contract_update_st() -> StateTransition {
        use dpp::data_contract::accessors::v0::DataContractV0Getters;
        use dpp::tests::fixtures::get_data_contract_fixture;
        use platform_version::TryIntoPlatformVersioned;
        let platform_version = platform_version::version::PlatformVersion::latest();
        let created_data_contract =
            get_data_contract_fixture(None, 1, platform_version.protocol_version);
        let data_contract = created_data_contract.data_contract().clone();
        let transition: dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition =
            (data_contract, 2u64).try_into_platform_versioned(platform_version).unwrap();
        transition.into()
    }

    use dpp::state_transition::batch_transition::BatchTransition;
    use dpp::state_transition::batch_transition::BatchTransitionV0;
    use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
    use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
    use dpp::state_transition::identity_create_transition::v0::IdentityCreateTransitionV0;
    use dpp::state_transition::identity_create_transition::IdentityCreateTransition;
    use dpp::state_transition::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;
    use dpp::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
    use dpp::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0;
    use dpp::state_transition::identity_update_transition::IdentityUpdateTransition;
    use dpp::state_transition::masternode_vote_transition::v0::MasternodeVoteTransitionV0;
    use dpp::state_transition::masternode_vote_transition::MasternodeVoteTransition;

    mod has_advanced_structure_validation_without_state {
        use super::*;

        #[test]
        fn should_return_true_for_identity_update_and_data_contract_create() {
            let transitions: Vec<(&str, StateTransition)> = vec![
                (
                    "IdentityUpdate",
                    StateTransition::IdentityUpdate(IdentityUpdateTransition::V0(
                        IdentityUpdateTransitionV0::default(),
                    )),
                ),
                ("DataContractCreate", make_data_contract_create_st()),
            ];
            for (name, st) in transitions {
                assert!(
                    st.has_advanced_structure_validation_without_state(),
                    "expected true for {}",
                    name
                );
            }
        }

        #[test]
        fn should_return_false_for_all_other_transitions() {
            let transitions: Vec<(&str, StateTransition)> = vec![
                ("DataContractUpdate", make_data_contract_update_st()),
                (
                    "Batch",
                    StateTransition::Batch(BatchTransition::V0(BatchTransitionV0::default())),
                ),
                (
                    "IdentityCreate",
                    StateTransition::IdentityCreate(IdentityCreateTransition::V0(
                        IdentityCreateTransitionV0::default(),
                    )),
                ),
                (
                    "IdentityCreditTransfer",
                    StateTransition::IdentityCreditTransfer(IdentityCreditTransferTransition::V0(
                        IdentityCreditTransferTransitionV0::default(),
                    )),
                ),
                (
                    "MasternodeVote",
                    StateTransition::MasternodeVote(MasternodeVoteTransition::V0(
                        MasternodeVoteTransitionV0::default(),
                    )),
                ),
            ];
            for (name, st) in transitions {
                assert!(
                    !st.has_advanced_structure_validation_without_state(),
                    "expected false for {}",
                    name
                );
            }
        }
    }
}
