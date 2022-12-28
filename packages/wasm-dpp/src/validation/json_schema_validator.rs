use anyhow::Context;
use dpp::validation::JsonSchemaValidator;
use serde_json::Value;
use wasm_bindgen::prelude::*;

use crate::utils::{ToSerdeJSONExt, WithJsError};

#[wasm_bindgen(js_name=JsonSchemaValidator)]
pub struct JsonSchemaValidatorWasm(JsonSchemaValidator);

#[wasm_bindgen(js_class=JsonSchemaValidator)]
impl JsonSchemaValidatorWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(
        schema_js: &JsValue,
        definitions: &JsValue,
    ) -> Result<JsonSchemaValidatorWasm, JsValue> {
        let schema = schema_js.with_serde_to_json_value()?;
        let maybe_defs: Option<Value> = if definitions.is_object() {
            Some(definitions.with_serde_to_json_value()?)
        } else {
            None
        };

        let validator = if let Some(defs) = maybe_defs {
            let map = defs
                .as_object()
                .context("definitions are not a map")
                .with_js_error()?;
            JsonSchemaValidator::new_with_definitions(schema, map)
                .context("Schema Validator creation with definitions failed")
                .with_js_error()?
        } else {
            JsonSchemaValidator::new(schema)
                .context("Schema Validator creation failed")
                .with_js_error()?
        };

        Ok(JsonSchemaValidatorWasm(validator))
    }
}
