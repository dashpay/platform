use std::collections::BTreeSet;

use anyhow::bail;
use serde_json::Value as JsonValue;

use crate::{
    consensus::basic::BasicError, util::json_value::JsonValueExt, validation::ValidationResult,
};

const MAX_DEPTH: usize = 500;

pub fn validate_data_contract_max_depth(raw_data_contract: &JsonValue) -> ValidationResult<()> {
    let mut result = ValidationResult::default();
    let schema_depth = match calc_max_depth(raw_data_contract) {
        Ok(depth) => depth,
        Err(err) => {
            result.add_error(err);
            return result;
        }
    };

    if schema_depth > MAX_DEPTH {
        result.add_error(BasicError::DataContractMaxDepthExceedError(MAX_DEPTH));
    }
    result
}

fn calc_max_depth(json_value: &JsonValue) -> Result<usize, BasicError> {
    let mut values_depth_queue: Vec<(&JsonValue, usize)> = vec![(json_value, 0)];
    let mut max_depth: usize = 0;
    let mut visited: BTreeSet<*const JsonValue> = BTreeSet::new();

    while let Some((value, depth)) = values_depth_queue.pop() {
        match value {
            JsonValue::Object(map) => {
                let new_depth = depth + 1;
                if max_depth < new_depth {
                    max_depth = new_depth
                }
                for (property_name, v) in map {
                    // handling the internal references
                    if property_name == "$ref" {
                        if let Some(uri) = v.as_str() {
                            let resolved = resolve_uri(json_value, uri).map_err(|e| {
                                BasicError::InvalidJsonSchemaRefError {
                                    ref_error: format!("invalid ref '{}': {}", uri, e),
                                }
                            })?;

                            if visited.contains(&(resolved as *const JsonValue)) {
                                return Err(BasicError::InvalidJsonSchemaRefError {
                                    ref_error: format!("the ref '{}' contains cycles", uri),
                                });
                            }

                            visited.insert(resolved as *const JsonValue);
                            values_depth_queue.push((resolved, new_depth));
                            continue;
                        }
                    }

                    if v.is_object() || v.is_array() {
                        values_depth_queue.push((v, new_depth))
                    }
                }
            }
            JsonValue::Array(array) => {
                let new_depth = depth + 1;
                if max_depth < new_depth {
                    max_depth = new_depth
                }
                for v in array {
                    if v.is_object() || v.is_array() {
                        values_depth_queue.push((v, new_depth))
                    }
                }
            }
            _ => visited.clear(),
        }
    }

    Ok(max_depth)
}

fn resolve_uri<'a>(json: &'a JsonValue, uri: &str) -> Result<&'a JsonValue, anyhow::Error> {
    if !uri.starts_with("#/") {
        bail!("only local references are allowed")
    }

    let string_path = uri.strip_prefix("#/").unwrap().replace('/', ".");
    json.get_value(&string_path)
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use super::*;

    #[test]
    fn should_return_error_when_cycle_is_spotted() {
        let schema = json!(
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
        );
        let result = calc_max_depth(&schema);
        assert!(matches!(
            result,
            Err(BasicError::InvalidJsonSchemaRefError { ref_error }) if ref_error == "the ref '#/$defs/object' contains cycles"
        ));
    }

    #[test]
    fn should_calculate_valid_depth_with_included_ref() {
        let schema = json!(
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
        );
        let result = calc_max_depth(&schema);
        assert!(matches!(result, Ok(5)));
    }

    #[test]
    fn should_return_error_with_non_existing_ref() {
        let schema = json!(
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
        );
        let result = calc_max_depth(&schema);
        println!("the result is {:#?}", result);
        assert!(matches!(
            result,
            Err(BasicError::InvalidJsonSchemaRefError { ref_error }) if ref_error.starts_with("invalid ref '#/$defs/object'")
        ));
    }

    #[test]
    fn should_return_error_with_external_ref() {
        let schema = json!(
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
        );
        let result = calc_max_depth(&schema);
        assert!(matches!(
            result,
            Err(BasicError::InvalidJsonSchemaRefError { ref_error }) if ref_error == "invalid ref 'https://json-schema.org/some': only local references are allowed"
        ));
    }

    #[test]
    fn should_return_error_with_empty_ref() {
        let schema = json!(
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
        );
        let result = calc_max_depth(&schema);
        assert!(matches!(
            result,
            Err(BasicError::InvalidJsonSchemaRefError { ref_error }) if ref_error == "invalid ref '': only local references are allowed"
        ));
    }

    #[test]
    fn should_calculate_valid_depth() {
        let schema = json!(
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
        );
        assert!(matches!(calc_max_depth(&schema), Ok(3)));
    }

    #[test]
    fn should_calculate_valid_depth_for_empty_json() {
        let schema = json!({});
        assert!(matches!(calc_max_depth(&schema), Ok(1)));
    }

    #[test]
    fn should_calculate_valid_depth_for_schema_containing_array() {
        let schema = json!({
                "type": "object",
                "properties": {
                  "foo": { "type": "integer" },
                  "bar": {
                    "type": "string",
                    "pattern": "([a-z]+)+$",
                  },
                },
                "required": [ { "alpha": "value_alpha"}, { "bravo" : { "a" :  "b"} }],

        });
        assert!(matches!(calc_max_depth(&schema), Ok(4)));
    }
}
