use dpp::block::block_info::BlockInfo;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::StateTransitionAction;

use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::state_transitions::data_contract_create::state::v1::DataContractCreateStateTransitionStateValidationV1;
use crate::execution::validation::state_transition::state_transitions::data_contract_create::gained_requirements::refuse_unmeetable_gained_requirements;
use crate::execution::validation::state_transition::ValidationMode;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;

pub(in crate::execution::validation::state_transition::state_transitions::data_contract_create) trait DataContractCreateStateTransitionStateValidationV2
{
    fn validate_state_v2<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        validation_mode: ValidationMode,
        tx: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl DataContractCreateStateTransitionStateValidationV2 for DataContractCreateTransition {
    /// v1, then the contract the action would store must have `required` lists a document can
    /// meet, the check `transform_into_action` 1 runs in check_tx
    /// ([`refuse_unmeetable_gained_requirements`]).
    ///
    /// A block builds a contract create's action here rather than through
    /// `transform_into_action`, so this is where a block runs the check. It runs after v1's
    /// reads of the state, so a contract refused for both reports the state's refusal.
    fn validate_state_v2<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        validation_mode: ValidationMode,
        tx: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let result = self.validate_state_v1(
            platform,
            block_info,
            validation_mode,
            tx,
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
    use crate::test::helpers::setup::TestPlatformBuilder;
    use assert_matches::assert_matches;
    use dpp::consensus::basic::BasicError;
    use dpp::consensus::state::state_error::StateError;
    use dpp::consensus::ConsensusError;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::serialized_version::DataContractInSerializationFormat;
    use dpp::data_contract::DataContract;
    use dpp::platform_value::Value;
    use dpp::prelude::IdentityNonce;
    use dpp::state_transition::data_contract_create_transition::DataContractCreateTransitionV0;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use drive::state_transition_action::system::bump_identity_nonce_action::BumpIdentityNonceActionAccessorsV0;
    use platform_version::{DefaultForPlatformVersion, TryIntoPlatformVersioned};

    /// The fixture contract, with `item` added when `item` is given.
    fn create_transition(
        data_contract: DataContract,
        item: Option<Value>,
        platform_version: &PlatformVersion,
    ) -> DataContractCreateTransition {
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
            identity_nonce: IdentityNonce::default(),
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: Default::default(),
        }
        .into()
    }

    fn fixture_contract(platform_version: &PlatformVersion) -> DataContract {
        get_data_contract_fixture(
            None,
            IdentityNonce::default(),
            platform_version.protocol_version,
        )
        .data_contract_owned()
    }

    /// Validates the transition's state as a block does, on a platform holding `stored`.
    fn validate_state(
        transition: &DataContractCreateTransition,
        stored: Option<&DataContract>,
        platform_version: &PlatformVersion,
    ) -> ConsensusValidationResult<StateTransitionAction> {
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();
        if let Some(stored) = stored {
            platform
                .drive
                .apply_contract(
                    stored,
                    BlockInfo::default(),
                    true,
                    None,
                    None,
                    platform_version,
                )
                .expect("failed to apply contract");
        }
        let state = platform.state.load_full();
        let platform_ref = PlatformRef {
            drive: &platform.drive,
            state: &state,
            config: &platform.config,
            core_rpc: &platform.core_rpc,
        };
        let mut execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)
                .expect("failed to create execution context");
        transition
            .validate_state_v2::<MockCoreRPCLike>(
                &platform_ref,
                &BlockInfo::default(),
                ValidationMode::Validator,
                None,
                &mut execution_context,
                platform_version,
            )
            .expect("failed to validate state")
    }

    /// A block refuses the contract check_tx refuses, bumping the owner's nonce.
    #[test]
    fn should_refuse_a_new_contract_whose_required_list_no_document_could_meet() {
        let platform_version = PlatformVersion::latest();
        let data_contract = fixture_contract(platform_version);
        let owner_id = data_contract.owner_id();
        let transition = create_transition(
            data_contract,
            Some(unmeetable_required_schema()),
            platform_version,
        );

        let result = validate_state(&transition, None, platform_version);

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::DataContractInvalidRequiredFieldsUpdateError(error)
            )] if error.document_type() == "item" && error.details().contains("'w'"),
            "{:?}",
            result.errors
        );
        assert_matches!(
            result.data,
            Some(StateTransitionAction::BumpIdentityNonceAction(action))
                if action.identity_id() == owner_id && action.identity_nonce() == 0
        );
    }

    #[test]
    fn should_pass_a_new_contract_whose_required_lists_can_be_met() {
        let platform_version = PlatformVersion::latest();
        let data_contract = fixture_contract(platform_version);
        let data_contract_id = data_contract.id();
        let transition = create_transition(data_contract, None, platform_version);

        let result = validate_state(&transition, None, platform_version);

        assert_matches!(result.errors.as_slice(), [], "{:?}", result.errors);
        assert_matches!(
            result.data,
            Some(StateTransitionAction::DataContractCreateAction(action))
                if action.data_contract_ref().id() == data_contract_id
        );
    }

    /// The check follows v1's reads of the state: a contract both refuse is refused for what
    /// the state holds.
    #[test]
    fn should_report_the_state_refusal_of_a_contract_both_refuse() {
        let platform_version = PlatformVersion::latest();
        let data_contract = fixture_contract(platform_version);
        let transition = create_transition(
            data_contract.clone(),
            Some(unmeetable_required_schema()),
            platform_version,
        );

        let result = validate_state(&transition, Some(&data_contract), platform_version);

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::StateError(
                StateError::DataContractAlreadyPresentError(_)
            )],
            "{:?}",
            result.errors
        );
    }
}
