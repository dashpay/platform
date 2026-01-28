mod basic_structure;
mod identity_contract_nonce;
mod state;
#[cfg(test)]
mod tests;

use basic_structure::v0::DataContractUpdateStateTransitionBasicStructureValidationV0;
use basic_structure::v1::DataContractUpdateStateTransitionBasicStructureValidationV1;
use dpp::address_funds::PlatformAddress;
use dpp::block::block_info::BlockInfo;
use dpp::dashcore::Network;
use dpp::fee::Credits;
use dpp::prelude::AddressNonce;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::validation::{ConsensusValidationResult, SimpleConsensusValidationResult};
use std::collections::BTreeMap;

use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;

use crate::error::execution::ExecutionError;
use crate::error::Error;

use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::processor::basic_structure::StateTransitionBasicStructureValidationV0;

use drive::state_transition_action::StateTransitionAction;

use crate::execution::validation::state_transition::data_contract_update::state::v0::DataContractUpdateStateTransitionStateValidationV0;
use crate::execution::validation::state_transition::data_contract_update::state::v1::DataContractUpdateStateTransitionStateValidationV1;
use crate::execution::validation::state_transition::transformer::StateTransitionActionTransformer;
use crate::execution::validation::state_transition::ValidationMode;
use crate::platform_types::platform::PlatformRef;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use crate::rpc::core::CoreRPCLike;

impl StateTransitionBasicStructureValidationV0 for DataContractUpdateTransition {
    fn validate_basic_structure(
        &self,
        network_type: Network,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .contract_update_state_transition
            .basic_structure
        {
            Some(0) => self.validate_basic_structure_v0(network_type, platform_version),
            Some(1) => self.validate_basic_structure_v1(network_type, platform_version),
            Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "data contract update transition: validate_basic_structure".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
            None => Err(Error::Execution(ExecutionError::VersionNotActive {
                method: "data contract update transition: validate_basic_structure".to_string(),
                known_versions: vec![0, 1],
            })),
        }
    }
}

impl StateTransitionActionTransformer for DataContractUpdateTransition {
    fn transform_into_action<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        _remaining_address_input_balances: &Option<
            BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        >,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version = platform.state.current_platform_version()?;

        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .contract_update_state_transition
            .transform_into_action
        {
            0 => self.transform_into_action_v0(
                block_info,
                validation_mode,
                execution_context,
                platform_version,
            ),
            1 => {
                // V0 transitions use the V0 transformer, V1 transitions use the V1 transformer
                match self {
                    DataContractUpdateTransition::V0(_) => self.transform_into_action_v0(
                        block_info,
                        validation_mode,
                        execution_context,
                        platform_version,
                    ),
                    DataContractUpdateTransition::V1(_) => self.transform_into_action_v1(
                        platform,
                        block_info,
                        validation_mode,
                        execution_context,
                        tx,
                        platform_version,
                    ),
                }
            }
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "data contract update transition: transform_into_action".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}
