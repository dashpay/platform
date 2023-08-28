use grovedb::batch::key_info::KeyInfo::KnownKey;
use grovedb::batch::KeyInfoPath;

use grovedb_storage::worst_case_costs::WorstKeyLength;

use DriveKeyInfo::{Key, KeyRef, KeySize};
use PathInfo::{PathFixedSizeIterator, PathIterator, PathWithSizes};

use crate::drive::object_size_info::drive_key_info::DriveKeyInfo;
use crate::error::drive::DriveError;
use crate::error::Error;

/// Info about a path.
#[derive(Clone)]
pub enum PathInfo<'a, const N: usize> {
    /// An into iter Path
    PathFixedSizeIterator([&'a [u8]; N]),

    /// An into iter Path
    PathIterator(Vec<Vec<u8>>),

    /// A path size
    PathWithSizes(KeyInfoPath),
}

impl<'a, const N: usize> PathInfo<'a, N> {
    /// Returns the length of the path as a usize.
    pub fn len(&self) -> u32 {
        match self {
            PathFixedSizeIterator(path_iterator) => {
                (*path_iterator).into_iter().map(|a| a.len() as u32).sum()
            }
            PathIterator(path_iterator) => path_iterator.iter().map(|a| a.len() as u32).sum(),
            PathWithSizes(path_size) => path_size.iterator().map(|a| a.max_length() as u32).sum(),
        }
    }

    /// Returns true if the path is empty.
    pub fn is_empty(&self) -> bool {
        match self {
            PathFixedSizeIterator(path_iterator) => (*path_iterator).is_empty(),
            PathIterator(path_iterator) => path_iterator.is_empty(),
            PathWithSizes(path_size) => path_size.is_empty(),
        }
    }

    /// Pushes the given key into the path.
    pub fn push(&mut self, key_info: DriveKeyInfo<'a>) -> Result<(), Error> {
        match self {
            PathFixedSizeIterator(_) => {
                return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                    "can not add a key to a fixed size path iterator",
                )))
            }
            PathIterator(path_iterator) => match key_info {
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
            PathFixedSizeIterator(path) => KeyInfoPath::from_known_path(path),
            PathIterator(path) => KeyInfoPath::from_known_owned_path(path),
            PathWithSizes(key_info_path) => key_info_path,
        }
    }
}
