use crate::abci::AbciError;
use crate::error::Error;
use tenderdash_abci::proto::abci::CommitInfo;

use tenderdash_abci::proto::types::VoteExtension;

/// The `CleanedCommitInfo` struct represents a `CommitInfo` that has been properly formatted.
/// It stores essential data required to finalize a block in a simplified format.
///
#[derive(Clone, PartialEq)]
pub struct CleanedCommitInfo {
    /// The consensus round number
    pub round: u32,
    /// The hash representing the quorum of validators
    pub quorum_hash: [u8; 32],
    /// The aggregated BLS signature for the block
    pub block_signature: [u8; 96],
    /// The list of additional votes extensions, if any
    pub threshold_vote_extensions: Vec<VoteExtension>,
}

impl TryFrom<CommitInfo> for CleanedCommitInfo {
    type Error = Error;

    fn try_from(value: CommitInfo) -> Result<Self, Self::Error> {
        let CommitInfo {
            round,
            quorum_hash,
            block_signature,
            threshold_vote_extensions,
        } = value;
        if round < 0 {
            return Err(Error::Abci(AbciError::BadRequest(
                "round is negative in commit info".to_string(),
            )));
        }

        let quorum_hash: [u8; 32] = quorum_hash.try_into().map_err(|_| {
            Error::Abci(AbciError::BadRequestDataSize(
                "commit info quorum hash is not 32 bytes long".to_string(),
            ))
        })?;

        let block_signature: [u8; 96] = block_signature.try_into().map_err(|_| {
            Error::Abci(AbciError::BadRequestDataSize(
                "commit info block signature is not 96 bytes long".to_string(),
            ))
        })?;

        Ok(CleanedCommitInfo {
            round: round as u32,
            quorum_hash,
            block_signature,
            threshold_vote_extensions,
        })
    }
}
