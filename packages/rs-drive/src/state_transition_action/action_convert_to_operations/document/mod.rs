use crate::error::Error;
use crate::util::batch::DriveOperation;
use dpp::block::epoch::Epoch;
use dpp::platform_value::Identifier;
use dpp::version::PlatformVersion;

mod document_create_transition;
mod document_delete_transition;
mod document_purchase_transition;
mod document_replace_transition;
mod document_transfer_transition;
mod document_transition;
mod document_update_price_transition;
mod documents_batch_transition;

/// A converter that will get High Level Drive Operations from State transitions
pub trait DriveHighLevelDocumentOperationConverter {
    /// This will get a list of atomic drive operations from a high level operations
    fn into_high_level_document_drive_operations<'a>(
        self,
        epoch: &Epoch,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'a>>, Error>;
}
