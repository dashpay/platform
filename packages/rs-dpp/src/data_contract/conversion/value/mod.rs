pub mod v0;

use crate::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
use crate::data_contract::v0::DataContractV0;
use crate::data_contract::DataContract;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::Value;

impl DataContractValueConversionMethodsV0 for DataContract {
    fn from_value(
        raw_object: Value,
        full_validation: bool,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .contract_structure_version
        {
            0 => Ok(
                DataContractV0::from_value(raw_object, full_validation, platform_version)?.into(),
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContract::from_object".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    fn to_value(&self, platform_version: &PlatformVersion) -> Result<Value, ProtocolError> {
        match self {
            DataContract::V0(v0) => v0.to_value(platform_version),
        }
    }

    fn into_value(self, platform_version: &PlatformVersion) -> Result<Value, ProtocolError> {
        match self {
            DataContract::V0(v0) => v0.into_value(platform_version),
        }
    }
}
