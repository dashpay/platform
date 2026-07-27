use crate::drive::identity::key::fetch::IdentityKeysRequest;
use crate::drive::Drive;
use crate::error::Error;
use grovedb::PathQuery;
use grovedb_version::version::GroveVersion;

impl Drive {
    /// Fetches the path queries for all keys associated with the specified identities.
    ///
    /// This function creates path queries for each identity ID provided, requesting
    /// all keys associated with each identity. It then merges the path queries into a single
    /// path query.
    ///
    /// # Arguments
    ///
    /// * `identity_ids` - A slice of identity IDs as 32-byte arrays. Each identity ID is used to
    ///   create a path query for fetching its associated keys.
    /// * `limit` - An optional `u16` value specifying the maximum number of keys to fetch for each
    ///   identity. If `None`, fetches all available keys.
    ///
    /// # Returns
    ///
    /// * `Result<PathQuery, Error>` - If successful, returns a `PathQuery` object containing the
    ///   merged path queries. If an error occurs during merging, returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function returns an error if the merging of path queries fails.
    pub fn fetch_identities_all_keys_query(
        &self,
        identity_ids: &[[u8; 32]],
        limit: Option<u16>,
        grove_version: &GroveVersion,
    ) -> Result<PathQuery, Error> {
        let path_queries = identity_ids
            .iter()
            .map(|identity_id| {
                let key_request = IdentityKeysRequest::new_all_keys_query(identity_id, limit);
                key_request.into_path_query()
            })
            .collect::<Vec<_>>();

        PathQuery::merge(path_queries.iter().collect(), grove_version).map_err(Error::from)
    }
}

#[cfg(feature = "server")]
#[cfg(test)]
mod tests {
    use crate::util::test_helpers::setup::setup_drive;
    use dpp::version::PlatformVersion;

    #[test]
    fn should_build_merged_query_for_single_identity() {
        let drive = setup_drive(None);
        let platform_version = PlatformVersion::latest();
        let grove_version = &platform_version.drive.grove_version;

        let identity_id: [u8; 32] = [1u8; 32];
        let result = drive.fetch_identities_all_keys_query(&[identity_id], None, grove_version);
        assert!(
            result.is_ok(),
            "expected successful path query for single identity"
        );

        let path_query = result.unwrap();
        assert!(
            !path_query.path.is_empty(),
            "expected non-empty path in query"
        );
    }

    #[test]
    fn should_build_merged_query_for_multiple_identities() {
        let drive = setup_drive(None);
        let platform_version = PlatformVersion::latest();
        let grove_version = &platform_version.drive.grove_version;

        let id_a: [u8; 32] = [1u8; 32];
        let id_b: [u8; 32] = [2u8; 32];
        let id_c: [u8; 32] = [3u8; 32];

        let result =
            drive.fetch_identities_all_keys_query(&[id_a, id_b, id_c], None, grove_version);
        assert!(
            result.is_ok(),
            "expected successful path query for multiple identities: {:?}",
            result.err()
        );
    }

    #[test]
    fn should_fail_for_empty_identity_ids_slice() {
        let drive = setup_drive(None);
        let platform_version = PlatformVersion::latest();
        let grove_version = &platform_version.drive.grove_version;

        let result = drive.fetch_identities_all_keys_query(&[], None, grove_version);
        // Merging zero path queries should fail
        assert!(
            result.is_err(),
            "expected error when merging zero path queries"
        );
    }

    #[test]
    fn should_build_query_with_limit() {
        let drive = setup_drive(None);
        let platform_version = PlatformVersion::latest();
        let grove_version = &platform_version.drive.grove_version;

        let identity_id: [u8; 32] = [7u8; 32];
        let result = drive.fetch_identities_all_keys_query(&[identity_id], Some(5), grove_version);
        assert!(result.is_ok(), "expected successful path query with limit");
    }
}
