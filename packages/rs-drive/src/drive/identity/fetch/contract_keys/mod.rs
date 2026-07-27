mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use std::collections::BTreeMap;

use dpp::identifier::Identifier;
use dpp::identity::Purpose;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl Drive {
    /// Fetches identities contract keys given identity ids, contract id, optional document type name and purposes
    ///
    /// This function uses the versioning system to call the appropriate handler based on the provided `DriveVersion`.
    ///
    /// # Arguments
    ///
    /// * `identity_ids` - The slice of identity ids to prove
    /// * `contract_id` - The contract id
    /// * `document_type_name` - The optional document type name
    /// * `purposes` - Key purposes
    /// * `transaction` - Transaction arguments.
    /// * `drive_version` - A reference to the drive version.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a map with keys per purpose per identity id, otherwise an `Error` if the operation fails or the version is not supported.
    pub fn fetch_identities_contract_keys(
        &self,
        identity_ids: &[[u8; 32]],
        contract_id: &[u8; 32],
        document_type_name: Option<String>,
        purposes: Vec<Purpose>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<Identifier, BTreeMap<Purpose, Vec<u8>>>, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .keys
            .fetch
            .fetch_identities_contract_keys
        {
            0 => self.fetch_identities_contract_keys_v0(
                identity_ids,
                contract_id,
                document_type_name,
                purposes,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_identities_contract_keys".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(feature = "server")]
#[cfg(test)]
mod tests {
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::identity::Purpose;
    use dpp::version::PlatformVersion;

    mod fetch_identities_contract_keys {
        use super::*;

        #[test]
        fn should_return_empty_map_when_no_contract_keys_exist() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let identity_ids = [[1u8; 32]];
            let contract_id = [2u8; 32];
            let purposes = vec![Purpose::ENCRYPTION];

            // When there are no contract keys bound, the query returns an
            // empty result (the identity subtree exists but has no contract info).
            let result = drive.fetch_identities_contract_keys(
                &identity_ids,
                &contract_id,
                None,
                purposes,
                None,
                platform_version,
            );

            let map = result.expect("expected Ok result for non-existent identity");
            assert!(
                map.is_empty(),
                "expected empty map for non-existent identity"
            );
        }

        #[test]
        fn should_return_empty_for_existing_identity_without_contract_keys() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            use dpp::block::block_info::BlockInfo;
            use dpp::identity::accessors::IdentityGettersV0;
            use dpp::identity::Identity;

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

            let identity_ids = [identity.id().to_buffer()];
            let contract_id = [0xabu8; 32];
            let purposes = vec![Purpose::ENCRYPTION];

            // The identity exists but has no contract-bound keys, so the
            // query should return an empty result or skip that identity.
            let result = drive.fetch_identities_contract_keys(
                &identity_ids,
                &contract_id,
                None,
                purposes,
                None,
                platform_version,
            );

            let map = result.expect("expected Ok result for identity without contract keys");
            assert!(
                map.is_empty(),
                "expected empty map when no contract keys exist"
            );
        }

        #[test]
        fn should_return_empty_for_empty_identity_ids() {
            let drive = setup_drive_with_initial_state_structure(None);
            let platform_version = PlatformVersion::latest();

            let identity_ids: [[u8; 32]; 0] = [];
            let contract_id = [3u8; 32];
            let purposes = vec![Purpose::ENCRYPTION];

            let result = drive
                .fetch_identities_contract_keys(
                    &identity_ids,
                    &contract_id,
                    None,
                    purposes,
                    None,
                    platform_version,
                )
                .expect("should not error for empty ids");

            assert!(result.is_empty());
        }
    }
}
