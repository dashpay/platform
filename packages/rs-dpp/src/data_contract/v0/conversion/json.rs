use crate::data_contract::conversion::json_conversion::DataContractJsonConversionMethodsV0;
use crate::data_contract::conversion::platform_value_conversion::v0::DataContractValueConversionMethodsV0;
use crate::data_contract::v0::DataContractV0;
use crate::data_contract::{DATA_CONTRACT_BINARY_FIELDS_V0, DATA_CONTRACT_IDENTIFIER_FIELDS_V0};
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::{ReplacementType, Value};
use serde_json::Value as JsonValue;
use std::convert::TryInto;

impl DataContractJsonConversionMethodsV0 for DataContractV0 {
    fn from_json_object(
        json_value: JsonValue,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let mut value: Value = json_value.into();
        value.replace_at_paths(DATA_CONTRACT_BINARY_FIELDS_V0, ReplacementType::BinaryBytes)?;
        value.replace_at_paths(
            DATA_CONTRACT_IDENTIFIER_FIELDS_V0,
            ReplacementType::Identifier,
        )?;
        Self::from_object(value, platform_version)
    }

    /// Returns Data Contract as a JSON Value
    fn to_json(&self, platform_version: &PlatformVersion) -> Result<JsonValue, ProtocolError> {
        self.to_object(platform_version)?
            .try_into()
            .map_err(ProtocolError::ValueError)
    }

    // TODO: is this method needed?
    fn to_json_object(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<JsonValue, ProtocolError> {
        self.to_object(platform_version)?
            .try_into_validating_json()
            .map_err(ProtocolError::ValueError)
    }
}
