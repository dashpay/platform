mod v0;
pub use v0::*;

use crate::data_contract::{DataContract, DataContractV0, DataContractV1};
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

impl DataContractJsonConversionMethodsV0 for DataContract {
    fn from_json(
        json_value: serde_json::Value,
        full_validation: bool,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        Self: Sized,
    {
        match platform_version
            .dpp
            .contract_versions
            .contract_structure_version
        {
            0 => Ok(
                DataContractV0::from_json(json_value, full_validation, platform_version)?.into(),
            ),
            1 => Ok(
                DataContractV1::from_json(json_value, full_validation, platform_version)?.into(),
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContract::from_json".to_string(),
                known_versions: vec![0, 1],
                received: version,
            }),
        }
    }

    fn to_json(&self, platform_version: &PlatformVersion) -> Result<serde_json::Value, ProtocolError> {
        match self {
            DataContract::V0(v0) => v0.to_json(platform_version),
            DataContract::V1(v1) => v1.to_json(platform_version),
        }
    }

    fn to_validating_json(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<serde_json::Value, ProtocolError> {
        match self {
            DataContract::V0(v0) => v0.to_validating_json(platform_version),
            DataContract::V1(v1) => v1.to_validating_json(platform_version),
        }
    }
}
