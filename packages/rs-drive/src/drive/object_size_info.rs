// MIT LICENSE
//
// Copyright (c) 2021 Dash Core Group
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
//

//! Object Size Info
//!
//! This module defines enums and implements functions relevant to the sizes of objects.
//!

use grovedb::Element;
use std::collections::HashSet;
use std::ops::AddAssign;

use DriveKeyInfo::{Key, KeyRef, KeySize};
use KeyValueInfo::{KeyRefRequest, KeyValueMaxSize};
use PathInfo::{PathFixedSizeIterator, PathIterator, PathSize};
use PathKeyElementInfo::{PathFixedSizeKeyElement, PathKeyElement, PathKeyElementSize};
use PathKeyInfo::{PathFixedSizeKey, PathFixedSizeKeyRef, PathKey, PathKeyRef, PathKeySize};

use crate::contract::document::Document;
use crate::contract::Contract;
use crate::drive::defaults::DEFAULT_HASH_SIZE;
use crate::drive::flags::StorageFlags;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::data_contract::extra::DocumentType;

use dpp::data_contract::extra::ContractError;

/// Info about a path.
#[derive(Clone)]
pub enum PathInfo<'a, const N: usize> {
    /// An into iter Path
    PathFixedSizeIterator([&'a [u8]; N]),

    /// An into iter Path
    PathIterator(Vec<Vec<u8>>),

    /// A path size
    PathSize(u32),
}

impl<'a, const N: usize> PathInfo<'a, N> {
    /// Returns the length of the path as a usize.
    pub fn len(&self) -> u32 {
        match self {
            PathFixedSizeIterator(path_iterator) => {
                (*path_iterator).into_iter().map(|a| a.len() as u32).sum()
            }
            PathIterator(path_iterator) => path_iterator
                .clone()
                .into_iter()
                .map(|a| a.len() as u32)
                .sum(),
            PathSize(path_size) => *path_size,
        }
    }

    /// Returns true if the path is empty.
    pub fn is_empty(&self) -> bool {
        match self {
            PathFixedSizeIterator(path_iterator) => {
                (*path_iterator).into_iter().all(|a| a.is_empty())
            }
            PathIterator(path_iterator) => path_iterator.clone().into_iter().all(|a| a.is_empty()),
            PathSize(path_size) => *path_size == 0,
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
                KeySize(_) => {
                    return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                        "can not add a key size to path iterator",
                    )))
                }
            },
            PathSize(mut path_size) => match key_info {
                Key(key) => path_size.add_assign(key.len() as u32),
                KeyRef(key_ref) => path_size.add_assign(key_ref.len() as u32),
                KeySize(key_size) => path_size.add_assign(key_size),
            },
        }
        Ok(())
    }
}

/// Key info
#[derive(Clone)]
pub enum DriveKeyInfo<'a> {
    /// A key
    Key(Vec<u8>),
    /// A key by reference
    KeyRef(&'a [u8]),
    /// A key size
    KeySize(u32),
}

impl<'a> Default for DriveKeyInfo<'a> {
    fn default() -> Self {
        Key(vec![])
    }
}

impl<'a> DriveKeyInfo<'a> {
    /// Returns the length of the key as a usize.
    pub fn len(&'a self) -> usize {
        match self {
            Key(key) => key.len(),
            KeyRef(key) => key.len(),
            KeySize(key_size) => *key_size as usize,
        }
    }

    /// Returns true if the key is empty.
    pub fn is_empty(&'a self) -> bool {
        match self {
            Key(key) => key.is_empty(),
            KeyRef(key) => key.is_empty(),
            KeySize(key_size) => *key_size == 0,
        }
    }

