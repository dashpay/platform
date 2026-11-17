//! Blocks signed by test quorums, and the answers a node would give for them, for tests of
//! this crate and of the crates that verify block proofs through it.
//!
//! A test quorum's key is derived from its hash, so these are synthetic blocks, never
//! network trust anchors: a verifier accepts them only from a provider that answers a test
//! quorum's key, as [`TestQuorum`] does. The sign hash is built here by hand, from
//! Tenderdash's definition of it, so the verifier is checked against that definition and
//! not against itself.

use super::header::{header_hash, header_time_ms, state_id_hash};
use super::merkle::{merkle_root, MerkleTree};
use super::transaction::{result_leaf, transaction_hash};
use crate::{ContextProvider, ContextProviderError};
use dapi_grpc::platform::v0::get_blocks_response::get_blocks_response_v0::{Block, Transaction};
use dapi_grpc::platform::v0::get_owner_transactions_in_block_response::get_owner_transactions_in_block_response_v0::OwnerTransaction;
use dapi_grpc::platform::v0::get_owner_transactions_in_block_response::GetOwnerTransactionsInBlockResponseV0;
use dapi_grpc::platform::v0::tenderdash_block_proof::{BlockId, Header};
use dapi_grpc::platform::v0::{TenderdashBlockProof, TenderdashTransactionProof};
use dapi_grpc::Message;
use dpp::bls::SecretKey;
use dpp::data_contract::TokenConfiguration;
use dpp::prelude::{CoreBlockHeight, DataContract, Identifier};
use dpp::version::PlatformVersion;
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};
use tenderdash_abci::proto::types::{CanonicalBlockId, CanonicalPartSetHeader};

/// The LLMQ type of the test quorums.
pub const QUORUM_TYPE: u32 = 106;
/// The quorum that signs a block unless a test names another.
pub const QUORUM_HASH: [u8; 32] = [2; 32];
/// The Core height the test blocks are chain locked at.
pub const CORE_HEIGHT: u32 = 2_300_000;
/// The chain of the test blocks.
pub const CHAIN_ID: &str = "dash-test-1";

/// A transaction of a test block: its bytes, and the code, data and gas of its result.
pub type TestTransaction = (Vec<u8>, u32, Vec<u8>, i64);

/// The key of the test quorum `quorum_hash`, derived from the first byte of its hash.
fn secret_key(quorum_hash: &[u8]) -> SecretKey {
    // Kept in 1..=0x73, so the repeated byte is a scalar below the order of the curve
    let seed = quorum_hash.first().copied().unwrap_or_default() % 0x73 + 1;
    SecretKey::from_be_bytes(&[seed; 32]).expect("a scalar below the curve order is a key")
}

/// The public key of the test quorum `quorum_hash`.
pub fn quorum_public_key(quorum_hash: [u8; 32]) -> [u8; 48] {
    secret_key(&quorum_hash).public_key().to_bytes()
}

/// A provider that knows the test quorum [`QUORUM_HASH`] and records the Core heights
/// reported to it as verified.
pub struct TestQuorum {
    public_key: [u8; 48],
    /// The quorum type, quorum hash and Core height of each verified signature.
    pub verified: Mutex<Vec<(u32, [u8; 32], u32)>>,
}

impl TestQuorum {
    /// Knows the quorum by the key it signs with.
    pub fn new() -> Self {
        Self {
            public_key: quorum_public_key(QUORUM_HASH),
            verified: Mutex::new(Vec::new()),
        }
    }

    /// Knows the quorum by another key than the one it signs with.
    pub fn with_another_key() -> Self {
        Self {
            public_key: quorum_public_key([9; 32]),
            verified: Mutex::new(Vec::new()),
        }
    }
}

impl Default for TestQuorum {
    fn default() -> Self {
        Self::new()
    }
}

impl ContextProvider for TestQuorum {
    fn get_data_contract(
        &self,
        _: &Identifier,
        _: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        Ok(None)
    }

    fn get_token_configuration(
        &self,
        _: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
        Ok(None)
    }

