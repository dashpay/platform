use crate::abci::AbciError;
use crate::error::Error;
use crate::platform_types::cleaned_abci_messages::hash_or_default;

use tenderdash_abci::proto::types::{BlockId, PartSetHeader};

/// The `CleanedBlockId` struct represents a `blockId` that has been properly formatted.
/// It stores essential data required to finalize a block in a simplified format.
///
#[derive(Clone, PartialEq)]
pub struct CleanedBlockId {
    /// The block id hash
    pub hash: [u8; 32],
    /// The part set header of the block id
    pub part_set_header: PartSetHeader,
    /// The state id
    pub state_id: [u8; 32],
}

impl TryFrom<BlockId> for CleanedBlockId {
    type Error = Error;

    fn try_from(value: BlockId) -> Result<Self, Self::Error> {
        let BlockId {
            hash,
            part_set_header,
            state_id,
        } = value;
        let hash = hash_or_default(hash).map_err(|_| {
            Error::Abci(AbciError::BadRequestDataSize(
                "hash is not 32 bytes long in block id".to_string(),
            ))
        })?;

        let Some(part_set_header) = part_set_header else {
            return Err(
                AbciError::BadRequest("block id is missing part set header".to_string()).into(),
            );
        };

        let state_id = hash_or_default(state_id).map_err(|_| {
            Error::Abci(AbciError::BadRequestDataSize(
                "state id is not 32 bytes long".to_string(),
            ))
        })?;

        Ok(CleanedBlockId {
            hash,
            part_set_header,
            state_id,
        })
    }
}

impl From<CleanedBlockId> for BlockId {
    fn from(value: CleanedBlockId) -> Self {
        Self {
            hash: value.hash.to_vec(),
            part_set_header: Some(value.part_set_header),
            state_id: value.state_id.to_vec(),
        }
    }
}
