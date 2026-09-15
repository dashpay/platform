//! The changes a delta-based contract update applies to a stored contract.
//!
//! A V1 [`DataContractUpdateTransition`](crate::state_transition::data_contract_update_transition::DataContractUpdateTransition)
//! carries only what changed. [`DataContractUpdateValues`] is the borrowed
//! view of that delta which [`DataContract::apply_update`](crate::data_contract::DataContract::apply_update)
//! merges onto the stored contract to produce the updated one.

use crate::block::block_info::BlockInfo;
use crate::consensus::basic::data_contract::DataContractUpdateEntryKind;
use crate::consensus::state::data_contract::data_contract_update_entry_already_exists_error::DataContractUpdateEntryAlreadyExistsError;
use crate::consensus::state::data_contract::data_contract_update_entry_not_found_error::DataContractUpdateEntryNotFoundError;
use crate::consensus::state::data_contract::data_contract_update_permission_error::DataContractUpdatePermissionError;
use crate::consensus::ConsensusError;
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::accessors::v1::DataContractV1Getters;
use crate::data_contract::associated_token::token_configuration::TokenConfiguration;
use crate::data_contract::config::DataContractConfig;
use crate::data_contract::group::Group;
use crate::data_contract::schema::DataContractSchemaMethodsV0;
use crate::data_contract::serialized_version::v1::DataContractInSerializationFormatV1;
use crate::data_contract::{
    DataContract, DefinitionName, DocumentName, GroupContractPosition, TokenContractPosition,
};
use bincode::{Decode, Encode};
use platform_value::{Identifier, Value};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[cfg(feature = "json-conversion")]
use crate::serialization::JsonSafeFields;

/// What a contract update does to the contract description.
///
/// A plain `Option<Option<String>>` cannot survive a JSON round trip
/// (`None` and `Some(None)` both serialize as `null`), so the three cases
/// are spelled out.
#[derive(Debug, Clone, PartialEq, Eq, Default, Encode, Decode, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DescriptionUpdate {
    /// Leave the description as it is.
    #[default]
    Keep,
    /// Remove the description.
    Clear,
    /// Replace the description.
    Set(String),
}

#[cfg(feature = "json-conversion")]
impl JsonSafeFields for DescriptionUpdate {}

/// A borrowed view of the changes a delta-based contract update applies.
///
/// Every section is additive or an in-place change of a named entry.
/// Removing document types, schema definitions, groups or tokens, and
/// changing existing groups or tokens, is not a legal contract update, so
/// the delta has no way to express it.
#[derive(Debug, Clone, Copy)]
pub struct DataContractUpdateValues<'a> {
    /// The identity submitting the update. It must own the stored contract.
    pub owner_id: Identifier,
    /// The contract version the update produces.
    pub version: u32,
    /// A replacement config, or `None` to keep the stored one.
    pub config: Option<&'a DataContractConfig>,
    /// New schemas for existing shared `$defs` definitions.
    pub updated_schema_defs: &'a BTreeMap<DefinitionName, Value>,
    /// Shared `$defs` definitions to add.
    pub new_schema_defs: &'a BTreeMap<DefinitionName, Value>,
    /// New schemas for existing document types.
    pub updated_document_schemas: &'a BTreeMap<DocumentName, Value>,
    /// Document types to add.
    pub new_document_schemas: &'a BTreeMap<DocumentName, Value>,
    /// Groups to add.
    pub new_groups: &'a BTreeMap<GroupContractPosition, Group>,
    /// Tokens to add.
    pub new_tokens: &'a BTreeMap<TokenContractPosition, TokenConfiguration>,
    /// Keywords to add.
    pub add_keywords: &'a [String],
    /// Keywords to remove.
    pub remove_keywords: &'a [String],
    /// What to do with the description.
    pub description: &'a DescriptionUpdate,
}