    /// Adds path info to the key. Returns `PathKeyInfo`.
    pub fn add_path_info<const N: usize>(self, path_info: PathInfo<'a, N>) -> PathKeyInfo<'a, N> {
        match self {
            Key(key) => match path_info {
                PathFixedSizeIterator(iter) => PathFixedSizeKey((iter, key)),
                PathIterator(iter) => PathKey((iter, key)),
                PathSize(size) => PathKeySize((size, key.len() as u32)),
            },
            KeyRef(key_ref) => match path_info {
                PathFixedSizeIterator(iter) => PathFixedSizeKeyRef((iter, key_ref)),
                PathIterator(iter) => PathKeyRef((iter, key_ref)),
                PathSize(size) => PathKeySize((size, key_ref.len() as u32)),
            },
            KeySize(key_size) => PathKeySize((path_info.len(), key_size)),
        }
    }

    /// Adds a fixed size path to the key. Returns `PathKeyInfo`.
    pub fn add_fixed_size_path<const N: usize>(self, path: [&'a [u8]; N]) -> PathKeyInfo<'a, N> {
        match self {
            Key(key) => PathFixedSizeKey((path, key)),
            KeyRef(key_ref) => PathFixedSizeKeyRef((path, key_ref)),
            KeySize(key_size) => PathKeySize((path.len() as u32, key_size)),
        }
    }

    /// Adds a path to the key. Returns `PathKeyInfo`.
    pub fn add_path<const N: usize>(self, path: Vec<Vec<u8>>) -> PathKeyInfo<'a, N> {
        match self {
            Key(key) => PathKey((path, key)),
            KeyRef(key_ref) => PathKeyRef((path, key_ref)),
            KeySize(key_size) => PathKeySize((path.len() as u32, key_size)),
        }
    }
}

/// Path key info
#[derive(Clone)]
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
    PathKeySize((u32, u32)),
}

impl<'a, const N: usize> PathKeyInfo<'a, N> {
    /// Returns the length of the path with key as a usize.
    pub fn len(&'a self) -> usize {
        match self {
            PathKey((path_iterator, key)) => {
                path_iterator
                    .clone()
                    .into_iter()
                    .map(|a| a.len())
                    .sum::<usize>()
                    + key.len()
            }
            PathKeyRef((path_iterator, key)) => {
                path_iterator
                    .clone()
                    .into_iter()
                    .map(|a| a.len())
                    .sum::<usize>()
                    + key.len()
            }
            PathFixedSizeKey((path_iterator, key)) => {
                (*path_iterator).into_iter().map(|a| a.len()).sum::<usize>() + key.len()
            }
            PathFixedSizeKeyRef((path_iterator, key)) => {
                (*path_iterator).into_iter().map(|a| a.len()).sum::<usize>() + key.len()
            }
            PathKeySize((path_size, key_size)) => *path_size as usize + *key_size as usize,
        }
    }

