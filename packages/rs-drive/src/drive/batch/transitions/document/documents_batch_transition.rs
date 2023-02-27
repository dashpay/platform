use dpp::document::DocumentsBatchTransition;
use crate::drive::batch::{DocumentOperationType, DriveOperation};
use crate::drive::batch::DriveOperation::DocumentOperation;
use crate::drive::batch::transitions::DriveHighLevelOperationConverter;
use crate::error::Error;

impl DriveHighLevelOperationConverter for DocumentsBatchTransition {
    fn to_high_level_drive_operations(&self) -> Result<Vec<DriveOperation>, Error> {
        let DocumentsBatchTransition {
            owner_id, transitions, ..
        } = self;
        transitions.iter().map(|transition| {
            transition.to_high_level_drive_operations()
        }).flatten().collect::<Result<Vec<DriveOperation>, Error>>()
    }
}