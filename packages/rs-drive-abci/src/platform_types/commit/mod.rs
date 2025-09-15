use crate::abci::AbciError;
use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::cleaned_abci_messages::{cleaned_block_id, cleaned_commit_info};
use crate::platform_types::commit::v0::CommitV0;
use dpp::bls_signatures;
use dpp::bls_signatures::Bls12381G2Impl;
use dpp::dashcore_rpc::dashcore_rpc_json::QuorumType;
use dpp::validation::SimpleValidationResult;
use dpp::version::PlatformVersion;
use tenderdash_abci::proto::abci::CommitInfo;
use tenderdash_abci::proto::types::BlockId;

/// Accessors for the commit
pub mod accessors;
pub(crate) mod v0;

/// Represents block commit
#[derive(Clone, Debug)]
pub enum Commit {
    /// Version 0
    V0(CommitV0),
}

impl Commit {
    /// Create new Commit struct based on commit info and block id received from Tenderdash
    pub fn new_from_cleaned(
        ci: cleaned_commit_info::v0::CleanedCommitInfo,
        block_id: cleaned_block_id::v0::CleanedBlockId,
        height: u64,
        quorum_type: QuorumType,
        chain_id: &str,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        match platform_version.drive_abci.structs.commit {
            0 => Ok(Commit::V0(CommitV0::new_from_cleaned(
                ci,
                block_id,
                height,
                quorum_type,
                chain_id,
            ))),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "identity credit withdrawal transition: transform_into_action".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Create new Commit struct based on commit info and block id received from Tenderdash
    pub fn new(
        ci: CommitInfo,
        block_id: BlockId,
        height: u64,
        quorum_type: QuorumType,
        chain_id: &str,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        match platform_version.drive_abci.structs.commit {
            0 => Ok(Commit::V0(CommitV0::new(
                ci,
                block_id,
                height,
                quorum_type,
                chain_id,
            ))),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "identity credit withdrawal transition: transform_into_action".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Verify all signatures using provided public key.
    ///
    /// ## Return value
    ///
    /// * Ok(true) when all signatures are correct
    /// * Ok(false) when at least one signature is invalid
    /// * Err(e) on error
    pub fn verify_signature(
        &self,
        signature: &[u8; 96],
        public_key: &bls_signatures::PublicKey<Bls12381G2Impl>,
    ) -> SimpleValidationResult<AbciError> {
        match self {
            Commit::V0(v0) => v0.verify_signature(signature, public_key),
        }
    }
}
