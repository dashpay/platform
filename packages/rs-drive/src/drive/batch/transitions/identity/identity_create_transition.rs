use crate::drive::batch::transitions::DriveHighLevelOperationConverter;
use crate::drive::batch::DriveOperation::{IdentityOperation, SystemOperation};
use crate::drive::batch::{DriveOperation, IdentityOperationType, SystemOperationType};

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
        let initial_balance_amount = self.initial_balance_amount();
        let asset_lock_outpoint = self.asset_lock_outpoint();
        let identity =
            Identity::try_from_identity_create_transition_action(self, platform_version)?;

        let drive_operations = vec![
            IdentityOperation(IdentityOperationType::AddNewIdentity {
                identity,
                is_masternode_identity: false,
            }),
            SystemOperation(SystemOperationType::AddToSystemCredits {
                amount: initial_balance_amount,
            }),
            SystemOperation(SystemOperationType::AddUsedAssetLock {
                asset_lock_outpoint,
            }),
        ];
        Ok(drive_operations)
    }
}
