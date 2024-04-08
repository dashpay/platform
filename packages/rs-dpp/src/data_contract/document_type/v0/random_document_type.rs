// MIT LICENSE
//
// Copyright (c) 2021 Dash Core Group
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
//

//! Random Document types.
//!
//!
//!
#[derive(Clone, Copy, Debug, PartialEq, Encode, Decode)]
pub struct FieldTypeWeights {
    pub string_weight: u16,
    pub float_weight: u16,
    pub integer_weight: u16,
    pub date_weight: u16,
    pub boolean_weight: u16,
    pub byte_array_weight: u16,
}

#[derive(Clone, Debug, PartialEq, Encode, Decode)]
pub struct FieldMinMaxBounds {
    pub string_min_len: Range<u16>,
    pub string_has_min_len_chance: f64,
    pub string_max_len: Range<u16>,
    pub string_has_max_len_chance: f64,
    pub integer_min: Range<u16>,
    pub integer_has_min_chance: f64,
    pub integer_max: Range<u16>,
    pub integer_has_max_chance: f64,
    pub float_min: Range<f64>,
    pub float_has_min_chance: f64,
    pub float_max: Range<f64>,
    pub float_has_max_chance: f64,
    pub date_min: i64,
    pub date_max: i64,
    pub byte_array_min_len: Range<u16>,
    pub byte_array_has_min_len_chance: f64,
    pub byte_array_max_len: Range<u16>,
    pub byte_array_has_max_len_chance: f64,
}

#[derive(Clone, Debug, PartialEq, Encode, Decode)]
pub struct RandomDocumentTypeParameters {
    pub new_fields_optional_count_range: Range<u16>,
    pub new_fields_required_count_range: Range<u16>,
    pub new_indexes_count_range: Range<u16>,
    pub field_weights: FieldTypeWeights,
    pub field_bounds: FieldMinMaxBounds,
    pub keep_history_chance: f64,
    pub documents_mutable_chance: f64,
}

impl RandomDocumentTypeParameters {
    fn validate_parameters(&self) -> Result<(), ProtocolError> {
        let min_string_len = self.field_bounds.string_min_len.end;
        let max_string_len = self.field_bounds.string_max_len.start;
        if min_string_len > max_string_len {
            return Err(ProtocolError::Generic(
                "String min length range end is greater than max length range start".to_string(),
            ));
        }

        let min_byte_array_len = self.field_bounds.byte_array_min_len.end;
        let max_byte_array_len = self.field_bounds.byte_array_max_len.start;
        if min_byte_array_len > max_byte_array_len {
            return Err(ProtocolError::Generic(
                "Byte array min length range end is greater than max length range start"
                    .to_string(),
            ));
        }

        Ok(())
    }
}

use crate::data_contract::document_type::array::ArrayItemType;
use crate::data_contract::document_type::index_level::IndexLevel;
#[cfg(feature = "validation")]
use crate::data_contract::document_type::v0::StatelessJsonSchemaLazyValidator;
use crate::data_contract::document_type::{
    v0::DocumentTypeV0, DocumentProperty, DocumentPropertyType, DocumentType, Index,
};
use crate::identity::SecurityLevel;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use indexmap::IndexMap;
use itertools::Itertools;
use platform_value::{platform_value, Identifier};
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::Rng;
use serde_json::json;
use std::collections::BTreeSet;
use std::ops::Range;

