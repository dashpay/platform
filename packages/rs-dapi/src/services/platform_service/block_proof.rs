//! The proofs `getBlocks` and `getOwnerTransactionsInBlock` return when asked to prove.
//!
//! Tenderdash already commits to a block's contents: the transactions and their results
//! are Merkle trees whose roots are in the block header, and the quorum signs the header.
//! Nothing here is new consensus data, only what a client needs to follow that chain.
//!
//! Every tree is rebuilt here and compared with the root in the header, so a node never
//! hands out a proof that its own data cannot satisfy.

use chrono::DateTime;
use dapi_grpc::Message;
use dapi_grpc::platform::proto::abci::ExecTxResult;
use dapi_grpc::platform::v0::tenderdash_block_proof::{BlockId, Header};
use dapi_grpc::platform::v0::{TenderdashBlockProof, TenderdashTransactionProof};
use sha2::{Digest, Sha256};

use super::error_mapping::base64_decode;
use super::get_blocks::decode_hex;
use crate::clients::tenderdash_client::{
    BlockHeader, BlockId as TenderdashBlockId, BlockTxResult, ResultCommit,
};
use crate::error::{DAPIResult, DapiError};

const LEAF_PREFIX: [u8; 1] = [0];
const INNER_PREFIX: [u8; 1] = [1];

fn leaf_hash(leaf: &[u8]) -> Vec<u8> {
    Sha256::new()
        .chain_update(LEAF_PREFIX)
        .chain_update(leaf)
        .finalize()
        .to_vec()
}

fn inner_hash(left: &[u8], right: &[u8]) -> Vec<u8> {
    Sha256::new()
        .chain_update(INNER_PREFIX)
        .chain_update(left)
        .chain_update(right)
        .finalize()
        .to_vec()
}

/// Tenderdash's Merkle tree (RFC 6962), with the paths of the selected leaves.
#[derive(Debug)]
struct MerkleTree {
    root: Vec<u8>,
    /// For each selected leaf, the hashes from its sibling up to a child of the root.
    paths: Vec<Vec<Vec<u8>>>,
}

impl MerkleTree {
    /// Build the tree over `leaves`, keeping paths only for the positions in `selected`,
    /// which must be ascending and within `leaves`.
    fn new<T: AsRef<[u8]>>(leaves: &[T], selected: &[usize]) -> Self {
        Self::build(leaves, 0, selected)
    }

    fn build<T: AsRef<[u8]>>(leaves: &[T], first: usize, selected: &[usize]) -> Self {
        if selected.is_empty() {
            return Self {
                root: merkle_root(leaves),
                paths: Vec::new(),
            };
        }
        if leaves.len() < 2 {
            return Self {
                root: merkle_root(leaves),
                paths: vec![Vec::new(); selected.len()],
            };
        }

        let split = split_point(leaves.len());
        let (left_leaves, right_leaves) = leaves.split_at(split);
        let (left_selected, right_selected) =
            selected.split_at(selected.partition_point(|&position| position < first + split));
        let left = Self::build(left_leaves, first, left_selected);
        let right = Self::build(right_leaves, first + split, right_selected);

        let root = inner_hash(&left.root, &right.root);
        let paths = left
            .paths
            .into_iter()
            .map(|mut path| {
                path.push(right.root.clone());
                path
            })
            .chain(right.paths.into_iter().map(|mut path| {
                path.push(left.root.clone());
                path
            }))
            .collect();
        Self { root, paths }
    }
}

/// The root of the same tree, for callers that need no paths.
fn merkle_root<T: AsRef<[u8]>>(leaves: &[T]) -> Vec<u8> {
    match leaves {
        [] => Sha256::digest([]).to_vec(),
        [leaf] => leaf_hash(leaf.as_ref()),
        _ => {
            let (left, right) = leaves.split_at(split_point(leaves.len()));
            inner_hash(&merkle_root(left), &merkle_root(right))
        }
    }
}

/// The largest power of two below `length`, which must be at least 2.
fn split_point(length: usize) -> usize {
    1 << (usize::BITS - 1 - (length - 1).leading_zeros())
}

/// Tenderdash hashes only these fields of an execution result into `results_hash`.
fn encode_result(code: u32, data: Vec<u8>, gas_used: i64) -> Vec<u8> {
    ExecTxResult {
        code,
        data,
        gas_used,
        ..Default::default()
    }
    .encode_to_vec()
}

