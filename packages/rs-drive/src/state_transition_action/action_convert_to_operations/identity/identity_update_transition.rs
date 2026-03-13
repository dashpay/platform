use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::util::batch::DriveOperation::IdentityOperation;
use crate::util::batch::{DriveOperation, IdentityOperationType};

use crate::error::Error;
use dpp::block::epoch::Epoch;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::IdentityPublicKey;

use crate::error::drive::DriveError;
use crate::state_transition_action::identity::identity_update::IdentityUpdateTransitionAction;
use dpp::version::PlatformVersion;

impl DriveHighLevelOperationConverter for IdentityUpdateTransitionAction {
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
            .identity_update_transition
        {
            0 => {
                let identity_id = self.identity_id();
                let revision = self.revision();
                let (add_public_keys, disable_public_keys) =
                    self.public_keys_to_add_and_disable_owned();

                let (unique_keys, non_unique_keys): (
                    Vec<IdentityPublicKey>,
                    Vec<IdentityPublicKey>,
                ) = add_public_keys
                    .into_iter()
                    .partition(|key| key.key_type().is_unique_key_type());

                let mut drive_operations = vec![];

                drive_operations.push(IdentityOperation(
                    IdentityOperationType::UpdateIdentityRevision {
                        identity_id: identity_id.to_buffer(),
                        revision,
                    },
                ));

                if !unique_keys.is_empty() || !non_unique_keys.is_empty() {
                    drive_operations.push(IdentityOperation(
                        IdentityOperationType::AddNewKeysToIdentity {
                            identity_id: identity_id.to_buffer(),
                            unique_keys_to_add: unique_keys,
                            non_unique_keys_to_add: non_unique_keys,
                        },
                    ));
                }
                if !disable_public_keys.is_empty() {
                    drive_operations.push(IdentityOperation(
                        IdentityOperationType::DisableIdentityKeys {
                            identity_id: identity_id.to_buffer(),
                            keys_ids: disable_public_keys,
                        },
                    ));
                }

                Ok(drive_operations)
            }
            1 => {
                let identity_id = self.identity_id();
                let revision = self.revision();
                let nonce = self.nonce();
                let (add_public_keys, disable_public_keys) =
                    self.public_keys_to_add_and_disable_owned();

                let (unique_keys, non_unique_keys): (
                    Vec<IdentityPublicKey>,
                    Vec<IdentityPublicKey>,
                ) = add_public_keys
                    .into_iter()
                    .partition(|key| key.key_type().is_unique_key_type());

                let mut drive_operations = vec![];

                drive_operations.push(IdentityOperation(
                    IdentityOperationType::UpdateIdentityRevision {
                        identity_id: identity_id.to_buffer(),
                        revision,
                    },
                ));

                drive_operations.push(IdentityOperation(
                    IdentityOperationType::UpdateIdentityNonce {
                        identity_id: identity_id.to_buffer(),
                        nonce,
                    },
                ));

                if !unique_keys.is_empty() || !non_unique_keys.is_empty() {
                    drive_operations.push(IdentityOperation(
                        IdentityOperationType::AddNewKeysToIdentity {
                            identity_id: identity_id.to_buffer(),
                            unique_keys_to_add: unique_keys,
                            non_unique_keys_to_add: non_unique_keys,
                        },
                    ));
                }
                if !disable_public_keys.is_empty() {
                    drive_operations.push(IdentityOperation(
                        IdentityOperationType::DisableIdentityKeys {
                            identity_id: identity_id.to_buffer(),
                            keys_ids: disable_public_keys,
                        },
                    ));
                }

                Ok(drive_operations)
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "IdentityUpdateTransitionAction::into_high_level_drive_operations"
                    .to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_transition_action::identity::identity_update::v0::IdentityUpdateTransitionActionV0;
    use dpp::block::epoch::Epoch;
    use dpp::platform_value::Identifier;
    use dpp::version::PlatformVersion;

    fn make_action_no_keys() -> IdentityUpdateTransitionAction {
        IdentityUpdateTransitionAction::V0(IdentityUpdateTransitionActionV0 {
            add_public_keys: vec![],
            disable_public_keys: vec![],
            identity_id: Identifier::from([0xAA; 32]),
            revision: 5,
            nonce: 10,
            user_fee_increase: 0,
        })
    }

    fn make_action_with_disable_keys() -> IdentityUpdateTransitionAction {
        IdentityUpdateTransitionAction::V0(IdentityUpdateTransitionActionV0 {
            add_public_keys: vec![],
            disable_public_keys: vec![1, 3, 5],
            identity_id: Identifier::from([0xAA; 32]),
            revision: 7,
            nonce: 20,
            user_fee_increase: 0,
        })
    }

    fn make_action_with_add_keys() -> IdentityUpdateTransitionAction {
        let platform_version = PlatformVersion::latest();
        let (key, _) =
            IdentityPublicKey::random_masternode_transfer_key(1, Some(42), platform_version)
                .expect("expected a random key");
        IdentityUpdateTransitionAction::V0(IdentityUpdateTransitionActionV0 {
            add_public_keys: vec![key],
            disable_public_keys: vec![],
            identity_id: Identifier::from([0xAA; 32]),
            revision: 3,
            nonce: 15,
            user_fee_increase: 0,
        })
    }

    #[test]
    fn test_v1_no_keys_produces_revision_and_nonce() {
        let action = make_action_no_keys();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        // Version 1 path: UpdateIdentityRevision + UpdateIdentityNonce
        // (version 0 would omit the nonce update)
        // We check based on what the latest version actually does
        assert_eq!(ops.len(), 2);

        match &ops[0] {
            IdentityOperation(IdentityOperationType::UpdateIdentityRevision {
                identity_id,
                revision,
            }) => {
                assert_eq!(*identity_id, [0xAA; 32]);
                assert_eq!(*revision, 5);
            }
            other => panic!("expected UpdateIdentityRevision, got {:?}", other),
        }

        // Verify the nonce update operation carries the correct identity and nonce
        match &ops[1] {
            IdentityOperation(IdentityOperationType::UpdateIdentityNonce {
                identity_id,
                nonce,
            }) => {
                assert_eq!(*identity_id, [0xAA; 32]);
                assert_eq!(*nonce, 10);
            }
            other => panic!("expected UpdateIdentityNonce, got {:?}", other),
        }
    }

    #[test]
    fn test_with_disable_keys_produces_disable_operation() {
        let action = make_action_with_disable_keys();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        // Should have a DisableIdentityKeys operation
        let has_disable = ops.iter().any(|op| {
            matches!(
                op,
                IdentityOperation(IdentityOperationType::DisableIdentityKeys { .. })
            )
        });
        assert!(has_disable, "expected DisableIdentityKeys operation");

        // Check the disable keys content
        for op in &ops {
            if let IdentityOperation(IdentityOperationType::DisableIdentityKeys {
                identity_id,
                keys_ids,
            }) = op
            {
                assert_eq!(*identity_id, [0xAA; 32]);
                assert_eq!(*keys_ids, vec![1u32, 3, 5]);
            }
        }
    }

    #[test]
    fn test_with_add_keys_produces_add_keys_operation() {
        let action = make_action_with_add_keys();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        // Should have an AddNewKeysToIdentity operation with correct payload
        let add_keys_op = ops
            .iter()
            .find(|op| {
                matches!(
                    op,
                    IdentityOperation(IdentityOperationType::AddNewKeysToIdentity { .. })
                )
            })
            .expect("expected AddNewKeysToIdentity operation");

        match add_keys_op {
            IdentityOperation(IdentityOperationType::AddNewKeysToIdentity {
                identity_id,
                unique_keys_to_add,
                non_unique_keys_to_add,
            }) => {
                assert_eq!(*identity_id, [0xAA; 32]);
                // ECDSA_HASH160 is not a unique key type, so it goes into non_unique
                assert!(
                    unique_keys_to_add.is_empty(),
                    "expected no unique keys for masternode transfer key"
                );
                assert_eq!(
                    non_unique_keys_to_add.len(),
                    1,
                    "expected exactly one non-unique key"
                );
                assert_eq!(non_unique_keys_to_add[0].id(), 1);
            }
            _ => unreachable!(),
        }
    }
}