impl DocumentTypeV0 {
    pub fn random_document_type(
        parameters: RandomDocumentTypeParameters,
        data_contract_id: Identifier,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        // Call the validation function at the beginning
        parameters.validate_parameters()?;

        let field_weights = &parameters.field_weights;

        let total_weight = field_weights.string_weight
            + field_weights.float_weight
            + field_weights.integer_weight
            + field_weights.date_weight
            + field_weights.boolean_weight
            + field_weights.byte_array_weight;

        let random_field = |required: bool, rng: &mut StdRng| -> DocumentProperty {
            let random_weight = rng.gen_range(0..total_weight);
            let document_type = if random_weight < field_weights.string_weight {
                let has_min_len = rng.gen_bool(parameters.field_bounds.string_has_min_len_chance);
                let min_len = if has_min_len {
                    Some(rng.gen_range(parameters.field_bounds.string_min_len.clone()))
                } else {
                    None
                };
                // If a string property is used in an index it must have maxLength 63 or less (v1.0-dev)
                let max_len = Some(63);
                DocumentPropertyType::String(min_len, max_len)
            } else if random_weight < field_weights.string_weight + field_weights.integer_weight {
                DocumentPropertyType::Integer
            } else if random_weight
                < field_weights.string_weight
                    + field_weights.integer_weight
                    + field_weights.float_weight
            {
                DocumentPropertyType::Number
            } else if random_weight
                < field_weights.string_weight
                    + field_weights.integer_weight
                    + field_weights.float_weight
                    + field_weights.date_weight
            {
                DocumentPropertyType::Date
            } else if random_weight
                < field_weights.string_weight
                    + field_weights.integer_weight
                    + field_weights.float_weight
                    + field_weights.date_weight
                    + field_weights.boolean_weight
            {
                DocumentPropertyType::Boolean
            } else {
                let has_min_len =
                    rng.gen_bool(parameters.field_bounds.byte_array_has_min_len_chance);
                let min_len = if has_min_len {
                    Some(rng.gen_range(parameters.field_bounds.byte_array_min_len.clone()))
                } else {
                    None
                };
                // Indexed arrays must have maxItems 255 or less (v1.0-dev)
                let max_len = Some(255);
                DocumentPropertyType::ByteArray(min_len, max_len)
            };

            DocumentProperty {
                property_type: document_type,
                required,
            }
        };

        let optional_field_count = if parameters.new_fields_optional_count_range.is_empty() {
            0
        } else {
            rng.gen_range(parameters.new_fields_optional_count_range.clone())
        };

        let required_field_count = if parameters.new_fields_required_count_range.is_empty() {
            0
        } else {
            rng.gen_range(parameters.new_fields_required_count_range.clone())
        };

        let mut properties = IndexMap::new();
        let mut required_fields = BTreeSet::new();

        for _ in 0..optional_field_count {
            let field_name = format!("field_{}", rng.gen::<u16>());
            properties.insert(field_name, random_field(false, rng));
        }

        for _ in 0..required_field_count {
            let field_name = format!("field_{}", rng.gen::<u16>());
            properties.insert(field_name.clone(), random_field(true, rng));
            required_fields.insert(field_name);
        }

        let index_count = if parameters.new_indexes_count_range.is_empty() {
            0
        } else {
            rng.gen_range(parameters.new_indexes_count_range.clone())
        };

        let field_names: Vec<String> = properties.keys().cloned().collect();
        // DPP only allows 10 properties per index (v1.0-dev)
        let ten_field_names = field_names
            .choose_multiple(&mut rand::thread_rng(), 10)
            .cloned()
            .collect_vec();

        let mut indices = Vec::with_capacity(index_count as usize);

        for _ in 0..index_count {
            match Index::random(&ten_field_names, &indices, rng) {
                Ok(index) => indices.push(index),
                Err(_) => break,
            }
        }

        let documents_keep_history = rng.gen_bool(parameters.keep_history_chance);
        let documents_mutable = rng.gen_bool(parameters.documents_mutable_chance);

        let name = format!("doc_type_{}", rng.gen::<u16>());

        let index_structure =
            IndexLevel::try_from_indices(indices.as_slice(), name.as_str(), platform_version)?;
        let (identifier_paths, binary_paths) = DocumentType::find_identifier_and_binary_paths(
            &properties,
            &PlatformVersion::latest()
                .dpp
                .contract_versions
                .document_type_versions,
        )?;

        // Generate properties JSON schema
        let mut position_counter = 0;
        let properties_json_schema = properties.iter().map(|(key, prop)| {
            let mut schema_part = match &prop.property_type {
                DocumentPropertyType::String(min, max) => {
                    let mut schema = serde_json::Map::new();
                    schema.insert("type".to_string(), serde_json::Value::String("string".to_owned()));
                    if let Some(min_len) = min {
                        schema.insert("minLength".to_string(), serde_json::Value::Number(serde_json::Number::from(*min_len)));
                    }
                    if let Some(max_len) = max {
                        schema.insert("maxLength".to_string(), serde_json::Value::Number(serde_json::Number::from(*max_len)));
                    }
                    serde_json::Value::Object(schema)
                },
                DocumentPropertyType::Integer => {
                    let mut schema = serde_json::Map::new();
                    schema.insert("type".to_string(), serde_json::Value::String("integer".to_owned()));
                    // Add min and max if specified in parameters
                    let integer_min = parameters.field_bounds.integer_min.start;
                    let integer_max = parameters.field_bounds.integer_max.end;
                    schema.insert("minimum".to_string(), serde_json::Value::Number(serde_json::Number::from(integer_min)));
                    schema.insert("maximum".to_string(), serde_json::Value::Number(serde_json::Number::from(integer_max)));
                    serde_json::Value::Object(schema)
                },
                DocumentPropertyType::Number => {
                    let mut schema = serde_json::Map::new();
                    schema.insert("type".to_string(), serde_json::Value::String("number".to_owned()));
                    // Add min and max if specified in parameters
                    let float_min = parameters.field_bounds.float_min.start;
                    let float_max = parameters.field_bounds.float_max.end;
                    schema.insert("minimum".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(float_min).unwrap()));
                    schema.insert("maximum".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(float_max).unwrap()));
                    serde_json::Value::Object(schema)
                },
                DocumentPropertyType::Date => {
                    let mut schema = serde_json::Map::new();
                    schema.insert("type".to_string(), serde_json::Value::String("string".to_owned()));
                    schema.insert("format".to_string(), serde_json::Value::String("date-time".to_owned()));
                    // There's a maxLength constraint in DPP, not sure what it is. Just putting 10 for now.
                    schema.insert("maxLength".to_string(), serde_json::Value::Number(serde_json::Number::from(10)));
                    serde_json::Value::Object(schema)
                },
                DocumentPropertyType::Boolean => {
                    serde_json::json!({"type": "boolean"})
                },
                DocumentPropertyType::ByteArray(min, max) => {
                    let mut schema = serde_json::Map::new();
                    schema.insert("type".to_string(), serde_json::Value::String("array".to_owned()));
                    if let Some(min_len) = min {
                        schema.insert("minItems".to_string(), serde_json::Value::Number(serde_json::Number::from(*min_len)));
                    }
                    if let Some(max_len) = max {
                        schema.insert("maxItems".to_string(), serde_json::Value::Number(serde_json::Number::from(*max_len)));
                    }
                    schema.insert("byteArray".to_string(), serde_json::Value::Bool(true));
                    serde_json::Value::Object(schema)
                },
                DocumentPropertyType::Identifier => {
                    json!({
                        "type": "array",
                        "items": {
                            "type": "string",
                            "pattern": "^[0-9a-fA-F]{64}$"
                        },
                        "minItems": 1,
                        "maxItems": 1,
                        "byteArray": true,
                    })
                },
                DocumentPropertyType::Object(sub_properties) => {
                    let sub_props_schema = sub_properties.iter().map(|(sub_key, _sub_prop)| {
                        (sub_key.clone(), serde_json::json!({"type": "string"}))
                    }).collect::<serde_json::Map<_, _>>();

                    json!({
                        "type": "object",
                        "properties": sub_props_schema,
                        "additionalProperties": false
                    })
                },
                DocumentPropertyType::Array(item_type) => {
                    let items_schema = match *item_type {
                        ArrayItemType::String(min, max) => json!({"type": "string", "minLength": min, "maxLength": max}),
                        ArrayItemType::Integer => json!({"type": "integer"}),
                        ArrayItemType::Number => json!({"type": "number"}),
                        ArrayItemType::ByteArray(min, max) => {
                            json!({"type": "array", "items": {"type": "byte"}, "minItems": min, "maxItems": max})
                        },
                        ArrayItemType::Identifier => json!({"type": "array"}),
                        ArrayItemType::Boolean => json!({"type": "bool"}),
                        ArrayItemType::Date => json!({"type": "date"}),
                    };

                    json!({
                        "type": "array",
                        "items": items_schema,
                        "byteArray": true,
                    })
                },
                DocumentPropertyType::VariableTypeArray(types) => {
                    let types_schema = types.iter().map(|t| {
                        match t {
                            ArrayItemType::String(_, _) => json!({"type": "string"}),
                            _ => json!({})
                        }
                    }).collect::<Vec<_>>();

                    json!({
                        "type": "array",
                        "items": {
                            "oneOf": types_schema
                        }
                    })
                },
            };

            if let serde_json::Value::Object(ref mut schema) = schema_part {
                schema.insert("position".to_string(), serde_json::Value::Number(serde_json::Number::from(position_counter)));
            }
            position_counter += 1;

            (key.clone(), schema_part)
        }).collect::<serde_json::Map<String, serde_json::Value>>();

        // Generate indices
        let indices_json_schema = indices
            .iter()
            .map(|index| {
                let properties_schema = index
                    .properties
                    .iter()
                    .map(|prop| {
                        // Only "asc" is allowed for now (v1.0-dev)
                        json!({ <std::string::String as Clone>::clone(&prop.name): "asc" })
                    })
                    .collect::<Vec<_>>();

                json!({
                    "name": index.name,
                    "properties": properties_schema,
                    "unique": index.unique,
                })
            })
            .collect::<Vec<_>>();

        // Combine everything into the final schema
        let schema = json!({
            "type": "object",
            "properties": properties_json_schema,
            "required": required_fields.iter().cloned().collect::<Vec<_>>(),
            "indices": indices_json_schema,
            "additionalProperties": false,
        });

        // TODO: It might not work properly
        Ok(DocumentTypeV0 {
            name,
            schema: schema.into(),
            indices,
            index_structure,
            flattened_properties: properties.clone(),
            properties,
            identifier_paths,
            binary_paths,
            required_fields,
            documents_keep_history,
            documents_mutable,
            data_contract_id,
            requires_identity_encryption_bounded_key: None,
            requires_identity_decryption_bounded_key: None,
            security_level_requirement: SecurityLevel::HIGH,
            #[cfg(feature = "validation")]
            json_schema_validator: StatelessJsonSchemaLazyValidator::new(),
        })
    }

    /// This is used to create an invalid random document type, often for testing
    pub fn invalid_random_document_type(
        parameters: RandomDocumentTypeParameters,
        data_contract_id: Identifier,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        // Call the validation function at the beginning
        parameters.validate_parameters()?;

        let field_weights = &parameters.field_weights;

        let total_weight = field_weights.string_weight
            + field_weights.float_weight
            + field_weights.integer_weight
            + field_weights.date_weight
            + field_weights.boolean_weight
            + field_weights.byte_array_weight;

        let random_field = |required: bool, rng: &mut StdRng| -> DocumentProperty {
            let random_weight = rng.gen_range(0..total_weight);
            let document_type = if random_weight < field_weights.string_weight {
                let has_min_len = rng.gen_bool(parameters.field_bounds.string_has_min_len_chance);
                let min_len = if has_min_len {
                    Some(rng.gen_range(parameters.field_bounds.string_min_len.clone()))
                } else {
                    None
                };
                // If a string property is used in an index it must have maxLength 63 or less (v1.0-dev)
                let max_len = Some(63);
                DocumentPropertyType::String(min_len, max_len)
            } else if random_weight < field_weights.string_weight + field_weights.integer_weight {
                DocumentPropertyType::Integer
            } else if random_weight
                < field_weights.string_weight
                    + field_weights.integer_weight
                    + field_weights.float_weight
            {
                DocumentPropertyType::Number
            } else if random_weight
                < field_weights.string_weight
                    + field_weights.integer_weight
                    + field_weights.float_weight
                    + field_weights.date_weight
            {
                DocumentPropertyType::Date
            } else if random_weight
                < field_weights.string_weight
                    + field_weights.integer_weight
                    + field_weights.float_weight
                    + field_weights.date_weight
                    + field_weights.boolean_weight
            {
                DocumentPropertyType::Boolean
            } else {
                let has_min_len =
                    rng.gen_bool(parameters.field_bounds.byte_array_has_min_len_chance);
                let min_len = if has_min_len {
                    Some(rng.gen_range(parameters.field_bounds.byte_array_min_len.clone()))
                } else {
                    None
                };
                // Indexed arrays must have maxItems 255 or less (v1.0-dev)
                let max_len = Some(255);
                DocumentPropertyType::ByteArray(min_len, max_len)
            };

            DocumentProperty {
                property_type: document_type,
                required,
            }
        };

        let optional_field_count = if parameters.new_fields_optional_count_range.is_empty() {
            0
        } else {
            rng.gen_range(parameters.new_fields_optional_count_range.clone())
        };

        let required_field_count = if parameters.new_fields_required_count_range.is_empty() {
            0
        } else {
            rng.gen_range(parameters.new_fields_required_count_range.clone())
        };

        let mut properties = IndexMap::new();
        let mut required_fields = BTreeSet::new();

        for _ in 0..optional_field_count {
            let field_name = format!("field_{}", rng.gen::<u16>());
            properties.insert(field_name, random_field(false, rng));
        }

        for _ in 0..required_field_count {
            let field_name = format!("field_{}", rng.gen::<u16>());
            properties.insert(field_name.clone(), random_field(true, rng));
            required_fields.insert(field_name);
        }

        let index_count = if parameters.new_indexes_count_range.is_empty() {
            0
        } else {
            rng.gen_range(parameters.new_indexes_count_range.clone())
        };

        let field_names: Vec<String> = properties.keys().cloned().collect();
        // DPP only allows 10 properties per index (v1.0-dev)
        let ten_field_names = field_names
            .choose_multiple(&mut rand::thread_rng(), 10)
            .cloned()
            .collect_vec();

        let mut indices = Vec::with_capacity(index_count as usize);

        for _ in 0..index_count {
            match Index::random(&ten_field_names, &indices, rng) {
                Ok(index) => indices.push(index),
                Err(_) => break,
            }
        }

        let documents_keep_history = rng.gen_bool(parameters.keep_history_chance);
        let documents_mutable = rng.gen_bool(parameters.documents_mutable_chance);

        let name = format!("doc_type_{}", rng.gen::<u16>());

        let index_structure =
            IndexLevel::try_from_indices(indices.as_slice(), name.as_str(), platform_version)?;
        let (identifier_paths, binary_paths) = DocumentType::find_identifier_and_binary_paths(
            &properties,
            &PlatformVersion::latest()
                .dpp
                .contract_versions
                .document_type_versions,
        )?;

        // Combine everything into the final schema
        let schema = platform_value!({
            "invalid": "yo",
        });

        // TODO: It might not work properly
        Ok(DocumentTypeV0 {
            name,
            schema,
            indices,
            index_structure,
            flattened_properties: properties.clone(),
            properties,
            identifier_paths,
            binary_paths,
            required_fields,
            documents_keep_history,
            documents_mutable,
            data_contract_id,
            requires_identity_encryption_bounded_key: None,
            requires_identity_decryption_bounded_key: None,
            security_level_requirement: SecurityLevel::HIGH,
            #[cfg(feature = "validation")]
            json_schema_validator: StatelessJsonSchemaLazyValidator::new(),
        })
    }
}
