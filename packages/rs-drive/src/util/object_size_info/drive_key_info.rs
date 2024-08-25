use crate::util::object_size_info::path_key_info::PathKeyInfo;
use crate::util::object_size_info::path_key_info::PathKeyInfo::{
    PathFixedSizeKey, PathFixedSizeKeyRef, PathKey, PathKeyRef, PathKeySize,
};
use crate::util::object_size_info::PathInfo;
use crate::util::object_size_info::PathInfo::{PathAsVec, PathFixedSizeArray, PathWithSizes};
use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::key_info::KeyInfo::KnownKey;
use grovedb::batch::KeyInfoPath;
use grovedb_storage::worst_case_costs::WorstKeyLength;
use DriveKeyInfo::{Key, KeyRef, KeySize};

/// Key info
#[derive(Clone)]
pub enum DriveKeyInfo<'a> {
    /// A key
    Key(Vec<u8>),
    /// A key by reference
    KeyRef(&'a [u8]),
    /// A key size
    KeySize(KeyInfo),
}

impl<'a> Default for DriveKeyInfo<'a> {
    fn default() -> Self {
        Key(vec![])
    }
}

impl<'a> DriveKeyInfo<'a> {
    /// Returns the length of the key as a usize.
    pub fn len(&'a self) -> u32 {
        match self {
            Key(key) => key.len() as u32,
            KeyRef(key) => key.len() as u32,
            KeySize(info) => info.max_length() as u32,
        }
    }

    /// Returns true if the key is empty.
    pub fn is_empty(&'a self) -> bool {
        match self {
            Key(key) => key.is_empty(),
            KeyRef(key) => key.is_empty(),
            KeySize(info) => info.max_length() == 0,
        }
    }

    /// Adds path info to the key. Returns `PathKeyInfo`.
    pub fn add_path_info<const N: usize>(self, path_info: PathInfo<'a, N>) -> PathKeyInfo<'a, N> {
        match self {
            Key(key) => match path_info {
                PathFixedSizeArray(iter) => PathFixedSizeKey((iter, key)),
                PathAsVec(iter) => PathKey((iter, key)),
                PathWithSizes(key_info_path) => PathKeySize(key_info_path, KnownKey(key)),
            },
            KeyRef(key_ref) => match path_info {
                PathFixedSizeArray(iter) => PathFixedSizeKeyRef((iter, key_ref)),
                PathAsVec(iter) => PathKeyRef((iter, key_ref)),
                PathWithSizes(key_info_path) => {
                    PathKeySize(key_info_path, KnownKey(key_ref.to_vec()))
                }
            },
            KeySize(key_info) => match path_info {
                PathFixedSizeArray(iter) => {
                    PathKeySize(KeyInfoPath::from_known_path(iter), key_info)
                }
                PathAsVec(iter) => PathKeySize(KeyInfoPath::from_known_owned_path(iter), key_info),
                PathWithSizes(key_info_path) => PathKeySize(key_info_path, key_info),
            },
        }
    }

    /// Adds a fixed size path to the key. Returns `PathKeyInfo`.
    pub fn add_fixed_size_path<const N: usize>(self, path: [&'a [u8]; N]) -> PathKeyInfo<'a, N> {
        match self {
            Key(key) => PathFixedSizeKey((path, key)),
            KeyRef(key_ref) => PathFixedSizeKeyRef((path, key_ref)),
            KeySize(key_info) => PathKeySize(KeyInfoPath::from_known_path(path), key_info),
        }
    }

    /// Adds a path to the key. Returns `PathKeyInfo`.
    pub fn add_path<const N: usize>(self, path: Vec<Vec<u8>>) -> PathKeyInfo<'a, N> {
        match self {
            Key(key) => PathKey((path, key)),
            KeyRef(key_ref) => PathKeyRef((path, key_ref)),
            KeySize(key_info) => PathKeySize(KeyInfoPath::from_known_owned_path(path), key_info),
        }
    }

    /// Convert to a KeyInfo
    pub fn to_owned_key_info(self) -> KeyInfo {
        match self {
            Key(key) => KnownKey(key),
            KeyRef(key_ref) => KnownKey(key_ref.to_vec()),
            KeySize(key_info) => key_info,
        }
    }

    /// Convert to a KeyInfo
    pub fn to_key_info(&self) -> KeyInfo {
        match self {
            Key(key) => KnownKey(key.clone()),
            KeyRef(key_ref) => KnownKey(key_ref.to_vec()),
            KeySize(key_info) => key_info.clone(),
        }
    }
}
