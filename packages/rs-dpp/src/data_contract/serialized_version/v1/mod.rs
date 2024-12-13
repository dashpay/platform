use crate::data_contract::config::v0::DataContractConfigV0;
use crate::data_contract::config::DataContractConfig;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;

use crate::data_contract::associated_token::token_configuration::TokenConfiguration;
use crate::data_contract::group::{Group, GroupName};
use crate::data_contract::v0::DataContractV0;
use crate::data_contract::v1::DataContractV1;
use crate::data_contract::{DataContract, DefinitionName, DocumentName, TokenName};
use crate::identity::state_transition::asset_lock_proof::{Decode, Encode};
use platform_value::{Identifier, Value};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Encode, Decode)]
#[serde(rename_all = "camelCase")]
pub struct DataContractInSerializationFormatV1 {
    /// A unique identifier for the data contract.
    pub id: Identifier,

    /// Internal configuration for the contract.
    #[serde(default = "DataContractConfigV0::default_with_version")]
    pub config: DataContractConfig,

    /// The version of this data contract.
    pub version: u32,

    /// The identifier of the contract owner.
    pub owner_id: Identifier,

    /// Shared subschemas to reuse across documents as $defs object
    pub schema_defs: Option<BTreeMap<DefinitionName, Value>>,

    /// Document JSON Schemas per type
    pub document_schemas: BTreeMap<DocumentName, Value>,

    /// Groups that allow for specific multiparty actions on the contract
    pub groups: BTreeMap<GroupName, Group>,

    /// The tokens on the contract.
    pub tokens: BTreeMap<TokenName, TokenConfiguration>,
}

impl From<DataContract> for DataContractInSerializationFormatV1 {
    fn from(value: DataContract) -> Self {
        match value {
            DataContract::V0(v0) => {
                let DataContractV0 {
                    id,
                    config,
                    version,
                    owner_id,
                    schema_defs,
                    document_types,
                    ..
                } = v0;

                DataContractInSerializationFormatV1 {
                    id,
                    config,
                    version,
                    owner_id,
                    schema_defs,
                    document_schemas: document_types
                        .into_iter()
                        .map(|(key, document_type)| (key, document_type.schema_owned()))
                        .collect(),
                    groups: Default::default(),
                    tokens: Default::default(),
                }
            }
            DataContract::V1(v1) => {
                let DataContractV1 {
                    id,
                    config,
                    version,
                    owner_id,
                    schema_defs,
                    document_types,
                    groups,
                    tokens,
                    ..
                } = v1;

                DataContractInSerializationFormatV1 {
                    id,
                    config,
                    version,
                    owner_id,
                    schema_defs,
                    document_schemas: document_types
                        .into_iter()
                        .map(|(key, document_type)| (key, document_type.schema_owned()))
                        .collect(),
                    groups,
                    tokens,
                }
            }
        }
    }
}
