use crate::drive::identity::key::fetch::IdentityKeysRequest;
use crate::drive::Drive;
use crate::error::Error;
use crate::error::Error::GroveDB;
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

        PathQuery::merge(path_queries.iter().collect(), grove_version).map_err(GroveDB)
    }
}
