use crate::error::drive::DriveError;
use crate::error::Error;
use crate::util::object_size_info::path_key_info::PathKeyInfo::{
    PathFixedSizeKey, PathFixedSizeKeyRef, PathKey, PathKeyRef, PathKeySize,
};
use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::key_info::KeyInfo::KnownKey;
use grovedb::batch::KeyInfoPath;
use grovedb_storage::worst_case_costs::WorstKeyLength;
use std::collections::HashSet;

/// Path key info
#[derive(Clone, Debug)]
pub enum PathKeyInfo<'a, const N: usize> {
    /// An into iter Path with a Key
    PathFixedSizeKey(([&'a [u8]; N], Vec<u8>)),
    /// An into iter Path with a Key
    PathFixedSizeKeyRef(([&'a [u8]; N], &'a [u8])),

    /// An into iter Path with a Key
    PathKey((Vec<Vec<u8>>, Vec<u8>)),
    /// An into iter Path with a Key
    PathKeyRef((Vec<Vec<u8>>, &'a [u8])),
    /// A path size
    PathKeySize(KeyInfoPath, KeyInfo),
}

impl<'a> TryFrom<Vec<Vec<u8>>> for PathKeyInfo<'a, 0> {
    type Error = Error;

    fn try_from(mut value: Vec<Vec<u8>>) -> Result<Self, Self::Error> {
        if value.is_empty() {
            Err(Error::Drive(DriveError::InvalidPath(
                "path must not be none to convert into a path key info",
            )))
        } else {
            let last = value.remove(value.len() - 1);
            Ok(PathKey((value, last)))
        }
    }
}

impl<'a, const N: usize> PathKeyInfo<'a, N> {
    /// Returns the length of the path with key as a usize.
    pub fn len(&'a self) -> u32 {
        match self {
            PathKey((path_iterator, key)) => {
                path_iterator.iter().map(|a| a.len() as u32).sum::<u32>() + key.len() as u32
            }
            PathKeyRef((path_iterator, key)) => {
                path_iterator.iter().map(|a| a.len() as u32).sum::<u32>() + key.len() as u32
            }
            PathFixedSizeKey((path_iterator, key)) => {
                (*path_iterator).iter().map(|a| a.len() as u32).sum::<u32>() + key.len() as u32
            }
            PathFixedSizeKeyRef((path_iterator, key)) => {
                (*path_iterator).iter().map(|a| a.len() as u32).sum::<u32>() + key.len() as u32
            }
            PathKeySize(key_info_path, key_size) => {
                key_info_path
                    .iterator()
                    .map(|a| a.max_length() as u32)
                    .sum::<u32>()
                    + key_size.max_length() as u32
            }
        }
    }

    /// Returns true if the path with key is empty.
    pub fn is_empty(&'a self) -> bool {
        match self {
            PathKey((path_iterator, key)) => {
                key.is_empty() && path_iterator.iter().all(|a| a.is_empty())
            }
            PathKeyRef((path_iterator, key)) => {
                key.is_empty() && path_iterator.iter().all(|a| a.is_empty())
            }
            PathFixedSizeKey((path_iterator, key)) => {
                key.is_empty() && (*path_iterator).iter().all(|a| a.is_empty())
            }
            PathFixedSizeKeyRef((path_iterator, key)) => {
                key.is_empty() && (*path_iterator).iter().all(|a| a.is_empty())
            }
            PathKeySize(path_info, key_info) => path_info.is_empty() && key_info.max_length() == 0,
        }
    }

    /// Returns true if the path with key is in cache.
    pub fn is_contained_in_cache(&'a self, qualified_paths: &HashSet<Vec<Vec<u8>>>) -> bool {
        match self {
            PathKey((path, key)) => {
                let mut qualified_path = path.clone();
                qualified_path.push(key.clone());
                qualified_paths.contains(&qualified_path)
            }
            PathKeyRef((path, key)) => {
                let mut qualified_path = path.clone();
                qualified_path.push(key.to_vec());
                qualified_paths.contains(&qualified_path)
            }
            PathFixedSizeKey((path, key)) => {
                let mut qualified_path = path.map(|a| a.to_vec()).to_vec();
                qualified_path.push(key.clone());
                qualified_paths.contains(&qualified_path)
            }
            PathFixedSizeKeyRef((path, key)) => {
                let mut qualified_path = path.map(|a| a.to_vec()).to_vec();
                qualified_path.push(key.to_vec());
                qualified_paths.contains(&qualified_path)
            }
            PathKeySize(path_info, key_info) => {
                let mut qualified_path = path_info.to_path();
                qualified_path.push(key_info.get_key_clone());
                qualified_paths.contains(&qualified_path)
            }
        }
    }

    /// Adds the path with key to cache.
    pub fn add_to_cache(&'a self, qualified_paths: &mut HashSet<Vec<Vec<u8>>>) -> bool {
        match self {
            PathKey((path, key)) => {
                let mut qualified_path = path.clone();
                qualified_path.push(key.clone());
                qualified_paths.insert(qualified_path)
            }
            PathKeyRef((path, key)) => {
                let mut qualified_path = path.clone();
                qualified_path.push(key.to_vec());
                qualified_paths.insert(qualified_path)
            }
            PathFixedSizeKey((path, key)) => {
                let mut qualified_path = path.map(|a| a.to_vec()).to_vec();
                qualified_path.push(key.clone());
                qualified_paths.insert(qualified_path)
            }
            PathFixedSizeKeyRef((path, key)) => {
                let mut qualified_path = path.map(|a| a.to_vec()).to_vec();
                qualified_path.push(key.to_vec());
                qualified_paths.insert(qualified_path)
            }
            PathKeySize(path_info, key_info) => {
                let mut qualified_path = path_info.to_path();
                qualified_path.push(key_info.get_key_clone());
                qualified_paths.insert(qualified_path)
            }
        }
    }

    /// Get the KeyInfoPath for grovedb estimated costs
    pub(crate) fn convert_to_key_info_path(self) -> Result<KeyInfoPath, Error> {
        match self {
            PathKey((path, key)) => {
                let mut key_info_path = KeyInfoPath::from_known_owned_path(path);
                key_info_path.push(KnownKey(key));
                Ok(key_info_path)
            }
            PathKeyRef((path, key)) => {
                let mut key_info_path = KeyInfoPath::from_known_owned_path(path);
                key_info_path.push(KnownKey(key.to_vec()));
                Ok(key_info_path)
            }
            PathFixedSizeKey((path, key)) => {
                let mut key_info_path = KeyInfoPath::from_known_path(path);
                key_info_path.push(KnownKey(key));
                Ok(key_info_path)
            }
            PathFixedSizeKeyRef((path, key)) => {
                let mut key_info_path = KeyInfoPath::from_known_path(path);
                key_info_path.push(KnownKey(key.to_vec()));
                Ok(key_info_path)
            }
            PathKeySize(path_info, key_info) => {
                let mut path = path_info;
                path.push(key_info);
                Ok(path)
            }
        }
    }
}
