// MIT LICENSE
//
// Copyright (c) 2021 Dash Core Group
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
//

//! Epoch Paths
//!
//! Defines and implements in `Epoch` functions related to paths related to epochs.
//!

use crate::drive::RootTree;
use crate::error::fee::FeeError;
use crate::error::Error;
use crate::fee_pools::epochs::epoch_key_constants;
use crate::fee_pools::epochs::epoch_key_constants::EPOCH_STORAGE_OFFSET;
use crate::fee_pools::epochs::Epoch;

impl Epoch {
    /// Get the path to the proposers tree of this epoch as a fixed length path
    pub fn get_proposers_path(&self) -> [&[u8]; 3] {
        [
            Into::<&[u8; 1]>::into(RootTree::Pools),
            &self.key,
            epoch_key_constants::KEY_PROPOSERS.as_slice(),
        ]
    }

    /// Get the path to the proposers tree of this epoch as a vector
    pub fn get_proposers_vec_path(&self) -> Vec<Vec<u8>> {
        vec![
            vec![RootTree::Pools as u8],
            self.key.to_vec(),
            epoch_key_constants::KEY_PROPOSERS.to_vec(),
        ]
    }

    /// Get the path to this epoch as a fixed size path
    pub fn get_path(&self) -> [&[u8]; 2] {
        [Into::<&[u8; 1]>::into(RootTree::Pools), &self.key]
    }

    /// Get the path to this epoch as a vector
    pub fn get_vec_path(&self) -> Vec<Vec<u8>> {
        vec![vec![RootTree::Pools as u8], self.key.to_vec()]
    }
}

/// Encodes an epoch index key with storage offset
pub fn encode_epoch_index_key(index: u16) -> Result<[u8; 2], Error> {
    let index_with_offset =
        index
            .checked_add(EPOCH_STORAGE_OFFSET)
            .ok_or(Error::Fee(FeeError::Overflow(
                "stored epoch index too high",
            )))?;

    Ok(index_with_offset.to_be_bytes())
}

/// Decodes an epoch index key
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
