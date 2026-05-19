mod fetch_identity_balance_with_keys;
mod fetch_identity_balance_with_keys_and_revision;
mod fetch_identity_keys;
mod fetch_identity_revision_with_keys;
mod fetch_identity_with_balance;

#[cfg(feature = "server")]
#[cfg(test)]
mod tests {
    use crate::drive::identity::key::fetch::{IdentityKeysRequest, KeyRequestType};
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::Identity;
    use dpp::version::PlatformVersion;

    mod fetch_identity_with_balance {
        use super::*;

        #[test]
        fn should_return_none_for_non_existent_identity() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let result = drive
                .fetch_identity_with_balance([0; 32], None, platform_version)
                .expect("should not error");

            assert!(result.is_none());
        }

        #[test]
        fn should_return_partial_identity_with_balance() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let identity = Identity::random_identity(3, Some(42), platform_version)
                .expect("expected a random identity");

            let expected_balance = identity.balance();

            drive
                .add_new_identity(
                    identity.clone(),
                    false,
                    &BlockInfo::default(),
                    true,
                    None,
                    platform_version,
                )
                .expect("expected to add identity");

            let partial = drive
                .fetch_identity_with_balance(identity.id().to_buffer(), None, platform_version)
                .expect("should not error")
                .expect("should have partial identity");

