use dpp::errors::consensus::basic::JsonSchemaError;
use serde_json::Value;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=JsonSchemaError)]
pub struct JsonSchemaErrorWasm {
    keyword: String,
    instance_path: String,
    schema_path: String,
    params: Value,
    property_name: String,
}

struct Kek {
    keyword: String,
    params: serde_json::Map<String, Value>,
    property_name: String,
}

impl From<&JsonSchemaError> for JsonSchemaErrorWasm {
    fn from(e: &JsonSchemaError) -> Self {
        let map = serde_json::Map::new();
        let params = Value::Object(map);

        // let kek = match e.kind() {
        //     ValidationErrorKind::AdditionalItems { .. } => {}
        //     ValidationErrorKind::AdditionalProperties { .. } => {}
        //     ValidationErrorKind::AnyOf => {}
        //     ValidationErrorKind::BacktrackLimitExceeded { .. } => {}
        //     ValidationErrorKind::Constant { .. } => {}
        //     ValidationErrorKind::Contains => {}
        //     ValidationErrorKind::ContentEncoding { .. } => {}
        //     ValidationErrorKind::ContentMediaType { .. } => {}
        //     ValidationErrorKind::Enum { .. } => {}
        //     ValidationErrorKind::ExclusiveMaximum { .. } => {}
        //     ValidationErrorKind::ExclusiveMinimum { .. } => {}
        //     ValidationErrorKind::FalseSchema => {}
        //     ValidationErrorKind::FileNotFound { .. } => {}
        //     ValidationErrorKind::Format { .. } => {}
        //     ValidationErrorKind::FromUtf8 { .. } => {}
        //     ValidationErrorKind::Utf8 { .. } => {}
        //     ValidationErrorKind::JSONParse { .. } => {}
        //     ValidationErrorKind::InvalidReference { .. } => {}
        //     ValidationErrorKind::InvalidURL { .. } => {}
        //     ValidationErrorKind::MaxItems { .. } => {}
        //     ValidationErrorKind::Maximum { .. } => {}
        //     ValidationErrorKind::MaxLength { .. } => {}
        //     ValidationErrorKind::MaxProperties { .. } => {}
        //     ValidationErrorKind::MinItems { .. } => {}
        //     ValidationErrorKind::Minimum { .. } => {}
        //     ValidationErrorKind::MinLength { .. } => {}
        //     ValidationErrorKind::MinProperties { .. } => {}
        //     ValidationErrorKind::MultipleOf { .. } => {}
        //     ValidationErrorKind::Not { .. } => {}
        //     ValidationErrorKind::OneOfMultipleValid => {}
        //     ValidationErrorKind::OneOfNotValid => {}
        //     ValidationErrorKind::Pattern { .. } => {}
        //     ValidationErrorKind::PropertyNames { .. } => {}
        //     ValidationErrorKind::Required { .. } => {}
        //     ValidationErrorKind::Schema => {}
        //     ValidationErrorKind::Type { .. } => {}
        //     ValidationErrorKind::UniqueItems => {}
        //     ValidationErrorKind::UnknownReferenceScheme { .. } => {}
        //     ValidationErrorKind::Resolver { .. } => {}
        // }

        Self {
            keyword: "".to_string(),
            instance_path: e.instance_path().to_string(),
            schema_path: e.schema_path().to_string(),
            params,
            property_name: "".to_string(),
        }
    }
}

#[wasm_bindgen(js_class=JsonSchemaError)]
impl JsonSchemaErrorWasm {
    #[wasm_bindgen(js_name=getKeyword)]
    pub fn keyword(&self) -> String {
        self.keyword.clone()
    }

    #[wasm_bindgen(js_name=getInstancePath)]
    pub fn instance_path(&self) -> String {
        self.instance_path.clone()
    }

    #[wasm_bindgen(js_name=getSchemaPath)]
    pub fn schema_path(&self) -> String {
        self.schema_path.clone()
    }

    #[wasm_bindgen(js_name=getPropertyName)]
    pub fn property_name(&self) -> String {
        self.property_name.clone()
    }

    #[wasm_bindgen(js_name=getParams)]
    pub fn params(&self) -> Result<JsValue, JsError> {
        serde_wasm_bindgen::to_value(&self.params).map_err(|e| e.into())
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        unimplemented!()
    }
}
