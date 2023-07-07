use crate::prelude::DataContract;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::Value;
use std::collections::BTreeMap;

mod v0;

impl DataContract {
    pub fn get_definitions<'a>(
        contract: &'a BTreeMap<String, Value>,
        platform_version: &'a PlatformVersion,
    ) -> Result<BTreeMap<String, &'a Value>, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .contract_class_method_versions
            .get_definitions
        {
            0 => Self::get_definitions_v0(contract),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "get_definitions".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
