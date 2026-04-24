use grovedb::batch::key_info::KeyInfo::KnownKey;
use grovedb::batch::KeyInfoPath;

use grovedb_storage::worst_case_costs::WorstKeyLength;

use DriveKeyInfo::{Key, KeyRef, KeySize};
use PathInfo::{PathAsVec, PathFixedSizeArray, PathWithSizes};

use crate::error::drive::DriveError;
use crate::error::Error;
use crate::util::object_size_info::drive_key_info::DriveKeyInfo;

/// Info about a path.
#[derive(Clone, Debug)]
pub enum PathInfo<'a, const N: usize> {
    /// An into iter Path
    PathFixedSizeArray([&'a [u8]; N]),

    /// An into iter Path
    PathAsVec(Vec<Vec<u8>>),

    /// A path size
    PathWithSizes(KeyInfoPath),
}

impl<'a, const N: usize> PathInfo<'a, N> {
    /// Returns the length of the path as a usize.
    pub fn len(&self) -> u32 {
        match self {
            PathFixedSizeArray(path_iterator) => {
                (*path_iterator).into_iter().map(|a| a.len() as u32).sum()
            }
            PathAsVec(path_iterator) => path_iterator.iter().map(|a| a.len() as u32).sum(),
            PathWithSizes(path_size) => path_size.iterator().map(|a| a.max_length() as u32).sum(),
        }
    }

    /// Returns true if the path is empty.
    pub fn is_empty(&self) -> bool {
        match self {
            PathFixedSizeArray(path_iterator) => (*path_iterator).is_empty(),
            PathAsVec(path_iterator) => path_iterator.is_empty(),
            PathWithSizes(path_size) => path_size.is_empty(),
        }
    }

    /// Pushes the given key into the path.
    pub fn push(&mut self, key_info: DriveKeyInfo<'a>) -> Result<(), Error> {
        match self {
            PathFixedSizeArray(_) => {
                return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                    "can not add a key to a fixed size path iterator",
                )))
            }
            PathAsVec(path_iterator) => match key_info {
                Key(key) => path_iterator.push(key),
                KeyRef(key_ref) => path_iterator.push(key_ref.to_vec()),
                KeySize(..) => {
                    return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                        "can not add a key size to path iterator",
                    )))
                }
            },
            PathWithSizes(key_info_path) => match key_info {
                Key(key) => key_info_path.push(KnownKey(key)),
                KeyRef(key_ref) => key_info_path.push(KnownKey(key_ref.to_vec())),
                KeySize(key_info) => key_info_path.push(key_info),
            },
        }
        Ok(())
    }

    /// Get the KeyInfoPath for grovedb estimated costs
    pub(crate) fn convert_to_key_info_path(self) -> KeyInfoPath {
        match self {
            PathFixedSizeArray(path) => KeyInfoPath::from_known_path(path),
            PathAsVec(path) => KeyInfoPath::from_known_owned_path(path),
            PathWithSizes(key_info_path) => key_info_path,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use grovedb::batch::key_info::KeyInfo::MaxKeySize;

    #[test]
    fn fixed_size_array_len_sums_elements() {
        let arr: [&[u8]; 3] = [&[1u8], &[2u8, 3u8], &[4u8, 5u8, 6u8]];
        let p: PathInfo<3> = PathInfo::PathFixedSizeArray(arr);
        assert_eq!(p.len(), 6);
        assert!(!p.is_empty());
    }

    #[test]
    fn fixed_size_empty_array_reports_zero_len() {
        let arr: [&[u8]; 0] = [];
        let p: PathInfo<0> = PathInfo::PathFixedSizeArray(arr);
        assert_eq!(p.len(), 0);
        assert!(p.is_empty());
    }

    #[test]
    fn as_vec_len_and_empty() {
        let p: PathInfo<0> = PathInfo::PathAsVec(vec![vec![1u8, 2], vec![3u8]]);
        assert_eq!(p.len(), 3);
        assert!(!p.is_empty());

        let p: PathInfo<0> = PathInfo::PathAsVec(vec![]);
        assert!(p.is_empty());
        assert_eq!(p.len(), 0);
    }

    #[test]
    fn with_sizes_len_uses_max_length() {
        let path = KeyInfoPath::from_known_path([b"abc".as_ref(), b"defg".as_ref()]);
        let p: PathInfo<0> = PathInfo::PathWithSizes(path);
        assert_eq!(p.len(), 7);
        assert!(!p.is_empty());
    }

    #[test]
    fn push_to_fixed_size_errors() {
        let arr: [&[u8]; 1] = [&[1u8]];
        let mut p: PathInfo<1> = PathInfo::PathFixedSizeArray(arr);
        let err = p.push(DriveKeyInfo::Key(vec![9])).expect_err("err");
        match err {
            Error::Drive(DriveError::CorruptedCodeExecution(msg)) => {
                assert!(msg.contains("fixed size"));
            }
            _ => panic!("expected CorruptedCodeExecution"),
        }
    }

    #[test]
    fn push_key_to_vec_appends_vec() {
        let mut p: PathInfo<0> = PathInfo::PathAsVec(vec![]);
        p.push(DriveKeyInfo::Key(vec![1, 2])).expect("push");
        let bytes: [u8; 1] = [3];
        p.push(DriveKeyInfo::KeyRef(&bytes)).expect("push ref");
        match p {
            PathInfo::PathAsVec(v) => {
                assert_eq!(v, vec![vec![1u8, 2], vec![3u8]]);
            }
            _ => panic!("expected vec"),
        }
    }

    #[test]
    fn push_key_size_to_vec_errors() {
        let mut p: PathInfo<0> = PathInfo::PathAsVec(vec![]);
        let err = p
            .push(DriveKeyInfo::KeySize(MaxKeySize {
                unique_id: vec![],
                max_size: 4,
            }))
            .expect_err("should err");
        match err {
            Error::Drive(DriveError::CorruptedCodeExecution(msg)) => {
                assert!(msg.contains("key size"));
            }
            _ => panic!("expected CorruptedCodeExecution"),
        }
    }

    #[test]
    fn push_all_variants_to_path_with_sizes() {
        let mut p: PathInfo<0> =
            PathInfo::PathWithSizes(KeyInfoPath::from_known_owned_path(vec![]));

        // Key
        p.push(DriveKeyInfo::Key(vec![1])).expect("push key");
        // KeyRef
        let bytes = [2u8];
        p.push(DriveKeyInfo::KeyRef(&bytes)).expect("push ref");
        // KeySize
        p.push(DriveKeyInfo::KeySize(MaxKeySize {
            unique_id: vec![0xA],
            max_size: 3,
        }))
        .expect("push size");

        match p {
            PathInfo::PathWithSizes(kip) => assert_eq!(kip.len(), 3),
            _ => panic!("expected PathWithSizes"),
        }
    }

    #[test]
    fn convert_to_key_info_path_fixed_size_array() {
        let arr: [&[u8]; 2] = [&[1u8], &[2u8, 3u8]];
        let p: PathInfo<2> = PathInfo::PathFixedSizeArray(arr);
        let kip = p.convert_to_key_info_path();
        assert_eq!(kip.len(), 2);
    }

    #[test]
    fn convert_to_key_info_path_as_vec() {
        let p: PathInfo<0> = PathInfo::PathAsVec(vec![vec![1], vec![2, 3]]);
        let kip = p.convert_to_key_info_path();
        assert_eq!(kip.len(), 2);
    }

    #[test]
    fn convert_to_key_info_path_passthrough() {
        let orig = KeyInfoPath::from_known_path([b"x".as_ref()]);
        let p: PathInfo<0> = PathInfo::PathWithSizes(orig);
        let kip = p.convert_to_key_info_path();
        assert_eq!(kip.len(), 1);
    }
}
