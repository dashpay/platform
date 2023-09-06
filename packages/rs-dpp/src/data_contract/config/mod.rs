mod methods;
pub mod v0;

use crate::data_contract::storage_requirements::keys_for_document_type::StorageKeyRequirements;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use derive_more::From;
use platform_value::Value;
use serde::{Deserialize, Serialize};
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
        match platform_version.dpp.contract_versions.config {
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
        match platform_version.dpp.contract_versions.config {
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

    fn keeps_previous_contract_versions(&self) -> bool {
        match self {
            DataContractConfig::V0(v0) => v0.keeps_previous_contract_versions,
        }
    }

    fn documents_keep_history_contract_default(&self) -> bool {
        match self {
            DataContractConfig::V0(v0) => v0.documents_keep_history_contract_default,
        }
    }

    fn documents_mutability_contract_default(&self) -> bool {
        match self {
            DataContractConfig::V0(v0) => v0.documents_mutability_contract_default,
        }
    }

    /// Encryption key storage requirements
    fn requires_identity_encryption_bounded_key(&self) -> Option<StorageKeyRequirements> {
        match self {
            DataContractConfig::V0(v0) => v0.requires_identity_encryption_bounded_key,
        }
    }

    /// Decryption key storage requirements
    fn requires_identity_decryption_bounded_key(&self) -> Option<StorageKeyRequirements> {
        match self {
            DataContractConfig::V0(v0) => v0.requires_identity_encryption_bounded_key,
        }
    }
}

impl DataContractConfigSettersV0 for DataContractConfig {
    fn set_allow_contract_deletion(&mut self, value: bool) {
        match self {
            DataContractConfig::V0(v0) => v0.allow_contract_deletion = value,
        }
    }

    fn set_allow_contract_update(&mut self, value: bool) {
        match self {
            DataContractConfig::V0(v0) => v0.allow_contract_update = value,
        }
    }

    fn set_keeps_previous_contract_versions(&mut self, value: bool) {
        match self {
            DataContractConfig::V0(v0) => v0.keeps_previous_contract_versions = value,
        }
    }

    fn set_documents_keep_history_contract_default(&mut self, value: bool) {
        match self {
            DataContractConfig::V0(v0) => v0.documents_keep_history_contract_default = value,
        }
    }

    fn set_documents_mutability_contract_default(&mut self, value: bool) {
        match self {
            DataContractConfig::V0(v0) => v0.documents_mutability_contract_default = value,
        }
    }

    fn set_requires_identity_encryption_bounded_key(
        &mut self,
        value: Option<StorageKeyRequirements>,
    ) {
        match self {
            DataContractConfig::V0(v0) => v0.requires_identity_encryption_bounded_key = value,
        }
    }

    fn set_requires_identity_decryption_bounded_key(
        &mut self,
        value: Option<StorageKeyRequirements>,
    ) {
        match self {
            DataContractConfig::V0(v0) => v0.requires_identity_decryption_bounded_key = value,
        }
    }
}
