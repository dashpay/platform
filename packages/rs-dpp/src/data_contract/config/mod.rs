mod fields;
mod methods;
pub mod v0;
pub mod v1;

use crate::data_contract::config::v1::{
    DataContractConfigGettersV1, DataContractConfigSettersV1, DataContractConfigV1,
};
use crate::data_contract::storage_requirements::keys_for_document_type::StorageKeyRequirements;
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use derive_more::From;
pub use fields::*;
use platform_value::Value;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use v0::{DataContractConfigGettersV0, DataContractConfigSettersV0, DataContractConfigV0};

#[cfg_attr(feature = "json-conversion", derive(JsonConvertible))]
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
#[derive(Serialize, Deserialize, Encode, Decode, Debug, Clone, Copy, PartialEq, Eq, From)]
#[serde(tag = "$formatVersion")]
pub enum DataContractConfig {
    #[serde(rename = "0")]
    V0(DataContractConfigV0),
    #[serde(rename = "1")]
    V1(DataContractConfigV1),
}

impl DataContractConfig {
    pub fn version(&self) -> u16 {
        match self {
            DataContractConfig::V0(_) => 0,
            DataContractConfig::V1(_) => 1,
        }
    }

    pub fn default_for_version(
        platform_version: &PlatformVersion,
    ) -> Result<DataContractConfig, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .config
            .default_current_version
        {
            0 => Ok(DataContractConfigV0::default().into()),
            1 => Ok(DataContractConfigV1::default().into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContractConfig::default_for_version".to_string(),
                known_versions: vec![0, 1],
                received: version,
            }),
        }
    }

    /// Adjusts the current `DataContractConfig` to be valid for the provided platform version.
    ///
    /// This replaces the internal version with the `default_current_version` defined in the platform version's
    /// feature bounds for contract config.
    pub fn config_valid_for_platform_version(
        self,
        platform_version: &PlatformVersion,
    ) -> DataContractConfig {
        match self {
            DataContractConfig::V0(v0) => DataContractConfig::V0(v0),
            DataContractConfig::V1(v1) => {
                if platform_version.dpp.contract_versions.config.max_version == 0 {
                    DataContractConfig::V0(v1.into())
                } else {
                    self
                }
            }
        }
    }

    /// **KEEP-AS-EXCEPTION** in the JSON/Value canonical-trait migration —
    /// this is a context-aware constructor, not a parallel conversion path:
    /// it dispatches the config variant on `platform_version` (the input map
    /// carries no `$formatVersion` tag in the contract-creation flow), so
    /// canonical `ValueConvertible::from_object` cannot replace it.
    pub fn from_value(
        value: Value,
        platform_version: &PlatformVersion,
    ) -> Result<DataContractConfig, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .config
            .default_current_version
        {
            0 => {
                let config: DataContractConfigV0 = platform_value::from_value(value)?;
                Ok(config.into())
            }
            1 => {
                let config: DataContractConfigV1 = platform_value::from_value(value)?;
                Ok(config.into())
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContractConfig::from_value".to_string(),
                known_versions: vec![0, 1],
                received: version,
            }),
        }
    }

    // TODO: Remove, it's not using
    /// Retrieve contract configuration properties.
    ///
    /// This method takes a BTreeMap representing a contract and retrieves
    /// the configuration properties based on the values found in the map.
    ///
    /// The process of retrieving contract configuration properties is versioned,
    /// and the version is determined by the platform version parameter.
    /// If the version is not supported, an error is returned.
    ///
    /// # Parameters
    ///
    /// * `contract`: BTreeMap representing the contract.
    /// * `platform_version`: The platform version being used.
    ///
    /// # Returns
    ///
    /// * `Result<ContractConfig, ProtocolError>`: On success, a ContractConfig.
    ///   On failure, a ProtocolError.
    pub(in crate::data_contract) fn get_contract_configuration_properties(
        contract: &BTreeMap<String, Value>,
        platform_version: &PlatformVersion,
    ) -> Result<DataContractConfig, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .config
            .default_current_version
        {
            0 => Ok(
                DataContractConfigV0::get_contract_configuration_properties_v0(contract)?.into(),
            ),
            1 => Ok(
                DataContractConfigV1::get_contract_configuration_properties_v1(contract)?.into(),
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContractConfig::get_contract_configuration_properties".to_string(),
                known_versions: vec![0, 1],
                received: version,
            }),
        }
    }
}

