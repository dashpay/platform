use crate::drive::shielded::paths::shielded_credit_pool_encrypted_notes_path_vec;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use grovedb::{Element, GroveDb, PathQuery, Query, SizedQuery};
use platform_version::version::PlatformVersion;

impl Drive {
    pub(super) fn verify_shielded_encrypted_notes_v0(
        proof: &[u8],
        start_index: u64,
        count: u32,
        max_elements: u32,
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<(Vec<u8>, Vec<u8>)>), Error> {
        let effective = if count == 0 || count > max_elements {
            max_elements
        } else {
            count
        };
        let limit = effective.min(u16::MAX as u32) as u16;

        let query = if start_index == 0 {
            Query::new_range_full()
        } else {
            let mut q = Query::new();
            q.insert_range_from(start_index.to_be_bytes().to_vec()..);
            q
        };

        let path_query = PathQuery {
            path: shielded_credit_pool_encrypted_notes_path_vec(),
            query: SizedQuery {
                query,
                limit: Some(limit),
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
                    // Value format: cmx (32 bytes) || encrypted_note (remaining bytes)
                    if value.len() <= 32 {
                        return Err(Error::Drive(DriveError::CorruptedElementType(
                            "encrypted note value too short: expected more than 32 bytes",
                        )));
                    }
                    notes.push((value[..32].to_vec(), value[32..].to_vec()));
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
