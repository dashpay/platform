use super::EMPTY_KEYWORDS;
use crate::data_contract::associated_token::token_configuration::TokenConfiguration;
use crate::data_contract::group::Group;
use crate::data_contract::serialized_version::v0::DataContractInSerializationFormatV0;
use crate::data_contract::serialized_version::v1::DataContractInSerializationFormatV1;
use crate::data_contract::v0::DataContractV0;
use crate::data_contract::v1::DataContractV1;
use crate::data_contract::{
    DataContract, DefinitionName, DocumentName, GroupContractPosition, TokenContractPosition,
    EMPTY_GROUPS, EMPTY_TOKENS,
};
use crate::validation::operations::ProtocolValidationOperation;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use derive_more::From;
use platform_value::{Identifier, Value};
use platform_version::{IntoPlatformVersioned, TryFromPlatformVersioned};
use platform_versioning::PlatformVersioned;
#[cfg(feature = "data-contract-serde-conversion")]
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

pub(in crate::data_contract) mod v0;
pub(in crate::data_contract) mod v1;

pub mod property_names {
    pub const ID: &str = "id";
    pub const OWNER_ID: &str = "ownerId";
    pub const VERSION: &str = "version";
    pub const DEFINITIONS: &str = "$defs";
}

pub const CONTRACT_DESERIALIZATION_LIMIT: usize = 15000;

/// Represents a field mismatch between two `DataContractInSerializationFormat::V1`
/// variants, or indicates a format version mismatch.
///
/// Used to diagnose why two data contracts are not considered equal
/// when ignoring auto-generated fields.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum DataContractMismatch {
    /// The `id` fields are not equal.
    Id,
    /// The `config` fields are not equal.
    Config,
    /// The `version` fields are not equal.
    Version,
    /// The `owner_id` fields are not equal.
    OwnerId,
    /// The `schema_defs` fields are not equal.
    SchemaDefs,
    /// The `document_schemas` fields are not equal.
    DocumentSchemas,
    /// The `groups` fields are not equal.
    Groups,
    /// The `tokens` fields are not equal.
    Tokens,
    /// The `keywords` fields are not equal.
    Keywords,
    /// The `description` fields are not equal.
    Description,
    /// The two variants are of different serialization formats (e.g., V0 vs V1).
    FormatVersionMismatch,
    /// The two variants are different in V0.
    V0Mismatch,
}

