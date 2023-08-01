use crate::data_contract::data_contract_config::DataContractConfig;
use crate::data_contract::document_type::DocumentType;
use crate::data_contract::schema::json_schema::DataContractSchema;
use crate::data_contract::{DocumentName, PropertyPath};
use crate::metadata::Metadata;
use crate::ProtocolError;
use platform_value::Identifier;
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;

pub trait DataContractV0Getters {
    /// Returns the unique identifier for the data contract.
    fn id(&self) -> Identifier;

    /// Returns the version of this data contract.
    fn version(&self) -> u32;

    /// Returns the identifier of the contract owner.
    fn owner_id(&self) -> Identifier;

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

    /// Returns data contract json schema
    fn schema(&self) -> &DataContractSchema;

    /// Returns a nested mapping of document names and property paths to their binary values.
    fn binary_properties(
        &self,
    ) -> Result<&BTreeMap<DocumentName, BTreeMap<PropertyPath, JsonValue>>, ProtocolError>;
}

pub trait DataContractV0Setters {
    /// Sets the unique identifier for the data contract.
    fn set_id(&mut self, id: Identifier);

    /// Sets the version of this data contract.
    fn set_version(&mut self, version: u32);

    /// Sets the identifier of the contract owner.
    fn set_owner_id(&mut self, owner_id: Identifier);

    /// Sets the optional metadata associated with the contract.
    fn set_metadata(&mut self, metadata: Option<Metadata>);

    /// Sets the internal configuration for the contract.
    fn set_config(&mut self, config: DataContractConfig);

    /// Sets data contract schema
    fn set_schema(&mut self, schema: DataContractSchema);
}
