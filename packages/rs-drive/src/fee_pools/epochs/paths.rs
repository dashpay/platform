use crate::drive::RootTree;
use crate::error::fee::FeeError;
use crate::error::Error;
use crate::fee_pools::epochs::epoch_key_constants;
use crate::fee_pools::epochs::epoch_key_constants::EPOCH_STORAGE_OFFSET;
use crate::fee_pools::epochs::Epoch;

impl Epoch {
    pub fn get_proposers_path(&self) -> [&[u8]; 3] {
        [
            Into::<&[u8; 1]>::into(RootTree::Pools),
            &self.key,
            epoch_key_constants::KEY_PROPOSERS.as_slice(),
        ]
    }

    pub fn get_proposers_vec_path(&self) -> Vec<Vec<u8>> {
        vec![
            vec![RootTree::Pools as u8],
            self.key.to_vec(),
            epoch_key_constants::KEY_PROPOSERS.to_vec(),
        ]
    }

    pub fn get_path(&self) -> [&[u8]; 2] {
        [Into::<&[u8; 1]>::into(RootTree::Pools), &self.key]
    }

    pub fn get_vec_path(&self) -> Vec<Vec<u8>> {
        vec![vec![RootTree::Pools as u8], self.key.to_vec()]
    }
}

pub fn encode_epoch_index_key(index: u16) -> Result<[u8; 2], Error> {
    let index_with_offset =
        index
            .checked_add(EPOCH_STORAGE_OFFSET)
            .ok_or(Error::Fee(FeeError::Overflow(
                "stored epoch index too high",
            )))?;

    Ok(index_with_offset.to_be_bytes())
}

pub fn decode_epoch_index_key(epoch_key: &[u8]) -> Result<u16, Error> {
    let index_with_offset = u16::from_be_bytes(epoch_key.try_into().map_err(|_| {
        Error::Fee(FeeError::CorruptedProposerBlockCountItemLength(
            "item have an invalid length",
        ))
    })?);

    index_with_offset
        .checked_sub(EPOCH_STORAGE_OFFSET)
        .ok_or(Error::Fee(FeeError::Overflow("stored epoch index too low")))
}
