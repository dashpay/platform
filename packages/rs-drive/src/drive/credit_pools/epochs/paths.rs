use crate::drive::credit_pools::epochs::epoch_key_constants;
use crate::drive::RootTree;
use crate::error::drive::DriveError;
use crate::error::fee::FeeError;
use crate::error::Error;
use dpp::block::epoch::{Epoch, EPOCH_KEY_OFFSET};

/// Proposer Trait for Epoch
pub trait EpochProposers {
    /// Get the path to this epoch as a vector
    fn get_path_vec(&self) -> Vec<Vec<u8>>;
    /// Get the path to this epoch as a fixed size path
    fn get_path(&self) -> [&[u8]; 2];
    /// Get the path to the proposers tree of this epoch as a vector
    fn get_proposers_path_vec(&self) -> Vec<Vec<u8>>;
    /// Get the path to the proposers tree of this epoch as a fixed length path
    fn get_proposers_path(&self) -> [&[u8]; 3];
}

impl EpochProposers for Epoch {
    /// Get the path to the proposers tree of this epoch as a fixed length path
    fn get_proposers_path(&self) -> [&[u8]; 3] {
        [
            Into::<&[u8; 1]>::into(RootTree::Pools),
            &self.key,
            epoch_key_constants::KEY_PROPOSERS.as_slice(),
        ]
    }

    /// Get the path to the proposers tree of this epoch as a vector
    fn get_proposers_path_vec(&self) -> Vec<Vec<u8>> {
        vec![
            vec![RootTree::Pools as u8],
            self.key.to_vec(),
            epoch_key_constants::KEY_PROPOSERS.to_vec(),
        ]
    }

    /// Get the path to this epoch as a fixed size path
    fn get_path(&self) -> [&[u8]; 2] {
        [Into::<&[u8; 1]>::into(RootTree::Pools), &self.key]
    }

    /// Get the path to this epoch as a vector
    fn get_path_vec(&self) -> Vec<Vec<u8>> {
        vec![vec![RootTree::Pools as u8], self.key.to_vec()]
    }
}

/// Encodes an epoch index key with storage offset
pub fn encode_epoch_index_key(index: u16) -> Result<[u8; 2], Error> {
    let index_with_offset =
        index
            .checked_add(EPOCH_KEY_OFFSET)
            .ok_or(Error::Fee(FeeError::Overflow(
                "stored epoch index too high",
            )))?;

    Ok(index_with_offset.to_be_bytes())
}

/// Decodes an epoch index key
pub fn decode_epoch_index_key(epoch_key: &[u8]) -> Result<u16, Error> {
    let index_with_offset = u16::from_be_bytes(epoch_key.try_into().map_err(|_| {
        Error::Drive(DriveError::CorruptedSerialization(String::from(
            "epoch index must be u16",
        )))
    })?);

    index_with_offset
        .checked_sub(EPOCH_KEY_OFFSET)
        .ok_or(Error::Fee(FeeError::Overflow("stored epoch index too low")))
}
