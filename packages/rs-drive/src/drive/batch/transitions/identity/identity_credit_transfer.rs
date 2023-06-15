use crate::drive::batch::transitions::DriveHighLevelOperationConverter;
use crate::drive::batch::DriveOperation::IdentityOperation;
use crate::drive::batch::{DriveOperation, IdentityOperationType};

use crate::error::Error;
use dpp::block::epoch::Epoch;
use dpp::identity::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransitionAction;

impl DriveHighLevelOperationConverter for IdentityCreditTransferTransitionAction {
    fn into_high_level_drive_operations<'a>(
        self,
        _epoch: &Epoch,
    ) -> Result<Vec<DriveOperation<'a>>, Error> {
        let IdentityCreditTransferTransitionAction {
            recipient_id,
            identity_id,
            transfer_amount,
            ..
        } = self;

        let drive_operations = vec![
            IdentityOperation(IdentityOperationType::RemoveFromIdentityBalance {
                identity_id: identity_id.to_buffer(),
                balance_to_remove: transfer_amount,
            }),
            IdentityOperation(IdentityOperationType::AddToIdentityBalance {
                identity_id: recipient_id.to_buffer(),
                added_balance: transfer_amount,
            }),
        ];
        Ok(drive_operations)
    }
}
