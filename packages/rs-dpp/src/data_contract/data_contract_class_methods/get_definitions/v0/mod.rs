use crate::prelude::DataContract;
use crate::ProtocolError;
use platform_value::btreemap_extensions::BTreeValueMapHelper;
use platform_value::Value;
use std::collections::BTreeMap;

impl DataContract {
    pub(super) fn get_definitions_v0(
        contract: &BTreeMap<String, Value>,
    ) -> Result<BTreeMap<String, &Value>, ProtocolError> {
        Ok(contract
            .get_optional_str_value_map("$defs")?
            .unwrap_or_default())
    }
}