impl DataContractConfigGettersV0 for DataContractConfig {
    fn can_be_deleted(&self) -> bool {
        match self {
            DataContractConfig::V0(v0) => v0.can_be_deleted,
            DataContractConfig::V1(v1) => v1.can_be_deleted,
        }
    }

    fn readonly(&self) -> bool {
        match self {
            DataContractConfig::V0(v0) => v0.readonly,
            DataContractConfig::V1(v1) => v1.readonly,
        }
    }

    fn keeps_history(&self) -> bool {
        match self {
            DataContractConfig::V0(v0) => v0.keeps_history,
            DataContractConfig::V1(v1) => v1.keeps_history,
        }
    }

    fn documents_keep_history_contract_default(&self) -> bool {
        match self {
            DataContractConfig::V0(v0) => v0.documents_keep_history_contract_default,
            DataContractConfig::V1(v1) => v1.documents_keep_history_contract_default,
        }
    }

    fn documents_mutable_contract_default(&self) -> bool {
        match self {
            DataContractConfig::V0(v0) => v0.documents_mutable_contract_default,
            DataContractConfig::V1(v1) => v1.documents_mutable_contract_default,
        }
    }

    fn documents_can_be_deleted_contract_default(&self) -> bool {
        match self {
            DataContractConfig::V0(v0) => v0.documents_can_be_deleted_contract_default,
            DataContractConfig::V1(v1) => v1.documents_can_be_deleted_contract_default,
        }
    }

    /// Encryption key storage requirements
    fn requires_identity_encryption_bounded_key(&self) -> Option<StorageKeyRequirements> {
        match self {
            DataContractConfig::V0(v0) => v0.requires_identity_encryption_bounded_key,
            DataContractConfig::V1(v1) => v1.requires_identity_encryption_bounded_key,
        }
    }

    /// Decryption key storage requirements
    fn requires_identity_decryption_bounded_key(&self) -> Option<StorageKeyRequirements> {
        match self {
            DataContractConfig::V0(v0) => v0.requires_identity_decryption_bounded_key,
            DataContractConfig::V1(v1) => v1.requires_identity_decryption_bounded_key,
        }
    }
}

impl DataContractConfigSettersV0 for DataContractConfig {
    fn set_can_be_deleted(&mut self, value: bool) {
        match self {
            DataContractConfig::V0(v0) => v0.can_be_deleted = value,
            DataContractConfig::V1(v1) => v1.can_be_deleted = value,
        }
    }

    fn set_readonly(&mut self, value: bool) {
        match self {
            DataContractConfig::V0(v0) => v0.readonly = value,
            DataContractConfig::V1(v1) => v1.readonly = value,
        }
    }

    fn set_keeps_history(&mut self, value: bool) {
        match self {
            DataContractConfig::V0(v0) => v0.keeps_history = value,
            DataContractConfig::V1(v1) => v1.keeps_history = value,
        }
    }

    fn set_documents_keep_history_contract_default(&mut self, value: bool) {
        match self {
            DataContractConfig::V0(v0) => v0.documents_keep_history_contract_default = value,
            DataContractConfig::V1(v1) => v1.documents_keep_history_contract_default = value,
        }
    }

    fn set_documents_can_be_deleted_contract_default(&mut self, value: bool) {
        match self {
            DataContractConfig::V0(v0) => v0.documents_can_be_deleted_contract_default = value,
            DataContractConfig::V1(v1) => v1.documents_can_be_deleted_contract_default = value,
        }
    }

    fn set_documents_mutable_contract_default(&mut self, value: bool) {
        match self {
            DataContractConfig::V0(v0) => v0.documents_mutable_contract_default = value,
            DataContractConfig::V1(v1) => v1.documents_mutable_contract_default = value,
        }
    }

    fn set_requires_identity_encryption_bounded_key(
        &mut self,
        value: Option<StorageKeyRequirements>,
    ) {
        match self {
            DataContractConfig::V0(v0) => v0.requires_identity_encryption_bounded_key = value,
            DataContractConfig::V1(v1) => v1.requires_identity_encryption_bounded_key = value,
        }
    }

    fn set_requires_identity_decryption_bounded_key(
        &mut self,
        value: Option<StorageKeyRequirements>,
    ) {
        match self {
            DataContractConfig::V0(v0) => v0.requires_identity_decryption_bounded_key = value,
            DataContractConfig::V1(v1) => v1.requires_identity_decryption_bounded_key = value,
        }
    }
}

