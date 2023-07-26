use crate::data_contract::data_contract_config::DataContractConfig;
use crate::data_contract::v0::DataContractV0;
use crate::data_contract::{DataContract, DefinitionName, DocumentName, PropertyPath};
use crate::identity::state_transition::asset_lock_proof::{Decode, Encode};
use platform_value::{Identifier, Value};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use crate::ProtocolError;

#[derive(Serialize, Deserialize, PlatformSerialize, PlatformDeserialize)]
#[platform_error_type(ProtocolError)]
#[platform_serialize(derive_bincode)]
#[serde(rename_all = "camelCase")]
pub struct DataContractSerializationFormatV0 {
    /// A unique identifier for the data contract.
    #[serde(rename = "$id")]
    pub id: Identifier,

    /// Internal configuration for the contract.
    pub config: DataContractConfig,

    /// A reference to the JSON schema that defines the contract.
    #[serde(rename = "$schema")]
    pub schema: String,

    /// The version of this data contract.
    pub version: u32,

    /// The identifier of the contract owner.
    pub owner_id: Identifier,

    /// A mapping of document names to their corresponding JSON values.
    pub documents: BTreeMap<DocumentName, Value>,

    /// Optional mapping of definition names to their corresponding JSON values.
    #[serde(rename = "$defs", default)]
    pub defs: Option<BTreeMap<DefinitionName, Value>>,
}

impl From<DataContract> for DataContractSerializationFormatV0 {
    fn from(value: DataContract) -> Self {
        match value {
            DataContract::V0(v0) => {
                let DataContractV0 {
                    id,
                    config,
                    schema,
                    version,
                    owner_id,
                    documents,
                    defs,
                    ..
                } = v0;
                DataContractSerializationFormatV0 {
                    id,
                    config,
                    schema,
                    version,
                    owner_id,
                    documents: documents
                        .into_iter()
                        .map(|(key, value)| (key, value.into()))
                        .collect(),
                    defs: defs.map(|defs| {
                        defs.into_iter()
                            .map(|(key, value)| (key, value.into()))
                            .collect()
                    }),
                }
            }
        }
    }
}
