use crate::drive::shielded::paths::shielded_anchors_credit_pool_path_vec;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use grovedb::{GroveDb, PathQuery, Query, SizedQuery};
use platform_version::version::PlatformVersion;

impl Drive {
    pub(super) fn verify_shielded_anchors_v0(
        proof: &[u8],
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<[u8; 32]>), Error> {
        let path_query = PathQuery {
            path: shielded_anchors_credit_pool_path_vec(),
            query: SizedQuery {
                query: Query::new_range_full(),
                limit: None,
                offset: None,
            },
        };

        let (root_hash, proved_key_values) = if verify_subset_of_proof {
            GroveDb::verify_subset_query(proof, &path_query, &platform_version.drive.grove_version)?
        } else {
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)?
        };

        let mut anchors = Vec::with_capacity(proved_key_values.len());
        for (_, key, _) in proved_key_values {
            let anchor: [u8; 32] = key.try_into().map_err(|_: Vec<u8>| {
                Error::Drive(DriveError::CorruptedElementType(
                    "anchor key is not 32 bytes",
                ))
            })?;
            anchors.push(anchor);
        }

        Ok((root_hash, anchors))
    }
}
