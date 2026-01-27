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
use platform_version::TryFromPlatformVersioned;

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
use platform_version::version::PlatformVersion;

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

impl TryFromPlatformVersioned<(DataContract, IdentityNonce)> for DataContractUpdateTransitionV1 {
    type Error = ProtocolError;

    fn try_from_platform_versioned(
        value: (DataContract, IdentityNonce),
        _platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error> {
        let (data_contract, identity_nonce) = value;
        Ok(DataContractUpdateTransitionV1 {
            update_contract_system_version: None,
            id: data_contract.id(),
            owner_id: data_contract.owner_id(),
            revision: data_contract.version(),
            updated_schema_defs: BTreeMap::new(),
            new_schema_defs: data_contract.schema_defs().cloned().unwrap_or_default(),
            updated_document_schemas: BTreeMap::new(),
            new_document_schemas: BTreeMap::new(),
            new_groups: BTreeMap::new(),
            new_tokens: BTreeMap::new(),
            remove_keywords: Vec::new(),
            add_keywords: Vec::new(),
            update_description: None,
            identity_contract_nonce: identity_nonce,
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: Default::default(),
        })
    }
}
