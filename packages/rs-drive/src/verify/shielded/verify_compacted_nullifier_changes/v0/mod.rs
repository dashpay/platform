use crate::drive::saved_block_transactions::COMPACTED_NULLIFIERS_KEY_U8;
use crate::drive::Drive;
use crate::drive::RootTree;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::verify::RootHash;
use grovedb::operations::proof::{GroveDBProof, ProofBytes};
use grovedb::{
    GroveDb, MerkProofDecoder, MerkProofNode, MerkProofOp, PathQuery, Query, SizedQuery,
};
use platform_version::version::PlatformVersion;

use super::VerifiedCompactedNullifierChanges;

/// Extract KV entries from merk proof bytes using the proper decoder.
#[allow(clippy::type_complexity)]
fn extract_kv_entries_from_merk_proof(merk_proof: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>, Error> {
    let mut entries = Vec::new();

    let decoder = MerkProofDecoder::new(merk_proof);

    for op in decoder {
        match op {
            Ok(MerkProofOp::Push(MerkProofNode::KV(key, value)))
            | Ok(MerkProofOp::PushInverted(MerkProofNode::KV(key, value))) => {
                entries.push((key, value));
            }
            Err(e) => {
                tracing::error!(%e, "merk proof decode error");
                return Err(Error::Proof(ProofError::CorruptedProof(format!(
                    "failed to decode merk proof op: {}",
                    e
                ))));
            }
            _ => {}
        }
    }

    Ok(entries)
}

impl Drive {
    /// Verifies compacted nullifier changes proof.
    ///
    /// This verification works by:
    /// 1. Decoding the GroveDBProof structure
    /// 2. Navigating to the compacted nullifiers layer ('o')
    /// 3. Extracting KV entries from the merk proof
    /// 4. Filtering entries where the key range contains start_block_height
    /// 5. Verifying the root hash using a subset query
    pub(super) fn verify_compacted_nullifier_changes_v0(
        proof: &[u8],
        start_block_height: u64,
        limit: Option<u16>,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, VerifiedCompactedNullifierChanges), Error> {
        let bincode_config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();

        // Decode the GroveDBProof to navigate its structure
        let grovedb_proof: GroveDBProof = bincode::decode_from_slice(proof, bincode_config)
            .map(|(p, _)| p)
            .map_err(|e| {
                Error::Proof(ProofError::CorruptedProof(format!(
                    "cannot decode GroveDBProof: {}",
                    e
                )))
            })?;

        // Navigate to the compacted nullifiers layer
        // Path: SavedBlockTransactions ('$' = 0x24) -> CompactedNullifiers ('o' = 0x6f)
        let saved_block_key = vec![RootTree::SavedBlockTransactions as u8];
        let compacted_key = vec![COMPACTED_NULLIFIERS_KEY_U8];

        // Extract KV entries from the compacted layer's merk proof to find
        // if there's a containing range for start_block_height.
        // V0 and V1 proofs have different layer types (MerkOnlyLayerProof vs LayerProof),
        // so we handle them separately.
        let kv_entries = match &grovedb_proof {
            GroveDBProof::V0(v0) => {
                let compacted_layer = v0
                    .root_layer
                    .lower_layers
                    .get(&saved_block_key)
                    .and_then(|layer| layer.lower_layers.get(&compacted_key));
                compacted_layer
                    .map(|layer| extract_kv_entries_from_merk_proof(&layer.merk_proof))
                    .transpose()?
                    .unwrap_or_default()
            }
            GroveDBProof::V1(v1) => {
                let compacted_layer = v1
                    .root_layer
                    .lower_layers
                    .get(&saved_block_key)
                    .and_then(|layer| layer.lower_layers.get(&compacted_key));
                compacted_layer
                    .map(|layer| match &layer.merk_proof {
                        ProofBytes::Merk(bytes) => extract_kv_entries_from_merk_proof(bytes),
                        _ => Ok(vec![]),
                    })
                    .transpose()?
                    .unwrap_or_default()
            }
        };

        // Look for a KV entry where the range contains start_block_height
        // Keys are 16 bytes: (start_block, end_block), both big-endian
        let containing_key = kv_entries.iter().find_map(|(key, _)| {
            if key.len() != 16 {
                return None;
            }
            // Safety: length verified to be 16 above
            let range_start =
                u64::from_be_bytes(key[0..8].try_into().expect("len checked to be 16"));
            let range_end =
                u64::from_be_bytes(key[8..16].try_into().expect("len checked to be 16"));

            // Check if this range contains start_block_height
            if range_start <= start_block_height && start_block_height <= range_end {
                Some(key.clone())
            } else {
                None
            }
        });

        // Determine the start_key for the query
        // Use the containing range's key if found, otherwise (start_block_height, start_block_height)
        let start_key = containing_key.unwrap_or_else(|| {
            let mut key = Vec::with_capacity(16);
            key.extend_from_slice(&start_block_height.to_be_bytes());
            key.extend_from_slice(&start_block_height.to_be_bytes());
            key
        });

        // Verify the proof and get results using subset query
        let path = vec![
            vec![RootTree::SavedBlockTransactions as u8],
            vec![COMPACTED_NULLIFIERS_KEY_U8],
        ];

        let mut query = Query::new();
        query.insert_range_from(start_key..);

        let path_query = PathQuery::new(path, SizedQuery::new(query, limit, None));

        let (root_hash, proved_key_values) = GroveDb::verify_subset_query(
            proof,
            &path_query,
            &platform_version.drive.grove_version,
        )?;

        // Process the verified results
        let mut compacted_changes = Vec::new();

        for (_path, key, maybe_element) in proved_key_values {
            let Some(element) = maybe_element else {
                continue;
            };

            if key.len() != 16 {
                return Err(Error::Proof(ProofError::CorruptedProof(
                    "invalid compacted block key length, expected 16 bytes".to_string(),
                )));
            }

            let range_start = u64::from_be_bytes(
                key[0..8]
                    .try_into()
                    .map_err(|_| {
                        Error::Proof(ProofError::CorruptedProof(
                            "invalid key slice length for block height".to_string(),
                        ))
                    })?,
            );
            let range_end = u64::from_be_bytes(
                key[8..16]
                    .try_into()
                    .map_err(|_| {
                        Error::Proof(ProofError::CorruptedProof(
                            "invalid key slice length for block height".to_string(),
                        ))
                    })?,
            );

            // Get the serialized data from the Item element
            let grovedb::Element::Item(serialized_data, _) = element else {
                return Err(Error::Proof(ProofError::CorruptedProof(
                    "expected item element for compacted nullifiers".to_string(),
                )));
            };

            // Deserialize the nullifier list
            let (nullifiers, _): (Vec<[u8; 32]>, usize) =
                bincode::decode_from_slice(&serialized_data, bincode_config).map_err(|e| {
                    Error::Proof(ProofError::CorruptedProof(format!(
                        "cannot decode compacted nullifiers: {}",
                        e
                    )))
                })?;

            compacted_changes.push((range_start, range_end, nullifiers));
        }

        Ok((root_hash, compacted_changes))
    }
}
