#![allow(clippy::result_large_err)] // Internal helpers return drive::Error; size acceptable here
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::util::object_size_info::path_key_element_info::PathKeyElementInfo::{
    PathFixedSizeKeyRefElement, PathKeyElementSize, PathKeyRefElement, PathKeyUnknownElementSize,
};
use crate::util::object_size_info::PathInfo::{PathAsVec, PathFixedSizeArray, PathWithSizes};
use crate::util::object_size_info::{KeyElementInfo, PathInfo};
use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::key_info::KeyInfo::KnownKey;
use grovedb::batch::KeyInfoPath;
use grovedb::Element;

/// Path key element info
#[derive(Debug)]
pub enum PathKeyElementInfo<'a, const N: usize> {
    /// A triple Path Key and Element
    PathFixedSizeKeyRefElement(([&'a [u8]; N], &'a [u8], Element)),
    /// A triple Path Key and Element
    PathKeyRefElement((Vec<Vec<u8>>, &'a [u8], Element)),
    /// A triple Path Key and Element
    PathKeyElement((Vec<Vec<u8>>, Vec<u8>, Element)),
    /// A triple of sum of Path lengths, Key length and Element size
    PathKeyElementSize((KeyInfoPath, KeyInfo, Element)),
    /// A triple of sum of Path lengths, Key length and Element size
    PathKeyUnknownElementSize((KeyInfoPath, KeyInfo, u32)),
}

impl<'a, const N: usize> PathKeyElementInfo<'a, N> {
    /// Create and return a `PathKeyElement` from `PathInfo` and `KeyElementInfo`
    pub fn from_path_info_and_key_element(
        path_info: PathInfo<'a, N>,
        key_element: KeyElementInfo<'a>,
    ) -> Result<Self, Error> {
        match path_info {
            PathAsVec(path) => match key_element {
                KeyElementInfo::KeyElement((key, element)) => {
                    Ok(PathKeyRefElement((path, key, element)))
                }
                KeyElementInfo::KeyElementSize((key, element)) => Ok(PathKeyElementSize((
                    KeyInfoPath::from_known_owned_path(path),
                    key,
                    element,
                ))),
                KeyElementInfo::KeyUnknownElementSize(_) => Err(Error::Drive(
                    DriveError::NotSupportedPrivate("path matched with key element size"),
                )),
            },
            PathWithSizes(path_size) => match key_element {
                KeyElementInfo::KeyElement((key, element)) => Ok(PathKeyElementSize((
                    path_size,
                    KnownKey(key.to_vec()),
                    element,
                ))),
                KeyElementInfo::KeyElementSize((key_len, element)) => {
                    Ok(PathKeyElementSize((path_size, key_len, element)))
                }
                KeyElementInfo::KeyUnknownElementSize((key_len, element_size)) => Ok(
                    PathKeyUnknownElementSize((path_size, key_len, element_size)),
                ),
            },
            PathFixedSizeArray(path) => match key_element {
                KeyElementInfo::KeyElement((key, element)) => {
                    Ok(PathFixedSizeKeyRefElement((path, key, element)))
                }
                KeyElementInfo::KeyElementSize((key, element)) => Ok(PathKeyElementSize((
                    KeyInfoPath::from_known_path(path),
                    key,
                    element,
                ))),
                KeyElementInfo::KeyUnknownElementSize(_) => Err(Error::Drive(
                    DriveError::NotSupportedPrivate("path matched with key element size"),
                )),
            },
        }
    }

    /// Create and return a `PathFixedSizeKeyRefElement` from a fixed-size path and `KeyElementInfo`
    pub fn from_fixed_size_path_and_key_element(
        path: [&'a [u8]; N],
        key_element: KeyElementInfo<'a>,
    ) -> Result<Self, Error> {
        match key_element {
            KeyElementInfo::KeyElement((key, element)) => {
                Ok(PathFixedSizeKeyRefElement((path, key, element)))
            }
            KeyElementInfo::KeyElementSize((key, element)) => Ok(PathKeyElementSize((
                KeyInfoPath::from_known_path(path),
                key,
                element,
            ))),
            KeyElementInfo::KeyUnknownElementSize(_) => Err(Error::Drive(
                DriveError::NotSupportedPrivate("path matched with key element size"),
            )),
        }
    }

    /// Create and return a `PathKeyElement` from a path and `KeyElementInfo`
    pub fn from_path_and_key_element(
        path: Vec<Vec<u8>>,
        key_element: KeyElementInfo<'a>,
    ) -> Result<Self, Error> {
        match key_element {
            KeyElementInfo::KeyElement((key, element)) => {
                Ok(PathKeyRefElement((path, key, element)))
            }
            KeyElementInfo::KeyElementSize((key, element)) => Ok(PathKeyElementSize((
                KeyInfoPath::from_known_owned_path(path),
                key,
                element,
            ))),
            KeyElementInfo::KeyUnknownElementSize(_) => Err(Error::Drive(
                DriveError::NotSupportedPrivate("path matched with key element size"),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use grovedb::batch::key_info::KeyInfo::MaxKeySize;

    fn item_element() -> Element {
        Element::new_item(vec![1, 2, 3])
    }

    #[test]
    fn from_path_info_vec_with_key_element_produces_ref_element() {
        let path = PathInfo::<0>::PathAsVec(vec![vec![1u8]]);
        let key: &[u8] = &[9u8];
        let res = PathKeyElementInfo::from_path_info_and_key_element(
            path,
            KeyElementInfo::KeyElement((key, item_element())),
        )
        .expect("ok");
        assert!(matches!(res, PathKeyRefElement(_)));
    }

    #[test]
    fn from_path_info_vec_with_unknown_element_size_errors() {
        let path = PathInfo::<0>::PathAsVec(vec![vec![1u8]]);
        let err = PathKeyElementInfo::from_path_info_and_key_element(
            path,
            KeyElementInfo::KeyUnknownElementSize((KnownKey(vec![9]), 16)),
        )
        .expect_err("should err");
        match err {
            Error::Drive(DriveError::NotSupportedPrivate(msg)) => {
                assert!(msg.contains("key element size"));
            }
            _ => panic!("expected NotSupportedPrivate"),
        }
    }

    #[test]
    fn from_path_info_fixed_size_with_unknown_element_size_errors() {
        let path: [&[u8]; 1] = [&[1u8]];
        let res = PathKeyElementInfo::<1>::from_path_info_and_key_element(
            PathInfo::PathFixedSizeArray(path),
            KeyElementInfo::KeyUnknownElementSize((KnownKey(vec![9]), 10)),
        );
        match res {
            Err(Error::Drive(DriveError::NotSupportedPrivate(_))) => {}
            _ => panic!("expected error for unknown element size on fixed-size path"),
        }
    }

    #[test]
    fn from_path_info_fixed_size_with_key_element_produces_ref_element() {
        let path: [&[u8]; 1] = [&[1u8]];
        let key: &[u8] = &[9u8];
        let res = PathKeyElementInfo::<1>::from_path_info_and_key_element(
            PathInfo::PathFixedSizeArray(path),
            KeyElementInfo::KeyElement((key, item_element())),
        )
        .expect("ok");
        assert!(matches!(res, PathFixedSizeKeyRefElement(_)));
    }

    #[test]
    fn from_path_info_with_sizes_supports_unknown_element_size() {
        let path = PathInfo::<0>::PathWithSizes(KeyInfoPath::from_known_owned_path(vec![vec![1]]));
        let res = PathKeyElementInfo::from_path_info_and_key_element(
            path,
            KeyElementInfo::KeyUnknownElementSize((KnownKey(vec![9]), 100)),
        )
        .expect("path-with-sizes supports unknown element size");
        assert!(matches!(res, PathKeyUnknownElementSize(_)));
    }

    #[test]
    fn from_path_info_with_sizes_key_element_wraps_with_known_key() {
        let path = PathInfo::<0>::PathWithSizes(KeyInfoPath::from_known_owned_path(vec![vec![1]]));
        let key: &[u8] = &[9u8];
        let res = PathKeyElementInfo::from_path_info_and_key_element(
            path,
            KeyElementInfo::KeyElement((key, item_element())),
        )
        .expect("ok");
        assert!(matches!(res, PathKeyElementSize(_)));
    }

    #[test]
    fn from_path_info_with_sizes_element_size_roundtrips() {
        let path = PathInfo::<0>::PathWithSizes(KeyInfoPath::from_known_owned_path(vec![vec![1]]));
        let res = PathKeyElementInfo::from_path_info_and_key_element(
            path,
            KeyElementInfo::KeyElementSize((
                MaxKeySize {
                    unique_id: vec![0xA],
                    max_size: 4,
                },
                item_element(),
            )),
        )
        .expect("ok");
        assert!(matches!(res, PathKeyElementSize(_)));
    }

    #[test]
    fn from_fixed_size_path_and_key_element_happy_path() {
        let path: [&[u8]; 2] = [&[1u8], &[2u8]];
        let key: &[u8] = &[9u8];
        let res = PathKeyElementInfo::<2>::from_fixed_size_path_and_key_element(
            path,
            KeyElementInfo::KeyElement((key, item_element())),
        )
        .expect("ok");
        assert!(matches!(res, PathFixedSizeKeyRefElement(_)));
    }

    #[test]
    fn from_fixed_size_path_and_key_unknown_size_errors() {
        let path: [&[u8]; 1] = [&[1u8]];
        let res = PathKeyElementInfo::<1>::from_fixed_size_path_and_key_element(
            path,
            KeyElementInfo::KeyUnknownElementSize((KnownKey(vec![]), 16)),
        );
        match res {
            Err(Error::Drive(DriveError::NotSupportedPrivate(_))) => {}
            _ => panic!("expected NotSupportedPrivate"),
        }
    }

    #[test]
    fn from_fixed_size_path_and_key_element_size_ok() {
        let path: [&[u8]; 1] = [&[1u8]];
        let res = PathKeyElementInfo::<1>::from_fixed_size_path_and_key_element(
            path,
            KeyElementInfo::KeyElementSize((
                MaxKeySize {
                    unique_id: vec![0xA],
                    max_size: 8,
                },
                item_element(),
            )),
        )
        .expect("ok");
        assert!(matches!(res, PathKeyElementSize(_)));
    }

    #[test]
    fn from_path_and_key_element_variants() {
        let path = vec![vec![1u8]];
        let key: &[u8] = &[9u8];
        let res = PathKeyElementInfo::<0>::from_path_and_key_element(
            path.clone(),
            KeyElementInfo::KeyElement((key, item_element())),
        )
        .expect("ok");
        assert!(matches!(res, PathKeyRefElement(_)));

        let res = PathKeyElementInfo::<0>::from_path_and_key_element(
            path.clone(),
            KeyElementInfo::KeyElementSize((
                MaxKeySize {
                    unique_id: vec![],
                    max_size: 8,
                },
                item_element(),
            )),
        )
        .expect("ok");
        assert!(matches!(res, PathKeyElementSize(_)));

        let err = PathKeyElementInfo::<0>::from_path_and_key_element(
            path,
            KeyElementInfo::KeyUnknownElementSize((KnownKey(vec![]), 8)),
        )
        .expect_err("err");
        assert!(matches!(
            err,
            Error::Drive(DriveError::NotSupportedPrivate(_))
        ));
    }
}