    /// Returns true if the path with key is empty.
    pub fn is_empty(&'a self) -> bool {
        match self {
            PathKey((path_iterator, key)) => {
                key.is_empty() && path_iterator.clone().into_iter().all(|a| a.is_empty())
            }
            PathKeyRef((path_iterator, key)) => {
                key.is_empty() && path_iterator.clone().into_iter().all(|a| a.is_empty())
            }
            PathFixedSizeKey((path_iterator, key)) => {
                key.is_empty() && (*path_iterator).into_iter().all(|a| a.is_empty())
            }
            PathFixedSizeKeyRef((path_iterator, key)) => {
                key.is_empty() && (*path_iterator).into_iter().all(|a| a.is_empty())
            }
            PathKeySize((path_size, key_size)) => (*path_size + *key_size) == 0,
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
            PathKeySize(_) => false,
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
            PathKeySize(_) => true,
        }
    }
}

/// Element info
pub enum ElementInfo {
    /// An element
    Element(Element),
    /// An element size
    ElementSize(u32),
}

/// Key element info
pub enum KeyElementInfo<'a> {
    /// An element
    KeyElement((&'a [u8], Element)),
    /// An element size
    KeyElementSize((u32, u32)),
}

/// Path key element info
pub enum PathKeyElementInfo<'a, const N: usize> {
    /// A triple Path Key and Element
    PathFixedSizeKeyElement(([&'a [u8]; N], &'a [u8], Element)),
    /// A triple Path Key and Element
    PathKeyElement((Vec<Vec<u8>>, &'a [u8], Element)),
    /// A triple of sum of Path lengths, Key length and Element size
    PathKeyElementSize((u32, u32, u32)),
}

impl<'a, const N: usize> PathKeyElementInfo<'a, N> {
    /// Create and return a `PathKeyElement` from `PathInfo` and `KeyElementInfo`
    pub fn from_path_info_and_key_element(
        path_info: PathInfo<'a, N>,
        key_element: KeyElementInfo<'a>,
    ) -> Result<Self, Error> {
        match path_info {
            PathIterator(path_iterator) => match key_element {
                KeyElementInfo::KeyElement((key, element)) => {
                    Ok(PathKeyElement((path_iterator, key, element)))
                }
                KeyElementInfo::KeyElementSize(_) => Err(Error::Drive(
                    DriveError::CorruptedCodeExecution("path matched with key element size"),
                )),
            },
            PathSize(path_size) => match key_element {
                KeyElementInfo::KeyElement((key, element)) => Ok(PathKeyElementSize((
                    path_size,
                    key.len() as u32,
                    element.node_byte_size(key.len() as u32),
                ))),
                KeyElementInfo::KeyElementSize((key_len, element_size)) => {
                    Ok(PathKeyElementSize((path_size, key_len, element_size)))
                }
            },
            PathFixedSizeIterator(path_iterator) => match key_element {
                KeyElementInfo::KeyElement((key, element)) => {
                    Ok(PathFixedSizeKeyElement((path_iterator, key, element)))
                }
                KeyElementInfo::KeyElementSize(_) => Err(Error::Drive(
                    DriveError::CorruptedCodeExecution("path matched with key element size"),
                )),
            },
        }
    }

    /// Create and return a `PathFixedSizeKeyElement` from a fixed-size path and `KeyElementInfo`
    pub fn from_fixed_size_path_and_key_element(
        path: [&'a [u8]; N],
        key_element: KeyElementInfo<'a>,
    ) -> Result<Self, Error> {
        match key_element {
            KeyElementInfo::KeyElement((key, element)) => {
                Ok(PathFixedSizeKeyElement((path, key, element)))
            }
            KeyElementInfo::KeyElementSize(_) => Err(Error::Drive(
                DriveError::CorruptedCodeExecution("path matched with key element size"),
            )),
        }
    }

    /// Create and return a `PathKeyElement` from a path and `KeyElementInfo`
    pub fn from_path_and_key_element(
        path: Vec<Vec<u8>>,
        key_element: KeyElementInfo<'a>,
    ) -> Result<Self, Error> {
        match key_element {
            KeyElementInfo::KeyElement((key, element)) => Ok(PathKeyElement((path, key, element))),
            KeyElementInfo::KeyElementSize(_) => Err(Error::Drive(
                DriveError::CorruptedCodeExecution("path matched with key element size"),
            )),
        }
    }

    /// Returns length of self
    pub fn insert_len(&'a self) -> u32 {
        match self {
            //todo v23: this is an incorrect approximation
            PathKeyElement((_, key, element)) => element.node_byte_size(key.len() as u32),
            PathKeyElementSize((_, key_size, element_size)) => *key_size + *element_size,
            PathFixedSizeKeyElement((_, key, element)) => element.node_byte_size(key.len() as u32),
        }
    }
}

/// Document and contract info
pub struct DocumentAndContractInfo<'a> {
    /// Document info
    pub document_info: DocumentInfo<'a>,
    /// Contract
    pub contract: &'a Contract,
    /// Document type
    pub document_type: &'a DocumentType,
    /// Owner ID
    pub owner_id: Option<[u8; 32]>,
}

/// Document info
#[derive(Clone)]
pub enum DocumentInfo<'a> {
    /// The borrowed document and it's serialized form
    DocumentRefAndSerialization((&'a Document, &'a [u8], Option<&'a StorageFlags>)),
    /// The borrowed document without it's serialized form
    DocumentRefWithoutSerialization((&'a Document, Option<&'a StorageFlags>)),
    /// The document without it's serialized form
    DocumentWithoutSerialization((Document, Option<StorageFlags>)),
    /// An element size
    DocumentSize(u32),
}

impl<'a> DocumentInfo<'a> {
    /// Returns true if self is a document with serialization.
    pub fn is_document_and_serialization(&self) -> bool {
        matches!(self, DocumentInfo::DocumentRefAndSerialization(..))
    }

    /// Makes the document ID the key.
    pub fn id_key_value_info(&self) -> KeyValueInfo {
        match self {
            DocumentInfo::DocumentRefAndSerialization((document, _, _))
            | DocumentInfo::DocumentRefWithoutSerialization((document, _)) => {
                KeyRefRequest(document.id.as_slice())
            }
            DocumentInfo::DocumentWithoutSerialization((document, _)) => {
                KeyRefRequest(document.id.as_slice())
            }
            DocumentInfo::DocumentSize(document_max_size) => {
                KeyValueMaxSize((32, *document_max_size))
            }
        }
    }

    /// Gets the raw path for the given document type
    pub fn get_raw_for_document_type(
        &self,
        key_path: &str,
        document_type: &DocumentType,
        owner_id: Option<[u8; 32]>,
    ) -> Result<Option<DriveKeyInfo>, Error> {
        match self {
            DocumentInfo::DocumentRefAndSerialization((document, _, _))
            | DocumentInfo::DocumentRefWithoutSerialization((document, _)) => {
                let raw_value =
                    document.get_raw_for_document_type(key_path, document_type, owner_id)?;
                match raw_value {
                    None => Ok(None),
                    Some(value) => Ok(Some(Key(value))),
                }
            }
            DocumentInfo::DocumentWithoutSerialization((document, _)) => {
                let raw_value =
                    document.get_raw_for_document_type(key_path, document_type, owner_id)?;
                match raw_value {
                    None => Ok(None),
                    Some(value) => Ok(Some(Key(value))),
                }
            }
            DocumentInfo::DocumentSize(_) => match key_path {
                "$ownerId" | "$id" => Ok(Some(KeySize(DEFAULT_HASH_SIZE))),
                _ => {
                    let document_field_type = document_type.properties.get(key_path).ok_or({
                        Error::Contract(ContractError::DocumentTypeFieldNotFound(
                            "incorrect key path for document type",
                        ))
                    })?;
                    let max_size = document_field_type.document_type.max_byte_size().ok_or({
                        Error::Drive(DriveError::CorruptedCodeExecution(
                            "document type must have a max size",
                        ))
                    })?;
                    Ok(Some(KeySize(max_size as u32)))
                }
            },
        }
    }

    /// Gets storage flags
    pub fn get_storage_flags_ref(&self) -> Option<&StorageFlags> {
        match self {
            DocumentInfo::DocumentRefAndSerialization((_, _, storage_flags))
            | DocumentInfo::DocumentRefWithoutSerialization((_, storage_flags)) => *storage_flags,
            DocumentInfo::DocumentWithoutSerialization((_, storage_flags)) => {
                storage_flags.as_ref()
            }
            DocumentInfo::DocumentSize(_) => StorageFlags::optional_default_as_ref(),
        }
    }
}

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

/// Deletion Info
pub struct DeletionInfo<'a, const N: usize> {
    upper_path: PathInfo<'a, N>,
    lower_path: Vec<KeyValueInfo<'a>>,
}
