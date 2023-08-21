use crate::drive::batch::transitions::DriveHighLevelOperationConverter;
use crate::drive::batch::DriveOperation::{IdentityOperation, SystemOperation};
use crate::drive::batch::{DriveOperation, IdentityOperationType, SystemOperationType};

use crate::error::Error;
use crate::state_transition_action::identity::identity_topup::IdentityTopUpTransitionAction;
use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;

impl DriveHighLevelOperationConverter for IdentityTopUpTransitionAction {
    fn into_high_level_drive_operations<'a>(
        self,
        _epoch: &Epoch,
        _platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'a>>, Error> {
        let top_up_balance_amount = self.top_up_balance_amount();
        let identity_id = self.identity_id();
        let asset_lock_outpoint = self.asset_lock_outpoint();

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
