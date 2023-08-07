use crate::data_contract::data_contract::DataContractV0;
use crate::data_contract::serialized_version::v0::DataContractInSerializationFormatV0;
use crate::data_contract::DataContract;
use crate::version::PlatformVersion;
use crate::version::PlatformVersionCurrentVersion;
use crate::ProtocolError;
use bincode::{BorrowDecode, Decode, Encode};
use derive_more::From;
use platform_value::Identifier;
use platform_version::TryFromPlatformVersioned;
use platform_versioning::{
    PlatformSerdeVersionedDeserialize, PlatformSerdeVersionedSerialize, PlatformVersioned,
};
use serde::{Deserialize, Serialize};

pub(in crate::data_contract) mod v0;

pub const CONTRACT_DESERIALIZATION_LIMIT: usize = 15000;

#[derive(Debug, Clone, Encode, Decode, PartialEq, PlatformVersioned, From)]
#[cfg_attr(
    feature = "data-contract-serde-conversion",
    derive(Serialize, PlatformSerdeVersionedDeserialize)
)]
#[platform_version_path_bounds("dpp.contract_versions.contract_serialization_version")]
pub enum DataContractInSerializationFormat {
    #[cfg_attr(feature = "state-transition-serde-conversion", versioned(0))]
    V0(DataContractInSerializationFormatV0),
}

impl DataContractInSerializationFormat {
    /// Returns the unique identifier for the data contract.
    pub fn id(&self) -> Identifier {
        match self {
            DataContractInSerializationFormat::V0(v0) => v0.id,
        }
    }

    /// Returns the owner identifier for the data contract.
    pub fn owner_id(&self) -> Identifier {
        match self {
            DataContractInSerializationFormat::V0(v0) => v0.owner_id,
        }
    }
}

impl TryFromPlatformVersioned<DataContractV0> for DataContractInSerializationFormat {
    type Error = ProtocolError;

    fn try_from_platform_versioned(
        value: DataContractV0,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error> {
        match platform_version
            .dpp
            .contract_versions
            .contract_serialization_version
            .default_current_version
        {
            0 => {
                let v0_format: DataContractInSerializationFormatV0 = DataContract::V0(value).into();
                Ok(v0_format.into())
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContract::serialize_to_default_current_version".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

impl TryFromPlatformVersioned<&DataContractV0> for DataContractInSerializationFormat {
    type Error = ProtocolError;

    fn try_from_platform_versioned(
        value: &DataContractV0,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error> {
        match platform_version
            .dpp
            .contract_versions
            .contract_serialization_version
            .default_current_version
        {
            0 => {
                let v0_format: DataContractInSerializationFormatV0 =
                    DataContract::V0(value.to_owned()).into();
                Ok(v0_format.into())
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContract::serialize_to_default_current_version".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

impl TryFromPlatformVersioned<&DataContract> for DataContractInSerializationFormat {
    type Error = ProtocolError;

    fn try_from_platform_versioned(
        value: &DataContract,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error> {
        match platform_version
            .dpp
            .contract_versions
            .contract_serialization_version
            .default_current_version
        {
            0 => {
                let v0_format: DataContractInSerializationFormatV0 = value.clone().into();
                Ok(v0_format.into())
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContract::serialize_to_default_current_version".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

impl TryFromPlatformVersioned<DataContract> for DataContractInSerializationFormat {
    type Error = ProtocolError;

    fn try_from_platform_versioned(
        value: DataContract,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error> {
        match platform_version
            .dpp
            .contract_versions
            .contract_serialization_version
            .default_current_version
        {
            0 => {
                let v0_format: DataContractInSerializationFormatV0 = value.into();
                Ok(v0_format.into())
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContract::serialize_consume_to_default_current_version".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

impl TryFromPlatformVersioned<DataContractInSerializationFormat> for DataContract {
    type Error = ProtocolError;

    fn try_from_platform_versioned(
        value: DataContractInSerializationFormat,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error> {
        match value {
            DataContractInSerializationFormat::V0(serialization_format_v0) => {
                match platform_version
                    .dpp
                    .contract_versions
                    .contract_structure_version
                {
                    0 => {
                        let data_contract = DataContractV0::try_from_platform_versioned(
                            serialization_format_v0,
                            platform_version,
                        )?;
                        Ok(data_contract.into())
                    }
                    version => Err(ProtocolError::UnknownVersionMismatch {
                        method: "DataContract::from_serialization_format".to_string(),
                        known_versions: vec![0],
                        received: version,
                    }),
                }
            }
        }
    }
}

impl TryFromPlatformVersioned<DataContractInSerializationFormat> for DataContractV0 {
    type Error = ProtocolError;

    fn try_from_platform_versioned(
        value: DataContractInSerializationFormat,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error> {
        match value {
            DataContractInSerializationFormat::V0(serialization_format_v0) => {
                match platform_version
                    .dpp
                    .contract_versions
                    .contract_structure_version
                {
                    0 => {
                        let data_contract = DataContractV0::try_from_platform_versioned(
                            serialization_format_v0,
                            platform_version,
                        )?;

                        Ok(data_contract)
                    }
                    version => Err(ProtocolError::UnknownVersionMismatch {
                        method: "DataContract::from_serialization_format".to_string(),
                        known_versions: vec![0],
                        received: version,
                    }),
                }
            }
        }
    }
}
