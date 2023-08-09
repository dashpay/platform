use crate::data_contract::config::v0::DataContractConfigGettersV0;
use crate::data_contract::document_type::DocumentType;
use crate::data_contract::serialized_version::v0::DataContractInSerializationFormatV0;
use crate::data_contract::v0::DataContractV0;
use crate::data_contract::DataContract;
use crate::version::{PlatformVersion, PlatformVersionCurrentVersion};
use crate::ProtocolError;
use platform_version::TryFromPlatformVersioned;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub mod bincode;

impl Serialize for DataContractV0 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let data_contract: DataContract = self.clone().into();
        let serialization_format = DataContractInSerializationFormatV0::from(data_contract);
        serialization_format.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for DataContractV0 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let serialization_format = DataContractInSerializationFormatV0::deserialize(deserializer)?;
        let current_version =
            PlatformVersion::get_current().map_err(|e| serde::de::Error::custom(e.to_string()))?;
        DataContractV0::try_from_platform_versioned(serialization_format, current_version)
            .map_err(serde::de::Error::custom)
    }
}

impl TryFromPlatformVersioned<DataContractInSerializationFormatV0> for DataContractV0 {
    type Error = ProtocolError;

    fn try_from_platform_versioned(
        data_contract_data: DataContractInSerializationFormatV0,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error> {
        let DataContractInSerializationFormatV0 {
            id,
            config,
            version,
            owner_id,
            document_schemas,
            schema_defs,
            ..
        } = data_contract_data;

        let document_types = DocumentType::create_document_types_from_document_schemas(
            id,
            document_schemas,
            schema_defs.as_ref(),
            config.documents_keep_history_contract_default(),
            config.documents_mutable_contract_default(),
            platform_version,
        )?;

        let data_contract = DataContractV0 {
            id,
            version,
            owner_id,
            document_types,
            metadata: None,
            config,
            schema_defs,
        };

        Ok(data_contract)
    }
}