    fn get_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        _: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        if (quorum_type, quorum_hash) != (QUORUM_TYPE, QUORUM_HASH) {
            return Err(ContextProviderError::InvalidQuorum(
                "not the test quorum".to_string(),
            ));
        }
        Ok(self.public_key)
    }

    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        Ok(1)
    }

    fn core_height_verified(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        core_chain_locked_height: u32,
    ) {
        if let Ok(mut verified) = self.verified.lock() {
            verified.push((quorum_type, quorum_hash, core_chain_locked_height));
        }
    }
}

fn sha256(bytes: &[u8]) -> Vec<u8> {
    Sha256::digest(bytes).to_vec()
}

fn reversed(mut bytes: Vec<u8>) -> Vec<u8> {
    bytes.reverse();
    bytes
}

/// The hash a quorum signs for a precommit vote, as Tenderdash defines it.
fn vote_sign_hash(proof: &TenderdashBlockProof) -> Vec<u8> {
    let header = proof.header.clone().unwrap_or_default();
    let block_id = proof.block_id.clone().unwrap_or_default();
    let height = header.height as i64;
    const PRECOMMIT: i32 = 2;

    let canonical_block_id = CanonicalBlockId {
        hash: block_id.hash,
        part_set_header: Some(CanonicalPartSetHeader {
            total: block_id.part_set_total,
            hash: block_id.part_set_hash,
        }),
    };
    let mut sign_bytes = Vec::new();
    sign_bytes.extend(PRECOMMIT.to_le_bytes());
    sign_bytes.extend(height.to_le_bytes());
    sign_bytes.extend(i64::from(proof.round).to_le_bytes());
    sign_bytes.extend(sha256(&canonical_block_id.encode_length_delimited_to_vec()));
    sign_bytes.extend(&block_id.state_id);
    sign_bytes.extend(header.chain_id.as_bytes());

    let mut request = b"dpbvote".to_vec();
    request.extend(height.to_le_bytes());
    request.extend((proof.round as i32).to_le_bytes());

    let mut message = vec![proof.quorum_type as u8];
    message.extend(reversed(proof.quorum_hash.clone()));
    message.extend(reversed(sha256(&request)));
    message.extend(reversed(sha256(&sign_bytes)));
    sha256(&sha256(&message))
}

/// Sign `proof` as it stands, as the test quorum it names.
pub fn sign(proof: &mut TenderdashBlockProof) {
    proof.signature = secret_key(&proof.quorum_hash)
        .sign(&vote_sign_hash(proof))
        .expect("a test key signs any digest")
        .to_bytes()
        .to_vec();
}

/// The block id Tenderdash derives from `header`.
pub fn block_id_of(header: &Header) -> BlockId {
    BlockId {
        hash: header_hash(header).to_vec(),
        part_set_total: 1,
        part_set_hash: vec![0xdd; 32],
        state_id: state_id_hash(header)
            .expect("a test header has a time a state id can carry")
            .to_vec(),
    }
}

/// The proof of the block of `header`, signed by the test quorum `quorum_hash`.
pub fn signed_proof_by(quorum_hash: [u8; 32], header: Header) -> TenderdashBlockProof {
    let mut proof = TenderdashBlockProof {
        block_id: Some(block_id_of(&header)),
        header: Some(header),
        round: 3,
        quorum_hash: quorum_hash.to_vec(),
        quorum_type: QUORUM_TYPE,
        signature: Vec::new(),
    };
    sign(&mut proof);
    proof
}

/// The proof of the block of `header`, signed by the test quorum [`QUORUM_HASH`].
pub fn signed_proof(header: Header) -> TenderdashBlockProof {
    signed_proof_by(QUORUM_HASH, header)
}

fn transaction_hashes(transactions: &[TestTransaction]) -> Vec<[u8; 32]> {
    transactions
        .iter()
        .map(|(transaction, ..)| transaction_hash(transaction))
        .collect()
}

fn result_leaves(transactions: &[TestTransaction]) -> Vec<Vec<u8>> {
    transactions
        .iter()
        .map(|(_, code, data, gas_used)| result_leaf(*code, data, *gas_used))
        .collect()
}

