use byteorder::{BigEndian, ReadBytesExt};
use grovedb::ElementFlags;

use crate::error::drive::DriveError;
use crate::error::Error;

// Struct Definitions
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StorageFlags {
    pub epoch: u16,
}

impl StorageFlags {
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

    pub fn from_element_flags(data: ElementFlags) -> Result<Self, Error> {
        let data = data.ok_or(Error::Drive(DriveError::CorruptedElementFlags(
            "no element flag on data",
        )))?;
        Self::from_slice(data.as_slice())
    }

    pub fn to_element_flags(&self) -> ElementFlags {
        let epoch_bytes = self.epoch.to_be_bytes().to_vec();
        Some(epoch_bytes)
    }
}
