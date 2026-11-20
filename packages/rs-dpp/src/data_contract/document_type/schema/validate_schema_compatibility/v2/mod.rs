//! Protocol v15 generation of the JSON-schema compatibility check.
//!
//! v2 is v1 with the `required` lists of a document type's inline object
//! properties left out of the diff, as v1 leaves out the type's own `required`,
//! at every depth of the inline objects below the root `properties`. Document
//! type `validate_update` v2 judges them
//! against the parsed types instead (its member rules): an object's list may
//! gain a member the same update adds with `requiredSince`, which the differ's
//! frozen `required` rule cannot express, and may lose nothing.
//!
//! Only the keyword is removed, at the schema positions that hold one: a
//! member named `required` is a property schema like any other and is still
//! diffed. The list written beside a `$ref`, and every list below one (in
//! `properties` written beside it), stays in the diff, frozen as before: the
//! object takes its members from a definition, and gains them only as optional.
//! So does a `required` on a property schema that is not an object. The
//! contract's `$defs` are
//! diffed through a wrapper with no root `properties`, so the lists of a
//! definition stay frozen: the parser never reads them.
//!
//! Pairing invariant: only a protocol version whose document type
//! `validate_update` is generation 2 or later may select this generation, since
//! that generation is what judges the lists this one leaves out.

use super::v1::validate_schema_compatibility_v1;
use crate::data_contract::document_type::property_names::{PROPERTIES, REF, REQUIRED, TYPE};
use crate::data_contract::document_type::schema::IncompatibleJsonSchemaOperation;
use crate::data_contract::JsonValue;
use crate::validation::SimpleValidationResult;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;
use serde_json::Map;
use std::borrow::Cow;

pub(super) fn validate_schema_compatibility_v2(
    original_schema: &JsonValue,
    new_schema: &JsonValue,
    platform_version: &PlatformVersion,
) -> Result<SimpleValidationResult<IncompatibleJsonSchemaOperation>, ProtocolError> {
    let original_schema = without_object_required_lists(original_schema);
    let new_schema = without_object_required_lists(new_schema);
    validate_schema_compatibility_v1(&original_schema, &new_schema, platform_version)
}

/// `schema` without the `required` keyword of the object properties below its root
/// `properties`, or `schema` itself when it has no root `properties`.
fn without_object_required_lists(schema: &JsonValue) -> Cow<'_, JsonValue> {
    let Some(JsonValue::Object(_)) = schema.get(PROPERTIES) else {
        return Cow::Borrowed(schema);
    };
    let mut schema = schema.clone();
    if let Some(JsonValue::Object(properties)) = schema.get_mut(PROPERTIES) {
        remove_member_required_lists(properties);
    }
    Cow::Owned(schema)
}

/// Removes the `required` keyword of every inline object property of `properties` (a
/// `properties` map: member name to property schema), and of the inline objects inside them.
/// A `$ref` keeps its list.
fn remove_member_required_lists(properties: &mut Map<String, JsonValue>) {
    for property in properties.values_mut() {
        let JsonValue::Object(property) = property else {
            continue;
        };
        if property.contains_key(REF) {
            continue;
        }
        if property.get(TYPE).and_then(JsonValue::as_str) == Some("object") {
            property.remove(REQUIRED);
            if let Some(JsonValue::Object(members)) = property.get_mut(PROPERTIES) {
                remove_member_required_lists(members);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn should_leave_out_only_the_required_keyword_of_object_properties() {
        let schema = json!({
            "type": "object",
            "properties": {
                "meta": {
                    "type": "object",
                    "position": 0,
                    "properties": {
                        "required": { "type": "string", "position": 0 },
                        "inner": {
                            "type": "object",
                            "position": 1,
                            "properties": { "p": { "type": "boolean", "position": 0 } },
                            "required": ["p"],
                        },
                    },
                    "required": ["required"],
                },
                "shared": {
                    "$ref": "#/$defs/m",
                    "position": 1,
                    "required": ["y"],
                    "properties": {
                        "q": { "type": "object", "position": 0, "properties": {}, "required": ["a"] },
                    },
                },
                "scalar": { "type": "string", "position": 2, "required": ["x"] },
            },
            "required": ["meta"],
        });

        let stripped = without_object_required_lists(&schema);

        assert_eq!(
            stripped.as_ref(),
            &json!({
                "type": "object",
                "properties": {
                    "meta": {
                        "type": "object",
                        "position": 0,
                        "properties": {
                            "required": { "type": "string", "position": 0 },
                            "inner": {
                                "type": "object",
                                "position": 1,
                                "properties": { "p": { "type": "boolean", "position": 0 } },
                            },
                        },
                    },
                    "shared": {
                        "$ref": "#/$defs/m",
                        "position": 1,
                        "required": ["y"],
                        "properties": {
                            "q": { "type": "object", "position": 0, "properties": {}, "required": ["a"] },
                        },
                    },
                    "scalar": { "type": "string", "position": 2, "required": ["x"] },
                },
                "required": ["meta"],
            }),
            "only the lists of inline objects go; a member named `required`, a scalar's list, \
             the list beside a $ref and below one, and the type's own list (v1 strips that) \
             stay"
        );
    }

    #[test]
    fn should_leave_the_contract_definitions_as_they_are() {
        let definitions = json!({
            "$defs": {
                "m": {
                    "type": "object",
                    "properties": { "y": { "type": "boolean" } },
                    "required": ["y"],
                }
            }
        });

        assert!(matches!(
            without_object_required_lists(&definitions),
            Cow::Borrowed(_)
        ));
    }
}