/// The header of a block at `height` that holds `transactions`, executed under the latest
/// protocol version.
pub fn header_of(height: u64, transactions: &[TestTransaction]) -> Header {
    Header {
        block_version: 14,
        app_version: u64::from(PlatformVersion::latest().protocol_version),
        chain_id: CHAIN_ID.to_string(),
        height,
        core_chain_locked_height: CORE_HEIGHT,
        time_seconds: 1_722_265_200 + height as i64,
        time_nanos: 123_456_789,
        last_block_id: Some(BlockId {
            hash: vec![0xaa; 32],
            part_set_total: 2,
            part_set_hash: vec![0xbb; 32],
            state_id: vec![0xcc; 32],
        }),
        last_commit_hash: vec![1; 32],
        data_hash: merkle_root(&transaction_hashes(transactions)).to_vec(),
        validators_hash: vec![3; 32],
        next_validators_hash: vec![4; 32],
        consensus_hash: vec![5; 32],
        next_consensus_hash: vec![6; 32],
        app_hash: vec![7; 32],
        results_hash: merkle_root(&result_leaves(transactions)).to_vec(),
        evidence_hash: vec![9; 32],
        proposer_pro_tx_hash: vec![10; 32],
        proposed_app_version: 15,
    }
}

/// The block an honest node returns from `getBlocks` when asked to prove the block of
/// `header`, which holds `transactions`, signed by the test quorum `quorum_hash`. Start
/// from [`header_of`] to give the block another protocol version or Core height.
pub fn block_answer_of(
    quorum_hash: [u8; 32],
    header: Header,
    transactions: &[TestTransaction],
) -> Block {
    let time_ms = header_time_ms(&header).expect("a test header has a time in milliseconds");
    let block_id = block_id_of(&header);

    Block {
        height: header.height,
        hash: block_id.hash,
        time_ms,
        core_chain_locked_height: header.core_chain_locked_height,
        app_hash: header.app_hash.clone(),
        proposer_pro_tx_hash: header.proposer_pro_tx_hash.clone(),
        transactions: transactions
            .iter()
            .cloned()
            .map(|(bytes, code, result_data, gas_used)| Transaction {
                hash: transaction_hash(&bytes).to_vec(),
                state_transition: bytes,
                code,
                result_data,
                gas_used,
            })
            .collect(),
        proof: Some(signed_proof_by(quorum_hash, header)),
    }
}

/// [`block_answer_of`] the block at `height` under the latest protocol version.
pub fn block_answer_by(
    quorum_hash: [u8; 32],
    height: u64,
    transactions: &[TestTransaction],
) -> Block {
    block_answer_of(quorum_hash, header_of(height, transactions), transactions)
}

/// [`block_answer_by`] the test quorum [`QUORUM_HASH`].
pub fn block_answer(height: u64, transactions: &[TestTransaction]) -> Block {
    block_answer_by(QUORUM_HASH, height, transactions)
}

/// What an honest node answers `getOwnerTransactionsInBlock` with when asked to prove the
/// transactions at `indexes` of the block of `header`, which holds `transactions`. The
/// answer lists them in the order of `indexes`, each with the paths of its own position.
pub fn owner_answer_of(
    header: Header,
    transactions: &[TestTransaction],
    indexes: &[usize],
) -> GetOwnerTransactionsInBlockResponseV0 {
    let total = transactions.len() as u32;
    let hashes = transaction_hashes(transactions);
    let leaves = result_leaves(transactions);

    let answered = indexes
        .iter()
        .filter_map(|&index| {
            let (bytes, code, result_data, gas_used) = transactions.get(index)?.clone();
            Some(OwnerTransaction {
                hash: transaction_hash(&bytes).to_vec(),
                state_transition: bytes,
                code,
                result_data,
                gas_used,
                proof: Some(TenderdashTransactionProof {
                    index: index as u32,
                    total,
                    transaction_path: MerkleTree::new(&hashes, &[index]).paths.pop()?,
                    result_path: MerkleTree::new(&leaves, &[index]).paths.pop()?,
                }),
            })
        })
        .collect();

    GetOwnerTransactionsInBlockResponseV0 {
        transactions: answered,
        proof: Some(signed_proof(header)),
    }
}

/// [`owner_answer_of`] the block at `height` under the latest protocol version.
pub fn owner_answer(
    height: u64,
    transactions: &[TestTransaction],
    indexes: &[usize],
) -> GetOwnerTransactionsInBlockResponseV0 {
    owner_answer_of(header_of(height, transactions), transactions, indexes)
}