            assert_eq!(partial.id, identity.id().clone());
            assert_eq!(partial.balance, Some(expected_balance));
            assert!(partial.loaded_public_keys.is_empty());
            assert!(partial.revision.is_none());
        }

        #[test]
        fn should_return_none_with_cost_when_not_applying() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let (partial, fee_result) = drive
                .fetch_identity_with_balance_with_cost([0; 32], false, None, platform_version)
                .expect("should not error");

            assert!(partial.is_none());
            assert!(fee_result.processing_fee > 0);
        }
    }

    mod fetch_identity_balance_with_keys {
        use super::*;

        #[test]
        fn should_return_none_for_non_existent_identity() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let key_request = IdentityKeysRequest {
                identity_id: [0; 32],
                request_type: KeyRequestType::AllKeys,
                limit: None,
                offset: None,
            };

            let result = drive
                .fetch_identity_balance_with_keys(key_request, None, platform_version)
                .expect("should not error");

            assert!(result.is_none());
        }

        #[test]
        fn should_return_partial_identity_with_balance_and_keys() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let identity = Identity::random_identity(3, Some(42), platform_version)
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
                .expect("expected to add identity");

            let key_request = IdentityKeysRequest {
                identity_id: identity.id().to_buffer(),
                request_type: KeyRequestType::AllKeys,
                limit: None,
                offset: None,
            };

            let partial = drive
                .fetch_identity_balance_with_keys(key_request, None, platform_version)
                .expect("should not error")
                .expect("should have partial identity");

            assert_eq!(partial.id, identity.id().clone());
            assert_eq!(partial.balance, Some(identity.balance()));
            assert_eq!(partial.loaded_public_keys.len(), 3);
            assert!(partial.revision.is_none());
        }

        #[test]
        fn should_return_partial_identity_with_specific_keys() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let identity = Identity::random_identity(5, Some(42), platform_version)
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
                .expect("expected to add identity");

            let key_request = IdentityKeysRequest {
                identity_id: identity.id().to_buffer(),
                request_type: KeyRequestType::SpecificKeys(vec![0, 1]),
                limit: Some(2),
                offset: None,
            };

            let partial = drive
                .fetch_identity_balance_with_keys(key_request, None, platform_version)
                .expect("should not error")
                .expect("should have partial identity");

            assert_eq!(partial.loaded_public_keys.len(), 2);
            assert!(partial.loaded_public_keys.contains_key(&0));
            assert!(partial.loaded_public_keys.contains_key(&1));
        }

        #[test]
        fn should_track_not_found_keys() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let identity = Identity::random_identity(3, Some(42), platform_version)
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
                .expect("expected to add identity");

            // Request key id 99 which does not exist
            let key_request = IdentityKeysRequest {
                identity_id: identity.id().to_buffer(),
                request_type: KeyRequestType::SpecificKeys(vec![0, 99]),
                limit: Some(2),
                offset: None,
            };

            let partial = drive
                .fetch_identity_balance_with_keys(key_request, None, platform_version)
                .expect("should not error")
                .expect("should have partial identity");

            assert_eq!(partial.loaded_public_keys.len(), 1);
            assert!(partial.loaded_public_keys.contains_key(&0));
            assert_eq!(partial.not_found_public_keys.len(), 1);
            assert!(partial.not_found_public_keys.contains(&99));
        }
    }

    mod fetch_identity_balance_with_keys_and_revision {
        use super::*;

        #[test]
        fn should_return_none_for_non_existent_identity() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let key_request = IdentityKeysRequest {
                identity_id: [0; 32],
                request_type: KeyRequestType::AllKeys,
                limit: None,
                offset: None,
            };

            let result = drive
                .fetch_identity_balance_with_keys_and_revision(key_request, None, platform_version)
                .expect("should not error");

            assert!(result.is_none());
        }

        #[test]
        fn should_return_partial_identity_with_balance_keys_and_revision() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let identity = Identity::random_identity(3, Some(42), platform_version)
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
                .expect("expected to add identity");

            let key_request = IdentityKeysRequest {
                identity_id: identity.id().to_buffer(),
                request_type: KeyRequestType::AllKeys,
                limit: None,
                offset: None,
            };

            let partial = drive
                .fetch_identity_balance_with_keys_and_revision(key_request, None, platform_version)
                .expect("should not error")
                .expect("should have partial identity");

            assert_eq!(partial.id, identity.id().clone());
            assert_eq!(partial.balance, Some(identity.balance()));
            assert_eq!(partial.revision, Some(identity.revision()));
            assert_eq!(partial.loaded_public_keys.len(), 3);
        }
    }

    mod fetch_identity_revision_with_keys {
        use super::*;

        #[test]
        fn should_return_none_for_non_existent_identity() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let key_request = IdentityKeysRequest {
                identity_id: [0; 32],
                request_type: KeyRequestType::AllKeys,
                limit: None,
                offset: None,
            };

            let result = drive
                .fetch_identity_revision_with_keys(key_request, None, platform_version)
                .expect("should not error");

            assert!(result.is_none());
        }

        #[test]
        fn should_return_partial_identity_with_revision_and_keys() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let identity = Identity::random_identity(3, Some(42), platform_version)
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
                .expect("expected to add identity");

            let key_request = IdentityKeysRequest {
                identity_id: identity.id().to_buffer(),
                request_type: KeyRequestType::AllKeys,
                limit: None,
                offset: None,
            };

            let partial = drive
                .fetch_identity_revision_with_keys(key_request, None, platform_version)
                .expect("should not error")
                .expect("should have partial identity");

            assert_eq!(partial.id, identity.id().clone());
            assert!(partial.balance.is_none());
            assert_eq!(partial.revision, Some(identity.revision()));
            assert_eq!(partial.loaded_public_keys.len(), 3);
        }
    }

    mod fetch_identity_keys_as_partial_identity {
        use super::*;

        #[test]
        fn should_return_partial_identity_with_only_keys() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let identity = Identity::random_identity(5, Some(42), platform_version)
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
                .expect("expected to add identity");

            let key_request = IdentityKeysRequest {
                identity_id: identity.id().to_buffer(),
                request_type: KeyRequestType::AllKeys,
                limit: None,
                offset: None,
            };

            let partial = drive
                .fetch_identity_keys_as_partial_identity(key_request, None, platform_version)
                .expect("should not error")
                .expect("should have partial identity");

            assert_eq!(partial.id, identity.id().clone());
            assert!(partial.balance.is_none());
            assert!(partial.revision.is_none());
            assert_eq!(partial.loaded_public_keys.len(), 5);
        }
    }
}