fn encode_results(results: &[BlockTxResult]) -> DAPIResult<Vec<Vec<u8>>> {
    results
        .iter()
        .enumerate()
        .map(|(index, result)| {
            let data = base64_decode(&result.data).ok_or_else(|| {
                DapiError::Internal(format!("result {index} of the block is not base64"))
            })?;
            Ok(encode_result(result.code, data, result.gas_used))
        })
        .collect()
}

fn check_root(name: &str, root: &[u8], header_root: &[u8]) -> DAPIResult<()> {
    if root != header_root {
        return Err(DapiError::Internal(format!(
            "the block's {name} do not hash to the root in its header"
        )));
    }
    Ok(())
}

/// Check that a block's contents are the ones its header commits to. `results` is empty
/// when the block has no transactions, and then nothing was read to compare.
pub(super) fn check_block_contents(
    proof: &TenderdashBlockProof,
    transaction_hashes: &[Vec<u8>],
    results: &[BlockTxResult],
) -> DAPIResult<()> {
    let header = proof_header(proof)?;
    check_root(
        "transactions",
        &merkle_root(transaction_hashes),
        &header.data_hash,
    )?;
    if !results.is_empty() {
        check_root(
            "results",
            &merkle_root(&encode_results(results)?),
            &header.results_hash,
        )?;
    }
    Ok(())
}

/// The Merkle paths of the transactions at `indexes`. `results` is the whole block's, or
/// empty when `indexes` is, since the results are read only for transactions returned.
pub(super) fn transaction_proofs(
    proof: &TenderdashBlockProof,
    block_txs: &[String],
    results: &[BlockTxResult],
    indexes: &[usize],
) -> DAPIResult<Vec<TenderdashTransactionProof>> {
    let header = proof_header(proof)?;
    let hashes = block_txs
        .iter()
        .enumerate()
        .map(|(index, tx)| {
            base64_decode(tx)
                .map(|bytes| Sha256::digest(bytes).to_vec())
                .ok_or_else(|| {
                    DapiError::Internal(format!("transaction {index} of the block is not base64"))
                })
        })
        .collect::<DAPIResult<Vec<_>>>()?;
    if indexes.is_empty() {
        check_root("transactions", &merkle_root(&hashes), &header.data_hash)?;
        return Ok(Vec::new());
    }
    let transactions = MerkleTree::new(&hashes, indexes);
    check_root("transactions", &transactions.root, &header.data_hash)?;
    let results = MerkleTree::new(&encode_results(results)?, indexes);
    check_root("results", &results.root, &header.results_hash)?;
    let total = u32::try_from(block_txs.len())
        .map_err(|_| DapiError::Internal("the block holds too many transactions".to_string()))?;

    indexes
        .iter()
        .zip(transactions.paths)
        .zip(results.paths)
        .map(|((&index, transaction_path), result_path)| {
            Ok(TenderdashTransactionProof {
                index: u32::try_from(index).map_err(|_| {
                    DapiError::Internal("the block holds too many transactions".to_string())
                })?,
                total,
                transaction_path,
                result_path,
            })
        })
        .collect()
}

fn proof_header(proof: &TenderdashBlockProof) -> DAPIResult<&Header> {
    proof
        .header
        .as_ref()
        .ok_or_else(|| DapiError::Internal("block proof without a header".to_string()))
}

fn convert_block_id(block_id: &TenderdashBlockId) -> DAPIResult<BlockId> {
    Ok(BlockId {
        hash: decode_hex("block id hash", &block_id.hash)?,
        part_set_total: block_id.parts.total,
        part_set_hash: decode_hex("part set hash", &block_id.parts.hash)?,
        state_id: decode_hex("state id", &block_id.state_id)?,
    })
}

