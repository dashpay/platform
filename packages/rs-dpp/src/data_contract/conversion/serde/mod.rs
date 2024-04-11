use crate::data_contract::serialized_version::DataContractInSerializationFormat;
use crate::prelude::DataContract;
use crate::version::PlatformVersionCurrentVersion;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;
use platform_version::TryIntoPlatformVersioned;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

impl Serialize for DataContract {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let current_version =
            PlatformVersion::get_current().map_err(|e| serde::ser::Error::custom(e.to_string()))?;
        let data_contract_in_serialization_format: DataContractInSerializationFormat = self
            .try_into_platform_versioned(current_version)
            .map_err(|e: ProtocolError| serde::ser::Error::custom(e.to_string()))?;
        data_contract_in_serialization_format.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for DataContract {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let serialization_format = DataContractInSerializationFormat::deserialize(deserializer)?;
        let current_version =
            PlatformVersion::get_current().map_err(|e| serde::de::Error::custom(e.to_string()))?;
        // when deserializing from json/platform_value/cbor we always want to validate (as this is not coming from the state)
        DataContract::try_from_platform_versioned(
            serialization_format,
            true,
            &mut vec![],
            current_version,
        )
        .map_err(serde::de::Error::custom)
    }
}