impl DataContractConfigGettersV1 for DataContractConfig {
    fn sized_integer_types(&self) -> bool {
        match self {
            DataContractConfig::V0(_) => false,
            DataContractConfig::V1(v1) => v1.sized_integer_types,
        }
    }
}

impl DataContractConfigSettersV1 for DataContractConfig {
    fn set_sized_integer_types_enabled(&mut self, enable: bool) {
        match self {
            DataContractConfig::V0(_) => {}
            DataContractConfig::V1(v1) => v1.sized_integer_types = enable,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::config::v0::DataContractConfigV0;
    use crate::data_contract::config::v1::DataContractConfigV1;
    use crate::data_contract::storage_requirements::keys_for_document_type::StorageKeyRequirements;
    use platform_version::version::PlatformVersion;

    mod default_for_version {
        use super::*;

        #[test]
        fn default_for_latest_platform_version() {
            let platform_version = PlatformVersion::latest();
            let config = DataContractConfig::default_for_version(platform_version)
                .expect("should create config for latest version");

            // Latest platform version uses contract config V1
            let expected_version = platform_version
                .dpp
                .contract_versions
                .config
                .default_current_version;

            assert_eq!(config.version(), expected_version);
        }

        #[test]
        fn default_for_first_platform_version() {
            let platform_version = PlatformVersion::first();
            let config = DataContractConfig::default_for_version(platform_version)
                .expect("should create config for first version");

            let expected_version = platform_version
                .dpp
                .contract_versions
                .config
                .default_current_version;

            assert_eq!(config.version(), expected_version);
        }
    }

    mod version_method {
        use super::*;

        #[test]
        fn v0_reports_version_0() {
            let config = DataContractConfig::V0(DataContractConfigV0::default());
            assert_eq!(config.version(), 0);
        }

        #[test]
        fn v1_reports_version_1() {
            let config = DataContractConfig::V1(DataContractConfigV1::default());
            assert_eq!(config.version(), 1);
        }
    }

    mod from_conversions {
        use super::*;

        #[test]
        fn v0_into_config() {
            let v0 = DataContractConfigV0::default();
            let config: DataContractConfig = v0.into();
            assert_eq!(config.version(), 0);
        }

        #[test]
        fn v1_into_config() {
            let v1 = DataContractConfigV1::default();
            let config: DataContractConfig = v1.into();
            assert_eq!(config.version(), 1);
        }

        #[test]
        fn v1_to_v0_conversion_preserves_fields() {
            let v1 = DataContractConfigV1 {
                can_be_deleted: true,
                readonly: true,
                keeps_history: true,
                documents_keep_history_contract_default: true,
                documents_mutable_contract_default: false,
                documents_can_be_deleted_contract_default: false,
                requires_identity_encryption_bounded_key: None,
                requires_identity_decryption_bounded_key: None,
                sized_integer_types: true,
            };
            let v0: DataContractConfigV0 = v1.into();
            assert!(v0.can_be_deleted);
            assert!(v0.readonly);
            assert!(v0.keeps_history);
            assert!(v0.documents_keep_history_contract_default);
            assert!(!v0.documents_mutable_contract_default);
            assert!(!v0.documents_can_be_deleted_contract_default);
        }
    }

    mod getters_v0 {
        use super::*;

        #[test]
        fn default_v0_getter_values() {
            let config = DataContractConfig::V0(DataContractConfigV0::default());
            assert_eq!(config.can_be_deleted(), DEFAULT_CONTRACT_CAN_BE_DELETED);
            assert_eq!(config.readonly(), !DEFAULT_CONTRACT_MUTABILITY);
            assert_eq!(config.keeps_history(), DEFAULT_CONTRACT_KEEPS_HISTORY);
            assert_eq!(
                config.documents_keep_history_contract_default(),
                DEFAULT_CONTRACT_DOCUMENTS_KEEPS_HISTORY
            );
            assert_eq!(
                config.documents_mutable_contract_default(),
                DEFAULT_CONTRACT_DOCUMENT_MUTABILITY
            );
            assert_eq!(
                config.documents_can_be_deleted_contract_default(),
                DEFAULT_CONTRACT_DOCUMENTS_CAN_BE_DELETED
            );
            assert!(config.requires_identity_encryption_bounded_key().is_none());
            assert!(config.requires_identity_decryption_bounded_key().is_none());
        }

        #[test]
        fn default_v1_getter_values() {
            let config = DataContractConfig::V1(DataContractConfigV1::default());
            assert_eq!(config.can_be_deleted(), DEFAULT_CONTRACT_CAN_BE_DELETED);
            assert_eq!(config.readonly(), !DEFAULT_CONTRACT_MUTABILITY);
            assert_eq!(config.keeps_history(), DEFAULT_CONTRACT_KEEPS_HISTORY);
            assert_eq!(
                config.documents_keep_history_contract_default(),
                DEFAULT_CONTRACT_DOCUMENTS_KEEPS_HISTORY
            );
            assert_eq!(
                config.documents_mutable_contract_default(),
                DEFAULT_CONTRACT_DOCUMENT_MUTABILITY
            );
            assert_eq!(
                config.documents_can_be_deleted_contract_default(),
                DEFAULT_CONTRACT_DOCUMENTS_CAN_BE_DELETED
            );
        }
    }

    mod setters_v0 {
        use super::*;

        #[test]
        fn set_can_be_deleted_on_v0() {
            let mut config = DataContractConfig::V0(DataContractConfigV0::default());
            config.set_can_be_deleted(true);
            assert!(config.can_be_deleted());
            config.set_can_be_deleted(false);
            assert!(!config.can_be_deleted());
        }

        #[test]
        fn set_readonly_on_v1() {
            let mut config = DataContractConfig::V1(DataContractConfigV1::default());
            config.set_readonly(true);
            assert!(config.readonly());
            config.set_readonly(false);
            assert!(!config.readonly());
        }

        #[test]
        fn set_keeps_history() {
            let mut config = DataContractConfig::V0(DataContractConfigV0::default());
            config.set_keeps_history(true);
            assert!(config.keeps_history());
        }

        #[test]
        fn set_documents_keep_history() {
            let mut config = DataContractConfig::V1(DataContractConfigV1::default());
            config.set_documents_keep_history_contract_default(true);
            assert!(config.documents_keep_history_contract_default());
        }

        #[test]
        fn set_documents_mutable() {
            let mut config = DataContractConfig::V0(DataContractConfigV0::default());
            config.set_documents_mutable_contract_default(false);
            assert!(!config.documents_mutable_contract_default());
        }

        #[test]
        fn set_documents_can_be_deleted() {
            let mut config = DataContractConfig::V1(DataContractConfigV1::default());
            config.set_documents_can_be_deleted_contract_default(false);
            assert!(!config.documents_can_be_deleted_contract_default());
        }

        #[test]
        fn set_encryption_key_requirements() {
            let mut config = DataContractConfig::V0(DataContractConfigV0::default());
            config
                .set_requires_identity_encryption_bounded_key(Some(StorageKeyRequirements::Unique));
            assert_eq!(
                config.requires_identity_encryption_bounded_key(),
                Some(StorageKeyRequirements::Unique)
            );
        }

        #[test]
        fn set_decryption_key_requirements() {
            let mut config = DataContractConfig::V1(DataContractConfigV1::default());
            config
                .set_requires_identity_decryption_bounded_key(Some(StorageKeyRequirements::Unique));
            assert_eq!(
                config.requires_identity_decryption_bounded_key(),
                Some(StorageKeyRequirements::Unique)
            );
        }
    }

    mod getters_setters_v1 {
        use super::*;

        #[test]
        fn sized_integer_types_default_v1() {
            let config = DataContractConfig::V1(DataContractConfigV1::default());
            // V1 defaults to sized_integer_types = true
            assert!(config.sized_integer_types());
        }

        #[test]
        fn sized_integer_types_v0_always_false() {
            let config = DataContractConfig::V0(DataContractConfigV0::default());
            assert!(!config.sized_integer_types());
        }

        #[test]
        fn set_sized_integer_types_on_v1() {
            let mut config = DataContractConfig::V1(DataContractConfigV1::default());
            config.set_sized_integer_types_enabled(false);
            assert!(!config.sized_integer_types());
            config.set_sized_integer_types_enabled(true);
            assert!(config.sized_integer_types());
        }

        #[test]
        fn set_sized_integer_types_on_v0_is_noop() {
            let mut config = DataContractConfig::V0(DataContractConfigV0::default());
            config.set_sized_integer_types_enabled(true);
            // V0 does not support sized_integer_types; should remain false
            assert!(!config.sized_integer_types());
        }
    }

    mod config_valid_for_platform_version {
        use super::*;

        #[test]
        fn v0_stays_v0_regardless_of_platform() {
            let config = DataContractConfig::V0(DataContractConfigV0::default());
            let result = config.config_valid_for_platform_version(PlatformVersion::latest());
            assert_eq!(result.version(), 0);
        }

        #[test]
        fn v1_downgraded_to_v0_when_max_version_is_0() {
            let config = DataContractConfig::V1(DataContractConfigV1 {
                can_be_deleted: true,
                readonly: false,
                keeps_history: true,
                documents_keep_history_contract_default: false,
                documents_mutable_contract_default: true,
                documents_can_be_deleted_contract_default: true,
                requires_identity_encryption_bounded_key: None,
                requires_identity_decryption_bounded_key: None,
                sized_integer_types: true,
            });

            // Use first platform version which has config max_version = 0
            let platform_version = PlatformVersion::first();
            if platform_version.dpp.contract_versions.config.max_version == 0 {
                let result = config.config_valid_for_platform_version(platform_version);
                assert_eq!(result.version(), 0);
                // The converted V0 should preserve basic fields
                assert!(result.can_be_deleted());
            }
        }

        #[test]
        fn v1_stays_v1_when_max_version_is_1_or_higher() {
            let config = DataContractConfig::V1(DataContractConfigV1::default());
            let platform_version = PlatformVersion::latest();
            if platform_version.dpp.contract_versions.config.max_version >= 1 {
                let result = config.config_valid_for_platform_version(platform_version);
                assert_eq!(result.version(), 1);
            }
        }
    }

    /// V0's `get_contract_configuration_properties_v0` has a historical
    /// copy-paste quirk: the decryption bounded-key field is parsed from
    /// the `requiresIdentityEncryptionBoundedKey` property (not the matching
    /// DECRYPTION one). This is part of V0 protocol behavior and MUST NOT be
    /// changed — altering it would fork the chain. V1 parses correctly; see
    /// `v1/mod.rs`. These tests lock the V0 behavior in place so the quirk
    /// is not silently "fixed" by a future well-intentioned refactor.
    mod get_contract_configuration_properties_v0_consensus_lock {
        use super::*;
        use crate::data_contract::config::property::{
            REQUIRES_IDENTITY_DECRYPTION_BOUNDED_KEY, REQUIRES_IDENTITY_ENCRYPTION_BOUNDED_KEY,
        };
        use platform_value::Value;
        use std::collections::BTreeMap;

        /// When the ENCRYPTION property is set, V0 applies that value to
        /// BOTH the encryption and decryption fields — because the parser
        /// reads both from the same key.
        #[test]
        fn encryption_property_populates_both_fields() {
            let mut map: BTreeMap<String, Value> = BTreeMap::new();
            map.insert(
                REQUIRES_IDENTITY_ENCRYPTION_BOUNDED_KEY.to_string(),
                Value::U8(StorageKeyRequirements::Unique as u8),
            );

            let config = DataContractConfigV0::get_contract_configuration_properties_v0(&map)
                .expect("should parse V0 config");

            assert_eq!(
                config.requires_identity_encryption_bounded_key,
                Some(StorageKeyRequirements::Unique)
            );
            assert_eq!(
                config.requires_identity_decryption_bounded_key,
                Some(StorageKeyRequirements::Unique),
                "V0 consensus quirk: decryption field is read from the ENCRYPTION key"
            );
        }

        /// When ONLY the DECRYPTION property is set, V0 ignores it entirely
        /// — neither field is populated, because V0 never reads the
        /// DECRYPTION key.
        #[test]
        fn decryption_property_is_ignored_by_v0() {
            let mut map: BTreeMap<String, Value> = BTreeMap::new();
            map.insert(
                REQUIRES_IDENTITY_DECRYPTION_BOUNDED_KEY.to_string(),
                Value::U8(StorageKeyRequirements::MultipleReferenceToLatest as u8),
            );

            let config = DataContractConfigV0::get_contract_configuration_properties_v0(&map)
                .expect("should parse V0 config");

            assert!(
                config.requires_identity_encryption_bounded_key.is_none(),
                "V0 does not read the DECRYPTION property at all"
            );
            assert!(
                config.requires_identity_decryption_bounded_key.is_none(),
                "V0 consensus quirk: decryption field is NOT sourced from the DECRYPTION key"
            );
        }

        /// When BOTH properties are set, the ENCRYPTION value wins for both
        /// fields; the DECRYPTION property is ignored.
        #[test]
        fn encryption_wins_when_both_properties_set() {
            let mut map: BTreeMap<String, Value> = BTreeMap::new();
            map.insert(
                REQUIRES_IDENTITY_ENCRYPTION_BOUNDED_KEY.to_string(),
                Value::U8(StorageKeyRequirements::Unique as u8),
            );
            map.insert(
                REQUIRES_IDENTITY_DECRYPTION_BOUNDED_KEY.to_string(),
                Value::U8(StorageKeyRequirements::Multiple as u8),
            );

            let config = DataContractConfigV0::get_contract_configuration_properties_v0(&map)
                .expect("should parse V0 config");

            assert_eq!(
                config.requires_identity_encryption_bounded_key,
                Some(StorageKeyRequirements::Unique)
            );
            assert_eq!(
                config.requires_identity_decryption_bounded_key,
                Some(StorageKeyRequirements::Unique),
                "V0 consensus quirk: the DECRYPTION property is ignored"
            );
        }

        /// Sanity check: with neither property set, both fields stay `None`.
        #[test]
        fn neither_property_set_leaves_both_none() {
            let map: BTreeMap<String, Value> = BTreeMap::new();
            let config = DataContractConfigV0::get_contract_configuration_properties_v0(&map)
                .expect("should parse V0 config with defaults");
            assert!(config.requires_identity_encryption_bounded_key.is_none());
            assert!(config.requires_identity_decryption_bounded_key.is_none());
        }
    }

    mod bincode_roundtrip {
        use super::*;
        use bincode::config;

        #[test]
        fn v0_bincode_roundtrip_preserves_fields() {
            let cfg = config::standard();
            let original = DataContractConfig::V0(DataContractConfigV0 {
                can_be_deleted: true,
                readonly: true,
                keeps_history: true,
                documents_keep_history_contract_default: true,
                documents_mutable_contract_default: false,
                documents_can_be_deleted_contract_default: false,
                requires_identity_encryption_bounded_key: Some(StorageKeyRequirements::Unique),
                requires_identity_decryption_bounded_key: None,
            });
            let bytes = bincode::encode_to_vec(original, cfg).expect("encode");
            let (decoded, _): (DataContractConfig, _) =
                bincode::decode_from_slice(&bytes, cfg).expect("decode");
            assert_eq!(decoded, original);
        }

        #[test]
        fn v1_bincode_roundtrip_preserves_sized_integer_types() {
            let cfg = config::standard();
            let original = DataContractConfig::V1(DataContractConfigV1 {
                can_be_deleted: false,
                readonly: false,
                keeps_history: false,
                documents_keep_history_contract_default: false,
                documents_mutable_contract_default: true,
                documents_can_be_deleted_contract_default: true,
                requires_identity_encryption_bounded_key: None,
                requires_identity_decryption_bounded_key: None,
                sized_integer_types: false,
            });
            let bytes = bincode::encode_to_vec(original, cfg).expect("encode");
            let (decoded, _): (DataContractConfig, _) =
                bincode::decode_from_slice(&bytes, cfg).expect("decode");
            assert_eq!(decoded, original);
            // And sized_integer_types is correctly false on the decoded copy
            assert!(!decoded.sized_integer_types());
        }
    }

    mod from_value_tests {
        use super::*;
        use platform_value::platform_value;

        #[test]
        fn from_value_yields_default_for_empty_object() {
            // Empty object -> defaults; succeeds on the latest platform version
            let value = platform_value!({});
            let platform_version = PlatformVersion::latest();
            let cfg = DataContractConfig::from_value(value, platform_version)
                .expect("empty object should deserialize to defaults");
            // All booleans should match defaults
            assert_eq!(cfg.can_be_deleted(), DEFAULT_CONTRACT_CAN_BE_DELETED);
        }
    }

    mod get_contract_configuration_properties_tests {
        use super::*;
        use platform_value::Value;
        use std::collections::BTreeMap;

        fn make_contract_map(can_be_deleted: bool, readonly: bool) -> BTreeMap<String, Value> {
            let mut m = BTreeMap::new();
            m.insert(
                property::CAN_BE_DELETED.to_string(),
                Value::Bool(can_be_deleted),
            );
            m.insert(property::READONLY.to_string(), Value::Bool(readonly));
            m
        }

        #[test]
        fn reads_booleans_from_map() {
            let platform_version = PlatformVersion::latest();
            // Use distinct values for both fields so a key mix-up (reading
            // one field from the other's key) would fail the assertion.
            for (can_be_deleted, readonly) in [(true, false), (false, true)] {
                let contract = make_contract_map(can_be_deleted, readonly);
                let cfg = DataContractConfig::get_contract_configuration_properties(
                    &contract,
                    platform_version,
                )
                .expect("should parse config from map");
                assert_eq!(cfg.can_be_deleted(), can_be_deleted);
                assert_eq!(cfg.readonly(), readonly);
            }
        }

        #[test]
        fn missing_keys_fall_back_to_defaults() {
            let platform_version = PlatformVersion::latest();
            let empty: BTreeMap<String, Value> = BTreeMap::new();
            let cfg =
                DataContractConfig::get_contract_configuration_properties(&empty, platform_version)
                    .expect("should parse empty contract map");
            // Defaults preserved
            assert_eq!(cfg.can_be_deleted(), DEFAULT_CONTRACT_CAN_BE_DELETED);
            assert_eq!(cfg.keeps_history(), DEFAULT_CONTRACT_KEEPS_HISTORY);
        }

        #[test]
        fn non_bool_value_errors() {
            let platform_version = PlatformVersion::latest();
            let mut m: BTreeMap<String, Value> = BTreeMap::new();
            m.insert(
                property::CAN_BE_DELETED.to_string(),
                Value::Text("not-a-bool".to_string()),
            );
            let result =
                DataContractConfig::get_contract_configuration_properties(&m, platform_version);
            assert!(result.is_err());
        }
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
    use crate::data_contract::storage_requirements::keys_for_document_type::StorageKeyRequirements;
    use platform_value::platform_value;
    use serde_json::json;

    /// Non-default values per field so the wire-shape assertion catches any
    /// silent zero-out / flip on round-trip.
    fn fixture() -> DataContractConfig {
        DataContractConfig::V0(DataContractConfigV0 {
            can_be_deleted: true,
            readonly: true,
            keeps_history: true,
            documents_keep_history_contract_default: true,
            documents_mutable_contract_default: false,
            documents_can_be_deleted_contract_default: false,
            requires_identity_encryption_bounded_key: Some(StorageKeyRequirements::Unique),
            requires_identity_decryption_bounded_key: Some(StorageKeyRequirements::Multiple),
        })
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // `requiresIdentity{En,De}cryptionBoundedKey` are `Option<StorageKeyRequirements>`
        // where `StorageKeyRequirements` is `#[repr(u8)]` with `Serialize_repr`
        // (Unique = 0, Multiple = 1). JSON has only one number type, so the
        // u8-ness of these fields is erased on the wire — the Value-path
        // assertion below uses `0u8` / `1u8` to lock in the sized variant.
        assert_eq!(
            json,
            json!({
                "$formatVersion": "0",
                "canBeDeleted": true,
                "readonly": true,
                "keepsHistory": true,
                "documentsKeepHistoryContractDefault": true,
                "documentsMutableContractDefault": false,
                "documentsCanBeDeletedContractDefault": false,
                "requiresIdentityEncryptionBoundedKey": 0,
                "requiresIdentityDecryptionBoundedKey": 1,
            })
        );
        let recovered = DataContractConfig::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        // `0u8` / `1u8`: `StorageKeyRequirements` is `#[repr(u8)]`, and
        // platform_value preserves sized variants (`Value::U8`, not `Value::U64`).
        assert_eq!(
            value,
            platform_value!({
                "$formatVersion": "0",
                "canBeDeleted": true,
                "readonly": true,
                "keepsHistory": true,
                "documentsKeepHistoryContractDefault": true,
                "documentsMutableContractDefault": false,
                "documentsCanBeDeletedContractDefault": false,
                "requiresIdentityEncryptionBoundedKey": 0u8,
                "requiresIdentityDecryptionBoundedKey": 1u8,
            })
        );
        let recovered = DataContractConfig::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
