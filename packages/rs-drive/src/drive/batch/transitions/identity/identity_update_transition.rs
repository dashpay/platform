use dpp::identity::state_transition::identity_create_transition::IdentityCreateTransition;
use dpp::identity::state_transition::identity_topup_transition::IdentityTopUpTransition;
use dpp::identity::state_transition::identity_update_transition::IdentityUpdateTransitionAction;
use dpp::prelude::Identity;
use crate::drive::batch::{DocumentOperationType, DriveOperation, IdentityOperationType};
use crate::drive::batch::DriveOperation::{DocumentOperation, IdentityOperation};
use crate::drive::batch::transitions::DriveHighLevelOperationConverter;
use crate::drive::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::error::Error;
use crate::fee_pools::epochs::Epoch;

impl DriveHighLevelOperationConverter for IdentityUpdateTransitionAction {
    fn to_high_level_drive_operations(self, epoch: &Epoch) -> Result<Vec<DriveOperation>, Error> {
        let IdentityUpdateTransitionAction {
            add_public_keys, disable_public_keys, public_keys_disabled_at, identity_id, ..
        } = self;


        let mut drive_operations = vec![];
        if !add_public_keys.is_empty() {
            drive_operations.push(IdentityOperation(IdentityOperationType::AddNewKeysToIdentity { identity_id: identity_id.to_buffer(), keys_to_add: add_public_keys}));
        }
        if let Some(public_keys_disabled_at) = public_keys_disabled_at {
            if !disable_public_keys.is_empty() {
                drive_operations.push(IdentityOperation(IdentityOperationType::DisableIdentityKeys { identity_id: identity_id.to_buffer(), keys_ids: disable_public_keys, disable_at: public_keys_disabled_at }));
            }
        }

        Ok(drive_operations)
    }
}