pub(crate) mod methods;
mod operations;
mod structs;

pub use structs::*;

#[cfg(test)]
mod tests {

    use dpp::prelude::*;

    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::identity::accessors::IdentityGettersV0;

    mod add_new_keys_to_identity {
        use super::*;
        use crate::fees::op::LowLevelDriveOperation;
        use dpp::block::block_info::BlockInfo;
        use dpp::block::epoch::Epoch;
        use dpp::fee::fee_result::FeeResult;
        use dpp::version::PlatformVersion;
        use rand::prelude::StdRng;
        use rand::{Rng, SeedableRng};

        // -------------------------------------------------
        // should_add_one_new_key_to_identity (4 tests)
        // -------------------------------------------------
        #[test]
        fn should_add_one_new_key_to_identity_first_version_apply() {
            let platform_version = PlatformVersion::first();
            let expected_fee_result = FeeResult {
                storage_fee: 14202000,
                processing_fee: 1098260,
                ..Default::default()
            };

            do_should_add_one_new_key_to_identity(true, platform_version, expected_fee_result);
        }

        #[test]
        fn should_add_one_new_key_to_identity_first_version_estimated() {
            let platform_version = PlatformVersion::first();
            // Adjust the expected processing_fee if it differs from "apply = true"
            // or as needed for your scenario
            let expected_fee_result = FeeResult {
                storage_fee: 17145000,
                processing_fee: 5483620,
                ..Default::default()
            };

            do_should_add_one_new_key_to_identity(false, platform_version, expected_fee_result);
        }

        #[test]
        fn should_add_one_new_key_to_identity_latest_version_apply() {
            let platform_version = PlatformVersion::latest();
            let expected_fee_result = FeeResult {
                storage_fee: 14202000,
                // 2 extra loaded bytes because the token tree is no longer empty
                // these 2 loaded bytes cost 20 credits each
                processing_fee: 1098300,
                ..Default::default()
            };

            do_should_add_one_new_key_to_identity(true, platform_version, expected_fee_result);
        }

        #[test]
        fn should_add_one_new_key_to_identity_latest_version_estimated() {
            let platform_version = PlatformVersion::latest();
            let expected_fee_result = FeeResult {
                storage_fee: 17145000,
                processing_fee: 5483620,
                ..Default::default()
            };

            do_should_add_one_new_key_to_identity(false, platform_version, expected_fee_result);
        }

        fn do_should_add_one_new_key_to_identity(
            apply: bool,
            platform_version: &PlatformVersion,
            expected_fee_result: FeeResult,
        ) {
            let drive = setup_drive_with_initial_state_structure(Some(platform_version));

            let identity = Identity::random_identity(5, Some(12345), platform_version)
                .expect("expected a random identity");

            let block = BlockInfo::default_with_epoch(Epoch::new(0).unwrap());

            // We assume the user wants to apply or just estimate for adding identity
            drive
                .add_new_identity(
                    identity.clone(),
                    false,
                    &block,
                    apply,
                    None,
                    platform_version,
                )
                .expect("expected to insert identity");

            let new_keys_to_add =
                IdentityPublicKey::random_authentication_keys(5, 1, Some(15), platform_version);

            let db_transaction = drive.grove.start_transaction();

            let fee_result = drive
                .add_new_unique_keys_to_identity(
                    identity.id().to_buffer(),
                    new_keys_to_add,
                    &block,
                    apply,
                    Some(&db_transaction),
                    platform_version,
                )
                .expect("expected to update identity with new keys");

            assert_eq!(fee_result, expected_fee_result);

            if apply {
                drive
                    .grove
                    .commit_transaction(db_transaction)
                    .unwrap()
                    .expect("expected to be able to commit a transaction");

                let identity_keys = drive
                    .fetch_all_identity_keys(identity.id().to_buffer(), None, platform_version)
                    .expect("expected to get keys");
                assert_eq!(identity_keys.len(), 6); // we had 5 keys and we added 1
            } else {
                // Not applying -> no commit. We can check root hash if we want
                let app_hash_after = drive
                    .grove
                    .root_hash(None, &platform_version.drive.grove_version)
                    .unwrap()
                    .expect("should return app hash");
                // Or any other logic to ensure no state changes actually took effect
                let _ = app_hash_after;
            }
        }

        // -------------------------------------------------
        // check_reference_below_tokens_cost (4 tests)
        // This test exists to make sure the update cost that goes through the tokens tree
        // (as key hash references are below the token tree)
        // stay the same cost.
        // -------------------------------------------------
        #[test]
        fn check_reference_below_tokens_cost_first_version_apply() {
            let platform_version = PlatformVersion::first();
            let expected_fee_result = FeeResult {
                storage_fee: 9423000,
                processing_fee: 406100,
                ..Default::default()
            };
            do_check_reference_below_tokens_cost(true, platform_version, expected_fee_result);
        }

