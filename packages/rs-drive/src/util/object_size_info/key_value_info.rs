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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_key_ref_request_ok_for_key_ref() {
        let bytes = [1u8, 2, 3];
        let info = KeyValueInfo::KeyRefRequest(&bytes);
        let got = info.as_key_ref_request().expect("should succeed");
        assert_eq!(got, &[1, 2, 3]);
    }

    #[test]
    fn as_key_ref_request_errors_for_max_size() {
        let info = KeyValueInfo::KeyValueMaxSize((32, 128));
        let err = info.as_key_ref_request().expect_err("should err");
        match err {
            Error::Drive(DriveError::CorruptedCodeExecution(msg)) => {
                assert!(msg.contains("key ref request"));
            }
            _ => panic!("expected CorruptedCodeExecution"),
        }
    }

    #[test]
    fn key_len_for_key_ref() {
        let bytes = [1u8; 7];
        let info = KeyValueInfo::KeyRefRequest(&bytes);
        assert_eq!(info.key_len(), 7);
    }

    #[test]
    fn key_len_for_max_size_uses_first_tuple_element() {
        let info = KeyValueInfo::KeyValueMaxSize((32, 1024));
        assert_eq!(info.key_len(), 32);
    }

    #[test]
    fn clone_preserves_ref_variant() {
        let bytes = [4u8, 5];
        let info = KeyValueInfo::KeyRefRequest(&bytes);
        let cloned = info.clone();
        assert_eq!(info.key_len(), cloned.key_len());
    }
}
