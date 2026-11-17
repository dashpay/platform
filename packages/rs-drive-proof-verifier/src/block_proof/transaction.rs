//! A block's transactions and their execution results under the two roots of its header.

use super::merkle::{merkle_root, root_from_path};
use super::BlockProofError;
use dapi_grpc::platform::v0::tenderdash_block_proof::Header;
use dapi_grpc::platform::v0::TenderdashTransactionProof;
use dapi_grpc::Message;
use sha2::{Digest, Sha256};
use tenderdash_abci::proto::abci::ExecTxResult;

/// The hash Tenderdash knows a transaction by, which is also its leaf in the tree behind
/// a header's `data_hash`.
pub fn transaction_hash(transaction: &[u8]) -> [u8; 32] {
    Sha256::digest(transaction).into()
}

/// The leaf of a transaction's execution result in the tree behind a header's
/// `results_hash`: Tenderdash hashes only these fields of the result.
pub fn result_leaf(code: u32, data: &[u8], gas_used: i64) -> Vec<u8> {
    ExecTxResult {
        code,
        data: data.to_vec(),
        gas_used,
        ..Default::default()
    }
    .encode_to_vec()
}

/// Check that a block's contents are the ones `header` commits to: every transaction, by
/// its hash, in block order, and the result leaf of each.
///
/// A block without transactions has no results, and its header commits to that too: the
/// root of nothing.
pub fn check_block_contents<T: AsRef<[u8]>, R: AsRef<[u8]>>(
    header: &Header,
    transaction_hashes: &[T],
    result_leaves: &[R],
) -> Result<(), BlockProofError> {
    let height = header.height;
    if merkle_root(transaction_hashes).as_slice() != header.data_hash {
        return Err(BlockProofError::TransactionsRootMismatch { height });
    }
    if merkle_root(result_leaves).as_slice() != header.results_hash {
        return Err(BlockProofError::ResultsRootMismatch { height });
    }
    Ok(())
}

