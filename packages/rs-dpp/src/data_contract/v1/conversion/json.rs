use crate::data_contract::conversion::json::DataContractJsonConversionMethodsV0;
use crate::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;

use crate::data_contract::DataContractV1;
use crate::version::PlatformVersion;
use crate::ProtocolError;

use serde_json::Value as JsonValue;

impl DataContractJsonConversionMethodsV0 for DataContractV1 {
    fn from_json(
        json_value: JsonValue,
        full_validation: bool,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        Self::from_value(json_value.into(), full_validation, platform_version)
    }
}
