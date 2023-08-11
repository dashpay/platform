mod fields;
pub mod v0;

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
#[serde(tag = "version")]
pub enum DataContractConfig {
    V0(DataContractConfigV0),
}

impl DataContractConfig {
    pub fn default_for_version(
        platform_version: &PlatformVersion,
    ) -> Result<DataContractConfig, ProtocolError> {
        match platform_version.dpp.contract_versions.config_version {
            0 => Ok(DataContractConfigV0::default().into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContractConfig::default_for_version".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    pub fn from_value(
        value: Value,
        platform_version: &PlatformVersion,
    ) -> Result<DataContractConfig, ProtocolError> {
        match platform_version.dpp.contract_versions.config_version {
            0 => {
                let config: DataContractConfigV0 = platform_value::from_value(value)?;
                Ok(config.into())
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContractConfig::from_value".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

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
        match platform_version.dpp.contract_versions.config_version {
            0 => Ok(
                DataContractConfigV0::get_contract_configuration_properties_v0(contract)?.into(),
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContractConfig::get_contract_configuration_properties".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

impl DataContractConfigGettersV0 for DataContractConfig {
    fn can_be_deleted(&self) -> bool {
        match self {
            DataContractConfig::V0(v0) => v0.can_be_deleted,
        }
    }

    fn readonly(&self) -> bool {
        match self {
            DataContractConfig::V0(v0) => v0.readonly,
        }
    }

    fn keeps_history(&self) -> bool {
        match self {
            DataContractConfig::V0(v0) => v0.keeps_history,
        }
    }

    fn documents_keep_history_contract_default(&self) -> bool {
        match self {
            DataContractConfig::V0(v0) => v0.documents_keep_history_contract_default,
        }
    }

    fn documents_mutable_contract_default(&self) -> bool {
        match self {
            DataContractConfig::V0(v0) => v0.documents_mutable_contract_default,
        }
    }
}

impl DataContractConfigSettersV0 for DataContractConfig {
    fn set_can_be_deleted(&mut self, value: bool) {
        match self {
            DataContractConfig::V0(v0) => v0.can_be_deleted = value,
            // If there are other enum variants, you might want to handle them here
            // _ => {} // For example, do nothing or panic
        }
    }

    fn set_readonly(&mut self, value: bool) {
        match self {
            DataContractConfig::V0(v0) => v0.readonly = value,
            // _ => {}
        }
    }

    fn set_keeps_history(&mut self, value: bool) {
        match self {
            DataContractConfig::V0(v0) => v0.keeps_history = value,
            // _ => {}
        }
    }

    fn set_documents_keep_history_contract_default(&mut self, value: bool) {
        match self {
            DataContractConfig::V0(v0) => v0.documents_keep_history_contract_default = value,
            // _ => {}
        }
    }

    fn set_documents_mutable_contract_default(&mut self, value: bool) {
        match self {
            DataContractConfig::V0(v0) => v0.documents_mutable_contract_default = value,
            // _ => {}
        }
    }
}
