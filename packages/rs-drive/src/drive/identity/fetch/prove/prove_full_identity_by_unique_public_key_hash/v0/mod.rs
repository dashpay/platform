use crate::drive::Drive;

use crate::error::Error;

use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Fetches an identity with all its information from storage.
    pub(super) fn prove_full_identity_by_unique_public_key_hash_v0(
        &self,
        public_key_hash: [u8; 20],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let identity_id = self.fetch_identity_id_by_unique_public_key_hash_operations(
            public_key_hash,
            transaction,
            &mut vec![],
            platform_version,
        )?;
        if let Some(identity_id) = identity_id {
            let query = Self::full_identity_with_public_key_hash_query(
                public_key_hash,
                identity_id,
                &platform_version.drive.grove_version,
            )?;
            self.grove_get_proved_path_query(
                &query,
                transaction,
                &mut vec![],
                &platform_version.drive,
            )
        } else {
            // We only prove the absence of the public key hash
            let query = Self::identity_id_by_unique_public_key_hash_query(public_key_hash);
            self.grove_get_proved_path_query(
                &query,
                transaction,
                &mut vec![],
                &platform_version.drive,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::identity::identity_public_key::methods::hash::IdentityPublicKeyHashMethodsV0;
    use dpp::identity::Identity;
    use dpp::version::PlatformVersion;

    #[test]
    fn should_prove_a_single_identity() {
        let drive = setup_drive_with_initial_state_structure();

        let platform_version = PlatformVersion::first();

        let identity = Identity::random_identity(3, Some(14), platform_version)
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

        let first_key_hash = identity
            .public_keys()
            .values()
            .find(|public_key| public_key.key_type().is_unique_key_type())
            .expect("expected a unique key")
            .public_key_hash()
            .expect("expected to hash data");

        let proof = drive
            .prove_full_identity_by_unique_public_key_hash(first_key_hash, None, platform_version)
            .expect("should not error when proving an identity");

        let (_, proved_identity) = Drive::verify_full_identity_by_public_key_hash(
            proof.as_slice(),
            first_key_hash,
            platform_version,
        )
        .expect("expect that this be verified");

        assert_eq!(proved_identity, Some(identity));
    }
}
