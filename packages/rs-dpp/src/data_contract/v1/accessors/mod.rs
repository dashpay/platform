use crate::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
use crate::data_contract::config::DataContractConfig;
use crate::data_contract::document_type::{DocumentType, DocumentTypeRef};
use crate::data_contract::errors::DataContractError;

use crate::data_contract::v1::DataContractV1;
use crate::data_contract::{DocumentName, GroupContractPosition, TokenContractPosition};
use crate::metadata::Metadata;

use crate::data_contract::accessors::v1::{DataContractV1Getters, DataContractV1Setters};
use crate::data_contract::associated_token::token_configuration::TokenConfiguration;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::group::Group;
use crate::tokens::calculate_token_id;
use crate::tokens::errors::TokenError;
use crate::ProtocolError;
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
    fn group(&self, position: GroupContractPosition) -> Result<&Group, ProtocolError> {
        self.groups
            .get(&position)
            .ok_or(ProtocolError::GroupNotFound(format!(
                "Group not found in contract {} at position {}",
                self.id(),
                position
            )))
    }

    fn groups(&self) -> &BTreeMap<GroupContractPosition, Group> {
        &self.groups
    }

    fn groups_mut(&mut self) -> Option<&mut BTreeMap<GroupContractPosition, Group>> {
        Some(&mut self.groups)
    }

    fn expected_group(&self, position: GroupContractPosition) -> Result<&Group, ProtocolError> {
        self.groups
            .get(&position)
            .ok_or(ProtocolError::GroupNotFound(format!(
                "Group not found at position {} in contract {}",
                position,
                self.id()
            )))
    }

    fn tokens(&self) -> &BTreeMap<TokenContractPosition, TokenConfiguration> {
        &self.tokens
    }

    fn tokens_mut(&mut self) -> Option<&mut BTreeMap<TokenContractPosition, TokenConfiguration>> {
        Some(&mut self.tokens)
    }

    fn expected_token_configuration(
        &self,
        position: TokenContractPosition,
    ) -> Result<&TokenConfiguration, ProtocolError> {
        self.tokens.get(&position).ok_or(ProtocolError::Token(
            TokenError::TokenNotFoundAtPositionError.into(),
        ))
    }

    fn token_configuration_mut(
        &mut self,
        position: TokenContractPosition,
    ) -> Option<&mut TokenConfiguration> {
        self.tokens.get_mut(&position)
    }

    /// Returns the token id if a token exists at that position
    fn token_id(&self, position: TokenContractPosition) -> Option<Identifier> {
        self.tokens
            .get(&position)
            .map(|_| calculate_token_id(self.id.as_bytes(), position).into())
    }
}

impl DataContractV1Setters for DataContractV1 {
    fn set_groups(&mut self, groups: BTreeMap<GroupContractPosition, Group>) {
        self.groups = groups;
    }

    fn set_tokens(&mut self, tokens: BTreeMap<TokenContractPosition, TokenConfiguration>) {
        self.tokens = tokens;
    }

    fn add_group(&mut self, group_position: GroupContractPosition, group: Group) {
        self.groups.insert(group_position, group);
    }

    fn add_token(&mut self, name: TokenContractPosition, token: TokenConfiguration) {
        self.tokens.insert(name, token);
    }
}
