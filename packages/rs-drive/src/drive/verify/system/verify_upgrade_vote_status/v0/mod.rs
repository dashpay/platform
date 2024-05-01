use crate::drive::protocol_upgrade::desired_version_for_validators_path_vec;
use crate::drive::verify::RootHash;
use crate::drive::Drive;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::query::{Query, QueryItem};
use dpp::util::deserializer::ProtocolVersion;
use grovedb::{GroveDb, PathQuery, SizedQuery};
use integer_encoding::VarInt;
use std::collections::BTreeMap;
use std::ops::RangeFull;
impl Drive {
    /// Verifies a proof containing the current upgrade state.
    ///
    /// # Parameters
    ///
    /// - `proof`: A byte slice representing the proof to be verified.
    /// - `first_pro_tx_hash`: the first pro tx hash that we are querying for.
    /// - `count`: the amount of Evonodes that we want to retrieve.
    ///
    /// # Returns
    ///
    /// Returns a `Result` with a tuple of `RootHash` and `BTreeMap<[u8;32], ProtocolVersion>`. The `BTreeMap<[u8;32], ProtocolVersion>`
    /// represents a map of the version that each Evonode has voted for.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    ///
    /// - The proof is corrupted.
    /// - The GroveDb query fails.
    #[inline(always)]
    pub(super) fn verify_upgrade_vote_status_v0(
        proof: &[u8],
        start_protx_hash: Option<[u8; 32]>,
        count: u16,
    ) -> Result<(RootHash, BTreeMap<[u8; 32], ProtocolVersion>), Error> {
        let path = desired_version_for_validators_path_vec();

        let query_item = if let Some(start_protx_hash) = start_protx_hash {
            QueryItem::RangeFrom(start_protx_hash.to_vec()..)
        } else {
            QueryItem::RangeFull(RangeFull)
        };

        let path_query = PathQuery::new(
            path,
            SizedQuery::new(Query::new_single_query_item(query_item), Some(count), None),
        );

        let (root_hash, elements) = GroveDb::verify_query(proof, &path_query)?;

        let protocol_version_map = elements
            .into_iter()
            .map(|(_, key, element)| {
                let pro_tx_hash: [u8; 32] = key.try_into().map_err(|_| {
                    ProofError::CorruptedProof("protocol version not decodable".to_string())
                })?;
                let element = element.ok_or(ProofError::CorruptedProof(
                    "expected a count for each version, got none".to_string(),
                ))?;
                let version_bytes = element.as_item_bytes().map_err(|_| {
                    ProofError::CorruptedProof(
                        "expected an item for the element of a version".to_string(),
                    )
                })?;
                let version = u32::decode_var(version_bytes)
                    .ok_or(ProofError::CorruptedProof(
                        "version count not decodable".to_string(),
                    ))?
                    .0;
                Ok((pro_tx_hash, version))
            })
            .collect::<Result<BTreeMap<[u8; 32], ProtocolVersion>, Error>>()?;

        Ok((root_hash, protocol_version_map))
    }
}
