use std::io;
use dpp::asset_lock::StoredAssetLockInfo;
use dpp::consensus::basic::decode::SerializedObjectParsingError;
use dpp::consensus::basic::identity::IdentityAssetLockTransactionOutputNotFoundError;
use dpp::consensus::ConsensusError;
use dpp::fee::Credits;
use dpp::identity::state_transition::AssetLockProved;
use dpp::state_transition::identity_create_transition::IdentityCreateTransition;
use dpp::validation::ConsensusValidationResult;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::StateTransitionAction;
use drive::state_transition_action::system::partially_use_asset_lock_action::PartiallyUseAssetLockAction;
use dpp::version::PlatformVersion;
use crate::error::Error;
use crate::platform_types::platform::PlatformStateRef;

pub(in crate::execution::validation::state_transition::state_transitions::identity_create) trait TransformIntoPartiallyUsedAssetLockActionV0
{
    fn transform_into_partially_used_asset_lock_action_v0(
        &self,
        errors: Vec<ConsensusError>,
        used_credits: Credits,
        platform: &PlatformStateRef,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl TransformIntoPartiallyUsedAssetLockActionV0 for IdentityCreateTransition {
    fn transform_into_partially_used_asset_lock_action_v0(&self, errors: Vec<ConsensusError>, used_credits: Credits, platform: &PlatformStateRef, transaction: TransactionArg, platform_version: &PlatformVersion) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let Some(asset_lock_outpoint) = self.asset_lock_proof().out_point() else {
            return Ok(ConsensusValidationResult::new_with_error(IdentityAssetLockTransactionOutputNotFoundError::new(
                self.asset_lock_proof().output_index() as usize
            ).into()));
        };

        let outpoint_bytes = asset_lock_outpoint
            .try_into()
            .map_err(|e: io::Error| SerializedObjectParsingError::new(e.to_string()))?;

        let stored_asset_lock_info = platform.drive.fetch_asset_lock_outpoint_info(outpoint_bytes, transaction, &platform_version.drive)?;

        match stored_asset_lock_info {
            StoredAssetLockInfo::Present => {
                // We have used the entirety of the asset lock already
            }
            StoredAssetLockInfo::PresentWithInfo(info) => {}
            StoredAssetLockInfo::NotPresent => {}
        }

        let bump_action = StateTransitionAction::PartiallyUseAssetLockAction(
            PartiallyUseAssetLockAction::try_from_borrowed_identity_create_transition(self)?,
        );

        Ok(ConsensusValidationResult::new_with_data_and_errors(
            bump_action,
            errors,
        ))
    }
}