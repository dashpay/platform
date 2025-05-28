use crate::data_contract::config::v0::DataContractConfigV0;
use crate::data_contract::config::DataContractConfig;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;

use crate::data_contract::v0::DataContractV0;
use crate::data_contract::v1::DataContractV1;
use crate::data_contract::{DataContract, DefinitionName, DocumentName};
use bincode::{Decode, Encode};
use platform_value::{Identifier, Value};
use platform_version::version::PlatformVersion;
use platform_version::FromPlatformVersioned;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Encode, Decode)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub struct DataContractInSerializationFormatV0 {
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
}

impl From<DataContract> for DataContractInSerializationFormatV0 {
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

                DataContractInSerializationFormatV0 {
                    id,
                    config,
                    version,
                    owner_id,
                    document_schemas: document_types
                        .into_iter()
                        .map(|(key, document_type)| (key, document_type.schema_owned()))
                        .collect(),
                    schema_defs,
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
                    ..
                } = v1;

                DataContractInSerializationFormatV0 {
                    id,
                    config,
                    version,
                    owner_id,
                    document_schemas: document_types
                        .into_iter()
                        .map(|(key, document_type)| (key, document_type.schema_owned()))
                        .collect(),
                    schema_defs,
                }
            }
        }
    }
}

impl FromPlatformVersioned<DataContract> for DataContractInSerializationFormatV0 {
    fn from_platform_versioned(value: DataContract, platform_version: &PlatformVersion) -> Self {
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

                let config = config.config_valid_for_platform_version(platform_version);

                DataContractInSerializationFormatV0 {
                    id,
                    config,
                    version,
                    owner_id,
                    document_schemas: document_types
                        .into_iter()
                        .map(|(key, document_type)| (key, document_type.schema_owned()))
                        .collect(),
                    schema_defs,
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
                    ..
                } = v1;

                let config = config.config_valid_for_platform_version(platform_version);

                DataContractInSerializationFormatV0 {
                    id,
                    config,
                    version,
                    owner_id,
                    document_schemas: document_types
                        .into_iter()
                        .map(|(key, document_type)| (key, document_type.schema_owned()))
                        .collect(),
                    schema_defs,
                }
            }
        }
    }
}
