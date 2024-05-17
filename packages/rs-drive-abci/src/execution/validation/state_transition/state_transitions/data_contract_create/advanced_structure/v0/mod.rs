use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use dpp::consensus::basic::data_contract::{
    InvalidDataContractIdError, InvalidDataContractVersionError,
};
use dpp::consensus::basic::BasicError;
use dpp::data_contract::INITIAL_DATA_CONTRACT_VERSION;
use dpp::prelude::DataContract;
use dpp::state_transition::data_contract_create_transition::accessors::DataContractCreateTransitionAccessorsV0;
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
use dpp::validation::ConsensusValidationResult;
use drive::state_transition_action::system::bump_identity_nonce_action::BumpIdentityNonceAction;
use drive::state_transition_action::StateTransitionAction;

pub(in crate::execution::validation::state_transition::state_transitions::data_contract_create) trait DataContractCreatedStateTransitionAdvancedStructureValidationV0 {
    fn validate_advanced_structure_v0(&self, execution_context: &mut StateTransitionExecutionContext) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl DataContractCreatedStateTransitionAdvancedStructureValidationV0
    for DataContractCreateTransition
{
    fn validate_advanced_structure_v0(
        &self,
        execution_context: &mut StateTransitionExecutionContext,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        if self.data_contract().version() != INITIAL_DATA_CONTRACT_VERSION {
            let bump_action = StateTransitionAction::BumpIdentityNonceAction(
                BumpIdentityNonceAction::from_borrowed_data_contract_create_transition(self),
            );

            return Ok(ConsensusValidationResult::new_with_data_and_errors(
                bump_action,
                vec![InvalidDataContractVersionError::new(
                    INITIAL_DATA_CONTRACT_VERSION,
                    self.data_contract().version(),
                )
                .into()],
            ));
        }

        // Validate data contract id
        let generated_id = DataContract::generate_data_contract_id_v0(
            self.data_contract().owner_id(),
            self.identity_nonce(),
        );

        // This hash will only take 1 block (64 bytes)
        execution_context.add_operation(ValidationOperation::DoubleSha256(1));

        if generated_id != self.data_contract().id() {
            let bump_action = StateTransitionAction::BumpIdentityNonceAction(
                BumpIdentityNonceAction::from_borrowed_data_contract_create_transition(self),
            );

            return Ok(ConsensusValidationResult::new_with_data_and_errors(
                bump_action,
                vec![
                    BasicError::InvalidDataContractIdError(InvalidDataContractIdError::new(
                        generated_id.to_vec(),
                        self.data_contract().id().to_vec(),
                    ))
                    .into(),
                ],
            ));
        }

        Ok(ConsensusValidationResult::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_matches::assert_matches;

    mod validate_advanced_structure {
        use super::*;
        use dpp::consensus::ConsensusError;
        use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
        use dpp::prelude::{Identifier, IdentityNonce};
        use dpp::state_transition::data_contract_create_transition::DataContractCreateTransitionV0;
        use dpp::tests::fixtures::get_data_contract_fixture;
        use drive::state_transition_action::system::bump_identity_nonce_action::BumpIdentityNonceActionAccessorsV0;
        use platform_version::version::PlatformVersion;
        use platform_version::{DefaultForPlatformVersion, TryIntoPlatformVersioned};

        #[test]
        fn should_return_invalid_result_if_contract_version_is_not_initial() {
            let platform_version = PlatformVersion::latest();
            let identity_nonce = IdentityNonce::default();

            let mut data_contract =
                get_data_contract_fixture(None, identity_nonce, platform_version.protocol_version)
                    .data_contract_owned();

            data_contract.set_version(6);

            let identity_id = data_contract.owner_id();

            let data_contract_for_serialization = data_contract
                .try_into_platform_versioned(platform_version)
                .expect("failed to convert data contract");

            let transition: DataContractCreateTransition = DataContractCreateTransitionV0 {
                data_contract: data_contract_for_serialization,
                identity_nonce,
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: Default::default(),
            }
            .into();

            let mut execution_context =
                StateTransitionExecutionContext::default_for_platform_version(platform_version)
                    .expect("failed to create execution context");

            let result = transition
                .validate_advanced_structure_v0(&mut execution_context)
                .expect("failed to validate advanced structure");

            assert_matches!(execution_context.operations_slice(), []);

            assert_matches!(
                result.data,
                Some(StateTransitionAction::BumpIdentityNonceAction(action))
                if action.identity_id() == identity_id && action.identity_nonce() == identity_nonce
            );

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(BasicError::InvalidDataContractVersionError(e))] if e.expected_version() == INITIAL_DATA_CONTRACT_VERSION && e.version() == 6
            );
        }

        #[test]
        fn should_return_invalid_result_if_contract_id_is_not_valid() {
            let platform_version = PlatformVersion::latest();
            let identity_nonce = IdentityNonce::default();

            let mut data_contract =
                get_data_contract_fixture(None, identity_nonce, platform_version.protocol_version)
                    .data_contract_owned();

            let identity_id = data_contract.owner_id();
            let original_id = data_contract.id();
            let invalid_id = Identifier::default();

            data_contract.set_id(invalid_id);

            let data_contract_for_serialization = data_contract
                .try_into_platform_versioned(platform_version)
                .expect("failed to convert data contract");

            let transition: DataContractCreateTransition = DataContractCreateTransitionV0 {
                data_contract: data_contract_for_serialization,
                identity_nonce,
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: Default::default(),
            }
            .into();

            let mut execution_context =
                StateTransitionExecutionContext::default_for_platform_version(platform_version)
                    .expect("failed to create execution context");

            let result = transition
                .validate_advanced_structure_v0(&mut execution_context)
                .expect("failed to validate advanced structure");

            assert_matches!(
                execution_context.operations_slice(),
                [ValidationOperation::DoubleSha256(1)]
            );

            assert_matches!(
                result.data,
                Some(StateTransitionAction::BumpIdentityNonceAction(action))
                if action.identity_id() == identity_id && action.identity_nonce() == identity_nonce
            );

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(BasicError::InvalidDataContractIdError(e))]
                if Identifier::try_from(e.expected_id()).unwrap() == original_id
                    && Identifier::try_from(e.invalid_id()).unwrap() == invalid_id
            );
        }
    }
}
