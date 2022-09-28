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

//! Flags
//!

use byteorder::{BigEndian, ReadBytesExt};
use grovedb::ElementFlags;

use crate::error::drive::DriveError;
use crate::error::Error;

/// Storage flags
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StorageFlags {
    /// Epoch
    pub epoch: u16,
}

impl StorageFlags {
    /// Creates storage flags from a slice.
    pub fn from_slice(data: &[u8]) -> Result<Self, Error> {
        let mut epoch_bytes =
            data.get(0..2)
                .ok_or(Error::Drive(DriveError::CorruptedElementFlags(
                    "unable to get epochs",
                )))?;
        let epoch = epoch_bytes.read_u16::<BigEndian>().map_err(|_| {
            Error::Drive(DriveError::CorruptedElementFlags("unable to parse epochs"))
        })?;
        Ok(StorageFlags { epoch })
    }

    /// Creates storage flags from element flags.
    pub fn from_element_flags(data: ElementFlags) -> Result<Self, Error> {
        let data = data.ok_or(Error::Drive(DriveError::CorruptedElementFlags(
            "no element flag on data",
        )))?;
        Self::from_slice(data.as_slice())
    }

    /// Creates element flags.
    pub fn to_element_flags(&self) -> ElementFlags {
        let epoch_bytes = self.epoch.to_be_bytes().to_vec();
        Some(epoch_bytes)
    }
}
