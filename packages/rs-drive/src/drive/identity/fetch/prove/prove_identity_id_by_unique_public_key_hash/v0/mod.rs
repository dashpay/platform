use crate::drive::Drive;

use crate::error::Error;

use dpp::version::drive_versions::DriveVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Proves an identity id against a public key hash.
    pub(super) fn prove_identity_id_by_unique_public_key_hash_v0(
        &self,
        public_key_hash: [u8; 20],
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<Vec<u8>, Error> {
        let path_query = Self::identity_id_by_unique_public_key_hash_query(public_key_hash);
        self.grove_get_proved_path_query(&path_query, transaction, &mut vec![], drive_version)
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

    #[test]
    fn should_prove_a_single_identity_id() {
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

        let first_key_hash = identity
            .public_keys()
            .values()
            .find(|public_key| public_key.key_type().is_unique_key_type())
            .expect("expected a unique key")
            .public_key_hash()
            .expect("expected to hash data");

        let proof = drive
            .prove_identity_id_by_unique_public_key_hash_v0(
                first_key_hash,
                None,
                &platform_version.drive,
            )
            .expect("should not error when proving an identity");

        let (_, proved_identity_id) = Drive::verify_identity_id_by_public_key_hash(
            proof.as_slice(),
            false,
            first_key_hash,
            platform_version,
        )
        .expect("expect that this be verified");

        assert_eq!(proved_identity_id, Some(identity_id));
    }
}
