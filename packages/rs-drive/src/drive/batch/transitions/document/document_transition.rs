use dpp::document::document_transition::DocumentTransitionAction;
use crate::drive::batch::DriveOperation;
use crate::drive::batch::transitions::DriveHighLevelOperationConverter;
use crate::error::Error;

impl DriveHighLevelOperationConverter for DocumentTransitionAction {
    fn to_high_level_drive_operations(&self) -> Result<Vec<DriveOperation>, Error> {
        match self {
            DocumentTransitionAction::CreateAction(document_create_transition) => document_create_transition.to_high_level_drive_operations(),
            DocumentTransitionAction::ReplaceAction(document_replace_transition) => document_replace_transition.to_high_level_drive_operations(),
            DocumentTransitionAction::DeleteAction(document_delete_transition) => document_delete_transition.to_high_level_drive_operations(),
        }
    }
}