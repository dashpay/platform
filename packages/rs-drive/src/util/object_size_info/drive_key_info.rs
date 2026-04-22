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

impl Default for DriveKeyInfo<'_> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use grovedb::batch::key_info::KeyInfo::MaxKeySize;

    #[test]
    fn default_is_empty_key() {
        let d = DriveKeyInfo::default();
        assert!(d.is_empty());
        assert_eq!(d.len(), 0);
    }

    #[test]
    fn key_len_and_is_empty() {
        let k = DriveKeyInfo::Key(vec![1, 2, 3]);
        assert_eq!(k.len(), 3);
        assert!(!k.is_empty());

        let empty = DriveKeyInfo::Key(vec![]);
        assert!(empty.is_empty());
    }

    #[test]
    fn key_ref_len_and_is_empty() {
        let bytes = [1u8, 2, 3, 4];
        let k = DriveKeyInfo::KeyRef(&bytes);
        assert_eq!(k.len(), 4);
        assert!(!k.is_empty());

        let empty: &[u8] = &[];
        let k = DriveKeyInfo::KeyRef(empty);
        assert!(k.is_empty());
    }

    #[test]
    fn key_size_len_and_is_empty() {
        let info = KeyInfo::MaxKeySize {
            unique_id: vec![0xAB],
            max_size: 10,
        };
        let k = DriveKeyInfo::KeySize(info);
        assert_eq!(k.len(), 10);
        assert!(!k.is_empty());

        let zero_size = KeyInfo::MaxKeySize {
            unique_id: vec![],
            max_size: 0,
        };
        let k = DriveKeyInfo::KeySize(zero_size);
        assert!(k.is_empty());
    }

    #[test]
    fn to_owned_key_info_from_key() {
        let k = DriveKeyInfo::Key(vec![9, 8, 7]);
        match k.to_owned_key_info() {
            KnownKey(v) => assert_eq!(v, vec![9, 8, 7]),
            _ => panic!("expected KnownKey"),
        }
    }

    #[test]
    fn to_owned_key_info_from_key_ref() {
        let bytes = [4u8, 5, 6];
        let k = DriveKeyInfo::KeyRef(&bytes);
        match k.to_owned_key_info() {
            KnownKey(v) => assert_eq!(v, vec![4, 5, 6]),
            _ => panic!("expected KnownKey"),
        }
    }

    #[test]
    fn to_owned_key_info_preserves_key_size() {
        let info = KeyInfo::MaxKeySize {
            unique_id: vec![0x11],
            max_size: 5,
        };
        let k = DriveKeyInfo::KeySize(info.clone());
        assert_eq!(k.to_owned_key_info(), info);
    }

    #[test]
    fn to_key_info_borrows_and_clones() {
        let k = DriveKeyInfo::Key(vec![1, 2]);
        let info_a = k.to_key_info();
        // k is still usable (not consumed)
        let info_b = k.to_key_info();
        assert_eq!(info_a, info_b);
        match info_a {
            KnownKey(v) => assert_eq!(v, vec![1, 2]),
            _ => panic!("expected KnownKey"),
        }
    }

    #[test]
    fn add_fixed_size_path_variants() {
        // Key -> PathFixedSizeKey
        let k = DriveKeyInfo::Key(vec![1]);
        let path: [&[u8]; 2] = [&[0u8, 1], &[2u8, 3]];
        let pk = k.add_fixed_size_path(path);
        match pk {
            PathKeyInfo::PathFixedSizeKey((p, key)) => {
                assert_eq!(p.len(), 2);
                assert_eq!(key, vec![1]);
            }
            _ => panic!("expected PathFixedSizeKey"),
        }

        // KeyRef -> PathFixedSizeKeyRef
        let bytes = [5u8];
        let k = DriveKeyInfo::KeyRef(&bytes);
        let pk = k.add_fixed_size_path(path);
        assert!(matches!(pk, PathKeyInfo::PathFixedSizeKeyRef(_)));

        // KeySize -> PathKeySize
        let k = DriveKeyInfo::KeySize(MaxKeySize {
            unique_id: vec![],
            max_size: 8,
        });
        let pk = k.add_fixed_size_path(path);
        assert!(matches!(pk, PathKeyInfo::PathKeySize(..)));
    }

    #[test]
    fn add_path_variants_for_vec_path() {
        let path = vec![vec![0u8], vec![1u8, 2u8]];

        // Key
        let k = DriveKeyInfo::Key(vec![9]);
        let pk: PathKeyInfo<0> = k.add_path(path.clone());
        assert!(matches!(pk, PathKeyInfo::PathKey(_)));

        // KeyRef
        let bytes = [7u8];
        let k = DriveKeyInfo::KeyRef(&bytes);
        let pk: PathKeyInfo<0> = k.add_path(path.clone());
        assert!(matches!(pk, PathKeyInfo::PathKeyRef(_)));

        // KeySize
        let k = DriveKeyInfo::KeySize(MaxKeySize {
            unique_id: vec![],
            max_size: 3,
        });
        let pk: PathKeyInfo<0> = k.add_path(path);
        assert!(matches!(pk, PathKeyInfo::PathKeySize(..)));
    }

    #[test]
    fn clone_preserves_variant() {
        let k = DriveKeyInfo::Key(vec![1, 2, 3]);
        let cloned = k.clone();
        assert_eq!(k.len(), cloned.len());
    }
}
