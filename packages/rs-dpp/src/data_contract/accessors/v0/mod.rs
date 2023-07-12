use std::collections::BTreeMap;
use platform_value::Identifier;
use crate::data_contract::contract_config::ContractConfigV0;
use crate::data_contract::document_type::DocumentType;
use crate::data_contract::{DefinitionName, DocumentName, JsonSchema, PropertyPath};
use crate::metadata::Metadata;
use crate::ProtocolError;
use serde_json::Value as JsonValue;

pub trait DataContractV0Getters {
    /// Returns the unique identifier for the data contract.
    fn id(&self) -> Identifier;

    /// Returns a reference to the JSON schema that defines the contract.
    fn schema(&self) -> &String;

    /// Returns the version of this data contract.
    fn version(&self) -> u32;

    /// Returns the identifier of the contract owner.
    fn owner_id(&self) -> Identifier;

    /// Returns a mapping of document names to their corresponding document types.
    fn document_types(&self) -> &BTreeMap<DocumentName, DocumentType>;

    /// Returns a mutable reference to the mapping of document names to their corresponding document types.
    fn document_types_mut(&mut self) -> &mut BTreeMap<DocumentName, DocumentType>;

    /// Returns optional metadata associated with the contract.
    fn metadata(&self) -> Option<&Metadata>;

    /// Returns a mutable reference to the optional metadata associated with the contract.
    fn metadata_mut(&mut self) -> Option<&mut Metadata>;

    /// Returns the internal configuration for the contract.
    fn config(&self) -> &ContractConfigV0;

    /// Returns a mapping of document names to their corresponding JSON schemas.
    fn documents(&self) -> Result<&BTreeMap<DocumentName, JsonSchema>, ProtocolError>;

    /// Returns a mutable reference to the mapping of document names to their corresponding JSON schemas.
    fn documents_mut(&mut self) -> Result<&mut BTreeMap<DocumentName, JsonSchema>, ProtocolError>;

    /// Returns optional mapping of definition names to their corresponding JSON schemas.
    fn defs(&self) -> Result<Option<&BTreeMap<DefinitionName, JsonSchema>>, ProtocolError>;

    /// Returns a mutable reference to the optional mapping of definition names to their corresponding JSON schemas.
    fn defs_mut(&mut self) -> Result<Option<&mut BTreeMap<DefinitionName, JsonSchema>>, ProtocolError>;

    /// Returns a nested mapping of document names and property paths to their binary values.
    fn binary_properties(&self) -> Result<&BTreeMap<DocumentName, BTreeMap<PropertyPath, JsonValue>>, ProtocolError>;

    /// Returns a mutable reference to the nested mapping of document names and property paths to their binary values.
    fn binary_properties_mut(&mut self) -> Result<&mut BTreeMap<DocumentName, BTreeMap<PropertyPath, JsonValue>>, ProtocolError>;
}

pub trait DataContractV0Setters {
    /// Sets the unique identifier for the data contract.
    fn set_id(&mut self, id: Identifier);

    /// Sets the reference to the JSON schema that defines the contract.
    fn set_schema(&mut self, schema: String);

    /// Sets the version of this data contract.
    fn set_version(&mut self, version: u32);

    /// Sets the identifier of the contract owner.
    fn set_owner_id(&mut self, owner_id: Identifier);

    /// Sets the mapping of document names to their corresponding document types.
    fn set_document_types(&mut self, document_types: BTreeMap<DocumentName, DocumentType>);

    /// Sets the optional metadata associated with the contract.
    fn set_metadata(&mut self, metadata: Option<Metadata>);

    /// Sets the internal configuration for the contract.
    fn set_config(&mut self, config: ContractConfigV0);

    /// Sets the mapping of document names to their corresponding JSON schemas.
    fn set_documents(&mut self, documents: BTreeMap<DocumentName, JsonSchema>);

    /// Sets the optional mapping of definition names to their corresponding JSON schemas.
    fn set_defs(&mut self, defs: Option<BTreeMap<DefinitionName, JsonSchema>>);

    /// Sets the nested mapping of document names and property paths to their binary values.
    fn set_binary_properties(&mut self, binary_properties: BTreeMap<DocumentName, BTreeMap<PropertyPath, JsonValue>>);
}
