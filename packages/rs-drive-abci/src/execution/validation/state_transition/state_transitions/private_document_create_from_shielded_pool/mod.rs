mod transform_into_action;

#[cfg(test)]
mod tests;

use dpp::state_transition::private_document_create_from_shielded_pool_transition::PrivateDocumentCreateFromShieldedPoolTransition;
use dpp::validation::ConsensusValidationResult;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::StateTransitionAction;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::validation::state_transition::private_document_create_from_shielded_pool::transform_into_action::v0::PrivateDocumentCreateFromShieldedPoolStateTransitionTransformIntoActionValidationV0;
use crate::platform_types::platform::PlatformRef;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use crate::rpc::core::CoreRPCLike;

/// A trait to transform into an action for the private document create from shielded pool
/// transition
pub trait StateTransitionPrivateDocumentCreateFromShieldedPoolTransitionActionTransformer {
    /// Transform into an action for the private document create from shielded pool transition
    fn transform_into_action_for_private_document_create_from_shielded_pool_transition<
        C: CoreRPCLike,
    >(
        &self,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl StateTransitionPrivateDocumentCreateFromShieldedPoolTransitionActionTransformer
    for PrivateDocumentCreateFromShieldedPoolTransition
{
    fn transform_into_action_for_private_document_create_from_shielded_pool_transition<
        C: CoreRPCLike,
    >(
        &self,
        platform: &PlatformRef<C>,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version = platform.state.current_platform_version()?;

        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .private_document_create_from_shielded_pool_state_transition
            .transform_into_action
        {
            0 => self.transform_into_action_v0(platform.drive, tx, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method:
                    "private document create from shielded pool transition: transform_into_action"
                        .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
