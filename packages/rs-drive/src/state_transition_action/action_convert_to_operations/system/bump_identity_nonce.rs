use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::state_transition_action::system::bump_identity_nonce_action::{
    BumpIdentityNonceAction, BumpIdentityNonceActionAccessorsV0,
};
use crate::util::batch::DriveOperation::IdentityOperation;
use crate::util::batch::{DriveOperation, IdentityOperationType};
use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;

impl DriveHighLevelOperationConverter for BumpIdentityNonceAction {
    fn into_high_level_drive_operations<'b>(
        self,
        _epoch: &Epoch,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'b>>, Error> {
        match platform_version
            .drive
            .methods
            .state_transitions
            .convert_to_high_level_operations
            .bump_identity_nonce
        {
            0 => {
                let identity_id = self.identity_id();

                let identity_nonce = self.identity_nonce();

                Ok(vec![IdentityOperation(
                    IdentityOperationType::UpdateIdentityNonce {
                        identity_id: identity_id.into_buffer(),
                        nonce: identity_nonce,
                    },
                )])
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "BumpIdentityNonceAction::into_high_level_drive_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_transition_action::system::bump_identity_nonce_action::BumpIdentityNonceActionV0;
    use dpp::block::epoch::Epoch;
    use dpp::platform_value::Identifier;
    use dpp::version::PlatformVersion;

    fn make_action() -> BumpIdentityNonceAction {
        BumpIdentityNonceAction::V0(BumpIdentityNonceActionV0 {
            identity_id: Identifier::from([0xAA; 32]),
            identity_nonce: 42,
            user_fee_increase: 5,
        })
    }

    #[test]
    fn test_into_high_level_drive_operations_returns_update_identity_nonce() {
        let action = make_action();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        assert_eq!(ops.len(), 1);
        match &ops[0] {
            IdentityOperation(IdentityOperationType::UpdateIdentityNonce {
                identity_id,
                nonce,
            }) => {
                assert_eq!(*identity_id, [0xAA; 32]);
                assert_eq!(*nonce, 42);
            }
            other => panic!("expected UpdateIdentityNonce, got {:?}", other),
        }
    }

    #[test]
    fn test_with_zero_nonce() {
        let action = BumpIdentityNonceAction::V0(BumpIdentityNonceActionV0 {
            identity_id: Identifier::from([0x00; 32]),
            identity_nonce: 0,
            user_fee_increase: 0,
        });
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        assert_eq!(ops.len(), 1);
        match &ops[0] {
            IdentityOperation(IdentityOperationType::UpdateIdentityNonce {
                identity_id,
                nonce,
            }) => {
                assert_eq!(*identity_id, [0x00; 32]);
                assert_eq!(*nonce, 0);
            }
            other => panic!("expected UpdateIdentityNonce, got {:?}", other),
        }
    }

    #[test]
    fn test_with_max_nonce() {
        let action = BumpIdentityNonceAction::V0(BumpIdentityNonceActionV0 {
            identity_id: Identifier::from([0xFF; 32]),
            identity_nonce: u64::MAX,
            user_fee_increase: u16::MAX,
        });
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        assert_eq!(ops.len(), 1);
        match &ops[0] {
            IdentityOperation(IdentityOperationType::UpdateIdentityNonce {
                identity_id,
                nonce,
            }) => {
                assert_eq!(*identity_id, [0xFF; 32]);
                assert_eq!(*nonce, u64::MAX);
            }
            other => panic!("expected UpdateIdentityNonce, got {:?}", other),
        }
    }
}
