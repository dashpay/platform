use crate::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
use crate::data_contract::config::DataContractConfig;
use crate::data_contract::document_type::{DocumentType, DocumentTypeRef};
use crate::data_contract::errors::DataContractError;

use crate::data_contract::v1::DataContractV1;
use crate::data_contract::{DocumentName, TokenContractPosition};
use crate::metadata::Metadata;

use crate::data_contract::accessors::v1::{DataContractV1Getters, DataContractV1Setters};
use crate::data_contract::associated_token::token_configuration::TokenConfiguration;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::group::{Group, GroupName};
use crate::util::hash::hash_double;
use platform_value::Identifier;
use std::collections::BTreeMap;

impl DataContractV0Getters for DataContractV1 {
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
        self.document_types.get(name).is_some()
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

impl DataContractV0Setters for DataContractV1 {
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

impl DataContractV1Getters for DataContractV1 {
    fn groups(&self) -> &BTreeMap<GroupName, Group> {
        &self.groups
    }

    fn groups_mut(&mut self) -> Option<&mut BTreeMap<GroupName, Group>> {
        Some(&mut self.groups)
    }

    fn tokens(&self) -> &BTreeMap<TokenContractPosition, TokenConfiguration> {
        &self.tokens
    }

    fn tokens_mut(&mut self) -> Option<&mut BTreeMap<TokenContractPosition, TokenConfiguration>> {
        Some(&mut self.tokens)
    }

    /// Returns the token id if a token exists at that position
    fn token_id(&self, position: TokenContractPosition) -> Option<Identifier> {
        self.tokens.get(&position).map(|_| {
            let mut bytes = b"dash_token".to_vec();
            bytes.extend_from_slice(self.id().as_bytes());
            bytes.extend_from_slice(&position.to_be_bytes());
            hash_double(bytes).into()
        })
    }
}

impl DataContractV1Setters for DataContractV1 {
    fn set_groups(&mut self, groups: BTreeMap<GroupName, Group>) {
        self.groups = groups;
    }

    fn set_tokens(&mut self, tokens: BTreeMap<TokenContractPosition, TokenConfiguration>) {
        self.tokens = tokens;
    }

    fn add_group(&mut self, name: GroupName, group: Group) {
        self.groups.insert(name, group);
    }

    fn add_token(&mut self, name: TokenContractPosition, token: TokenConfiguration) {
        self.tokens.insert(name, token);
    }
}
