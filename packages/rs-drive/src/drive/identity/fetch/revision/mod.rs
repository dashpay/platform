mod fetch_identity_revision;

#[cfg(feature = "server")]
#[cfg(test)]
mod tests {
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::Identity;
    use dpp::version::PlatformVersion;

    mod fetch_identity_revision {
        use super::*;

        #[test]
        fn should_return_none_for_non_existent_identity() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let revision = drive
                .fetch_identity_revision([0; 32], true, None, platform_version)
                .expect("should not error");

            assert!(revision.is_none());
        }

        #[test]
        fn should_return_initial_revision_for_new_identity() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let identity = Identity::random_identity(3, Some(42), platform_version)
                .expect("expected a random identity");

            let expected_revision = identity.revision();

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

            let revision = drive
                .fetch_identity_revision(identity.id().to_buffer(), true, None, platform_version)
                .expect("should not error")
                .expect("should have revision");

            assert_eq!(revision, expected_revision);
        }

        #[test]
        fn should_return_updated_revision_after_update() {
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

            let new_revision = 5;
            let block_info = BlockInfo::default();

            drive
                .update_identity_revision(
                    identity.id().to_buffer(),
                    new_revision,
                    &block_info,
                    true,
                    None,
                    platform_version,
                    None,
                )
                .expect("expected to update revision");

            let revision = drive
                .fetch_identity_revision(identity.id().to_buffer(), true, None, platform_version)
                .expect("should not error")
                .expect("should have revision");

            assert_eq!(revision, new_revision);
        }

        #[test]
        fn should_return_revision_with_fees() {
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

            let block_info = BlockInfo::default();

            let (revision, fee_result) = drive
                .fetch_identity_revision_with_fees(
                    identity.id().to_buffer(),
                    &block_info,
                    true,
                    None,
                    platform_version,
                )
                .expect("should not error");

            assert_eq!(revision, Some(identity.revision()));
            assert!(fee_result.processing_fee > 0);
        }
    }
}
