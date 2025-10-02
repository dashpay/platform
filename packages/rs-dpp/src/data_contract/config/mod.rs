mod fields;
mod methods;
pub mod v0;
pub mod v1;

use crate::data_contract::config::v1::{
    DataContractConfigGettersV1, DataContractConfigSettersV1, DataContractConfigV1,
};
use crate::data_contract::storage_requirements::keys_for_document_type::StorageKeyRequirements;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use derive_more::From;
pub use fields::*;
use platform_value::Value;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use v0::{DataContractConfigGettersV0, DataContractConfigSettersV0, DataContractConfigV0};

#[derive(Serialize, Deserialize, Encode, Decode, Debug, Clone, Copy, PartialEq, Eq, From)]
#[serde(tag = "$format_version")]
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