impl DataContractUpdateValues<'_> {
    /// Merges this delta onto `stored` and returns the contract it produces,
    /// in serialization form.
    ///
    /// This is the one definition of what a delta means. Consensus applies
    /// it through [`DataContract::apply_update`](crate::data_contract::DataContract::apply_update)
    /// and then holds the result to the full update rules; proof
    /// verification applies it to the pre-update contract the client
    /// already holds, to learn the exact contract the delta must have
    /// produced. Only what the delta shape itself makes checkable is
    /// checked here: the owner matches, updated entries exist, new entries
    /// do not, a keyword to remove is present and one to add is not.
    /// Group and token rules on the merged result stay with the callers.
    ///
    /// `block_info` stamps the `updatedAt` fields.
    ///
    /// # Returns
    /// - `Ok(DataContractInSerializationFormatV1)`: the merged contract.
    /// - `Err(ConsensusError)`: the delta does not fit the stored contract.
    pub fn merge_onto(
        &self,
        stored: &DataContract,
        block_info: &BlockInfo,
    ) -> Result<DataContractInSerializationFormatV1, ConsensusError> {
        let contract_id = stored.id();

        // The delta names its owner explicitly, so the ownership check that
        // a full-contract update gets from comparing embedded and stored
        // owners happens here, before anything is merged.
        if self.owner_id != stored.owner_id() {
            return Err(DataContractUpdatePermissionError::new(contract_id, self.owner_id).into());
        }

        let not_found = |kind: DataContractUpdateEntryKind, name: String| {
            Err(DataContractUpdateEntryNotFoundError::new(contract_id, kind, name).into())
        };
        let already_exists = |kind: DataContractUpdateEntryKind, name: String| {
            Err(DataContractUpdateEntryAlreadyExistsError::new(contract_id, kind, name).into())
        };

        let mut document_schemas: BTreeMap<DocumentName, Value> = stored
            .document_schemas()
            .into_iter()
            .map(|(name, schema)| (name, schema.clone()))
            .collect();
        for (name, schema) in self.updated_document_schemas {
            match document_schemas.get_mut(name) {
                Some(existing) => *existing = schema.clone(),
                None => return not_found(DataContractUpdateEntryKind::DocumentType, name.clone()),
            }
        }
        for (name, schema) in self.new_document_schemas {
            if document_schemas
                .insert(name.clone(), schema.clone())
                .is_some()
            {
                return already_exists(DataContractUpdateEntryKind::DocumentType, name.clone());
            }
        }

        let mut schema_defs = stored.schema_defs().cloned();
        for (name, definition) in self.updated_schema_defs {
            match schema_defs
                .as_mut()
                .and_then(|definitions| definitions.get_mut(name))
            {
                Some(existing) => *existing = definition.clone(),
                None => return not_found(DataContractUpdateEntryKind::SchemaDef, name.clone()),
            }
        }
        for (name, definition) in self.new_schema_defs {
            if schema_defs
                .get_or_insert_with(BTreeMap::new)
                .insert(name.clone(), definition.clone())
                .is_some()
            {
                return already_exists(DataContractUpdateEntryKind::SchemaDef, name.clone());
            }
        }

        let mut groups = stored.groups().clone();
        for (position, group) in self.new_groups {
            if groups.insert(*position, group.clone()).is_some() {
                return already_exists(DataContractUpdateEntryKind::Group, position.to_string());
            }
        }

        let mut tokens = stored.tokens().clone();
        for (position, token) in self.new_tokens {
            if tokens.insert(*position, token.clone()).is_some() {
                return already_exists(DataContractUpdateEntryKind::Token, position.to_string());
            }
        }

        let mut keywords = stored.keywords().clone();
        for keyword in self.remove_keywords {
            match keywords.iter().position(|existing| existing == keyword) {
                Some(index) => {
                    keywords.remove(index);
                }
                None => return not_found(DataContractUpdateEntryKind::Keyword, keyword.clone()),
            }
        }
        for keyword in self.add_keywords {
            if keywords.contains(keyword) {
                return already_exists(DataContractUpdateEntryKind::Keyword, keyword.clone());
            }
            keywords.push(keyword.clone());
        }

        let description = match self.description {
            DescriptionUpdate::Keep => stored.description().cloned(),
            DescriptionUpdate::Clear => None,
            DescriptionUpdate::Set(description) => Some(description.clone()),
        };

        let config = self.config.copied().unwrap_or_else(|| *stored.config());

        Ok(DataContractInSerializationFormatV1 {
            id: contract_id,
            config,
            version: self.version,
            owner_id: stored.owner_id(),
            schema_defs,
            document_schemas,
            created_at: stored.created_at(),
            updated_at: Some(block_info.time_ms),
            created_at_block_height: stored.created_at_block_height(),
            updated_at_block_height: Some(block_info.height),
            created_at_epoch: stored.created_at_epoch(),
            updated_at_epoch: Some(block_info.epoch.index),
            groups,
            tokens,
            keywords,
            description,
        })
    }
}
