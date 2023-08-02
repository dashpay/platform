use crate::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
use crate::data_contract::data_contract_config::DataContractConfig;
use crate::data_contract::document_type::DocumentType;
use crate::data_contract::v0::DataContractV0;
use crate::data_contract::{DocumentName, PropertyPath};
use crate::metadata::Metadata;
use crate::ProtocolError;
use platform_value::Identifier;
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;

impl DataContractV0Getters for DataContractV0 {
    fn id(&self) -> Identifier {
        self.id
    }

    fn version(&self) -> u32 {
        self.version
    }

    fn owner_id(&self) -> Identifier {
        self.owner_id
    }

    fn document_types(&self) -> &BTreeMap<DocumentName, DocumentType> {
        &self.document_types
    }

    fn metadata(&self) -> Option<&Metadata> {
        self.metadata.as_ref()
    }

    fn metadata_mut(&mut self) -> Option<&mut Metadata> {
        self.metadata.as_mut()
    }

    fn config(&self) -> &DataContractConfig {
        &self.config
    }

    fn config_mut(&mut self) -> &mut DataContractConfig {
        &mut self.config
    }

    fn schema(&self) -> &DataContractSchema {
        &self.schema
    }

    fn binary_properties(
        &self,
    ) -> Result<&BTreeMap<DocumentName, BTreeMap<PropertyPath, JsonValue>>, ProtocolError> {
        Ok(&self.binary_properties)
    }
}

impl DataContractV0Setters for DataContractV0 {
    fn set_id(&mut self, id: Identifier) {
        self.id = id;
    }

    fn set_version(&mut self, version: u32) {
        self.version = version;
    }

    fn set_owner_id(&mut self, owner_id: Identifier) {
        self.owner_id = owner_id;
    }

    fn set_metadata(&mut self, metadata: Option<Metadata>) {
        self.metadata = metadata;
    }

    fn set_config(&mut self, config: DataContractConfig) {
        self.config = config;
    }

    fn set_schema(&mut self, schema: DataContractSchema) {
        self.schema = schema;
    }
}
