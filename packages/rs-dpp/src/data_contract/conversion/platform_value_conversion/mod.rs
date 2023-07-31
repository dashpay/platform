pub mod v0;

use crate::data_contract::conversion::platform_value_conversion::v0::DataContractValueConversionMethodsV0;
use crate::data_contract::v0::DataContractV0;
use crate::data_contract::DataContract;
use crate::version::{FeatureVersion, PlatformVersion};
use crate::ProtocolError;
use platform_value::Value;

impl DataContractValueConversionMethodsV0 for DataContract {
    fn from_object(
        mut raw_object: Value,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .contract_structure_version
        {
            0 => Ok(DataContractV0::from_object(raw_object, platform_version)?.into()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContract::from_object".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }

    fn to_object(&self) -> Result<Value, ProtocolError> {
        match self {
            DataContract::V0(v0) => v0.to_object(),
        }
    }

    fn to_cleaned_object(&self) -> Result<Value, ProtocolError> {
        match self {
            DataContract::V0(v0) => v0.to_cleaned_object(),
        }
    }

    fn into_object(self) -> Result<Value, ProtocolError> {
        match self {
            DataContract::V0(v0) => v0.into_object(),
        }
    }
}
