use crate::drive::batch::transitions::DriveHighLevelOperationConverter;
use crate::drive::batch::DriveOperation::IdentityOperation;
use crate::drive::batch::{DriveOperation, IdentityOperationType};

use crate::error::Error;
use crate::state_transition_action::identity::identity_credit_transfer::IdentityCreditTransferTransitionAction;
use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;

impl DriveHighLevelOperationConverter for IdentityCreditTransferTransitionAction {
    fn into_high_level_drive_operations<'a>(
        self,
        _epoch: &Epoch,
        _platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'a>>, Error> {
        let recipient_id = self.recipient_id();
        let identity_id = self.identity_id();
        let transfer_amount = self.transfer_amount();
        let revision = self.revision();

        let drive_operations = vec![
            IdentityOperation(IdentityOperationType::UpdateIdentityRevision {
                identity_id: identity_id.into_buffer(),
                revision,
            }),
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
