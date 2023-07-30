use crate::data_contract::data_contract_config::v0::DataContractConfigGettersV0;
use crate::data_contract::serialized_version::v0::DataContractSerializationFormatV0;
use crate::data_contract::v0::DataContractV0;
use crate::data_contract::{DataContract, DefinitionName, DocumentName, JsonSchema, PropertyPath};
use crate::version::{PlatformVersion, PlatformVersionCurrentVersion};
use crate::ProtocolError;
use platform_value::Value;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;
use std::convert::TryInto;

pub mod bincode;

impl Serialize for DataContractV0 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let data_contract: DataContract = self.clone().into();
        let serialization_format = DataContractSerializationFormatV0::from(data_contract);
        serialization_format.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for DataContractV0 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let serialization_format = DataContractSerializationFormatV0::deserialize(deserializer)?;
        let current_version =
            PlatformVersion::get_current().map_err(|e| serde::de::Error::custom(e.to_string()))?;
        DataContractV0::try_from(serialization_format, current_version)
            .map_err(serde::de::Error::custom)
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
    // TODO: Here we should do structure validation
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

        // TODO: Validate schema

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
            // TODO: Why they are default? Do we have Anton
            config.documents_keep_history_contract_default(),
            config.documents_mutable_contract_default(),
            platform_version,
        )?;

        // TODO: Those must be consensus errors or we do it in the next task
        let binary_properties = documents
            .iter()
            .map(|(doc_type, schema)| Ok((String::from(doc_type), DataContract::get_binary_properties(&schema.clone().try_into()?, platform_version)?)))
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
