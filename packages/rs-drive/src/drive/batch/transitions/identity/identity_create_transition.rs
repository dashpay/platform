use dpp::identity::state_transition::identity_create_transition::IdentityCreateTransition;
use dpp::prelude::Identity;
use crate::drive::batch::{DocumentOperationType, DriveOperation, IdentityOperationType};
use crate::drive::batch::DriveOperation::{DocumentOperation, IdentityOperation};
use crate::drive::batch::transitions::DriveHighLevelOperationConverter;
use crate::drive::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::error::Error;

impl DriveHighLevelOperationConverter for IdentityCreateTransition {
    fn to_high_level_drive_operations(&self) -> Result<Vec<DriveOperation>, Error> {
        let IdentityCreateTransition {
            public_keys, asset_lock_proof, identity_id, protocol_version, transition_type, signature, execution_context
        } = self;

        let mut drive_operations = vec![];
        /// We must create the contract
        drive_operations.push(IdentityOperation(IdentityOperationType::AddNewIdentity { identity: Default::default() });

        Ok(drive_operations)
    }
}