        #[test]
        fn check_reference_below_tokens_cost_first_version_estimated() {
            let platform_version = PlatformVersion::first();
            let expected_fee_result = FeeResult {
                storage_fee: 9423000,
                processing_fee: 314560,
                ..Default::default()
            };
            do_check_reference_below_tokens_cost(false, platform_version, expected_fee_result);
        }

        #[test]
        fn check_reference_below_tokens_cost_latest_version_apply() {
            let platform_version = PlatformVersion::latest();
            let expected_fee_result = FeeResult {
                storage_fee: 9423000,
                // 2 extra loaded bytes because the token tree is no longer empty
                // these 2 loaded bytes cost 20 credits each
                processing_fee: 406140,
                ..Default::default()
            };
            do_check_reference_below_tokens_cost(true, platform_version, expected_fee_result);
        }

        #[test]
        fn check_reference_below_tokens_cost_latest_version_estimated() {
            let platform_version = PlatformVersion::latest();
            let expected_fee_result = FeeResult {
                storage_fee: 9423000,
                // 2 extra loaded bytes because the token tree is no longer empty
                // these 2 loaded bytes cost 20 credits each
                processing_fee: 314600,
                ..Default::default()
            };
            do_check_reference_below_tokens_cost(false, platform_version, expected_fee_result);
        }

        fn do_check_reference_below_tokens_cost(
            apply: bool,
            platform_version: &PlatformVersion,
            expected_fee_result: FeeResult,
        ) {
            let drive = setup_drive_with_initial_state_structure(Some(platform_version));

            let identity = Identity::random_identity(5, Some(12345), platform_version)
                .expect("expected a random identity");

            let block = BlockInfo::default_with_epoch(Epoch::new(0).unwrap());

            drive
                .add_new_identity(
                    identity.clone(),
                    false,
                    &block,
                    apply,
                    None,
                    platform_version,
                )
                .expect("expected to insert identity");

            IdentityPublicKey::random_authentication_keys(5, 1, Some(15), platform_version);

            let mut rng = StdRng::seed_from_u64(23450);

            let db_transaction = drive.grove.start_transaction();

            let batch_operations = drive
                .insert_non_unique_public_key_hash_reference_to_identity_operations(
                    identity.id().to_buffer(),
                    rng.gen(),
                    &mut None,
                    Some(&db_transaction),
                    &platform_version.drive,
                )
                .expect("expected to update identity with new keys");

            let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
            drive
                .apply_batch_low_level_drive_operations(
                    None,
                    Some(&db_transaction),
                    batch_operations,
                    &mut drive_operations,
                    &platform_version.drive,
                )
                .expect("expected to apply operations");

            let fee_result = drive
                .calculate_fee(
                    None,
                    Some(drive_operations),
                    &Epoch::new(0).unwrap(),
                    platform_version,
                )
                .expect("expected fee result");

            assert_eq!(fee_result, expected_fee_result);

            if apply {
                drive
                    .grove
                    .commit_transaction(db_transaction)
                    .unwrap()
                    .expect("expected to be able to commit a transaction");
            } else {
                // Not applying -> no commit
            }
        }

        // -------------------------------------------------
        // should_add_two_dozen_new_keys_to_identity (4 tests)
        // -------------------------------------------------
        #[test]
        fn should_add_two_dozen_new_keys_to_identity_first_version_apply() {
            let platform_version = PlatformVersion::first();
            let expected_fee_result = FeeResult {
                storage_fee: 347382000,
                processing_fee: 6819220,
                ..Default::default()
            };

            do_should_add_two_dozen_new_keys_to_identity(
                true,
                platform_version,
                expected_fee_result,
            );
        }

        #[test]
        fn should_add_two_dozen_new_keys_to_identity_first_version_estimated() {
            let platform_version = PlatformVersion::first();
            // Possibly different processing fee if "estimated" differs
            let expected_fee_result = FeeResult {
                storage_fee: 356211000,
                processing_fee: 11699520,
                ..Default::default()
            };

            do_should_add_two_dozen_new_keys_to_identity(
                false,
                platform_version,
                expected_fee_result,
            );
        }

