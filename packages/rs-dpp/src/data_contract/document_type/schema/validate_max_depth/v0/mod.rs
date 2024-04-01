use platform_value::Value;
use std::collections::BTreeSet;

use crate::consensus::basic::data_contract::data_contract_max_depth_exceed_error::DataContractMaxDepthExceedError;
use crate::consensus::basic::data_contract::InvalidJsonSchemaRefError;
use crate::consensus::basic::BasicError;
use crate::util::json_schema::resolve_uri;
use crate::validation::SimpleConsensusValidationResult;

const MAX_DEPTH: usize = 500;

#[inline(always)]
pub(super) fn validate_max_depth_v0(value: &Value) -> SimpleConsensusValidationResult {
    let mut result = SimpleConsensusValidationResult::default();
    let schema_depth = match calculate_max_depth(value) {
        Ok(depth) => depth,
        Err(err) => {
            result.add_error(err);
            return result;
        }
    };

    if schema_depth > MAX_DEPTH {
        result.add_error(BasicError::DataContractMaxDepthExceedError(
            DataContractMaxDepthExceedError::new(schema_depth, MAX_DEPTH),
        ));
    }
    result
}

fn calculate_max_depth(platform_value: &Value) -> Result<usize, BasicError> {
    let mut values_depth_queue: Vec<(&Value, usize)> = vec![(platform_value, 0)];
    let mut max_depth: usize = 0;
    let mut visited: BTreeSet<*const Value> = BTreeSet::new();
    let ref_value = Value::Text("$ref".to_string());

    while let Some((value, depth)) = values_depth_queue.pop() {
        match value {
            Value::Map(map) => {
                let new_depth = depth + 1;
                if max_depth < new_depth {
                    max_depth = new_depth
                }
                for (property_name, v) in map {
                    // handling the internal references
                    if property_name == &ref_value {
                        if let Some(uri) = v.as_str() {
                            let resolved = resolve_uri(platform_value, uri).map_err(|e| {
                                BasicError::InvalidJsonSchemaRefError(
                                    InvalidJsonSchemaRefError::new(format!(
                                        "invalid ref for max depth '{}': {}",
                                        uri, e
                                    )),
                                )
                            })?;

                            if visited.contains(&(resolved as *const Value)) {
                                return Err(BasicError::InvalidJsonSchemaRefError(
                                    InvalidJsonSchemaRefError::new(format!(
                                        "the ref '{}' contains cycles",
                                        uri
                                    )),
                                ));
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
                if max_depth < new_depth {
                    max_depth = new_depth
                }
                for v in array {
                    if v.is_map() || v.is_array() {
                        values_depth_queue.push((v, new_depth))
                    }
                }
            }
            _ => visited.clear(),
        }
    }

    Ok(max_depth)
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
        let result = calculate_max_depth(&schema);

        let err = get_ref_error(result);
        assert_eq!(
            err.message(),
            "the ref '#/$defs/object' contains cycles".to_string()
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
        let result = calculate_max_depth(&schema);
        assert!(matches!(result, Ok(5)));
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
        let result = calculate_max_depth(&schema);

        let err = get_ref_error(result);
        assert_eq!(
            err.message(),
            "invalid ref for max depth '#/$defs/object': value decoding error: StructureError(\"unable to get property $defs in $defs.object\")"
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
        let result = calculate_max_depth(&schema);

        let err = get_ref_error(result);
        assert_eq!(
            err.message(),
            "invalid ref for max depth 'https://json-schema.org/some': invalid uri error: only local uri references are allowed"
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
        let result = calculate_max_depth(&schema);

        let err = get_ref_error(result);
        assert_eq!(
            err.message(),
            "invalid ref for max depth '': invalid uri error: only local uri references are allowed"
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
        assert!(matches!(calculate_max_depth(&schema), Ok(3)));
    }

    #[test]
    fn should_calculate_valid_depth_for_empty_json() {
        let schema: Value = json!({}).into();
        assert!(matches!(calculate_max_depth(&schema), Ok(1)));
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
        assert!(matches!(calculate_max_depth(&schema), Ok(4)));
    }

    pub fn get_ref_error<T>(result: Result<T, BasicError>) -> InvalidJsonSchemaRefError {
        match result {
            Ok(_) => panic!("expected to have validation error"),
            Err(e) => match e {
                BasicError::InvalidJsonSchemaRefError(err) => err,
                _ => panic!("expected error to be a InvalidJsonSchemaRefError"),
            },
        }
    }
}
