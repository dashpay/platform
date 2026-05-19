use crate::drive::Drive;
use crate::drive::RootTree;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::address_funds::PlatformAddress;

/// The subtree key for compacted address balances storage as u8
const COMPACTED_ADDRESS_BALANCES_KEY_U8: u8 = b'c';
use dpp::balances::credits::BlockAwareCreditOperation;
use grovedb::operations::proof::{GroveDBProof, ProofBytes};
use grovedb::{
    GroveDb, MerkProofDecoder, MerkProofNode, MerkProofOp, PathQuery, Query, SizedQuery,
};
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

use super::VerifiedCompactedAddressBalanceChanges;

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
    /// Verifies compacted address balance changes proof.
    ///
    /// This verification works by:
    /// 1. Decoding the GroveDBProof structure
    /// 2. Navigating to the compacted address balances layer ('c')
    /// 3. Extracting KV entries from the merk proof
    /// 4. Filtering entries where the key range contains start_block_height
    /// 5. Verifying the root hash using a subset query
    pub(super) fn verify_compacted_address_balance_changes_v0(
        proof: &[u8],
        start_block_height: u64,
        limit: Option<u16>,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, VerifiedCompactedAddressBalanceChanges), Error> {
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

        // Navigate to the compacted address balances layer
        // Path: SavedBlockTransactions ('$' = 0x24) -> CompactedAddressBalances ('c' = 0x63)
        let saved_block_key = vec![RootTree::SavedBlockTransactions as u8];
        let compacted_key = vec![COMPACTED_ADDRESS_BALANCES_KEY_U8];

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
                        other => Err(Error::Proof(ProofError::CorruptedProof(format!(
                            "unsupported V1 proof bytes variant for compacted address balances: {:?}",
                            std::mem::discriminant(other)
                        )))),
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
            let range_start = u64::from_be_bytes(key[0..8].try_into().unwrap());
            let range_end = u64::from_be_bytes(key[8..16].try_into().unwrap());

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
            vec![COMPACTED_ADDRESS_BALANCES_KEY_U8],
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

            let range_start = u64::from_be_bytes(key[0..8].try_into().unwrap());
            let range_end = u64::from_be_bytes(key[8..16].try_into().unwrap());

            // Get the serialized data from the Item element
            let grovedb::Element::Item(serialized_data, _) = element else {
                return Err(Error::Proof(ProofError::CorruptedProof(
                    "expected item element for compacted address balances".to_string(),
                )));
            };

            // Deserialize the address balance map
            let (address_balances, _): (
                BTreeMap<PlatformAddress, BlockAwareCreditOperation>,
                usize,
            ) = bincode::decode_from_slice(&serialized_data, bincode_config).map_err(|e| {
                Error::Proof(ProofError::CorruptedProof(format!(
                    "cannot decode compacted address balances: {}",
                    e
                )))
            })?;

            compacted_changes.push((range_start, range_end, address_balances));
        }

        Ok((root_hash, compacted_changes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::address_funds::PlatformAddress;
    use dpp::balances::credits::CreditOperation;
    use platform_version::version::PlatformVersion;
    use std::collections::BTreeMap;

    #[test]
    fn should_prove_and_verify_compacted_address_balance_changes_roundtrip() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let address_1 = PlatformAddress::P2pkh([10; 20]);
        let address_2 = PlatformAddress::P2sh([11; 20]);

        // Store enough blocks of changes to trigger compaction
        // The compaction threshold is controlled by platform_version
        // We insert many blocks to ensure compaction happens
        let max_blocks = platform_version
            .drive
            .methods
            .saved_block_transactions
            .max_blocks_before_compaction as u64;

        for block_height in 1u64..=(max_blocks + 1) {
            let mut changes = BTreeMap::new();
            changes.insert(
                address_1,
                CreditOperation::AddToCredits(block_height * 1000),
            );
            if block_height % 2 == 0 {
                changes.insert(address_2, CreditOperation::AddToCredits(block_height * 500));
            }
            drive
                .store_address_balances_for_block(
                    &changes,
                    block_height,
                    block_height * 1000,
                    None,
                    platform_version,
                )
                .expect("should store balances");
        }

        // Prove compacted changes from block 1
        let proof = drive
            .prove_compacted_address_balance_changes(1, None, None, platform_version)
            .expect("should prove compacted address balance changes");

        // Verify the proof
        let (root_hash, compacted_changes) = Drive::verify_compacted_address_balance_changes(
            proof.as_slice(),
            1,
            None,
            platform_version,
        )
        .expect("should verify proof");

        assert!(!root_hash.is_empty(), "root hash should not be empty");
        // We should have at least one compacted entry
        assert!(
            !compacted_changes.is_empty(),
            "should have at least one compacted entry"
        );

        // All entries should have valid block ranges
        for (start, end, changes) in &compacted_changes {
            assert!(*start <= *end, "start should be <= end");
            assert!(!changes.is_empty(), "each entry should have changes");
        }
    }

    #[test]
    fn should_prove_and_verify_empty_compacted_address_balance_changes() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // Prove compacted changes from block 100 with no data stored
        let proof = drive
            .prove_compacted_address_balance_changes(100, None, None, platform_version)
            .expect("should prove empty compacted changes");

        // Verify the proof
        let (root_hash, compacted_changes) = Drive::verify_compacted_address_balance_changes(
            proof.as_slice(),
            100,
            None,
            platform_version,
        )
        .expect("should verify proof");

        assert!(!root_hash.is_empty(), "root hash should not be empty");
        assert!(
            compacted_changes.is_empty(),
            "should have no compacted entries when no data stored"
        );
    }

    #[test]
    fn test_verify_compacted_address_balance_changes_proof() {
        // This proof was generated with start_block_height = 329
        // Path: [[36], [99]] = [['$'], ['c']] = SavedBlockTransactions -> CompactedAddressBalances
        // Query: RangeTo(..[0, 0, 0, 0, 0, 0, 1, 73, 0, 0, 0, 0, 0, 0, 1, 73]) = RangeTo(..(329, 329))
        let proof: Vec<u8> = vec![
            0, 251, 1, 24, 1, 24, 215, 122, 183, 233, 12, 7, 14, 37, 119, 175, 74, 229, 6, 76, 88,
            209, 79, 175, 135, 230, 45, 89, 116, 17, 76, 49, 87, 242, 4, 40, 35, 2, 33, 229, 84,
            218, 198, 132, 136, 197, 212, 253, 180, 247, 125, 228, 176, 179, 248, 100, 19, 202,
            143, 20, 196, 132, 112, 114, 44, 43, 11, 174, 49, 96, 16, 4, 1, 36, 0, 5, 2, 1, 1, 101,
            0, 126, 152, 206, 30, 157, 147, 101, 232, 212, 68, 50, 242, 52, 170, 136, 72, 39, 136,
            235, 41, 105, 28, 199, 93, 222, 168, 227, 241, 80, 234, 2, 185, 2, 120, 2, 233, 247,
            175, 89, 203, 114, 37, 58, 187, 95, 169, 151, 247, 245, 82, 228, 73, 77, 131, 5, 100,
            241, 7, 152, 139, 156, 94, 138, 33, 173, 16, 2, 124, 156, 122, 25, 79, 47, 39, 233, 96,
            90, 9, 165, 232, 130, 103, 245, 113, 145, 31, 183, 102, 94, 40, 86, 140, 146, 128, 172,
            85, 184, 13, 220, 16, 1, 48, 30, 57, 216, 246, 121, 172, 45, 163, 129, 189, 9, 238,
            108, 64, 21, 231, 140, 164, 160, 37, 184, 182, 34, 151, 148, 194, 74, 83, 124, 199, 87,
            17, 17, 2, 199, 32, 12, 6, 52, 58, 219, 92, 42, 60, 69, 132, 209, 186, 193, 248, 18,
            33, 26, 78, 216, 110, 114, 103, 202, 56, 45, 175, 163, 68, 139, 6, 16, 1, 62, 159, 56,
            33, 169, 193, 143, 7, 131, 179, 169, 31, 163, 163, 50, 117, 235, 211, 94, 247, 244, 60,
            246, 149, 186, 142, 105, 187, 230, 247, 212, 141, 17, 1, 1, 36, 125, 4, 1, 99, 0, 20,
            2, 1, 16, 0, 0, 0, 0, 0, 0, 1, 4, 0, 0, 0, 0, 0, 0, 1, 8, 0, 97, 206, 71, 247, 121, 93,
            103, 7, 209, 119, 82, 59, 209, 145, 171, 254, 112, 14, 204, 81, 30, 98, 213, 203, 146,
            141, 32, 167, 232, 34, 0, 37, 2, 170, 200, 228, 36, 229, 197, 116, 242, 100, 137, 25,
            37, 45, 57, 56, 2, 38, 8, 75, 144, 250, 71, 108, 90, 106, 133, 2, 231, 236, 42, 149,
            206, 16, 1, 77, 52, 100, 69, 203, 109, 100, 190, 84, 2, 6, 238, 168, 74, 208, 99, 16,
            56, 200, 98, 181, 205, 24, 79, 120, 235, 223, 144, 61, 197, 8, 215, 17, 1, 1, 99, 220,
            1, 28, 113, 173, 87, 207, 150, 171, 166, 221, 201, 207, 122, 14, 62, 119, 8, 5, 100,
            182, 50, 112, 191, 244, 5, 125, 9, 161, 17, 66, 201, 126, 148, 2, 170, 62, 172, 42,
            117, 12, 62, 165, 78, 141, 84, 194, 75, 135, 140, 198, 85, 219, 214, 218, 149, 56, 32,
            251, 16, 134, 21, 44, 31, 22, 80, 69, 16, 1, 191, 139, 156, 120, 188, 0, 187, 111, 169,
            41, 135, 146, 156, 103, 102, 125, 235, 0, 70, 180, 94, 103, 134, 250, 135, 56, 55, 144,
            156, 185, 212, 67, 2, 223, 93, 226, 244, 94, 203, 160, 13, 145, 161, 22, 104, 133, 135,
            132, 239, 27, 61, 12, 167, 134, 237, 120, 58, 229, 208, 50, 55, 70, 139, 211, 234, 16,
            2, 58, 253, 249, 36, 12, 24, 122, 198, 7, 161, 13, 237, 229, 3, 224, 98, 176, 51, 237,
            101, 105, 33, 10, 45, 111, 96, 89, 201, 212, 82, 1, 141, 5, 16, 0, 0, 0, 0, 0, 0, 1,
            32, 0, 0, 0, 0, 0, 0, 1, 36, 77, 228, 149, 252, 55, 96, 22, 145, 235, 199, 188, 83, 13,
            88, 13, 241, 48, 191, 78, 152, 20, 50, 252, 186, 199, 105, 134, 73, 62, 136, 228, 196,
            17, 17, 17, 0, 1,
        ];

        let start_block_height = 329u64;
        let platform_version = PlatformVersion::latest();

        let result = Drive::verify_compacted_address_balance_changes(
            &proof,
            start_block_height,
            None,
            platform_version,
        );

        assert!(
            result.is_ok(),
            "proof verification failed: {:?}",
            result.err()
        );

        let (root_hash, compacted_changes) = result.unwrap();

        // Verify we got a valid root hash
        assert!(!root_hash.is_empty(), "root hash should not be empty");

        // The proof shows entry (288, 292) is the rightmost in the tree.
        // Since 292 < 329 (our start_block_height), there are no results.
        // The KVDigest at the boundary proves nothing exists >= (329, 329).
        assert!(
            compacted_changes.is_empty(),
            "expected empty results since start_block_height 329 > last entry end_block 292"
        );

        // Log what we found for debugging
        eprintln!("Root hash: {:?}", root_hash);
        eprintln!("Number of compacted entries: {}", compacted_changes.len());
        for (start, end, changes) in &compacted_changes {
            eprintln!(
                "  Blocks {}-{}: {} address changes",
                start,
                end,
                changes.len()
            );
        }
    }
}