        #[test]
        fn should_add_two_dozen_new_keys_to_identity_latest_version_apply() {
            let platform_version = PlatformVersion::latest();
            let expected_fee_result = FeeResult {
                storage_fee: 347382000,
                processing_fee: 6819260,
                ..Default::default()
            };

            do_should_add_two_dozen_new_keys_to_identity(
                true,
                platform_version,
                expected_fee_result,
            );
        }

        #[test]
        fn should_add_two_dozen_new_keys_to_identity_latest_version_estimated() {
            let platform_version = PlatformVersion::latest();
            let expected_fee_result = FeeResult {
                storage_fee: 356211000,
                processing_fee: 11699520,
                ..Default::default()
            };

            do_should_add_two_dozen_new_keys_to_identity(
                false,
                platform_version,
                expected_fee_result,
            );
        }

        fn do_should_add_two_dozen_new_keys_to_identity(
            apply: bool,
            platform_version: &PlatformVersion,
            expected_fee_result: FeeResult,
        ) {
            let drive = setup_drive_with_initial_state_structure(Some(platform_version));

            let identity = Identity::random_identity(5, Some(12345), platform_version)
                .expect("expected a random identity");

            let block = BlockInfo::default_with_epoch(Epoch::new(0).unwrap());

            drive
                .add_new_identity(
                    identity.clone(),
                    false,
                    &block,
                    apply,
                    None,
                    platform_version,
                )
                .expect("expected to insert identity");

            let new_keys_to_add =
                IdentityPublicKey::random_authentication_keys(5, 24, Some(15), platform_version);

            let db_transaction = drive.grove.start_transaction();

            let fee_result = drive
                .add_new_unique_keys_to_identity(
                    identity.id().to_buffer(),
                    new_keys_to_add,
                    &block,
                    apply,
                    Some(&db_transaction),
                    platform_version,
                )
                .expect("expected to update identity with new keys");

            assert_eq!(fee_result, expected_fee_result);

            if apply {
                drive
                    .grove
                    .commit_transaction(db_transaction)
                    .unwrap()
                    .expect("expected to be able to commit a transaction");

                let identity_keys = drive
                    .fetch_all_identity_keys(identity.id().to_buffer(), None, platform_version)
                    .expect("expected to get balance");

                assert_eq!(identity_keys.len(), 29); // we had 5 keys, added 24
            } else {
                // Not applying => no commit
            }
        }
    }

    mod add_new_keys_to_masternode_identity {
        use super::*;
        use crate::drive::Drive;
        use crate::error::identity::IdentityError;
        use crate::error::Error;
        use crate::util::batch::drive_op_batch::DriveLowLevelOperationConverter;
        use crate::util::batch::DriveOperation;
        use crate::util::batch::IdentityOperationType;
        use crate::util::estimated_layers::EstimatedLayers;
        use dpp::block::block_info::BlockInfo;
        use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
        use dpp::identity::identity_public_key::methods::hash::IdentityPublicKeyHashMethodsV0;
        use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
        use dpp::identity::{IdentityV0, KeyID, KeyType, Purpose, SecurityLevel};
        use dpp::platform_value::BinaryData;
        use dpp::version::PlatformVersion;
        use std::collections::BTreeMap;

        const MASTERNODE_IDENTITY_ID: [u8; 32] = [1; 32];

        fn masternode_key(id: KeyID, key_type: KeyType, purpose: Purpose) -> IdentityPublicKey {
            IdentityPublicKeyV0 {
                id,
                key_type,
                purpose,
                security_level: SecurityLevel::CRITICAL,
                read_only: true,
                data: BinaryData::new(vec![(id + 2) as u8; 20]),
                disabled_at: None,
                contract_bounds: None,
            }
            .into()
        }

