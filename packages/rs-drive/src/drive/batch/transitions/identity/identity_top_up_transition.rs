use crate::drive::batch::transitions::DriveHighLevelOperationConverter;
use crate::drive::batch::DriveOperation::{IdentityOperation, SystemOperation};
use crate::drive::batch::{DriveOperation, IdentityOperationType, SystemOperationType};

use crate::error::Error;
use dpp::block::epoch::Epoch;
use dpp::identity::state_transition::identity_topup_transition::IdentityTopUpTransitionAction;

impl DriveHighLevelOperationConverter for IdentityTopUpTransitionAction {
    fn into_high_level_drive_operations<'a>(
        self,
        _epoch: &Epoch,
    ) -> Result<Vec<DriveOperation<'a>>, Error> {
        let IdentityTopUpTransitionAction {
            top_up_balance_amount,
            identity_id,
            asset_lock_outpoint,
            ..
        } = self;

        let drive_operations = vec![
            IdentityOperation(IdentityOperationType::AddToIdentityBalance {
                identity_id: identity_id.to_buffer(),
                added_balance: top_up_balance_amount,
            }),
            SystemOperation(SystemOperationType::AddToSystemCredits {
                amount: top_up_balance_amount,
            }),
            SystemOperation(SystemOperationType::AddUsedAssetLock {
                asset_lock_outpoint,
            }),
        ];
        Ok(drive_operations)
    }
}
