use crate::data_contract::v0::DataContractV0;
use crate::data_contract::{DATA_CONTRACT_BINARY_FIELDS_V0, DATA_CONTRACT_IDENTIFIER_FIELDS_V0};
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::{ReplacementType, Value};
use serde_json::Value as JsonValue;

impl DataContractV0 {
    pub fn from_json_object(
        json_value: JsonValue,
        platform_version: &PlatformVersion,
    ) -> Result<DataContractV0, ProtocolError> {
        let mut value: Value = json_value.into();
        value.replace_at_paths(DATA_CONTRACT_BINARY_FIELDS_V0, ReplacementType::BinaryBytes)?;
        value.replace_at_paths(
            DATA_CONTRACT_IDENTIFIER_FIELDS_V0,
            ReplacementType::Identifier,
        )?;
        Self::from_raw_object(value, platform_version)
    }
}
