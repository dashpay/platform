use crate::drive::defaults::{
    DEFAULT_HASH_SIZE_U16, DEFAULT_HASH_SIZE_U8, U32_SIZE_U16, U32_SIZE_U8, U64_SIZE_U16,
    U64_SIZE_U8,
};
use crate::drive::flags::StorageFlags;
use crate::drive::object_size_info::DriveKeyInfo::{Key, KeySize};
use crate::drive::object_size_info::KeyValueInfo::{KeyRefRequest, KeyValueMaxSize};
use crate::drive::object_size_info::{DriveKeyInfo, KeyValueInfo};
use crate::error::drive::DriveError;
use crate::error::fee::FeeError;
use crate::error::Error;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::data_contract::document_type::{DocumentTypeRef, IndexLevel};
use dpp::document::document_methods::DocumentMethodsV0;
use dpp::document::{Document, DocumentV0Getters};
use dpp::version::PlatformVersion;
use grovedb::batch::key_info::KeyInfo;
use std::borrow::Cow;

/// Document info
#[derive(Clone, Debug)]
pub enum DocumentInfo<'a> {
    /// The document without it's serialized form
    DocumentOwnedInfo((Document, Option<Cow<'a, StorageFlags>>)),
    /// The borrowed document without it's serialized form
    DocumentRefInfo((&'a Document, Option<Cow<'a, StorageFlags>>)),
    /// The borrowed document and it's serialized form
    DocumentRefAndSerialization((&'a Document, &'a [u8], Option<Cow<'a, StorageFlags>>)),
    /// The document and it's serialized form
    DocumentAndSerialization((Document, Vec<u8>, Option<Cow<'a, StorageFlags>>)),
    /// An element size
    DocumentEstimatedAverageSize(u32),
}

/// DocumentInfo V0 Methods
pub trait DocumentInfoV0Methods {
    /// Returns true if self is a document with serialization.
    fn is_document_and_serialization(&self) -> bool;
    /// Returns true if self is a document size.
    fn is_document_size(&self) -> bool;
    /// Gets the borrowed document
    fn get_borrowed_document(&self) -> Option<&Document>;
    /// Makes the document ID the key.
    fn id_key_value_info(&self) -> KeyValueInfo;
    /// Gets the raw path for the given document type
    fn get_estimated_size_for_document_type(
        &self,
        key_path: &str,
        document_type: DocumentTypeRef,
    ) -> Result<u16, Error>;
    /// Gets the raw path for the given document type
    fn get_raw_for_document_type(
        &self,
        key_path: &str,
        document_type: DocumentTypeRef,
        owner_id: Option<[u8; 32]>,
        size_info_with_base_event: Option<(&IndexLevel, [u8; 32])>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<DriveKeyInfo>, Error>;
    /// Gets the borrowed document
    fn get_borrowed_document_and_storage_flags(&self)
        -> Option<(&Document, Option<&StorageFlags>)>;
    /// Gets storage flags
    fn get_storage_flags_ref(&self) -> Option<&StorageFlags>;
    /// Gets storage flags
    fn get_document_id_as_slice(&self) -> Option<&[u8]>;
}

impl<'a> DocumentInfoV0Methods for DocumentInfo<'a> {
    /// Returns true if self is a document with serialization.
    fn is_document_and_serialization(&self) -> bool {
        matches!(self, DocumentInfo::DocumentRefAndSerialization(..))
    }

    /// Returns true if self is a document size.
    fn is_document_size(&self) -> bool {
        matches!(self, DocumentInfo::DocumentEstimatedAverageSize(_))
    }

    /// Gets the borrowed document
    fn get_borrowed_document(&self) -> Option<&Document> {
        match self {
            DocumentInfo::DocumentRefAndSerialization((document, _, _))
            | DocumentInfo::DocumentRefInfo((document, _)) => Some(document),
            DocumentInfo::DocumentOwnedInfo((document, _))
            | DocumentInfo::DocumentAndSerialization((document, _, _)) => Some(document),
            DocumentInfo::DocumentEstimatedAverageSize(_) => None,
        }
    }

    /// Makes the document ID the key.
    fn id_key_value_info(&self) -> KeyValueInfo {
        match self {
            DocumentInfo::DocumentRefAndSerialization((document, _, _))
            | DocumentInfo::DocumentRefInfo((document, _)) => {
                KeyRefRequest(document.id_ref().as_slice())
            }
            DocumentInfo::DocumentOwnedInfo((document, _))
            | DocumentInfo::DocumentAndSerialization((document, _, _)) => {
                KeyRefRequest(document.id_ref().as_slice())
            }
            DocumentInfo::DocumentEstimatedAverageSize(document_max_size) => {
                KeyValueMaxSize((32, *document_max_size))
            }
        }
    }

    /// Gets the raw path for the given document type
    fn get_estimated_size_for_document_type(
        &self,
        key_path: &str,
        document_type: DocumentTypeRef,
    ) -> Result<u16, Error> {
        match key_path {
            "$ownerId" | "$id" => Ok(DEFAULT_HASH_SIZE_U16),
            "$createdAt" | "$updatedAt" | "$transferredAt" => Ok(U64_SIZE_U16),
            "$createdAtBlockHeight" | "$updatedAtBlockHeight" | "$transferredAtBlockHeight" => {
                Ok(U64_SIZE_U16)
            }
            "$createdAtCoreBlockHeight"
            | "$updatedAtCoreBlockHeight"
            | "$transferredAtCoreBlockHeight" => Ok(U32_SIZE_U16),
            _ => {
                let property = document_type.flattened_properties().get(key_path).ok_or({
                    Error::Fee(FeeError::DocumentTypeFieldNotFoundForEstimation(
                        "incorrect key path for document type for estimated sizes",
                    ))
                })?;
                let estimated_size = property.property_type.middle_byte_size_ceil().ok_or({
                    Error::Drive(DriveError::CorruptedCodeExecution(
                        "document type must have a max size",
                    ))
                })?;
                Ok(estimated_size)
            }
        }
    }

    /// Gets the raw path for the given document type
    fn get_raw_for_document_type(
        &self,
        key_path: &str,
        document_type: DocumentTypeRef,
        owner_id: Option<[u8; 32]>,
        size_info_with_base_event: Option<(&IndexLevel, [u8; 32])>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<DriveKeyInfo>, Error> {
        match self {
            DocumentInfo::DocumentRefAndSerialization((document, _, _))
            | DocumentInfo::DocumentRefInfo((document, _)) => {
                let raw_value = document.get_raw_for_document_type(
                    key_path,
                    document_type,
                    owner_id,
                    platform_version,
                )?;
                match raw_value {
                    None => Ok(None),
                    Some(value) => Ok(Some(Key(value))),
                }
            }
            DocumentInfo::DocumentOwnedInfo((document, _))
            | DocumentInfo::DocumentAndSerialization((document, _, _)) => {
                let raw_value = document.get_raw_for_document_type(
                    key_path,
                    document_type,
                    owner_id,
                    platform_version,
                )?;
                match raw_value {
                    None => Ok(None),
                    Some(value) => Ok(Some(Key(value))),
                }
            }
            DocumentInfo::DocumentEstimatedAverageSize(_) => {
                let (index_level, base_event) = size_info_with_base_event.ok_or(Error::Drive(
                    DriveError::CorruptedCodeExecution("size_info_with_base_event None but needed"),
                ))?;
                match key_path {
                    "$ownerId" | "$id" => Ok(Some(KeySize(KeyInfo::MaxKeySize {
                        unique_id: document_type
                            .unique_id_for_document_field(index_level, base_event)
                            .to_vec(),
                        max_size: DEFAULT_HASH_SIZE_U8,
                    }))),
                    "$createdAt" | "$updatedAt" | "$transferredAt" => {
                        Ok(Some(KeySize(KeyInfo::MaxKeySize {
                            unique_id: document_type
                                .unique_id_for_document_field(index_level, base_event)
                                .to_vec(),
                            max_size: U64_SIZE_U8,
                        })))
                    }
                    "$createdAtBlockHeight"
                    | "$updatedAtBlockHeight"
                    | "$transferredAtBlockHeight" => Ok(Some(KeySize(KeyInfo::MaxKeySize {
                        unique_id: document_type
                            .unique_id_for_document_field(index_level, base_event)
                            .to_vec(),
                        max_size: U64_SIZE_U8,
                    }))),
                    "$createdAtCoreBlockHeight"
                    | "$updatedAtCoreBlockHeight"
                    | "$transferredAtCoreBlockHeight" => Ok(Some(KeySize(KeyInfo::MaxKeySize {
                        unique_id: document_type
                            .unique_id_for_document_field(index_level, base_event)
                            .to_vec(),
                        max_size: U32_SIZE_U8,
                    }))),
                    _ => {
                        let property =
                            document_type.flattened_properties().get(key_path).ok_or({
                                Error::Fee(FeeError::DocumentTypeFieldNotFoundForEstimation(
                                    "incorrect key path for document type",
                                ))
                            })?;

                        let estimated_middle_size =
                            property.property_type.middle_byte_size_ceil().ok_or({
                                Error::Drive(DriveError::CorruptedCodeExecution(
                                    "document type must have a max size",
                                ))
                            })?;
                        if estimated_middle_size > u8::MAX as u16 {
                            // this is too big for a key
                            return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                                "estimated middle size is too big for a key",
                            )));
                        }
                        Ok(Some(KeySize(KeyInfo::MaxKeySize {
                            unique_id: document_type
                                .unique_id_for_document_field(index_level, base_event)
                                .to_vec(),
                            max_size: estimated_middle_size as u8,
                        })))
                    }
                }
            }
        }
    }

    /// Gets the borrowed document
    fn get_borrowed_document_and_storage_flags(
        &self,
    ) -> Option<(&Document, Option<&StorageFlags>)> {
        match self {
            DocumentInfo::DocumentRefAndSerialization((document, _, storage_flags))
            | DocumentInfo::DocumentRefInfo((document, storage_flags)) => {
                Some((document, storage_flags.as_ref().map(|flags| flags.as_ref())))
            }
            DocumentInfo::DocumentOwnedInfo((document, storage_flags))
            | DocumentInfo::DocumentAndSerialization((document, _, storage_flags)) => {
                Some((document, storage_flags.as_ref().map(|flags| flags.as_ref())))
            }
            DocumentInfo::DocumentEstimatedAverageSize(_) => None,
        }
    }

    /// Gets storage flags
    fn get_storage_flags_ref(&self) -> Option<&StorageFlags> {
        match self {
            DocumentInfo::DocumentRefAndSerialization((_, _, storage_flags))
            | DocumentInfo::DocumentRefInfo((_, storage_flags))
            | DocumentInfo::DocumentOwnedInfo((_, storage_flags))
            | DocumentInfo::DocumentAndSerialization((_, _, storage_flags)) => {
                storage_flags.as_ref().map(|flags| flags.as_ref())
            }
            DocumentInfo::DocumentEstimatedAverageSize(_) => {
                StorageFlags::optional_default_as_ref()
            }
        }
    }

    /// Gets storage flags
    fn get_document_id_as_slice(&self) -> Option<&[u8]> {
        match self {
            DocumentInfo::DocumentRefAndSerialization((document, _, _))
            | DocumentInfo::DocumentRefInfo((document, _)) => Some(document.id_ref().as_slice()),
            DocumentInfo::DocumentOwnedInfo((document, _))
            | DocumentInfo::DocumentAndSerialization((document, _, _)) => {
                Some(document.id_ref().as_slice())
            }
            DocumentInfo::DocumentEstimatedAverageSize(_) => None,
        }
    }
}
