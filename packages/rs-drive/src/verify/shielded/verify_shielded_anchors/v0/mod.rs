use crate::drive::shielded::paths::shielded_anchors_credit_pool_path_vec;
use crate::drive::Drive;
use crate::error::Error;
use crate::verify::RootHash;
use grovedb::{GroveDb, PathQuery, Query, SizedQuery};
use platform_version::version::PlatformVersion;

impl Drive {
    pub(super) fn verify_shielded_anchors_v0(
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<Vec<u8>>), Error> {
        let path_query = PathQuery {
            path: shielded_anchors_credit_pool_path_vec(),
            query: SizedQuery {
                query: Query::new_range_full(),
                limit: None,
                offset: None,
            },
        };

        let (root_hash, proved_key_values) =
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)?;

        let anchors = proved_key_values
            .into_iter()
            .map(|(_, key, _)| key)
            .collect();

        Ok((root_hash, anchors))
    }
}
