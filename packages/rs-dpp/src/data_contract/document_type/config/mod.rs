use crate::identity::SecurityLevel;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

mod v0;

pub use v0::DocumentTypeConfigAccessorsV0;

#[derive(Debug, Clone, PartialEq)]
pub enum DocumentTypeConfig {
    V0(v0::DocumentTypeConfigV0),
}

impl DocumentTypeConfig {
    pub fn default_with_platform_version(
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match platform_version.dpp.contract_versions.config {
            0 => Ok(DocumentTypeConfig::V0(v0::DocumentTypeConfigV0::default())),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "default_from_data_contract_config".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

impl DocumentTypeConfigAccessorsV0 for DocumentTypeConfig {
    fn keep_history(&self) -> Option<bool> {
        match self {
            DocumentTypeConfig::V0(v0) => v0.keep_history,
        }
    }

    fn set_keep_history(&mut self, keep_history: Option<bool>) {
        match self {
            DocumentTypeConfig::V0(v0) => v0.keep_history = keep_history,
        }
    }

    fn mutable(&self) -> Option<bool> {
        match self {
            DocumentTypeConfig::V0(v0) => v0.mutable,
        }
    }

    fn set_mutable(&mut self, mutable: Option<bool>) {
        match self {
            DocumentTypeConfig::V0(v0) => v0.mutable = mutable,
        }
    }

    fn security_level_requirement(&self) -> SecurityLevel {
        match self {
            DocumentTypeConfig::V0(v0) => v0.security_level_requirement,
        }
    }

    fn set_security_level_requirement(&mut self, security_level_requirement: SecurityLevel) {
        match self {
            DocumentTypeConfig::V0(v0) => {
                v0.security_level_requirement = security_level_requirement
            }
        }
    }
}
