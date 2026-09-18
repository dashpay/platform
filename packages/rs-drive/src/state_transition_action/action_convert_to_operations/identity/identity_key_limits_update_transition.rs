use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::util::batch::DriveOperation::IdentityOperation;
use crate::util::batch::{DriveOperation, IdentityOperationType};

use crate::error::Error;
use dpp::block::epoch::Epoch;

use crate::error::drive::DriveError;
use crate::state_transition_action::identity::identity_key_limits_update::v0::IdentityKeyLimitsUpdateTransitionActionV0;
use crate::state_transition_action::identity::identity_key_limits_update::IdentityKeyLimitsUpdateTransitionAction;
use dpp::version::PlatformVersion;

impl DriveHighLevelOperationConverter for IdentityKeyLimitsUpdateTransitionAction {
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
            .identity_key_limits_update_transition
        {
            0 => {
                let IdentityKeyLimitsUpdateTransitionAction::V0(
                    IdentityKeyLimitsUpdateTransitionActionV0 {
                        identity_id,
                        revision,
                        nonce,
                        stored_key,
                        total_budget,
                        expires_at,
                        ..
                    },
                ) = self;
                let identity_id = identity_id.to_buffer();

                Ok(vec![
                    IdentityOperation(IdentityOperationType::UpdateIdentityRevision {
                        identity_id,
                        revision,
                    }),
                    IdentityOperation(IdentityOperationType::UpdateIdentityNonce {
                        identity_id,
                        nonce,
                    }),
                    IdentityOperation(IdentityOperationType::UpdateIdentityKeyLimits {
                        identity_id,
                        key: stored_key,
                        total_budget,
                        expires_at,
                    }),
                ])
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "IdentityKeyLimitsUpdateTransitionAction::into_high_level_drive_operations"
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
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::identity::IdentityPublicKey;
    use dpp::platform_value::Identifier;

    #[test]
    fn should_produce_the_revision_nonce_and_key_limits_operations_in_order() {
        let platform_version = PlatformVersion::latest();
        let stored_key =
            IdentityPublicKey::random_authentication_keys(3, 1, Some(15), platform_version)
                .remove(0)
                .with_limits(Some(500), None);
        let action = IdentityKeyLimitsUpdateTransitionAction::V0(
            IdentityKeyLimitsUpdateTransitionActionV0 {
                identity_id: Identifier::from([0xAA; 32]),
                revision: 5,
                nonce: 10,
                key_id: 3,
                stored_key,
                total_budget: Some(700),
                expires_at: None,
                user_fee_increase: 0,
            },
        );
        let epoch = Epoch::new(0).expect("epoch");

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        assert_eq!(ops.len(), 3);
        assert!(matches!(
            &ops[0],
            IdentityOperation(IdentityOperationType::UpdateIdentityRevision { identity_id, revision: 5 })
                if *identity_id == [0xAA; 32]
        ));
        assert!(matches!(
            &ops[1],
            IdentityOperation(IdentityOperationType::UpdateIdentityNonce { identity_id, nonce: 10 })
                if *identity_id == [0xAA; 32]
        ));
        assert!(matches!(
            &ops[2],
            IdentityOperation(IdentityOperationType::UpdateIdentityKeyLimits {
                identity_id,
                key,
                total_budget: Some(700),
                expires_at: None,
            }) if *identity_id == [0xAA; 32] && key.id() == 3
        ));
    }
}
