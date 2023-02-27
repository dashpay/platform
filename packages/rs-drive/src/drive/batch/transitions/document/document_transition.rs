use dpp::prelude::DocumentTransition;
use crate::drive::batch::DriveOperation;
use crate::drive::batch::transitions::DriveHighLevelOperationConverter;
use crate::error::Error;

impl DriveHighLevelOperationConverter for DocumentTransition {
    fn to_high_level_drive_operations(&self) -> Result<Vec<DriveOperation>, Error> {
        match self {
            DocumentTransition::Create(document_create_transition) => document_create_transition.to_high_level_drive_operations(),
            DocumentTransition::Replace(document_replace_transition) => document_replace_transition.to_high_level_drive_operations(),
            DocumentTransition::Delete(document_delete_transition) => document_delete_transition.to_high_level_drive_operations(),
        }
    }
}