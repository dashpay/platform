use serde_json::Value as JsonValue;

use crate::{consensus::basic::BasicError, validation::ValidationResult};

const MAX_DEPTH: usize = 500;

// TODO: Unlike the JS version, this validator doesn't resolve the schema local references.
// TODO: The JS version don't resolve external defs by design. Is this validator necessary if
// TODO: the limit can be escaped by introducing external definitions?
pub fn validate_data_contract_max_depth(raw_data_contract: &JsonValue) -> ValidationResult {
    let mut result = ValidationResult::default();
    if calc_max_depth(raw_data_contract) > MAX_DEPTH {
        result.add_error(BasicError::DataContractMaxDepthExceedError(MAX_DEPTH));
    }
    result
}

fn calc_max_depth(json_value: &JsonValue) -> usize {
    let mut values_depth_queue: Vec<(&JsonValue, usize)> = vec![(json_value, 0)];
    let mut max_depth: usize = 0;

    while let Some((value, depth)) = values_depth_queue.pop() {
        match value {
            JsonValue::Object(map) => {
                let new_depth = depth + 1;
                if max_depth < new_depth {
                    max_depth = new_depth
                }
                for (_, v) in map {
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
            _ => {}
        }
    }
    max_depth
}

#[cfg(test)]
mod test {
    use super::*;
    use serde_json::json;

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
        assert_eq!(calc_max_depth(&schema), 3);
    }

    #[test]
    fn should_calculate_valid_depth_for_empty_json() {
        let schema = json!({});
        assert_eq!(calc_max_depth(&schema), 1);
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
        assert_eq!(calc_max_depth(&schema), 4);
    }
}
