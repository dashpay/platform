use crate::abci::AbciError;
use crate::error::Error;
use crate::platform_types::cleaned_abci_messages::cleaned_header;

use tenderdash_abci::proto::types::{Block, Commit, CoreChainLock, Data, EvidenceList};

/// The `CleanedBlock` struct represents a block that has been properly formatted.
/// It stores essential data required to finalize a block in a simplified format.
///
#[derive(Clone, PartialEq)]
pub struct CleanedBlock {
    /// The block header containing metadata about the block, such as its version, height, and hash.
    pub header: cleaned_header::v0::CleanedHeader,
    /// The block data containing the actual transactions and other relevant information.
    pub data: Data,
    /// A list of evidence items that may be used for validating or invalidating transactions or other data within the block.
    pub evidence: EvidenceList,
    /// An optional field containing the last commit information, which can be used to verify the consensus of the network on the previous block.
    pub last_commit: Option<Commit>,
    /// An optional field containing the core chain lock information, which can be used to ensure the finality and irreversibility of a block in the chain.
    pub core_chain_lock: Option<CoreChainLock>,
}

impl TryFrom<Block> for CleanedBlock {
    type Error = Error;

    fn try_from(value: Block) -> Result<Self, Self::Error> {
        let Block {
            header,
            data,
            evidence,
            last_commit,
            core_chain_lock,
        } = value;
        let Some(header) = header else {
            return Err(AbciError::BadRequest("block is missing header".to_string()).into());
        };
        let Some(data) = data else {
            return Err(AbciError::BadRequest("block is missing data".to_string()).into());
        };

        let Some(evidence) = evidence else {
            return Err(AbciError::BadRequest("block is missing evidence".to_string()).into());
        };

        Ok(CleanedBlock {
            header: header.try_into()?,
            data,
            evidence,
            last_commit,
            core_chain_lock,
        })
    }
}
