mod fetch_full_identities_by_unique_public_key_hashes;
mod fetch_full_identity_by_unique_public_key_hash;
mod fetch_identity_id_by_unique_public_key_hash;
mod fetch_identity_ids_by_non_unique_public_key_hash;
mod fetch_identity_ids_by_unique_public_key_hashes;
mod has_any_of_unique_public_key_hashes;
mod has_non_unique_public_key_hash;
mod has_non_unique_public_key_hash_already_for_identity;
mod has_unique_public_key_hash;

#[cfg(feature = "server")]
#[cfg(test)]
mod tests {
    use crate::tests::helpers::setup::setup_drive;
    use dpp::block::block_info::BlockInfo;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::identity::identity_public_key::methods::hash::IdentityPublicKeyHashMethodsV0;
    use dpp::identity::Identity;
    use dpp::version::drive_versions::DriveVersion;
    use dpp::version::PlatformVersion;

    #[test]
    fn test_fetch_all_keys_on_identity() {
        let drive = setup_drive(None);
        let drive_version = DriveVersion::latest();

        let transaction = drive.grove.start_transaction();

        let platform_version = PlatformVersion::first();

        drive
            .create_initial_state_structure(Some(&transaction), platform_version)
            .expect("expected to create root tree successfully");

        let identity = Identity::random_identity(5, Some(12345), platform_version)
            .expect("expected a random identity");

        drive
            .add_new_identity(
                identity.clone(),
                false,
                &BlockInfo::default(),
                true,
                Some(&transaction),
                platform_version,
            )
            .expect("expected to insert identity");

        let public_keys = drive
            .fetch_all_identity_keys(
                identity.id().to_buffer(),
                Some(&transaction),
                platform_version,
            )
            .expect("expected to fetch keys");

        assert_eq!(public_keys.len(), 5);

        for (_, key) in public_keys {
            let hash = key.hash().expect("expected to get hash");
            if key.key_type().is_unique_key_type() {
                let identity_id = drive
                    .fetch_identity_id_by_unique_public_key_hash(
                        hash,
                        Some(&transaction),
                        platform_version,
                    )
                    .expect("expected to fetch identity_id")
                    .expect("expected to get an identity id");
                assert_eq!(identity_id, identity.id().to_buffer());
            } else {
                let identity_ids = drive
                    .fetch_identity_ids_by_non_unique_public_key_hash(
                        hash,
                        Some(&transaction),
                        &drive_version,
                    )
                    .expect("expected to get identity ids");
                assert!(identity_ids.contains(&identity.id().to_buffer()));
            }
        }
    }
}
