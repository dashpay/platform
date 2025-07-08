use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::batch::DriveHighLevelBatchOperationConverter;
use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::state_transition_action::batch::batched_transition::BatchedTransitionAction;
use crate::util::batch::DriveOperation;
use dpp::block::epoch::Epoch;
use dpp::prelude::Identifier;
use dpp::version::PlatformVersion;

impl DriveHighLevelBatchOperationConverter for BatchedTransitionAction {
    fn into_high_level_batch_drive_operations<'b>(
        self,
        epoch: &Epoch,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'b>>, Error> {
        match self {
            BatchedTransitionAction::DocumentAction(document_action) => document_action
                .into_high_level_batch_drive_operations(epoch, owner_id, platform_version),
            BatchedTransitionAction::TokenAction(token_action) => token_action
                .into_high_level_batch_drive_operations(epoch, owner_id, platform_version),
            BatchedTransitionAction::BumpIdentityDataContractNonce(
                bump_identity_contract_nonce_action,
            ) => bump_identity_contract_nonce_action
                .into_high_level_drive_operations(epoch, platform_version),
        }
    }
}
