use crate::drive::Drive;
use crate::drive::RootTree;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::address_funds::PlatformAddress;

/// The subtree key for compacted address balances storage as u8
const COMPACTED_ADDRESS_BALANCES_KEY_U8: u8 = b'c';
/// Decode budget for the legacy single GroveDB proof. Mirrors the v1
/// envelope budget: far above any realistic proof while bounding hostile
/// allocations before GroveDB verification runs.
const MAX_COMPACTED_PROOF_DECODE_BYTES: usize = 16 * 1024 * 1024;
/// A compacted row contains at most one configured address chunk. Keep a
/// separate semantic-object budget after the GroveDB proof is decoded.
const MAX_COMPACTED_BALANCE_ROW_DECODE_BYTES: usize = 1024 * 1024;
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
    /// Verifies compacted address balance changes proof — legacy single-proof
    /// wire format used by protocol versions whose
    /// `verify_compacted_address_balance_changes` feature version is 0.
    ///
    /// This verification works by:
    /// 1. Decoding the GroveDBProof structure
    /// 2. Navigating to the compacted address balances layer ('c')
    /// 3. Extracting KV entries from the merk proof
    /// 4. Filtering entries where the key range contains start_block_height
    /// 5. Verifying the root hash using a subset query
    ///
    /// Kept byte-for-byte wire-compatible with the proofs emitted by
    /// `prove_compacted_address_balance_changes_v0`, so nodes and clients on
    /// pre-envelope protocol versions keep interoperating. Its start key is
    /// derived from proof-carried (not independently verified) KV entries —
    /// the binding soundness fix requires the two-proof envelope and
    /// therefore lives in v1; only decode-allocation bounds are applied
    /// here, as they do not change the accepted format.
    pub(super) fn verify_compacted_address_balance_changes_v0(
        proof: &[u8],
        start_block_height: u64,
        limit: Option<u16>,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, VerifiedCompactedAddressBalanceChanges), Error> {
        if proof.len() > MAX_COMPACTED_PROOF_DECODE_BYTES {
            return Err(Error::Proof(ProofError::CorruptedProof(
                "compacted address balance proof exceeds the decoding limit".to_string(),
            )));
        }

        let bincode_config = bincode::config::standard()
            .with_big_endian()
            .with_limit::<MAX_COMPACTED_PROOF_DECODE_BYTES>();

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

            // Deserialize the address balance map within its bounded budget
            if serialized_data.len() > MAX_COMPACTED_BALANCE_ROW_DECODE_BYTES {
                return Err(Error::Proof(ProofError::CorruptedProof(
                    "compacted address balance row exceeds the decoding limit".to_string(),
                )));
            }
            let row_decode_config = bincode::config::standard()
                .with_big_endian()
                .with_limit::<MAX_COMPACTED_BALANCE_ROW_DECODE_BYTES>();
            let (address_balances, _): (
                BTreeMap<PlatformAddress, BlockAwareCreditOperation>,
                usize,
            ) = bincode::decode_from_slice(&serialized_data, row_decode_config).map_err(|e| {
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

    /// Latest protocol version still on the legacy (v0) single-proof wire
    /// format for compacted address balance proofs.
    fn legacy_platform_version() -> &'static PlatformVersion {
        let version = PlatformVersion::get(12).expect("protocol version 12 must exist");
        assert_eq!(
            version
                .drive
                .methods
                .verify
                .address_funds
                .verify_compacted_address_balance_changes,
            0,
            "protocol version 12 must still use the legacy proof format"
        );
        version
    }

    #[test]
    fn should_prove_and_verify_legacy_compacted_address_balance_changes_roundtrip() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = legacy_platform_version();

        let address_1 = PlatformAddress::P2pkh([10; 20]);
        let address_2 = PlatformAddress::P2sh([11; 20]);

        // Store enough blocks of changes to trigger compaction
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

        // Prove compacted changes from block 1 — dispatches to the legacy
        // single-proof prover on this protocol version.
        let proof = drive
            .prove_compacted_address_balance_changes(1, None, None, platform_version)
            .expect("should prove compacted address balance changes");

        // Verify the proof — dispatches to the legacy verifier.
        let (root_hash, compacted_changes) = Drive::verify_compacted_address_balance_changes(
            proof.as_slice(),
            1,
            None,
            platform_version,
        )
        .expect("should verify proof");

        assert!(!root_hash.is_empty(), "root hash should not be empty");
        assert!(
            !compacted_changes.is_empty(),
            "should have at least one compacted entry"
        );

        for (start, end, changes) in &compacted_changes {
            assert!(*start <= *end, "start should be <= end");
            assert!(!changes.is_empty(), "each entry should have changes");
        }
    }

    #[test]
    fn should_prove_and_verify_empty_legacy_compacted_address_balance_changes() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = legacy_platform_version();

        let proof = drive
            .prove_compacted_address_balance_changes(100, None, None, platform_version)
            .expect("should prove empty compacted changes");

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

    /// Cross-format guard: an envelope proof produced under a protocol
    /// version using the v1 two-proof format must fail closed when verified
    /// under a legacy protocol version, rather than being misinterpreted.
    #[test]
    fn envelope_proof_is_rejected_by_legacy_verifier() {
        let drive = setup_drive_with_initial_state_structure(None);
        let envelope_version = PlatformVersion::latest();
        assert_eq!(
            envelope_version
                .drive
                .methods
                .verify
                .address_funds
                .verify_compacted_address_balance_changes,
            1,
            "latest protocol version must use the envelope proof format"
        );

        let proof = drive
            .prove_compacted_address_balance_changes(100, None, None, envelope_version)
            .expect("should prove with the envelope format");

        let result = Drive::verify_compacted_address_balance_changes(
            proof.as_slice(),
            100,
            None,
            legacy_platform_version(),
        );

        assert!(
            result.is_err(),
            "an envelope proof must not verify under the legacy format"
        );
    }
}
