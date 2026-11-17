//! The proofs `getBlocks` and `getOwnerTransactionsInBlock` return when asked to prove.
//!
//! Tenderdash already commits to a block's contents: the transactions and their results
//! are Merkle trees whose roots are in the block header, and the quorum signs the header.
//! Nothing here is new consensus data, only what a client needs to follow that chain.
//!
//! Every hash on that chain is recomputed here before a proof leaves: the header against
//! the block id the quorum signed, and each tree against its root in the header. The
//! hashing is `drive_proof_verifier::block_proof`, the code a client checks a proof with,
//! so a node never hands out a proof that its own data cannot satisfy.

use chrono::DateTime;
use dapi_grpc::platform::v0::get_blocks_response::get_blocks_response_v0::Block;
use dapi_grpc::platform::v0::tenderdash_block_proof::{BlockId, Header};
use dapi_grpc::platform::v0::{TenderdashBlockProof, TenderdashTransactionProof};
use drive_proof_verifier::block_proof::{
    BlockProofError, MerkleTree, check_block_contents as check_contents_against_header,
    check_block_fields as check_fields_against_header, check_header, merkle_root, proof_parts,
    result_leaf, transaction_hash,
};
#[cfg(test)]
use drive_proof_verifier::block_proof::{header_hash, state_id_hash};

use super::error_mapping::base64_decode;
use super::get_blocks::decode_hex;
use crate::clients::tenderdash_client::{
    BlockHeader, BlockId as TenderdashBlockId, BlockTxResult, ResultCommit,
};
use crate::error::{DAPIResult, DapiError};

/// A proof this node built does not hold: its own data is inconsistent.
fn unsatisfied(error: BlockProofError) -> DapiError {
    DapiError::Internal(error.to_string())
}

