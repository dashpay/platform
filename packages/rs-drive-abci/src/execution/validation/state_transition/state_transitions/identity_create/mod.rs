mod advanced_structure;
mod basic_structure;
pub(crate) mod identity_and_signatures;
mod state;

use crate::error::Error;

use crate::error::execution::ExecutionError;

use crate::execution::validation::state_transition::identity_create::basic_structure::v0::IdentityCreateStateTransitionBasicStructureValidationV0;
use crate::execution::validation::state_transition::identity_create::state::v0::IdentityCreateStateTransitionStateValidationV0;
use crate::execution::validation::state_transition::processor::v0::StateTransitionBasicStructureValidationV0;
use crate::platform_types::platform::PlatformRef;

use crate::rpc::core::CoreRPCLike;

use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::identity_create_transition::IdentityCreateTransition;

use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;

use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::identity_create::advanced_structure::v0::IdentityCreateStateTransitionAdvancedStructureValidationV0;
use crate::execution::validation::state_transition::ValidationMode;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::identity::identity_create::IdentityCreateTransitionAction;
use drive::state_transition_action::StateTransitionAction;

/// A trait for transforming into an action for the identity create transition
pub trait StateTransitionActionTransformerForIdentityCreateTransitionV0 {
    /// Transforming into the action
    fn transform_into_action_for_identity_create_transition<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        signable_bytes: Vec<u8>,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl StateTransitionActionTransformerForIdentityCreateTransitionV0 for IdentityCreateTransition {
    fn transform_into_action_for_identity_create_transition<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        signable_bytes: Vec<u8>,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version = platform.state.current_platform_version()?;
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .identity_create_state_transition
            .transform_into_action
        {
            0 => self.transform_into_action_v0(
                platform,
                signable_bytes,
                validation_mode,
                execution_context,
                tx,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "identity create transition: transform_into_action".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

impl StateTransitionBasicStructureValidationV0 for IdentityCreateTransition {
    fn validate_basic_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .identity_create_state_transition
            .basic_structure
        {
            Some(0) => {
                // There is nothing expensive to add as validation methods to the execution context
                self.validate_basic_structure_v0(platform_version)
            }
            Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "identity create transition: validate_basic_structure".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Execution(ExecutionError::VersionNotActive {
                method: "identity create transition: validate_basic_structure".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}

/// A trait for advanced structure validation after transforming into an action
pub trait StateTransitionStructureKnownInStateValidationForIdentityCreateTransitionV0 {
    /// Validation of the advanced structure
    fn validate_advanced_structure_from_state_for_identity_create_transition(
        &self,
        action: &IdentityCreateTransitionAction,
        signable_bytes: Vec<u8>,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl StateTransitionStructureKnownInStateValidationForIdentityCreateTransitionV0
    for IdentityCreateTransition
{
    fn validate_advanced_structure_from_state_for_identity_create_transition(
        &self,
        action: &IdentityCreateTransitionAction,
        signable_bytes: Vec<u8>,
        execution_context: &mut StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .identity_create_state_transition
            .advanced_structure
        {
            Some(0) => self.validate_advanced_structure_from_state_v0(
                action,
                signable_bytes,
                execution_context,
                platform_version,
            ),
            Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "identity create transition: validate_advanced_structure_from_state"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Execution(ExecutionError::VersionNotActive {
                method: "identity create transition: validate_advanced_structure_from_state"
                    .to_string(),
                known_versions: vec![0],
            })),
        }
    }
}

/// A trait for state validation for the identity create transition
pub trait StateTransitionStateValidationForIdentityCreateTransitionV0 {
    /// Validate state
    fn validate_state_for_identity_create_transition<C: CoreRPCLike>(
        &self,
        action: IdentityCreateTransitionAction,
        platform: &PlatformRef<C>,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl StateTransitionStateValidationForIdentityCreateTransitionV0 for IdentityCreateTransition {
    fn validate_state_for_identity_create_transition<C: CoreRPCLike>(
        &self,
        action: IdentityCreateTransitionAction,
        platform: &PlatformRef<C>,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version = platform.state.current_platform_version()?;
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .identity_create_state_transition
            .state
        {
            0 => self.validate_state_v0(platform, action, execution_context, tx, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "identity create transition: validate_state".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