impl fmt::Display for DataContractMismatch {
    /// Formats the enum into a human-readable string describing the mismatch.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let description = match self {
            DataContractMismatch::Id => "ID fields differ",
            DataContractMismatch::Config => "Config fields differ",
            DataContractMismatch::Version => "Version fields differ",
            DataContractMismatch::OwnerId => "Owner ID fields differ",
            DataContractMismatch::SchemaDefs => "Schema definitions differ",
            DataContractMismatch::DocumentSchemas => "Document schemas differ",
            DataContractMismatch::Groups => "Groups differ",
            DataContractMismatch::Tokens => "Tokens differ",
            DataContractMismatch::Keywords => "Keywords differ",
            DataContractMismatch::Description => "Description fields differ",
            DataContractMismatch::FormatVersionMismatch => {
                "Serialization format versions differ (e.g., V0 vs V1)"
            }
            DataContractMismatch::V0Mismatch => "V0 versions differ",
        };
        write!(f, "{}", description)
    }
}

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

    pub fn keywords(&self) -> &Vec<String> {
        match self {
            DataContractInSerializationFormat::V0(_) => &EMPTY_KEYWORDS,
            DataContractInSerializationFormat::V1(v1) => &v1.keywords,
        }
    }

    pub fn description(&self) -> &Option<String> {
        match self {
            DataContractInSerializationFormat::V0(_) => &None,
            DataContractInSerializationFormat::V1(v1) => &v1.description,
        }
    }

    /// Compares `self` to another `DataContractInSerializationFormat` instance
    /// and returns the first mismatching field, if any.
    ///
    /// This comparison ignores auto-generated fields and is only sensitive to
    /// significant differences in contract content. For V0 formats, any difference
    /// results in a generic mismatch. For differing format versions (V0 vs V1),
    /// a `FormatVersionMismatch` is returned.
    ///
    /// # Returns
    ///
    /// - `None` if the contracts are equal according to the relevant fields.
    /// - `Some(DataContractMismatch)` indicating the first field where they differ.
    pub fn first_mismatch(&self, other: &Self) -> Option<DataContractMismatch> {
        match (self, other) {
            (
                DataContractInSerializationFormat::V0(v0_self),
                DataContractInSerializationFormat::V0(v0_other),
            ) => {
                if v0_self != v0_other {
                    Some(DataContractMismatch::V0Mismatch)
                } else {
                    None
                }
            }
            (
                DataContractInSerializationFormat::V1(v1_self),
                DataContractInSerializationFormat::V1(v1_other),
            ) => {
                if v1_self.id != v1_other.id {
                    Some(DataContractMismatch::Id)
                } else if v1_self.config != v1_other.config {
                    Some(DataContractMismatch::Config)
                } else if v1_self.version != v1_other.version {
                    Some(DataContractMismatch::Version)
                } else if v1_self.owner_id != v1_other.owner_id {
                    Some(DataContractMismatch::OwnerId)
                } else if v1_self.schema_defs != v1_other.schema_defs {
                    Some(DataContractMismatch::SchemaDefs)
                } else if v1_self.document_schemas != v1_other.document_schemas {
                    Some(DataContractMismatch::DocumentSchemas)
                } else if v1_self.groups != v1_other.groups {
                    Some(DataContractMismatch::Groups)
                } else if v1_self.tokens != v1_other.tokens {
                    Some(DataContractMismatch::Tokens)
                } else if v1_self.keywords.len() != v1_other.keywords.len()
                    || v1_self
                        .keywords
                        .iter()
                        .zip(v1_other.keywords.iter())
                        .any(|(a, b)| a.to_lowercase() != b.to_lowercase())
                {
                    Some(DataContractMismatch::Keywords)
                } else if v1_self.description != v1_other.description {
                    Some(DataContractMismatch::Description)
                } else {
                    None
                }
            }
            _ => Some(DataContractMismatch::FormatVersionMismatch),
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
                let v0_format: DataContractInSerializationFormatV0 =
                    DataContract::V0(value).into_platform_versioned(platform_version);
                Ok(v0_format.into())
            }
            1 => {
                let v1_format: DataContractInSerializationFormatV1 =
                    DataContract::V0(value).into_platform_versioned(platform_version);
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
                    DataContract::V0(value.to_owned()).into_platform_versioned(platform_version);
                Ok(v0_format.into())
            }
            1 => {
                let v1_format: DataContractInSerializationFormatV1 =
                    DataContract::V0(value.to_owned()).into_platform_versioned(platform_version);
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
                let v0_format: DataContractInSerializationFormatV0 =
                    DataContract::V1(value).into_platform_versioned(platform_version);
                Ok(v0_format.into())
            }
            1 => {
                let v1_format: DataContractInSerializationFormatV1 =
                    DataContract::V1(value).into_platform_versioned(platform_version);
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
                    DataContract::V1(value.to_owned()).into_platform_versioned(platform_version);
                Ok(v0_format.into())
            }
            1 => {
                let v1_format: DataContractInSerializationFormatV1 =
                    DataContract::V1(value.to_owned()).into_platform_versioned(platform_version);
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
                let v0_format: DataContractInSerializationFormatV0 =
                    value.clone().into_platform_versioned(platform_version);
                Ok(v0_format.into())
            }
            1 => {
                let v1_format: DataContractInSerializationFormatV1 =
                    value.clone().into_platform_versioned(platform_version);
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
                let v0_format: DataContractInSerializationFormatV0 =
                    value.into_platform_versioned(platform_version);
                Ok(v0_format.into())
            }
            1 => {
                let v1_format: DataContractInSerializationFormatV1 =
                    value.into_platform_versioned(platform_version);
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
