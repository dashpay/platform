use crate::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
use crate::data_contract::data_contract_config::DataContractConfig;
use crate::data_contract::document_type::DocumentType;
use crate::data_contract::{DocumentName, PropertyPath};
use crate::metadata::Metadata;
use crate::prelude::DataContract;
use crate::ProtocolError;
use platform_value::Identifier;
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;

pub mod v0;

impl DataContractV0Getters for DataContract {
    fn id(&self) -> Identifier {
        match self {
            DataContract::V0(v0) => v0.id(),
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

    fn document_type(&self, name: &DocumentName) -> Option<&DocumentType> {
        match self {
            DataContract::V0(v0) => v0.document_type(name),
        }
    }

    fn document_types(&self) -> &BTreeMap<DocumentName, DocumentType> {
        match self {
            DataContract::V0(v0) => v0.document_types(),
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

    fn config_mut(&mut self) -> &mut DataContractConfig {
        match self {
            DataContract::V0(v0) => v0.config_mut(),
        }
    }
}

impl DataContractV0Setters for DataContract {
    fn set_id(&mut self, id: Identifier) {
        match self {
            DataContract::V0(v0) => v0.set_id(id),
        }
    }

    fn set_version(&mut self, version: u32) {
        match self {
            DataContract::V0(v0) => v0.set_version(version),
        }
    }

    fn set_owner_id(&mut self, owner_id: Identifier) {
        match self {
            DataContract::V0(v0) => v0.set_owner_id(owner_id),
        }
    }

    fn set_metadata(&mut self, metadata: Option<Metadata>) {
        match self {
            DataContract::V0(v0) => v0.set_metadata(metadata),
        }
    }

    fn set_config(&mut self, config: DataContractConfig) {
        match self {
            DataContract::V0(v0) => v0.set_config(config),
        }
    }
}
