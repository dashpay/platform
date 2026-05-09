use crate::data_contract::conversion::json::DataContractJsonConversionMethodsV0;
use crate::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;

use crate::data_contract::serialized_version::DataContractInSerializationFormat;
use crate::data_contract::v0::DataContractV0;
use crate::version::PlatformVersion;
use crate::ProtocolError;

use platform_version::TryFromPlatformVersioned;
use serde_json::Value as JsonValue;

impl DataContractJsonConversionMethodsV0 for DataContractV0 {
    fn from_json_validated(
        json_value: JsonValue,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        Self::from_value_validated(json_value.into(), platform_version)
    }

    /// Returns Data Contract as a JSON Value that can be used for validation
    /// (binary fields rendered as JSON arrays of u8 instead of base64).
    fn to_validating_json(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<JsonValue, ProtocolError> {
        let format =
            DataContractInSerializationFormat::try_from_platform_versioned(self, platform_version)?;
        let value = platform_value::to_value(format).map_err(ProtocolError::ValueError)?;
        value
            .try_into_validating_json()
            .map_err(ProtocolError::ValueError)
    }
}
