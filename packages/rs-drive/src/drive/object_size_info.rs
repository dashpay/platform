use crate::contract::{Contract, Document, DocumentType};
use crate::drive::defaults::DEFAULT_HASH_SIZE;
use crate::error::contract::ContractError;
use crate::error::drive::DriveError;
use crate::error::Error;
use grovedb::Element;
use KeyInfo::{Key, KeyRef, KeySize};
use KeyValueInfo::{KeyRefRequest, KeyValueMaxSize};
use PathInfo::{PathFixedSizeIterator, PathIterator, PathSize};
use PathKeyElementInfo::{PathFixedSizeKeyElement, PathKeyElement, PathKeyElementSize};
use PathKeyInfo::{PathFixedSizeKey, PathFixedSizeKeyRef, PathKey, PathKeyRef, PathKeySize};

#[derive(Clone)]
pub enum PathInfo<'a, const N: usize> {
    /// An into iter Path
    PathFixedSizeIterator([&'a [u8]; N]),

    /// An into iter Path
    PathIterator(Vec<Vec<u8>>),

    /// A path size
    PathSize(usize),
}

impl<'a, const N: usize> PathInfo<'a, N> {
    pub fn len(&self) -> usize {
        match self {
            PathFixedSizeIterator(path_iterator) => {
                (*path_iterator).into_iter().map(|a| a.len()).sum()
            }
            PathIterator(path_iterator) => path_iterator.clone().into_iter().map(|a| a.len()).sum(),
            PathSize(path_size) => *path_size,
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            PathFixedSizeIterator(path_iterator) => {
                (*path_iterator).into_iter().all(|a| a.is_empty())
            }
            PathIterator(path_iterator) => path_iterator.clone().into_iter().all(|a| a.is_empty()),
            PathSize(path_size) => *path_size == 0,
        }
    }

    pub fn push(&mut self, key_info: KeyInfo<'a>) -> Result<(), Error> {
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
                Key(key) => path_size += key.len(),
                KeyRef(key_ref) => path_size += key_ref.len(),
                KeySize(key_size) => path_size += key_size,
            },
        }
        Ok(())
    }
}

#[derive(Clone)]
pub enum KeyInfo<'a> {
    /// A key
    Key(Vec<u8>),
    /// A key by reference
    KeyRef(&'a [u8]),
    /// A key size
    KeySize(usize),
}

impl<'a> Default for KeyInfo<'a> {
    fn default() -> Self {
        Key(vec![])
    }
}

impl<'a> KeyInfo<'a> {
    pub fn len(&'a self) -> usize {
        match self {
            Key(key) => key.len(),
            KeyRef(key) => key.len(),
            KeySize(key_size) => *key_size,
        }
    }

    pub fn is_empty(&'a self) -> bool {
        match self {
            Key(key) => key.is_empty(),
            KeyRef(key) => key.is_empty(),
            KeySize(key_size) => *key_size == 0,
        }
    }

    pub fn add_path_info<const N: usize>(self, path_info: PathInfo<'a, N>) -> PathKeyInfo<'a, N> {
        match self {
            Key(key) => match path_info {
                PathFixedSizeIterator(iter) => PathFixedSizeKey((iter, key)),
                PathIterator(iter) => PathKey((iter, key)),
                PathSize(size) => PathKeySize((size, key.len())),
            },
            KeyRef(key_ref) => match path_info {
                PathFixedSizeIterator(iter) => PathFixedSizeKeyRef((iter, key_ref)),
                PathIterator(iter) => PathKeyRef((iter, key_ref)),
                PathSize(size) => PathKeySize((size, key_ref.len())),
            },
            KeySize(key_size) => PathKeySize((path_info.len(), key_size)),
        }
    }

    pub fn add_fixed_size_path<const N: usize>(self, path: [&'a [u8]; N]) -> PathKeyInfo<'a, N> {
        match self {
            Key(key) => PathFixedSizeKey((path, key)),
            KeyRef(key_ref) => PathFixedSizeKeyRef((path, key_ref)),
            KeySize(key_size) => PathKeySize((path.len(), key_size)),
        }
    }

    pub fn add_path<const N: usize>(self, path: Vec<Vec<u8>>) -> PathKeyInfo<'a, N> {
        match self {
            Key(key) => PathKey((path, key)),
            KeyRef(key_ref) => PathKeyRef((path, key_ref)),
            KeySize(key_size) => PathKeySize((path.len(), key_size)),
        }
    }
}

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
    PathKeySize((usize, usize)),
}

impl<'a, const N: usize> PathKeyInfo<'a, N> {
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
            PathKeySize((path_size, key_size)) => *path_size + *key_size,
        }
    }

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
}