        #[test]
        fn should_emit_the_operations_of_add_new_keys_to_identity_at_every_protocol_version() {
            for protocol_version in 1..=PlatformVersion::latest().protocol_version {
                let platform_version =
                    PlatformVersion::get(protocol_version).expect("expected a platform version");
                let drive = setup_drive_with_initial_state_structure(Some(platform_version));
                let masternode_identity: Identity = IdentityV0 {
                    id: MASTERNODE_IDENTITY_ID.into(),
                    public_keys: BTreeMap::from([(
                        0,
                        masternode_key(0, KeyType::ECDSA_HASH160, Purpose::TRANSFER),
                    )]),
                    balance: 0,
                    revision: 0,
                }
                .into();
                drive
                    .add_new_identity(
                        masternode_identity,
                        true,
                        &BlockInfo::default(),
                        true,
                        None,
                        platform_version,
                    )
                    .expect("expected to insert the masternode identity");

                let keys = vec![
                    masternode_key(1, KeyType::ECDSA_HASH160, Purpose::TRANSFER),
                    masternode_key(2, KeyType::EDDSA_25519_HASH160, Purpose::SYSTEM),
                ];
                let operations = |operation: IdentityOperationType, estimate: bool| {
                    let mut estimated_costs = estimate.then(EstimatedLayers::new);
                    let operations = operation
                        .into_low_level_drive_operations(
                            &drive,
                            &mut estimated_costs,
                            &BlockInfo::default(),
                            None,
                            platform_version,
                        )
                        .expect("expected operations");
                    (operations, estimated_costs)
                };
                for estimate in [false, true] {
                    assert_eq!(
                        operations(
                            IdentityOperationType::AddNewKeysToMasternodeIdentity {
                                identity_id: MASTERNODE_IDENTITY_ID,
                                non_unique_keys_to_add: keys.clone(),
                            },
                            estimate
                        ),
                        operations(
                            IdentityOperationType::AddNewKeysToIdentity {
                                identity_id: MASTERNODE_IDENTITY_ID,
                                unique_keys_to_add: vec![],
                                non_unique_keys_to_add: keys.clone(),
                            },
                            estimate
                        ),
                        "protocol version {protocol_version}, estimate {estimate}"
                    );
                }
            }
        }

        /// A masternode identity holding only a key with hash `[2; 20]`, created like Platform
        /// creates one, at `platform_version`.
        fn drive_with_masternode_identity(platform_version: &PlatformVersion) -> Drive {
            let drive = setup_drive_with_initial_state_structure(Some(platform_version));
            let masternode_identity: Identity = IdentityV0 {
                id: MASTERNODE_IDENTITY_ID.into(),
                public_keys: BTreeMap::from([(
                    0,
                    masternode_key(0, KeyType::ECDSA_HASH160, Purpose::TRANSFER),
                )]),
                balance: 0,
                revision: 0,
            }
            .into();
            drive
                .add_new_identity(
                    masternode_identity,
                    true,
                    &BlockInfo::default(),
                    true,
                    None,
                    platform_version,
                )
                .expect("expected to insert the masternode identity");
            drive
        }

        /// A masternode key with id `id`, of `key_type` and `purpose`, and data `data`.
        fn key_with_data(
            id: KeyID,
            key_type: KeyType,
            purpose: Purpose,
            data: [u8; 20],
        ) -> IdentityPublicKey {
            IdentityPublicKeyV0 {
                id,
                key_type,
                purpose,
                security_level: SecurityLevel::CRITICAL,
                read_only: true,
                data: BinaryData::new(data.to_vec()),
                disabled_at: None,
                contract_bounds: None,
            }
            .into()
        }

        fn add_keys(
            drive: &Drive,
            keys: Vec<IdentityPublicKey>,
            platform_version: &PlatformVersion,
        ) -> Result<(), Error> {
            drive
                .apply_drive_operations(
                    vec![DriveOperation::IdentityOperation(
                        IdentityOperationType::AddNewKeysToMasternodeIdentity {
                            identity_id: MASTERNODE_IDENTITY_ID,
                            non_unique_keys_to_add: keys,
                        },
                    )],
                    true,
                    &BlockInfo::default(),
                    None,
                    platform_version,
                )
                .map(|_| ())
        }

        fn key_ids(drive: &Drive, platform_version: &PlatformVersion) -> Vec<KeyID> {
            drive
                .fetch_full_identity(MASTERNODE_IDENTITY_ID, None, platform_version)
                .expect("expected to fetch the identity")
                .expect("expected the identity")
                .public_keys()
                .keys()
                .copied()
                .collect()
        }

        fn holds_reference(drive: &Drive, hash: [u8; 20]) -> bool {
            drive
                .has_non_unique_public_key_hash_already_for_identity_operations(
                    hash,
                    MASTERNODE_IDENTITY_ID,
                    None,
                    &mut vec![],
                    &PlatformVersion::latest().drive,
                )
                .expect("expected to read the reference")
        }

