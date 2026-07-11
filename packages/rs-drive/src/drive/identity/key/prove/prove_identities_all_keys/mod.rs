mod v0;

use crate::drive::Drive;
use crate::error::{drive::DriveError, Error};
use dpp::version::drive_versions::DriveVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Proves the existence of all keys associated with the specified identities.
    ///
    /// This function creates a path query for each identity ID provided, requesting
    /// all keys associated with each identity. It then proves the existence of the keys
    /// using the provided `transaction`.
    ///
    /// # Arguments
    ///
    /// * `identity_ids` - A slice of identity IDs as 32-byte arrays. Each identity ID is used to
    ///   create a path query for proving its associated keys.
    /// * `limit` - An optional `u16` value specifying the maximum number of keys to fetch for each
    ///   identity. If `None`, fetches all available keys.
    /// * `transaction` - A `TransactionArg` object representing the transaction to be used
    ///   for proving the existence of the keys.
    /// * `drive_version` - A reference to the drive version.
    ///
    /// # Returns
    ///
    /// * `Result<Vec<u8>, Error>` - If successful, returns a `Vec<u8>` containing the proof data.
    ///   If an error occurs during the proof generation, returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function returns an error if the path query generation or proof generation fails or the version is not supported.
    pub fn prove_identities_all_keys(
        &self,
        identity_ids: &[[u8; 32]],
        limit: Option<u16>,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<Vec<u8>, Error> {
        match drive_version
            .methods
            .identity
            .keys
            .prove
            .prove_identities_all_keys
        {
            0 => self.prove_identities_all_keys_v0(identity_ids, limit, transaction, drive_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_identities_all_keys".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::util::test_helpers::setup::setup_drive;
    use dpp::block::block_info::BlockInfo;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::Identity;
    use dpp::version::PlatformVersion;

    #[test]
    fn should_prove_single_identity_all_keys() {
        let drive = setup_drive(None);
        let platform_version = PlatformVersion::latest();

        drive
            .create_initial_state_structure(None, platform_version)
            .expect("expected to create root tree successfully");

        let identity = Identity::random_identity(3, Some(44444), platform_version)
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
            .expect("expected to insert identity");

        let proof = drive
            .prove_identities_all_keys(
                &[identity.id().to_buffer()],
                None,
                None,
                &platform_version.drive,
            )
            .expect("expected to generate proof for single identity");

        assert!(!proof.is_empty(), "proof should be non-empty");
    }

    #[test]
    fn should_prove_multiple_identities_all_keys() {
        let drive = setup_drive(None);
        let platform_version = PlatformVersion::latest();

        drive
            .create_initial_state_structure(None, platform_version)
            .expect("expected to create root tree successfully");

        let identity_a = Identity::random_identity(3, Some(55555), platform_version)
            .expect("expected a random identity");
        let identity_b = Identity::random_identity(2, Some(66666), platform_version)
            .expect("expected a random identity");

        drive
            .add_new_identity(
                identity_a.clone(),
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to insert identity a");

        drive
            .add_new_identity(
                identity_b.clone(),
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to insert identity b");

        let proof = drive
            .prove_identities_all_keys(
                &[identity_a.id().to_buffer(), identity_b.id().to_buffer()],
                None,
                None,
                &platform_version.drive,
            )
            .expect("expected to generate proof for multiple identities");

        assert!(!proof.is_empty(), "proof should be non-empty");
    }

    #[test]
    fn should_prove_identities_all_keys_with_limit() {
        let drive = setup_drive(None);
        let platform_version = PlatformVersion::latest();

        drive
            .create_initial_state_structure(None, platform_version)
            .expect("expected to create root tree successfully");

        let identity = Identity::random_identity(5, Some(77777), platform_version)
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
            .expect("expected to insert identity");

        let proof = drive
            .prove_identities_all_keys(
                &[identity.id().to_buffer()],
                Some(2),
                None,
                &platform_version.drive,
            )
            .expect("expected to generate proof with limit");

        assert!(!proof.is_empty(), "proof should be non-empty");
    }
}
