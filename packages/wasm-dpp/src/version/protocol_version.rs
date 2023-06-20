use crate::errors::CompatibleProtocolVersionIsNotDefinedErrorWasm;
use crate::utils::ToSerdeJSONExt;
use crate::validation::ValidationResultWasm;
use dpp::util::json_value::JsonValueExt;
use dpp::version::ProtocolVersionValidator;
use std::collections::HashMap;
use std::convert::TryFrom;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=ProtocolVersionValidator)]
pub struct ProtocolVersionValidatorWasm(ProtocolVersionValidator);

#[wasm_bindgen(js_class=ProtocolVersionValidator)]
impl ProtocolVersionValidatorWasm {
    // TODO should the constructor be without parameters?
    #[wasm_bindgen(constructor)]
    pub fn new(options: JsValue) -> Result<ProtocolVersionValidatorWasm, JsValue> {
        if options.is_undefined() {
            Ok(ProtocolVersionValidatorWasm(
                ProtocolVersionValidator::default(),
            ))
        } else {
            let metadata_options = options.with_serde_to_json_value()?;
            let current_protocol_version: u32 = metadata_options
                .get_u32("currentProtocolVersion")
                .map_err(|e| JsError::new(&e.to_string()))?;
            let latest_protocol_version: u32 = metadata_options
                .get_u32("latestProtocolVersion")
                .map_err(|e| JsError::new(&e.to_string()))?;
            let compatibility_map_value: serde_json::Value = metadata_options
                .get_value("versionCompatibilityMap")
                .map(|v| v.clone())
                .map_err(|e| JsError::new(&e.to_string()))?;

            let hash_map = compatibility_map_value
                .as_object()
                .ok_or_else(|| {
                    JsValue::from(JsError::new("Expected compatibility map to be an object"))
                })?
                .into_iter()
                .map(|(key, value)| {
                    let new_key = key
                        .parse::<u32>()
                        .map_err(|e| JsError::new(&e.to_string()))?;

                    let new_value_64 = value.as_u64().ok_or_else(|| {
                        JsError::new("Expect values in compatibility map to contain only numbers")
                    })?;
                    let new_value =
                        u32::try_from(new_value_64).map_err(|e| JsError::new(&e.to_string()))?;

                    Ok((new_key, new_value))
                })
                .collect::<Result<HashMap<u32, u32>, JsError>>()?;

            Ok(ProtocolVersionValidatorWasm(ProtocolVersionValidator::new(
                current_protocol_version,
                latest_protocol_version,
                hash_map,
            )))
        }
    }

    #[wasm_bindgen]
    pub fn validate(&self, version: u32) -> Result<ValidationResultWasm, JsValue> {
        self.0
            .validate(version)
            .map(|v| v.map(|_| JsValue::undefined()))
            .map(ValidationResultWasm::from)
            .map_err(|e| CompatibleProtocolVersionIsNotDefinedErrorWasm::new(e).into())
    }
}

impl ProtocolVersionValidatorWasm {
    pub fn protocol_version(&self) -> u32 {
        self.0.protocol_version()
    }
}

impl From<ProtocolVersionValidator> for ProtocolVersionValidatorWasm {
    fn from(doc_validator: ProtocolVersionValidator) -> Self {
        ProtocolVersionValidatorWasm(doc_validator)
    }
}

impl From<&ProtocolVersionValidator> for ProtocolVersionValidatorWasm {
    fn from(doc_validator: &ProtocolVersionValidator) -> Self {
        ProtocolVersionValidatorWasm(doc_validator.clone())
    }
}

impl From<ProtocolVersionValidatorWasm> for ProtocolVersionValidator {
    fn from(val: ProtocolVersionValidatorWasm) -> Self {
        val.0
    }
}

impl From<&ProtocolVersionValidatorWasm> for ProtocolVersionValidator {
    fn from(val: &ProtocolVersionValidatorWasm) -> Self {
        val.0.clone()
    }
}
