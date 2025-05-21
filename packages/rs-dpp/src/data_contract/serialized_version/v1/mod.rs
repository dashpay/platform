use crate::data_contract::config::v0::DataContractConfigV0;
use crate::data_contract::config::DataContractConfig;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;

use crate::block::epoch::EpochIndex;
use crate::data_contract::associated_token::token_configuration::TokenConfiguration;
use crate::data_contract::group::v0::GroupV0;
use crate::data_contract::group::Group;
use crate::data_contract::v0::DataContractV0;
use crate::data_contract::v1::DataContractV1;
use crate::data_contract::{
    DataContract, DefinitionName, DocumentName, GroupContractPosition, TokenContractPosition,
};
use crate::identity::TimestampMillis;
use crate::prelude::BlockHeight;
use bincode::{Decode, Encode};
use platform_value::{Identifier, Value};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
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

    /// The time in milliseconds that the contract was created.
    pub created_at: Option<TimestampMillis>,
    /// The time in milliseconds that the contract was last updated.
    pub updated_at: Option<TimestampMillis>,
    /// The block that the document was created.
    pub created_at_block_height: Option<BlockHeight>,
    /// The block that the contract was last updated
    pub updated_at_block_height: Option<BlockHeight>,
    /// The epoch at which the contract was created.
    pub created_at_epoch: Option<EpochIndex>,
    /// The epoch at which the contract was last updated.
    pub updated_at_epoch: Option<EpochIndex>,

    /// Groups that allow for specific multiparty actions on the contract
    #[serde(default, deserialize_with = "deserialize_u16_group_map")]
    pub groups: BTreeMap<GroupContractPosition, Group>,

    /// The tokens on the contract.
    #[serde(default, deserialize_with = "deserialize_u16_token_configuration_map")]
    pub tokens: BTreeMap<TokenContractPosition, TokenConfiguration>,

    /// The contract's keywords for searching
    #[serde(default)]
    pub keywords: Vec<String>,

    /// The contract's description
    #[serde(default)]
    pub description: Option<String>,
}

/// Deserialize `groups` map with stringified u16 keys and enum-wrapped `Group` values.
///
/// Accepts:
/// ```json
/// {
///   "0": {
///     "V0": {
///       "members": { "...": 1 },
///       "required_power": 3
///     }
///   }
/// }
/// ```
fn deserialize_u16_group_map<'de, D>(
    deserializer: D,
) -> Result<BTreeMap<GroupContractPosition, Group>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let map: BTreeMap<String, Value> = BTreeMap::deserialize(deserializer)?;

    let mut out = BTreeMap::new();

    for (key_str, group_value) in map {
        let key = key_str.parse::<GroupContractPosition>().map_err(|e| {
            serde::de::Error::custom(format!("invalid group key '{}': {}", key_str, e))
        })?;

        let group_obj = group_value.into_btree_string_map().map_err(|e| {
            serde::de::Error::custom(format!("invalid group structure at '{}': {}", key_str, e))
        })?;

        let v0_value = group_obj.get("V0").ok_or_else(|| {
            serde::de::Error::custom(format!("missing 'V0' variant under group '{}'", key_str))
        })?;

        let v0_json: JsonValue = v0_value.clone().try_into_validating_json().map_err(|e| {
            serde::de::Error::custom(format!("failed to convert to json at '{}': {}", key_str, e))
        })?;

        let group_v0: GroupV0 = serde_json::from_value(v0_json).map_err(|e| {
            serde::de::Error::custom(format!(
                "failed to deserialize GroupV0 at '{}': {}",
                key_str, e
            ))
        })?;

        out.insert(key, Group::V0(group_v0));
    }

    Ok(out)
}

fn deserialize_u16_token_configuration_map<'de, D>(
    deserializer: D,
) -> Result<BTreeMap<TokenContractPosition, TokenConfiguration>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let map: BTreeMap<String, TokenConfiguration> = BTreeMap::deserialize(deserializer)?;
    map.into_iter()
        .map(|(k, v)| {
            k.parse::<TokenContractPosition>()
                .map_err(serde::de::Error::custom)
                .map(|key| (key, v))
        })
        .collect()
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
                    created_at: None,
                    updated_at: None,
                    created_at_block_height: None,
                    updated_at_block_height: None,
                    created_at_epoch: None,
                    updated_at_epoch: None,
                    groups: Default::default(),
                    tokens: Default::default(),
                    keywords: Default::default(),
                    description: None,
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
                    created_at,
                    updated_at,
                    created_at_block_height,
                    updated_at_block_height,
                    created_at_epoch,
                    updated_at_epoch,
                    groups,
                    tokens,
                    keywords,
                    description,
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
                    created_at,
                    updated_at,
                    created_at_block_height,
                    updated_at_block_height,
                    created_at_epoch,
                    updated_at_epoch,
                    groups,
                    tokens,
                    keywords,
                    description,
                }
            }
        }
    }
}
