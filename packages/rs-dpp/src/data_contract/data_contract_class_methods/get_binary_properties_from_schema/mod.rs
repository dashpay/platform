use std::collections::BTreeMap;
use crate::data_contract::DataContract;
use crate::ProtocolError;
use serde_json::Value as JsonValue;
use crate::version::PlatformVersion;

mod v0;

impl DataContract {
    pub fn get_binary_properties(schema: &JsonValue, platform_version: &PlatformVersion) -> Result<BTreeMap<String, JsonValue>, ProtocolError> {
        match platform_version.dpp.contract_versions.contract_class_method_versions.get_binary_properties_from_schema {
            0 => Ok(Self::get_binary_properties_v0(schema)),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "get_binary_properties".to_string(),
                known_versions: vec![0],
                received: version,
            })
        }
    }
}
