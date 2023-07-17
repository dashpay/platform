use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::document_type::DocumentType;
use crate::data_contract::{DefinitionName, DocumentName, JsonSchema, PropertyPath};
use crate::metadata::Metadata;
use crate::prelude::DataContract;
use crate::ProtocolError;
use platform_value::Identifier;
use serde_json::Value;
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;
use crate::data_contract::data_contract_config::DataContractConfig;

pub mod v0;
impl DataContractV0Getters for DataContract {
    fn id(&self) -> Identifier {
        match self {
            DataContract::V0(v0) => v0.id(),
        }
    }

    fn schema(&self) -> &String {
        match self {
            DataContract::V0(v0) => v0.schema(),
        }
    }

    fn version(&self) -> u32 {
        match self {
            DataContract::V0(v0) => v0.version(),
        }
    }

    fn owner_id(&self) -> Identifier {
        match self {
            DataContract::V0(v0) => v0.owner_id(),
        }
    }

    fn document_types(&self) -> &BTreeMap<DocumentName, DocumentType> {
        match self {
            DataContract::V0(v0) => v0.document_types(),
        }
    }

    fn document_types_mut(&mut self) -> &mut BTreeMap<DocumentName, DocumentType> {
        match self {
            DataContract::V0(v0) => v0.document_types_mut(),
        }
    }

    fn metadata(&self) -> Option<&Metadata> {
        match self {
            DataContract::V0(v0) => v0.metadata(),
        }
    }

    fn metadata_mut(&mut self) -> Option<&mut Metadata> {
        match self {
            DataContract::V0(v0) => v0.metadata_mut(),
        }
    }

    fn config(&self) -> &DataContractConfig {
        match self {
            DataContract::V0(v0) => v0.config(),
        }
    }

    fn documents(&self) -> Result<&BTreeMap<DocumentName, JsonSchema>, ProtocolError> {
        match self {
            DataContract::V0(v0) => v0.documents(),
        }
    }

    fn documents_mut(&mut self) -> Result<&mut BTreeMap<DocumentName, JsonSchema>, ProtocolError> {
        match self {
            DataContract::V0(v0) => v0.documents_mut(),
        }
    }

    fn defs(&self) -> Result<Option<&BTreeMap<DefinitionName, JsonSchema>>, ProtocolError> {
        match self {
            DataContract::V0(v0) => v0.defs(),
        }
    }

    fn defs_mut(
        &mut self,
    ) -> Result<Option<&mut BTreeMap<DefinitionName, JsonSchema>>, ProtocolError> {
        match self {
            DataContract::V0(v0) => v0.defs_mut(),
        }
    }

    fn binary_properties(
        &self,
    ) -> Result<&BTreeMap<DocumentName, BTreeMap<PropertyPath, JsonValue>>, ProtocolError> {
        match self {
            DataContract::V0(v0) => v0.binary_properties(),
        }
    }

    fn binary_properties_mut(
        &mut self,
    ) -> Result<&mut BTreeMap<DocumentName, BTreeMap<PropertyPath, JsonValue>>, ProtocolError> {
        match self {
            DataContract::V0(v0) => v0.binary_properties_mut(),
        }
    }
}
