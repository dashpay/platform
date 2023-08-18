pub mod v0;

use crate::version::PlatformVersion;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use derive_more::From;
use platform_value::Value;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use v0::{DataContractConfigGettersV0, DataContractConfigSettersV0, DataContractConfigV0};

#[derive(Serialize, Deserialize, Encode, Decode, Debug, Clone, Copy, PartialEq, Eq, From)]
#[serde(tag = "$format_version")]
pub enum DataContractConfig {
    #[serde(rename = "0")]
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
}

impl DataContractConfigGettersV0 for DataContractConfig {
    fn is_contract_deletion_allowed(&self) -> bool {
        match self {
            DataContractConfig::V0(v0) => v0.allow_contract_deletion,
        }
    }

    fn is_contract_update_allowed(&self) -> bool {
        match self {
            DataContractConfig::V0(v0) => v0.allow_contract_update,
        }
    }

    fn keep_previous_contract_versions(&self) -> bool {
        match self {
            DataContractConfig::V0(v0) => v0.keep_previous_contract_versions,
        }
    }

    fn documents_keep_history_contract_default(&self) -> bool {
        match self {
            DataContractConfig::V0(v0) => v0.documents_keep_history_contract_default,
        }
    }

    fn documents_read_only_contract_default(&self) -> bool {
        match self {
            DataContractConfig::V0(v0) => v0.documents_read_only_contract_default,
        }
    }
}

impl DataContractConfigSettersV0 for DataContractConfig {
    fn set_allow_contract_deletion(&mut self, value: bool) {
        match self {
            DataContractConfig::V0(v0) => v0.allow_contract_deletion = value,
            // If there are other enum variants, you might want to handle them here
            // _ => {} // For example, do nothing or panic
        }
    }

    fn set_allow_contract_update(&mut self, value: bool) {
        match self {
            DataContractConfig::V0(v0) => v0.allow_contract_update = value,
            // _ => {}
        }
    }

    fn set_keep_previous_contract_versions(&mut self, value: bool) {
        match self {
            DataContractConfig::V0(v0) => v0.keep_previous_contract_versions = value,
            // _ => {}
        }
    }

    fn set_documents_keep_history_contract_default(&mut self, value: bool) {
        match self {
            DataContractConfig::V0(v0) => v0.documents_keep_history_contract_default = value,
            // _ => {}
        }
    }

    fn set_documents_read_only_contract_default(&mut self, value: bool) {
        match self {
            DataContractConfig::V0(v0) => v0.documents_read_only_contract_default = value,
            // _ => {}
        }
    }
}