        /// Core accepts any 20 bytes as an operator payout or platform node id, so a masternode
        /// identity can need a key whose hash a key added to it earlier has, such as a payout
        /// equal to an earlier node id. Up to protocol version 14 that key fails the batch with
        /// `IdentityKeyAlreadyExists`; from 15 it is stored under its own id, and the identity
        /// keeps its one reference for the hash.
        #[test]
        fn should_add_a_key_whose_hash_the_identity_already_references_from_protocol_version_15() {
            let hash = [0x9f; 20];
            for (platform_version, added) in [
                (
                    PlatformVersion::get(14).expect("expected protocol version 14"),
                    false,
                ),
                (PlatformVersion::latest(), true),
            ] {
                let drive = drive_with_masternode_identity(platform_version);
                add_keys(
                    &drive,
                    vec![key_with_data(
                        1,
                        KeyType::EDDSA_25519_HASH160,
                        Purpose::SYSTEM,
                        hash,
                    )],
                    platform_version,
                )
                .expect("expected to add the first key");
                let result = add_keys(
                    &drive,
                    vec![key_with_data(
                        2,
                        KeyType::ECDSA_HASH160,
                        Purpose::TRANSFER,
                        hash,
                    )],
                    platform_version,
                );
                let context = format!("protocol version {}", platform_version.protocol_version);
                if added {
                    result.expect("expected to add the second key");
                    assert_eq!(key_ids(&drive, platform_version), [0, 1, 2], "{context}");
                } else {
                    assert!(
                        matches!(
                            result,
                            Err(Error::Identity(IdentityError::IdentityKeyAlreadyExists(_)))
                        ),
                        "{context}: {result:?}"
                    );
                    assert_eq!(key_ids(&drive, platform_version), [0, 1], "{context}");
                }
                assert!(holds_reference(&drive, hash), "{context}");
            }
        }

        /// Two keys with one hash in a single call are both stored from protocol version 15,
        /// under one reference for the hash.
        #[test]
        fn should_add_two_keys_with_one_hash_in_one_call() {
            let platform_version = PlatformVersion::latest();
            let hash = [0x9e; 20];
            let drive = drive_with_masternode_identity(platform_version);
            add_keys(
                &drive,
                vec![
                    key_with_data(1, KeyType::EDDSA_25519_HASH160, Purpose::SYSTEM, hash),
                    key_with_data(2, KeyType::ECDSA_HASH160, Purpose::TRANSFER, hash),
                ],
                platform_version,
            )
            .expect("expected to add both keys");
            assert_eq!(key_ids(&drive, platform_version), [0, 1, 2]);
            assert!(holds_reference(&drive, hash));
        }

        /// Masternodes whose operators share a payout address add keys of one hash no identity held
        /// before, to several identities in one batch. Each identity's operation queues that hash's
        /// tree under the non-unique key hashes tree; from protocol version 15 the batch creates it
        /// once and keeps every identity's key and entry, also where batches are checked for
        /// consistency, which refuse the repeated insert at protocol version 14.
        #[test]
        fn should_add_keys_of_one_new_hash_to_several_identities_in_one_batch() {
            let other_identity_id = [2; 32];
            let hash = [0x9b; 20];
            for (platform_version, applied) in [
                (
                    PlatformVersion::get(14).expect("expected protocol version 14"),
                    false,
                ),
                (PlatformVersion::latest(), true),
            ] {
                let drive = drive_with_masternode_identity(platform_version);
                let other_identity: Identity = IdentityV0 {
                    id: other_identity_id.into(),
                    public_keys: BTreeMap::from([(
                        0,
                        key_with_data(0, KeyType::ECDSA_HASH160, Purpose::TRANSFER, [3; 20]),
                    )]),
                    balance: 0,
                    revision: 0,
                }
                .into();
                drive
                    .add_new_identity(
                        other_identity,
                        true,
                        &BlockInfo::default(),
                        true,
                        None,
                        platform_version,
                    )
                    .expect("expected to insert the other masternode identity");
                let add = |identity_id: [u8; 32]| {
                    DriveOperation::IdentityOperation(
                        IdentityOperationType::AddNewKeysToMasternodeIdentity {
                            identity_id,
                            non_unique_keys_to_add: vec![key_with_data(
                                1,
                                KeyType::ECDSA_HASH160,
                                Purpose::TRANSFER,
                                hash,
                            )],
                        },
                    )
                };
                let result = drive.apply_drive_operations(
                    vec![add(MASTERNODE_IDENTITY_ID), add(other_identity_id)],
                    true,
                    &BlockInfo::default(),
                    None,
                    platform_version,
                );
                let context = format!("protocol version {}", platform_version.protocol_version);
                if applied {
                    result.expect("expected to add both keys");
                    assert_eq!(key_ids(&drive, platform_version), [0, 1], "{context}");
                    assert!(holds_reference(&drive, hash), "{context}");
                    assert!(
                        drive
                            .has_non_unique_public_key_hash_already_for_identity_operations(
                                hash,
                                other_identity_id,
                                None,
                                &mut vec![],
                                &platform_version.drive,
                            )
                            .expect("expected to read the reference"),
                        "{context}"
                    );
                } else {
                    assert!(result.is_err(), "{context}: {result:?}");
                }
            }
        }

