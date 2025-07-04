use crate::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
use crate::data_contract::config::DataContractConfig;
use crate::data_contract::document_type::{DocumentType, DocumentTypeRef};
use crate::data_contract::errors::DataContractError;

use crate::data_contract::v0::DataContractV0;
use crate::data_contract::DocumentName;

use crate::data_contract::document_type::accessors::{
    DocumentTypeV0Getters, DocumentTypeV0Setters,
};
use platform_value::Identifier;
use std::collections::BTreeMap;

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

    fn document_type_cloned_for_name(&self, name: &str) -> Result<DocumentType, DataContractError> {
        self.document_type_cloned_optional_for_name(name)
            .ok_or_else(|| {
                DataContractError::DocumentTypeNotFound(
                    "can not get document type from contract".to_string(),
                )
            })
    }

    fn document_type_borrowed_for_name(
        &self,
        name: &str,
    ) -> Result<&DocumentType, DataContractError> {
        self.document_types.get(name).ok_or_else(|| {
            DataContractError::DocumentTypeNotFound(
                "can not get document type from contract".to_string(),
            )
        })
    }

    fn document_type_for_name(&self, name: &str) -> Result<DocumentTypeRef, DataContractError> {
        self.document_type_optional_for_name(name).ok_or_else(|| {
            DataContractError::DocumentTypeNotFound(
                "can not get document type from contract".to_string(),
            )
        })
    }

    fn document_type_optional_for_name(&self, name: &str) -> Option<DocumentTypeRef> {
        self.document_types
            .get(name)
            .map(|document_type| document_type.as_ref())
    }

    fn document_type_cloned_optional_for_name(&self, name: &str) -> Option<DocumentType> {
        self.document_types.get(name).cloned()
    }

    fn has_document_type_for_name(&self, name: &str) -> bool {
        self.document_types.contains_key(name)
    }

    fn document_types_with_contested_indexes(&self) -> BTreeMap<&DocumentName, &DocumentType> {
        self.document_types
            .iter()
            .filter(|(_, document_type)| {
                document_type
                    .indexes()
                    .iter()
                    .any(|(_, index)| index.contested_index.is_some())
            })
            .collect()
    }

    fn document_types(&self) -> &BTreeMap<DocumentName, DocumentType> {
        &self.document_types
    }

    fn document_types_mut(&mut self) -> &mut BTreeMap<DocumentName, DocumentType> {
        &mut self.document_types
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
            .for_each(|(_, document_type)| document_type.set_data_contract_id(id))
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

    fn set_config(&mut self, config: DataContractConfig) {
        self.config = config;
    }
}
