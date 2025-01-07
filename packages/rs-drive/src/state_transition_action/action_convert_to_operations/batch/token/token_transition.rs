use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::batch::DriveHighLevelBatchOperationConverter;
use crate::state_transition_action::batch::batched_transition::token_transition::TokenTransitionAction;
use crate::util::batch::DriveOperation;
use dpp::block::epoch::Epoch;
use dpp::prelude::Identifier;
use dpp::version::PlatformVersion;

impl DriveHighLevelBatchOperationConverter for TokenTransitionAction {
    fn into_high_level_batch_drive_operations<'b>(
        self,
        epoch: &Epoch,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'b>>, Error> {
        match self {
            TokenTransitionAction::BurnAction(token_burn_transition) => token_burn_transition
                .into_high_level_batch_drive_operations(epoch, owner_id, platform_version),
            TokenTransitionAction::MintAction(token_mint_transition) => token_mint_transition
                .into_high_level_batch_drive_operations(epoch, owner_id, platform_version),
            TokenTransitionAction::TransferAction(token_transfer_transition) => {
                token_transfer_transition.into_high_level_batch_drive_operations(
                    epoch,
                    owner_id,
                    platform_version,
                )
            }
            TokenTransitionAction::FreezeAction(token_freeze_action) => token_freeze_action
                .into_high_level_batch_drive_operations(epoch, owner_id, platform_version),
            TokenTransitionAction::UnfreezeAction(token_unfreeze_action) => token_unfreeze_action
                .into_high_level_batch_drive_operations(epoch, owner_id, platform_version),
            TokenTransitionAction::EmergencyActionAction(token_emergency_action) => {
                token_emergency_action.into_high_level_batch_drive_operations(
                    epoch,
                    owner_id,
                    platform_version,
                )
            }
            TokenTransitionAction::DestroyFrozenFundsAction(token_destroy_frozen_funds) => {
                token_destroy_frozen_funds.into_high_level_batch_drive_operations(
                    epoch,
                    owner_id,
                    platform_version,
                )
            }
        }
    }
}