fn convert_header(header: BlockHeader) -> DAPIResult<Header> {
    let timestamp = DateTime::parse_from_rfc3339(&header.time)
        .map_err(|error| DapiError::Internal(format!("block time {:?}: {error}", header.time)))?;
    let time_nanos = i32::try_from(timestamp.timestamp_subsec_nanos())
        .map_err(|_| DapiError::Internal(format!("block time {:?}", header.time)))?;

    Ok(Header {
        block_version: header.version.block,
        app_version: header.version.app,
        chain_id: header.chain_id,
        height: header.height,
        core_chain_locked_height: header.core_chain_locked_height,
        time_seconds: timestamp.timestamp(),
        time_nanos,
        last_block_id: Some(convert_block_id(&header.last_block_id)?),
        last_commit_hash: decode_hex("last_commit_hash", &header.last_commit_hash)?,
        data_hash: decode_hex("data_hash", &header.data_hash)?,
        validators_hash: decode_hex("validators_hash", &header.validators_hash)?,
        next_validators_hash: decode_hex("next_validators_hash", &header.next_validators_hash)?,
        consensus_hash: decode_hex("consensus_hash", &header.consensus_hash)?,
        next_consensus_hash: decode_hex("next_consensus_hash", &header.next_consensus_hash)?,
        app_hash: decode_hex("app_hash", &header.app_hash)?,
        results_hash: decode_hex("results_hash", &header.results_hash)?,
        evidence_hash: decode_hex("evidence_hash", &header.evidence_hash)?,
        proposer_pro_tx_hash: decode_hex("proposer_pro_tx_hash", &header.proposer_pro_tx_hash)?,
        proposed_app_version: header.proposed_protocol_version,
    })
}

/// Build the proof of the block at `height`, whose id the node reported as `block_id`.
pub(super) fn build_block_proof(
    height: u64,
    block_id: &TenderdashBlockId,
    commit: ResultCommit,
    quorum_type: u32,
) -> DAPIResult<TenderdashBlockProof> {
    let ResultCommit {
        signed_header: signed,
    } = commit;

    let block_hash = decode_hex("hash", &block_id.hash)?;
    let signed_hash = decode_hex("hash", &signed.commit.block_id.hash)?;
    if signed.header.height != height || signed_hash != block_hash {
        return Err(DapiError::Internal(format!(
            "the commit Tenderdash returned for height {height} is for another block"
        )));
    }
    let signature = base64_decode(&signed.commit.threshold_block_signature).ok_or_else(|| {
        DapiError::Internal(format!("the signature of block {height} is not base64"))
    })?;

    Ok(TenderdashBlockProof {
        block_id: Some(convert_block_id(&signed.commit.block_id)?),
        round: signed.commit.round,
        quorum_hash: decode_hex("quorum_hash", &signed.commit.quorum_hash)?,
        quorum_type,
        signature,
        header: Some(convert_header(signed.header)?),
    })
}