/// The leaves of a block's results tree.
fn result_leaves(results: &[BlockTxResult]) -> DAPIResult<Vec<Vec<u8>>> {
    results
        .iter()
        .enumerate()
        .map(|(index, result)| {
            let data = base64_decode(&result.data).ok_or_else(|| {
                DapiError::Internal(format!("result {index} of the block is not base64"))
            })?;
            Ok(result_leaf(result.code, &data, result.gas_used))
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

/// Check that a block's contents are the ones its header commits to. A block without
/// transactions has no results, and its header commits to that too: the root of nothing.
pub(super) fn check_block_contents(
    proof: &TenderdashBlockProof,
    transaction_hashes: &[Vec<u8>],
    results: &[BlockTxResult],
) -> DAPIResult<()> {
    let (header, _) = proof_parts(proof).map_err(unsatisfied)?;
    check_contents_against_header(header, transaction_hashes, &result_leaves(results)?)
        .map_err(unsatisfied)
}

/// Check that the fields a block repeats from its header are the signed ones. They were
/// read from another Tenderdash answer than the proof, so nothing else ties them to it.
pub(super) fn check_block_fields(proof: &TenderdashBlockProof, block: &Block) -> DAPIResult<()> {
    check_fields_against_header(proof, block).map_err(unsatisfied)
}

/// The Merkle paths of the transactions at `indexes`. `results` is the whole block's, or
/// empty when `indexes` is, since the results are read only for transactions returned.
pub(super) fn transaction_proofs(
    proof: &TenderdashBlockProof,
    block_txs: &[String],
    results: &[BlockTxResult],
    indexes: &[usize],
) -> DAPIResult<Vec<TenderdashTransactionProof>> {
    let (header, _) = proof_parts(proof).map_err(unsatisfied)?;
    let hashes = block_txs
        .iter()
        .enumerate()
        .map(|(index, tx)| {
            base64_decode(tx)
                .map(|bytes| transaction_hash(&bytes))
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
    let results = MerkleTree::new(&result_leaves(results)?, indexes);
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

    let signed_block_id = convert_block_id(&signed.commit.block_id)?;
    let header = convert_header(signed.header)?;
    // A client repeats this check, so a header that fails it here would be a proof no
    // client accepts
    check_header(&header, &signed_block_id).map_err(unsatisfied)?;

    Ok(TenderdashBlockProof {
        block_id: Some(signed_block_id),
        round: signed.commit.round,
        quorum_hash: decode_hex("quorum_hash", &signed.commit.quorum_hash)?,
        quorum_type,
        signature,
        header: Some(header),
    })
}

/// A `commit` result as Tenderdash's RPC serves it: a header that commits to these
/// transaction hashes and results, and the block id Tenderdash would sign for it.
#[cfg(test)]
pub(super) fn tenderdash_commit(
    height: u64,
    transaction_hashes: &[Vec<u8>],
    results: &[BlockTxResult],
) -> ResultCommit {
    serde_json::from_value(tenderdash_commit_json(height, transaction_hashes, results)).unwrap()
}

/// The JSON of [`tenderdash_commit`], for a test that serves it as Tenderdash would.
#[cfg(test)]
pub(super) fn tenderdash_commit_json(
    height: u64,
    transaction_hashes: &[Vec<u8>],
    results: &[BlockTxResult],
) -> serde_json::Value {
    let results_root = merkle_root(&result_leaves(results).unwrap());
    let mut commit = serde_json::json!({
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
                "block_id": { "parts": { "total": 1, "hash": "DD" } },
                "quorum_hash": "0B0C",
                "threshold_block_signature": "AQID",
                "threshold_vote_extensions": [],
            },
        },
        "canonical": true,
    });

    let unsigned: ResultCommit = serde_json::from_value(commit.clone()).unwrap();
    let header = convert_header(unsigned.signed_header.header).unwrap();
    let block_id = &mut commit["signed_header"]["commit"]["block_id"];
    block_id["hash"] = hex::encode_upper(header_hash(&header)).into();
    block_id["state_id"] = hex::encode_upper(state_id_hash(&header).unwrap()).into();
    commit
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::tenderdash_client::BlockId as TenderdashBlockId;
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;
    use drive_proof_verifier::block_proof::fixtures::{QUORUM_HASH, QUORUM_TYPE, TestQuorum, sign};
    use drive_proof_verifier::block_proof::{verify_block_proof, verify_transaction_proof};
    use sha2::{Digest, Sha256};

    /// Build the proof of `commit`'s block as the block Tenderdash served at `height`.
    fn prove(height: u64, commit: ResultCommit) -> DAPIResult<TenderdashBlockProof> {
        let block_id = commit.signed_header.commit.block_id.clone();
        build_block_proof(height, &block_id, commit, 6)
    }

    fn sample_txs() -> Vec<String> {
        (0u8..5).map(|byte| STANDARD.encode([byte; 3])).collect()
    }

    fn sample_hashes() -> Vec<Vec<u8>> {
        (0u8..5)
            .map(|byte| transaction_hash(&[byte; 3]).to_vec())
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
        prove(7, tenderdash_commit(7, &sample_hashes(), &sample_results())).unwrap()
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

        assert_eq!(block_id.hash, header_hash(header));
        assert_eq!(block_id.part_set_total, 1);
        assert_eq!(block_id.part_set_hash, [0xdd]);
        assert_eq!(block_id.state_id, state_id_hash(header).unwrap());
        assert_eq!(proof.round, 3);
        assert_eq!(proof.quorum_hash, [0x0b, 0x0c]);
        assert_eq!(proof.quorum_type, 6);
        assert_eq!(proof.signature, [1, 2, 3]);
    }

    #[test]
    fn should_reject_a_commit_for_another_block() {
        let other_block = TenderdashBlockId {
            hash: "ABCE".to_string(),
            ..Default::default()
        };
        assert!(matches!(
            build_block_proof(7, &other_block, tenderdash_commit(7, &[], &[]), 6),
            Err(DapiError::Internal(_))
        ));

        assert!(matches!(
            prove(7, tenderdash_commit(8, &[], &[])),
            Err(DapiError::Internal(_))
        ));
    }

    #[test]
    fn should_reject_a_header_that_does_not_hash_to_the_signed_block_id() {
        let mut commit = tenderdash_commit(7, &[], &[]);
        commit.signed_header.header.validators_hash = "FF".to_string();

        assert!(matches!(prove(7, commit), Err(DapiError::Internal(_))));
    }

    #[test]
    fn should_reject_a_header_that_does_not_hash_to_the_signed_state_id() {
        // The block hash leaves this field out, so only the state id catches it
        let mut commit = tenderdash_commit(7, &[], &[]);
        commit.signed_header.header.core_chain_locked_height += 1;

        assert!(matches!(prove(7, commit), Err(DapiError::Internal(_))));
    }

    /// Each proof is checked the way a client checks it.
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
            verify_transaction_proof(
                header,
                &transaction_hash(&[index as u8; 3]),
                &result_leaf(results[index].code, &[], results[index].gas_used),
                transaction,
            )
            .unwrap();
        }
    }

    /// The whole way from Tenderdash's RPC to a client: a commit as the RPC serves it,
    /// for a block a test quorum signed, becomes proofs the client's verifier accepts.
    #[test]
    fn should_build_proofs_a_client_accepts_from_a_commit_the_quorum_signed() {
        let results = sample_results();
        let mut commit = tenderdash_commit(7, &sample_hashes(), &results);
        commit.signed_header.commit.quorum_hash = hex::encode_upper(QUORUM_HASH);
        let block_id = commit.signed_header.commit.block_id.clone();
        let mut proof = build_block_proof(7, &block_id, commit, QUORUM_TYPE).unwrap();
        // The signature Tenderdash would serve in the commit
        sign(&mut proof);
        let quorum = TestQuorum::new();

        let header = verify_block_proof(&proof, &quorum).unwrap();
        let proofs = transaction_proofs(&proof, &sample_txs(), &results, &[0, 3]).unwrap();

        assert_eq!(header.height, 7);
        assert_eq!(
            *quorum.verified.lock().unwrap(),
            [(QUORUM_TYPE, QUORUM_HASH, 2_300_000)]
        );
        for (transaction, index) in proofs.iter().zip([0usize, 3]) {
            verify_transaction_proof(
                header,
                &transaction_hash(&[index as u8; 3]),
                &result_leaf(results[index].code, &[], results[index].gas_used),
                transaction,
            )
            .unwrap();
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

    /// A field's name in an error message, and a change that makes it another value.
    type FieldChange = (&'static str, fn(&mut Block));

    /// The block fields `sample_proof` signs.
    fn sample_block() -> Block {
        let proof = sample_proof();
        let header = proof.header.unwrap();
        Block {
            height: header.height,
            hash: proof.block_id.unwrap().hash,
            time_ms: 1_722_265_200_123,
            core_chain_locked_height: header.core_chain_locked_height,
            app_hash: header.app_hash,
            proposer_pro_tx_hash: header.proposer_pro_tx_hash,
            ..Default::default()
        }
    }

    #[test]
    fn should_accept_a_block_whose_fields_are_the_signed_ones() {
        assert!(check_block_fields(&sample_proof(), &sample_block()).is_ok());
    }

    #[test]
    fn should_refuse_a_block_field_that_is_not_the_signed_one() {
        let changes: [FieldChange; 6] = [
            ("height", |block| block.height += 1),
            ("hash", |block| block.hash[0] ^= 1),
            ("time", |block| block.time_ms += 1),
            ("core chain locked height", |block| {
                block.core_chain_locked_height += 1
            }),
            ("app hash", |block| block.app_hash[0] ^= 1),
            ("proposer", |block| block.proposer_pro_tx_hash[0] ^= 1),
        ];

        for (name, change) in changes {
            let mut block = sample_block();
            change(&mut block);

            let Err(DapiError::Internal(message)) = check_block_fields(&sample_proof(), &block)
            else {
                panic!("a block with another {name} than its signed header was accepted");
            };
            assert!(message.contains(&format!("the {name} of")), "{message}");
        }
    }

    #[test]
    fn should_accept_a_block_without_transactions_whose_header_has_the_empty_results_root() {
        let commit = tenderdash_commit(7, &[], &[]);
        assert_eq!(
            commit.signed_header.header.results_hash,
            hex::encode_upper(Sha256::digest([]))
        );
        let proof = prove(7, commit).unwrap();

        check_block_contents(&proof, &[], &[]).unwrap();
    }

    #[test]
    fn should_refuse_a_block_without_transactions_whose_header_has_another_results_root() {
        let proof = prove(7, tenderdash_commit(7, &[], &sample_results())).unwrap();

        let Err(DapiError::Internal(message)) = check_block_contents(&proof, &[], &[]) else {
            panic!("an empty block was accepted under a results root that is not the empty one");
        };
        assert!(message.contains("the results of block 7"), "{message}");
    }
}
