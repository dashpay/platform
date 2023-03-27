use crate::drive::batch::transitions::DriveHighLevelOperationConverter;
use crate::drive::batch::DriveOperation::{DocumentOperation, IdentityOperation, SystemOperation};
use crate::drive::batch::{
    DocumentOperationType, DriveOperation, IdentityOperationType, SystemOperationType,
};
use crate::drive::defaults::PROTOCOL_VERSION;
use crate::drive::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::error::Error;
use crate::fee_pools::epochs::Epoch;
use dpp::identity::state_transition::identity_create_transition::{
    IdentityCreateTransition, IdentityCreateTransitionAction,
};
use dpp::prelude::Identity;

impl DriveHighLevelOperationConverter for IdentityCreateTransitionAction {
    fn into_high_level_drive_operations(self, epoch: &Epoch) -> Result<Vec<DriveOperation>, Error> {
        let IdentityCreateTransitionAction {
            public_keys,
            initial_balance_amount,
            identity_id,
            ..
        } = self;

        let mut drive_operations = vec![];
        /// We must create the contract
        drive_operations.push(IdentityOperation(IdentityOperationType::AddNewIdentity {
            identity: Identity {
                //todo: deal with protocol version
                protocol_version: PROTOCOL_VERSION,
                id: identity_id,
                public_keys: public_keys.into_iter().map(|key| (key.id, key)).collect(),
                balance: initial_balance_amount,
                revision: 0,
                asset_lock_proof: None,
                metadata: None,
            },
        }));
        drive_operations.push(SystemOperation(SystemOperationType::AddToSystemCredits {
            amount: initial_balance_amount,
        }));

        Ok(drive_operations)
    }
}
