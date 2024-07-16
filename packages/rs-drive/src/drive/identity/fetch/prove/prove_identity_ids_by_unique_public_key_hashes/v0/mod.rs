use crate::drive::Drive;

use crate::error::Error;

use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Proves identity ids against public key hashes.
    pub(super) fn prove_identity_ids_by_unique_public_key_hashes_v0(
        &self,
        public_key_hashes: &[[u8; 20]],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let path_query = Self::identity_ids_by_unique_public_key_hash_query(public_key_hashes);
        self.grove_get_proved_path_query(
            &path_query,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::Drive;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::identity::identity_public_key::methods::hash::IdentityPublicKeyHashMethodsV0;
    use dpp::identity::Identity;

    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

    #[test]
    fn should_prove_multiple_identity_ids() {
        let drive = setup_drive_with_initial_state_structure();
        let platform_version = PlatformVersion::latest();

        let identities: BTreeMap<[u8; 32], Identity> =
            Identity::random_identities(10, 3, Some(14), platform_version)
                .expect("expected to get random identities")
                .into_iter()
                .map(|identity| (identity.id().to_buffer(), identity))
                .collect();

        for identity in identities.values() {
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
        }

        let key_hashes_to_identity_ids = identities
            .values()
            .flat_map(|identity| {
                identity
                    .public_keys()
                    .values()
                    .filter(|public_key| public_key.key_type().is_unique_key_type())
                    .map(move |public_key| {
                        (
                            public_key
                                .public_key_hash()
                                .expect("expected to be 20 bytes"),
                            Some(identity.id().to_buffer()),
                        )
                    })
            })
            .collect::<BTreeMap<[u8; 20], Option<[u8; 32]>>>();

        let key_hashes = key_hashes_to_identity_ids
            .keys()
            .copied()
            .collect::<Vec<[u8; 20]>>();

        let proof = drive
            .prove_identity_ids_by_unique_public_key_hashes(&key_hashes, None, platform_version)
            .expect("should not error when proving an identity");

        let (_, proved_identity_id): ([u8; 32], BTreeMap<[u8; 20], Option<[u8; 32]>>) =
            Drive::verify_identity_ids_by_public_key_hashes(
                proof.as_slice(),
                false,
                &key_hashes,
                platform_version,
            )
            .expect("expect that this be verified");

        assert_eq!(proved_identity_id, key_hashes_to_identity_ids);
    }
}
