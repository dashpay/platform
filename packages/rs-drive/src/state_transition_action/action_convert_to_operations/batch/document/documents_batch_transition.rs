use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::batch::DriveHighLevelBatchOperationConverter;
use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::state_transition_action::batch::BatchTransitionAction;
use crate::util::batch::{ContractModerationOperationType, DriveOperation};
use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;

impl DriveHighLevelOperationConverter for BatchTransitionAction {
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
                        transition.into_high_level_batch_drive_operations(
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
            // Protocol version 14: the batch also sweeps the lapsed suspensions the transformer
            // found for its owner, one delete per (contract, identity) after the transitions'
            // own operations, always the owner's own suspension. And before a change of a
            // seated moderation team, it settles the team's moderators pot first: the payouts
            // its state validation settled and the reset of the action counts, operations on
            // other keys than the change's own.
            1 => {
                let mut action = self;
                let owner_id = action.owner_id();
                let lapsed_suspensions = action.lapsed_suspensions().clone();
                let settlements = action.take_moderators_pot_settlements();
                let transitions = action.transitions_owned();
                let mut operations = settlements
                    .into_iter()
                    .map(|settlement| settlement.into_drive_operations())
                    .collect::<Result<Vec<Vec<DriveOperation>>, Error>>()?
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>();
                let transition_operations = transitions
                    .into_iter()
                    .map(|transition| {
                        transition.into_high_level_batch_drive_operations(
                            epoch,
                            owner_id,
                            platform_version,
                        )
                    })
                    .collect::<Result<Vec<Vec<DriveOperation>>, Error>>()?
                    .into_iter()
                    .flatten();
                operations.extend(transition_operations);
                operations.extend(lapsed_suspensions.into_iter().map(|contract_id| {
                    DriveOperation::ContractModerationOperation(
                        ContractModerationOperationType::RemoveSuspension {
                            contract_id,
                            identity_id: owner_id,
                        },
                    )
                }));
                Ok(operations)
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "DocumentsBatchTransitionAction::into_high_level_drive_operations"
                    .to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}
