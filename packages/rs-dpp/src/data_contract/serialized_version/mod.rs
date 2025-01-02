use crate::data_contract::serialized_version::v0::DataContractInSerializationFormatV0;
use crate::data_contract::{
    DataContract, DefinitionName, DocumentName, GroupContractPosition, TokenContractPosition,
    EMPTY_GROUPS, EMPTY_TOKENS,
};
use crate::version::PlatformVersion;
use std::collections::BTreeMap;

use crate::data_contract::associated_token::token_configuration::TokenConfiguration;
use crate::data_contract::group::Group;
use crate::data_contract::serialized_version::v1::DataContractInSerializationFormatV1;
use crate::data_contract::v0::DataContractV0;
use crate::data_contract::v1::DataContractV1;
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
pub(in crate::data_contract) mod v1;

pub mod property_names {
    pub const ID: &str = "id";
    pub const OWNER_ID: &str = "ownerId";
    pub const VERSION: &str = "version";
    pub const DEFINITIONS: &str = "$defs";
}

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
    #[cfg_attr(feature = "data-contract-serde-conversion", serde(rename = "1"))]
    V1(DataContractInSerializationFormatV1),
}

impl DataContractInSerializationFormat {
    /// Returns the unique identifier for the data contract.
    pub fn id(&self) -> Identifier {
        match self {
            DataContractInSerializationFormat::V0(v0) => v0.id,
            DataContractInSerializationFormat::V1(v1) => v1.id,
        }
    }

    /// Returns the owner identifier for the data contract.
    pub fn owner_id(&self) -> Identifier {
        match self {
            DataContractInSerializationFormat::V0(v0) => v0.owner_id,
            DataContractInSerializationFormat::V1(v1) => v1.owner_id,
        }
    }

    pub fn document_schemas(&self) -> &BTreeMap<DocumentName, Value> {
        match self {
            DataContractInSerializationFormat::V0(v0) => &v0.document_schemas,
            DataContractInSerializationFormat::V1(v1) => &v1.document_schemas,
        }
    }

    pub fn schema_defs(&self) -> Option<&BTreeMap<DefinitionName, Value>> {
        match self {
            DataContractInSerializationFormat::V0(v0) => v0.schema_defs.as_ref(),
            DataContractInSerializationFormat::V1(v1) => v1.schema_defs.as_ref(),
        }
    }

    pub fn version(&self) -> u32 {
        match self {
            DataContractInSerializationFormat::V0(v0) => v0.version,
            DataContractInSerializationFormat::V1(v1) => v1.version,
        }
    }

    pub fn groups(&self) -> &BTreeMap<GroupContractPosition, Group> {
        match self {
            DataContractInSerializationFormat::V0(_) => &EMPTY_GROUPS,
            DataContractInSerializationFormat::V1(v1) => &v1.groups,
        }
    }
    pub fn tokens(&self) -> &BTreeMap<TokenContractPosition, TokenConfiguration> {
        match self {
            DataContractInSerializationFormat::V0(_) => &EMPTY_TOKENS,
            DataContractInSerializationFormat::V1(v1) => &v1.tokens,
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
            1 => {
                let v1_format: DataContractInSerializationFormatV1 = DataContract::V0(value).into();
                Ok(v1_format.into())
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContract::serialize_to_default_current_version".to_string(),
                known_versions: vec![0, 1],
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
            1 => {
                let v1_format: DataContractInSerializationFormatV1 =
                    DataContract::V0(value.to_owned()).into();
                Ok(v1_format.into())
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContract::serialize_to_default_current_version".to_string(),
                known_versions: vec![0, 1],
                received: version,
            }),
        }
    }
}

impl TryFromPlatformVersioned<DataContractV1> for DataContractInSerializationFormat {
    type Error = ProtocolError;

    fn try_from_platform_versioned(
        value: DataContractV1,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error> {
        match platform_version
            .dpp
            .contract_versions
            .contract_serialization_version
            .default_current_version
        {
            0 => {
                let v0_format: DataContractInSerializationFormatV0 = DataContract::V1(value).into();
                Ok(v0_format.into())
            }
            1 => {
                let v1_format: DataContractInSerializationFormatV1 = DataContract::V1(value).into();
                Ok(v1_format.into())
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContract::serialize_to_default_current_version".to_string(),
                known_versions: vec![0, 1],
                received: version,
            }),
        }
    }
}

impl TryFromPlatformVersioned<&DataContractV1> for DataContractInSerializationFormat {
    type Error = ProtocolError;

    fn try_from_platform_versioned(
        value: &DataContractV1,
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
                    DataContract::V1(value.to_owned()).into();
                Ok(v0_format.into())
            }
            1 => {
                let v1_format: DataContractInSerializationFormatV1 =
                    DataContract::V1(value.to_owned()).into();
                Ok(v1_format.into())
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContract::serialize_to_default_current_version".to_string(),
                known_versions: vec![0, 1],
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
            1 => {
                let v1_format: DataContractInSerializationFormatV1 = value.clone().into();
                Ok(v1_format.into())
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContract::serialize_to_default_current_version".to_string(),
                known_versions: vec![0, 1],
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
            1 => {
                let v1_format: DataContractInSerializationFormatV1 = value.into();
                Ok(v1_format.into())
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContract::serialize_consume_to_default_current_version".to_string(),
                known_versions: vec![0, 1],
                received: version,
            }),
        }
    }
}

impl DataContract {
    pub fn try_from_platform_versioned(
        value: DataContractInSerializationFormat,
        full_validation: bool,
        validation_operations: &mut Vec<ProtocolValidationOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .contract_structure_version
        {
            0 => DataContractV0::try_from_platform_versioned(
                value,
                full_validation,
                validation_operations,
                platform_version,
            )
            .map(|contract| contract.into()),
            1 => DataContractV1::try_from_platform_versioned(
                value,
                full_validation,
                validation_operations,
                platform_version,
            )
            .map(|contract| contract.into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContract::try_from_platform_versioned".to_string(),
                known_versions: vec![0, 1],
                received: version,
            }),
        }
    }
}
