// use jsonschema::{ErrorIterator, JSONSchema, ValidationError};
// use serde_json::json;
// use serde_json::Value as JsonValue;
// use std::borrow::Cow;
// use std::iter::once;
//
// pub(crate) fn error(instance: ValidationError) -> ErrorIterator {
//     Box::new(once(instance))
// }
//
// pub fn validate(json_schema: &JsonValue) -> Result<(), ErrorIterator> {
//     let byte_array_meta = json!({
//         "$schema": "https://json-schema.org/draft/2020-12/schema",
//         "$id": "https://schema.dash.org/dpp-0-4-0/meta/byte-array",
//         "description": "Byte array keyword meta schema",
//         "type": "object",
//         "properties": {
//             "properties": {
//                 "type": "object",
//                 "additionalProperties": {
//                     "type": "object",
//                     "properties": {
//                         "byteArray": {
//                             "type": "boolean",
//                             "const": true
//                         }
//                     }
//                 },
//                 "dependentSchemas": {
//                     "byteArray": {
//                       "description": "should be used only with array type",
//                       "properties": {
//                         "type": {
//                           "type": "string",
//                           "const": "array"
//                         }
//                       },
//                       "not": {
//                         "properties": {
//                           "items": {
//                             "type": "array"
//                           }
//                         },
//                         "required": ["items"]
//                       }
//                     },
//                     "contentMediaType": {
//                       "if": {
//                         "properties": {
//                           "contentMediaType": {
//                             "const": "application/x.dash.dpp.identifier"
//                           }
//                         }
//                       },
//                       "then": {
//                         "properties": {
//                           "byteArray": {
//                             "const": true
//                           },
//                           "minItems": {
//                             "const": 32
//                           },
//                           "maxItems": {
//                             "const": 32
//                           }
//                         },
//                         "required": ["byteArray", "minItems", "maxItems"]
//                       }
//                     },
//                 }
//             }
//         }
//     });
//
//     let byte_array_meta_schema = JSONSchema::compile(&byte_array_meta).map_err(|err| {
//         error(ValidationError {
//             instance_path: err.instance_path.clone(),
//             instance: Cow::Owned(err.instance.into_owned()),
//             kind: err.kind,
//             schema_path: err.schema_path,
//         })
//     })?;
//
//     byte_array_meta_schema
//         .validate(json_schema)
//         .map_err(|err| {
//             let error: Vec<ValidationError> = err
//                 .map(|err| {
//                     ValidationError {
//                         instance_path: err.instance_path.clone(),
//                         instance: Cow::Owned(err.instance.into_owned()),
//                         kind: err.kind,
//                         schema_path: err.schema_path,
//                     }
//                 })
//                 .collect();
//
//             Box::new(error.into_iter())
//         })
// }