        /// Estimating the cost of a call with two keys of one hash counts the hash's reference
        /// once, as applying it writes it once: fewer operations than for two keys of two hashes.
        #[test]
        fn should_estimate_one_reference_for_two_keys_with_one_hash() {
            let platform_version = PlatformVersion::latest();
            let drive = drive_with_masternode_identity(platform_version);
            let estimated_operations = |second_hash: [u8; 20]| {
                let mut estimated_costs = Some(EstimatedLayers::new());
                IdentityOperationType::AddNewKeysToMasternodeIdentity {
                    identity_id: MASTERNODE_IDENTITY_ID,
                    non_unique_keys_to_add: vec![
                        key_with_data(1, KeyType::EDDSA_25519_HASH160, Purpose::SYSTEM, [0x9d; 20]),
                        key_with_data(2, KeyType::ECDSA_HASH160, Purpose::TRANSFER, second_hash),
                    ],
                }
                .into_low_level_drive_operations(
                    &drive,
                    &mut estimated_costs,
                    &BlockInfo::default(),
                    None,
                    platform_version,
                )
                .expect("expected operations")
                .len()
            };
            assert!(estimated_operations([0x9d; 20]) < estimated_operations([0x9c; 20]));
        }

        /// A key whose hash is another identity's unique key hash is added without a non-unique
        /// key hash reference, at protocol version 14 and from 15 alike.
        #[test]
        fn should_add_a_key_with_a_unique_key_hash_without_a_reference() {
            for platform_version in [
                PlatformVersion::get(14).expect("expected protocol version 14"),
                PlatformVersion::latest(),
            ] {
                let drive = drive_with_masternode_identity(platform_version);
                let user_identity = Identity::random_identity(3, Some(15), platform_version)
                    .expect("expected a random identity");
                let unique_hash = user_identity
                    .public_keys()
                    .values()
                    .find(|key| key.key_type().is_unique_key_type())
                    .expect("expected a unique key")
                    .public_key_hash()
                    .expect("expected the key's hash");
                drive
                    .add_new_identity(
                        user_identity,
                        false,
                        &BlockInfo::default(),
                        true,
                        None,
                        platform_version,
                    )
                    .expect("expected to insert the user identity");
                add_keys(
                    &drive,
                    vec![key_with_data(
                        1,
                        KeyType::ECDSA_HASH160,
                        Purpose::TRANSFER,
                        unique_hash,
                    )],
                    platform_version,
                )
                .expect("expected to add the key");
                let context = format!("protocol version {}", platform_version.protocol_version);
                assert_eq!(key_ids(&drive, platform_version), [0, 1], "{context}");
                assert!(!holds_reference(&drive, unique_hash), "{context}");
            }
        }
    }

    mod disable_identity_keys {
        use super::*;
        use chrono::Utc;
        use dpp::block::block_info::BlockInfo;
        use dpp::block::epoch::Epoch;
        use dpp::fee::fee_result::FeeResult;
        use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
        use dpp::version::PlatformVersion;

        // -------------------------------------------------
        // should_disable_a_few_keys (4 tests)
        // -------------------------------------------------
        #[test]
        fn should_disable_a_few_keys_first_version_apply() {
            let platform_version = PlatformVersion::first();
            let expected_fee_result = FeeResult {
                storage_fee: 513000,
                processing_fee: 869380,
                ..Default::default()
            };
            do_should_disable_a_few_keys(true, platform_version, expected_fee_result);
        }

        #[test]
        fn should_disable_a_few_keys_first_version_estimated() {
            let platform_version = PlatformVersion::first();
            let expected_fee_result = FeeResult {
                storage_fee: 486000,
                processing_fee: 3216860,
                ..Default::default()
            };
            do_should_disable_a_few_keys(false, platform_version, expected_fee_result);
        }

        #[test]
        fn should_disable_a_few_keys_latest_version_apply() {
            let platform_version = PlatformVersion::latest();
            let expected_fee_result = FeeResult {
                storage_fee: 513000,
                processing_fee: 869380,
                ..Default::default()
            };
            do_should_disable_a_few_keys(true, platform_version, expected_fee_result);
        }

        #[test]
        fn should_disable_a_few_keys_protocol_version_13_estimated() {
            // Protocol 13 estimates with a maximal stand-in key (disable_identity_keys v0).
            let platform_version = PlatformVersion::get(13).expect("protocol 13");
            let expected_fee_result = FeeResult {
                storage_fee: 486000,
                processing_fee: 3216860,
                ..Default::default()
            };
            do_should_disable_a_few_keys(false, platform_version, expected_fee_result);
        }

