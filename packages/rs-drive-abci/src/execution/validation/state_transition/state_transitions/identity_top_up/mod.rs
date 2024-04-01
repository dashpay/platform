pub(crate) mod identity_retrieval;
mod structure;
mod transform_into_action;
use dpp::state_transition::identity_topup_transition::IdentityTopUpTransition;
use dpp::validation::{ConsensusValidationResult, SimpleConsensusValidationResult};
use dpp::version::PlatformVersion;

use drive::grovedb::TransactionArg;
use drive::state_transition_action::StateTransitionAction;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;

use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;

use crate::execution::validation::state_transition::identity_top_up::structure::v0::IdentityTopUpStateTransitionStructureValidationV0;
use crate::execution::validation::state_transition::identity_top_up::transform_into_action::v0::IdentityTopUpStateTransitionStateValidationV0;
use crate::execution::validation::state_transition::processor::v0::StateTransitionBasicStructureValidationV0;

use crate::execution::validation::state_transition::ValidationMode;

/// A trait to transform into a top up action
pub trait StateTransitionIdentityTopUpTransitionActionTransformer {
    /// Transform into a top up action
    fn transform_into_action_for_identity_top_up_transition<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        signable_bytes: Vec<u8>,
        validation_mode: ValidationMode,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl StateTransitionIdentityTopUpTransitionActionTransformer for IdentityTopUpTransition {
    fn transform_into_action_for_identity_top_up_transition<C: CoreRPCLike>(
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
            .identity_top_up_state_transition
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
                method: "identity top up transition: transform_top_up_into_action".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

impl StateTransitionBasicStructureValidationV0 for IdentityTopUpTransition {
    fn validate_basic_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .identity_top_up_state_transition
            .basic_structure
        {
            Some(0) => {
                // There is nothing expensive here, so need to ask users to pay for anything
                self.validate_basic_structure_v0(platform_version)
            }
            Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "identity top up transition: validate_basic_structure".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Execution(ExecutionError::VersionNotActive {
                method: "identity top up transition: validate_basic_structure".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}
