use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::document::DriveHighLevelDocumentOperationConverter;
use crate::state_transition_action::document::documents_batch::document_transition::BatchTransitionAction;
use crate::util::batch::DriveOperation;
use dpp::block::epoch::Epoch;
use dpp::prelude::Identifier;
use dpp::version::PlatformVersion;

impl DriveHighLevelDocumentOperationConverter for BatchTransitionAction {
    fn into_high_level_document_drive_operations<'b>(
        self,
        epoch: &Epoch,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'b>>, Error> {
        match self {
            BatchTransitionAction::DocumentAction(document_action) => document_action
                .into_high_level_document_drive_operations(epoch, owner_id, platform_version),
            BatchTransitionAction::TokenAction(token_action) => token_action
                .into_high_level_document_drive_operations(epoch, owner_id, platform_version),
        }
    }
}
