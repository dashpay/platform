mod basic_structure;
mod identity_nonce;
mod state;

use dpp::block::block_info::BlockInfo;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;

use drive::grovedb::TransactionArg;
use drive::state_transition_action::StateTransitionAction;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;

use crate::execution::validation::state_transition::data_contract_create::basic_structure::v0::DataContractCreatedStateTransitionBasicStructureValidationV0;
use crate::execution::validation::state_transition::data_contract_create::state::v0::DataContractCreateStateTransitionStateValidationV0;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;

use crate::execution::validation::state_transition::processor::v0::{
    StateTransitionBasicStructureValidationV0, StateTransitionStateValidationV0,
};
use crate::execution::validation::state_transition::transformer::StateTransitionActionTransformerV0;
use crate::execution::validation::state_transition::ValidationMode;

impl ValidationMode {
    /// Returns if we should validate the contract when we transform it from its serialized form
    pub fn should_validate_contract_on_transform_into_action(&self) -> bool {
        match self {
            ValidationMode::CheckTx => false,
            ValidationMode::RecheckTx => false,
            ValidationMode::Validator => true,
        }
    }
}

impl StateTransitionActionTransformerV0 for DataContractCreateTransition {
    fn transform_into_action<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        _block_info: &BlockInfo,
        validation_mode: ValidationMode,
        _execution_context: &mut StateTransitionExecutionContext,
        _tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version = platform.state.current_platform_version()?;

        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .contract_create_state_transition
            .transform_into_action
        {
            0 => self.transform_into_action_v0::<C>(validation_mode, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "data contract create transition: transform_into_action".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

impl StateTransitionBasicStructureValidationV0 for DataContractCreateTransition {
    fn validate_basic_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .contract_create_state_transition
            .basic_structure
        {
            Some(0) => self.validate_base_structure_v0(platform_version),
            Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "data contract create transition: validate_basic_structure".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Execution(ExecutionError::VersionNotActive {
                method: "data contract create transition: validate_basic_structure".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}

impl StateTransitionStateValidationV0 for DataContractCreateTransition {
    fn validate_state<C: CoreRPCLike>(
        &self,
        _action: Option<StateTransitionAction>,
        platform: &PlatformRef<C>,
        validation_mode: ValidationMode,
        _execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version = platform.state.current_platform_version()?;

        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .contract_create_state_transition
            .state
        {
            0 => self.validate_state_v0(platform, validation_mode, tx, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "data contract create transition: validate_state".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
