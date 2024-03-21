use crate::drive::protocol_upgrade::versions_counter_path_vec;
use crate::drive::verify::RootHash;
use crate::drive::Drive;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::query::{Query, QueryItem};
use dpp::util::deserializer::ProtocolVersion;
use grovedb::{GroveDb, PathQuery};
use integer_encoding::VarInt;
use nohash_hasher::IntMap;
use std::ops::RangeFull;
impl Drive {
    /// Verifies a proof containing the current upgrade state.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice representing the proof to be verified.
    /// - `platform_version`: the platform version,
    ///
    /// # Returns
    ///
    /// Returns a `Result` with a tuple of `RootHash` and `IntMap<ProtocolVersion, u64>`. The `IntMap<ProtocolVersion, u64>`
    /// represents vote count of each version in the current epoch.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    ///
    /// - The proof is corrupted.
    /// - The GroveDb query fails.
    #[inline(always)]
    pub(super) fn verify_upgrade_state_v0(
        proof: &[u8],
    ) -> Result<(RootHash, IntMap<ProtocolVersion, u64>), Error> {
        let path_query = PathQuery::new_unsized(
            versions_counter_path_vec(),
            Query::new_single_query_item(QueryItem::RangeFull(RangeFull)),
        );

        let (root_hash, elements) = GroveDb::verify_query(proof, &path_query)?;

        let protocol_version_map = elements
            .into_iter()
            .map(|(_, key, element)| {
                let version = ProtocolVersion::decode_var(key.as_slice())
                    .ok_or(ProofError::CorruptedProof(
                        "protocol version not decodable".to_string(),
                    ))?
                    .0;
                let element = element.ok_or(ProofError::CorruptedProof(
                    "expected a count for each version, got none".to_string(),
                ))?;
                let count_bytes = element.as_item_bytes().map_err(|_| {
                    ProofError::CorruptedProof(
                        "expected an item for the element of a version".to_string(),
                    )
                })?;
                let count = u64::decode_var(count_bytes)
                    .ok_or(ProofError::CorruptedProof(
                        "version count not decodable".to_string(),
                    ))?
                    .0;
                Ok((version, count))
            })
            .collect::<Result<IntMap<ProtocolVersion, u64>, Error>>()?;

        Ok((root_hash, protocol_version_map))
    }
}
