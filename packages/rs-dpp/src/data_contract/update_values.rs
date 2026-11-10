//! What a delta contract update changes in a stored contract.
//!
//! Version 1 of the
//! [`DataContractUpdateTransition`](crate::state_transition::data_contract_update_transition::DataContractUpdateTransition)
//! carries only what an update changes. [`DataContractUpdateValues`] is the borrowed view of
//! that delta which [`DataContract::apply_update`](crate::data_contract::DataContract::apply_update)
//! merges onto the stored contract.

use crate::data_contract::associated_token::token_configuration::TokenConfiguration;
use crate::data_contract::config::DataContractConfig;
use crate::data_contract::group::Group;
use crate::data_contract::serialized_version::v1::DataContractInSerializationFormatV1;
use crate::data_contract::serialized_version::DataContractInSerializationFormat;
use crate::data_contract::{
    DefinitionName, DocumentName, GroupContractPosition, TokenContractPosition,
};
use crate::fee::Credits;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_value::{Identifier, Value};
use platform_version::version::PlatformVersion;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[cfg(feature = "json-conversion")]
use crate::serialization::JsonSafeFields;

/// What a contract update does to the contract's description.
///
/// An `Option<Option<String>>` does not survive a JSON round trip (`None` and `Some(None)`
/// both serialize as `null`), so the three cases are spelled out.
#[derive(Debug, Clone, PartialEq, Eq, Default, Encode, Decode, DecodeUntrusted)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
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

/// A borrowed view of what a delta contract update changes.
///
/// Every section adds entries or replaces a named entry in place. A contract update may not
/// remove a document type, a schema definition, a group or a token, nor change a group or a
/// token, so a delta has no way to say so.
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
    /// Keywords to add, after the stored ones that stay.
    pub add_keywords: &'a [String],
    /// Keywords to remove.
    pub remove_keywords: &'a [String],
    /// What to do with the description.
    pub description: &'a DescriptionUpdate,
}

impl DataContractUpdateValues<'_> {
    /// The registration cost of what this delta registers.
    ///
    /// An update that carries the whole contract pays the registration cost of the whole
    /// contract again. A delta registers only its new and replaced document types (with their
    /// indexes), its new tokens and its added keywords, so it pays for those alone: the
    /// registration cost of a contract holding only them, less that of a contract holding
    /// nothing, which is the fee of registering a contract. Both are priced by
    /// [`DataContractInSerializationFormat::registration_cost`], so a delta follows every
    /// generation of the registration cost.
    pub fn registration_cost(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Credits, ProtocolError> {
        let contract_registering = |document_schemas, tokens, keywords| {
            DataContractConfig::default_for_version(platform_version).map(|config| {
                DataContractInSerializationFormat::V1(DataContractInSerializationFormatV1 {
                    id: Identifier::default(),
                    config,
                    version: self.version,
                    owner_id: self.owner_id,
                    schema_defs: None,
                    document_schemas,
                    created_at: None,
                    updated_at: None,
                    created_at_block_height: None,
                    updated_at_block_height: None,
                    created_at_epoch: None,
                    updated_at_epoch: None,
                    groups: BTreeMap::new(),
                    tokens,
                    keywords,
                    description: None,
                })
            })
        };

        let registered = contract_registering(
            self.new_document_schemas
                .iter()
                .chain(self.updated_document_schemas)
                .map(|(name, schema)| (name.clone(), schema.clone()))
                .collect(),
            self.new_tokens.clone(),
            self.add_keywords.to_vec(),
        )?
        .registration_cost(platform_version)?;
        let nothing_registered = contract_registering(BTreeMap::new(), BTreeMap::new(), vec![])?
            .registration_cost(platform_version)?;

        Ok(registered.saturating_sub(nothing_registered))
    }
}
