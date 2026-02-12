use crate::drive::shielded::paths::shielded_credit_pool_nullifiers_path_vec;
use crate::drive::Drive;
use crate::error::Error;
use crate::verify::RootHash;
use grovedb::{GroveDb, PathQuery, Query, SizedQuery};
use platform_version::version::PlatformVersion;

impl Drive {
    pub(super) fn verify_shielded_nullifiers_v0(
        proof: &[u8],
        nullifiers: &[Vec<u8>],
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<(Vec<u8>, bool)>), Error> {
        let mut query = Query::new();
        query.insert_keys(nullifiers.to_vec());

        let path_query = PathQuery {
            path: shielded_credit_pool_nullifiers_path_vec(),
            query: SizedQuery {
                query,
                limit: None,
                offset: None,
            },
        };

        let (root_hash, proved_key_values) = if verify_subset_of_proof {
            GroveDb::verify_subset_query_with_absence_proof(
                proof,
                &path_query,
                &platform_version.drive.grove_version,
            )?
        } else {
            GroveDb::verify_query_with_absence_proof(
                proof,
                &path_query,
                &platform_version.drive.grove_version,
            )?
        };

        // Map each proved entry: if element is Some, nullifier is spent; if None, not spent
        let statuses = proved_key_values
            .into_iter()
            .map(|(_, key, maybe_element)| {
                let is_spent = maybe_element.is_some();
                (key, is_spent)
            })
            .collect();

        Ok((root_hash, statuses))
    }
}
