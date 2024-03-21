pub mod accessors;
mod property;
pub use property::*;
pub mod class_methods;
mod index;
pub mod methods;
pub use index::*;
mod index_level;
pub use index_level::IndexLevel;

#[cfg(feature = "random-documents")]
pub mod random_document;
pub mod schema;
pub mod v0;

use crate::data_contract::document_type::methods::DocumentTypeV0Methods;
use crate::data_contract::document_type::v0::DocumentTypeV0;
use crate::document::Document;
use crate::prelude::{BlockHeight, CoreBlockHeight, Revision};
use crate::version::PlatformVersion;
use crate::ProtocolError;
use derive_more::From;
use platform_value::{Identifier, Value};
use std::collections::BTreeMap;

mod property_names {
    pub const DOCUMENTS_KEEP_HISTORY: &str = "documentsKeepHistory";
    pub const DOCUMENTS_MUTABLE: &str = "documentsMutable";
    pub const SECURITY_LEVEL_REQUIREMENT: &str = "signatureSecurityLevelRequirement";
    pub const REQUIRES_IDENTITY_ENCRYPTION_BOUNDED_KEY: &str =
        "requiresIdentityEncryptionBoundedKey";
    pub const REQUIRES_IDENTITY_DECRYPTION_BOUNDED_KEY: &str =
        "requiresIdentityDecryptionBoundedKey";
    pub const INDICES: &str = "indices";
    pub const PROPERTIES: &str = "properties";
    pub const POSITION: &str = "position";
    pub const REQUIRED: &str = "required";
    pub const TYPE: &str = "type";
    pub const REF: &str = "$ref";
    pub const CREATED_AT: &str = "$createdAt";
    pub const UPDATED_AT: &str = "$updatedAt";
    pub const MIN_ITEMS: &str = "minItems";
    pub const MAX_ITEMS: &str = "maxItems";
    pub const MIN_LENGTH: &str = "minLength";
    pub const MAX_LENGTH: &str = "maxLength";
    pub const BYTE_ARRAY: &str = "byteArray";
    pub const CONTENT_MEDIA_TYPE: &str = "contentMediaType";
    pub const ENCRYPTION_KEY_REQUIREMENTS: &str = "encryptionKeyReqs";
    pub const DECRYPTION_KEY_REQUIREMENTS: &str = "decryptionKeyReqs";
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DocumentTypeRef<'a> {
    V0(&'a DocumentTypeV0),
}

#[derive(Debug)]
pub enum DocumentTypeMutRef<'a> {
    V0(&'a mut DocumentTypeV0),
}

#[derive(Debug, Clone, PartialEq, From)]
pub enum DocumentType {
    V0(DocumentTypeV0),
}

impl DocumentType {
    pub const fn as_ref(&self) -> DocumentTypeRef {
        match self {
            DocumentType::V0(v0) => DocumentTypeRef::V0(v0),
        }
    }

    pub fn as_mut_ref(&mut self) -> DocumentTypeMutRef {
        match self {
            DocumentType::V0(v0) => DocumentTypeMutRef::V0(v0),
        }
    }
}

impl<'a> DocumentTypeRef<'a> {
    pub fn to_owned_document_type(&self) -> DocumentType {
        match self {
            DocumentTypeRef::V0(v0) => DocumentType::V0((*v0).to_owned()),
        }
    }
}

impl<'a> DocumentTypeV0Methods for DocumentTypeRef<'a> {
    fn index_for_types(
        &self,
        index_names: &[&str],
        in_field_name: Option<&str>,
        order_by: &[&str],
        platform_version: &PlatformVersion,
    ) -> Result<Option<(&Index, u16)>, ProtocolError> {
        match self {
            DocumentTypeRef::V0(v0) => {
                v0.index_for_types(index_names, in_field_name, order_by, platform_version)
            }
        }
    }

    fn serialize_value_for_key(
        &self,
        key: &str,
        value: &Value,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, ProtocolError> {
        match self {
            DocumentTypeRef::V0(v0) => v0.serialize_value_for_key(key, value, platform_version),
        }
    }

    fn max_size(&self, platform_version: &PlatformVersion) -> Result<u16, ProtocolError> {
        match self {
            DocumentTypeRef::V0(v0) => v0.max_size(platform_version),
        }
    }

    fn estimated_size(&self, platform_version: &PlatformVersion) -> Result<u16, ProtocolError> {
        match self {
            DocumentTypeRef::V0(v0) => v0.estimated_size(platform_version),
        }
    }

    fn unique_id_for_storage(&self) -> [u8; 32] {
        match self {
            DocumentTypeRef::V0(v0) => v0.unique_id_for_storage(),
        }
    }

    fn unique_id_for_document_field(
        &self,
        index_level: &IndexLevel,
        base_event: [u8; 32],
    ) -> Vec<u8> {
        match self {
            DocumentTypeRef::V0(v0) => v0.unique_id_for_document_field(index_level, base_event),
        }
    }

    fn initial_revision(&self) -> Option<Revision> {
        match self {
            DocumentTypeRef::V0(v0) => v0.initial_revision(),
        }
    }

    fn requires_revision(&self) -> bool {
        match self {
            DocumentTypeRef::V0(v0) => v0.requires_revision(),
        }
    }

    fn top_level_indices(&self) -> Vec<&IndexProperty> {
        match self {
            DocumentTypeRef::V0(v0) => v0.top_level_indices(),
        }
    }

    fn create_document_from_data(
        &self,
        data: Value,
        owner_id: Identifier,
        block_height: BlockHeight,
        core_block_height: CoreBlockHeight,
        document_entropy: [u8; 32],
        platform_version: &PlatformVersion,
    ) -> Result<Document, ProtocolError> {
        match self {
            DocumentTypeRef::V0(v0) => v0.create_document_from_data(
                data,
                owner_id,
                block_height,
                core_block_height,
                document_entropy,
                platform_version,
            ),
        }
    }

    fn create_document_with_prevalidated_properties(
        &self,
        id: Identifier,
        owner_id: Identifier,
        block_height: BlockHeight,
        core_block_height: CoreBlockHeight,
        properties: BTreeMap<String, Value>,
        platform_version: &PlatformVersion,
    ) -> Result<Document, ProtocolError> {
        match self {
            DocumentTypeRef::V0(v0) => v0.create_document_with_prevalidated_properties(
                id,
                owner_id,
                block_height,
                core_block_height,
                properties,
                platform_version,
            ),
        }
    }
}
