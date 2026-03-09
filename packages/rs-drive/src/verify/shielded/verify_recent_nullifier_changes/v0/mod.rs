use crate::drive::shielded::nullifiers::queries::shielded_recent_nullifiers_path_vec;
use crate::drive::shielded::nullifiers::types::{CompactedNullifiers, NullifierChangePerBlock};
use crate::drive::Drive;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::verify::RootHash;
use grovedb::{Element, GroveDb, PathQuery, Query, SizedQuery};
use platform_version::version::PlatformVersion;

impl Drive {
    /// Verifies recent nullifier changes proof.
    ///
    /// Uses the same query as the prove function: a simple range query
    /// starting from start_block_height.
    pub(super) fn verify_recent_nullifier_changes_v0(
        proof: &[u8],
        start_block_height: u64,
        limit: Option<u16>,
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<NullifierChangePerBlock>), Error> {
        let path = shielded_recent_nullifiers_path_vec();

        // Create the same range query as the prove function
        let mut query = Query::new();
        query.insert_range_from(start_block_height.to_be_bytes().to_vec()..);

        let path_query = PathQuery::new(path, SizedQuery::new(query, limit, None));

        let (root_hash, proved_key_values) = if verify_subset_of_proof {
            GroveDb::verify_subset_query(proof, &path_query, &platform_version.drive.grove_version)?
        } else {
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)?
        };

        let mut nullifier_changes = Vec::new();

        for (_path, key, maybe_element) in proved_key_values {
            let Some(element) = maybe_element else {
                continue;
            };

            // Parse block height from key (8 bytes, big-endian)
            let height_bytes: [u8; 8] = key.try_into().map_err(|_| {
                Error::Proof(ProofError::CorruptedProof(
                    "invalid block height key length".to_string(),
                ))
            })?;
            let block_height = u64::from_be_bytes(height_bytes);

            // Get the serialized data from the ItemWithSumItem element
            let Element::ItemWithSumItem(serialized_data, _, _) = element else {
                return Err(Error::Proof(ProofError::CorruptedProof(
                    "expected item with sum item element for nullifiers".to_string(),
                )));
            };

            // Deserialize the nullifier list
            let nullifiers = CompactedNullifiers::decode(&serialized_data)?;

            nullifier_changes.push(NullifierChangePerBlock {
                block_height,
                nullifiers,
            });
        }

        Ok((root_hash, nullifier_changes))
    }
}
