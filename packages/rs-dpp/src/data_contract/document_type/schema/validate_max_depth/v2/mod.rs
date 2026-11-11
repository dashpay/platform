use platform_value::Value;
use platform_version::version::PlatformVersion;
use std::borrow::Cow;
use std::collections::BTreeSet;

use crate::consensus::basic::data_contract::data_contract_max_depth_exceed_error::DataContractMaxDepthExceedError;
use crate::consensus::basic::data_contract::InvalidJsonSchemaRefError;
use crate::consensus::basic::BasicError;
use crate::data_contract::document_type::schema::MaxDepthValidationResult;
use crate::data_contract::errors::DataContractError;
use crate::validation::ConsensusValidationResult;

/// Same walk as v1, except a `$ref` pointer is read as the document validator reads it, by
/// RFC 6901: each `/` separates a segment, `~1` and `~0` in a segment are `/` and `~`, and a
/// segment is one key however many `.` it holds. v1 read it through `resolve_uri`, which
/// turns the pointer into a `.`-separated path, so a key holding a `.` was split and a
/// pointer through one landed elsewhere than the validator reaches, hiding from the depth
/// and cycle check a schema the validator walks. Reading the pointer the validator's way
/// lets the check see that schema, so a `$ref` the two read differently, or one through a
/// `~` or percent escape, is refused here as it is nowhere followed. For a schema whose
/// `$ref` pointers hold no `.`, `~` or percent escape the result equals v1's.
#[inline(always)]
pub(super) fn validate_max_depth_v2(
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
                            let resolved =
                                match resolve_json_pointer(platform_value, uri).map_err(|e| {
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

/// The schema `uri`, a `$ref`, points at, read as the document validator's `$ref` resolver
/// reads it (RFC 6901), so the depth and cycle check sees the schema the validator walks.
/// `uri` is a pointer into the document schema, `#` or `#/...`, without a percent escape,
/// whitespace or control character, which the validator's resolver parses into a URL that
/// would drop or re-encode those. In each segment `~1` is `/` and `~0` is `~`, an array
/// index has no sign or leading zero, and a segment naming a repeated map key reaches its
/// last entry, as the validator reads a map.
fn resolve_json_pointer<'a>(root: &'a Value, uri: &str) -> Result<&'a Value, DataContractError> {
    let pointer = uri
        .strip_prefix('#')
        .filter(|pointer| pointer.is_empty() || pointer.starts_with('/'))
        .filter(|pointer| {
            !pointer.contains(|c: char| c == '%' || c.is_whitespace() || c.is_control())
        })
        .ok_or_else(|| {
            DataContractError::InvalidURI(format!(
                "`{uri}` is not a JSON pointer into the document schema (`#` or `#/...`, \
                 without `%`, whitespace or control characters)"
            ))
        })?;
    let unresolved = || {
        DataContractError::InvalidURI(format!(
            "`{uri}` does not resolve within the document schema"
        ))
    };
    let mut target = root;
    let Some(pointer) = pointer.strip_prefix('/') else {
        return Ok(target);
    };
    for segment in pointer.split('/') {
        let segment = if segment.contains('~') {
            Cow::Owned(segment.replace("~1", "/").replace("~0", "~"))
        } else {
            Cow::Borrowed(segment)
        };
        target = match target {
            Value::Map(map) => last_value(map, &segment).ok_or_else(unresolved)?,
            Value::Array(elements) => array_index(&segment)
                .and_then(|index| elements.get(index))
                .ok_or_else(unresolved)?,
            _ => return Err(unresolved()),
        };
    }
    Ok(target)
}

/// The value of the last entry of `map` whose key is the text `key`, as the document
/// validator reads a map keeping a repeated key's last entry; `None` when it holds none.
fn last_value<'a>(map: &'a [(Value, Value)], key: &str) -> Option<&'a Value> {
    map.iter()
        .rev()
        .find(|(name, _)| name.as_text() == Some(key))
        .map(|(_, value)| value)
}

/// An array index in a JSON pointer segment, as the document validator reads one: decimal
/// digits with no sign and no leading zero.
fn array_index(segment: &str) -> Option<usize> {
    if segment.starts_with('+') || (segment.starts_with('0') && segment.len() != 1) {
        None
    } else {
        segment.parse().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::super::{validate_max_depth, MaxDepthValidationResult};
    use super::*;
    use crate::consensus::basic::BasicError;
    use crate::consensus::ConsensusError;
    use serde_json::json;

    /// A document type schema whose `contains` holds, through a `$ref` pointer with a key
    /// that holds a `.`, a schema referring to itself through `items` and `contains`. The
    /// pointer lands on the scalar `"array"` when read as a `.`-separated path, so v1 does
    /// not see the self-reference, and on the self-referring schema when read by RFC 6901, so
    /// v2 does.
    fn dotted_self_reference() -> Value {
        let reference = "#/properties/p/contains/properties/a.type";
        json!({
            "type": "object",
            "properties": {
                "p": {
                    "type": "array",
                    "byteArray": true,
                    "maxItems": 64,
                    "contains": {
                        "items": { "$ref": reference },
                        "contains": { "$ref": reference },
                        "properties": {
                            "a.type": { "items": { "$ref": reference }, "contains": { "$ref": reference } },
                            "a": { "type": "array" }
                        }
                    }
                }
            },
            "additionalProperties": false
        })
        .into()
    }

    #[test]
    fn should_see_a_self_reference_hidden_behind_a_dotted_key_from_protocol_version_15() {
        let schema = dotted_self_reference();

        // Protocol version 14 reads the pointer as a dotted path, lands on a scalar, and does
        // not see the self-reference.
        let pv14 = validate_max_depth(
            &schema,
            PlatformVersion::get(14).expect("expected protocol version 14"),
        )
        .expect("expected a known version");
        assert!(pv14.is_valid(), "{:?}", pv14.errors);

        // Protocol version 15 reads the pointer as the validator does and refuses the cycle.
        let latest = validate_max_depth(&schema, PlatformVersion::latest())
            .expect("expected a known version");
        assert!(
            matches!(
                latest.errors.as_slice(),
                [ConsensusError::BasicError(BasicError::InvalidJsonSchemaRefError(e))]
                    if e.to_string().contains("contains cycles")
            ),
            "{:?}",
            latest.errors
        );
    }

    #[test]
    fn should_measure_a_schema_whose_refs_hold_no_special_character_as_v1_does() {
        let schema: Value = json!({
            "$defs": { "object": { "nested": { "type": "string" } } },
            "type": "object",
            "properties": {
                "foo": { "type": "integer" },
                "fooWithRef": { "$ref": "#/$defs/object" },
            },
            "additionalProperties": false,
        })
        .into();

        let pv14 = validate_max_depth(
            &schema,
            PlatformVersion::get(14).expect("expected protocol version 14"),
        )
        .expect("expected a known version");
        let latest = validate_max_depth(&schema, PlatformVersion::latest())
            .expect("expected a known version");
        assert!(pv14.is_valid(), "{:?}", pv14.errors);
        assert_eq!(pv14.data, latest.data);
        assert_eq!(
            latest.data,
            Some(MaxDepthValidationResult { depth: 5, size: 14 })
        );
    }
}