/// Check that a transaction and its execution result are in the block of `header`, at the
/// position `proof` names.
///
/// `proof.total` is the node's claim, which no single path settles: the paths show that
/// the transaction and its result are in the block, and where they are only if the block
/// holds that many transactions.
pub fn verify_transaction_proof(
    header: &Header,
    transaction_hash: &[u8; 32],
    result_leaf: &[u8],
    proof: &TenderdashTransactionProof,
) -> Result<(), BlockProofError> {
    let (height, index, total) = (header.height, proof.index, proof.total);

    let data_root = root_from_path(index, total, transaction_hash, &proof.transaction_path)?;
    if data_root.as_slice() != header.data_hash {
        return Err(BlockProofError::TransactionNotInBlock { height, index });
    }
    let results_root = root_from_path(index, total, result_leaf, &proof.result_path)?;
    if results_root.as_slice() != header.results_hash {
        return Err(BlockProofError::ResultNotInBlock { height, index });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::merkle::MerkleTree;
    use super::*;

    fn transactions() -> Vec<Vec<u8>> {
        (0u8..5).map(|byte| vec![byte; 3]).collect()
    }

    fn hashes() -> Vec<[u8; 32]> {
        transactions()
            .iter()
            .map(|transaction| transaction_hash(transaction))
            .collect()
    }

    fn result_leaves() -> Vec<Vec<u8>> {
        (0u32..5)
            .map(|index| result_leaf(index * 100, &[index as u8], i64::from(index) + 1))
            .collect()
    }

    fn header() -> Header {
        Header {
            height: 7,
            data_hash: merkle_root(&hashes()).to_vec(),
            results_hash: merkle_root(&result_leaves()).to_vec(),
            ..Default::default()
        }
    }

    fn proof_of(index: usize) -> TenderdashTransactionProof {
        TenderdashTransactionProof {
            index: index as u32,
            total: 5,
            transaction_path: MerkleTree::new(&hashes(), &[index]).paths.remove(0),
            result_path: MerkleTree::new(&result_leaves(), &[index]).paths.remove(0),
        }
    }

    fn verify(index: usize, proof: &TenderdashTransactionProof) -> Result<(), BlockProofError> {
        verify_transaction_proof(&header(), &hashes()[index], &result_leaves()[index], proof)
    }

    #[test]
    fn should_encode_a_result_as_tenderdash_does() {
        // code = 1 (varint), data = 2 (bytes), gas_used = 6 (varint)
        assert_eq!(
            result_leaf(40100, &[1, 2], 9),
            [0x08, 0xa4, 0xb9, 0x02, 0x12, 0x02, 0x01, 0x02, 0x30, 0x09]
        );
        // zero values are not encoded
        assert_eq!(result_leaf(0, &[], 0), Vec::<u8>::new());
    }

    #[test]
    fn should_accept_every_transaction_of_a_block_with_its_result() {
        for index in 0..5 {
            assert_eq!(verify(index, &proof_of(index)), Ok(()), "{index}");
        }
    }

    #[test]
    fn should_refuse_another_transaction_under_a_path() {
        let other = transaction_hash(&[9; 3]);

        assert_eq!(
            verify_transaction_proof(&header(), &other, &result_leaves()[1], &proof_of(1)),
            Err(BlockProofError::TransactionNotInBlock {
                height: 7,
                index: 1
            })
        );
    }

    #[test]
    fn should_refuse_another_result_under_a_path() {
        for other in [
            result_leaf(101, &[1], 2),
            result_leaf(100, &[2], 2),
            result_leaf(100, &[1], 3),
            // the result of another transaction of the same block
            result_leaves()[2].clone(),
        ] {
            assert_eq!(
                verify_transaction_proof(&header(), &hashes()[1], &other, &proof_of(1)),
                Err(BlockProofError::ResultNotInBlock {
                    height: 7,
                    index: 1
                })
            );
        }
    }

    #[test]
    fn should_refuse_a_changed_path() {
        let mut proof = proof_of(1);
        proof.transaction_path[0][0] ^= 1;
        assert_eq!(
            verify(1, &proof),
            Err(BlockProofError::TransactionNotInBlock {
                height: 7,
                index: 1
            })
        );

        let mut proof = proof_of(1);
        proof.result_path[1][0] ^= 1;
        assert_eq!(
            verify(1, &proof),
            Err(BlockProofError::ResultNotInBlock {
                height: 7,
                index: 1
            })
        );

        let mut proof = proof_of(1);
        proof.transaction_path.pop();
        assert_eq!(
            verify(1, &proof),
            Err(BlockProofError::MalformedPath { index: 1, total: 5 })
        );
    }

    #[test]
    fn should_refuse_a_transaction_at_another_position() {
        // position 0 has a path of the same length, hashed in another order
        let mut proof = proof_of(1);
        proof.index = 0;
        assert_eq!(
            verify(1, &proof),
            Err(BlockProofError::TransactionNotInBlock {
                height: 7,
                index: 0
            })
        );

        // position 4 of five has a shorter path
        let mut proof = proof_of(1);
        proof.index = 4;
        assert_eq!(
            verify(1, &proof),
            Err(BlockProofError::MalformedPath { index: 4, total: 5 })
        );

        let mut proof = proof_of(1);
        proof.index = 5;
        assert_eq!(
            verify(1, &proof),
            Err(BlockProofError::IndexOutOfRange { index: 5, total: 5 })
        );
    }

    #[test]
    fn should_refuse_a_transaction_of_another_block() {
        let mut header = header();
        header.data_hash[0] ^= 1;
        assert_eq!(
            verify_transaction_proof(&header, &hashes()[1], &result_leaves()[1], &proof_of(1)),
            Err(BlockProofError::TransactionNotInBlock {
                height: 7,
                index: 1
            })
        );
    }

    #[test]
    fn should_accept_a_block_whose_contents_match_its_header() {
        assert_eq!(
            check_block_contents(&header(), &hashes(), &result_leaves()),
            Ok(())
        );
    }

    #[test]
    fn should_refuse_contents_the_header_does_not_commit_to() {
        let mut changed = hashes();
        changed.swap(0, 1);
        assert_eq!(
            check_block_contents(&header(), &changed, &result_leaves()),
            Err(BlockProofError::TransactionsRootMismatch { height: 7 })
        );
        assert_eq!(
            check_block_contents(&header(), &hashes()[..4], &result_leaves()),
            Err(BlockProofError::TransactionsRootMismatch { height: 7 })
        );

        let mut changed = result_leaves();
        changed[0] = result_leaf(1, &[], 1);
        assert_eq!(
            check_block_contents(&header(), &hashes(), &changed),
            Err(BlockProofError::ResultsRootMismatch { height: 7 })
        );
        assert_eq!(
            check_block_contents(&header(), &hashes(), &result_leaves()[..4]),
            Err(BlockProofError::ResultsRootMismatch { height: 7 })
        );
    }

    #[test]
    fn should_hold_a_block_without_transactions_to_the_root_of_no_results() {
        let nothing: [Vec<u8>; 0] = [];
        let mut header = Header {
            height: 7,
            data_hash: merkle_root(&nothing).to_vec(),
            results_hash: merkle_root(&nothing).to_vec(),
            ..Default::default()
        };
        assert_eq!(check_block_contents(&header, &nothing, &nothing), Ok(()));

        header.results_hash = merkle_root(&result_leaves()).to_vec();
        assert_eq!(
            check_block_contents(&header, &nothing, &nothing),
            Err(BlockProofError::ResultsRootMismatch { height: 7 })
        );
    }
}
