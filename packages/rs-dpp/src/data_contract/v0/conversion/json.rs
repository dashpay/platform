use crate::data_contract::conversion::json::DataContractJsonConversionMethodsV0;
use crate::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
use crate::data_contract::serialized_version::v0::property_names;
use crate::data_contract::v0::DataContractV0;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::{ReplacementType, Value};
use serde_json::Value as JsonValue;
use std::convert::TryInto;

pub const DATA_CONTRACT_IDENTIFIER_FIELDS_V0: [&str; 2] =
    [property_names::ID, property_names::OWNER_ID];

impl DataContractJsonConversionMethodsV0 for DataContractV0 {
    fn from_json(
        json_value: JsonValue,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let mut value: Value = json_value.into();
        // TODO: Revisit this. We defo don't have entropy
        value.replace_at_paths(
            DATA_CONTRACT_IDENTIFIER_FIELDS_V0,
            ReplacementType::Identifier,
        )?;
        // TODO: We also need to replace the binary fields in document schemas
        Self::from_value(value, platform_version)
    }

    /// Returns Data Contract as a JSON Value
    fn to_json(&self, platform_version: &PlatformVersion) -> Result<JsonValue, ProtocolError> {
        self.to_value(platform_version)?
            .try_into()
            .map_err(ProtocolError::ValueError)

        // TODO: I guess we should convert the binary fields back to base64/base58?
    }
}
