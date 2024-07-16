use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::util::batch::DriveOperation::{IdentityOperation, SystemOperation};
use crate::util::batch::{DriveOperation, IdentityOperationType, SystemOperationType};
use dpp::asset_lock::reduced_asset_lock_value::{AssetLockValueGettersV0, AssetLockValueSettersV0};

use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::identity::identity_topup::IdentityTopUpTransitionAction;
use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;

impl DriveHighLevelOperationConverter for IdentityTopUpTransitionAction {
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
            .identity_top_up_transition
        {
            0 => {
                let identity_id = self.identity_id();
                let asset_lock_outpoint = self.asset_lock_outpoint();

                let mut asset_lock_value = self.top_up_asset_lock_value_consume();

                let added_balance = asset_lock_value.remaining_credit_value();

                asset_lock_value.set_remaining_credit_value(0);

                let drive_operations = vec![
                    IdentityOperation(IdentityOperationType::AddToIdentityBalance {
                        identity_id: identity_id.to_buffer(),
                        added_balance,
                    }),
                    SystemOperation(SystemOperationType::AddToSystemCredits {
                        amount: added_balance,
                    }),
                    SystemOperation(SystemOperationType::AddUsedAssetLock {
                        asset_lock_outpoint,
                        asset_lock_value,
                    }),
                ];
                Ok(drive_operations)
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "IdentityTopUpTransitionAction::into_high_level_drive_operations"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
