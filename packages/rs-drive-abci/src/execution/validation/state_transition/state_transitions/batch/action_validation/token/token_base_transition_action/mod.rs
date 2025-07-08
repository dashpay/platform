use std::collections::BTreeMap;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
use dpp::data_contract::change_control_rules::ChangeControlRules;
use dpp::data_contract::group::Group;
use dpp::data_contract::GroupContractPosition;
use dpp::identifier::Identifier;
use dpp::validation::SimpleConsensusValidationResult;
use drive::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionAction;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;
use crate::error::Error;
use crate::error::execution::ExecutionError;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::batch::action_validation::token::token_base_transition_action::state_v0::TokenBaseTransitionActionStateValidationV0;
use crate::execution::validation::state_transition::batch::action_validation::token::token_base_transition_action::structure_v0::TokenBaseTransitionActionStructureValidationV0;
use crate::platform_types::platform::PlatformStateRef;

mod state_v0;
mod structure_v0;

pub trait TokenBaseTransitionActionValidation {
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;

    fn validate_state(
        &self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;

    #[allow(clippy::too_many_arguments)]
    fn validate_group_action(
        &self,
        rules: &ChangeControlRules,
        owner_id: Identifier,
        contract_owner_id: Identifier,
        main_control_group: Option<GroupContractPosition>,
        groups: &BTreeMap<GroupContractPosition, Group>,
        action_type_string: String,
        token_configuration: &TokenConfiguration,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl TokenBaseTransitionActionValidation for TokenBaseTransitionAction {
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .batch_state_transition
            .token_base_transition_structure_validation
        {
            0 => self.validate_structure_v0(platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "TokenBaseTransitionAction::validate_structure".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    fn validate_state(
        &self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .batch_state_transition
            .token_base_transition_state_validation
        {
            0 => self.validate_state_v0(
                platform,
                owner_id,
                block_info,
                execution_context,
                transaction,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "TokenBaseTransitionAction::validate_state".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    fn validate_group_action(
        &self,
        rules: &ChangeControlRules,
        owner_id: Identifier,
        contract_owner_id: Identifier,
        main_control_group: Option<GroupContractPosition>,
        groups: &BTreeMap<GroupContractPosition, Group>,
        action_type_string: String,
        token_configuration: &TokenConfiguration,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .batch_state_transition
            .token_base_transition_group_action_validation
        {
            0 => self.validate_group_action_v0(
                rules,
                owner_id,
                contract_owner_id,
                main_control_group,
                groups,
                action_type_string,
                token_configuration,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "TokenBaseTransitionAction::validate_group_action".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
