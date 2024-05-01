use crate::abci::AbciError;
use crate::error::Error;
use crate::platform_types::cleaned_abci_messages::cleaned_block::v0::CleanedBlock;
use crate::platform_types::cleaned_abci_messages::cleaned_block_id::v0::CleanedBlockId;
use crate::platform_types::cleaned_abci_messages::cleaned_commit_info::v0::CleanedCommitInfo;
use tenderdash_abci::proto::abci::{Misbehavior, RequestFinalizeBlock};

/// The `FinalizeBlockCleanedRequest` struct represents a `RequestFinalizeBlock` that has been
/// properly formatted.
/// It stores essential data required to finalize the request in a simplified format.
///
#[derive(Clone, PartialEq)]
pub struct FinalizeBlockCleanedRequest {
    /// Info about the current commit
    pub commit: CleanedCommitInfo,
    /// List of information about validators that acted incorrectly.
    pub misbehavior: Vec<Misbehavior>,
    /// The block header's hash. Present for convenience (can be derived from the block header).
    pub hash: [u8; 32],
    /// The height of the finalized block.
    pub height: u64,
    /// Round number for the block
    pub round: u32,
    /// The block that was finalized
    pub block: CleanedBlock,
    /// The block ID that was finalized
    pub block_id: CleanedBlockId,
}

impl TryFrom<RequestFinalizeBlock> for FinalizeBlockCleanedRequest {
    type Error = Error;

    fn try_from(value: RequestFinalizeBlock) -> Result<Self, Self::Error> {
        let RequestFinalizeBlock {
            commit,
            misbehavior,
            hash,
            height,
            round,
            block,
            block_id,
        } = value;

        let Some(commit) = commit else {
            return Err(
                AbciError::BadRequest("finalize block is missing commit".to_string()).into(),
            );
        };

        let Some(block) = block else {
            return Err(AbciError::BadRequest(
                "finalize block is missing actual block".to_string(),
            )
            .into());
        };

        let Some(block_id) = block_id else {
            return Err(
                AbciError::BadRequest("finalize block is missing block_id".to_string()).into(),
            );
        };

        let hash = hash.try_into().map_err(|_| {
            Error::Abci(AbciError::BadRequestDataSize(
                "finalize block hash is not 32 bytes long".to_string(),
            ))
        })?;

        if height < 0 {
            return Err(AbciError::BadRequest(
                "height is negative in request prepare proposal".to_string(),
            )
            .into());
        }
        if round < 0 {
            return Err(AbciError::BadRequest(
                "round is negative in request prepare proposal".to_string(),
            )
            .into());
        }

        Ok(FinalizeBlockCleanedRequest {
            commit: commit.try_into()?,
            misbehavior,
            hash,
            height: height as u64,
            round: round as u32,
            block: block.try_into()?,
            block_id: block_id.try_into()?,
        })
    }
}
