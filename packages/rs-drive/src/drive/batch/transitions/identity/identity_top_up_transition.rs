use dpp::identity::state_transition::identity_create_transition::IdentityCreateTransition;
use dpp::identity::state_transition::identity_topup_transition::IdentityTopUpTransition;
use dpp::prelude::Identity;
use crate::drive::batch::{DocumentOperationType, DriveOperation, IdentityOperationType};
use crate::drive::batch::DriveOperation::{DocumentOperation, IdentityOperation};
use crate::drive::batch::transitions::DriveHighLevelOperationConverter;
use crate::drive::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::error::Error;

impl DriveHighLevelOperationConverter for IdentityTopUpTransition {
    fn to_high_level_drive_operations(&self) -> Result<Vec<DriveOperation>, Error> {
        let IdentityTopUpTransition {
            asset_lock_proof, identity_id, protocol_version, transition_type, signature, execution_context
        } = self;


        let mut drive_operations = vec![];
        /// We must create the contract
        drive_operations.push(IdentityOperation(IdentityOperationType::AddToIdentityBalance { identity_id: identity_id.to_buffer(), added_balance: 0 });

        Ok(drive_operations)
    }
}