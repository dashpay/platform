use dpp::document::state_transition::documents_batch_transition::DocumentsBatchTransitionAction;
use crate::drive::batch::{DocumentOperationType, DriveOperation};
use crate::drive::batch::DriveOperation::DocumentOperation;
use crate::drive::batch::transitions::DriveHighLevelOperationConverter;
use crate::error::Error;
use crate::fee_pools::epochs::Epoch;

impl DriveHighLevelOperationConverter for DocumentsBatchTransitionAction {
    fn to_high_level_drive_operations(&self, epoch: &Epoch) -> Result<Vec<DriveOperation>, Error> {
        let DocumentsBatchTransitionAction {
            owner_id, transitions, ..
        } = self;
        transitions.iter().map(|transition| {
            transition.to_high_level_document_drive_operations(epoch, *owner_id)
        }).flatten().collect::<Result<Vec<DriveOperation>, Error>>()
    }
}