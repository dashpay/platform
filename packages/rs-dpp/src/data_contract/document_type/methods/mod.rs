mod create_document_from_data;
mod create_document_with_prevalidated_properties;
mod estimated_size;
mod index_for_types;
mod max_size;
mod serialize_value_for_key;
mod validate_update;

use std::collections::BTreeMap;

use crate::data_contract::document_type::index::{Index, IndexProperty};
use crate::data_contract::document_type::index_level::IndexLevel;

use crate::data_contract::document_type::v0::DocumentTypeV0;
use crate::document::Document;
use crate::document::INITIAL_REVISION;
use crate::prelude::Revision;
use crate::version::PlatformVersion;
use crate::ProtocolError;

use platform_value::{Identifier, Value};

// TODO: Some of those methods are only for tests. Hide under feature
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

    fn max_size(&self, platform_version: &PlatformVersion) -> Result<u16, ProtocolError>;

    fn estimated_size(&self, platform_version: &PlatformVersion) -> Result<u16, ProtocolError>;

    /// Non versioned
    fn unique_id_for_storage(&self) -> [u8; 32];

    /// Non versioned
    fn unique_id_for_document_field(
        &self,
        index_level: &IndexLevel,
        base_event: [u8; 32],
    ) -> Vec<u8>;

    /// Non versioned
    fn initial_revision(&self) -> Option<Revision>;

    /// Non versioned
    fn requires_revision(&self) -> bool;

    /// Non versioned
    fn top_level_indices(&self) -> Vec<&IndexProperty>;

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
            .methods
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
            .methods
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

    fn max_size(&self, platform_version: &PlatformVersion) -> Result<u16, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .methods
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
            .methods
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

    // TODO: Super wired. Remove from here.
    fn unique_id_for_storage(&self) -> [u8; 32] {
        rand::random::<[u8; 32]>()
    }

    fn unique_id_for_document_field(
        &self,
        index_level: &IndexLevel,
        base_event: [u8; 32],
    ) -> Vec<u8> {
        let mut bytes = index_level.identifier().to_be_bytes().to_vec();
        bytes.extend_from_slice(&base_event);
        bytes
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
            if let Some(property) = index.properties.first() {
                index_properties.push(property);
            }
        }
        index_properties
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
            .methods
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
            .methods
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
