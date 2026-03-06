use crate::drive::shielded::paths::{shielded_credit_pool_path_vec, SHIELDED_NOTES_KEY};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use grovedb::{Element, GroveDb, PathQuery, Query, QueryItem, SizedQuery, SubqueryBranch};
use platform_version::version::PlatformVersion;

impl Drive {
    pub(super) fn verify_shielded_encrypted_notes_v0(
        proof: &[u8],
        start_index: u64,
        count: u32,
        max_elements: u32,
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<(Vec<u8>, Vec<u8>, Vec<u8>)>), Error> {
        if max_elements == 0 {
            return Err(Error::Drive(DriveError::CorruptedElementType(
                "max_elements must be greater than zero",
            )));
        }

        // start_index must be chunk-aligned (multiple of max_elements)
        let chunk_size = max_elements as u64;
        if start_index % chunk_size != 0 {
            return Err(Error::Drive(DriveError::CorruptedElementType(
                "start_index is not chunk-aligned; must be a multiple of max_elements",
            )));
        }

        let effective = if count == 0 || count > max_elements {
            max_elements
        } else {
            count
        };
        let limit = effective.min(u16::MAX as u32) as u16;

        // PathQuery must match the server-side proof generation exactly:
        // path = [AddressBalances, "s"], key = [SHIELDED_NOTES_KEY],
        // subquery = range_inclusive(start..=end)
        let end_index = start_index + limit as u64 - 1;
        let mut inner_query = Query::new();
        inner_query.insert_range_inclusive(
            start_index.to_be_bytes().to_vec()..=end_index.to_be_bytes().to_vec(),
        );

        let path_query = PathQuery {
            path: shielded_credit_pool_path_vec(),
            query: SizedQuery {
                query: Query {
                    items: vec![QueryItem::Key(vec![SHIELDED_NOTES_KEY])],
                    default_subquery_branch: SubqueryBranch {
                        subquery_path: None,
                        subquery: Some(inner_query.into()),
                    },
                    left_to_right: true,
                    conditional_subquery_branches: None,
                    add_parent_tree_on_subquery: false,
                },
                limit: None,
                offset: None,
            },
        };

        let (root_hash, proved_key_values) = if verify_subset_of_proof {
            GroveDb::verify_subset_query(proof, &path_query, &platform_version.drive.grove_version)?
        } else {
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)?
        };

        let mut notes = Vec::with_capacity(proved_key_values.len());
        for (_, _key, maybe_element) in proved_key_values {
            match maybe_element {
                Some(Element::Item(value, _)) => {
                    // Value format: cmx (32) || nullifier (32) || encrypted_note (rest)
                    if value.len() <= 64 {
                        return Err(Error::Drive(DriveError::CorruptedElementType(
                            "encrypted note value too short: expected more than 64 bytes (cmx + nullifier + encrypted_note)",
                        )));
                    }
                    // Return (cmx, nullifier, encrypted_note)
                    notes.push((
                        value[..32].to_vec(),
                        value[32..64].to_vec(),
                        value[64..].to_vec(),
                    ));
                }
                Some(_) => {
                    return Err(Error::Drive(DriveError::CorruptedElementType(
                        "expected Item element for encrypted note, got different element type",
                    )));
                }
                None => {
                    // Absent elements in proof results are normal (key doesn't exist)
                }
            }
        }

        Ok((root_hash, notes))
    }
}
