use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::document::DriveHighLevelDocumentOperationConverter;
use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::state_transition_action::document::documents_batch::DocumentsBatchTransitionAction;
use crate::util::batch::DriveOperation;
use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;

impl DriveHighLevelOperationConverter for DocumentsBatchTransitionAction {
    fn into_high_level_drive_operations<'b>(
        self,
        epoch: &Epoch,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'b>>, Error> {
        match platform_version
            .drive
            .methods
            .state_transitions
            .convert_to_high_level_operations
            .documents_batch_transition
        {
            0 => {
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
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "DocumentsBatchTransitionAction::into_high_level_drive_operations"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