pub enum ElementInfo {
    /// An element
    Element(Element),
    /// An element size
    ElementSize(usize),
}

pub enum KeyElementInfo<'a> {
    /// An element
    KeyElement((&'a [u8], Element)),
    /// An element size
    KeyElementSize((usize, usize)),
}

pub enum PathKeyElementInfo<'a, const N: usize> {
    /// A triple Path Key and Element
    PathFixedSizeKeyElement(([&'a [u8]; N], &'a [u8], Element)),
    /// A triple Path Key and Element
    PathKeyElement((Vec<Vec<u8>>, &'a [u8], Element)),
    /// A triple of sum of Path lengths, Key length and Element size
    PathKeyElementSize((usize, usize, usize)),
}

impl<'a, const N: usize> PathKeyElementInfo<'a, N> {
    pub fn from_path_info_and_key_element(
        path_info: PathInfo<'a, N>,
        key_element: KeyElementInfo<'a>,
    ) -> Result<Self, Error> {
        match path_info {
            PathIterator(path_interator) => match key_element {
                KeyElementInfo::KeyElement((key, element)) => {
                    Ok(PathKeyElement((path_interator, key, element)))
                }
                KeyElementInfo::KeyElementSize(_) => Err(Error::Drive(
                    DriveError::CorruptedCodeExecution("path matched with key element size"),
                )),
            },
            PathSize(path_size) => match key_element {
                KeyElementInfo::KeyElement((key, element)) => Ok(PathKeyElementSize((
                    path_size,
                    key.len(),
                    element.node_byte_size(key.len()),
                ))),
                KeyElementInfo::KeyElementSize((key_len, element_size)) => {
                    Ok(PathKeyElementSize((path_size, key_len, element_size)))
                }
            },
            PathFixedSizeIterator(path_interator) => match key_element {
                KeyElementInfo::KeyElement((key, element)) => {
                    Ok(PathFixedSizeKeyElement((path_interator, key, element)))
                }
                KeyElementInfo::KeyElementSize(_) => Err(Error::Drive(
                    DriveError::CorruptedCodeExecution("path matched with key element size"),
                )),
            },
        }
    }

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

    pub fn insert_len(&'a self) -> usize {
        match self {
            //todo v23: this is an incorrect approximation
            PathKeyElementInfo::PathKeyElement((_, key, element)) => {
                element.node_byte_size(key.len())
            }
            PathKeyElementInfo::PathKeyElementSize((_, key_size, element_size)) => {
                *key_size + *element_size
            }
            PathFixedSizeKeyElement((_, key, element)) => element.node_byte_size(key.len()),
        }
    }
}

pub struct DocumentAndContractInfo<'a> {
    pub document_info: DocumentInfo<'a>,
    pub contract: &'a Contract,
    pub document_type: &'a DocumentType,
    pub owner_id: Option<&'a [u8]>,
}

#[derive(Clone)]
pub enum DocumentInfo<'a> {
    /// The document and it's serialized form
    DocumentAndSerialization((&'a Document, &'a [u8])),
    /// An element size
    DocumentSize(usize),
}

impl<'a> DocumentInfo<'a> {
    pub fn is_document_and_serialization(&self) -> bool {
        match self {
            DocumentInfo::DocumentAndSerialization(_) => true,
            DocumentInfo::DocumentSize(_) => false,
        }
    }

    pub fn id_key_value_info(&self) -> KeyValueInfo {
        match self {
            DocumentInfo::DocumentAndSerialization((document, _)) => {
                KeyValueInfo::KeyRefRequest(document.id.as_slice())
            }
            DocumentInfo::DocumentSize(document_max_size) => {
                KeyValueInfo::KeyValueMaxSize((32, *document_max_size))
            }
        }
    }

    pub fn get_raw_for_document_type(
        &self,
        key_path: &str,
        document_type: &DocumentType,
        owner_id: Option<&[u8]>,
    ) -> Result<Option<KeyInfo>, Error> {
        match self {
            DocumentInfo::DocumentAndSerialization((document, _)) => {
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
                    let max_size = document_field_type.max_byte_size().ok_or({
                        Error::Drive(DriveError::CorruptedCodeExecution(
                            "document type must have a max size",
                        ))
                    })?;
                    Ok(Some(KeySize(max_size)))
                }
            },
        }
    }
}

#[derive(Clone)]
pub enum KeyValueInfo<'a> {
    /// A key by reference
    KeyRefRequest(&'a [u8]),
    /// Max size possible for value
    KeyValueMaxSize((usize, usize)),
}

impl<'a> KeyValueInfo<'a> {
    pub fn key_len(&'a self) -> usize {
        match self {
            KeyRefRequest(key) => key.len(),
            KeyValueMaxSize((key_size, _)) => *key_size,
        }
    }
}
