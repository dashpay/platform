use crate::data_contract::config::DataContractConfig;
use crate::data_contract::document_type::{DocumentType, DocumentTypeRef};
use crate::data_contract::DocumentName;
use crate::metadata::Metadata;
use crate::ProtocolError;
use platform_value::Identifier;
use std::collections::BTreeMap;

pub trait DataContractV0Getters {
    /// Returns the unique identifier for the data contract.
    fn id(&self) -> Identifier;

    fn id_ref(&self) -> &Identifier;

    /// Returns the version of this data contract.
    fn version(&self) -> u32;

    /// Returns the identifier of the contract owner.
    fn owner_id(&self) -> Identifier;
    fn document_type_cloned_for_name(&self, name: &str) -> Result<DocumentType, ProtocolError>;

    /// Returns the document type for the given document name.
    fn document_type_for_name(&self, name: &str) -> Result<DocumentTypeRef, ProtocolError>;

    fn document_type_optional_for_name(&self, name: &str) -> Option<DocumentTypeRef>;
    fn document_type_cloned_optional_for_name(&self, name: &str) -> Option<DocumentType>;

    fn has_document_type_for_name(&self, name: &str) -> bool;

    /// Returns a mapping of document names to their corresponding document types.
    fn document_types(&self) -> &BTreeMap<DocumentName, DocumentType>;

    /// Returns optional metadata associated with the contract.
    fn metadata(&self) -> Option<&Metadata>;

    /// Returns a mutable reference to the optional metadata associated with the contract.
    fn metadata_mut(&mut self) -> Option<&mut Metadata>;

    /// Returns the internal configuration for the contract.
    fn config(&self) -> &DataContractConfig;

    /// Returns the internal configuration for the contract as mutable.
    fn config_mut(&mut self) -> &mut DataContractConfig;
}

pub trait DataContractV0Setters {
    /// Sets the unique identifier for the data contract.
    fn set_id(&mut self, id: Identifier);

    /// Sets the version of this data contract.
    fn set_version(&mut self, version: u32);

    fn increment_version(&mut self);

    /// Sets the identifier of the contract owner.
    fn set_owner_id(&mut self, owner_id: Identifier);

    /// Sets the optional metadata associated with the contract.
    fn set_metadata(&mut self, metadata: Option<Metadata>);

    /// Sets the internal configuration for the contract.
    fn set_config(&mut self, config: DataContractConfig);
}
