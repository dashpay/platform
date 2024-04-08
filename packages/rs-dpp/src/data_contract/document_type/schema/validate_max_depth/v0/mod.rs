use platform_value::Value;
use platform_version::version::PlatformVersion;
use std::collections::BTreeSet;

use crate::consensus::basic::data_contract::data_contract_max_depth_exceed_error::DataContractMaxDepthExceedError;
use crate::consensus::basic::data_contract::InvalidJsonSchemaRefError;
use crate::consensus::basic::BasicError;
use crate::data_contract::document_type::schema::MaxDepthValidationResult;
use crate::util::json_schema::resolve_uri;
use crate::validation::ConsensusValidationResult;

#[inline(always)]
pub(super) fn validate_max_depth_v0(
    platform_value: &Value,
    platform_version: &PlatformVersion,
) -> ConsensusValidationResult<MaxDepthValidationResult> {
    let max_allowed_depth = platform_version
        .dpp
        .contract_versions
        .document_type_versions
        .schema
        .max_depth as usize;
    let mut values_depth_queue: Vec<(&Value, usize)> = vec![(platform_value, 0)];
    let mut max_reached_depth: usize = 0;
    let mut visited: BTreeSet<*const Value> = BTreeSet::new();
    let ref_value = Value::Text("$ref".to_string());

    let mut size: u64 = 1; // we start at 1, because we are a value

    while let Some((value, depth)) = values_depth_queue.pop() {
        match value {
            Value::Map(map) => {
                let new_depth = depth + 1;
                if new_depth > max_allowed_depth {
                    return ConsensusValidationResult::new_with_error(
                        BasicError::DataContractMaxDepthExceedError(
                            DataContractMaxDepthExceedError::new(max_allowed_depth),
                        )
                        .into(),
                    );
                }
                if max_reached_depth < new_depth {
                    max_reached_depth = new_depth
                }
                for (property_name, v) in map {
                    size += 1;
                    // handling the internal references
                    if property_name == &ref_value {
                        if let Some(uri) = v.as_str() {
                            let resolved = match resolve_uri(platform_value, uri).map_err(|e| {
                                BasicError::InvalidJsonSchemaRefError(
                                    InvalidJsonSchemaRefError::new(format!(
                                        "invalid ref for max depth '{}': {}",
                                        uri, e
                                    )),
                                )
                            }) {
                                Ok(resolved) => resolved,
                                Err(e) => {
                                    return ConsensusValidationResult::new_with_data_and_errors(
                                        MaxDepthValidationResult {
                                            depth: max_reached_depth as u16, // Not possible this is bigger than u16 max
                                            size,
                                        },
                                        vec![e.into()],
                                    );
                                }
                            };

                            if visited.contains(&(resolved as *const Value)) {
                                return ConsensusValidationResult::new_with_data_and_errors(
                                    MaxDepthValidationResult {
                                        depth: max_reached_depth as u16, // Not possible this is bigger than u16 max
                                        size,
                                    },
                                    vec![BasicError::InvalidJsonSchemaRefError(
                                        InvalidJsonSchemaRefError::new(format!(
                                            "the ref '{}' contains cycles",
                                            uri
                                        )),
                                    )
                                    .into()],
                                );
                            }

                            visited.insert(resolved as *const Value);
                            values_depth_queue.push((resolved, new_depth));
                            continue;
                        }
                    }

                    if v.is_map() || v.is_array() {
                        values_depth_queue.push((v, new_depth))
                    }
                }
            }
            Value::Array(array) => {
                let new_depth = depth + 1;
                if max_reached_depth < new_depth {
                    max_reached_depth = new_depth
                }
                for v in array {
                    size += 1;
                    if v.is_map() || v.is_array() {
                        values_depth_queue.push((v, new_depth))
                    }
                }
            }
            _ => visited.clear(),
        }
    }

    ConsensusValidationResult::new_with_data(MaxDepthValidationResult {
        depth: max_reached_depth as u16, // Not possible this is bigger than u16 max
        size,
    })
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use super::*;

    #[test]
    fn should_return_error_when_cycle_is_spotted() {
        let schema: Value = json!(
             {
                "$defs" : {
                    "object": {
                        "$ref":   "#/$defs/objectTwo"
                    },
                    "objectTwo": {
                        "$ref":  "#/$defs/object"
                    }
                },
                "type": "object",
                "properties": {
                  "foo": { "type": "integer" },
                  "bar": {
                    "type": "string",
                    "pattern": "([a-z]+)+$",
                  },
                  "fooWithRef": {
                    "$ref" : "#/$defs/object"
                  },
                },
                "required": ["foo"],
                "additionalProperties": false,
              }
        )
        .into();

        let result = validate_max_depth_v0(&schema, PlatformVersion::first());

        let err = result.errors.first().expect("expected an error");
        assert_eq!(
            err.to_string(),
            "Invalid JSON Schema $ref: the ref '#/$defs/object' contains cycles".to_string()
        );
    }

    #[test]
    fn should_calculate_valid_depth_with_included_ref() {
        let schema: Value = json!(
             {
                "$defs" : {
                    "object": {
                        "nested":   {
                            "type" : "string"
                        }
                    }
                },
                "type": "object",
                "properties": {
                  "foo": { "type": "integer" },
                  "bar": {
                    "type": "string",
                    "pattern": "([a-z]+)+$",
                  },
                  "fooWithRef": {
                    "$ref" : "#/$defs/object"
                  },
                },
                "required": ["foo"],
                "additionalProperties": false,
              }
        )
        .into();
        let result = validate_max_depth_v0(&schema, PlatformVersion::first())
            .data
            .expect("expected data");
        assert_eq!(result, MaxDepthValidationResult { depth: 5, size: 19 });
    }

    #[test]
    fn should_return_error_with_non_existing_ref() {
        let schema: Value = json!(
             {
                "type": "object",
                "properties": {
                  "foo": { "type": "integer" },
                  "bar": {
                    "type": "string",
                    "pattern": "([a-z]+)+$",
                  },
                  "fooWithRef": {
                    "$ref" : "#/$defs/object"
                  },
                },
                "required": ["foo"],
                "additionalProperties": false,
              }
        )
        .into();
        let result = validate_max_depth_v0(&schema, PlatformVersion::first());

        let err = result.errors.first().expect("expected an error");
        assert_eq!(
            err.to_string(),
            "Invalid JSON Schema $ref: invalid ref for max depth '#/$defs/object': value decoding error: StructureError(\"unable to get property $defs in $defs.object\")"
                .to_string()
        );
    }

    #[test]
    fn should_return_error_with_external_ref() {
        let schema: Value = json!(
             {
                "type": "object",
                "properties": {
                  "foo": { "type": "integer" },
                  "bar": {
                    "type": "string",
                    "pattern": "([a-z]+)+$",
                  },
                  "fooWithRef": {
                    "$ref" : "https://json-schema.org/some"
                  },
                },
                "required": ["foo"],
                "additionalProperties": false,
              }
        )
        .into();
        let result = validate_max_depth_v0(&schema, PlatformVersion::first());

        let err = result.errors.first().expect("expected an error");
        assert_eq!(
            err.to_string(),
            "Invalid JSON Schema $ref: invalid ref for max depth 'https://json-schema.org/some': invalid uri error: only local uri references are allowed"
                .to_string()
        );
    }

    #[test]
    fn should_return_error_with_empty_ref() {
        let schema: Value = json!(
             {
                "type": "object",
                "properties": {
                  "foo": { "type": "integer" },
                  "bar": {
                    "type": "string",
                    "pattern": "([a-z]+)+$",
                  },
                  "fooWithRef": {
                    "$ref" : ""
                  },
                },
                "required": ["foo"],
                "additionalProperties": false,
              }
        )
        .into();
        let result = validate_max_depth_v0(&schema, PlatformVersion::first());

        let err = result.errors.first().expect("expected an error");
        assert_eq!(
            err.to_string(),
            "Invalid JSON Schema $ref: invalid ref for max depth '': invalid uri error: only local uri references are allowed"
                .to_string()
        );
    }

    #[test]
    fn should_calculate_valid_depth() {
        let schema: Value = json!(
             {
                "type": "object",
                "properties": {
                  "foo": { "type": "integer" },
                  "bar": {
                    "type": "string",
                    "pattern": "([a-z]+)+$",
                  },
                },
                "required": ["foo"],
                "additionalProperties": false,
              }
        )
        .into();
        let found_depth = validate_max_depth_v0(&schema, PlatformVersion::first())
            .data
            .expect("expected data")
            .depth;
        assert_eq!(found_depth, 3);
    }

    #[test]
    fn should_calculate_valid_depth_for_empty_json() {
        let schema: Value = json!({}).into();
        let found_depth = validate_max_depth_v0(&schema, PlatformVersion::first())
            .data
            .expect("expected data")
            .depth;
        assert_eq!(found_depth, 1);
    }

    #[test]
    fn should_calculate_valid_depth_for_schema_containing_array() {
        let schema: Value = json!({
                "type": "object",
                "properties": {
                  "foo": { "type": "integer" },
                  "bar": {
                    "type": "string",
                    "pattern": "([a-z]+)+$",
                  },
                },
                "required": [ { "alpha": "value_alpha"}, { "bravo" : { "a" :  "b"} }],

        })
        .into();

        let found_depth = validate_max_depth_v0(&schema, PlatformVersion::first())
            .data
            .expect("expected data")
            .depth;

        assert_eq!(found_depth, 4);
    }
}
