use crate::drive::batch::transitions::DriveHighLevelOperationConverter;
use crate::drive::batch::DriveOperation::SystemOperation;
use crate::drive::batch::{DriveOperation, SystemOperationType};
use crate::error::Error;
use crate::state_transition_action::system::partially_use_asset_lock_action::{
    PartiallyUseAssetLockAction, PartiallyUseAssetLockActionAccessorsV0,
};
use dpp::asset_lock::reduced_asset_lock_value::AssetLockValue;
use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;

impl DriveHighLevelOperationConverter for PartiallyUseAssetLockAction {
    fn into_high_level_drive_operations<'b>(
        self,
        _epoch: &Epoch,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'b>>, Error> {
        let initial_credit_value = self.initial_credit_value();
        // The remaining credit value is already computed here
        let remaining_credit_value = self.remaining_credit_value();
        let used_credits = self.used_credits();
        let asset_lock_outpoint = self.asset_lock_outpoint();

        let tx_out_script = self.asset_lock_script_owned();

        let drive_operations = vec![
            SystemOperation(SystemOperationType::AddToSystemCredits {
                amount: used_credits,
            }),
            SystemOperation(SystemOperationType::AddUsedAssetLock {
                asset_lock_outpoint,
                asset_lock_value: AssetLockValue::new(
                    initial_credit_value,
                    tx_out_script,
                    remaining_credit_value,
                    platform_version,
                )?,
            }),
        ];
        Ok(drive_operations)
    }
}
