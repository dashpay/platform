//! Proofs of Tenderdash blocks: what `getBlocks` and `getOwnerTransactionsInBlock` return
//! when asked to prove.
//!
//! Tenderdash already commits to a block's contents. The transactions and their execution
//! results are Merkle trees whose roots are in the block header, the header hashes to the
//! block id, and the validator quorum signs a vote for that block id. A proof carries the
//! header, the block id and the signature, and for single transactions the Merkle paths
//! to the two roots.
//!
//! This module is the one definition of every hash on that chain. DAPI builds its proofs
//! with it and checks them before they leave ([`check_header`], [`check_block_contents`],
//! [`check_block_fields`], [`MerkleTree`]), and a client verifies them with it ([`verify_block_proof`],
//! [`verify_transaction_proof`]), so a node cannot hand out a proof that clients refuse.

mod block;
mod error;
#[cfg(any(test, feature = "mocks"))]
pub mod fixtures;
mod header;
mod merkle;
mod transaction;

pub use block::{check_block_fields, proof_parts, verify_block_proof};
pub use error::BlockProofError;
pub use header::{check_header, header_hash, header_time_ms, state_id_hash};
pub use merkle::{merkle_root, root_from_path, MerkleTree};
pub use transaction::{
    check_block_contents, result_leaf, transaction_hash, verify_transaction_proof,
};
