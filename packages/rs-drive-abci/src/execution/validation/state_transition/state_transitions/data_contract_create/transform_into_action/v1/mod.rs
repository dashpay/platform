use dpp::block::block_info::BlockInfo;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
use dpp::version::PlatformVersion;
use drive::state_transition_action::StateTransitionAction;

use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::state_transitions::data_contract_create::gained_requirements::refuse_unmeetable_gained_requirements;
use crate::execution::validation::state_transition::state_transitions::data_contract_create::state::v0::DataContractCreateStateTransitionStateValidationV0;
use crate::execution::validation::state_transition::ValidationMode;
use crate::rpc::core::CoreRPCLike;

pub(in crate::execution::validation::state_transition::state_transitions::data_contract_create) trait DataContractCreateStateTransitionTransformIntoActionV1
{
    fn transform_into_action_v1<C: CoreRPCLike>(
        &self,
        block_info: &BlockInfo,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl DataContractCreateStateTransitionTransformIntoActionV1 for DataContractCreateTransition {
    /// v0, then the contract the action would store must have `required` lists a document can
    /// meet ([`refuse_unmeetable_gained_requirements`]).
    ///
    /// check_tx and recheck build a contract create's action here. A block builds it in state
    /// validation, whose generation 2 runs the same check.
    fn transform_into_action_v1<C: CoreRPCLike>(
        &self,
        block_info: &BlockInfo,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let result = self.transform_into_action_v0::<C>(
            block_info,
            validation_mode,
            execution_context,
            platform_version,
        )?;
        refuse_unmeetable_gained_requirements(self, result, platform_version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::validation::state_transition::state_transitions::data_contract_common::contract_structure_test_harness::unmeetable_required_schema;
    use crate::rpc::core::MockCoreRPCLike;
    use assert_matches::assert_matches;
    use dpp::consensus::basic::BasicError;
    use dpp::consensus::ConsensusError;
    use dpp::data_contract::serialized_version::DataContractInSerializationFormat;
    use dpp::platform_value::Value;
    use dpp::prelude::IdentityNonce;
    use dpp::state_transition::data_contract_create_transition::DataContractCreateTransitionV0;
    use dpp::state_transition::StateTransitionOwned;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use drive::state_transition_action::system::bump_identity_nonce_action::BumpIdentityNonceActionAccessorsV0;
    use platform_version::{DefaultForPlatformVersion, TryIntoPlatformVersioned};

    /// The fixture contract, with `item` added when `item` is given.
    fn create_transition(
        item: Option<Value>,
        platform_version: &PlatformVersion,
    ) -> DataContractCreateTransition {
        let identity_nonce = IdentityNonce::default();
        let data_contract =
            get_data_contract_fixture(None, identity_nonce, platform_version.protocol_version)
                .data_contract_owned();
        let mut carried: DataContractInSerializationFormat = data_contract
            .try_into_platform_versioned(platform_version)
            .expect("failed to convert data contract");
        if let Some(item) = item {
            carried
                .document_schemas_mut()
                .expect("expected a transport format's schemas")
                .insert("item".to_string(), item);
        }
        DataContractCreateTransitionV0 {
            data_contract: carried,
            identity_nonce,
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: Default::default(),
        }
        .into()
    }

    fn transform(
        transition: &DataContractCreateTransition,
        validation_mode: ValidationMode,
        platform_version: &PlatformVersion,
    ) -> ConsensusValidationResult<StateTransitionAction> {
        let mut execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)
                .expect("failed to create execution context");
        transition
            .transform_into_action_v1::<MockCoreRPCLike>(
                &BlockInfo::default(),
                validation_mode,
                &mut execution_context,
                platform_version,
            )
            .expect("failed to transform the transition")
    }

    /// The check runs on the parsed contract whether or not the parse validated it, so check_tx
    /// and recheck refuse the contract with the error a block charges for, bumping the nonce.
    #[test]
    fn should_refuse_a_new_contract_whose_required_list_no_document_could_meet() {
        let platform_version = PlatformVersion::latest();
        let transition = create_transition(Some(unmeetable_required_schema()), platform_version);
        let owner_id = transition.owner_id();

        for validation_mode in [
            ValidationMode::CheckTx,
            ValidationMode::RecheckTx,
            ValidationMode::Validator,
        ] {
            let result = transform(&transition, validation_mode, platform_version);
            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::DataContractInvalidRequiredFieldsUpdateError(error)
                )] if error.document_type() == "item" && error.details().contains("'w'"),
                "{validation_mode:?}: {:?}",
                result.errors
            );
            assert_matches!(
                result.data,
                Some(StateTransitionAction::BumpIdentityNonceAction(action))
                    if action.identity_id() == owner_id && action.identity_nonce() == 0,
                "{validation_mode:?}"
            );
        }
    }

    #[test]
    fn should_build_the_create_action_of_a_contract_whose_required_lists_can_be_met() {
        let platform_version = PlatformVersion::latest();
        let transition = create_transition(None, platform_version);

        for validation_mode in [
            ValidationMode::CheckTx,
            ValidationMode::RecheckTx,
            ValidationMode::Validator,
        ] {
            let result = transform(&transition, validation_mode, platform_version);
            assert_matches!(result.errors.as_slice(), [], "{validation_mode:?}");
            assert_matches!(
                result.data,
                Some(StateTransitionAction::DataContractCreateAction(_)),
                "{validation_mode:?}"
            );
        }
    }

    /// A contract the parse refuses keeps v0's refusal: the check reads only a parsed contract.
    #[test]
    fn should_keep_the_refusal_of_a_contract_the_parse_refuses() {
        let platform_version = PlatformVersion::latest();
        let transition = create_transition(Some(Value::Null), platform_version);

        let result = transform(&transition, ValidationMode::Validator, platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(BasicError::ContractError(_))],
            "{:?}",
            result.errors
        );
        assert_matches!(
            result.data,
            Some(StateTransitionAction::BumpIdentityNonceAction(_))
        );
    }
}
