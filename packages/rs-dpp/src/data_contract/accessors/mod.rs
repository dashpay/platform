use crate::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
use crate::data_contract::config::DataContractConfig;
use crate::data_contract::document_type::{DocumentType, DocumentTypeRef};
use crate::data_contract::{
    DocumentName, GroupContractPosition, TokenContractPosition, EMPTY_GROUPS, EMPTY_TOKENS,
};
use crate::metadata::Metadata;
use crate::prelude::DataContract;

use platform_value::Identifier;

use crate::data_contract::accessors::v1::{DataContractV1Getters, DataContractV1Setters};
use crate::data_contract::associated_token::token_configuration::TokenConfiguration;
use crate::data_contract::errors::DataContractError;
use crate::data_contract::group::Group;
use crate::ProtocolError;
use std::collections::BTreeMap;

pub mod v0;
pub mod v1;

impl DataContractV0Getters for DataContract {
    fn id(&self) -> Identifier {
        match self {
            DataContract::V0(v0) => v0.id(),
            DataContract::V1(v1) => v1.id(),
        }
    }

    fn id_ref(&self) -> &Identifier {
        match self {
            DataContract::V0(v0) => v0.id_ref(),
            DataContract::V1(v1) => v1.id_ref(),
        }
    }

    fn version(&self) -> u32 {
        match self {
            DataContract::V0(v0) => v0.version(),
            DataContract::V1(v1) => v1.version(),
        }
    }

    fn owner_id(&self) -> Identifier {
        match self {
            DataContract::V0(v0) => v0.owner_id(),
            DataContract::V1(v1) => v1.owner_id(),
        }
    }

    fn document_type_cloned_for_name(&self, name: &str) -> Result<DocumentType, DataContractError> {
        match self {
            DataContract::V0(v0) => v0.document_type_cloned_for_name(name),
            DataContract::V1(v1) => v1.document_type_cloned_for_name(name),
        }
    }

    fn document_type_borrowed_for_name(
        &self,
        name: &str,
    ) -> Result<&DocumentType, DataContractError> {
        match self {
            DataContract::V0(v0) => v0.document_type_borrowed_for_name(name),
            DataContract::V1(v1) => v1.document_type_borrowed_for_name(name),
        }
    }

    fn document_type_for_name(&self, name: &str) -> Result<DocumentTypeRef, DataContractError> {
        match self {
            DataContract::V0(v0) => v0.document_type_for_name(name),
            DataContract::V1(v1) => v1.document_type_for_name(name),
        }
    }

    fn document_type_optional_for_name(&self, name: &str) -> Option<DocumentTypeRef> {
        match self {
            DataContract::V0(v0) => v0.document_type_optional_for_name(name),
            DataContract::V1(v1) => v1.document_type_optional_for_name(name),
        }
    }

    fn document_type_cloned_optional_for_name(&self, name: &str) -> Option<DocumentType> {
        match self {
            DataContract::V0(v0) => v0.document_type_cloned_optional_for_name(name),
            DataContract::V1(v1) => v1.document_type_cloned_optional_for_name(name),
        }
    }

    fn has_document_type_for_name(&self, name: &str) -> bool {
        match self {
            DataContract::V0(v0) => v0.has_document_type_for_name(name),
            DataContract::V1(v1) => v1.has_document_type_for_name(name),
        }
    }

    fn document_types_with_contested_indexes(&self) -> BTreeMap<&DocumentName, &DocumentType> {
        match self {
            DataContract::V0(v0) => v0.document_types_with_contested_indexes(),
            DataContract::V1(v1) => v1.document_types_with_contested_indexes(),
        }
    }

    fn document_types(&self) -> &BTreeMap<DocumentName, DocumentType> {
        match self {
            DataContract::V0(v0) => v0.document_types(),
            DataContract::V1(v1) => v1.document_types(),
        }
    }

    fn document_types_mut(&mut self) -> &mut BTreeMap<DocumentName, DocumentType> {
        match self {
            DataContract::V0(v0) => v0.document_types_mut(),
            DataContract::V1(v1) => v1.document_types_mut(),
        }
    }

    fn metadata(&self) -> Option<&Metadata> {
        match self {
            DataContract::V0(v0) => v0.metadata(),
            DataContract::V1(v1) => v1.metadata(),
        }
    }

    fn metadata_mut(&mut self) -> Option<&mut Metadata> {
        match self {
            DataContract::V0(v0) => v0.metadata_mut(),
            DataContract::V1(v1) => v1.metadata_mut(),
        }
    }

    fn config(&self) -> &DataContractConfig {
        match self {
            DataContract::V0(v0) => v0.config(),
            DataContract::V1(v1) => v1.config(),
        }
    }

