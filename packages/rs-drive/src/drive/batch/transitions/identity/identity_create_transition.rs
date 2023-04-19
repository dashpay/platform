use crate::drive::batch::transitions::DriveHighLevelOperationConverter;
use crate::drive::batch::DriveOperation::{IdentityOperation, SystemOperation};
use crate::drive::batch::{DriveOperation, IdentityOperationType, SystemOperationType};
use crate::drive::defaults::PROTOCOL_VERSION;

use crate::error::Error;
use dpp::block::epoch::Epoch;
use dpp::identity::state_transition::identity_create_transition::IdentityCreateTransitionAction;
use dpp::prelude::Identity;

impl DriveHighLevelOperationConverter for IdentityCreateTransitionAction {
    fn into_high_level_drive_operations<'a>(
        self,
        _epoch: &Epoch,
    ) -> Result<Vec<DriveOperation<'a>>, Error> {
        let IdentityCreateTransitionAction {
            public_keys,
            initial_balance_amount,
            identity_id,
            ..
        } = self;

        let drive_operations = vec![
            IdentityOperation(IdentityOperationType::AddNewIdentity {
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
            }),
            SystemOperation(SystemOperationType::AddToSystemCredits {
                amount: initial_balance_amount,
            }),
        ];
        Ok(drive_operations)
    }
}
