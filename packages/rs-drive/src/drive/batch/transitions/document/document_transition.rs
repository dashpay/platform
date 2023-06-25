use crate::drive::batch::transitions::document::DriveHighLevelDocumentOperationConverter;
use crate::drive::batch::DriveOperation;
use crate::error::Error;
use dpp::block::epoch::Epoch;
use dpp::document::document_transition::DocumentTransitionAction;
use dpp::prelude::Identifier;
use dpp::state_transition::documents_batch_transition::document_transition::DocumentTransitionAction;

impl DriveHighLevelDocumentOperationConverter for DocumentTransitionAction {
    fn into_high_level_document_drive_operations<'a>(
        self,
        epoch: &Epoch,
        owner_id: Identifier,
    ) -> Result<Vec<DriveOperation<'a>>, Error> {
        match self {
            DocumentTransitionAction::CreateAction(document_create_transition) => {
                document_create_transition
                    .into_high_level_document_drive_operations(epoch, owner_id)
            }
            DocumentTransitionAction::ReplaceAction(document_replace_transition) => {
                document_replace_transition
                    .into_high_level_document_drive_operations(epoch, owner_id)
            }
            DocumentTransitionAction::DeleteAction(document_delete_transition) => {
                document_delete_transition
                    .into_high_level_document_drive_operations(epoch, owner_id)
            }
        }
    }
}
