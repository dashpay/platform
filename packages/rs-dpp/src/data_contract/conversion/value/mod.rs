pub mod v0;

use crate::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
use crate::data_contract::serialized_version::DataContractInSerializationFormat;
use crate::data_contract::v0::DataContractV0;
use crate::data_contract::v1::DataContractV1;
use crate::data_contract::DataContract;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::Value;
use platform_version::TryIntoPlatformVersioned;

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
            1 => Ok(
                DataContractV1::from_value(raw_object, full_validation, platform_version)?.into(),
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContract::from_value".to_string(),
                known_versions: vec![0, 1],
                received: version,
            }),
        }
    }

    fn to_value(&self, platform_version: &PlatformVersion) -> Result<Value, ProtocolError> {
        let format: DataContractInSerializationFormat =
            self.try_into_platform_versioned(platform_version)?;
        platform_value::to_value(&format).map_err(ProtocolError::ValueError)
    }
}
