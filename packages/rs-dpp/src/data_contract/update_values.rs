//! The changes a delta-based contract update applies to a stored contract.
//!
//! A V1 [`DataContractUpdateTransition`](crate::state_transition::data_contract_update_transition::DataContractUpdateTransition)
//! carries only what changed. [`DataContractUpdateValues`] is the borrowed
//! view of that delta which [`DataContract::apply_update`](crate::data_contract::DataContract::apply_update)
//! merges onto the stored contract to produce the updated one.

use crate::data_contract::associated_token::token_configuration::TokenConfiguration;
use crate::data_contract::config::DataContractConfig;
use crate::data_contract::group::Group;
use crate::data_contract::{
    DefinitionName, DocumentName, GroupContractPosition, TokenContractPosition,
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
