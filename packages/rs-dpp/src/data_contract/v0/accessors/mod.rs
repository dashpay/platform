use crate::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
use crate::data_contract::document_type::DocumentType;
use crate::data_contract::v0::DataContractV0;
use crate::data_contract::{DefinitionName, DocumentName, JsonSchema, PropertyPath};
use crate::metadata::Metadata;
use crate::ProtocolError;
use platform_value::Identifier;
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;
use crate::data_contract::data_contract_config::DataContractConfig;

impl DataContractV0Getters for DataContractV0 {
    fn id(&self) -> Identifier {
        self.id
    }

    fn schema(&self) -> &String {
        &self.schema
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

    fn document_types_mut(&mut self) -> &mut BTreeMap<DocumentName, DocumentType> {
        &mut self.document_types
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

    fn documents(&self) -> Result<&BTreeMap<DocumentName, JsonSchema>, ProtocolError> {
        Ok(&self.documents)
    }

    fn documents_mut(&mut self) -> Result<&mut BTreeMap<DocumentName, JsonSchema>, ProtocolError> {
        Ok(&mut self.documents)
    }

    fn defs(&self) -> Result<Option<&BTreeMap<DefinitionName, JsonSchema>>, ProtocolError> {
        Ok(self.defs.as_ref())
    }

    fn defs_mut(
        &mut self,
    ) -> Result<Option<&mut BTreeMap<DefinitionName, JsonSchema>>, ProtocolError> {
        Ok(self.defs.as_mut())
    }

    fn binary_properties(
        &self,
    ) -> Result<&BTreeMap<DocumentName, BTreeMap<PropertyPath, JsonValue>>, ProtocolError> {
        Ok(&self.binary_properties)
    }

    fn binary_properties_mut(
        &mut self,
    ) -> Result<&mut BTreeMap<DocumentName, BTreeMap<PropertyPath, JsonValue>>, ProtocolError> {
        Ok(&mut self.binary_properties)
    }
}

impl DataContractV0Setters for DataContractV0 {
    fn set_id(&mut self, id: Identifier) {
        self.id = id;
    }

    fn set_schema(&mut self, schema: String) {
        self.schema = schema;
    }

    fn set_version(&mut self, version: u32) {
        self.version = version;
    }

    fn set_owner_id(&mut self, owner_id: Identifier) {
        self.owner_id = owner_id;
    }

    fn set_document_types(&mut self, document_types: BTreeMap<DocumentName, DocumentType>) {
        self.document_types = document_types;
    }

    fn set_metadata(&mut self, metadata: Option<Metadata>) {
        self.metadata = metadata;
    }

    fn set_config(&mut self, config: DataContractConfig) {
        self.config = config;
    }

    fn set_documents(&mut self, documents: BTreeMap<DocumentName, JsonSchema>) {
        self.documents = documents;
    }

    fn set_defs(&mut self, defs: Option<BTreeMap<DefinitionName, JsonSchema>>) {
        self.defs = defs;
    }

    fn set_binary_properties(
        &mut self,
        binary_properties: BTreeMap<DocumentName, BTreeMap<PropertyPath, JsonValue>>,
    ) {
        self.binary_properties = binary_properties;
    }
}
