use std::io;
use dpp::asset_lock::StoredAssetLockInfo;
use dpp::consensus::basic::decode::SerializedObjectParsingError;
use dpp::consensus::basic::identity::IdentityAssetLockTransactionOutputNotFoundError;
use dpp::identity::state_transition::AssetLockProved;
use crate::error::Error;
use dpp::state_transition::identity_create_transition::accessors::IdentityCreateTransitionAccessorsV0;
use dpp::state_transition::identity_create_transition::IdentityCreateTransition;
use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use dpp::validation::ConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::identity::identity_create::IdentityCreateTransitionAction;
use drive::state_transition_action::StateTransitionAction;
use drive::state_transition_action::system::partially_use_asset_lock_action::PartiallyUseAssetLockAction;
use crate::platform_types::platform::{PlatformRef, PlatformStateRef};

pub(in crate::execution::validation::state_transition::state_transitions::identity_create) trait IdentityCreateStateTransitionAdvancedStructureValidationV0
{
    fn validate_advanced_structure_from_state_v0<C>(
        &self,
        // The state here is only to be used to query information for the action
        platform: &PlatformStateRef,
        action: &IdentityCreateTransitionAction,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl IdentityCreateStateTransitionAdvancedStructureValidationV0 for IdentityCreateTransition {
    fn validate_advanced_structure_from_state_v0<C>(
        &self,
        platform: &PlatformStateRef,
        action: &IdentityCreateTransitionAction,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let validation_result = IdentityPublicKeyInCreation::validate_identity_public_keys_structure(
            self.public_keys(),
            true,
            platform_version,
        )
        .map_err(Error::Protocol)?;

        if !validation_result.is_valid() {
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
                    // We have used the
                }
                StoredAssetLockInfo::PresentWithInfo(info) => {}
                StoredAssetLockInfo::NotPresent => {}
            }

            let bump_action = StateTransitionAction::PartiallyUseAssetLockAction(
                PartiallyUseAssetLockAction::try_from_borrowed_identity_create_transition(self)?,
            );

            Ok(ConsensusValidationResult::new_with_data_and_errors(
                bump_action,
                validation_result.errors,
            ))
        } else {
            Ok(ConsensusValidationResult::new())
        }
    }
}
