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
