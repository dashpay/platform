use crate::drive::batch::transitions::DriveHighLevelOperationConverter;
use crate::drive::batch::DriveOperation::{IdentityOperation, SystemOperation};
use crate::drive::batch::{DriveOperation, IdentityOperationType, SystemOperationType};

use crate::error::Error;
use crate::fee_pools::epochs::Epoch;
use dpp::identity::state_transition::identity_topup_transition::IdentityTopUpTransitionAction;

impl DriveHighLevelOperationConverter for IdentityTopUpTransitionAction {
    fn into_high_level_drive_operations(
        self,
        _epoch: &Epoch,
    ) -> Result<Vec<DriveOperation>, Error> {
        let IdentityTopUpTransitionAction {
            top_up_balance_amount,
            identity_id,
            ..
        } = self;

        let mut drive_operations = vec![
            IdentityOperation(IdentityOperationType::AddToIdentityBalance {
                identity_id: identity_id.to_buffer(),
                added_balance: top_up_balance_amount,
            }),
            SystemOperation(SystemOperationType::AddToSystemCredits {
                amount: top_up_balance_amount,
            }),
        ];
        Ok(drive_operations)
    }
}
