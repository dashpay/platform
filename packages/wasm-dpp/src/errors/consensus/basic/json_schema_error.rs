use dpp::errors::consensus::basic::JsonSchemaError;
use serde::Serialize;
use serde_json::Value;
use std::ops::Deref;

use dpp::jsonschema::error::{TypeKind, ValidationErrorKind};
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

impl From<&ValidationErrorKind> for Params {
    fn from(validation_error_kind: &ValidationErrorKind) -> Self {
        match validation_error_kind {
            ValidationErrorKind::Required { property } => ParamsBuilder::new()
                .set_keyword("required")
                .set_property_name(property.to_string())
                .add_param("missingProperty", property.clone())
                .build(),
            ValidationErrorKind::AdditionalItems { limit } => ParamsBuilder::new()
                .set_keyword("additionalItems")
                .add_param("maxItems", Value::from(*limit))
                .build(),
            ValidationErrorKind::AdditionalProperties { unexpected } => ParamsBuilder::new()
                .set_keyword("additionalProperties")
                .add_param("additionalProperties", unexpected.clone().into())
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
            ValidationErrorKind::FalseSchema => {
                ParamsBuilder::new().set_keyword("falseSchema").build()
            }
            ValidationErrorKind::FileNotFound { .. } => {
                ParamsBuilder::new().set_keyword("fileNotFound").build()
            }
            ValidationErrorKind::Format { format } => ParamsBuilder::new()
                .set_keyword("format")
                .add_param("format", format.to_string().into())
                .build(),
            ValidationErrorKind::FromUtf8 { .. } => {
                ParamsBuilder::new().set_keyword("fromUtf8").build()
            }
            ValidationErrorKind::Utf8 { .. } => ParamsBuilder::new().set_keyword("utf8").build(),
            ValidationErrorKind::JSONParse { .. } => {
                ParamsBuilder::new().set_keyword("JSONParse").build()
            }
            ValidationErrorKind::InvalidReference { reference } => ParamsBuilder::new()
                .set_keyword("invalidReference")
                .add_param("invalidReference", reference.clone().into())
                .build(),
            ValidationErrorKind::InvalidURL { .. } => {
                ParamsBuilder::new().set_keyword("invalidURL").build()
            }
            ValidationErrorKind::MaxItems { limit } => ParamsBuilder::new()
                .set_keyword("maxItems")
                .add_param("maxItems", Value::from(*limit))
                .build(),
            ValidationErrorKind::Maximum { limit } => ParamsBuilder::new()
                .set_keyword("maximum")
                .add_param("maximum", limit.clone())
                .build(),
            ValidationErrorKind::MaxLength { limit } => ParamsBuilder::new()
                .set_keyword("maxLength")
                .add_param("maxLength", Value::from(*limit))
                .build(),
            ValidationErrorKind::MaxProperties { limit } => ParamsBuilder::new()
                .set_keyword("maxProperties")
                .add_param("maxProperties", Value::from(*limit))
                .build(),
            ValidationErrorKind::MinItems { limit } => ParamsBuilder::new()
                .set_keyword("minItems")
                .add_param("maximum", Value::from(*limit))
                .build(),
            ValidationErrorKind::Minimum { limit } => ParamsBuilder::new()
                .set_keyword("minimum")
                .add_param("minimum", limit.clone())
                .build(),
            ValidationErrorKind::MinLength { limit } => ParamsBuilder::new()
                .set_keyword("minLength")
                .add_param("minLength", Value::from(*limit))
                .build(),
            ValidationErrorKind::MinProperties { limit } => ParamsBuilder::new()
                .set_keyword("minProperties")
                .add_param("minProperties", Value::from(*limit))
                .build(),
            ValidationErrorKind::MultipleOf { multiple_of } => ParamsBuilder::new()
                .set_keyword("multipleOf")
                .add_param("multipleOf", Value::from(*multiple_of))
                .build(),
            ValidationErrorKind::Not { schema } => ParamsBuilder::new()
                .set_keyword("not")
                .add_param("not", schema.clone())
                .build(),
            ValidationErrorKind::OneOfMultipleValid => ParamsBuilder::new()
                .set_keyword("oneOfMultipleValid")
                .build(),
            ValidationErrorKind::OneOfNotValid => {
                ParamsBuilder::new().set_keyword("oneOfNotValid").build()
            }
            ValidationErrorKind::Pattern { pattern } => ParamsBuilder::new()
                .set_keyword("pattern")
                .add_param("pattern", pattern.clone().into())
                .build(),
            ValidationErrorKind::PropertyNames { error } => {
                let Params {
                    keyword,
                    params,
                    property_name,
                } = Params::from(&error.kind);

                ParamsBuilder::new()
                    .set_keyword("propertyNames")
                    .add_param("instancePath", error.instance_path.to_string().into())
                    .add_param("schemaPath", error.schema_path.to_string().into())
                    .add_param("instance", error.instance.deref().clone())
                    .add_param("params", params.into())
                    .add_param("keyword", keyword.into())
                    .add_param("propertyName", property_name.into())
                    .build()
            }
            ValidationErrorKind::Schema => ParamsBuilder::new().set_keyword("schema").build(),
            ValidationErrorKind::Type { kind } => {
                let val: Value = match kind {
                    TypeKind::Single(single) => single.to_string().into(),
                    TypeKind::Multiple(multiple) => multiple
                        .into_iter()
                        .map(|single| Value::from(single.to_string()))
                        .collect::<Vec<Value>>()
                        .into(),
                };
                ParamsBuilder::new()
                    .set_keyword("type")
                    .add_param("type", val)
                    .build()
            }
            ValidationErrorKind::UniqueItems => {
                ParamsBuilder::new().set_keyword("uniqueItems").build()
            }
            ValidationErrorKind::UnknownReferenceScheme { scheme } => ParamsBuilder::new()
                .set_keyword("unknownReferenceScheme")
                .add_param("unknownReferenceScheme", scheme.clone().into())
                .build(),
            ValidationErrorKind::Resolver { url, error: _ } => ParamsBuilder::new()
                .set_keyword("resolver")
                .add_param("url", url.to_string().into())
                .build(),
        }
    }
}

impl From<&JsonSchemaError> for JsonSchemaErrorWasm {
    fn from(e: &JsonSchemaError) -> Self {
        let Params {
            keyword,
            params,
            property_name,
        } = Params::from(e.kind());

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
        let ser = serde_wasm_bindgen::Serializer::json_compatible();
        self.params.serialize(&ser).map_err(|e| e.into())
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        unimplemented!()
    }
}
