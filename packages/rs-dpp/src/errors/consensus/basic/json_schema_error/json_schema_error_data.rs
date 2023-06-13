use jsonschema::error::{TypeKind, ValidationErrorKind};
use jsonschema::paths::PathChunk;
use jsonschema::ValidationError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::ops::Deref;
use std::fmt::Write;

#[derive(Debug, Serialize, Default, Deserialize)]
pub struct JsonSchemaErrorData {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    pub keyword: String,
    pub params: serde_json::Map<String, Value>,
    pub property_name: String,
    pub error_message: String
}

impl<'a> From<&ValidationError<'a>> for JsonSchemaErrorData {
    fn from(validation_error: &ValidationError<'a>) -> Self {
        let mut error_message = String::new();
        let _ = write!(&mut error_message, "{}", validation_error);
        let builder = DataBuilder::new().set_error_message(error_message);

        match &validation_error.kind {
            ValidationErrorKind::Required { property } => DataBuilder::new()
                .set_keyword("required")
                .set_property_name(property.to_string())
                .add_param("missingProperty", property.clone())
                .build(),
            ValidationErrorKind::AdditionalItems { limit } => DataBuilder::new()
                .set_keyword("additionalItems")
                .add_param("maxItems", Value::from(*limit))
                .build(),
            ValidationErrorKind::AdditionalProperties { unexpected } => DataBuilder::new()
                .set_keyword("additionalProperties")
                .add_param("additionalProperties", unexpected.clone().into())
                .build(),
            ValidationErrorKind::AnyOf => DataBuilder::new().set_keyword("anyOf").build(),
            ValidationErrorKind::BacktrackLimitExceeded { .. } => DataBuilder::new()
                .set_keyword("backtrackLimitExceeded")
                .build(),
            ValidationErrorKind::Constant { expected_value } => DataBuilder::new()
                .set_keyword("const")
                .add_param("allowedValue", expected_value.clone())
                .build(),
            ValidationErrorKind::Contains => DataBuilder::new().set_keyword("contains").build(),
            ValidationErrorKind::ContentEncoding { content_encoding } => DataBuilder::new()
                .set_keyword("contentEncoding")
                .add_param("contentEncoding", content_encoding.clone().into())
                .build(),
            ValidationErrorKind::ContentMediaType { content_media_type } => DataBuilder::new()
                .set_keyword("contentMediaType")
                .add_param("contentMediaType", content_media_type.clone().into())
                .build(),
            ValidationErrorKind::Enum { options } => DataBuilder::new()
                .set_keyword("enum")
                .add_param("enum", options.clone())
                .build(),
            ValidationErrorKind::ExclusiveMaximum { limit } => DataBuilder::new()
                .set_keyword("exclusiveMaximum")
                .add_param("exclusiveMaximum", limit.clone())
                .build(),
            ValidationErrorKind::ExclusiveMinimum { limit } => DataBuilder::new()
                .set_keyword("exclusiveMinimum")
                .add_param("exclusiveMinimum", limit.clone())
                .build(),
            ValidationErrorKind::FalseSchema => {
                let last_path_segment = validation_error.schema_path.last();
                let keyword = match last_path_segment {
                    Some(PathChunk::Keyword(keyword)) => keyword,
                    _ => "falseSchema",
                };

                DataBuilder::new().set_keyword(keyword).build()
            }
            ValidationErrorKind::FileNotFound { .. } => {
                DataBuilder::new().set_keyword("fileNotFound").build()
            }
            ValidationErrorKind::Format { format } => DataBuilder::new()
                .set_keyword("format")
                .add_param("format", format.to_string().into())
                .build(),
            ValidationErrorKind::FromUtf8 { .. } => {
                DataBuilder::new().set_keyword("fromUtf8").build()
            }
            ValidationErrorKind::Utf8 { .. } => DataBuilder::new().set_keyword("utf8").build(),
            ValidationErrorKind::JSONParse { .. } => {
                DataBuilder::new().set_keyword("JSONParse").build()
            }
            ValidationErrorKind::InvalidReference { reference } => DataBuilder::new()
                .set_keyword("invalidReference")
                .add_param("invalidReference", reference.clone().into())
                .build(),
            ValidationErrorKind::InvalidURL { .. } => {
                DataBuilder::new().set_keyword("invalidURL").build()
            }
            ValidationErrorKind::MaxItems { limit } => DataBuilder::new()
                .set_keyword("maxItems")
                .add_param("maxItems", Value::from(*limit))
                .build(),
            ValidationErrorKind::Maximum { limit } => DataBuilder::new()
                .set_keyword("maximum")
                .add_param("maximum", limit.clone())
                .build(),
            ValidationErrorKind::MaxLength { limit } => DataBuilder::new()
                .set_keyword("maxLength")
                .add_param("maxLength", Value::from(*limit))
                .build(),
            ValidationErrorKind::MaxProperties { limit } => DataBuilder::new()
                .set_keyword("maxProperties")
                .add_param("maxProperties", Value::from(*limit))
                .build(),
            ValidationErrorKind::MinItems { limit } => DataBuilder::new()
                .set_keyword("minItems")
                .add_param("minItems", Value::from(*limit))
                .build(),
            ValidationErrorKind::Minimum { limit } => DataBuilder::new()
                .set_keyword("minimum")
                .add_param("minimum", limit.clone())
                .build(),
            ValidationErrorKind::MinLength { limit } => DataBuilder::new()
                .set_keyword("minLength")
                .add_param("minLength", Value::from(*limit))
                .build(),
            ValidationErrorKind::MinProperties { limit } => DataBuilder::new()
                .set_keyword("minProperties")
                .add_param("minProperties", Value::from(*limit))
                .build(),
            ValidationErrorKind::MultipleOf { multiple_of } => DataBuilder::new()
                .set_keyword("multipleOf")
                .add_param("multipleOf", Value::from(*multiple_of))
                .build(),
            ValidationErrorKind::Not { schema } => DataBuilder::new()
                .set_keyword("not")
                .add_param("not", schema.clone())
                .build(),
            ValidationErrorKind::OneOfMultipleValid => {
                DataBuilder::new().set_keyword("oneOfMultipleValid").build()
            }
            ValidationErrorKind::OneOfNotValid => {
                DataBuilder::new().set_keyword("oneOfNotValid").build()
            }
            ValidationErrorKind::Pattern { pattern } => DataBuilder::new()
                .set_keyword("pattern")
                .add_param("pattern", pattern.clone().into())
                .build(),
            ValidationErrorKind::PropertyNames { error } => {
                let JsonSchemaErrorData {
                    keyword,
                    params,
                    property_name,
                    error_message,
                } = JsonSchemaErrorData::from(error.deref());

                DataBuilder::new()
                    .set_keyword("propertyNames")
                    .add_param("instancePath", error.instance_path.to_string().into())
                    .add_param("schemaPath", error.schema_path.to_string().into())
                    .add_param("instance", error.instance.deref().clone())
                    .add_param("params", params.into())
                    .add_param("keyword", keyword.into())
                    .add_param("propertyName", property_name.into())
                    .build()
            }
            ValidationErrorKind::Schema => DataBuilder::new().set_keyword("schema").build(),
            ValidationErrorKind::Type { kind } => {
                let val: Value = match kind {
                    TypeKind::Single(single) => single.to_string().into(),
                    TypeKind::Multiple(multiple) => multiple
                        .into_iter()
                        .map(|single| Value::from(single.to_string()))
                        .collect::<Vec<Value>>()
                        .into(),
                };
                DataBuilder::new()
                    .set_keyword("type")
                    .add_param("type", val)
                    .build()
            }
            ValidationErrorKind::UniqueItems => {
                DataBuilder::new().set_keyword("uniqueItems").build()
            }
            ValidationErrorKind::UnknownReferenceScheme { scheme } => DataBuilder::new()
                .set_keyword("unknownReferenceScheme")
                .add_param("unknownReferenceScheme", scheme.clone().into())
                .build(),
            ValidationErrorKind::Resolver { url, error: _ } => DataBuilder::new()
                .set_keyword("resolver")
                .add_param("url", url.to_string().into())
                .build(),
        }
    }
}

struct DataBuilder {
    data: JsonSchemaErrorData,
}

impl DataBuilder {
    fn new() -> Self {
        Self {
            data: JsonSchemaErrorData::default(),
        }
    }

    fn set_keyword(mut self, keyword: impl Into<String>) -> Self {
        self.data.keyword = keyword.into();
        self
    }

    fn set_property_name(mut self, property_name: impl Into<String>) -> Self {
        self.data.property_name = property_name.into().trim_matches('"').to_string();
        self
    }

    fn add_param(mut self, key: impl Into<String>, value: Value) -> Self {
        self.data.params.insert(key.into(), value);
        self
    }

    fn set_error_message(mut self, message: String) -> Self {
        self.data.error_message = message;
        self
    }

    fn build(self) -> JsonSchemaErrorData {
        self.data
    }
}
