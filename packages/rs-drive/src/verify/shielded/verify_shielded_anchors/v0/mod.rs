use crate::drive::shielded::paths::shielded_credit_pool_anchors_path_vec;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use grovedb::{Element, GroveDb, PathQuery, Query, SizedQuery};
use platform_version::version::PlatformVersion;

impl Drive {
    pub(super) fn verify_shielded_anchors_v0(
        proof: &[u8],
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<[u8; 32]>), Error> {
        let path_query = PathQuery {
            path: shielded_credit_pool_anchors_path_vec(),
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
        for (_, _key, maybe_element) in proved_key_values {
            match maybe_element {
                Some(Element::Item(value, _)) => {
                    let anchor: [u8; 32] = value.try_into().map_err(|_v: Vec<u8>| {
                        Error::Drive(DriveError::CorruptedElementType(
                            "anchor value is not 32 bytes",
                        ))
                    })?;
                    anchors.push(anchor);
                }
                Some(_) => {
                    return Err(Error::Drive(DriveError::CorruptedElementType(
                        "expected Item element for anchor, got different element type",
                    )));
                }
                None => {}
            }
        }

        Ok((root_hash, anchors))
    }
}
