use crate::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
use crate::data_contract::config::DataContractConfig;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::{DocumentType, DocumentTypeRef};
use crate::data_contract::errors::DataContractError;
use crate::data_contract::storage_requirements::keys_for_document_type::StorageKeyRequirements;
use crate::data_contract::v0::DataContractV0;
use crate::data_contract::DocumentName;
use crate::metadata::Metadata;
use crate::ProtocolError;
use platform_value::Identifier;
use std::collections::{BTreeMap, BTreeSet};

impl DataContractV0Getters for DataContractV0 {
    fn id(&self) -> Identifier {
        self.id
    }

    fn id_ref(&self) -> &Identifier {
        &self.id
    }

    fn version(&self) -> u32 {
        self.version
    }

    fn owner_id(&self) -> Identifier {
        self.owner_id
    }

    fn document_type_cloned_for_name(&self, name: &str) -> Result<DocumentType, ProtocolError> {
        self.document_type_cloned_optional_for_name(name)
            .ok_or_else(|| {
                ProtocolError::DataContractError(DataContractError::DocumentTypeNotFound(
                    "can not get document type from contract",
                ))
            })
    }

    fn document_type_for_name(&self, name: &str) -> Result<DocumentTypeRef, ProtocolError> {
        self.document_type_optional_for_name(name).ok_or_else(|| {
            ProtocolError::DataContractError(DataContractError::DocumentTypeNotFound(
                "can not get document type from contract",
            ))
        })
    }

    fn document_type_optional_for_name(&self, name: &str) -> Option<DocumentTypeRef> {
        self.document_types
            .get(name)
            .map(|document_type| document_type.as_ref())
    }

    fn document_type_cloned_optional_for_name(&self, name: &str) -> Option<DocumentType> {
        self.document_types
            .get(name)
            .map(|document_type| document_type.clone())
    }

    fn has_document_type_for_name(&self, name: &str) -> bool {
        self.document_types.get(name).is_some()
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
}

impl DataContractV0Setters for DataContractV0 {
    fn set_id(&mut self, id: Identifier) {
        self.id = id;

        self.document_types
            .iter_mut()
            .for_each(|(_, document_type)| match document_type {
                DocumentType::V0(v0) => v0.data_contract_id = id,
            })
    }

    fn set_version(&mut self, version: u32) {
        self.version = version;
    }

    fn increment_version(&mut self) {
        self.version += 1;
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
}
