use dpp::platform_value::Identifier;
use crate::drive::batch::DriveOperation;
use crate::error::Error;
use crate::fee_pools::epochs::Epoch;

mod documents_batch_transition;
mod document_transition;
mod document_create_transition;
mod document_update_transition;
mod document_delete_transition;

/// A converter that will get High Level Drive Operations from State transitions
pub trait DriveHighLevelDocumentOperationConverter {
    /// This will get a list of atomic drive operations from a high level operations
    fn to_high_level_document_drive_operations(&self, epoch: &Epoch, owner_id: Identifier) -> Result<Vec<DriveOperation>, Error>;
}
