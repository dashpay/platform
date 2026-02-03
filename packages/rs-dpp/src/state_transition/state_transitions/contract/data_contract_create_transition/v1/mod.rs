mod identity_signed;
#[cfg(feature = "state-transition-json-conversion")]
mod json_conversion;
mod state_transition_like;
mod types;
pub(crate) mod v0_methods;
#[cfg(feature = "state-transition-value-conversion")]
mod value_conversion;
mod version;

use platform_serialization_derive::PlatformSignable;
use std::collections::BTreeMap;

use platform_value::{BinaryData, Identifier, Value};
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};

use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::accessors::v1::DataContractV1Getters;
use crate::data_contract::schema::DataContractSchemaMethodsV0;
use crate::{data_contract::DataContract, identity::KeyID, ProtocolError};

use crate::data_contract::associated_token::token_configuration::TokenConfiguration;
use crate::data_contract::config::DataContractConfig;
use crate::data_contract::created_data_contract::CreatedDataContract;
use crate::data_contract::group::Group;
use crate::data_contract::{
    DefinitionName, DocumentName, GroupContractPosition, TokenContractPosition,
};
use crate::prelude::{IdentityNonce, UserFeeIncrease};
use crate::state_transition::data_contract_create_transition::DataContractCreateTransition;
use crate::state_transition::StateTransition;
use crate::version::PlatformVersion;
use bincode::{Decode, Encode};
use platform_version::TryFromPlatformVersioned;

/// DataContractCreateTransitionV1 stores the contract fields directly
/// rather than embedding a serialization format. The contract `id` is
/// derived from `owner_id + identity_nonce` and is not stored.

#[derive(Debug, Clone, Encode, Decode, PartialEq, PlatformSignable)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
pub struct DataContractCreateTransitionV1 {
    /// The contract system version specifying which system features should be
    /// activated for this data contract.
    pub contract_system_version: u16,

    /// The identifier of the contract owner (the identity creating the contract).
    pub owner_id: Identifier,

    /// Internal configuration for the contract.
    pub config: DataContractConfig,

    /// Shared subschemas to reuse across documents as $defs object.
    pub schema_defs: Option<BTreeMap<DefinitionName, Value>>,

    /// Document JSON Schemas per type.
    pub document_schemas: BTreeMap<DocumentName, Value>,

    /// Groups that allow for specific multiparty actions on the contract.
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(default))]
    pub groups: BTreeMap<GroupContractPosition, Group>,

    /// The tokens on the contract.
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(default))]
    pub tokens: BTreeMap<TokenContractPosition, TokenConfiguration>,

    /// The contract's keywords for searching.
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(default))]
    pub keywords: Vec<String>,

    /// The contract's description.
    #[cfg_attr(feature = "state-transition-serde-conversion", serde(default))]
    pub description: Option<String>,

    /// The identity nonce used to derive the contract id.
    pub identity_nonce: IdentityNonce,

    /// User fee increase for priority processing.
    pub user_fee_increase: UserFeeIncrease,

    /// The public key id used to sign.
    #[platform_signable(exclude_from_sig_hash)]
    pub signature_public_key_id: KeyID,

    /// The signature.
    #[platform_signable(exclude_from_sig_hash)]
    pub signature: BinaryData,
}

impl DataContractCreateTransitionV1 {
    /// Computes the data contract id from owner_id and identity_nonce.
    pub fn data_contract_id(&self) -> Identifier {
        DataContract::generate_data_contract_id_v0(self.owner_id, self.identity_nonce)
    }
}

impl From<DataContractCreateTransitionV1> for StateTransition {
    fn from(value: DataContractCreateTransitionV1) -> Self {
        let transition: DataContractCreateTransition = value.into();
        transition.into()
    }
}

impl From<&DataContractCreateTransitionV1> for StateTransition {
    fn from(value: &DataContractCreateTransitionV1) -> Self {
        let transition: DataContractCreateTransition = value.clone().into();
        transition.into()
    }
}

impl TryFromPlatformVersioned<DataContract> for DataContractCreateTransitionV1 {
    type Error = ProtocolError;

    fn try_from_platform_versioned(
        data_contract: DataContract,
        _platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error> {
        let contract_system_version = match &data_contract {
            DataContract::V0(_) => 0,
            DataContract::V1(_) => 1,
        };
        Ok(DataContractCreateTransitionV1 {
            contract_system_version,
            owner_id: data_contract.owner_id(),
            config: *data_contract.config(),
            schema_defs: data_contract.schema_defs().cloned(),
            document_schemas: data_contract
                .document_schemas()
                .into_iter()
                .map(|(k, v)| (k, v.clone()))
                .collect(),
            groups: data_contract.groups().clone(),
            tokens: data_contract.tokens().clone(),
            keywords: data_contract.keywords().clone(),
            description: data_contract.description().cloned(),
            identity_nonce: Default::default(),
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: Default::default(),
        })
    }
}

impl TryFromPlatformVersioned<CreatedDataContract> for DataContractCreateTransitionV1 {
    type Error = ProtocolError;

    fn try_from_platform_versioned(
        value: CreatedDataContract,
        _platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error> {
        let (data_contract, identity_nonce) = value.data_contract_and_identity_nonce();
        let contract_system_version = match &data_contract {
            DataContract::V0(_) => 0,
            DataContract::V1(_) => 1,
        };
        Ok(DataContractCreateTransitionV1 {
            contract_system_version,
            owner_id: data_contract.owner_id(),
            config: *data_contract.config(),
            schema_defs: data_contract.schema_defs().cloned(),
            document_schemas: data_contract
                .document_schemas()
                .into_iter()
                .map(|(k, v)| (k, v.clone()))
                .collect(),
            groups: data_contract.groups().clone(),
            tokens: data_contract.tokens().clone(),
            keywords: data_contract.keywords().clone(),
            description: data_contract.description().cloned(),
            identity_nonce,
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: Default::default(),
        })
    }
}
