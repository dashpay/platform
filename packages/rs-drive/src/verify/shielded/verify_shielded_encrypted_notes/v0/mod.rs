use crate::drive::shielded::paths::shielded_credit_pool_encrypted_notes_path_vec;
use crate::drive::Drive;
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
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<(Vec<u8>, Vec<u8>)>), Error> {
        let limit = if count == 0 || count > max_elements {
            max_elements as u16
        } else {
            count as u16
        };

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

        let (root_hash, proved_key_values) =
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)?;

        let notes = proved_key_values
            .into_iter()
            .filter_map(|(_, _key, maybe_element)| {
                if let Some(Element::Item(value, _)) = maybe_element {
                    // Value format: cmx (32 bytes) || encrypted_note (remaining bytes)
                    if value.len() > 32 {
                        Some((value[..32].to_vec(), value[32..].to_vec()))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        Ok((root_hash, notes))
    }
}
