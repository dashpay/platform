use crate::data_contract::data_contract::DataContractV0;
use crate::data_contract::serialized_version::v0::DataContractInSerializationFormatV0;
use crate::data_contract::{DataContract, DefinitionName, DocumentName};
use crate::version::PlatformVersion;
use std::collections::BTreeMap;

use crate::validation::operations::ProtocolValidationOperation;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use derive_more::From;
use platform_value::{Identifier, Value};
use platform_version::TryFromPlatformVersioned;
use platform_versioning::PlatformVersioned;
#[cfg(feature = "data-contract-serde-conversion")]
use serde::{Deserialize, Serialize};

pub(in crate::data_contract) mod v0;

pub const CONTRACT_DESERIALIZATION_LIMIT: usize = 15000;

#[derive(Debug, Clone, Encode, Decode, PartialEq, PlatformVersioned, From)]
#[cfg_attr(
    feature = "data-contract-serde-conversion",
    derive(Serialize, Deserialize),
    serde(tag = "$format_version")
)]
pub enum DataContractInSerializationFormat {
    #[cfg_attr(feature = "data-contract-serde-conversion", serde(rename = "0"))]
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

    pub fn document_schemas(&self) -> &BTreeMap<DocumentName, Value> {
        match self {
            DataContractInSerializationFormat::V0(v0) => &v0.document_schemas,
        }
    }

    pub fn schema_defs(&self) -> Option<&BTreeMap<DefinitionName, Value>> {
        match self {
            DataContractInSerializationFormat::V0(v0) => v0.schema_defs.as_ref(),
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

impl DataContract {
    pub fn try_from_platform_versioned(
        value: DataContractInSerializationFormat,
        validate: bool,
        validation_operations: &mut Vec<ProtocolValidationOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match value {
            DataContractInSerializationFormat::V0(serialization_format_v0) => {
                match platform_version
                    .dpp
                    .contract_versions
                    .contract_structure_version
                {
                    0 => {
                        let data_contract = DataContractV0::try_from_platform_versioned_v0(
                            serialization_format_v0,
                            validate,
                            validation_operations,
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
