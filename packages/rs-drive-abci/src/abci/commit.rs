//! Tenderdash commit logic
use tenderdash_abci::proto::{self, abci::CommitInfo, signatures::SignBytes, types::BlockId};

/// Represents commit for a block
pub struct Commit {
    inner: proto::types::Commit,
}

impl Commit {
    /// Create new Commit struct based on commit info and block id received from Tenderdash
    pub fn new(ci: CommitInfo, block_id: BlockId, height: i64) -> Self {
        Self {
            inner: proto::types::Commit {
                block_id: Some(block_id),
                height,
                round: ci.round,
                quorum_hash: ci.quorum_hash,
                threshold_block_signature: ci.block_signature,
                threshold_vote_extensions: ci.threshold_vote_extensions,
            },
        }
    }
}

impl SignBytes for Commit {
    fn sign_bytes(&self, chain_id: &str, height: i64, round: i32) -> Result<Vec<u8>, proto::Error> {
        self.inner.sign_bytes(chain_id, height, round)
    }
}
