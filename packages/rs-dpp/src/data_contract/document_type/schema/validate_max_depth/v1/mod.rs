use platform_value::Value;
use platform_version::version::PlatformVersion;
use std::collections::BTreeSet;

use crate::consensus::basic::data_contract::data_contract_max_depth_exceed_error::DataContractMaxDepthExceedError;
use crate::consensus::basic::data_contract::InvalidJsonSchemaRefError;
use crate::consensus::basic::BasicError;
use crate::data_contract::document_type::schema::MaxDepthValidationResult;
use crate::util::json_schema::resolve_uri;
use crate::validation::ConsensusValidationResult;

/// Same walk as v0, except a `$ref` whose target is not a map or array is not
/// walked and `visited` is never cleared. Every ref target is therefore
/// expanded at most once, so the walk is bounded by the schema size times its
/// depth. For
/// schemas without a scalar `$ref` target the result equals v0's.
#[inline(always)]
pub(super) fn validate_max_depth_v1(
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

                            // A scalar target adds no depth, so it is not walked.
                            if !resolved.is_map() && !resolved.is_array() {
                                continue;
                            }

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
            _ => {}
        }
    }

    ConsensusValidationResult::new_with_data(MaxDepthValidationResult {
        depth: max_reached_depth as u16, // Not possible this is bigger than u16 max
        size,
    })
}

#[cfg(test)]
mod test {
    use super::super::v0::validate_max_depth_v0;
    use super::*;
    use crate::consensus::ConsensusError;
    use serde_json::json;
    use std::time::{Duration, Instant};

    fn platform_version() -> &'static PlatformVersion {
        PlatformVersion::get(14).expect("expected protocol version 14")
    }

    /// `$defs` chain whose scalar `$ref`s made v0 expand every level twice.
    fn exponential_chain_schema(levels: usize) -> Value {
        let mut defs = serde_json::Map::new();
        for i in 0..levels {
            let next = format!("#/$defs/d{}", i + 1);
            defs.insert(
                format!("d{}", i),
                json!({
                    "a": { "$ref": next },
                    "m": { "$ref": "#/$schema" },
                    "y": { "$ref": next },
                    "zz": { "$ref": "#/$schema" },
                }),
            );
        }
        defs.insert(format!("d{}", levels), json!({}));
        json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$defs": defs,
            "type": "object",
            "properties": { "foo": { "type": "integer" } },
            "additionalProperties": false,
        })
        .into()
    }

    #[test]
    fn should_reject_exponential_ref_chain_in_bounded_time() {
        let schema = exponential_chain_schema(30);

        let start = Instant::now();
        let result = validate_max_depth_v1(&schema, platform_version());
        assert!(start.elapsed() < Duration::from_secs(1));

        let Some(ConsensusError::BasicError(BasicError::InvalidJsonSchemaRefError(e))) =
            result.errors.first()
        else {
            panic!(
                "expected InvalidJsonSchemaRefError, got {:?}",
                result.errors
            );
        };
        assert!(e.to_string().contains("contains cycles"));
    }

    #[test]
    fn should_match_v0_for_schema_without_scalar_ref() {
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
                    "type": "object",
                    "properties": {
                        "baz": { "type": "array", "items": [{ "type": "string" }] },
                    },
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

        let v0 = validate_max_depth_v0(&schema, platform_version())
            .data
            .expect("expected data");
        let v1 = validate_max_depth_v1(&schema, platform_version())
            .data
            .expect("expected data");
        assert_eq!(v0, v1);
    }

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
                  "fooWithRef": {
                    "$ref" : "#/$defs/object"
                  },
                },
                "required": ["foo"],
                "additionalProperties": false,
              }
        )
        .into();

        let result = validate_max_depth_v1(&schema, platform_version());

        let err = result.errors.first().expect("expected an error");
        assert_eq!(
            err.to_string(),
            "Invalid JSON Schema $ref: the ref '#/$defs/object' contains cycles".to_string()
        );
    }

    #[test]
    fn should_not_walk_scalar_ref_target() {
        let schema: Value = json!(
             {
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "type": "object",
                "properties": {
                  "foo": { "type": "integer" },
                  "scalarRef": { "$ref": "#/$schema" },
                },
                "additionalProperties": false,
              }
        )
        .into();

        let v0 = validate_max_depth_v0(&schema, platform_version())
            .data
            .expect("expected data");
        let v1 = validate_max_depth_v1(&schema, platform_version());
        assert!(v1.is_valid());
        assert_eq!(v1.data.expect("expected data"), v0);
    }
}
