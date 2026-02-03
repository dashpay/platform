mod identity_signed;
#[cfg(feature = "state-transition-json-conversion")]
mod json_conversion;
mod state_transition_like;
mod types;
pub(super) mod v0_methods;
#[cfg(feature = "state-transition-value-conversion")]
mod value_conversion;
mod version;

use std::collections::BTreeMap;

use platform_value::{BinaryData, Identifier, Value};
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};

use bincode::{Decode, Encode};
use platform_serialization_derive::PlatformSignable;

use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::associated_token::token_configuration::TokenConfiguration;
use crate::data_contract::group::Group;
use crate::data_contract::schema::DataContractSchemaMethodsV0;
use crate::data_contract::DataContract;
use crate::data_contract::{
    DefinitionName, DocumentName, GroupContractPosition, TokenContractPosition,
};
use crate::prelude::{IdentityNonce, UserFeeIncrease};
use crate::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use crate::state_transition::StateTransition;
use crate::{identity::KeyID, ProtocolError};

/// DataContractUpdateTransitionV1 stores the contract fields directly
/// rather than embedding a serialization format.

#[derive(Debug, Clone, Encode, Decode, PartialEq, PlatformSignable)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
pub struct DataContractUpdateTransitionV1 {
    /// Optional updated contract system version. When present, the contract
    /// will be upgraded to this system version.
    /// The system version defines the features of the contract.
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(default))]
    pub update_contract_system_version: Option<u16>,

    /// The unique identifier of the data contract being updated.
    pub id: Identifier,

    /// The identifier of the contract owner.
    pub owner_id: Identifier,

    /// The new revision number for this update.
    pub revision: u32,

    /// Updated shared subschemas ($defs) - must be compatible with existing ones.
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(default))]
    pub updated_schema_defs: BTreeMap<DefinitionName, Value>,

    /// New shared subschemas to add to $defs object (when none existed before).
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(default))]
    pub new_schema_defs: BTreeMap<DefinitionName, Value>,

    /// Updated document JSON Schemas for existing document types.
    /// Currently, we can not update document schemas as of version 3.1
    /// This will change in the future so we have this field
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(default))]
    pub updated_document_schemas: BTreeMap<DocumentName, Value>,

    /// New document JSON Schemas for new document types.
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(default))]
    pub new_document_schemas: BTreeMap<DocumentName, Value>,

    /// New groups that allow for specific multiparty actions on the contract.
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(default))]
    pub new_groups: BTreeMap<GroupContractPosition, Group>,

    /// New tokens on the contract.
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(default))]
    pub new_tokens: BTreeMap<TokenContractPosition, TokenConfiguration>,

    /// Keywords to remove from the contract.
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(default))]
    pub remove_keywords: Vec<String>,

    /// Keywords to add to the contract.
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(default))]
    pub add_keywords: Vec<String>,

    /// Updated description for the contract.
    /// None = don't update, Some(None) = clear description, Some(Some(value)) = set new description.
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(default))]
    pub update_description: Option<Option<String>>,

    /// The identity contract nonce.
    #[cfg_attr(
        feature = "state-transition-serde-conversion",
        serde(rename = "$identity-contract-nonce")
    )]
    pub identity_contract_nonce: IdentityNonce,

    /// User fee increase for priority processing.
    pub user_fee_increase: UserFeeIncrease,

    /// The public key id used to sign.
    #[platform_signable(exclude_from_sig_hash)]
    pub signature_public_key_id: KeyID,

    /// The signature.
    #[platform_signable(exclude_from_sig_hash)]
    pub signature: BinaryData,
}

impl From<DataContractUpdateTransitionV1> for StateTransition {
    fn from(value: DataContractUpdateTransitionV1) -> Self {
        let transition: DataContractUpdateTransition = value.into();
        transition.into()
    }
}

impl From<&DataContractUpdateTransitionV1> for StateTransition {
    fn from(value: &DataContractUpdateTransitionV1) -> Self {
        let transition: DataContractUpdateTransition = value.clone().into();
        transition.into()
    }
}

impl DataContractUpdateTransitionV1 {
    /// Creates a V1 update transition by computing the delta between old and new contracts.
    pub fn from_contract_update(
        old_contract: &DataContract,
        new_contract: &DataContract,
        identity_nonce: IdentityNonce,
    ) -> Result<Self, ProtocolError> {
        use crate::data_contract::accessors::v1::DataContractV1Getters;

        // Compute schema_defs delta
        let old_schema_defs = old_contract.schema_defs().cloned().unwrap_or_default();
        let new_schema_defs_map = new_contract.schema_defs().cloned().unwrap_or_default();

        let mut updated_schema_defs = BTreeMap::new();
        let mut new_schema_defs = BTreeMap::new();

        for (name, new_def) in &new_schema_defs_map {
            if let Some(old_def) = old_schema_defs.get(name) {
                if old_def != new_def {
                    updated_schema_defs.insert(name.clone(), new_def.clone());
                }
            } else {
                new_schema_defs.insert(name.clone(), new_def.clone());
            }
        }

        // Compute document schemas delta
        let old_doc_schemas: BTreeMap<_, _> = old_contract
            .document_schemas()
            .into_iter()
            .map(|(name, schema)| (name, schema.clone()))
            .collect();
        let new_doc_schemas: BTreeMap<_, _> = new_contract
            .document_schemas()
            .into_iter()
            .map(|(name, schema)| (name, schema.clone()))
            .collect();

        let mut updated_document_schemas = BTreeMap::new();
        let mut new_document_schemas = BTreeMap::new();

        for (name, new_schema) in &new_doc_schemas {
            if let Some(old_schema) = old_doc_schemas.get(name) {
                if old_schema != new_schema {
                    updated_document_schemas.insert(name.clone(), new_schema.clone());
                }
            } else {
                new_document_schemas.insert(name.clone(), new_schema.clone());
            }
        }

        // Compute groups delta (only new groups, can't modify existing)
        let old_groups = old_contract.groups();
        let new_groups_map = new_contract.groups();

        let mut new_groups = BTreeMap::new();
        for (pos, group) in new_groups_map {
            if !old_groups.contains_key(pos) {
                new_groups.insert(*pos, group.clone());
            }
        }

        // Compute tokens delta (only new tokens, can't modify existing)
        let old_tokens = old_contract.tokens();
        let new_tokens_map = new_contract.tokens();

        let mut new_tokens = BTreeMap::new();
        for (pos, token) in new_tokens_map {
            if !old_tokens.contains_key(pos) {
                new_tokens.insert(*pos, token.clone());
            }
        }

        // Compute keywords delta
        let old_keywords: std::collections::HashSet<_> =
            old_contract.keywords().iter().cloned().collect();
        let new_keywords: std::collections::HashSet<_> =
            new_contract.keywords().iter().cloned().collect();

        let remove_keywords: Vec<_> = old_keywords.difference(&new_keywords).cloned().collect();
        let add_keywords: Vec<_> = new_keywords.difference(&old_keywords).cloned().collect();

        // Compute description delta
        let update_description = if old_contract.description() != new_contract.description() {
            Some(new_contract.description().cloned())
        } else {
            None
        };

        Ok(DataContractUpdateTransitionV1 {
            update_contract_system_version: None,
            id: new_contract.id(),
            owner_id: new_contract.owner_id(),
            revision: new_contract.version(),
            updated_schema_defs,
            new_schema_defs,
            updated_document_schemas,
            new_document_schemas,
            new_groups,
            new_tokens,
            remove_keywords,
            add_keywords,
            update_description,
            identity_contract_nonce: identity_nonce,
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: Default::default(),
        })
    }
}