/// A `commit` result in the shape Tenderdash's RPC serves, whose header commits to these
/// transaction hashes and results.
#[cfg(test)]
pub(super) fn tenderdash_commit(
    height: u64,
    block_hash: &str,
    transaction_hashes: &[Vec<u8>],
    results: &[BlockTxResult],
) -> ResultCommit {
    let results_root = merkle_root(&encode_results(results).unwrap());
    serde_json::from_value(serde_json::json!({
        "signed_header": {
            "header": {
                "version": { "block": "14", "app": "11" },
                "chain_id": "dash-1",
                "height": height.to_string(),
                "core_chain_locked_height": 2300000,
                "time": "2024-07-29T15:00:00.123456789Z",
                "last_block_id": {
                    "hash": "AA",
                    "parts": { "total": 2, "hash": "BB" },
                    "state_id": "CC",
                },
                "last_commit_hash": "01",
                "data_hash": hex::encode_upper(merkle_root(transaction_hashes)),
                "validators_hash": "03",
                "next_validators_hash": "04",
                "consensus_hash": "05",
                "next_consensus_hash": "06",
                "app_hash": "07",
                "results_hash": hex::encode_upper(results_root),
                "evidence_hash": "09",
                "proposer_pro_tx_hash": "0A",
                "proposed_protocol_version": 15,
            },
            "commit": {
                "height": height,
                "round": 3,
                "block_id": {
                    "hash": block_hash,
                    "parts": { "total": 1, "hash": "DD" },
                    "state_id": "EE",
                },
                "quorum_hash": "0B0C",
                "threshold_block_signature": "AQID",
                "threshold_vote_extensions": [],
            },
        },
        "canonical": true,
    }))
    .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::tenderdash_client::BlockId as TenderdashBlockId;
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;

    /// `merkle.ProofsFromByteSlices` of Tenderdash's `crypto/merkle` over `leaf-0`..`leaf-6`.
    const TENDERDASH_ROOT: &str =
        "0b007fb915eb9b2a146f54b1c86ec53b664f8e455b7660b0b6ee13edc0d921c0";
    const TENDERDASH_PATHS: [&[&str]; 7] = [
        &[
            "3145c409f259b7c53e32036090ff76751025a2498ba9823ef718cac50b4e616f",
            "bd45ff28796704d88bdac51b1df553fda59837b616d6d1cb2114dbc3b087ff69",
            "8eae6bd3b3a07f1f75ee72a531629e6eb31e42e62f760e47de52a53c3641ef23",
        ],
        &[
            "305df59f9590c3c9ac63d2b2743c388e3792449078cebf7fb3dbe6471643b2b7",
            "bd45ff28796704d88bdac51b1df553fda59837b616d6d1cb2114dbc3b087ff69",
            "8eae6bd3b3a07f1f75ee72a531629e6eb31e42e62f760e47de52a53c3641ef23",
        ],
        &[
            "f76836325aec5699d8d71f8e42e9d47c5c29b08059ba296384f7ca40ad3a40ae",
            "60a53eed0de87a90c8e59427c59c46253c33a76a09502a51801300927b7e6bdc",
            "8eae6bd3b3a07f1f75ee72a531629e6eb31e42e62f760e47de52a53c3641ef23",
        ],
        &[
            "fca89f57c9f8c8eb4047a7ff9d333acf9e0f3384b20b255bceab0f216dcca267",
            "60a53eed0de87a90c8e59427c59c46253c33a76a09502a51801300927b7e6bdc",
            "8eae6bd3b3a07f1f75ee72a531629e6eb31e42e62f760e47de52a53c3641ef23",
        ],
        &[
            "8f1593cb92f429d9340b9bbc1f0bb122adf8026c42a4a42142e2168931727236",
            "676f3782f5b3a5fb4370ed49572cedc523f4a66322269c85f2af0509d17b0a4d",
            "bdd1c5ff55b19cb6b0e7c761bf9a6ccaa27fbbfc07b74f1fabb6e911a0bd2ab3",
        ],
        &[
            "ea9fc1a1b6e191b460d0d6306e3e870c173f39330f13cda1b70cfc72bdc398ba",
            "676f3782f5b3a5fb4370ed49572cedc523f4a66322269c85f2af0509d17b0a4d",
            "bdd1c5ff55b19cb6b0e7c761bf9a6ccaa27fbbfc07b74f1fabb6e911a0bd2ab3",
        ],
        &[
            "985bb5d36b927800876871da925a7e82abe83a9ddba5882920a007a55ea2b376",
            "bdd1c5ff55b19cb6b0e7c761bf9a6ccaa27fbbfc07b74f1fabb6e911a0bd2ab3",
        ],
    ];

    fn hex_of(value: &[u8]) -> String {
        hex::encode(value)
    }

    /// Follow a path up from a leaf hash, the way a client checks one.
    fn root_from_path(index: usize, total: usize, leaf: Vec<u8>, path: &[Vec<u8>]) -> Vec<u8> {
        if total <= 1 {
            assert!(path.is_empty());
            return leaf;
        }
        let (aunt, rest) = path.split_last().unwrap();
        let split = split_point(total);
        if index < split {
            inner_hash(&root_from_path(index, split, leaf, rest), aunt)
        } else {
            inner_hash(
                aunt,
                &root_from_path(index - split, total - split, leaf, rest),
            )
        }
    }

    fn block_id(hash: &str) -> TenderdashBlockId {
        TenderdashBlockId {
            hash: hash.to_string(),
            ..Default::default()
        }
    }

    fn sample_txs() -> Vec<String> {
        (0u8..5).map(|byte| STANDARD.encode([byte; 3])).collect()
    }

    fn sample_hashes() -> Vec<Vec<u8>> {
        (0u8..5)
            .map(|byte| Sha256::digest([byte; 3]).to_vec())
            .collect()
    }

    fn sample_results() -> Vec<BlockTxResult> {
        (0..5)
            .map(|index| BlockTxResult {
                code: index * 100,
                data: String::new(),
                gas_used: i64::from(index) + 1,
            })
            .collect()
    }

    fn sample_proof() -> TenderdashBlockProof {
        let commit = tenderdash_commit(7, "ABCD", &sample_hashes(), &sample_results());
        build_block_proof(7, &block_id("ABCD"), commit, 6).unwrap()
    }

    #[test]
    fn should_build_the_same_tree_as_tenderdash() {
        let leaves = (0..7)
            .map(|index| format!("leaf-{index}").into_bytes())
            .collect::<Vec<_>>();

        let tree = MerkleTree::new(&leaves, &(0..leaves.len()).collect::<Vec<_>>());

        assert_eq!(hex_of(&tree.root), TENDERDASH_ROOT);
        for (path, expected) in tree.paths.iter().zip(TENDERDASH_PATHS) {
            assert_eq!(path.iter().map(|h| hex_of(h)).collect::<Vec<_>>(), expected);
        }
    }

    #[test]
    fn should_hash_edge_trees_like_rfc_6962() {
        assert_eq!(
            hex_of(&merkle_root::<Vec<u8>>(&[])),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            hex_of(&merkle_root(&[Vec::new()])),
            "6e340b9cffb37a989ca544e6bb780a2c78901d3fb33738768511a30617afa01d"
        );
        assert_eq!(
            hex_of(&merkle_root(&[b"L123456".to_vec()])),
            "395aa064aa4c29f7010acfe3f25db9485bbd4b91897b6ad7ad547639252b4d56"
        );
        assert_eq!(
            hex_of(&inner_hash(b"N123", b"N456")),
            "aa217fe888e47007fa15edab33c2b492a722cb106c64667fc2b044444de66bbb"
        );
    }

    #[test]
    fn should_compute_the_same_root_without_building_paths() {
        for total in 0..=33usize {
            let leaves = (0..total)
                .map(|index| index.to_be_bytes().to_vec())
                .collect::<Vec<_>>();
            let all = (0..total).collect::<Vec<_>>();

            assert_eq!(merkle_root(&leaves), MerkleTree::new(&leaves, &all).root);
        }
    }

    #[test]
    fn should_build_only_the_selected_paths() {
        let leaves = (0..7)
            .map(|index| format!("leaf-{index}").into_bytes())
            .collect::<Vec<_>>();

        for selected in [vec![], vec![0], vec![6], vec![1, 4], vec![2, 3, 5]] {
            let tree = MerkleTree::new(&leaves, &selected);

            assert_eq!(hex_of(&tree.root), TENDERDASH_ROOT);
            assert_eq!(tree.paths.len(), selected.len());
            for (path, position) in tree.paths.iter().zip(&selected) {
                assert_eq!(
                    path.iter().map(|h| hex_of(h)).collect::<Vec<_>>(),
                    TENDERDASH_PATHS[*position]
                );
            }
        }
    }

    #[test]
    fn should_give_every_leaf_a_path_to_the_root() {
        for total in 1..=33usize {
            let leaves = (0..total)
                .map(|index| index.to_be_bytes().to_vec())
                .collect::<Vec<_>>();
            let tree = MerkleTree::new(&leaves, &(0..total).collect::<Vec<_>>());

            for (index, leaf) in leaves.iter().enumerate() {
                let computed = root_from_path(index, total, leaf_hash(leaf), &tree.paths[index]);
                assert_eq!(computed, tree.root, "leaf {index} of {total}");
            }
        }
    }

    #[test]
    fn should_encode_a_result_as_tenderdash_does() {
        // code = 1 (varint), data = 2 (bytes), gas_used = 6 (varint)
        assert_eq!(
            encode_result(40100, vec![1, 2], 9),
            [0x08, 0xa4, 0xb9, 0x02, 0x12, 0x02, 0x01, 0x02, 0x30, 0x09]
        );
        // zero values are not encoded
        assert_eq!(encode_result(0, Vec::new(), 0), Vec::<u8>::new());
    }

    #[test]
    fn should_carry_every_field_of_the_signed_header() {
        let proof = sample_proof();
        let header = proof.header.as_ref().unwrap();
        let block_id = proof.block_id.as_ref().unwrap();
        let last = header.last_block_id.as_ref().unwrap();

        assert_eq!((header.block_version, header.app_version), (14, 11));
        assert_eq!(header.chain_id, "dash-1");
        assert_eq!(header.height, 7);
        assert_eq!(header.core_chain_locked_height, 2_300_000);
        assert_eq!(header.time_seconds, 1_722_265_200);
        assert_eq!(header.time_nanos, 123_456_789);
        assert_eq!(
            (
                last.hash.as_slice(),
                last.part_set_total,
                last.part_set_hash.as_slice()
            ),
            ([0xaa].as_slice(), 2, [0xbb].as_slice())
        );
        assert_eq!(last.state_id, [0xcc]);
        assert_eq!(header.last_commit_hash, [1]);
        assert_eq!(header.validators_hash, [3]);
        assert_eq!(header.next_validators_hash, [4]);
        assert_eq!(header.consensus_hash, [5]);
        assert_eq!(header.next_consensus_hash, [6]);
        assert_eq!(header.app_hash, [7]);
        assert_eq!(header.evidence_hash, [9]);
        assert_eq!(header.proposer_pro_tx_hash, [10]);
        assert_eq!(header.proposed_app_version, 15);

        assert_eq!(block_id.hash, [0xab, 0xcd]);
        assert_eq!(block_id.part_set_total, 1);
        assert_eq!(block_id.part_set_hash, [0xdd]);
        assert_eq!(block_id.state_id, [0xee]);
        assert_eq!(proof.round, 3);
        assert_eq!(proof.quorum_hash, [0x0b, 0x0c]);
        assert_eq!(proof.quorum_type, 6);
        assert_eq!(proof.signature, [1, 2, 3]);
    }

    #[test]
    fn should_reject_a_commit_for_another_block() {
        let commit = tenderdash_commit(7, "ABCD", &[], &[]);
        assert!(matches!(
            build_block_proof(7, &block_id("ABCE"), commit, 6),
            Err(DapiError::Internal(_))
        ));

        let commit = tenderdash_commit(8, "ABCD", &[], &[]);
        assert!(matches!(
            build_block_proof(7, &block_id("ABCD"), commit, 6),
            Err(DapiError::Internal(_))
        ));
    }

    #[test]
    fn should_prove_the_requested_transactions_and_their_results() {
        let proof = sample_proof();
        let header = proof.header.as_ref().unwrap();
        let results = sample_results();

        let proofs = transaction_proofs(&proof, &sample_txs(), &results, &[1, 4]).unwrap();

        assert_eq!(proofs.len(), 2);
        for (transaction, index) in proofs.iter().zip([1usize, 4]) {
            assert_eq!(transaction.index as usize, index);
            assert_eq!(transaction.total, 5);
            assert_eq!(
                root_from_path(
                    index,
                    5,
                    leaf_hash(&sample_hashes()[index]),
                    &transaction.transaction_path,
                ),
                header.data_hash
            );
            let encoded = encode_result(results[index].code, Vec::new(), results[index].gas_used);
            assert_eq!(
                root_from_path(index, 5, leaf_hash(&encoded), &transaction.result_path),
                header.results_hash
            );
        }
    }

    #[test]
    fn should_return_no_paths_when_no_transaction_was_selected() {
        let proofs = transaction_proofs(&sample_proof(), &sample_txs(), &[], &[]).unwrap();

        assert!(proofs.is_empty());
    }

    #[test]
    fn should_refuse_a_proof_its_transactions_do_not_satisfy() {
        let mut txs = sample_txs();
        txs[0] = STANDARD.encode([9; 3]);

        assert!(matches!(
            transaction_proofs(&sample_proof(), &txs, &sample_results(), &[0]),
            Err(DapiError::Internal(_))
        ));
    }

    #[test]
    fn should_refuse_a_proof_its_results_do_not_satisfy() {
        let mut results = sample_results();
        results[0].gas_used += 1;

        assert!(matches!(
            transaction_proofs(&sample_proof(), &sample_txs(), &results, &[0]),
            Err(DapiError::Internal(_))
        ));
        assert!(matches!(
            check_block_contents(&sample_proof(), &sample_hashes(), &results),
            Err(DapiError::Internal(_))
        ));
    }

    #[test]
    fn should_accept_a_block_whose_contents_match_its_header() {
        check_block_contents(&sample_proof(), &sample_hashes(), &sample_results()).unwrap();
    }

    #[test]
    fn should_not_compare_results_of_a_block_without_transactions() {
        let commit = tenderdash_commit(7, "ABCD", &[], &[]);
        let proof = build_block_proof(7, &block_id("ABCD"), commit, 6).unwrap();

        check_block_contents(&proof, &[], &[]).unwrap();
    }
}
