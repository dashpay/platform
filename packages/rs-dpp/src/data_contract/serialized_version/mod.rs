use super::EMPTY_KEYWORDS;
use crate::data_contract::associated_token::token_configuration::TokenConfiguration;
use crate::data_contract::config::DataContractConfig;
use crate::data_contract::group::Group;
use crate::data_contract::serialized_version::v0::DataContractInSerializationFormatV0;
use crate::data_contract::serialized_version::v1::DataContractInSerializationFormatV1;
use crate::data_contract::v0::DataContractV0;
use crate::data_contract::v1::DataContractV1;
use crate::data_contract::{
    DataContract, DefinitionName, DocumentName, GroupContractPosition, TokenContractPosition,
    EMPTY_GROUPS, EMPTY_TOKENS,
};
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use crate::validation::operations::ProtocolValidationOperation;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use derive_more::From;
use platform_value::{Identifier, Value};
use platform_version::{IntoPlatformVersioned, TryFromPlatformVersioned};
use platform_versioning::PlatformVersioned;
#[cfg(feature = "serde-conversion")]
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

#[cfg_attr(
    all(feature = "json-conversion", feature = "serde-conversion"),
    derive(JsonConvertible)
)]
#[cfg_attr(
    all(feature = "value-conversion", feature = "serde-conversion"),
    derive(ValueConvertible)
)]
#[derive(Debug, Clone, Encode, Decode, PartialEq, PlatformVersioned, From)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(tag = "$formatVersion")
)]
pub enum DataContractInSerializationFormat {
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(DataContractInSerializationFormatV0),
    #[cfg_attr(feature = "serde-conversion", serde(rename = "1"))]
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

