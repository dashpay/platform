use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::state_transition_action::system::partially_use_asset_lock_action::{
    PartiallyUseAssetLockAction, PartiallyUseAssetLockActionAccessorsV0,
};
use crate::util::batch::DriveOperation::SystemOperation;
use crate::util::batch::{DriveOperation, SystemOperationType};
use dpp::asset_lock::reduced_asset_lock_value::AssetLockValue;
use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;

impl DriveHighLevelOperationConverter for PartiallyUseAssetLockAction {
    fn into_high_level_drive_operations<'a>(
        self,
        _epoch: &Epoch,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'a>>, Error> {
        match platform_version
            .drive
            .methods
            .state_transitions
            .convert_to_high_level_operations
            .partially_use_asset_lock
        {
            0 => {
                let initial_credit_value = self.initial_credit_value();
                // The remaining credit value is already computed here
                let mut remaining_credit_value = self.remaining_credit_value();
                let used_credits = self.used_credits();
                let asset_lock_outpoint = self.asset_lock_outpoint();

                let previous_transaction_hashes = if self.previous_transaction_hashes_ref().len()
                    as u16
                    >= platform_version
                        .drive_abci
                        .validation_and_processing
                        .state_transitions
                        .max_asset_lock_usage_attempts
                {
                    // There have been 16 failed attempts at using the asset lock
                    // In this case the remaining credit value is burned and there is no need to keep around previous
                    // transaction hashes
                    remaining_credit_value = 0;
                    vec![]
                } else {
                    self.previous_transaction_hashes_ref().clone()
                };

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
                            previous_transaction_hashes,
                            platform_version,
                        )?,
                    }),
                ];
                Ok(drive_operations)
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "PartiallyUseAssetLockAction::into_high_level_drive_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
