mod advanced_structure;
mod basic_structure;
pub(crate) mod public_key_signatures;
mod state;
#[cfg(test)]
mod tests;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use dpp::address_funds::PlatformAddress;
use dpp::dashcore::Network;
use dpp::fee::Credits;
use std::collections::BTreeMap;

use crate::execution::validation::state_transition::identity_create_from_addresses::basic_structure::v0::IdentityCreateFromAddressesStateTransitionBasicStructureValidationV0;
use crate::execution::validation::state_transition::identity_create_from_addresses::state::v0::IdentityCreateFromAddressesStateTransitionStateValidationV0;
use crate::execution::validation::state_transition::processor::basic_structure::StateTransitionBasicStructureValidationV0;
use crate::platform_types::platform::PlatformRef;

use crate::rpc::core::CoreRPCLike;

use dpp::prelude::{AddressNonce, ConsensusValidationResult};
use dpp::state_transition::identity_create_from_addresses_transition::IdentityCreateFromAddressesTransition;

use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;

use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::identity::identity_create_from_addresses::IdentityCreateFromAddressesTransitionAction;
use drive::state_transition_action::StateTransitionAction;
use crate::execution::validation::state_transition::identity_create_from_addresses::advanced_structure::v0::IdentityCreateFromAddressesStateTransitionAdvancedStructureValidationV0;

/// A trait for transforming into an action for the identity create from addresses transition
pub trait StateTransitionActionTransformerForIdentityCreateFromAddressesTransitionV0 {
    /// Transforming into the action
    fn transform_into_action_for_identity_create_from_addresses_transition<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        remaining_address_input_balances: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl StateTransitionActionTransformerForIdentityCreateFromAddressesTransitionV0
    for IdentityCreateFromAddressesTransition
{
    fn transform_into_action_for_identity_create_from_addresses_transition<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        remaining_address_input_balances: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version = platform.state.current_platform_version()?;
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .identity_create_from_addresses_state_transition
            .transform_into_action
        {
            0 => self.transform_into_action_v0(remaining_address_input_balances),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "identity create from addresses transition: transform_into_action"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

impl StateTransitionBasicStructureValidationV0 for IdentityCreateFromAddressesTransition {
    fn validate_basic_structure(
        &self,
        _network_type: Network,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .identity_create_from_addresses_state_transition
            .basic_structure
        {
            Some(0) => {
                // There is nothing expensive to add as validation methods to the execution context
                self.validate_basic_structure_v0(platform_version)
            }
            Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "identity create from addresses transition: validate_basic_structure"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Execution(ExecutionError::VersionNotActive {
                method: "identity create from addresses transition: validate_basic_structure"
                    .to_string(),
                known_versions: vec![0],
            })),
        }
    }
}

/// A trait for advanced structure validation after transforming into an action
pub trait StateTransitionStructureKnownInStateValidationForIdentityCreateFromAddressesTransitionV0 {
    /// Validation of the advanced structure
    fn validate_advanced_structure_from_state_for_identity_create_from_addresses_transition(
        &self,
        signable_bytes: Vec<u8>,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl StateTransitionStructureKnownInStateValidationForIdentityCreateFromAddressesTransitionV0
    for IdentityCreateFromAddressesTransition
{
    fn validate_advanced_structure_from_state_for_identity_create_from_addresses_transition(
        &self,
        signable_bytes: Vec<u8>,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .identity_create_from_addresses_state_transition
            .advanced_structure
        {
            Some(0) => self.validate_advanced_structure_v0(
                signable_bytes,
                execution_context,
                platform_version,
            ),
            Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "identity create from addresses transition: validate_advanced_structure_from_state"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Execution(ExecutionError::VersionNotActive {
                method: "identity create from addresses transition: validate_advanced_structure_from_state"
                    .to_string(),
                known_versions: vec![0],
            })),
        }
    }
}

/// A trait for state validation for the identity create from addresses transition
pub trait StateTransitionStateValidationForIdentityCreateFromAddressesTransitionV0 {
    /// Validate state
    fn validate_state_for_identity_create_from_addresses_transition<C: CoreRPCLike>(
        &self,
        action: IdentityCreateFromAddressesTransitionAction,
        platform: &PlatformRef<C>,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl StateTransitionStateValidationForIdentityCreateFromAddressesTransitionV0
    for IdentityCreateFromAddressesTransition
{
    fn validate_state_for_identity_create_from_addresses_transition<C: CoreRPCLike>(
        &self,
        action: IdentityCreateFromAddressesTransitionAction,
        platform: &PlatformRef<C>,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version = platform.state.current_platform_version()?;
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .identity_create_from_addresses_state_transition
            .state
        {
            0 => self.validate_state_v0(platform, action, execution_context, tx, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "identity create from addresses transition: validate_state".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