        #[test]
        fn should_disable_a_few_keys_latest_version_estimated() {
            // Protocol 14 estimates from the stored keys (disable_identity_keys v1), so the
            // storage estimate matches the applied cost exactly.
            let platform_version = PlatformVersion::latest();
            let expected_fee_result = FeeResult {
                storage_fee: 513000,
                processing_fee: 3195760,
                ..Default::default()
            };
            do_should_disable_a_few_keys(false, platform_version, expected_fee_result);
        }

        fn do_should_disable_a_few_keys(
            apply: bool,
            platform_version: &PlatformVersion,
            expected_fee_result: FeeResult,
        ) {
            let drive = setup_drive_with_initial_state_structure(Some(platform_version));

            let identity = Identity::random_identity(5, Some(12345), platform_version)
                .expect("expected a random identity");

            let block_info = BlockInfo::default_with_epoch(Epoch::new(0).unwrap());

            drive
                .add_new_identity(
                    identity.clone(),
                    false,
                    &block_info,
                    true,
                    None,
                    platform_version,
                )
                .expect("expected to insert identity");

            let new_keys_to_add = IdentityPublicKey::random_keys(5, 2, Some(15), platform_version);

            drive
                .add_new_unique_keys_to_identity(
                    identity.id().to_buffer(),
                    new_keys_to_add.clone(),
                    &block_info,
                    true,
                    None,
                    platform_version,
                )
                .expect("expected to update identity with new keys");

            let db_transaction = drive.grove.start_transaction();

            let key_ids = new_keys_to_add.into_iter().map(|key| key.id()).collect();

            let disable_at = Utc::now().timestamp_millis() as TimestampMillis;

            let fee_result = drive
                .disable_identity_keys(
                    identity.id().to_buffer(),
                    key_ids,
                    disable_at,
                    &block_info,
                    apply,
                    Some(&db_transaction),
                    platform_version,
                )
                .expect("should disable a few keys");

            assert_eq!(fee_result, expected_fee_result);

            if apply {
                drive
                    .grove
                    .commit_transaction(db_transaction)
                    .unwrap()
                    .expect("expected to commit transaction");

                let identity_keys = drive
                    .fetch_all_identity_keys(identity.id().to_buffer(), None, platform_version)
                    .expect("expected to get balance");

                assert_eq!(identity_keys.len(), 7); // we had 5 keys and we added 2
                for (_, key) in identity_keys.into_iter().skip(5) {
                    assert_eq!(key.disabled_at(), Some(disable_at));
                }
            }
        }

        #[test]
        fn estimated_costs_should_have_same_storage_cost_first_version() {
            let platform_version = PlatformVersion::first();
            let expected_estimated_fee_result = FeeResult {
                storage_fee: 486000,
                processing_fee: 3216860,
                ..Default::default()
            };
            let expected_fee_result = FeeResult {
                storage_fee: 486000,
                processing_fee: 794720,
                ..Default::default()
            };
            estimated_costs_should_have_same_storage_cost(
                platform_version,
                expected_estimated_fee_result,
                expected_fee_result,
            );
        }

        #[test]
        fn estimated_costs_should_have_same_storage_cost_protocol_version_13() {
            let platform_version = PlatformVersion::get(13).expect("protocol 13");
            let expected_estimated_fee_result = FeeResult {
                storage_fee: 486000,
                processing_fee: 3216860,
                ..Default::default()
            };
            let expected_fee_result = FeeResult {
                storage_fee: 486000,
                processing_fee: 794720,
                ..Default::default()
            };
            estimated_costs_should_have_same_storage_cost(
                platform_version,
                expected_estimated_fee_result,
                expected_fee_result,
            );
        }

        #[test]
        fn estimated_costs_should_have_same_storage_cost_latest_version() {
            let platform_version = PlatformVersion::latest();
            // disable_identity_keys v1 also reads the stored keys during estimation.
            let expected_estimated_fee_result = FeeResult {
                storage_fee: 486000,
                processing_fee: 3251060,
                ..Default::default()
            };
            let expected_fee_result = FeeResult {
                storage_fee: 486000,
                processing_fee: 794720,
                ..Default::default()
            };
            estimated_costs_should_have_same_storage_cost(
                platform_version,
                expected_estimated_fee_result,
                expected_fee_result,
            );
        }

