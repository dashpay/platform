use crate::drive::batch::transitions::DriveHighLevelOperationConverter;
use crate::drive::batch::DriveOperation::{IdentityOperation, SystemOperation};
use crate::drive::batch::{DriveOperation, IdentityOperationType, SystemOperationType};
use dpp::asset_lock::reduced_asset_lock_value::{AssetLockValueGettersV0, AssetLockValueSettersV0};

use crate::error::Error;
use crate::state_transition_action::identity::identity_create::{
    IdentityCreateTransitionAction, IdentityFromIdentityCreateTransitionAction,
};
use dpp::block::epoch::Epoch;
use dpp::prelude::Identity;
use dpp::version::PlatformVersion;

impl DriveHighLevelOperationConverter for IdentityCreateTransitionAction {
    fn into_high_level_drive_operations<'a>(
        self,
        _epoch: &Epoch,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'a>>, Error> {
        let asset_lock_outpoint = self.asset_lock_outpoint();

        let (identity, mut asset_lock_value) =
            Identity::try_from_identity_create_transition_action_returning_asset_lock_value(
                self,
                platform_version,
            )?;

        let initial_balance = asset_lock_value.remaining_credit_value();

        asset_lock_value.set_remaining_credit_value(0); // We are using the entire value

        let drive_operations = vec![
            IdentityOperation(IdentityOperationType::AddNewIdentity {
                identity,
                is_masternode_identity: false,
            }),
            SystemOperation(SystemOperationType::AddToSystemCredits {
                amount: initial_balance,
            }),
            SystemOperation(SystemOperationType::AddUsedAssetLock {
                asset_lock_outpoint,
                asset_lock_value,
            }),
        ];
        Ok(drive_operations)
    }
}
