use crate::data_contract::document_type::DocumentTypeRef;
use crate::data_contract::DataContract;
use crate::document::{Document, ExtendedDocument};
use crate::identity::TimestampMillis;
use crate::metadata::Metadata;
use crate::prelude::{BlockHeight, CoreBlockHeight, Revision};
use crate::ProtocolError;
use platform_value::{Bytes32, Identifier, Value};
use std::collections::BTreeMap;

impl ExtendedDocument {
    /// Returns an immutable reference to the properties of the document.
    pub fn properties(&self) -> &BTreeMap<String, Value> {
        match self {
            ExtendedDocument::V0(v0) => v0.properties(),
        }
    }

    /// Returns a mutable reference to the properties of the document.
    pub fn properties_as_mut(&mut self) -> &mut BTreeMap<String, Value> {
        match self {
            ExtendedDocument::V0(v0) => v0.properties_as_mut(),
        }
    }

    /// Returns the document's identifier.
    pub fn id(&self) -> Identifier {
        match self {
            ExtendedDocument::V0(v0) => v0.id(),
        }
    }

    /// Returns the document's owner identifier.
    pub fn owner_id(&self) -> Identifier {
        match self {
            ExtendedDocument::V0(v0) => v0.owner_id(),
        }
    }

    /// Returns a reference to the document's type.
    ///
    /// # Errors
    ///
    /// Returns a `ProtocolError` if the document type is not found in the data contract.
    pub fn document_type(&self) -> Result<DocumentTypeRef, ProtocolError> {
        match self {
            ExtendedDocument::V0(v0) => v0.document_type(),
        }
    }

    /// Returns an optional reference to the document's revision.
    pub fn revision(&self) -> Option<Revision> {
        match self {
            ExtendedDocument::V0(v0) => v0.revision(),
        }
    }

    /// Returns an optional reference to the document's creation timestamp in milliseconds.
    /// It will be None if it is not required by the schema.
    pub fn created_at(&self) -> Option<TimestampMillis> {
        match self {
            ExtendedDocument::V0(v0) => v0.created_at(),
        }
    }

    /// Returns an optional reference to the document's last update timestamp in milliseconds.
    /// It will be None if it is not required by the schema.
    pub fn updated_at(&self) -> Option<TimestampMillis> {
        match self {
            ExtendedDocument::V0(v0) => v0.updated_at(),
        }
    }

    /// Returns an optional block height at which the document was created.
    /// It will be None if it is not required by the schema.
    pub fn created_at_block_height(&self) -> Option<BlockHeight> {
        match self {
            ExtendedDocument::V0(v0) => v0.created_at_block_height(),
        }
    }

    /// Returns an optional block height at which the document was last updated.
    /// It will be None if it is not required by the schema.
    pub fn updated_at_block_height(&self) -> Option<BlockHeight> {
        match self {
            ExtendedDocument::V0(v0) => v0.updated_at_block_height(),
        }
    }

    /// Returns an optional core block height at which the document was created.
    /// It will be None if it is not required by the schema.
    pub fn created_at_core_block_height(&self) -> Option<CoreBlockHeight> {
        match self {
            ExtendedDocument::V0(v0) => v0.created_at_core_block_height(),
        }
    }

    /// Returns an optional core block height at which the document was last updated.
    /// It will be None if it is not required by the schema.
    pub fn updated_at_core_block_height(&self) -> Option<CoreBlockHeight> {
        match self {
            ExtendedDocument::V0(v0) => v0.updated_at_core_block_height(),
        }
    }

    /// Returns the document type name as a reference to a string.
    pub fn document_type_name(&self) -> &String {
        match self {
            ExtendedDocument::V0(v0) => &v0.document_type_name,
        }
    }

    /// Sets the document type name.
    pub fn set_document_type_name(&mut self, document_type_name: String) {
        match self {
            ExtendedDocument::V0(v0) => v0.document_type_name = document_type_name,
        }
    }

    /// Returns the identifier of the associated data contract.
    pub fn data_contract_id(&self) -> Identifier {
        match self {
            ExtendedDocument::V0(v0) => v0.data_contract_id,
        }
    }

    /// Sets the data contract identifier.
    pub fn set_data_contract_id(&mut self, data_contract_id: Identifier) {
        match self {
            ExtendedDocument::V0(v0) => v0.data_contract_id = data_contract_id,
        }
    }

    /// Returns a reference to the actual document object containing the data.
    pub fn document(&self) -> &Document {
        match self {
            ExtendedDocument::V0(v0) => &v0.document,
        }
    }

    /// Returns a reference to the actual document object containing the data.
    pub fn document_mut(&mut self) -> &mut Document {
        match self {
            ExtendedDocument::V0(v0) => &mut v0.document,
        }
    }

    /// Sets the document object.
    pub fn set_document(&mut self, document: Document) {
        match self {
            ExtendedDocument::V0(v0) => v0.document = document,
        }
    }

    /// Returns a reference to the data contract associated with the document.
    pub fn data_contract(&self) -> &DataContract {
        match self {
            ExtendedDocument::V0(v0) => &v0.data_contract,
        }
    }

    /// Sets the data contract associated with the document.
    pub fn set_data_contract(&mut self, data_contract: DataContract) {
        match self {
            ExtendedDocument::V0(v0) => v0.data_contract = data_contract,
        }
    }

    /// Returns a reference to the optional metadata associated with the document.
    pub fn metadata(&self) -> &Option<Metadata> {
        match self {
            ExtendedDocument::V0(v0) => &v0.metadata,
        }
    }

    /// Sets the optional metadata associated with the document.
    pub fn set_metadata(&mut self, metadata: Option<Metadata>) {
        match self {
            ExtendedDocument::V0(v0) => v0.metadata = metadata,
        }
    }

    /// Returns a reference to the entropy stored as `Bytes32`.
    pub fn entropy(&self) -> &Bytes32 {
        match self {
            ExtendedDocument::V0(v0) => &v0.entropy,
        }
    }

    /// Sets the entropy stored as `Bytes32`.
    pub fn set_entropy(&mut self, entropy: Bytes32) {
        match self {
            ExtendedDocument::V0(v0) => v0.entropy = entropy,
        }
    }
}