        fn estimated_costs_should_have_same_storage_cost(
            platform_version: &PlatformVersion,
            expected_estimated_fee_result: FeeResult,
            expected_fee_result: FeeResult,
        ) {
            let drive = setup_drive_with_initial_state_structure(Some(platform_version));

            let identity = Identity::random_identity(5, Some(12345), platform_version)
                .expect("expected a random identity");

            drive
                .add_new_identity(
                    identity.clone(),
                    false,
                    &BlockInfo::default(),
                    true,
                    None,
                    platform_version,
                )
                .expect("expected to add an identity");

            let block_info = BlockInfo::default_with_epoch(Epoch::new(0).unwrap());

            let disable_at = Utc::now().timestamp_millis() as TimestampMillis;

            let estimated_fee_result = drive
                .disable_identity_keys(
                    identity.id().to_buffer(),
                    vec![0, 1],
                    disable_at,
                    &block_info,
                    false,
                    None,
                    platform_version,
                )
                .expect("should estimate the disabling of a few keys");

            let fee_result = drive
                .disable_identity_keys(
                    identity.id().to_buffer(),
                    vec![0, 1],
                    disable_at,
                    &block_info,
                    true,
                    None,
                    platform_version,
                )
                .expect("should get the cost of the disabling a few keys");

            assert_eq!(estimated_fee_result.storage_fee, fee_result.storage_fee);
            assert_eq!(estimated_fee_result, expected_estimated_fee_result);
            assert_eq!(fee_result, expected_fee_result);
        }
    }

    mod update_identity_revision {
        use super::*;
        use dpp::block::block_info::BlockInfo;
        use dpp::block::epoch::Epoch;
        use dpp::fee::fee_result::FeeResult;
        use dpp::version::PlatformVersion;

        // -------------------------------------------------
        // should_update_revision (4 tests)
        // -------------------------------------------------
        #[test]
        fn should_update_revision_first_version_apply() {
            let platform_version = PlatformVersion::first();
            let expected_fee_result = FeeResult {
                storage_fee: 0,
                processing_fee: 238820,
                removed_bytes_from_system: 0,
                ..Default::default()
            };

            do_should_update_revision(true, platform_version, expected_fee_result);
        }

        #[test]
        fn should_update_revision_first_version_estimated() {
            let platform_version = PlatformVersion::first();
            // Possibly different if your scenario's estimated cost differs
            let expected_fee_result = FeeResult {
                storage_fee: 0,
                processing_fee: 1813560,
                removed_bytes_from_system: 0,
                ..Default::default()
            };

            do_should_update_revision(false, platform_version, expected_fee_result);
        }

        #[test]
        fn should_update_revision_latest_version_apply() {
            let platform_version = PlatformVersion::latest();
            let expected_fee_result = FeeResult {
                storage_fee: 0,
                processing_fee: 238820,
                removed_bytes_from_system: 0,
                ..Default::default()
            };

            do_should_update_revision(true, platform_version, expected_fee_result);
        }

        #[test]
        fn should_update_revision_latest_version_estimated() {
            let platform_version = PlatformVersion::latest();
            // Possibly different if your scenario's estimated cost differs
            let expected_fee_result = FeeResult {
                storage_fee: 0,
                processing_fee: 1813560,
                removed_bytes_from_system: 0,
                ..Default::default()
            };

            do_should_update_revision(false, platform_version, expected_fee_result);
        }

        fn do_should_update_revision(
            apply: bool,
            platform_version: &PlatformVersion,
            expected_fee_result: FeeResult,
        ) {
            let drive = setup_drive_with_initial_state_structure(Some(platform_version));

            let identity = Identity::random_identity(5, Some(12345), platform_version)
                .expect("expected a random identity");
            let block_info = BlockInfo::default_with_epoch(Epoch::new(0).unwrap());

            // Insert identity
            drive
                .add_new_identity(
                    identity.clone(),
                    false,
                    &block_info,
                    apply,
                    None,
                    platform_version,
                )
                .expect("expected to insert identity");

            // We'll update the revision from 0 -> 2
            let revision = 2;

            let db_transaction = drive.grove.start_transaction();

            let fee_result = drive
                .update_identity_revision(
                    identity.id().to_buffer(),
                    revision,
                    &block_info,
                    apply,
                    Some(&db_transaction),
                    platform_version,
                )
                .expect("should update revision");

            assert_eq!(fee_result, expected_fee_result);

            if apply {
                drive
                    .grove
                    .commit_transaction(db_transaction)
                    .unwrap()
                    .expect("expected to be able to commit a transaction");

                // check new revision
                let updated_revision = drive
                    .fetch_identity_revision(
                        identity.id().to_buffer(),
                        true,
                        None,
                        platform_version,
                    )
                    .expect("expected to get revision");
                assert_eq!(updated_revision, Some(revision));
            } else {
                // No commit => no changes
            }
        }
    }
}
