use crate::drive::batch::transitions::DriveHighLevelOperationConverter;
use crate::drive::batch::DriveOperation::SystemOperation;
use crate::drive::batch::{DriveOperation, SystemOperationType};
use crate::error::Error;
use crate::state_transition_action::system::partially_use_asset_lock_action::{
    PartiallyUseAssetLockAction, PartiallyUseAssetLockActionAccessorsV0,
};
use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;

impl DriveHighLevelOperationConverter for PartiallyUseAssetLockAction {
    fn into_high_level_drive_operations<'b>(
        self,
        _epoch: &Epoch,
        _platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'b>>, Error> {
        let initial_credit_value = self.initial_credit_value();
        let remaining_credit_value = self.remaining_credit_value();
        let used_credits = self.used_credits();
        let asset_lock_outpoint = self.asset_lock_outpoint();

        let drive_operations = vec![
            SystemOperation(SystemOperationType::AddToSystemCredits {
                amount: used_credits,
            }),
            SystemOperation(SystemOperationType::AddPartiallyUsedAssetLock {
                asset_lock_outpoint,
                remaining_credit_value,
                initial_credit_value,
            }),
        ];
        Ok(drive_operations)
    }
}
