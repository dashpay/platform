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
use crate::prelude::{BlockHeight, CoreBlockHeight, Revision};
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
        block_height: BlockHeight,
        core_block_height: CoreBlockHeight,
        document_entropy: [u8; 32],
        platform_version: &PlatformVersion,
    ) -> Result<Document, ProtocolError>;

    /// Creates a document at the current time based on specified document type information.
    /// This function requires that all properties provided are pre-validated according to
    /// the document's schema requirements.
    ///
    /// # Parameters:
    /// - `id`: An identifier for the document. Unique within the context of the document's type.
    /// - `owner_id`: The identifier of the entity that will own this document.
    /// - `block_height`: The block height at which this document is considered to have been created.
    ///    While this value is recorded in the document, it is ignored when the document is broadcasted
    ///    to the network. This is because the actual block height at the time of broadcast may differ.
    ///    This parameter is included to fulfill schema requirements that specify a block height; you may
    ///    use the current block height, a placeholder value of 0, or any other value as necessary.
    /// - `core_block_height`: Similar to `block_height`, this represents the core network's block height
    ///    at the document's creation time. It is handled the same way as `block_height` regarding broadcast
    ///    and schema requirements.
    /// - `properties`: A collection of properties for the document, structured as a `BTreeMap<String, Value>`.
    ///    These must be pre-validated to match the document's schema definitions.
    /// - `platform_version`: A reference to the current version of the platform for which the document is created.
    ///
    /// # Returns:
    /// A `Result<Document, ProtocolError>`, which is `Ok` if the document was successfully created, or an error
    /// indicating what went wrong during the creation process.
    ///
    /// # Note:
    /// The `block_height` and `core_block_height` are primarily included for schema compliance and local record-keeping.
    /// These values are not used when the document is broadcasted to the network, as the network assigns its own block
    /// heights upon receipt and processing of the document. After broadcasting, it is recommended to update these fields
    /// in their created_at/updated_at variants as well as the base created_at/updated_at in the client-side
    /// representation of the document to reflect the values returned by the network. The base created_at/updated_at
    /// uses current time when creating the local document and is also ignored as it is also set network side.
    fn create_document_with_prevalidated_properties(
        &self,
        id: Identifier,
        owner_id: Identifier,
        block_height: BlockHeight,
        core_block_height: CoreBlockHeight,
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
        block_height: BlockHeight,
        core_block_height: CoreBlockHeight,
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
                block_height,
                core_block_height,
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
        block_height: BlockHeight,
        core_block_height: CoreBlockHeight,
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
                block_height,
                core_block_height,
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