    fn config_mut(&mut self) -> &mut DataContractConfig {
        match self {
            DataContract::V0(v0) => v0.config_mut(),
            DataContract::V1(v1) => v1.config_mut(),
        }
    }
}

impl DataContractV0Setters for DataContract {
    fn set_id(&mut self, id: Identifier) {
        match self {
            DataContract::V0(v0) => v0.set_id(id),
            DataContract::V1(v1) => v1.set_id(id),
        }
    }

    fn set_version(&mut self, version: u32) {
        match self {
            DataContract::V0(v0) => v0.set_version(version),
            DataContract::V1(v1) => v1.set_version(version),
        }
    }

    fn increment_version(&mut self) {
        match self {
            DataContract::V0(v0) => v0.increment_version(),
            DataContract::V1(v1) => v1.increment_version(),
        }
    }

    fn set_owner_id(&mut self, owner_id: Identifier) {
        match self {
            DataContract::V0(v0) => v0.set_owner_id(owner_id),
            DataContract::V1(v1) => v1.set_owner_id(owner_id),
        }
    }

    fn set_metadata(&mut self, metadata: Option<Metadata>) {
        match self {
            DataContract::V0(v0) => v0.set_metadata(metadata),
            DataContract::V1(v1) => v1.set_metadata(metadata),
        }
    }

    fn set_config(&mut self, config: DataContractConfig) {
        match self {
            DataContract::V0(v0) => v0.set_config(config),
            DataContract::V1(v1) => v1.set_config(config),
        }
    }
}

/// Implementing DataContractV1Getters for DataContract
impl DataContractV1Getters for DataContract {
    /// Returns a reference to the groups map.
    fn group(&self, position: GroupContractPosition) -> Result<&Group, ProtocolError> {
        match self {
            DataContract::V0(_) => Err(ProtocolError::GroupNotFound(
                "There can not be a group in v0 data contracts".to_string(),
            )),
            DataContract::V1(v1) => v1.group(position),
        }
    }

    /// Returns a reference to the groups map.
    fn groups(&self) -> &BTreeMap<GroupContractPosition, Group> {
        match self {
            DataContract::V0(_) => &EMPTY_GROUPS,
            DataContract::V1(v1) => &v1.groups,
        }
    }

    /// Returns a mutable reference to the groups map.
    /// Returns `None` for V0 since it doesn't have groups.
    fn groups_mut(&mut self) -> Option<&mut BTreeMap<GroupContractPosition, Group>> {
        match self {
            DataContract::V0(_) => None,
            DataContract::V1(v1) => Some(&mut v1.groups),
        }
    }

    /// Returns a reference to the tokens map.
    fn tokens(&self) -> &BTreeMap<TokenContractPosition, TokenConfiguration> {
        match self {
            DataContract::V0(_) => &EMPTY_TOKENS,
            DataContract::V1(v1) => &v1.tokens,
        }
    }

    /// Returns a mutable reference to the tokens map.
    /// Returns `None` for V0 since it doesn't have tokens.
    fn tokens_mut(&mut self) -> Option<&mut BTreeMap<TokenContractPosition, TokenConfiguration>> {
        match self {
            DataContract::V0(_) => None,
            DataContract::V1(v1) => Some(&mut v1.tokens),
        }
    }

    fn token_id(&self, position: TokenContractPosition) -> Option<Identifier> {
        match self {
            DataContract::V0(_) => None,
            DataContract::V1(v1) => v1.token_id(position),
        }
    }
}

impl DataContractV1Setters for DataContract {
    /// Sets the groups map for the data contract.
    fn set_groups(&mut self, groups: BTreeMap<GroupContractPosition, Group>) {
        match self {
            DataContract::V0(_) => {}
            DataContract::V1(v1) => {
                v1.groups = groups;
            }
        }
    }

    /// Sets the tokens map for the data contract.
    fn set_tokens(&mut self, tokens: BTreeMap<TokenContractPosition, TokenConfiguration>) {
        match self {
            DataContract::V0(_) => {}
            DataContract::V1(v1) => {
                v1.tokens = tokens;
            }
        }
    }

    /// Adds or updates a single group in the groups map.
    fn add_group(&mut self, position: GroupContractPosition, group: Group) {
        match self {
            DataContract::V0(_) => {}
            DataContract::V1(v1) => {
                v1.groups.insert(position, group);
            }
        }
    }

    /// Adds or updates a single token configuration in the tokens map.
    fn add_token(&mut self, id: TokenContractPosition, token: TokenConfiguration) {
        match self {
            DataContract::V0(_) => {}
            DataContract::V1(v1) => {
                v1.tokens.insert(id, token);
            }
        }
    }
}
