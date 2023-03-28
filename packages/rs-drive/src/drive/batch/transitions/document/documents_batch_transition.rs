use crate::drive::batch::transitions::document::DriveHighLevelDocumentOperationConverter;
use crate::drive::batch::transitions::DriveHighLevelOperationConverter;
use crate::drive::batch::DriveOperation;
use crate::error::Error;
use crate::fee_pools::epochs::Epoch;
use dpp::document::state_transition::documents_batch_transition::DocumentsBatchTransitionAction;

impl DriveHighLevelOperationConverter for DocumentsBatchTransitionAction {
    fn into_high_level_drive_operations(self, epoch: &Epoch) -> Result<Vec<DriveOperation>, Error> {
        let DocumentsBatchTransitionAction {
            owner_id,
            transitions,
            ..
        } = self;
        Ok(transitions
            .into_iter()
            .map(|transition| {
                transition.into_high_level_document_drive_operations(epoch, owner_id)
            })
            .collect::<Result<Vec<Vec<DriveOperation>>, Error>>()?.into_iter().flatten().collect())
    }
}
