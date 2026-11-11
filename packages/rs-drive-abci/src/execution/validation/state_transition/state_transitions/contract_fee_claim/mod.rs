mod basic_structure;
mod identity_contract_nonce;
mod state;
#[cfg(test)]
mod tests;

use dpp::address_funds::PlatformAddress;
use dpp::block::block_info::BlockInfo;
use dpp::dashcore::Network;
use dpp::fee::Credits;
use dpp::prelude::AddressNonce;
use dpp::state_transition::contract_fee_claim_transition::ContractFeeClaimTransition;
use dpp::validation::{ConsensusValidationResult, SimpleConsensusValidationResult};
use dpp::version::PlatformVersion;
use drive::state_transition_action::StateTransitionAction;
use std::collections::BTreeMap;

use drive::grovedb::TransactionArg;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;

use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;

use crate::execution::validation::state_transition::contract_fee_claim::basic_structure::v0::ContractFeeClaimStateTransitionStructureValidationV0;
use crate::execution::validation::state_transition::contract_fee_claim::state::v0::ContractFeeClaimStateTransitionStateValidationV0;
use crate::execution::validation::state_transition::processor::basic_structure::StateTransitionBasicStructureValidationV0;
use crate::execution::validation::state_transition::processor::state::StateTransitionStateValidation;
use crate::execution::validation::state_transition::transformer::StateTransitionActionTransformer;
use crate::execution::validation::state_transition::ValidationMode;
use crate::platform_types::platform_state::PlatformStateV0Methods;

impl StateTransitionActionTransformer for ContractFeeClaimTransition {
    fn transform_into_action<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        _remaining_address_input_balances: &Option<
            BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        >,
        _validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version = platform.state.current_platform_version()?;

        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .contract_fee_claim_state_transition
            .transform_into_action
        {
            0 => self.transform_into_action_v0(
                platform,
                block_info,
                execution_context,
                tx,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "contract fee claim transition: transform_into_action".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

impl StateTransitionBasicStructureValidationV0 for ContractFeeClaimTransition {
    fn validate_basic_structure(
        &self,
        _network_type: Network,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .contract_fee_claim_state_transition
            .basic_structure
        {
            Some(0) => self.validate_basic_structure_v0(platform_version),
            Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "contract fee claim transition: validate_basic_structure".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Execution(ExecutionError::VersionNotActive {
                method: "contract fee claim transition: validate_basic_structure".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}

impl StateTransitionStateValidation for ContractFeeClaimTransition {
    fn validate_state<C: CoreRPCLike>(
        &self,
        _action: Option<StateTransitionAction>,
        platform: &PlatformRef<C>,
        _validation_mode: ValidationMode,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version = platform.state.current_platform_version()?;
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .contract_fee_claim_state_transition
            .state
        {
            // The state validation is the transformation: the contract and the pot are read
            // and checked while the action that carries the payouts is built.
            0 => self.transform_into_action_v0(
                platform,
                block_info,
                execution_context,
                tx,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "contract fee claim transition: validate_state".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
