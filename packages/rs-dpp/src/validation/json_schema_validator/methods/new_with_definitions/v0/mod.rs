use crate::util::json_value::JsonValueExt;
use crate::validation::JsonSchemaValidator;
use crate::version::PlatformVersion;
use crate::DashPlatformProtocolInitError;
use serde_json::{json, Value as JsonValue};
use std::collections::HashMap;

impl JsonSchemaValidator {
    /// creates a new json schema validator from the json schema and allows to add the definitions
    pub(super) fn new_with_definitions_v0<'a>(
        mut schema_json: JsonValue,
        definitions: impl IntoIterator<Item = (&'a String, &'a JsonValue)>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, DashPlatformProtocolInitError> {
        let defs: HashMap<&str, &'a JsonValue> = definitions
            .into_iter()
            .map(|(k, v)| (k.as_ref(), v))
            .collect();
        let _ = schema_json.insert(String::from("$defs"), json!(defs));

        let mut json_schema_validator = Self {
            raw_schema_json: schema_json,
            schema: None,
        };

        let compilation_options = Self::get_schema_compilation_options(platform_version);
        let json_schema = compilation_options.compile(&json_schema_validator.raw_schema_json)?;
        json_schema_validator.schema = Some(json_schema);

        Ok(json_schema_validator)
    }
}
