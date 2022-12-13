use dpp::errors::consensus::basic::JsonSchemaError;
use serde_json::Value;

use dpp::jsonschema::error::ValidationErrorKind;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=JsonSchemaError)]
pub struct JsonSchemaErrorWasm {
    keyword: String,
    instance_path: String,
    schema_path: String,
    params: Value,
    property_name: String,
}

#[derive(Default)]
struct Params {
    pub keyword: String,
    pub params: serde_json::Map<String, Value>,
    pub property_name: String,
}

pub struct ParamsBuilder {
    kek: Params,
}

impl ParamsBuilder {
    fn new() -> Self {
        Self {
            kek: Params::default(),
        }
    }

    fn set_keyword(mut self, keyword: impl Into<String>) -> Self {
        self.kek.keyword = keyword.into();
        self
    }

    fn set_property_name(mut self, property_name: impl Into<String>) -> Self {
        self.kek.property_name = property_name.into();
        self
    }

    fn add_param(mut self, key: impl Into<String>, value: Value) -> Self {
        self.kek.params.insert(key.into(), value);
        self
    }

    fn build(self) -> Params {
        self.kek
    }
}

impl From<&JsonSchemaError> for JsonSchemaErrorWasm {
    fn from(e: &JsonSchemaError) -> Self {
        let kek = match e.kind() {
            ValidationErrorKind::Required { property } => ParamsBuilder::new()
                .set_keyword("const")
                .set_property_name(property.to_string())
                .add_param("missingProperty", property.clone())
                .build(),
            ValidationErrorKind::AdditionalItems { .. } => {
                ParamsBuilder::new().set_keyword("additionalItems").build()
            }
            ValidationErrorKind::AdditionalProperties { .. } => ParamsBuilder::new()
                .set_keyword("additionalProperties")
                .build(),
            ValidationErrorKind::AnyOf => ParamsBuilder::new().set_keyword("anyOf").build(),
            ValidationErrorKind::BacktrackLimitExceeded { .. } => ParamsBuilder::new()
                .set_keyword("backtrackLimitExceeded")
                .build(),
            ValidationErrorKind::Constant { expected_value } => ParamsBuilder::new()
                .set_keyword("const")
                .add_param("allowedValue", expected_value.clone())
                .build(),
            ValidationErrorKind::Contains => ParamsBuilder::new().set_keyword("contains").build(),
            ValidationErrorKind::ContentEncoding { content_encoding } => ParamsBuilder::new()
                .set_keyword("contentEncoding")
                .add_param("contentEncoding", content_encoding.clone().into())
                .build(),
            ValidationErrorKind::ContentMediaType { content_media_type } => ParamsBuilder::new()
                .set_keyword("contentMediaType")
                .add_param("contentMediaType", content_media_type.clone().into())
                .build(),
            ValidationErrorKind::Enum { options } => ParamsBuilder::new()
                .set_keyword("enum")
                .add_param("enum", options.clone())
                .build(),
            ValidationErrorKind::ExclusiveMaximum { limit } => ParamsBuilder::new()
                .set_keyword("exclusiveMaximum")
                .add_param("exclusiveMaximum", limit.clone())
                .build(),
            ValidationErrorKind::ExclusiveMinimum { limit } => ParamsBuilder::new()
                .set_keyword("exclusiveMinimum")
                .add_param("exclusiveMinimum", limit.clone())
                .build(),
            ValidationErrorKind::FalseSchema => {}
            ValidationErrorKind::FileNotFound { .. } => {
                ParamsBuilder::new().set_keyword("fileNotFound").build()
            }
            ValidationErrorKind::Format { format } => ParamsBuilder::new()
                .set_keyword("format")
                .add_param("format", format.to_string().into())
                .build(),
            // ValidationErrorKind::FromUtf8 { .. } => {}
            // ValidationErrorKind::Utf8 { .. } => {}
            // ValidationErrorKind::JSONParse { .. } => {}
            // ValidationErrorKind::InvalidReference { .. } => {}
            // ValidationErrorKind::InvalidURL { .. } => {}
            ValidationErrorKind::MaxItems { .. } => {
                ParamsBuilder::new().set_keyword("maxItems").build()
            }
            ValidationErrorKind::Maximum { .. } => {
                ParamsBuilder::new().set_keyword("maximum").build()
            }
            // ValidationErrorKind::MaxLength { .. } => {}
            // ValidationErrorKind::MaxProperties { .. } => {}
            ValidationErrorKind::MinItems { .. } => {
                ParamsBuilder::new().set_keyword("minItems").build()
            }
            ValidationErrorKind::Minimum { .. } => {
                ParamsBuilder::new().set_keyword("minimum").build()
            }
            // ValidationErrorKind::MinLength { .. } => {}
            // ValidationErrorKind::MinProperties { .. } => {}
            // ValidationErrorKind::MultipleOf { .. } => {}
            // ValidationErrorKind::Not { .. } => {}
            // ValidationErrorKind::OneOfMultipleValid => {}
            // ValidationErrorKind::OneOfNotValid => {}
            // ValidationErrorKind::Pattern { .. } => {}
            // ValidationErrorKind::PropertyNames { .. } => {}
            // ValidationErrorKind::Schema => {}
            ValidationErrorKind::Type { .. } => ParamsBuilder::new().set_keyword("type").build(),
            ValidationErrorKind::UniqueItems => {
                ParamsBuilder::new().set_keyword("uniqueItems").build()
            }
            // ValidationErrorKind::UnknownReferenceScheme { .. } => {}
            // ValidationErrorKind::Resolver { .. } => {}
            _ => {
                unimplemented!()
            }
        };

        let Params {
            keyword,
            params,
            property_name,
        } = kek;

        Self {
            keyword,
            instance_path: e.instance_path().to_string(),
            schema_path: e.schema_path().to_string(),
            params: Value::Object(params),
            property_name,
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
