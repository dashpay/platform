use crate::drive::shielded::paths::{
    shielded_credit_pool_path_vec, SHIELDED_MOST_RECENT_ANCHOR_KEY,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::verify::RootHash;
use grovedb::{Element, GroveDb, PathQuery, Query, SizedQuery};
use platform_version::version::PlatformVersion;

impl Drive {
    pub(super) fn verify_most_recent_shielded_anchor_v0(
        proof: &[u8],
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<[u8; 32]>), Error> {
        let path_query = PathQuery {
            path: shielded_credit_pool_path_vec(),
            query: SizedQuery {
                query: Query::new_single_key(vec![SHIELDED_MOST_RECENT_ANCHOR_KEY]),
                limit: Some(1),
                offset: None,
            },
        };

        let (root_hash, mut proved_key_values) = if verify_subset_of_proof {
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

        if proved_key_values.len() > 1 {
            return Err(Error::Proof(ProofError::TooManyElements(
                "expected at most 1 element for most recent shielded anchor",
            )));
        }

        let anchor = if let Some(proved) = proved_key_values.pop() {
            match proved.2 {
                Some(Element::Item(value, _)) => {
                    let anchor: [u8; 32] = value.try_into().map_err(|_| {
                        Error::Drive(DriveError::CorruptedElementType(
                            "most recent anchor is not 32 bytes",
                        ))
                    })?;
                    // A zero anchor means no anchor has been recorded yet
                    if anchor == [0u8; 32] {
                        None
                    } else {
                        Some(anchor)
                    }
                }
                Some(_) => {
                    return Err(Error::Proof(ProofError::CorruptedProof(
                        "expected an item for most recent shielded anchor".to_string(),
                    )));
                }
                None => None,
            }
        } else {
            None
        };

        Ok((root_hash, anchor))
    }
}
