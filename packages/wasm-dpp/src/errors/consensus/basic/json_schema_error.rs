use dpp::errors::consensus::basic::json_schema_error::JsonSchemaError;

use dpp::errors::consensus::codes::ErrorWithCode;

use dpp::errors::consensus::ConsensusError;

use serde_json::Value;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=JsonSchemaError, inspectable)]
#[derive(Debug)]
pub struct JsonSchemaErrorWasm {
    inner: JsonSchemaError,
}

impl JsonSchemaErrorWasm {
    pub fn new(e: &JsonSchemaError) -> Self {
        Self {
            inner: e.to_owned(),
        }
    }
}

#[wasm_bindgen(js_class=JsonSchemaError)]
impl JsonSchemaErrorWasm {
    #[wasm_bindgen(js_name=getKeyword)]
    pub fn keyword(&self) -> String {
        self.inner.keyword().to_string()
    }

    #[wasm_bindgen(js_name=getInstancePath)]
    pub fn instance_path(&self) -> String {
        self.inner.instance_path().to_string()
    }

    #[wasm_bindgen(js_name=getSchemaPath)]
    pub fn schema_path(&self) -> String {
        self.inner.schema_path().to_string()
    }

    #[wasm_bindgen(js_name=getPropertyName)]
    pub fn property_name(&self) -> String {
        self.inner.property_name().to_string()
    }

    #[wasm_bindgen(js_name=getParams)]
    pub fn params(&self) -> Result<JsValue, JsError> {
        // let ser = serde_wasm_bindgen::Serializer::json_compatible();
        let json_value = self.inner.params().clone().try_into_validating_json()?;

        serde_wasm_bindgen::to_value(&json_value).map_err(|e| e.into())

        // json_value.serialize(&ser).map_err(|e| e.into())
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        ConsensusError::from(self.inner.clone()).code()
    }

    #[wasm_bindgen(js_name=toString)]
    pub fn to_string_format(&self) -> String {
        format!("{:#?}", self)
    }
}
