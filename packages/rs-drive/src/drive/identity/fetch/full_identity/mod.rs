mod fetch_full_identities;
mod fetch_full_identity;

#[cfg(feature = "server")]
#[cfg(test)]
mod tests {

    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;

    mod fetch_full_identities {
        use super::*;
        use dpp::block::block_info::BlockInfo;
        use dpp::identity::accessors::IdentityGettersV0;
        use dpp::identity::Identity;
        use dpp::version::PlatformVersion;
        use std::collections::BTreeMap;

        #[test]
        fn should_get_full_identities() {
            let drive = setup_drive_with_initial_state_structure();
            let platform_version = PlatformVersion::latest();

            let identities: BTreeMap<[u8; 32], Option<Identity>> =
                Identity::random_identities(10, 3, Some(14), platform_version)
                    .expect("expected to get random identities")
                    .into_iter()
                    .map(|identity| (identity.id().to_buffer(), Some(identity)))
                    .collect();

            for identity in identities.values() {
                drive
                    .add_new_identity(
                        identity.as_ref().unwrap().clone(),
                        false,
                        &BlockInfo::default(),
                        true,
                        None,
                        platform_version,
                    )
                    .expect("expected to add an identity");
            }
            let fetched_identities = drive
                .fetch_full_identities(
                    identities.keys().copied().collect::<Vec<_>>().as_slice(),
                    None,
                    platform_version,
                )
                .expect("should get identities");

            assert_eq!(identities, fetched_identities);
        }
    }

    mod fetch_full_identity {
        use super::*;
        use dpp::block::block_info::BlockInfo;
        use dpp::identity::accessors::IdentityGettersV0;
        use dpp::identity::Identity;
        use dpp::version::PlatformVersion;

        #[test]
        fn should_return_none_if_identity_is_not_present() {
            let drive = setup_drive_with_initial_state_structure();

            let platform_version = PlatformVersion::latest();

            let identity = drive
                .fetch_full_identity(
                    [
                        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                        0, 0, 0, 0, 0, 0, 0,
                    ],
                    None,
                    platform_version,
                )
                .expect("should return none");

            assert!(identity.is_none());
        }

        #[test]
        fn should_get_a_full_identity() {
            let drive = setup_drive_with_initial_state_structure();
            let platform_version = PlatformVersion::latest();

            let identity = Identity::random_identity(3, Some(14), platform_version)
                .expect("expected a random identity");

            let identity_id = identity.id().to_buffer();
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
            let fetched_identity = drive
                .fetch_full_identity(identity_id, None, platform_version)
                .expect("should not error when fetching an identity")
                .expect("should find an identity");

            assert_eq!(identity, fetched_identity);
        }
    }
}
