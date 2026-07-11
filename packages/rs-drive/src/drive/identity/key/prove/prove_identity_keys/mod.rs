mod v0;

use crate::drive::identity::key::fetch::IdentityKeysRequest;
use crate::drive::Drive;
use crate::error::{drive::DriveError, Error};

use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Prove the requested identity keys.
    ///
    /// This function takes an `IdentityKeysRequest` and a `TransactionArg` as arguments
    /// and returns a proof of the requested identity keys as a `Vec<u8>` or an error
    /// if the proof cannot be generated.
    ///
    /// # Arguments
    ///
    /// * `key_request` - An `IdentityKeysRequest` containing the details of the
    ///   requested identity keys, such as the identity ID, request type, limit, and offset.
    /// * `transaction` - A `TransactionArg` representing the current transaction.
    /// * `drive_version` - A reference to the drive version.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<u8>)` - A proof of the requested identity keys as a `Vec<u8>` if the
    ///   proof is successfully generated.
    /// * `Err(Error)` - An error if the proof cannot be generated or the version is not supported.
    ///
    /// # Errors
    ///
    /// This function may return `UnknownVersionMismatch` error if the version is not supported.
    pub fn prove_identity_keys(
        &self,
        key_request: IdentityKeysRequest,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .keys
            .prove
            .prove_identity_keys
        {
            0 => self.prove_identity_keys_v0(key_request, transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_identity_keys".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::identity::key::fetch::IdentityKeysRequest;
    use crate::drive::identity::key::fetch::KeyRequestType;
    use crate::util::test_helpers::setup::setup_drive;
    use dpp::block::block_info::BlockInfo;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::Identity;
    use dpp::version::PlatformVersion;

    #[test]
    fn should_prove_all_identity_keys() {
        let drive = setup_drive(None);
        let platform_version = PlatformVersion::latest();

        drive
            .create_initial_state_structure(None, platform_version)
            .expect("expected to create root tree successfully");

        let identity = Identity::random_identity(3, Some(11111), platform_version)
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

        let key_request = IdentityKeysRequest::new_all_keys_query(&identity.id().to_buffer(), None);

        let proof = drive
            .prove_identity_keys(key_request, None, platform_version)
            .expect("expected to generate proof for all keys");

        assert!(!proof.is_empty(), "proof should be non-empty");
    }

    #[test]
    fn should_prove_specific_identity_keys() {
        let drive = setup_drive(None);
        let platform_version = PlatformVersion::latest();

        drive
            .create_initial_state_structure(None, platform_version)
            .expect("expected to create root tree successfully");

        let identity = Identity::random_identity(5, Some(22222), platform_version)
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

        let key_request =
            IdentityKeysRequest::new_specific_keys_query(&identity.id().to_buffer(), vec![0, 1]);

        let proof = drive
            .prove_identity_keys(key_request, None, platform_version)
            .expect("expected to generate proof for specific keys");

        assert!(!proof.is_empty(), "proof should be non-empty");
    }

    #[test]
    fn should_prove_latest_auth_master_key() {
        let drive = setup_drive(None);
        let platform_version = PlatformVersion::latest();

        drive
            .create_initial_state_structure(None, platform_version)
            .expect("expected to create root tree successfully");

        let identity = Identity::random_identity(5, Some(33333), platform_version)
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

        let key_request = IdentityKeysRequest {
            identity_id: identity.id().to_buffer(),
            request_type: KeyRequestType::LatestAuthenticationMasterKey,
            limit: None,
            offset: None,
        };

        let proof = drive
            .prove_identity_keys(key_request, None, platform_version)
            .expect("expected to generate proof for latest auth master key");

        assert!(!proof.is_empty(), "proof should be non-empty");
    }
}
