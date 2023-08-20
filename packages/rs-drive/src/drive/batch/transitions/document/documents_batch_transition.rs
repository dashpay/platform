use crate::drive::batch::transitions::document::DriveHighLevelDocumentOperationConverter;
use crate::drive::batch::transitions::DriveHighLevelOperationConverter;
use crate::drive::batch::DriveOperation;
use crate::error::Error;
use crate::state_transition_action::document::documents_batch::DocumentsBatchTransitionAction;
use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;

impl DriveHighLevelOperationConverter for DocumentsBatchTransitionAction {
    fn into_high_level_drive_operations<'b>(
        self,
        epoch: &Epoch,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'b>>, Error> {
        let owner_id = self.owner_id();
        let transitions = self.transitions_owned();
        Ok(transitions
            .into_iter()
            .map(|transition| {
                transition.into_high_level_document_drive_operations(
                    epoch,
                    owner_id,
                    platform_version,
                )
            })
            .collect::<Result<Vec<Vec<DriveOperation>>, Error>>()?
            .into_iter()
            .flatten()
            .collect())
    }
}
