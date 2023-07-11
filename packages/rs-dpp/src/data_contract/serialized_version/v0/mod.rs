use crate::data_contract::contract_config::ContractConfigV0;
use crate::data_contract::v0::DataContractV0;
use crate::data_contract::{
    DataContract, DataContractV0Methods, DefinitionName, DocumentName, JsonSchema, PropertyPath,
};
use crate::identity::state_transition::asset_lock_proof::{Decode, Encode};
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::{Identifier, Value};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;
use std::convert::{TryFrom, TryInto};

#[derive(Encode, Decode)]
pub struct DataContractSerializationFormatV0 {
    /// A unique identifier for the data contract.
    pub id: Identifier,

    /// Internal configuration for the contract.
    pub config: ContractConfigV0,

    /// A reference to the JSON schema that defines the contract.
    pub schema: String,

    /// The version of this data contract.
    pub version: u32,

    /// The identifier of the contract owner.
    pub owner_id: Identifier,

    /// A mapping of document names to their corresponding JSON values.
    pub documents: BTreeMap<DocumentName, Value>,

    /// Optional mapping of definition names to their corresponding JSON values.
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

impl DataContractSerializationFormatV0 {
    pub(in crate::data_contract) fn try_into(
        self,
        platform_version: &PlatformVersion,
    ) -> Result<DataContractV0, ProtocolError> {
        DataContractV0::try_from(self, platform_version)
    }
}

impl DataContractV0 {
    pub(in crate::data_contract) fn try_from(
        value: DataContractSerializationFormatV0,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let DataContractSerializationFormatV0 {
            id,
            config,
            schema,
            version,
            owner_id,
            documents,
            defs,
            ..
        } = value;

        let document_types = DataContract::get_document_types_from_value_array(
            id,
            &documents
                .iter()
                .map(|(key, value)| (key.as_str(), value))
                .collect(),
            &defs
                .as_ref()
                .map(|defs| {
                    defs.iter()
                        .map(|(key, value)| Ok((key.clone(), value)))
                        .collect::<Result<BTreeMap<String, &Value>, ProtocolError>>()
                })
                .transpose()?
                .unwrap_or_default(),
            config.documents_keep_history_contract_default,
            config.documents_mutable_contract_default,
            platform_version,
        )?;

        let binary_properties = documents
            .iter()
            .map(|(doc_type, schema)| Ok((String::from(doc_type), DataContract::get_binary_properties(&schema.clone().try_into()?, platform_version))))
            .collect::<Result<BTreeMap<DocumentName, BTreeMap<PropertyPath, JsonValue>>, ProtocolError>>()?;

        let data_contract = DataContractV0 {
            id,
            schema,
            version,
            owner_id,
            document_types,
            metadata: None,
            config,
            documents: documents
                .into_iter()
                .map(|(key, value)| Ok((key, value.try_into()?)))
                .collect::<Result<BTreeMap<DocumentName, JsonSchema>, ProtocolError>>()?,
            defs: defs
                .map(|defs| {
                    defs.into_iter()
                        .map(|(key, value)| Ok((key, value.try_into()?)))
                        .collect::<Result<BTreeMap<DefinitionName, JsonSchema>, ProtocolError>>()
                })
                .transpose()?,
            binary_properties,
        };

        Ok(data_contract)
    }
}
