use crate::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
use crate::data_contract::config::DataContractConfig;
use crate::data_contract::document_type::{DocumentType, DocumentTypeRef};
use crate::data_contract::DocumentName;
use crate::metadata::Metadata;
use crate::prelude::DataContract;
use crate::ProtocolError;
use platform_value::Identifier;

use crate::identity::SecurityLevel;
use std::collections::{BTreeMap, BTreeSet};

pub mod v0;

impl DataContractV0Getters for DataContract {
    fn id(&self) -> Identifier {
        match self {
            DataContract::V0(v0) => v0.id(),
        }
    }

    fn id_ref(&self) -> &Identifier {
        match self {
            DataContract::V0(v0) => v0.id_ref(),
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

    fn document_type_for_name(&self, name: &str) -> Result<DocumentTypeRef, ProtocolError> {
        match self {
            DataContract::V0(v0) => v0.document_type_for_name(name),
        }
    }

    fn document_type_optional_for_name(&self, name: &str) -> Option<DocumentTypeRef> {
        match self {
            DataContract::V0(v0) => v0.document_type_optional_for_name(name),
        }
    }

    fn has_document_type_for_name(&self, name: &str) -> bool {
        match self {
            DataContract::V0(v0) => v0.has_document_type_for_name(name),
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

    fn are_documents_keep_history_for_type(
        &self,
        document_type_name: &str,
    ) -> Result<bool, ProtocolError> {
        match self {
            DataContract::V0(v0) => v0.are_documents_keep_history_for_type(document_type_name),
        }
    }

    fn are_documents_mutable_for_type(
        &self,
        document_type_name: &str,
    ) -> Result<bool, ProtocolError> {
        match self {
            DataContract::V0(v0) => v0.are_documents_mutable_for_type(document_type_name),
        }
    }

    fn security_level_requirement_for_type(
        &self,
        document_type_name: &str,
    ) -> Result<SecurityLevel, ProtocolError> {
        match self {
            DataContract::V0(v0) => v0.security_level_requirement_for_type(document_type_name),
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

    fn increment_version(&mut self) {
        match self {
            DataContract::V0(v0) => v0.increment_version(),
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

    fn set_documents_keep_history_for_type(
        &mut self,
        document_type_name: &str,
        keep_history: bool,
    ) -> Result<(), ProtocolError> {
        match self {
            DataContract::V0(v0) => {
                v0.set_documents_keep_history_for_type(document_type_name, keep_history)
            }
        }
    }

    fn set_documents_mutable_for_type(
        &mut self,
        document_type_name: &str,
        mutable: bool,
    ) -> Result<(), ProtocolError> {
        match self {
            DataContract::V0(v0) => v0.set_documents_mutable_for_type(document_type_name, mutable),
        }
    }

    fn set_security_level_requirement_for_type(
        &mut self,
        document_type_name: &str,
        requirement: SecurityLevel,
    ) -> Result<(), ProtocolError> {
        match self {
            DataContract::V0(v0) => {
                v0.set_security_level_requirement_for_type(document_type_name, requirement)
            }
        }
    }
}
