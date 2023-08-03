pub mod accessors;
pub mod array_field;
mod document_field;
pub use document_field::*;
pub mod document_type_class_methods;
pub mod document_type_methods;
mod index;
pub use index::*;
mod index_level;
pub use index_level::IndexLevel;
pub mod enrich_with_base_schema;
#[cfg(feature = "random-documents")]
pub mod random_document;
pub mod v0;
mod validate_data_contract_max_depth;

use crate::data_contract::document_type::v0::v0_methods::DocumentTypeV0Methods;
use crate::data_contract::document_type::v0::DocumentTypeV0;
use crate::document::Document;
use crate::prelude::Revision;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use derive_more::From;
use platform_value::{Identifier, Value};
use std::collections::{BTreeMap, BTreeSet};

pub(self) mod property_names {
    pub const DOCUMENTS_KEEP_HISTORY: &str = "documentsKeepHistory";
    pub const DOCUMENTS_MUTABLE: &str = "documentsMutable";
    pub const INDICES: &str = "indices";
    pub const PROPERTIES: &str = "properties";
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

    fn string_to_field_type(field_type_name: &str) -> Option<DocumentFieldType> {
        match field_type_name {
            "integer" => Some(DocumentFieldType::Integer),
            "number" => Some(DocumentFieldType::Number),
            "boolean" => Some(DocumentFieldType::Boolean),
            "date" => Some(DocumentFieldType::Date),
            _ => None,
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

    fn convert_value_to_document(
        &self,
        data: Value,
        platform_version: &PlatformVersion,
    ) -> Result<Document, ProtocolError> {
        match self {
            DocumentTypeRef::V0(v0) => v0.convert_value_to_document(data, platform_version),
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

    fn document_field_type_for_property(
        &self,
        property: &str,
        platform_version: &PlatformVersion,
    ) -> Result<Option<DocumentFieldType>, ProtocolError> {
        match self {
            DocumentTypeRef::V0(v0) => {
                v0.document_field_type_for_property(property, platform_version)
            }
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

    fn field_can_be_null(&self, name: &str) -> bool {
        match self {
            DocumentTypeRef::V0(v0) => v0.field_can_be_null(name),
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

    fn document_field_for_property(&self, property: &str) -> Option<DocumentField> {
        match self {
            DocumentTypeRef::V0(v0) => v0.document_field_for_property(property),
        }
    }

    fn create_document_from_data(
        &self,
        data: Value,
        owner_id: Identifier,
        document_entropy: [u8; 32],
        platform_version: &PlatformVersion,
    ) -> Result<Document, ProtocolError> {
        match self {
            DocumentTypeRef::V0(v0) => {
                v0.create_document_from_data(data, owner_id, document_entropy, platform_version)
            }
        }
    }

    fn create_document_with_prevalidated_properties(
        &self,
        id: Identifier,
        owner_id: Identifier,
        properties: BTreeMap<String, Value>,
        platform_version: &PlatformVersion,
    ) -> Result<Document, ProtocolError> {
        match self {
            DocumentTypeRef::V0(v0) => v0.create_document_with_prevalidated_properties(
                id,
                owner_id,
                properties,
                platform_version,
            ),
        }
    }
}
