use dpp::consensus::ConsensusError;
use dpp::fee::Credits;
use dpp::state_transition::identity_create_transition::IdentityCreateTransition;
use dpp::validation::ConsensusValidationResult;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::StateTransitionAction;
use dpp::version::PlatformVersion;
use crate::error::Error;
use crate::error::execution::ExecutionError;
use crate::execution::validation::state_transition::identity_create::transform_into_partially_used_asset_lock_action::v0::TransformIntoPartiallyUsedAssetLockActionV0;
use crate::platform_types::platform::PlatformStateRef;

mod v0;

pub(in crate::execution::validation::state_transition::state_transitions::identity_create) trait TransformIntoPartiallyUsedAssetLockAction
{
    fn transform_into_partially_used_asset_lock_action(
        &self,
        errors: Vec<ConsensusError>,
        used_credits: Credits,
        platform: &PlatformStateRef,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl TransformIntoPartiallyUsedAssetLockAction for IdentityCreateTransition {
    fn transform_into_partially_used_asset_lock_action(&self, errors: Vec<ConsensusError>, used_credits: Credits, platform: &PlatformStateRef, transaction: TransactionArg, platform_version: &PlatformVersion) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        match platform_version.drive_abci.validation_and_processing.state_transitions.identity_create_state_transition.transform_into_partially_used_asset_lock_action
        {
            Some(0) => self.transform_into_partially_used_asset_lock_action_v0(errors, used_credits, platform, transaction, platform_version),
            Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "identity create transition: transform_into_partially_used_asset_lock_action".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Execution(ExecutionError::VersionNotActive {
                method: "identity create transition: transform_into_partially_used_asset_lock_action".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}