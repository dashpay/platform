use dpp::identity::state_transition::identity_topup_transition::IdentityTopUpTransitionAction;
use dpp::prelude::Identity;
use crate::drive::batch::{DocumentOperationType, DriveOperation, IdentityOperationType, SystemOperationType};
use crate::drive::batch::DriveOperation::{DocumentOperation, IdentityOperation, SystemOperation};
use crate::drive::batch::transitions::DriveHighLevelOperationConverter;
use crate::drive::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::error::Error;

impl DriveHighLevelOperationConverter for IdentityTopUpTransitionAction {
    fn to_high_level_drive_operations(self) -> Result<Vec<DriveOperation>, Error> {
        let IdentityTopUpTransitionAction {
            top_up_balance_amount, identity_id, ..
        } = self;


        let mut drive_operations = vec![];

        drive_operations.push(IdentityOperation(IdentityOperationType::AddToIdentityBalance { identity_id: identity_id.to_buffer(), added_balance: top_up_balance_amount }));
        drive_operations.push(SystemOperation(SystemOperationType::AddToSystemCredits { amount: top_up_balance_amount }));
        Ok(drive_operations)
    }
}