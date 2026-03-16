mod fetch_identity_nonce;
mod prove_identity_nonce;

#[cfg(feature = "server")]
#[cfg(test)]
mod tests {
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::Identity;
    use dpp::version::PlatformVersion;

    mod fetch_identity_nonce {
        use super::*;

        #[test]
        fn should_return_none_for_non_existent_identity() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let nonce = drive
                .fetch_identity_nonce([0; 32], true, None, platform_version)
                .expect("should not error");

            assert!(nonce.is_none());
        }

        #[test]
        fn should_return_initial_nonce_for_new_identity() {
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

            let nonce = drive
                .fetch_identity_nonce(identity.id().to_buffer(), true, None, platform_version)
                .expect("should not error")
                .expect("should have nonce");

            // New identity should have nonce 0
            assert_eq!(nonce, 0);
        }

        #[test]
        fn should_return_updated_nonce_after_merge() {
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

            // Merge nonce to update it
            drive
                .merge_identity_nonce(
                    identity.id().to_buffer(),
                    1,
                    &BlockInfo::default(),
                    true,
                    None,
                    platform_version,
                )
                .expect("expected to merge nonce");

            let nonce = drive
                .fetch_identity_nonce(identity.id().to_buffer(), true, None, platform_version)
                .expect("should not error")
                .expect("should have nonce");

            // After merging nonce 1 onto an identity with initial nonce 0, the stored
            // value is exactly 1: the low 40 bits hold the nonce tip (1) and the upper
            // 24 bits hold missing-revision flags which are all zero since nonce 0 was
            // already present and there are no gaps.
            assert_eq!(nonce, 1);
        }

        #[test]
        fn should_return_nonce_with_fees() {
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

            let (nonce, fee_result) = drive
                .fetch_identity_nonce_with_fees(
                    identity.id().to_buffer(),
                    &block_info,
                    true,
                    None,
                    platform_version,
                )
                .expect("should not error");

            assert_eq!(nonce, Some(0));
            assert!(fee_result.processing_fee > 0);
        }
    }
}
