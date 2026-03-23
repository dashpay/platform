use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::util::batch::DriveOperation::{IdentityOperation, SystemOperation};
use crate::util::batch::{DriveOperation, IdentityOperationType, SystemOperationType};
use dpp::asset_lock::reduced_asset_lock_value::{AssetLockValueGettersV0, AssetLockValueSettersV0};

use crate::error::drive::DriveError;
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
        match platform_version
            .drive
            .methods
            .state_transitions
            .convert_to_high_level_operations
            .identity_create_transition
        {
            0 => {
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
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "IdentityCreateTransitionAction::into_high_level_drive_operations"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_transition_action::identity::identity_create::v0::IdentityCreateTransitionActionV0;
    use dpp::asset_lock::reduced_asset_lock_value::{AssetLockValue, AssetLockValueGettersV0};
    use dpp::block::epoch::Epoch;
    use dpp::identity::IdentityPublicKey;
    use dpp::platform_value::{Bytes32, Bytes36, Identifier};
    use dpp::state_transition::signable_bytes_hasher::SignableBytesHasher;
    use dpp::version::PlatformVersion;

    fn make_asset_lock_value() -> AssetLockValue {
        let platform_version = PlatformVersion::latest();
        AssetLockValue::new(1000, vec![1, 2, 3], 500, vec![], platform_version)
            .expect("expected asset lock value")
    }

    fn make_action() -> IdentityCreateTransitionAction {
        let platform_version = PlatformVersion::latest();
        let (key, _private) =
            IdentityPublicKey::random_masternode_transfer_key(1, Some(42), platform_version)
                .expect("expected a random key");
        IdentityCreateTransitionAction::V0(IdentityCreateTransitionActionV0 {
            signable_bytes_hasher: SignableBytesHasher::PreHashed(Bytes32([0xCC; 32])),
            public_keys: vec![key],
            asset_lock_value_to_be_consumed: make_asset_lock_value(),
            identity_id: Identifier::from([0xAA; 32]),
            asset_lock_outpoint: Bytes36([0xDD; 36]),
            user_fee_increase: 0,
        })
    }

    #[test]
    fn test_produces_three_operations() {
        let action = make_action();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        // AddNewIdentity + AddToSystemCredits + AddUsedAssetLock
        assert_eq!(ops.len(), 3);
    }

    #[test]
    fn test_first_op_is_add_new_identity() {
        let action = make_action();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        match &ops[0] {
            IdentityOperation(IdentityOperationType::AddNewIdentity {
                identity,
                is_masternode_identity,
            }) => {
                assert!(!is_masternode_identity);
                // The identity should have balance equal to remaining_credit_value (500)
                use dpp::identity::accessors::IdentityGettersV0;
                assert_eq!(identity.balance(), 500);
                assert_eq!(identity.id(), Identifier::from([0xAA; 32]));
            }
            other => panic!("expected AddNewIdentity, got {:?}", other),
        }
    }

    #[test]
    fn test_second_op_is_add_to_system_credits() {
        let action = make_action();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        match &ops[1] {
            SystemOperation(SystemOperationType::AddToSystemCredits { amount }) => {
                assert_eq!(*amount, 500); // remaining_credit_value
            }
            other => panic!("expected AddToSystemCredits, got {:?}", other),
        }
    }

    #[test]
    fn test_third_op_is_add_used_asset_lock_with_zero_remaining() {
        let action = make_action();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        match &ops[2] {
            SystemOperation(SystemOperationType::AddUsedAssetLock {
                asset_lock_outpoint,
                asset_lock_value,
            }) => {
                assert_eq!(*asset_lock_outpoint, Bytes36([0xDD; 36]));
                // remaining_credit_value should have been set to 0
                assert_eq!(asset_lock_value.remaining_credit_value(), 0);
            }
            other => panic!("expected AddUsedAssetLock, got {:?}", other),
        }
    }
}