    pub fn document_schemas_mut(&mut self) -> &mut BTreeMap<DocumentName, Value> {
        match self {
            DataContractInSerializationFormat::V0(v0) => &mut v0.document_schemas,
            DataContractInSerializationFormat::V1(v1) => &mut v1.document_schemas,
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

    /// Returns the config for the data contract.
    pub fn config(&self) -> &DataContractConfig {
        match self {
            DataContractInSerializationFormat::V0(v0) => &v0.config,
            DataContractInSerializationFormat::V1(v1) => &v1.config,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::config::v0::DataContractConfigV0;
    use crate::data_contract::config::v1::DataContractConfigV1;
    use crate::data_contract::group::v0::GroupV0;
    use crate::data_contract::serialized_version::v0::DataContractInSerializationFormatV0;
    use crate::data_contract::serialized_version::v1::DataContractInSerializationFormatV1;
    use platform_value::Identifier;
    use std::collections::BTreeMap;

    /// Helper to create a default V0 serialization format.
    fn make_v0() -> DataContractInSerializationFormatV0 {
        DataContractInSerializationFormatV0 {
            id: Identifier::default(),
            config: DataContractConfig::V0(DataContractConfigV0::default()),
            version: 1,
            owner_id: Identifier::default(),
            schema_defs: None,
            document_schemas: BTreeMap::new(),
        }
    }

    /// Helper to create a default V1 serialization format.
    fn make_v1() -> DataContractInSerializationFormatV1 {
        DataContractInSerializationFormatV1 {
            id: Identifier::default(),
            config: DataContractConfig::V1(DataContractConfigV1::default()),
            version: 1,
            owner_id: Identifier::default(),
            schema_defs: None,
            document_schemas: BTreeMap::new(),
            created_at: None,
            updated_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            created_at_epoch: None,
            updated_at_epoch: None,
            groups: BTreeMap::new(),
            tokens: BTreeMap::new(),
            keywords: vec![],
            description: None,
        }
    }

    // -----------------------------------------------------------------------
    // first_mismatch: V0-V0
    // -----------------------------------------------------------------------

    #[test]
    fn first_mismatch_v0_v0_identical_returns_none() {
        let a = DataContractInSerializationFormat::V0(make_v0());
        let b = DataContractInSerializationFormat::V0(make_v0());
        assert_eq!(a.first_mismatch(&b), None);
    }

    #[test]
    fn first_mismatch_v0_v0_different_id() {
        let mut v0_b = make_v0();
        v0_b.id = Identifier::from([1u8; 32]);
        let a = DataContractInSerializationFormat::V0(make_v0());
        let b = DataContractInSerializationFormat::V0(v0_b);
        assert_eq!(a.first_mismatch(&b), Some(DataContractMismatch::V0Mismatch));
    }

    #[test]
    fn first_mismatch_v0_v0_different_config() {
        let mut v0_b = make_v0();
        let mut cfg = DataContractConfigV0::default();
        cfg.readonly = !cfg.readonly;
        v0_b.config = DataContractConfig::V0(cfg);
        let a = DataContractInSerializationFormat::V0(make_v0());
        let b = DataContractInSerializationFormat::V0(v0_b);
        assert_eq!(a.first_mismatch(&b), Some(DataContractMismatch::V0Mismatch));
    }

    #[test]
    fn first_mismatch_v0_v0_different_version() {
        let mut v0_b = make_v0();
        v0_b.version = 99;
        let a = DataContractInSerializationFormat::V0(make_v0());
        let b = DataContractInSerializationFormat::V0(v0_b);
        assert_eq!(a.first_mismatch(&b), Some(DataContractMismatch::V0Mismatch));
    }

    #[test]
    fn first_mismatch_v0_v0_different_owner_id() {
        let mut v0_b = make_v0();
        v0_b.owner_id = Identifier::from([2u8; 32]);
        let a = DataContractInSerializationFormat::V0(make_v0());
        let b = DataContractInSerializationFormat::V0(v0_b);
        assert_eq!(a.first_mismatch(&b), Some(DataContractMismatch::V0Mismatch));
    }

    #[test]
    fn first_mismatch_v0_v0_different_document_schemas() {
        let mut v0_b = make_v0();
        v0_b.document_schemas
            .insert("doc".to_string(), Value::Bool(true));
        let a = DataContractInSerializationFormat::V0(make_v0());
        let b = DataContractInSerializationFormat::V0(v0_b);
        assert_eq!(a.first_mismatch(&b), Some(DataContractMismatch::V0Mismatch));
    }

    // -----------------------------------------------------------------------
    // first_mismatch: format mismatch (V0 vs V1)
    // -----------------------------------------------------------------------

    #[test]
    fn first_mismatch_v0_v1_returns_format_version_mismatch() {
        let a = DataContractInSerializationFormat::V0(make_v0());
        let b = DataContractInSerializationFormat::V1(make_v1());
        assert_eq!(
            a.first_mismatch(&b),
            Some(DataContractMismatch::FormatVersionMismatch)
        );
    }

    #[test]
    fn first_mismatch_v1_v0_returns_format_version_mismatch() {
        let a = DataContractInSerializationFormat::V1(make_v1());
        let b = DataContractInSerializationFormat::V0(make_v0());
        assert_eq!(
            a.first_mismatch(&b),
            Some(DataContractMismatch::FormatVersionMismatch)
        );
    }

    // -----------------------------------------------------------------------
    // first_mismatch: V1-V1 identical
    // -----------------------------------------------------------------------

    #[test]
    fn first_mismatch_v1_v1_identical_returns_none() {
        let a = DataContractInSerializationFormat::V1(make_v1());
        let b = DataContractInSerializationFormat::V1(make_v1());
        assert_eq!(a.first_mismatch(&b), None);
    }

    // -----------------------------------------------------------------------
    // first_mismatch: V1-V1 field-by-field mismatches
    // -----------------------------------------------------------------------

    #[test]
    fn first_mismatch_v1_v1_different_id() {
        let mut v1_b = make_v1();
        v1_b.id = Identifier::from([1u8; 32]);
        let a = DataContractInSerializationFormat::V1(make_v1());
        let b = DataContractInSerializationFormat::V1(v1_b);
        assert_eq!(a.first_mismatch(&b), Some(DataContractMismatch::Id));
    }

    #[test]
    fn first_mismatch_v1_v1_different_config() {
        let mut v1_b = make_v1();
        let mut cfg = DataContractConfigV1::default();
        cfg.readonly = !cfg.readonly;
        v1_b.config = DataContractConfig::V1(cfg);
        let a = DataContractInSerializationFormat::V1(make_v1());
        let b = DataContractInSerializationFormat::V1(v1_b);
        assert_eq!(a.first_mismatch(&b), Some(DataContractMismatch::Config));
    }

    #[test]
    fn first_mismatch_v1_v1_different_version() {
        let mut v1_b = make_v1();
        v1_b.version = 42;
        let a = DataContractInSerializationFormat::V1(make_v1());
        let b = DataContractInSerializationFormat::V1(v1_b);
        assert_eq!(a.first_mismatch(&b), Some(DataContractMismatch::Version));
    }

    #[test]
    fn first_mismatch_v1_v1_different_owner_id() {
        let mut v1_b = make_v1();
        v1_b.owner_id = Identifier::from([3u8; 32]);
        let a = DataContractInSerializationFormat::V1(make_v1());
        let b = DataContractInSerializationFormat::V1(v1_b);
        assert_eq!(a.first_mismatch(&b), Some(DataContractMismatch::OwnerId));
    }

    #[test]
    fn first_mismatch_v1_v1_different_schema_defs() {
        let mut v1_b = make_v1();
        let mut defs = BTreeMap::new();
        defs.insert("someDef".to_string(), Value::Bool(true));
        v1_b.schema_defs = Some(defs);
        let a = DataContractInSerializationFormat::V1(make_v1());
        let b = DataContractInSerializationFormat::V1(v1_b);
        assert_eq!(a.first_mismatch(&b), Some(DataContractMismatch::SchemaDefs));
    }

    #[test]
    fn first_mismatch_v1_v1_different_document_schemas() {
        let mut v1_b = make_v1();
        v1_b.document_schemas
            .insert("doc".to_string(), Value::U64(1));
        let a = DataContractInSerializationFormat::V1(make_v1());
        let b = DataContractInSerializationFormat::V1(v1_b);
        assert_eq!(
            a.first_mismatch(&b),
            Some(DataContractMismatch::DocumentSchemas)
        );
    }

    #[test]
    fn first_mismatch_v1_v1_different_groups() {
        let mut v1_b = make_v1();
        v1_b.groups.insert(
            0,
            Group::V0(GroupV0 {
                members: Default::default(),
                required_power: 1,
            }),
        );
        let a = DataContractInSerializationFormat::V1(make_v1());
        let b = DataContractInSerializationFormat::V1(v1_b);
        assert_eq!(a.first_mismatch(&b), Some(DataContractMismatch::Groups));
    }

    #[test]
    fn first_mismatch_v1_v1_different_tokens() {
        let mut v1_b = make_v1();
        v1_b.tokens.insert(
            0,
            TokenConfiguration::V0(
                crate::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0::default_most_restrictive(),
            ),
        );
        let a = DataContractInSerializationFormat::V1(make_v1());
        let b = DataContractInSerializationFormat::V1(v1_b);
        assert_eq!(a.first_mismatch(&b), Some(DataContractMismatch::Tokens));
    }

    #[test]
    fn first_mismatch_v1_v1_different_keywords() {
        let mut v1_b = make_v1();
        v1_b.keywords = vec!["test".to_string()];
        let a = DataContractInSerializationFormat::V1(make_v1());
        let b = DataContractInSerializationFormat::V1(v1_b);
        assert_eq!(a.first_mismatch(&b), Some(DataContractMismatch::Keywords));
    }

    #[test]
    fn first_mismatch_v1_v1_keywords_case_insensitive_match() {
        let mut v1_a = make_v1();
        v1_a.keywords = vec!["Test".to_string()];
        let mut v1_b = make_v1();
        v1_b.keywords = vec!["test".to_string()];
        let a = DataContractInSerializationFormat::V1(v1_a);
        let b = DataContractInSerializationFormat::V1(v1_b);
        // The comparison uses to_lowercase, so "Test" and "test" should match
        assert_eq!(a.first_mismatch(&b), None);
    }

    #[test]
    fn first_mismatch_v1_v1_keywords_different_length() {
        let mut v1_a = make_v1();
        v1_a.keywords = vec!["a".to_string()];
        let mut v1_b = make_v1();
        v1_b.keywords = vec!["a".to_string(), "b".to_string()];
        let a = DataContractInSerializationFormat::V1(v1_a);
        let b = DataContractInSerializationFormat::V1(v1_b);
        assert_eq!(a.first_mismatch(&b), Some(DataContractMismatch::Keywords));
    }

    #[test]
    fn first_mismatch_v1_v1_different_description() {
        let mut v1_b = make_v1();
        v1_b.description = Some("a description".to_string());
        let a = DataContractInSerializationFormat::V1(make_v1());
        let b = DataContractInSerializationFormat::V1(v1_b);
        assert_eq!(
            a.first_mismatch(&b),
            Some(DataContractMismatch::Description)
        );
    }

    // -----------------------------------------------------------------------
    // first_mismatch: priority ordering in V1 (id detected before config, etc.)
    // -----------------------------------------------------------------------

    #[test]
    fn first_mismatch_v1_v1_id_takes_priority_over_config() {
        let mut v1_b = make_v1();
        v1_b.id = Identifier::from([5u8; 32]);
        let mut cfg = DataContractConfigV1::default();
        cfg.readonly = !cfg.readonly;
        v1_b.config = DataContractConfig::V1(cfg);
        let a = DataContractInSerializationFormat::V1(make_v1());
        let b = DataContractInSerializationFormat::V1(v1_b);
        // Id is checked before config
        assert_eq!(a.first_mismatch(&b), Some(DataContractMismatch::Id));
    }

    // -----------------------------------------------------------------------
    // DataContractMismatch Display
    // -----------------------------------------------------------------------

    #[test]
    fn data_contract_mismatch_display() {
        assert_eq!(format!("{}", DataContractMismatch::Id), "ID fields differ");
        assert_eq!(
            format!("{}", DataContractMismatch::FormatVersionMismatch),
            "Serialization format versions differ (e.g., V0 vs V1)"
        );
        assert_eq!(
            format!("{}", DataContractMismatch::V0Mismatch),
            "V0 versions differ"
        );
        assert_eq!(format!("{}", DataContractMismatch::Tokens), "Tokens differ");
        assert_eq!(
            format!("{}", DataContractMismatch::Keywords),
            "Keywords differ"
        );
        assert_eq!(
            format!("{}", DataContractMismatch::Description),
            "Description fields differ"
        );
    }

    // -----------------------------------------------------------------------
    // Accessor methods
    // -----------------------------------------------------------------------

    #[test]
    fn accessor_id_v0() {
        let v0 = make_v0();
        let expected_id = v0.id;
        let format = DataContractInSerializationFormat::V0(v0);
        assert_eq!(format.id(), expected_id);
    }

    #[test]
    fn accessor_id_v1() {
        let v1 = make_v1();
        let expected_id = v1.id;
        let format = DataContractInSerializationFormat::V1(v1);
        assert_eq!(format.id(), expected_id);
    }

    #[test]
    fn accessor_owner_id_v0() {
        let mut v0 = make_v0();
        v0.owner_id = Identifier::from([7u8; 32]);
        let expected = v0.owner_id;
        let format = DataContractInSerializationFormat::V0(v0);
        assert_eq!(format.owner_id(), expected);
    }

    #[test]
    fn accessor_version_v0() {
        let mut v0 = make_v0();
        v0.version = 10;
        let format = DataContractInSerializationFormat::V0(v0);
        assert_eq!(format.version(), 10);
    }

    #[test]
    fn accessor_version_v1() {
        let mut v1 = make_v1();
        v1.version = 20;
        let format = DataContractInSerializationFormat::V1(v1);
        assert_eq!(format.version(), 20);
    }

    #[test]
    fn accessor_groups_v0_returns_empty() {
        let format = DataContractInSerializationFormat::V0(make_v0());
        assert!(format.groups().is_empty());
    }

    #[test]
    fn accessor_tokens_v0_returns_empty() {
        let format = DataContractInSerializationFormat::V0(make_v0());
        assert!(format.tokens().is_empty());
    }

    #[test]
    fn accessor_keywords_v0_returns_empty() {
        let format = DataContractInSerializationFormat::V0(make_v0());
        assert!(format.keywords().is_empty());
    }

    #[test]
    fn accessor_description_v0_returns_none() {
        let format = DataContractInSerializationFormat::V0(make_v0());
        assert_eq!(format.description(), &None);
    }

    #[test]
    fn accessor_keywords_v1() {
        let mut v1 = make_v1();
        v1.keywords = vec!["hello".to_string()];
        let format = DataContractInSerializationFormat::V1(v1);
        assert_eq!(format.keywords(), &vec!["hello".to_string()]);
    }

    #[test]
    fn accessor_description_v1_some() {
        let mut v1 = make_v1();
        v1.description = Some("desc".to_string());
        let format = DataContractInSerializationFormat::V1(v1);
        assert_eq!(format.description(), &Some("desc".to_string()));
    }

    #[test]
    fn accessor_document_schemas_v0() {
        let mut v0 = make_v0();
        v0.document_schemas
            .insert("note".to_string(), Value::Bool(true));
        let format = DataContractInSerializationFormat::V0(v0);
        assert_eq!(format.document_schemas().len(), 1);
        assert!(format.document_schemas().contains_key("note"));
    }

    #[test]
    fn accessor_schema_defs_v0_none() {
        let format = DataContractInSerializationFormat::V0(make_v0());
        assert!(format.schema_defs().is_none());
    }

    #[test]
    fn accessor_schema_defs_v1_some() {
        let mut v1 = make_v1();
        let mut defs = BTreeMap::new();
        defs.insert("def1".to_string(), Value::Null);
        v1.schema_defs = Some(defs);
        let format = DataContractInSerializationFormat::V1(v1);
        assert!(format.schema_defs().is_some());
        assert!(format.schema_defs().unwrap().contains_key("def1"));
    }

    // -----------------------------------------------------------------------
    // TryFromPlatformVersioned: DataContractV0 -> DataContractInSerializationFormat
    // -----------------------------------------------------------------------

    #[test]
    fn try_from_platform_versioned_data_contract_v0_version_0() {
        let platform_version = PlatformVersion::first();
        // V1 contract versions use default_current_version: 0
        let v0 = DataContractV0 {
            id: Identifier::from([10u8; 32]),
            config: DataContractConfig::V0(DataContractConfigV0::default()),
            version: 1,
            owner_id: Identifier::from([20u8; 32]),
            schema_defs: None,
            document_types: BTreeMap::new(),
            metadata: None,
        };
        let result = DataContractInSerializationFormat::try_from_platform_versioned(
            v0.clone(),
            platform_version,
        );
        assert!(result.is_ok());
        let format = result.unwrap();
        assert!(matches!(format, DataContractInSerializationFormat::V0(_)));
        assert_eq!(format.id(), Identifier::from([10u8; 32]));
        assert_eq!(format.owner_id(), Identifier::from([20u8; 32]));
    }

    #[test]
    fn try_from_platform_versioned_data_contract_v0_ref_version_0() {
        let platform_version = PlatformVersion::first();
        let v0 = DataContractV0 {
            id: Identifier::from([11u8; 32]),
            config: DataContractConfig::V0(DataContractConfigV0::default()),
            version: 2,
            owner_id: Identifier::from([22u8; 32]),
            schema_defs: None,
            document_types: BTreeMap::new(),
            metadata: None,
        };
        let result =
            DataContractInSerializationFormat::try_from_platform_versioned(&v0, platform_version);
        assert!(result.is_ok());
        let format = result.unwrap();
        assert!(matches!(format, DataContractInSerializationFormat::V0(_)));
        assert_eq!(format.version(), 2);
    }

    #[test]
    fn try_from_platform_versioned_data_contract_v0_version_1() {
        let platform_version = PlatformVersion::latest();
        // Latest uses default_current_version: 1
        let v0 = DataContractV0 {
            id: Identifier::from([10u8; 32]),
            config: DataContractConfig::V0(DataContractConfigV0::default()),
            version: 1,
            owner_id: Identifier::from([20u8; 32]),
            schema_defs: None,
            document_types: BTreeMap::new(),
            metadata: None,
        };
        let result = DataContractInSerializationFormat::try_from_platform_versioned(
            v0.clone(),
            platform_version,
        );
        assert!(result.is_ok());
        let format = result.unwrap();
        assert!(matches!(format, DataContractInSerializationFormat::V1(_)));
    }

    // -----------------------------------------------------------------------
    // TryFromPlatformVersioned: DataContractV1 -> DataContractInSerializationFormat
    // -----------------------------------------------------------------------

    #[test]
    fn try_from_platform_versioned_data_contract_v1_version_0() {
        let platform_version = PlatformVersion::first();
        let v1 = DataContractV1 {
            id: Identifier::from([10u8; 32]),
            config: DataContractConfig::V0(DataContractConfigV0::default()),
            version: 1,
            owner_id: Identifier::from([20u8; 32]),
            schema_defs: None,
            document_types: BTreeMap::new(),
            created_at: None,
            updated_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            created_at_epoch: None,
            updated_at_epoch: None,
            groups: BTreeMap::new(),
            tokens: BTreeMap::new(),
            keywords: vec![],
            description: None,
        };
        let result = DataContractInSerializationFormat::try_from_platform_versioned(
            v1.clone(),
            platform_version,
        );
        assert!(result.is_ok());
        let format = result.unwrap();
        assert!(matches!(format, DataContractInSerializationFormat::V0(_)));
    }

    #[test]
    fn try_from_platform_versioned_data_contract_v1_version_1() {
        let platform_version = PlatformVersion::latest();
        let v1 = DataContractV1 {
            id: Identifier::from([10u8; 32]),
            config: DataContractConfig::V1(DataContractConfigV1::default()),
            version: 1,
            owner_id: Identifier::from([20u8; 32]),
            schema_defs: None,
            document_types: BTreeMap::new(),
            created_at: None,
            updated_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            created_at_epoch: None,
            updated_at_epoch: None,
            groups: BTreeMap::new(),
            tokens: BTreeMap::new(),
            keywords: vec![],
            description: None,
        };
        let result = DataContractInSerializationFormat::try_from_platform_versioned(
            v1.clone(),
            platform_version,
        );
        assert!(result.is_ok());
        let format = result.unwrap();
        assert!(matches!(format, DataContractInSerializationFormat::V1(_)));
    }

    #[test]
    fn try_from_platform_versioned_data_contract_v1_ref_version_1() {
        let platform_version = PlatformVersion::latest();
        let v1 = DataContractV1 {
            id: Identifier::from([10u8; 32]),
            config: DataContractConfig::V1(DataContractConfigV1::default()),
            version: 3,
            owner_id: Identifier::from([20u8; 32]),
            schema_defs: None,
            document_types: BTreeMap::new(),
            created_at: None,
            updated_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            created_at_epoch: None,
            updated_at_epoch: None,
            groups: BTreeMap::new(),
            tokens: BTreeMap::new(),
            keywords: vec![],
            description: None,
        };
        let result =
            DataContractInSerializationFormat::try_from_platform_versioned(&v1, platform_version);
        assert!(result.is_ok());
        let format = result.unwrap();
        assert!(matches!(format, DataContractInSerializationFormat::V1(_)));
        assert_eq!(format.version(), 3);
    }

    // -----------------------------------------------------------------------
    // TryFromPlatformVersioned: DataContract -> DataContractInSerializationFormat
    // -----------------------------------------------------------------------

    #[test]
    fn try_from_platform_versioned_data_contract_ref_version_0() {
        let platform_version = PlatformVersion::first();
        let contract = DataContract::V0(DataContractV0 {
            id: Identifier::from([10u8; 32]),
            config: DataContractConfig::V0(DataContractConfigV0::default()),
            version: 1,
            owner_id: Identifier::from([20u8; 32]),
            schema_defs: None,
            document_types: BTreeMap::new(),
            metadata: None,
        });
        let result = DataContractInSerializationFormat::try_from_platform_versioned(
            &contract,
            platform_version,
        );
        assert!(result.is_ok());
        assert!(matches!(
            result.unwrap(),
            DataContractInSerializationFormat::V0(_)
        ));
    }

    #[test]
    fn try_from_platform_versioned_data_contract_owned_version_1() {
        let platform_version = PlatformVersion::latest();
        let contract = DataContract::V0(DataContractV0 {
            id: Identifier::from([10u8; 32]),
            config: DataContractConfig::V0(DataContractConfigV0::default()),
            version: 1,
            owner_id: Identifier::from([20u8; 32]),
            schema_defs: None,
            document_types: BTreeMap::new(),
            metadata: None,
        });
        let result = DataContractInSerializationFormat::try_from_platform_versioned(
            contract,
            platform_version,
        );
        assert!(result.is_ok());
        assert!(matches!(
            result.unwrap(),
            DataContractInSerializationFormat::V1(_)
        ));
    }

    // -----------------------------------------------------------------------
    // Verify serialization version routing
    // -----------------------------------------------------------------------

    #[test]
    fn first_platform_version_uses_serialization_version_0() {
        let pv = PlatformVersion::first();
        assert_eq!(
            pv.dpp
                .contract_versions
                .contract_serialization_version
                .default_current_version,
            0
        );
    }

    #[test]
    fn latest_platform_version_uses_serialization_version_1() {
        let pv = PlatformVersion::latest();
        assert_eq!(
            pv.dpp
                .contract_versions
                .contract_serialization_version
                .default_current_version,
            1
        );
    }

    #[test]
    fn first_platform_version_uses_contract_structure_0() {
        let pv = PlatformVersion::first();
        assert_eq!(pv.dpp.contract_versions.contract_structure_version, 0);
    }

    #[test]
    fn latest_platform_version_uses_contract_structure_1() {
        let pv = PlatformVersion::latest();
        assert_eq!(pv.dpp.contract_versions.contract_structure_version, 1);
    }
}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
mod json_convertible_tests {
    use super::*;
    use crate::data_contract::config::v0::DataContractConfigV0;
    use crate::data_contract::config::DataContractConfig;
    use crate::data_contract::serialized_version::v0::DataContractInSerializationFormatV0;
    use platform_value::Identifier;
    use std::collections::BTreeMap;

    fn fixture() -> DataContractInSerializationFormat {
        DataContractInSerializationFormat::V0(DataContractInSerializationFormatV0 {
            id: Identifier::new([0xa1; 32]),
            config: DataContractConfig::V0(DataContractConfigV0::default()),
            version: 1,
            owner_id: Identifier::new([0xb2; 32]),
            schema_defs: None,
            document_schemas: BTreeMap::new(),
        })
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        use serde_json::json;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // Tier 3 envelope-only: `DataContractInSerializationFormat` embeds a
        // versioned `DataContractConfig` and arbitrary `document_schemas` /
        // `schema_defs` Values. The full inline expansion is verified for the
        // `DataContractConfig` in its own module. We still pin the top-level
        // envelope keys + their types here so that any silent drop / rename /
        // re-keying at this layer would fail the test.
        assert_eq!(json["$formatVersion"], "0");
        assert_eq!(
            json["id"],
            json!("Bswb3UyeD1pUTaGiE6WvqwFpJZsQSEY1xhJePCDTHdvp")
        );
        assert_eq!(
            json["ownerId"],
            json!("D2ZcUbtpG5sKq7XLeB4YnpNnTGSptKCxTddoNeydzJQq")
        );
        assert_eq!(json["version"], json!(1));
        assert_eq!(json["schemaDefs"], json!(null));
        assert_eq!(json["documentSchemas"], json!({}));
        assert!(json.get("config").is_some(), "config envelope present");
        assert_eq!(json["config"]["$formatVersion"], "0");
        let recovered = DataContractInSerializationFormat::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        use platform_value::Value;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        // Tier 3 envelope-only: see JSON test above. Keys remain `Identifier` /
        // `Map` / typed integers in non-HR mode (no base58 stringification).
        let map = match &value {
            Value::Map(m) => m,
            other => panic!("expected Value::Map, got {:?}", other),
        };
        let get = |k: &str| -> &Value {
            map.iter()
                .find(|(key, _)| matches!(key, Value::Text(t) if t == k))
                .map(|(_, v)| v)
                .unwrap_or_else(|| panic!("missing key {k}"))
        };
        assert_eq!(get("$formatVersion"), &Value::Text("0".to_string()));
        assert_eq!(get("id"), &Value::Identifier([0xa1; 32]));
        assert_eq!(get("ownerId"), &Value::Identifier([0xb2; 32]));
        assert_eq!(get("version"), &Value::U32(1));
        assert_eq!(get("schemaDefs"), &Value::Null);
        // documentSchemas: empty Map
        assert!(matches!(get("documentSchemas"), Value::Map(m) if m.is_empty()));
        // config: nested Map with its own $formatVersion="0"
        assert!(matches!(get("config"), Value::Map(_)));
        let recovered = DataContractInSerializationFormat::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }

    #[test]
    fn json_preserves_format_version_tag() {
        use crate::serialization::JsonConvertible;
        let json = fixture().to_json().expect("to_json");
        assert_eq!(json["$formatVersion"], "0");
    }
}
