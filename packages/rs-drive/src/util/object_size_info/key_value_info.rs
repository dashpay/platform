use crate::error::drive::DriveError;
use crate::error::Error;
use crate::util::object_size_info::key_value_info::KeyValueInfo::{KeyRefRequest, KeyValueMaxSize};

/// Key value info
#[derive(Clone)]
pub enum KeyValueInfo<'a> {
    /// A key by reference
    KeyRefRequest(&'a [u8]),
    /// Max size possible for value
    KeyValueMaxSize((u16, u32)),
}

impl<'a> KeyValueInfo<'a> {
    /// Returns key ref request
    pub fn as_key_ref_request(&'a self) -> Result<&'a [u8], Error> {
        match self {
            KeyRefRequest(key) => Ok(key),
            KeyValueMaxSize((_, _)) => Err(Error::Drive(DriveError::CorruptedCodeExecution(
                "requesting KeyValueInfo as key ref request however it is a key value max size",
            ))),
        }
    }

    /// Returns key length
    pub fn key_len(&'a self) -> u16 {
        match self {
            KeyRefRequest(key) => key.len() as u16,
            KeyValueMaxSize((key_size, _)) => *key_size,
        }
    }
}
