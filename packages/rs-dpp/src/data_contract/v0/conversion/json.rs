use crate::data_contract::conversion::json::DataContractJsonConversionMethodsV0;
use crate::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;

use crate::data_contract::v0::DataContractV0;
use crate::version::PlatformVersion;
use crate::ProtocolError;

use serde_json::Value as JsonValue;
use std::convert::TryInto;

impl DataContractJsonConversionMethodsV0 for DataContractV0 {
    fn from_json(
        json_value: JsonValue,
        full_validation: bool,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        Self::from_value(json_value.into(), full_validation, platform_version)
    }

    /// Returns Data Contract as a JSON Value
    fn to_json(&self, platform_version: &PlatformVersion) -> Result<JsonValue, ProtocolError> {
        self.to_value(platform_version)?
            .try_into()
            .map_err(ProtocolError::ValueError)

        // TODO: I guess we should convert the binary fields back to base64/base58?
    }

    /// Returns Data Contract as a JSON Value that can be used for validation
    fn to_validating_json(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<JsonValue, ProtocolError> {
        self.to_value(platform_version)?
            .try_into_validating_json()
            .map_err(ProtocolError::ValueError)
    }
}
