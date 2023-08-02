use itertools::Itertools;
use std::collections::{BTreeMap, BTreeSet};
use std::convert::TryInto;

use crate::data_contract::document_type::{property_names, DocumentTypeRef};
use crate::data_contract::errors::{DataContractError, StructureError};

use crate::data_contract::document_type::document_field::{DocumentField, DocumentFieldType};
use crate::data_contract::document_type::index::{Index, IndexProperty};
use crate::data_contract::document_type::index_level::IndexLevel;
use crate::data_contract::document_type::v0::{DocumentTypeV0, DEFAULT_HASH_SIZE, MAX_INDEX_SIZE};
use crate::document::INITIAL_REVISION;
use crate::document::{Document, DocumentV0};
use crate::prelude::Revision;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::btreemap_extensions::{BTreeValueMapHelper, BTreeValueRemoveFromMapHelper};
use platform_value::{Identifier, ReplacementType, Value};
use serde::{Deserialize, Serialize};

// TODO: Verify we need all those methods
pub trait DocumentTypeV0Methods {
    fn index_for_types(
        &self,
        index_names: &[&str],
        in_field_name: Option<&str>,
        order_by: &[&str],
        platform_version: &PlatformVersion,
    ) -> Result<Option<(&Index, u16)>, ProtocolError>;

    fn serialize_value_for_key(
        &self,
        key: &str,
        value: &Value,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, ProtocolError>;

    fn convert_value_to_document(
        &self,
        data: Value,
        platform_version: &PlatformVersion,
    ) -> Result<Document, ProtocolError>;

    fn max_size(&self, platform_version: &PlatformVersion) -> Result<u16, ProtocolError>;

    fn estimated_size(&self, platform_version: &PlatformVersion) -> Result<u16, ProtocolError>;

    fn document_field_type_for_property(
        &self,
        property: &str,
        platform_version: &PlatformVersion,
    ) -> Result<Option<DocumentFieldType>, ProtocolError>;

    /// Non versioned
    fn unique_id_for_storage(&self) -> [u8; 32];

    /// Non versioned
    fn unique_id_for_document_field(
        &self,
        index_level: &IndexLevel,
        base_event: [u8; 32],
    ) -> Vec<u8>;

    /// Non versioned
    fn field_can_be_null(&self, name: &str) -> bool;

    /// Non versioned
    fn initial_revision(&self) -> Option<Revision>;

    /// Non versioned
    fn requires_revision(&self) -> bool;

    /// Non versioned
    fn top_level_indices(&self) -> Vec<&IndexProperty>;

    /// Non versioned
    fn document_field_for_property(&self, property: &str) -> Option<DocumentField>;
    fn create_document_from_data(
        &self,
        data: Value,
        owner_id: Identifier,
        document_entropy: [u8; 32],
        platform_version: &PlatformVersion,
    ) -> Result<Document, ProtocolError>;

    /// Creates a document at the current time based on document type information
    /// Properties set here must be pre validated
    fn create_document_with_prevalidated_properties(
        &self,
        id: Identifier,
        owner_id: Identifier,
        properties: BTreeMap<String, Value>,
        platform_version: &PlatformVersion,
    ) -> Result<Document, ProtocolError>;
}

impl DocumentTypeV0Methods for DocumentTypeV0 {
    fn index_for_types(
        &self,
        index_names: &[&str],
        in_field_name: Option<&str>,
        order_by: &[&str],
        platform_version: &PlatformVersion,
    ) -> Result<Option<(&Index, u16)>, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .index_for_types
        {
            0 => Ok(self.index_for_types_v0(index_names, in_field_name, order_by)),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "store_ephemeral_state".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    fn serialize_value_for_key(
        &self,
        key: &str,
        value: &Value,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .serialize_value_for_key
        {
            0 => self.serialize_value_for_key_v0(key, value),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "serialize_value_for_key".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    fn convert_value_to_document(
        &self,
        data: Value,
        platform_version: &PlatformVersion,
    ) -> Result<Document, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .convert_value_to_document
        {
            0 => self.convert_value_to_document_v0(data, platform_version),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "convert_value_to_document".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    fn max_size(&self, platform_version: &PlatformVersion) -> Result<u16, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .max_size
        {
            0 => Ok(self.max_size_v0()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "max_size".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    fn estimated_size(&self, platform_version: &PlatformVersion) -> Result<u16, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .estimated_size
        {
            0 => Ok(self.estimated_size_v0()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "estimated_size".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    fn document_field_type_for_property(
        &self,
        property: &str,
        platform_version: &PlatformVersion,
    ) -> Result<Option<DocumentFieldType>, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .document_field_type_for_property
        {
            0 => Ok(self.document_field_type_for_property_v0(property)),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "document_field_type_for_property".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    fn unique_id_for_storage(&self) -> [u8; 32] {
        rand::random::<[u8; 32]>()
    }

    fn unique_id_for_document_field(
        &self,
        index_level: &IndexLevel,
        base_event: [u8; 32],
    ) -> Vec<u8> {
        let mut bytes = index_level.level_identifier.to_be_bytes().to_vec();
        bytes.extend_from_slice(&base_event);
        bytes
    }

    fn field_can_be_null(&self, name: &str) -> bool {
        !self.required_fields.contains(name)
    }

    fn initial_revision(&self) -> Option<Revision> {
        if self.documents_mutable {
            Some(INITIAL_REVISION)
        } else {
            None
        }
    }

    fn requires_revision(&self) -> bool {
        self.documents_mutable
    }

    fn top_level_indices(&self) -> Vec<&IndexProperty> {
        let mut index_properties: Vec<&IndexProperty> = Vec::with_capacity(self.indices.len());
        for index in &self.indices {
            if let Some(property) = index.properties.get(0) {
                index_properties.push(property);
            }
        }
        index_properties
    }

    fn document_field_for_property(&self, property: &str) -> Option<DocumentField> {
        self.flattened_properties.get(property).cloned()
    }

    fn create_document_from_data(
        &self,
        data: Value,
        owner_id: Identifier,
        document_entropy: [u8; 32],
        platform_version: &PlatformVersion,
    ) -> Result<Document, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .create_document_from_data
        {
            0 => self.create_document_from_data_v0(
                data,
                owner_id,
                document_entropy,
                platform_version,
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "create_document_from_data".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    fn create_document_with_prevalidated_properties(
        &self,
        id: Identifier,
        owner_id: Identifier,
        properties: BTreeMap<String, Value>,
        platform_version: &PlatformVersion,
    ) -> Result<Document, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .create_document_with_prevalidated_properties
        {
            0 => self.create_document_with_prevalidated_properties_v0(
                id,
                owner_id,
                properties,
                platform_version,
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "create_document_with_prevalidated_properties".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
