use super::*;
#[cfg(feature = "validation")]
use crate::data_contract::document_type::property_constraints::DocumentSystemValues;
#[cfg(feature = "validation")]
use crate::data_contract::methods::validate_document::DataContractDocumentValidationMethodsV0;
use crate::tests::fixtures::get_dashpay_contract_fixture;
#[cfg(feature = "validation")]
use crate::tests::fixtures::get_data_contract_fixture;
#[cfg(feature = "validation")]
use crate::validation::JsonSchemaValidator;
use platform_value::platform_value;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use strum::IntoEnumIterator;

fn build(root: Value) -> ValidationSchema {
    ValidationSchema::try_from_root(&root).expect("expected the schema to build")
}

fn object(node: &SchemaNode) -> &ObjectConstraints {
    node.object.as_deref().expect("expected object keywords")
}

fn array(node: &SchemaNode) -> &ArrayConstraints {
    node.array.as_deref().expect("expected array keywords")
}

fn string(node: &SchemaNode) -> &StringConstraints {
    node.string.as_deref().expect("expected string keywords")
}

fn number(node: &SchemaNode) -> &NumberConstraints {
    node.number.as_deref().expect("expected number keywords")
}

fn format(node: &SchemaNode) -> &FormatConstraints {
    node.format.as_deref().expect("expected format keywords")
}

fn property<'a>(node: &'a SchemaNode, name: &str) -> &'a SchemaNode {
    object(node)
        .properties
        .as_ref()
        .expect("expected properties")
        .iter()
        .find(|(property_name, _)| property_name == name)
        .map(|(_, node)| node)
        .unwrap_or_else(|| panic!("expected a property `{name}`"))
}

/// An array property whose `contains` holds `contains`. The document type parser never
/// reads `contains`, so a `$ref` there is resolved by the JSON Schema validator alone.
fn containing(contains: Value) -> Value {
    platform_value!({
        "type": "array",
        "maxItems": 4,
        "items": { "type": "string", "maxLength": 16 },
        "contains": contains,
    })
}

/// An array property whose `contains` is `{ "$ref": reference }`.
fn containing_reference(reference: &str) -> Value {
    containing(platform_value!({ "$ref": reference }))
}

/// The error the builder refuses `root` with.
fn refusal(root: Value) -> String {
    ValidationSchema::try_from_root(&root)
        .expect_err("expected the schema to be refused")
        .to_string()
}

/// A text value.
fn text_value(text: &str) -> Value {
    Value::Text(text.to_string())
}

/// Whether the existing validator, compiling `schema` as the document validator compiles a
/// document schema, accepts `document`.
#[cfg(feature = "validation")]
fn existing_validator_accepts(schema: &serde_json::Value, document: &serde_json::Value) -> bool {
    JsonSchemaValidator::new()
        .compile_and_validate(schema, document, PlatformVersion::latest())
        .expect("expected the schema to compile")
        .is_valid()
}

/// Whether the existing validator compiles `schema` as the document validator compiles the
/// document schema of a contract loaded from state. Registration compiles it under a
/// smaller regex size limit.
#[cfg(feature = "validation")]
fn existing_validator_compiles(schema: &serde_json::Value) -> bool {
    JsonSchemaValidator::new()
        .compile_and_validate(schema, &serde_json::json!({}), PlatformVersion::latest())
        .is_ok()
}

/// `root` as the JSON the existing validator compiles.
#[cfg(feature = "validation")]
fn validating_json(root: &Value) -> serde_json::Value {
    root.try_to_validating_json()
        .expect("expected the schema to convert")
}

#[test]
fn should_keep_every_validation_keyword_of_a_node() {
    let schema = build(platform_value!({
        "type": "object",
        "properties": {
            "name": {
                "type": "string",
                "minLength": 1,
                "maxLength": 63,
                "maxBytes": 126,
                "pattern": "^[a-z]+$",
                "format": "uri",
                "enum": ["a", "b"],
            },
            "score": {
                "type": ["integer", "null"],
                "minimum": 0,
                "maximum": 100.5,
                "exclusiveMinimum": -1,
                "exclusiveMaximum": 101,
                "multipleOf": 5,
                "const": 10,
            },
            "tags": {
                "type": "array",
                "items": { "type": "string", "maxLength": 8 },
                "contains": { "type": "string", "const": "x" },
                "minItems": 1,
                "maxItems": 4,
                "uniqueItems": true,
            },
            "key": {
                "type": "array",
                "byteArray": true,
                "contentMediaType": "application/x.dash.dpp.identifier",
                "minItems": 32,
                "maxItems": 32,
            },
            "message": {
                "type": "array",
                "byteArray": true,
                "minItems": 32,
                "maxItems": 1040,
                "encryptedFor": {
                    "recipient": "key",
                    "recipientKey": "recipientKeyId",
                    "senderKey": "senderKeyId",
                    "scheme": "ecdh-secp256k1-aes256-cbc",
                },
            },
            "meta": {
                "type": "object",
                "properties": { "a": { "type": "string" } },
                "minProperties": 1,
                "maxProperties": 2,
                "dependentRequired": { "a": ["b"] },
                "additionalProperties": false,
            },
        },
        "required": ["name"],
        "additionalProperties": false,
    }));

    let root = &schema.root;
    assert_eq!(root.types, Some(JsonTypes::One(JsonType::Object)));
    assert_eq!(object(root).required, Some(vec!["name".to_string()]));
    assert_eq!(object(root).additional_properties, Some(false));

    let name = property(root, "name");
    assert_eq!(string(name).min_length, Some(1));
    assert_eq!(string(name).max_length, Some(63));
    assert_eq!(string(name).max_bytes, Some(126));
    assert_eq!(string(name).pattern.as_deref(), Some("^[a-z]+$"));
    assert_eq!(format(name).format.as_deref(), Some("uri"));
    assert_eq!(
        name.enum_values,
        Some(vec![Value::Text("a".into()), Value::Text("b".into())])
    );

    let score = property(root, "score");
    assert_eq!(
        score.types,
        Some(JsonTypes::AnyOf(vec![JsonType::Integer, JsonType::Null]))
    );
    assert!(number(score)
        .minimum
        .as_ref()
        .is_some_and(Value::is_integer));
    assert_eq!(number(score).maximum, Some(Value::Float(100.5)));
    assert!(number(score).exclusive_minimum.is_some());
    assert!(number(score).exclusive_maximum.is_some());
    assert!(number(score).multiple_of.is_some());
    assert!(score.constant.is_some());

    let tags = property(root, "tags");
    assert_eq!(
        array(tags)
            .items
            .as_deref()
            .and_then(|items| string(items).max_length),
        Some(8)
    );
    assert!(array(tags)
        .contains
        .as_deref()
        .is_some_and(|contains| contains.constant.is_some()));
    assert_eq!(
        (array(tags).min_items, array(tags).max_items),
        (Some(1), Some(4))
    );
    assert_eq!(array(tags).unique_items, Some(true));

    let key = property(root, "key");
    assert!(format(key).byte_array);
    // `contentMediaType` makes it an identifier, which constrains no value
    assert!(!key
        .to_value()
        .as_map()
        .expect("expected a map")
        .iter()
        .any(|(name, _)| name.as_text() == Some("contentMediaType")));

    let message = property(root, "message");
    let encrypted_for = format(message)
        .encrypted_for
        .as_ref()
        .expect("expected encryptedFor");
    assert_eq!(encrypted_for.recipient.to_string(), "key");
    assert_eq!(
        (
            encrypted_for.recipient_key.as_str(),
            encrypted_for.sender_key.as_str()
        ),
        ("recipientKeyId", "senderKeyId")
    );
    assert_eq!(encrypted_for.scheme.as_str(), "ecdh-secp256k1-aes256-cbc");
    assert_eq!(
        message.value_of(SchemaKeyword::Format(FormatKeyword::EncryptedFor)),
        Some(platform_value!({
            "recipient": "key",
            "recipientKey": "recipientKeyId",
            "senderKey": "senderKeyId",
            "scheme": "ecdh-secp256k1-aes256-cbc",
        }))
    );

    let meta = property(root, "meta");
    assert_eq!(
        (object(meta).min_properties, object(meta).max_properties),
        (Some(1), Some(2))
    );
    assert_eq!(
        object(meta).dependent_required,
        Some(BTreeMap::from([("a".to_string(), vec!["b".to_string()])]))
    );
}

#[test]
fn should_allocate_only_the_keyword_groups_a_node_uses() {
    let schema = build(platform_value!({
        "type": "object",
        "properties": {
            "name": { "type": "string", "maxLength": 10, "position": 0 },
            "flag": { "type": "boolean", "position": 1 },
        },
    }));

    let name = property(&schema.root, "name");
    assert!(name.string.is_some());
    assert!(name.object.is_none() && name.array.is_none());
    assert!(name.number.is_none() && name.format.is_none());

    let flag = property(&schema.root, "flag");
    assert_eq!(flag.types, Some(JsonTypes::One(JsonType::Boolean)));
    assert!(flag.object.is_none() && flag.array.is_none() && flag.string.is_none());
    assert!(flag.number.is_none() && flag.format.is_none());
}

#[test]
fn should_keep_the_keywords_beside_a_reference() {
    let schema = build(platform_value!({
        "type": "object",
        "properties": {
            "label": { "$ref": "#/$defs/label", "maxLength": 5, "position": 0 },
        },
        "$defs": { "label": { "type": "string", "minLength": 1 } },
    }));

    let label = property(&schema.root, "label");
    assert_eq!(label.reference.as_deref(), Some("#/$defs/label"));
    assert_eq!(string(label).max_length, Some(5));
    assert_eq!(
        schema
            .references
            .get("#/$defs/label")
            .map(|node| string(node).min_length),
        Some(Some(1))
    );
}

#[test]
fn should_build_each_reference_once_when_references_cycle() {
    let schema = build(platform_value!({
        "type": "object",
        "properties": { "node": { "$ref": "#/$defs/a" } },
        "$defs": {
            "a": { "type": "object", "properties": { "next": { "$ref": "#/$defs/b" } } },
            "b": { "type": "object", "properties": { "back": { "$ref": "#/$defs/a" } } },
        },
    }));

    assert_eq!(
        schema.references.keys().cloned().collect::<Vec<_>>(),
        vec!["#/$defs/a".to_string(), "#/$defs/b".to_string()]
    );
}

#[test]
fn should_resolve_escaped_and_percent_encoded_pointers() {
    let schema = build(platform_value!({
        "type": "object",
        "properties": {
            "slash": containing_reference("#/$defs/a~1b"),
            "tilde": containing_reference("#/$defs/c~0d"),
            "space": containing_reference("#/$defs/e%20f"),
        },
        "$defs": {
            "a/b": { "maxLength": 1 },
            "c~d": { "maxLength": 2 },
            "e f": { "maxLength": 3 },
        },
    }));

    for (reference, max_length) in [
        ("#/$defs/a~1b", 1),
        ("#/$defs/c~0d", 2),
        ("#/$defs/e%20f", 3),
    ] {
        assert_eq!(
            schema
                .references
                .get(reference)
                .and_then(|node| string(node).max_length),
            Some(max_length),
            "{reference}"
        );
    }
}

#[test]
fn should_decode_the_whole_fragment_before_splitting_it() {
    let schema = build(platform_value!({
        "type": "object",
        "properties": {
            "encodedSeparators": containing_reference("#/$defs/wrapper%2Fproperties%2Flabel"),
            "encodedLeadingSlash": containing_reference("#%2F$defs%2Fwrapper"),
            "lonePercent": containing_reference("#/$defs/100%"),
        },
        "$defs": {
            "wrapper": {
                "type": "object",
                "properties": { "label": { "type": "string", "maxLength": 7 } },
            },
            "wrapper/properties/label": { "type": "string", "maxLength": 1 },
            "100%": { "type": "string", "maxLength": 3 },
        },
    }));

    // `%2F` separates segments once decoded: the label inside `wrapper`, not the
    // definition whose name holds slashes.
    assert_eq!(
        schema
            .references
            .get("#/$defs/wrapper%2Fproperties%2Flabel")
            .and_then(|node| string(node).max_length),
        Some(7)
    );
    assert_eq!(
        schema
            .references
            .get("#%2F$defs%2Fwrapper")
            .map(|node| &node.types),
        Some(&Some(JsonTypes::One(JsonType::Object)))
    );
    // A `%` not followed by two hex digits is kept as written.
    assert_eq!(
        schema
            .references
            .get("#/$defs/100%")
            .and_then(|node| string(node).max_length),
        Some(3)
    );
}

#[test]
fn should_reuse_the_root_for_a_reference_to_it() {
    let schema = build(platform_value!({
        "$schema": "https://example.com/meta",
        "type": "object",
        "properties": { "child": containing_reference("#") },
        "$defs": { "unused": { "type": "string" } },
    }));

    assert_eq!(
        schema.references.get("#").map(|node| node.as_ref()),
        Some(&schema.root)
    );
}

#[test]
fn should_read_a_repeated_key_by_its_last_entry_in_the_place_of_its_first() {
    let schema = build(Value::Map(vec![
        (text_value("type"), text_value("object")),
        (
            text_value("properties"),
            Value::Map(vec![
                // Shadowed by the last `value`: an unknown type the validator never reads.
                (text_value("value"), platform_value!({ "type": "bytes" })),
                (text_value("other"), platform_value!({ "type": "boolean" })),
                (
                    text_value("value"),
                    platform_value!({ "type": "string", "maxLength": 10 }),
                ),
                (
                    text_value("label"),
                    containing(Value::Map(vec![
                        (text_value("$ref"), text_value("#/$defs/missing")),
                        (text_value("$ref"), text_value("#/$defs/label")),
                    ])),
                ),
            ]),
        ),
        (
            text_value("$defs"),
            Value::Map(vec![
                (text_value("label"), platform_value!({ "maxLength": 1 })),
                (text_value("label"), platform_value!({ "maxLength": 2 })),
            ]),
        ),
    ]));

    let names: Vec<&str> = object(&schema.root)
        .properties
        .iter()
        .flatten()
        .map(|(name, _)| name.as_str())
        .collect();
    assert_eq!(names, ["value", "other", "label"]);
    assert_eq!(string(property(&schema.root, "value")).max_length, Some(10));
    let label = property(&schema.root, "label");
    assert_eq!(
        array(label)
            .contains
            .as_ref()
            .and_then(|contains| contains.reference.as_deref()),
        Some("#/$defs/label")
    );
    assert_eq!(
        schema.references.keys().collect::<Vec<_>>(),
        ["#/$defs/label"]
    );
    assert_eq!(
        string(&schema.references["#/$defs/label"]).max_length,
        Some(2)
    );
}

#[test]
fn should_resolve_a_reference_through_the_schema_an_id_names() {
    // The validator looks a reference's URL up among the schemas an `$id` names before it
    // follows the pointer, and so does the builder
    let schema = build(platform_value!({
        "type": "object",
        "properties": {
            "named": { "$id": "#/$defs/label", "type": "string", "maxLength": 1 },
            "users": containing_reference("#/$defs/label"),
        },
        "$defs": { "label": { "type": "string", "maxLength": 10 } },
    }));
    assert_eq!(
        string(&schema.references["#/$defs/label"]).max_length,
        Some(1)
    );

    // A property's `$ref` is refused, since the parser follows the pointer instead
    let message = refusal(platform_value!({
        "type": "object",
        "properties": {
            "named": { "$id": "#/$defs/label", "type": "string", "maxLength": 1 },
            "user": { "$ref": "#/$defs/label" },
        },
        "$defs": { "label": { "type": "string", "maxLength": 10 } },
    }));
    assert!(
        message.contains("the document type parser resolves"),
        "{message}"
    );
}

#[test]
fn should_resolve_a_location_independent_reference_through_the_schema_its_id_names() {
    let schema = build(platform_value!({
        "type": "object",
        "properties": { "labels": containing_reference("#label") },
        "$defs": { "label": { "$id": "#label", "type": "string", "maxLength": 1 } },
    }));
    assert_eq!(string(&schema.references["#label"]).max_length, Some(1));
}

/// What the test above rests on: the existing validator resolves `#label` through the
/// schema whose `$id` it is.
#[cfg(feature = "validation")]
#[test]
fn should_match_the_existing_validator_on_a_location_independent_reference() {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "labels": {
                "type": "array",
                "maxItems": 4,
                "items": { "type": "string", "maxLength": 16 },
                "contains": { "$ref": "#label" },
            },
        },
        "$defs": { "label": { "$id": "#label", "type": "string", "maxLength": 1 } },
    });
    let validator_accepts = |labels: serde_json::Value| {
        existing_validator_accepts(&schema, &serde_json::json!({ "labels": labels }))
    };
    assert!(validator_accepts(serde_json::json!(["long", "a"])));
    assert!(!validator_accepts(serde_json::json!(["long"])));
}

#[test]
fn should_not_index_an_id_below_a_member_named_enum_or_const() {
    // The validator's resolver skips every member named `enum` or `const`, a property so
    // named included, so its `$id` names nothing and the reference follows its pointer
    for name in ["enum", "const"] {
        let schema = build(platform_value!({
            "type": "object",
            "properties": {
                name: { "$id": "#/$defs/label", "type": "string", "maxLength": 1 },
                "users": containing_reference("#/$defs/label"),
            },
            "$defs": { "label": { "type": "string", "maxLength": 10 } },
        }));
        assert_eq!(
            string(&schema.references["#/$defs/label"]).max_length,
            Some(10),
            "{name}"
        );
    }
}

/// What the test above rests on: the existing validator does not index an `$id` below a
/// property named `enum` or `const`.
#[cfg(feature = "validation")]
#[test]
fn should_match_the_existing_validator_on_an_id_below_a_member_named_enum_or_const() {
    for name in ["enum", "const"] {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                name: { "$id": "#/$defs/label", "type": "string", "maxLength": 1 },
                "users": {
                    "type": "array",
                    "maxItems": 4,
                    "items": { "type": "string", "maxLength": 16 },
                    "contains": { "$ref": "#/$defs/label" },
                },
            },
            "$defs": { "label": { "type": "string", "maxLength": 10 } },
        });
        let accepted =
            existing_validator_accepts(&schema, &serde_json::json!({ "users": ["hello"] }));
        assert!(
            accepted,
            "{name}: the definition's maxLength of 10 holds, not the unindexed one of 1"
        );
    }
}

#[test]
fn should_build_a_schema_that_names_a_subschema_but_reaches_no_reference() {
    let schema = build(platform_value!({
        "type": "object",
        "properties": {
            "named": { "$id": "#named", "type": "string", "maxLength": 1 },
            "choice": { "type": "string", "enum": [{ "$id": "#not-a-schema" }] },
        },
    }));
    assert_eq!(string(property(&schema.root, "named")).max_length, Some(1));
}

#[test]
fn should_build_a_schema_whose_only_ids_are_outside_its_nodes_and_not_fragments() {
    // As in a contract on testnet: a reference's `where` condition matches a document by
    // its `$id`, which is not a schema `$id`.
    let schema = build(platform_value!({
        "type": "object",
        "ownerRefersTo": { "where": { "$id": "targetId" } },
        "properties": { "repoId": { "$ref": "#/$defs/id", "position": 0 } },
        "$defs": { "id": { "type": "string", "maxLength": 3 } },
    }));
    assert_eq!(string(&schema.references["#/$defs/id"]).max_length, Some(3));
}

#[test]
fn should_refuse_a_property_reference_a_fragment_id_anywhere_names() {
    // The validator's resolver indexes an `$id` in any map, a document type keyword's too,
    // so the reference reaches the `where` map, where the parser follows the pointer
    let message = refusal(platform_value!({
        "type": "object",
        "ownerRefersTo": { "where": { "$id": "#/$defs/id" } },
        "properties": { "repoId": { "$ref": "#/$defs/id", "position": 0 } },
        "$defs": { "id": { "type": "string" } },
    }));
    assert!(
        message.contains("the document type parser resolves"),
        "{message}"
    );
}

#[test]
fn should_resolve_a_reference_through_the_schema_an_absolute_id_names() {
    // An `examples` value is searched for identifiers too, and an absolute `$id` resolving to
    // the reference's URL names the schema the reference reaches.
    let schema = build(platform_value!({
        "type": "object",
        "properties": {
            "note": {
                "type": "string",
                "maxLength": 8,
                "examples": [{ "$id": "json-schema:///#/$defs/label", "maxLength": 1 }],
            },
            "users": containing_reference("#/$defs/label"),
        },
        "$defs": { "label": { "type": "string", "maxLength": 10 } },
    }));
    assert_eq!(
        string(&schema.references["#/$defs/label"]).max_length,
        Some(1)
    );
}

#[test]
fn should_build_a_reference_no_id_names_or_rescopes() {
    let schema = build(platform_value!({
        "type": "object",
        "properties": {
            "named": { "$id": "#named", "type": "string" },
            // A fragment `$id` above a reference leaves the URL it resolves to unchanged.
            "scoped": {
                "$id": "#scoped",
                "type": "object",
                "properties": { "user": { "$ref": "#/$defs/label" } },
            },
        },
        "$defs": { "label": { "type": "string", "maxLength": 10 } },
    }));
    assert_eq!(
        string(&schema.references["#/$defs/label"]).max_length,
        Some(10)
    );
}

#[test]
fn should_build_a_reference_below_a_relative_id() {
    // A relative `$id` resolves to a `json-schema:` URL, under which the existing validator
    // gives a reference the root's URL, as if no `$id` were above it.
    let schema = build(platform_value!({
        "type": "object",
        "properties": {
            "scoped": {
                "$id": "elsewhere",
                "type": "object",
                "properties": { "user": { "$ref": "#/$defs/label" } },
            },
        },
        "$defs": { "label": { "type": "string", "maxLength": 1 } },
    }));
    assert_eq!(
        string(&schema.references["#/$defs/label"]).max_length,
        Some(1)
    );
}

#[test]
fn should_refuse_a_reference_an_absolute_id_above_it_rescopes() {
    let message = refusal(platform_value!({
        "type": "object",
        "properties": {
            "scoped": {
                "$id": "https://example.com/scoped",
                "type": "object",
                "properties": { "user": { "$ref": "#/$defs/label" } },
            },
        },
        "$defs": { "label": { "type": "string" } },
    }));
    assert!(
        message.contains("given another URL by an `$id`"),
        "{message}"
    );
}

/// What the two tests above rest on: the existing validator resolves a reference below a
/// relative `$id` from the root, and one below an absolute `$id` within the schema that
/// `$id` names.
#[cfg(feature = "validation")]
#[test]
fn should_match_the_existing_validator_in_scoping_a_reference_below_an_id() {
    let valid = |schema: &serde_json::Value, user: &str| {
        existing_validator_accepts(schema, &serde_json::json!({ "scoped": { "user": user } }))
    };

    let relative = serde_json::json!({
        "type": "object",
        "properties": {
            "scoped": {
                "$id": "elsewhere",
                "type": "object",
                "properties": { "user": { "$ref": "#/$defs/label" } },
            },
        },
        "$defs": { "label": { "type": "string", "maxLength": 1 } },
    });
    assert!(valid(&relative, "a"));
    assert!(
        !valid(&relative, "ab"),
        "the root's label holds `user` to a maxLength of 1"
    );

    let absolute = serde_json::json!({
        "type": "object",
        "properties": {
            "scoped": {
                "$id": "https://example.com/scoped",
                "type": "object",
                "properties": { "user": { "$ref": "#/$defs/label" } },
                "$defs": { "label": { "type": "string", "maxLength": 1 } },
            },
        },
        "$defs": { "label": { "type": "string", "maxLength": 10 } },
    });
    assert!(valid(&absolute, "a"));
    assert!(
        !valid(&absolute, "ab"),
        "the scoped schema's label holds `user`, not the root's"
    );
}

/// What the tests resolving a reference through the schema an `$id` names rest on: the
/// existing validator checks a `$ref` against the subschema an `$id` names, not the one its
/// pointer reaches.
#[cfg(feature = "validation")]
#[test]
fn should_match_the_existing_validator_in_resolving_a_reference_through_an_id() {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "named": { "$id": "#/$defs/label", "type": "string", "maxLength": 1 },
            "user": { "$ref": "#/$defs/label" },
        },
        "$defs": { "label": { "type": "string", "maxLength": 10 } },
    });
    let accepted = existing_validator_accepts(&schema, &serde_json::json!({ "user": "hello" }));
    assert!(
        !accepted,
        "the existing validator holds `user` to the named schema's maxLength of 1"
    );
    assert!(existing_validator_accepts(
        &schema,
        &serde_json::json!({ "user": "a" })
    ));

    let absolute_alias = serde_json::json!({
        "type": "object",
        "properties": {
            "note": {
                "type": "string",
                "examples": [{ "$id": "json-schema:///#/$defs/label", "maxLength": 1 }],
            },
            "user": { "$ref": "#/$defs/label" },
        },
        "$defs": { "label": { "type": "string", "maxLength": 10 } },
    });
    let accepted =
        existing_validator_accepts(&absolute_alias, &serde_json::json!({ "user": "hello" }));
    assert!(
        !accepted,
        "the existing validator holds `user` to the schema the absolute `$id` names"
    );
    assert!(existing_validator_accepts(
        &absolute_alias,
        &serde_json::json!({ "user": "a" })
    ));

    // Where no `$id` names the reference's URL or rescopes it, the existing validator
    // follows the pointer, as the validation schema does.
    let unredirected = serde_json::json!({
        "type": "object",
        "properties": {
            "named": { "$id": "#named", "type": "string" },
            "scoped": {
                "$id": "#scoped",
                "type": "object",
                "properties": { "user": { "$ref": "#/$defs/label" } },
            },
        },
        "$defs": { "label": { "type": "string", "maxLength": 10 } },
    });
    for (user, valid) in [("hello", true), ("hello world", false)] {
        let accepted = existing_validator_accepts(
            &unredirected,
            &serde_json::json!({ "scoped": { "user": user } }),
        );
        assert_eq!(accepted, valid, "{user}");
    }
}

#[test]
fn should_resolve_a_reference_from_the_url_the_validator_parses_it_into() {
    // The URL parse drops a tab and trims a trailing space, so both references reach `label`,
    // not the keys written as they are.
    let helper = platform_value!({
        "type": "integer",
        "examples": [{
            "label": { "maximum": 1 },
            "la\tbel": { "maximum": 10 },
            "label ": { "maximum": 20 },
        }],
    });
    let schema = build(platform_value!({
        "type": "object",
        "properties": {
            "tab": containing_reference("#/$defs/helper/examples/0/la\tbel"),
            "trailingSpace": containing_reference("#/$defs/helper/examples/0/label "),
            "absolute": containing_reference("json-schema:///#/$defs/helper/examples/0/label"),
        },
        "$defs": { "helper": helper.clone() },
    }));
    for reference in [
        "#/$defs/helper/examples/0/la\tbel",
        "#/$defs/helper/examples/0/label ",
        "json-schema:///#/$defs/helper/examples/0/label",
    ] {
        assert_eq!(
            number(&schema.references[reference])
                .maximum
                .as_ref()
                .and_then(|maximum| maximum.as_integer::<i64>()),
            Some(1),
            "{reference:?}"
        );
    }

    // An index with a sign or a leading zero, and a URL naming another document, do not
    // resolve.
    for reference in [
        "#/$defs/helper/examples/+0/label",
        "#/$defs/helper/examples/00/label",
        "other#/$defs/helper",
    ] {
        let message = refusal(platform_value!({
            "type": "object",
            "properties": { "value": containing_reference(reference) },
            "$defs": { "helper": helper.clone() },
        }));
        assert!(
            message.contains("does not resolve"),
            "{reference:?}: {message}"
        );
    }
}

/// What the test above rests on: the existing validator resolves a reference from the URL
/// it parses it into.
#[cfg(feature = "validation")]
#[test]
fn should_match_the_existing_validator_in_resolving_a_reference_from_its_url() {
    let valid = |reference: &str, bytes: serde_json::Value| {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "bytes": {
                    "type": "array",
                    "byteArray": true,
                    "maxItems": 4,
                    "contains": { "$ref": reference },
                },
            },
            "$defs": {
                "helper": {
                    "type": "integer",
                    "examples": [{
                        "label": { "maximum": 1 },
                        "la\tbel": { "maximum": 10 },
                        "label ": { "maximum": 20 },
                    }],
                },
            },
        });
        existing_validator_accepts(&schema, &serde_json::json!({ "bytes": bytes }))
    };
    for reference in [
        "#/$defs/helper/examples/0/la\tbel",
        "#/$defs/helper/examples/0/label ",
    ] {
        assert!(valid(reference, serde_json::json!([0])), "{reference:?}");
        assert!(
            !valid(reference, serde_json::json!([5])),
            "{reference:?}: the existing validator holds an element to `label`'s maximum of 1"
        );
    }
    // A reference that does not resolve matches no element.
    assert!(!valid(
        "#/$defs/helper/examples/00/label",
        serde_json::json!([0])
    ));
}

#[test]
fn should_keep_platform_keywords_where_consensus_checks_them() {
    let schema = build(platform_value!({
        "type": "object",
        "properties": {
            "note": { "type": "string", "maxLength": 63, "maxBytes": 8 },
            "name": { "$ref": "#/$defs/label" },
            "tags": {
                "type": "array",
                "maxItems": 4,
                "items": { "type": "string", "maxLength": 16, "maxBytes": 4 },
            },
            "meta": {
                "type": "object",
                "properties": { "tag": { "type": "string", "maxLength": 8, "maxBytes": 2 } },
            },
            "blob": { "$ref": "#/$defs/blob", "byteArray": true },
        },
        "$defs": {
            "label": { "type": "string", "maxLength": 63, "maxBytes": 16 },
            "blob": { "type": "array", "maxItems": 8 },
        },
    }));

    let root = &schema.root;
    assert_eq!(string(property(root, "note")).max_bytes, Some(8));
    assert_eq!(
        string(&schema.references["#/$defs/label"]).max_bytes,
        Some(16)
    );
    let tags = property(root, "tags");
    assert_eq!(
        string(array(tags).items.as_deref().expect("expected items")).max_bytes,
        Some(4)
    );
    assert_eq!(
        string(property(property(root, "meta"), "tag")).max_bytes,
        Some(2)
    );
    // The JSON Schema validator checks `byteArray` beside a `$ref`
    assert!(format(property(root, "blob")).byte_array);
}

#[test]
fn should_refuse_platform_keywords_where_consensus_does_not_check_them() {
    let encrypted_for = platform_value!({
        "recipient": "key",
        "recipientKey": "recipientKeyId",
        "senderKey": "senderKeyId",
        "scheme": "ecdh-secp256k1-aes256-cbc",
    });
    let defs = platform_value!({
        "label": { "type": "string", "maxLength": 63 },
        "bounded": { "type": "string", "maxLength": 63, "maxBytes": 4 },
        "alias": { "$ref": "#/$defs/bounded" },
        "blob": { "type": "array", "byteArray": true, "maxItems": 64 },
        "record": { "type": "object", "properties": { "tag": { "type": "string" } } },
    });
    for (case, value, place) in [
        (
            "maxBytes beside a property's $ref",
            platform_value!({ "$ref": "#/$defs/label", "type": "string", "maxBytes": 4 }),
            "beside a property's `$ref`",
        ),
        (
            "encryptedFor beside a property's $ref",
            platform_value!({ "$ref": "#/$defs/blob", "encryptedFor": encrypted_for.clone() }),
            "beside a property's `$ref`",
        ),
        (
            "maxBytes below the keywords beside a property's $ref",
            platform_value!({
                "$ref": "#/$defs/record",
                "properties": { "tag": { "type": "string", "maxBytes": 4 } },
            }),
            "does not read it",
        ),
        (
            "maxBytes reached through the $ref of a referenced property",
            platform_value!({ "$ref": "#/$defs/alias" }),
            "does not read it",
        ),
        (
            "encryptedFor on the items of an array",
            platform_value!({
                "type": "array",
                "maxItems": 4,
                "items": { "type": "array", "byteArray": true, "encryptedFor": encrypted_for.clone() },
            }),
            "does not read it",
        ),
        (
            "maxBytes inside contains",
            containing(platform_value!({ "type": "string", "maxBytes": 4 })),
            "inside `contains`",
        ),
        (
            "byteArray inside contains",
            containing(platform_value!({ "byteArray": true })),
            "inside `contains`",
        ),
        (
            "maxBytes a reference inside contains reaches",
            containing_reference("#/$defs/bounded"),
            "inside `contains`",
        ),
    ] {
        let message = refusal(platform_value!({
            "type": "object",
            "properties": { "value": value },
            "$defs": defs.clone(),
        }));
        assert!(message.contains(place), "{case}: {message}");
    }

    // A definition a property reaches, where its `maxBytes` is checked, and `contains`
    // reaches too, where it is not
    let message = refusal(platform_value!({
        "type": "object",
        "properties": {
            "name": { "$ref": "#/$defs/bounded" },
            "names": containing_reference("#/$defs/bounded"),
        },
        "$defs": defs,
    }));
    assert!(message.contains("inside `contains`"), "{message}");
}

#[test]
fn should_follow_the_parser_read_through_references_and_items() {
    // The items of the schema a property's `$ref` reaches are a typed array's elements,
    // whose `maxBytes` the parser reads
    let schema = build(platform_value!({
        "type": "object",
        "properties": { "tags": { "$ref": "#/$defs/tags" } },
        "$defs": {
            "tags": {
                "type": "array",
                "maxItems": 4,
                "items": { "type": "string", "maxLength": 8, "maxBytes": 8 },
            },
        },
    }));
    let items = array(&schema.references["#/$defs/tags"])
        .items
        .as_deref()
        .expect("expected items");
    assert_eq!(string(items).max_bytes, Some(8));

    // The properties of a schema only the validator reads, here the one a referenced
    // schema's own `$ref` reaches, are not the parser's; nor is the schema the document
    // schema's own `$ref` reaches
    for root in [
        platform_value!({
            "type": "object",
            "properties": { "record": { "$ref": "#/$defs/alias" } },
            "$defs": {
                "alias": { "$ref": "#/$defs/record" },
                "record": {
                    "type": "object",
                    "properties": { "tag": { "type": "string", "maxLength": 8, "maxBytes": 8 } },
                },
            },
        }),
        platform_value!({
            "type": "object",
            "$ref": "#/$defs/name",
            "$defs": { "name": { "type": "string", "maxLength": 8, "maxBytes": 8 } },
        }),
    ] {
        let message = refusal(root);
        assert!(
            message.contains("where the document type parser does not read it"),
            "{message}"
        );
    }
}

#[test]
fn should_refuse_a_property_reference_the_parser_resolves_to_another_schema() {
    // The parser follows a property's `$ref` as a dotted path below `#/`, unescaped, so it
    // resolves neither of the first two, and reads the first entry of a repeated key, where
    // the JSON Schema validator reads the last.
    let repeated = Value::Map(vec![
        (text_value("type"), text_value("object")),
        (
            text_value("properties"),
            platform_value!({ "value": { "$ref": "#/$defs/label" } }),
        ),
        (
            text_value("$defs"),
            Value::Map(vec![
                (
                    text_value("label"),
                    platform_value!({ "type": "string", "maxBytes": 1 }),
                ),
                (text_value("label"), platform_value!({ "type": "string" })),
            ]),
        ),
    ]);
    for (case, root, expected) in [
        (
            "escaped",
            platform_value!({
                "type": "object",
                "properties": { "value": { "$ref": "#/$defs/a~1b" } },
                "$defs": { "a/b": { "type": "string" } },
            }),
            "the document type parser does not resolve",
        ),
        (
            "the root",
            platform_value!({
                "type": "object",
                "properties": { "value": { "$ref": "#" } },
            }),
            "the document type parser does not resolve",
        ),
        (
            "a repeated definition",
            repeated,
            "the document type parser resolves",
        ),
    ] {
        let message = refusal(root);
        assert!(message.contains(expected), "{case}: {message}");
    }
}

/// A contract holding a document type `item`, whose properties are `properties`, with
/// `defs` added to its `$defs`, registered with full validation at the latest protocol
/// version.
#[cfg(feature = "validation")]
fn registered(properties: Value, defs: Value) -> DataContract {
    let platform_version = PlatformVersion::latest();
    let mut contract =
        get_data_contract_fixture(None, 0, platform_version.protocol_version).data_contract_owned();
    let mut all_defs = contract.schema_defs().cloned().unwrap_or_default();
    all_defs.extend(defs.into_btree_string_map().expect("expected a map"));
    contract
        .set_schema_defs(Some(all_defs), true, &mut Vec::new(), platform_version)
        .expect("expected the definitions to register");
    contract
        .set_document_schema(
            "item",
            platform_value!({
                "type": "object",
                "properties": properties,
                "additionalProperties": false,
            }),
            true,
            &mut Vec::new(),
            platform_version,
        )
        .expect("expected the document type to register");
    contract
}

/// Whether document validation, the JSON Schema validation and the `maxBytes` check that
/// follows it, accepts `properties` as an `item`.
#[cfg(feature = "validation")]
fn accepts(contract: &DataContract, properties: Value) -> bool {
    contract
        .validate_document_properties(
            "item",
            properties,
            &DocumentSystemValues::default(),
            PlatformVersion::latest(),
        )
        .expect("expected document validation to run")
        .is_valid()
}

/// The ground truth for the two tests above: document validation checks `maxBytes` on the
/// definition a property's `$ref` reaches, where the validation schema holds it, and
/// neither `maxBytes` beside that `$ref` nor `maxBytes` and `byteArray` inside `contains`,
/// which the validation schema refuses.
#[cfg(feature = "validation")]
#[test]
fn should_hold_platform_keywords_exactly_where_document_validation_checks_them() {
    let platform_version = PlatformVersion::latest();
    let defs = platform_value!({
        "label": { "type": "string", "maxLength": 63, "maxBytes": 4 },
        "wide": { "type": "string", "maxLength": 63 },
    });

    let contract = registered(
        platform_value!({ "name": { "$ref": "#/$defs/label", "position": 0 } }),
        defs.clone(),
    );
    assert!(accepts(&contract, platform_value!({ "name": "abcd" })));
    assert!(!accepts(&contract, platform_value!({ "name": "abcde" })));
    let schema = contract
        .document_type_validation_schema("item", platform_version)
        .expect("expected the schema to build");
    assert_eq!(
        string(&schema.references["#/$defs/label"]).max_bytes,
        Some(4)
    );

    let contract = registered(
        platform_value!({
            "name": { "$ref": "#/$defs/wide", "type": "string", "maxBytes": 4, "position": 0 },
        }),
        defs.clone(),
    );
    assert!(accepts(
        &contract,
        platform_value!({ "name": "hello world" })
    ));
    assert!(contract
        .document_type_validation_schema("item", platform_version)
        .is_err());

    for contains in [
        platform_value!({ "type": "string", "maxBytes": 1 }),
        platform_value!({ "byteArray": true }),
    ] {
        let contract = registered(
            platform_value!({
                "tags": {
                    "type": "array",
                    "maxItems": 4,
                    "items": { "type": "string", "maxLength": 16 },
                    "contains": contains,
                    "position": 0,
                },
            }),
            defs.clone(),
        );
        assert!(accepts(&contract, platform_value!({ "tags": ["hello"] })));
        assert!(contract
            .document_type_validation_schema("item", platform_version)
            .is_err());
    }
}

/// A document schema with the one property `value`.
fn with_value(value: Value) -> Value {
    platform_value!({ "type": "object", "properties": { "value": value } })
}

#[test]
fn should_keep_enum_const_and_bounds_as_the_validator_compares_them() {
    let schema = build(with_value(Value::Map(vec![
        (text_value("type"), text_value("array")),
        (text_value("enum"), Value::Bytes(vec![1, 2, 3])),
        (
            text_value("const"),
            Value::Map(vec![
                (text_value("a"), Value::U8(1)),
                (text_value("id"), Value::Identifier([7; 32])),
                (text_value("a"), Value::I32(2)),
            ]),
        ),
        (text_value("minimum"), Value::I8(-1)),
    ])));

    let value = property(&schema.root, "value");
    // Bytes are an array of integers, as the validator reads them
    assert_eq!(
        value.enum_values,
        Some(vec![Value::U64(1), Value::U64(2), Value::U64(3)])
    );
    // A repeated key keeps its last value, and an identifier is an array of integers
    assert_eq!(
        value.constant,
        Some(Value::Map(vec![
            (text_value("a"), Value::U64(2)),
            (text_value("id"), Value::Array(vec![Value::U64(7); 32])),
        ]))
    );
    assert_eq!(number(value).minimum, Some(Value::I64(-1)));

    // What the conversion cannot represent the validator never compiles
    for (case, bound) in [
        ("NaN", Value::Float(f64::NAN)),
        ("infinity", Value::Float(f64::INFINITY)),
        ("beyond 64 bits", Value::U128(u128::MAX)),
    ] {
        let message = refusal(with_value(Value::Map(vec![
            (text_value("type"), text_value("number")),
            (text_value("maximum"), bound),
        ])));
        assert!(message.contains("not a JSON value"), "{case}: {message}");
    }
}

#[test]
fn should_read_a_repeated_encrypted_for_key_by_its_last_entry() {
    let schema = build(with_value(platform_value!({
        "type": "array",
        "byteArray": true,
        "maxItems": 1040,
        "encryptedFor": Value::Map(vec![
            (text_value("recipient"), text_value("key")),
            (text_value("recipientKey"), text_value("recipientKeyId")),
            (text_value("senderKey"), text_value("senderKeyId")),
            (text_value("scheme"), text_value("not-a-scheme")),
            (text_value("scheme"), text_value("ecdh-secp256k1-aes256-cbc")),
        ]),
    })));
    let encrypted_for = format(property(&schema.root, "value"))
        .encrypted_for
        .as_ref()
        .expect("expected encryptedFor");
    assert_eq!(encrypted_for.scheme.as_str(), "ecdh-secp256k1-aes256-cbc");
}

#[test]
fn should_refuse_a_format_the_validator_does_not_know() {
    let schema = build(with_value(
        platform_value!({ "type": "string", "maxLength": 64, "format": "date-time" }),
    ));
    assert_eq!(
        format(property(&schema.root, "value")).format.as_deref(),
        Some("date-time")
    );

    for root in [
        with_value(platform_value!({ "type": "string", "maxLength": 36, "format": "uuid" })),
        platform_value!({
            "type": "object",
            "properties": { "value": { "$ref": "#/$defs/uid" } },
            "$defs": { "uid": { "type": "string", "maxLength": 36, "format": "uuid" } },
        }),
    ] {
        let message = refusal(root);
        assert!(message.contains("knows no format `uuid`"), "{message}");
    }
}

/// What the test above rests on: the existing validator compiles a referenced schema when a
/// value first reaches it, and one naming an unknown format refuses every value then.
#[cfg(feature = "validation")]
#[test]
fn should_match_the_existing_validator_on_an_unknown_format_a_reference_reaches() {
    let schema = serde_json::json!({
        "type": "object",
        "properties": { "uid": { "$ref": "#/$defs/uid" } },
        "$defs": { "uid": { "type": "string", "maxLength": 36, "format": "uuid" } },
    });
    let validator_accepts =
        |document: serde_json::Value| existing_validator_accepts(&schema, &document);
    assert!(validator_accepts(serde_json::json!({})));
    assert!(!validator_accepts(
        serde_json::json!({ "uid": "123e4567-e89b-12d3-a456-426614174000" })
    ));
}

#[test]
fn should_ignore_what_the_validator_ignores_in_a_system_property_and_at_the_root() {
    // Every document meta-schema admits any schema for a system property
    let schema = build(platform_value!({
        "type": "object",
        "documentsMutable": false,
        "properties": {
            "$createdAt": {
                "type": "integer",
                "minimum": 0,
                "default": 0,
                "deprecated": false,
                "position": 0,
            },
            "name": { "type": "string", "maxLength": 8, "position": 1 },
        },
    }));
    assert_eq!(
        number(property(&schema.root, "$createdAt"))
            .minimum
            .as_ref()
            .and_then(|minimum| minimum.as_integer::<i64>()),
        Some(0)
    );

    // but a keyword the validator compiles and the validation schema does not hold is
    // refused there and at the root
    for root in [
        platform_value!({
            "type": "object",
            "properties": { "$createdAt": { "type": "integer", "not": { "minimum": 10 } } },
        }),
        platform_value!({
            "type": "object",
            "properties": {
                "a": { "type": "string", "maxLength": 8 },
                "b": { "type": "string", "maxLength": 8 },
            },
            "not": { "required": ["a", "b"] },
        }),
    ] {
        let message = refusal(root);
        assert!(
            message.contains("the JSON Schema validator checks `not`"),
            "{message}"
        );
    }
    // An ordinary property's schema skips a key the validator ignores, as any schema does
    let schema = build(with_value(
        platform_value!({ "type": "string", "default": "x" }),
    ));
    assert_eq!(
        property(&schema.root, "value").types,
        Some(JsonTypes::One(JsonType::String))
    );
}

/// What the test above rests on: the existing validator applies an applicator written at
/// the root.
#[cfg(feature = "validation")]
#[test]
fn should_match_the_existing_validator_on_an_applicator_at_the_root() {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "a": { "type": "string", "maxLength": 8 },
            "b": { "type": "string", "maxLength": 8 },
        },
        "not": { "required": ["a", "b"] },
    });
    let validator_accepts =
        |document: serde_json::Value| existing_validator_accepts(&schema, &document);
    assert!(validator_accepts(serde_json::json!({ "a": "x" })));
    assert!(!validator_accepts(
        serde_json::json!({ "a": "x", "b": "y" })
    ));
}

#[test]
fn should_resolve_references_against_an_absolute_id_at_the_root() {
    for id in ["https://example.com/item", "json-schema:///item"] {
        let schema = build(platform_value!({
            "$id": id,
            "type": "object",
            "properties": { "label": { "$ref": "#/$defs/label" } },
            "$defs": { "label": { "type": "string", "maxLength": 1 } },
        }));
        assert_eq!(
            string(&schema.references["#/$defs/label"]).max_length,
            Some(1),
            "{id}"
        );
    }

    let message = refusal(platform_value!({
        "$id": "item",
        "type": "object",
        "properties": { "label": { "$ref": "#/$defs/label" } },
        "$defs": { "label": { "type": "string" } },
    }));
    assert!(message.contains("is not an absolute URL"), "{message}");
}

/// What the test above rests on: the existing validator resolves the references of a
/// document schema with an absolute `$id` to that schema.
#[cfg(feature = "validation")]
#[test]
fn should_match_the_existing_validator_in_resolving_against_an_id_at_the_root() {
    let schema = serde_json::json!({
        "$id": "https://example.com/item",
        "type": "object",
        "properties": { "label": { "$ref": "#/$defs/label" } },
        "$defs": { "label": { "type": "string", "maxLength": 1 } },
    });
    let validator_accepts =
        |label: &str| existing_validator_accepts(&schema, &serde_json::json!({ "label": label }));
    assert!(validator_accepts("a"));
    assert!(!validator_accepts("ab"));
}

/// A document schema with an absolute `$id` whose `contains` reaches `helper` through the
/// default URL written out: the validator compiles `helper`, and the `$ref` in it, against
/// that URL, so the `$ref` reaches the definition `label`, not the `examples` entry an `$id`
/// names under the document schema's own URL.
fn a_reference_compiled_against_another_url() -> Value {
    platform_value!({
        "$id": "https://example.com/root",
        "type": "object",
        "properties": {
            "note": {
                "type": "string",
                "maxLength": 8,
                "examples": [{
                    "$id": "https://example.com/root#/$defs/label",
                    "type": "string",
                    "maxLength": 1,
                }],
            },
            "names": containing_reference("json-schema:///#/$defs/helper"),
        },
        "$defs": {
            "helper": { "type": "string", "$ref": "#/$defs/label" },
            "label": { "type": "string", "maxLength": 10 },
        },
    })
}

#[test]
fn should_refuse_a_reference_whose_target_compiles_its_references_against_another_url() {
    let message = refusal(a_reference_compiled_against_another_url());
    assert!(
        message.contains("may compile against another URL"),
        "{message}"
    );
}

/// What the test above rests on: the existing validator holds `helper` to the definition's
/// maxLength of 10.
#[cfg(feature = "validation")]
#[test]
fn should_match_the_existing_validator_on_a_reference_compiled_against_another_url() {
    let schema = validating_json(&a_reference_compiled_against_another_url());
    assert!(existing_validator_accepts(
        &schema,
        &serde_json::json!({ "names": ["hello"] })
    ));
    assert!(!existing_validator_accepts(
        &schema,
        &serde_json::json!({ "names": ["hello world!"] })
    ));
}

#[test]
fn should_build_a_reference_whose_uncompiled_copy_sits_below_an_absolute_id() {
    // The validator never compiles `examples`, so the copy there names a schema but is
    // resolved by nothing
    let schema = build(platform_value!({
        "type": "object",
        "properties": {
            "note": {
                "type": "string",
                "maxLength": 8,
                "examples": [{ "$id": "https://example.com/copy", "$ref": "#/$defs/label" }],
            },
            "user": { "$ref": "#/$defs/label" },
        },
        "$defs": { "label": { "type": "string", "maxLength": 10 } },
    }));
    assert_eq!(
        string(&schema.references["#/$defs/label"]).max_length,
        Some(10)
    );
}

#[test]
fn should_build_a_reference_to_a_schema_whose_id_names_its_own_pointer() {
    // The validator finds the schema by its `$id` before it follows the pointer, and both
    // reach the same schema
    let schema = build(platform_value!({
        "type": "object",
        "properties": { "user": { "$ref": "#/$defs/label" } },
        "$defs": {
            "label": { "$id": "#/$defs/label", "type": "string", "maxLength": 1 },
        },
    }));
    assert_eq!(
        string(&schema.references["#/$defs/label"]).max_length,
        Some(1)
    );
}

/// What the test above rests on: the existing validator holds the reference to the schema
/// both its `$id` and its pointer reach.
#[cfg(feature = "validation")]
#[test]
fn should_match_the_existing_validator_on_a_schema_whose_id_names_its_own_pointer() {
    let schema = serde_json::json!({
        "type": "object",
        "properties": { "user": { "$ref": "#/$defs/label" } },
        "$defs": {
            "label": { "$id": "#/$defs/label", "type": "string", "maxLength": 1 },
        },
    });
    let validator_accepts =
        |user: &str| existing_validator_accepts(&schema, &serde_json::json!({ "user": user }));
    assert!(validator_accepts("a"));
    assert!(!validator_accepts("ab"));
}

#[test]
fn should_read_no_identifiers_from_enum_and_const_values_but_from_properties_so_named() {
    // `enum` and `const` values are data: the validator indexes no `$id` in them and
    // compiles no `$ref`
    let schema = build(platform_value!({
        "type": "object",
        "properties": {
            "choice": {
                "type": "object",
                "enum": [{ "$id": "https://example.com/member", "$ref": "#/$defs/label" }],
                "const": { "$id": "#/$defs/label" },
            },
            "user": { "$ref": "#/$defs/label" },
        },
        "$defs": { "label": { "type": "string", "maxLength": 10 } },
    }));
    assert_eq!(
        string(&schema.references["#/$defs/label"]).max_length,
        Some(10)
    );

    // but a property named `enum` is a schema, whose reference the validator compiles: one
    // the parser resolves elsewhere is refused there as anywhere
    let message = refusal(platform_value!({
        "type": "object",
        "properties": {
            "named": { "$id": "#/$defs/label", "type": "string", "maxLength": 1 },
            "enum": { "$ref": "#/$defs/label" },
        },
        "$defs": { "label": { "type": "string", "maxLength": 10 } },
    }));
    assert!(
        message.contains("the document type parser resolves"),
        "{message}"
    );
}

#[test]
fn should_refuse_a_platform_keyword_of_the_root_when_contains_reaches_the_root() {
    // Matched against an element inside `contains`, the root's `maxBytes` bounds nothing
    let message = refusal(platform_value!({
        "type": "object",
        "properties": {
            "name": { "type": "string", "maxLength": 63, "maxBytes": 8 },
            "names": containing_reference("#"),
        },
    }));
    assert!(message.contains("inside `contains`"), "{message}");
}

#[test]
fn should_refuse_a_repeated_property_carrying_parser_keywords() {
    // The parser reads the first entry of a repeated property name below the document
    // schema and of a repeated `properties` of the document schema, the validator the last
    let repeated_name = Value::Map(vec![
        (text_value("type"), text_value("object")),
        (
            text_value("properties"),
            Value::Map(vec![
                (
                    text_value("x"),
                    platform_value!({ "type": "string", "maxLength": 63 }),
                ),
                (
                    text_value("x"),
                    platform_value!({ "type": "string", "maxLength": 63, "maxBytes": 8 }),
                ),
            ]),
        ),
    ]);
    let repeated_properties = Value::Map(vec![
        (text_value("type"), text_value("object")),
        (
            text_value("properties"),
            platform_value!({ "name": { "type": "string", "maxLength": 63, "maxBytes": 8 } }),
        ),
        (
            text_value("properties"),
            platform_value!({ "name": { "type": "string", "maxLength": 63 } }),
        ),
    ]);
    for root in [with_value(repeated_name), repeated_properties] {
        let message = refusal(root);
        assert!(message.contains("declared more than once"), "{message}");
    }

    // and so is one whose entries may reach them through a `$ref`, in either order
    let referenced = platform_value!({ "$ref": "#/$defs/bounded", "position": 0 });
    let plain = platform_value!({ "type": "string", "maxLength": 63, "position": 0 });
    for (first, last) in [(referenced.clone(), plain.clone()), (plain, referenced)] {
        let message = refusal(platform_value!({
            "type": "object",
            "properties": {
                "meta": {
                    "type": "object",
                    "properties": Value::Map(vec![
                        (text_value("x"), first),
                        (text_value("x"), last),
                    ]),
                },
            },
            "$defs": { "bounded": { "type": "string", "maxLength": 63, "maxBytes": 8 } },
        }));
        assert!(message.contains("declared more than once"), "{message}");
    }

    // Without those keywords, the validator's last entry is the one held
    let schema = build(with_value(Value::Map(vec![
        (text_value("type"), text_value("object")),
        (
            text_value("properties"),
            Value::Map(vec![
                (text_value("x"), platform_value!({ "type": "integer" })),
                (
                    text_value("x"),
                    platform_value!({ "type": "string", "maxLength": 63 }),
                ),
            ]),
        ),
    ])));
    assert_eq!(
        string(property(property(&schema.root, "value"), "x")).max_length,
        Some(63)
    );
}

#[test]
fn should_refuse_a_reference_whose_fragment_is_not_utf8_though_an_id_names_it() {
    // The validator decodes the fragment before it looks the URL up among the named schemas
    let message = refusal(platform_value!({
        "type": "object",
        "properties": { "values": containing_reference("#%FF") },
        "$defs": { "t": { "$id": "#%FF", "type": "string", "maxLength": 5 } },
    }));
    assert!(message.contains("does not resolve"), "{message}");
}

#[test]
fn should_refuse_a_byte_array_keyword_written_false() {
    let message = refusal(with_value(
        platform_value!({ "type": "array", "byteArray": false, "maxItems": 4 }),
    ));
    assert!(message.contains("`byteArray: false`"), "{message}");
}

#[test]
fn should_refuse_a_pattern_the_validator_does_not_compile() {
    // The ECMA escapes the validator rewrites compile
    let schema = build(with_value(
        platform_value!({ "type": "string", "maxLength": 8, "pattern": "^\\d\\w\\s\\cA$" }),
    ));
    assert_eq!(
        string(property(&schema.root, "value")).pattern.as_deref(),
        Some("^\\d\\w\\s\\cA$")
    );

    // Look-around does not, under the `regex` crate
    let message = refusal(with_value(
        platform_value!({ "type": "string", "maxLength": 8, "pattern": "^(?!admin)" }),
    ));
    assert!(message.contains("compiles no `pattern`"), "{message}");

    // A pattern too large for registration's size limit compiles under the crate's default,
    // which the validator of a contract loaded from state uses; one too large for that does
    // not
    let schema = build(with_value(
        platform_value!({ "type": "string", "maxLength": 512, "pattern": PATTERN_BETWEEN_LIMITS }),
    ));
    assert_eq!(
        string(property(&schema.root, "value")).pattern.as_deref(),
        Some(PATTERN_BETWEEN_LIMITS)
    );
    let message = refusal(with_value(
        platform_value!({ "type": "string", "maxLength": 512, "pattern": PATTERN_BEYOND_LIMITS }),
    ));
    assert!(message.contains("compiles no `pattern`"), "{message}");
}

/// A pattern the `regex` crate compiles under its default size limit but not under the
/// 5 MiB one registration compiles a schema with.
const PATTERN_BETWEEN_LIMITS: &str = "^\\p{L}{180}$";

/// A pattern the `regex` crate compiles under neither size limit.
const PATTERN_BEYOND_LIMITS: &str = "^\\p{L}{400}$";

/// What the test above rests on: the existing validator compiles the rewritten ECMA escapes
/// and refuses look-around, and the validator of a contract loaded from state compiles a
/// pattern registration's size limit refuses.
#[cfg(feature = "validation")]
#[test]
fn should_match_the_existing_validator_on_patterns() {
    let schema = |pattern: &str| {
        serde_json::json!({
            "type": "object",
            "properties": { "name": { "type": "string", "pattern": pattern } },
        })
    };
    let compiles = |pattern: &str| existing_validator_compiles(&schema(pattern));
    assert!(compiles("^\\d\\w\\s\\cA$"));
    assert!(!compiles("^(?!admin)"));

    for pattern in [
        "^\\d+$",
        "\\D\\W\\S",
        "\\cz",
        "\\c1",
        "\\\\cA",
        "\\c\\cA",
        "[\\d-z]",
        "(?i)abc",
        "a{2,1}",
        "\\p{L}+",
        "\\bword\\b",
        "(?<name>x)",
        "(x)\\1",
        "(?=a)",
        "\\u0041",
    ] {
        assert_eq!(
            compiles_as_validator_pattern(pattern),
            compiles(pattern),
            "{pattern}"
        );
    }

    assert!(compiles(PATTERN_BETWEEN_LIMITS));
    assert!(JsonSchemaValidator::new()
        .compile(&schema(PATTERN_BETWEEN_LIMITS), PlatformVersion::latest())
        .is_err());
    assert!(!compiles(PATTERN_BEYOND_LIMITS));
}

#[test]
fn should_refuse_a_dependent_required_list_naming_a_property_twice() {
    let message = refusal(with_value(platform_value!({
        "type": "object",
        "properties": { "a": { "type": "string" } },
        "dependentRequired": { "a": ["b", "b"] },
    })));
    assert!(message.contains("lists a name twice"), "{message}");
}

#[test]
fn should_build_a_schema_whose_enum_or_const_data_holds_an_id_that_is_not_a_url() {
    // The validator never reads an `$id` in `enum` or `const` data
    let schema = build(platform_value!({
        "type": "object",
        "properties": {
            "note": { "type": "string", "maxLength": 5, "const": { "$id": "http://" } },
            "choice": { "type": "object", "enum": [{ "$id": "http://[::1" }] },
            "user": { "$ref": "#/$defs/label" },
        },
        "$defs": { "label": { "type": "string", "maxLength": 10 } },
    }));
    assert_eq!(
        string(&schema.references["#/$defs/label"]).max_length,
        Some(10)
    );

    // but a `$ref` reaching a schema in that data that carries such an `$id`, or sits below
    // one, is refused
    for root in references_into_data_below_an_id_that_is_not_a_url() {
        let message = refusal(root);
        assert!(message.contains("may fail to enter"), "{message}");
    }
}

/// Document schemas whose property `label` refers to a schema in `const` data that carries
/// an `$id` that is not a URL reference, or that sits below one with another `$id` above
/// it on the pointer path.
fn references_into_data_below_an_id_that_is_not_a_url() -> [Value; 2] {
    [
        platform_value!({
            "type": "object",
            "properties": {
                "kind": {
                    "type": "string",
                    "maxLength": 8,
                    "const": { "$id": "http://[::1", "type": "string", "maxLength": 8 },
                },
                "label": { "$ref": "#/properties/kind/const" },
            },
        }),
        platform_value!({
            "type": "object",
            "properties": {
                "kind": {
                    "type": "string",
                    "maxLength": 8,
                    "const": {
                        "$id": "a",
                        "x": {
                            "$id": "http://[::1",
                            "inner": { "type": "string", "maxLength": 8 },
                        },
                    },
                },
                "label": { "$ref": "#/properties/kind/const/x/inner" },
            },
        }),
    ]
}

/// What the test above rests on: the existing validator compiles both schemas but refuses
/// every document carrying `label`.
#[cfg(feature = "validation")]
#[test]
fn should_match_the_existing_validator_on_a_reference_into_data_below_an_id_that_is_not_a_url() {
    for root in references_into_data_below_an_id_that_is_not_a_url() {
        let schema = validating_json(&root);
        assert!(existing_validator_accepts(&schema, &serde_json::json!({})));
        assert!(!existing_validator_accepts(
            &schema,
            &serde_json::json!({ "label": "abc" })
        ));
    }
}

#[test]
fn should_refuse_an_id_that_is_not_a_url_without_any_reference() {
    // The validator's resolver reads every `$id` outside `enum` and `const` data whenever it
    // compiles the schema, and compiles nothing when one is not a URL reference
    for root in ids_that_are_not_urls_without_any_reference() {
        let message = refusal(root);
        assert!(message.contains("is not a URL reference"), "{message}");
    }
}

/// Document schemas holding no `$ref` and an `$id` that is not a URL reference, in an
/// `examples` entry and in a system property's schema.
fn ids_that_are_not_urls_without_any_reference() -> [Value; 2] {
    [
        platform_value!({
            "type": "object",
            "properties": {
                "n": { "type": "integer", "examples": [{ "$id": "http://[::1" }] },
            },
        }),
        platform_value!({
            "type": "object",
            "properties": { "$createdAt": { "$id": "http://[::1", "type": "integer" } },
        }),
    ]
}

/// What the test above rests on: the existing validator compiles neither schema.
#[cfg(feature = "validation")]
#[test]
fn should_match_the_existing_validator_on_an_id_that_is_not_a_url_without_any_reference() {
    for root in ids_that_are_not_urls_without_any_reference() {
        assert!(!existing_validator_compiles(&validating_json(&root)));
    }
}

#[test]
fn should_skip_keys_the_validator_ignores_in_any_reference_target() {
    let schema = build(platform_value!({
        "type": "object",
        "properties": {
            // A system property's schema, reached by a property's `$ref`
            "$createdAt": { "type": "integer", "minimum": 0, "default": 0 },
            "copy": { "$ref": "#/properties/$createdAt" },
            // An `examples` entry an `$id` names, reached from `contains`
            "notes": {
                "type": "string",
                "maxLength": 8,
                "examples": [{ "$id": "#/$defs/id", "status": "active" }],
            },
            "ids": containing_reference("#/$defs/id"),
            // A `contains` schema
            "tags": containing(platform_value!({ "type": "string", "title": "a tag" })),
        },
        "$defs": { "id": { "type": "string" } },
    }));
    assert_eq!(
        number(&schema.references["#/properties/$createdAt"])
            .minimum
            .as_ref()
            .and_then(|minimum| minimum.as_integer::<i64>()),
        Some(0)
    );
    assert!(schema.references.contains_key("#/$defs/id"));

    // and so does a definition, whose other keys the meta-schemas refuse at registration
    let schema = build(platform_value!({
        "type": "object",
        "properties": { "ids": containing_reference("#/$defs/id") },
        "$defs": { "id": { "type": "string", "default": "x" } },
    }));
    assert!(schema.references.contains_key("#/$defs/id"));
}

#[test]
fn should_refuse_a_protocol_version_before_14_and_accept_14() {
    let platform_version = PlatformVersion::get(13).expect("expected protocol version 13");
    let contract = get_dashpay_contract_fixture(None, 0, platform_version.protocol_version)
        .data_contract_owned();
    let name = contract
        .document_types()
        .keys()
        .next()
        .expect("expected a document type")
        .clone();
    let error = contract
        .document_type_validation_schema(&name, platform_version)
        .expect_err("expected protocol version 13 to be refused");
    assert!(
        error.to_string().contains("protocol version 14 and later"),
        "{error}"
    );

    let platform_version = PlatformVersion::get(14).expect("expected protocol version 14");
    get_dashpay_contract_fixture(None, 0, platform_version.protocol_version)
        .data_contract_owned()
        .document_type_validation_schema(&name, platform_version)
        .expect("expected protocol version 14 to be accepted");
}

#[test]
fn should_resolve_references_against_a_root_id_joined_to_itself() {
    // `http:foo` parses to `http://foo/` but, joined to itself as the validator enters the
    // schema, to `http://foo/foo`, the URL the schema's references resolve against
    let schema = build(platform_value!({
        "$id": "http:foo",
        "type": "object",
        "properties": { "label": { "$ref": "#/$defs/l" } },
        "$defs": { "l": { "type": "string", "maxLength": 1 } },
    }));
    assert_eq!(string(&schema.references["#/$defs/l"]).max_length, Some(1));

    let message = refusal(platform_value!({
        "$id": "http:foo",
        "type": "object",
        "properties": { "labels": containing_reference("http://foo/#/$defs/l") },
        "$defs": { "l": { "type": "string", "maxLength": 1 } },
    }));
    assert!(message.contains("does not resolve"), "{message}");
}

#[test]
fn should_share_one_node_among_the_spellings_of_a_reference() {
    let schema = build(platform_value!({
        "type": "object",
        "properties": {
            "a": containing_reference("#/$defs/label"),
            "b": containing_reference("#/$defs/%6Cabel"),
        },
        "$defs": { "label": { "type": "string", "maxLength": 3 } },
    }));
    assert!(Arc::ptr_eq(
        &schema.references["#/$defs/label"],
        &schema.references["#/$defs/%6Cabel"]
    ));
}

#[test]
fn should_name_every_json_type_and_read_it_back() {
    for json_type in JsonType::iter() {
        assert_eq!(JsonType::from_name(json_type.name()), Some(json_type));
    }
    assert_eq!(JsonType::Integer.name(), "integer");
    assert_eq!(JsonType::from_name("Integer"), None);
}

/// Ties [`VALIDATOR_FORMATS`] and [`VALIDATOR_KEYWORDS`] to the existing validator: it
/// compiles exactly the formats the table holds, and ignores the draft 2020-12 keywords the
/// table leaves out.
#[cfg(feature = "validation")]
#[test]
fn should_hold_the_formats_and_keywords_the_existing_validator_compiles() {
    for format_name in [
        "date",
        "date-time",
        "duration",
        "email",
        "hostname",
        "idn-email",
        "idn-hostname",
        "ipv4",
        "ipv6",
        "iri",
        "iri-reference",
        "json-pointer",
        "regex",
        "relative-json-pointer",
        "time",
        "uri",
        "uri-reference",
        "uri-template",
        "uuid",
    ] {
        let compiles = existing_validator_compiles(
            &serde_json::json!({ "type": "string", "format": format_name }),
        );
        assert_eq!(
            compiles,
            VALIDATOR_FORMATS.contains(&format_name),
            "{format_name}"
        );
    }

    for (keyword, value, instance) in [
        (
            "unevaluatedItems",
            serde_json::json!(false),
            serde_json::json!([1]),
        ),
        (
            "contentEncoding",
            serde_json::json!("base64"),
            serde_json::json!("!"),
        ),
        (
            "contentMediaType",
            serde_json::json!("application/json"),
            serde_json::json!("{"),
        ),
        (
            "$dynamicRef",
            serde_json::json!("#nowhere"),
            serde_json::json!(1),
        ),
        (
            "$recursiveRef",
            serde_json::json!("#nowhere"),
            serde_json::json!(1),
        ),
        (
            "contentSchema",
            serde_json::json!(false),
            serde_json::json!("1"),
        ),
    ] {
        assert!(!VALIDATOR_KEYWORDS.contains(&keyword), "{keyword}");
        assert!(
            existing_validator_accepts(&serde_json::json!({ keyword: value }), &instance),
            "{keyword}"
        );
    }

    // Each keyword the table holds refuses an instance it constrains, nested below the root
    let mut probed = BTreeSet::new();
    for (keyword, schema, instance) in [
        (
            "$ref",
            serde_json::json!({ "$ref": "#/nowhere" }),
            serde_json::json!(1),
        ),
        (
            "additionalItems",
            serde_json::json!({ "items": [true], "additionalItems": false }),
            serde_json::json!([1, 2]),
        ),
        (
            "additionalProperties",
            serde_json::json!({ "additionalProperties": false }),
            serde_json::json!({ "a": 1 }),
        ),
        (
            "allOf",
            serde_json::json!({ "allOf": [false] }),
            serde_json::json!(1),
        ),
        (
            "anyOf",
            serde_json::json!({ "anyOf": [false] }),
            serde_json::json!(1),
        ),
        (
            "const",
            serde_json::json!({ "const": 1 }),
            serde_json::json!(2),
        ),
        (
            "contains",
            serde_json::json!({ "contains": false }),
            serde_json::json!([1]),
        ),
        (
            "dependencies",
            serde_json::json!({ "dependencies": { "a": ["b"] } }),
            serde_json::json!({ "a": 1 }),
        ),
        (
            "dependentRequired",
            serde_json::json!({ "dependentRequired": { "a": ["b"] } }),
            serde_json::json!({ "a": 1 }),
        ),
        (
            "dependentSchemas",
            serde_json::json!({ "dependentSchemas": { "a": false } }),
            serde_json::json!({ "a": 1 }),
        ),
        (
            "else",
            serde_json::json!({ "if": false, "else": false }),
            serde_json::json!(1),
        ),
        (
            "enum",
            serde_json::json!({ "enum": [1] }),
            serde_json::json!(2),
        ),
        (
            "exclusiveMaximum",
            serde_json::json!({ "exclusiveMaximum": 0 }),
            serde_json::json!(1),
        ),
        (
            "exclusiveMinimum",
            serde_json::json!({ "exclusiveMinimum": 0 }),
            serde_json::json!(-1),
        ),
        (
            "format",
            serde_json::json!({ "format": "ipv4" }),
            serde_json::json!("x"),
        ),
        (
            "if",
            serde_json::json!({ "if": true, "then": false }),
            serde_json::json!(1),
        ),
        (
            "items",
            serde_json::json!({ "items": false }),
            serde_json::json!([1]),
        ),
        (
            "maxContains",
            serde_json::json!({ "contains": true, "maxContains": 0 }),
            serde_json::json!([1]),
        ),
        (
            "maxItems",
            serde_json::json!({ "maxItems": 0 }),
            serde_json::json!([1]),
        ),
        (
            "maxLength",
            serde_json::json!({ "maxLength": 0 }),
            serde_json::json!("x"),
        ),
        (
            "maxProperties",
            serde_json::json!({ "maxProperties": 0 }),
            serde_json::json!({ "a": 1 }),
        ),
        (
            "maximum",
            serde_json::json!({ "maximum": 0 }),
            serde_json::json!(1),
        ),
        (
            "minContains",
            serde_json::json!({ "contains": { "const": 1 }, "minContains": 2 }),
            serde_json::json!([1]),
        ),
        (
            "minItems",
            serde_json::json!({ "minItems": 1 }),
            serde_json::json!([]),
        ),
        (
            "minLength",
            serde_json::json!({ "minLength": 1 }),
            serde_json::json!(""),
        ),
        (
            "minProperties",
            serde_json::json!({ "minProperties": 1 }),
            serde_json::json!({}),
        ),
        (
            "minimum",
            serde_json::json!({ "minimum": 1 }),
            serde_json::json!(0),
        ),
        (
            "multipleOf",
            serde_json::json!({ "multipleOf": 2 }),
            serde_json::json!(1),
        ),
        (
            "not",
            serde_json::json!({ "not": true }),
            serde_json::json!(1),
        ),
        (
            "oneOf",
            serde_json::json!({ "oneOf": [false] }),
            serde_json::json!(1),
        ),
        (
            "pattern",
            serde_json::json!({ "pattern": "^a$" }),
            serde_json::json!("b"),
        ),
        (
            "patternProperties",
            serde_json::json!({ "patternProperties": { "^a$": false } }),
            serde_json::json!({ "a": 1 }),
        ),
        (
            "prefixItems",
            serde_json::json!({ "prefixItems": [false] }),
            serde_json::json!([1]),
        ),
        (
            "properties",
            serde_json::json!({ "properties": { "a": false } }),
            serde_json::json!({ "a": 1 }),
        ),
        (
            "propertyNames",
            serde_json::json!({ "propertyNames": false }),
            serde_json::json!({ "a": 1 }),
        ),
        (
            "required",
            serde_json::json!({ "required": ["a"] }),
            serde_json::json!({}),
        ),
        (
            "then",
            serde_json::json!({ "if": true, "then": false }),
            serde_json::json!(1),
        ),
        (
            "type",
            serde_json::json!({ "type": "string" }),
            serde_json::json!(1),
        ),
        (
            "unevaluatedProperties",
            serde_json::json!({ "unevaluatedProperties": false }),
            serde_json::json!({ "a": 1 }),
        ),
        (
            "uniqueItems",
            serde_json::json!({ "uniqueItems": true }),
            serde_json::json!([1, 1]),
        ),
    ] {
        assert!(VALIDATOR_KEYWORDS.contains(&keyword), "{keyword}");
        probed.insert(keyword);
        assert!(
            !existing_validator_accepts(
                &serde_json::json!({ "type": "object", "properties": { "v": schema } }),
                &serde_json::json!({ "v": instance }),
            ),
            "{keyword}"
        );
    }
    assert_eq!(
        probed,
        VALIDATOR_KEYWORDS.into_iter().collect::<BTreeSet<_>>()
    );
}

#[test]
fn should_refuse_a_reference_that_does_not_resolve() {
    let message = refusal(platform_value!({
        "type": "object",
        "properties": { "a": { "$ref": "#/$defs/missing" } },
    }));
    assert!(message.contains("does not resolve"), "{message}");
}

#[test]
fn should_ignore_the_document_type_keywords_at_the_root() {
    let schema = build(platform_value!({
        "$schema": "https://example.com/meta",
        "type": "object",
        "indices": [],
        "documentsMutable": false,
        "transient": ["a"],
        "properties": { "a": { "type": "string" } },
        "additionalProperties": false,
    }));

    assert_eq!(schema.root.types, Some(JsonTypes::One(JsonType::Object)));
}

#[test]
fn should_ignore_annotations_and_platform_keywords_below_the_root() {
    let schema = build(platform_value!({
        "type": "object",
        "properties": {
            "a": {
                "type": "string",
                "position": 0,
                "description": "a name",
                "$comment": "kept short",
                "requiredSince": 2,
            },
        },
    }));

    assert_eq!(
        property(&schema.root, "a").types,
        Some(JsonTypes::One(JsonType::String))
    );
}

#[test]
fn should_refuse_a_keyword_the_validator_compiles_below_the_root() {
    let message = refusal(platform_value!({
        "type": "object",
        "properties": {
            "a": { "type": "string", "oneOf": [{ "maxLength": 1 }, { "minLength": 3 }] },
        },
    }));
    assert!(
        message.contains("the JSON Schema validator checks `oneOf`"),
        "{message}"
    );
}

#[test]
fn should_refuse_what_the_validators_meta_schema_refuses_at_the_root() {
    for (name, root) in roots_the_validators_meta_schema_refuses() {
        let message = refusal(root);
        assert!(message.contains("meta-schema refuses"), "{name}: {message}");
    }

    // Below the root, the validator compiles a `required` list naming a property twice
    let schema = build(with_value(platform_value!({
        "type": "object",
        "properties": { "a": { "type": "string" } },
        "required": ["a", "a"],
    })));
    assert_eq!(
        object(property(&schema.root, "value")).required,
        Some(vec!["a".to_string(), "a".to_string()])
    );
}

/// A document schema with one property, `a`, and `name` valued `value`.
fn a_root_with(name: &str, value: Value) -> Value {
    Value::Map(vec![
        (
            text_value("properties"),
            platform_value!({ "a": { "type": "string" } }),
        ),
        (text_value(name), value),
    ])
}

/// Document schemas the draft 2020-12 meta-schema refuses at the root, each with the key
/// it refuses. A root `$id` with a trailing space before its fragment is one: the
/// validator's `uri-reference` format refuses whitespace.
fn roots_the_validators_meta_schema_refuses() -> Vec<(&'static str, Value)> {
    [
        ("required", platform_value!(["a", "a"])),
        ("type", platform_value!(["object", "object"])),
        ("type", platform_value!([])),
        ("title", platform_value!(5)),
        ("description", platform_value!(true)),
        ("$comment", platform_value!(5)),
        ("examples", platform_value!({})),
        ("multipleOf", platform_value!(0)),
        ("multipleOf", platform_value!(-1)),
        ("minContains", platform_value!(-1)),
        ("maxContains", platform_value!(1.5)),
        ("deprecated", platform_value!("x")),
        ("readOnly", platform_value!("x")),
        ("writeOnly", platform_value!("x")),
        ("contentMediaType", platform_value!(5)),
        ("contentEncoding", platform_value!(5)),
        ("definitions", platform_value!(1)),
        ("$defs", platform_value!(1)),
        ("$vocabulary", platform_value!({ "a": true })),
        ("$schema", platform_value!("x")),
        ("$anchor", platform_value!("1a")),
        ("$dynamicAnchor", platform_value!("1a")),
        ("$recursiveAnchor", platform_value!("1a")),
        ("$id", platform_value!(5)),
        ("$id", platform_value!("urn:a #")),
        ("$id", platform_value!("https://example.com/a b")),
        ("$ref", platform_value!("a b")),
        ("$dynamicRef", platform_value!("a b")),
        ("$recursiveRef", platform_value!("a b")),
    ]
    .into_iter()
    .map(|(name, value)| (name, a_root_with(name, value)))
    .collect()
}

/// What the test above rests on: the existing validator compiles none of those roots, but a
/// repeated `required` name below the root.
#[cfg(feature = "validation")]
#[test]
fn should_match_the_existing_validator_on_what_its_meta_schema_refuses_at_the_root() {
    for (name, root) in roots_the_validators_meta_schema_refuses() {
        assert!(
            !existing_validator_compiles(&validating_json(&root)),
            "{name}"
        );
    }
    let nested = serde_json::json!({
        "type": "object",
        "properties": {
            "value": {
                "type": "object",
                "properties": { "a": { "type": "string" } },
                "required": ["a", "a"],
            },
        },
    });
    assert!(existing_validator_accepts(
        &nested,
        &serde_json::json!({ "value": { "a": "x" } })
    ));
}

#[test]
fn should_keep_required_names_that_no_property_carries() {
    let schema = build(platform_value!({
        "type": "object",
        "properties": { "a": { "type": "string" } },
        "required": ["a", "missing"],
    }));

    assert_eq!(
        object(&schema.root).required,
        Some(vec!["a".to_string(), "missing".to_string()])
    );
}

#[test]
fn should_name_every_keyword_once_and_read_it_back() {
    let mut names = BTreeSet::new();
    for keyword in SchemaKeyword::iter() {
        assert!(names.insert(keyword.name()), "{keyword:?} shares its name");
        assert_eq!(SchemaKeyword::from_name(keyword.name()), Some(keyword));
    }
    for (keyword, name) in [
        (SchemaKeyword::Any(AnyKeyword::Ref), "$ref"),
        (SchemaKeyword::Meta(MetaKeyword::Id), "$id"),
        (
            SchemaKeyword::Object(ObjectKeyword::AdditionalProperties),
            "additionalProperties",
        ),
        (
            SchemaKeyword::Number(NumberKeyword::MultipleOf),
            "multipleOf",
        ),
        (
            SchemaKeyword::Meta(MetaKeyword::ContentMediaType),
            "contentMediaType",
        ),
        (
            SchemaKeyword::Meta(MetaKeyword::RequiredSince),
            "requiredSince",
        ),
        (
            SchemaKeyword::Relationship(RelationshipKeyword::DistinctFrom),
            "distinctFrom",
        ),
    ] {
        assert_eq!(keyword.name(), name);
    }
}

#[test]
fn should_classify_every_keyword_a_document_meta_schema_admits() {
    let mut admitted = BTreeSet::new();
    let mut admitted_at_the_root = BTreeSet::new();
    let mut versions = 0;
    let meta_schemas = Path::new(env!("CARGO_MANIFEST_DIR")).join("schema/meta_schemas/document");
    for entry in fs::read_dir(&meta_schemas).expect("expected the document meta-schemas") {
        let path = entry
            .expect("expected a meta-schema directory")
            .path()
            .join("document-meta.json");
        if !path.is_file() {
            continue;
        }
        let meta_schema: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(&path).expect("expected the meta-schema to read"),
        )
        .expect("expected the meta-schema to be JSON");
        let document_schema = meta_schema["$defs"]["documentSchema"]["properties"]
            .as_object()
            .unwrap_or_else(|| panic!("expected `documentSchema` in {}", path.display()));
        admitted.extend(document_schema.keys().cloned());
        // From v3
        if let Some(properties) =
            meta_schema["$defs"]["documentArrayItem"]["properties"].as_object()
        {
            admitted.extend(properties.keys().cloned());
        }
        if let Some(properties) = meta_schema["properties"].as_object() {
            admitted_at_the_root.extend(properties.keys().cloned());
        }
        versions += 1;
    }
    assert!(
        versions >= 4,
        "expected the document meta-schemas from v0, found {versions}"
    );

    let unclassified: Vec<_> = admitted
        .iter()
        .filter(|keyword| SchemaKeyword::from_name(keyword).is_none())
        .collect();
    assert!(
        unclassified.is_empty(),
        "keywords a document meta-schema admits below the root that `SchemaKeyword` does not classify, which the builder would skip unread: {unclassified:?}"
    );

    // At the root, a key the validator compiles must be a keyword the validation schema holds
    let unheld_at_the_root: Vec<_> = admitted_at_the_root
        .iter()
        .filter(|key| {
            VALIDATOR_KEYWORDS.contains(&key.as_str()) && SchemaKeyword::from_name(key).is_none()
        })
        .collect();
    assert!(
        unheld_at_the_root.is_empty(),
        "keys a document meta-schema admits at the root that the validator compiles but the validation schema does not hold: {unheld_at_the_root:?}"
    );
}

#[test]
fn should_build_the_validation_schema_of_a_contract_document_type() {
    let platform_version = PlatformVersion::latest();
    let contract = get_dashpay_contract_fixture(None, 0, platform_version.protocol_version)
        .data_contract_owned();

    for (name, document_type) in contract.document_types() {
        let schema = contract
            .document_type_validation_schema(name, platform_version)
            .unwrap_or_else(|error| panic!("expected `{name}` to build: {error}"));
        let property_names: BTreeSet<&str> = object(&schema.root)
            .properties
            .iter()
            .flatten()
            .map(|(name, _)| name.as_str())
            .collect();
        let document_type_property_names: BTreeSet<&str> = document_type
            .properties()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(property_names, document_type_property_names, "{name}");
    }
}

/// A `$ref` into `const` data through an `$id` that resolves to a URL with no base, then a
/// relative `$id`: the validator joins the second onto the first and fails.
fn a_reference_into_data_below_an_opaque_id() -> Value {
    platform_value!({
        "type": "object",
        "properties": {
            "kind": {
                "type": "object",
                "const": {
                    "$id": "a",
                    "x": {
                        "$id": "json-schema:opaque",
                        "y": {
                            "$id": "b",
                            "inner": { "type": "string", "maxLength": 8 },
                        },
                    },
                },
            },
            "label": { "$ref": "#/properties/kind/const/x/y/inner" },
        },
    })
}

#[test]
fn should_refuse_a_reference_into_data_below_an_id_with_no_base() {
    let message = refusal(a_reference_into_data_below_an_opaque_id());
    assert!(message.contains("may fail to enter"), "{message}");
}

/// What the test above rests on: the existing validator refuses every document carrying
/// `label`.
#[cfg(feature = "validation")]
#[test]
fn should_match_the_existing_validator_on_a_reference_into_data_below_an_id_with_no_base() {
    let schema = validating_json(&a_reference_into_data_below_an_opaque_id());
    assert!(existing_validator_accepts(&schema, &serde_json::json!({})));
    assert!(!existing_validator_accepts(
        &schema,
        &serde_json::json!({ "label": "a" })
    ));
}

#[test]
fn should_refuse_a_required_repeated_with_other_names() {
    let repeated = |first: Value, last: Value| {
        Value::Map(vec![
            (text_value("type"), text_value("object")),
            (
                text_value("properties"),
                platform_value!({ "a": { "type": "string" }, "b": { "type": "string" } }),
            ),
            (text_value("required"), first),
            (text_value("required"), last),
        ])
    };
    for root in [
        repeated(platform_value!(["a", "b"]), platform_value!(["a"])),
        with_value(repeated(
            platform_value!(["a", "b"]),
            platform_value!(["a"]),
        )),
    ] {
        let message = refusal(root);
        assert!(
            message.contains("`required` is declared more than once"),
            "{message}"
        );
    }

    let schema = build(repeated(platform_value!(["a"]), platform_value!(["a"])));
    assert_eq!(object(&schema.root).required, Some(vec!["a".to_string()]));
}

#[test]
fn should_skip_a_keyword_the_validator_compiles_only_beside_another_where_it_stands_alone() {
    let schema = build(platform_value!({
        "type": "object",
        "properties": {
            "$createdAt": { "type": "integer", "then": false },
            "a": { "type": "string", "if": { "maxLength": 1 } },
            "b": { "type": "string", "else": false },
            "c": {
                "type": "array",
                "items": { "type": "string" },
                "minContains": 2,
                "maxContains": 3,
                "additionalItems": false,
            },
        },
    }));
    assert_eq!(
        property(&schema.root, "a").types,
        Some(JsonTypes::One(JsonType::String))
    );

    for (keyword, value) in [
        (
            "if",
            platform_value!({ "if": { "maxLength": 1 }, "then": false }),
        ),
        (
            "minContains",
            platform_value!({ "minContains": 2, "contains": { "type": "string" } }),
        ),
        (
            "additionalItems",
            platform_value!({ "additionalItems": false, "items": [{ "type": "string" }] }),
        ),
        (
            "additionalItems",
            platform_value!({ "additionalItems": { "type": "string" }, "items": false }),
        ),
    ] {
        let message = refusal(with_value(value));
        assert!(
            message.contains(&format!("the JSON Schema validator checks `{keyword}`")),
            "{message}"
        );
    }
}

/// What the test above rests on: the existing validator ignores each of those keywords
/// standing alone.
#[cfg(feature = "validation")]
#[test]
fn should_match_the_existing_validator_on_a_keyword_standing_alone() {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "$createdAt": { "type": "integer", "then": false },
            "b": { "type": "string", "else": false },
            "c": {
                "type": "array",
                "items": { "type": "string" },
                "minContains": 2,
                "additionalItems": false,
            },
        },
    });
    assert!(existing_validator_accepts(
        &schema,
        &serde_json::json!({ "$createdAt": 1, "b": "x", "c": ["x", "y"] })
    ));
}

#[test]
fn should_build_a_repeated_root_property_whose_last_entry_the_parser_keeps() {
    let root = |entries: Vec<(&str, Value)>| {
        Value::Map(vec![
            (text_value("type"), text_value("object")),
            (
                text_value("properties"),
                Value::Map(
                    entries
                        .into_iter()
                        .map(|(name, schema)| (text_value(name), schema))
                        .collect(),
                ),
            ),
        ])
    };
    let entry = |position: u64, max_bytes: u64| platform_value!({ "type": "string", "maxBytes": max_bytes, "position": position });
    let other = platform_value!({ "type": "string", "position": 0 });

    // The parser orders the document schema's properties by position, keeping the order of
    // equal ones, and keeps the last entry of a name
    let schema = build(root(vec![("a", entry(0, 4)), ("a", entry(0, 8))]));
    assert_eq!(string(property(&schema.root, "a")).max_bytes, Some(8));
    build(root(vec![
        ("b", other.clone()),
        ("a", entry(0, 4)),
        ("a", entry(1, 8)),
    ]));

    let message = refusal(root(vec![
        ("a", entry(1, 4)),
        ("b", other),
        ("a", entry(0, 8)),
    ]));
    assert!(
        message.contains("`a` is declared more than once"),
        "{message}"
    );
}

/// Document schemas holding at the root a value of another type than the JSON the
/// validator compiles: bytes, which it reads as an array of integers.
fn roots_holding_bytes() -> [Value; 2] {
    [
        a_root_with("examples", Value::Bytes(vec![1, 2])),
        a_root_with("required", Value::Bytes(vec![])),
    ]
}

#[test]
fn should_read_the_root_as_the_json_the_validator_compiles() {
    let [examples, required] = roots_holding_bytes();
    build(examples);
    let schema = build(required);
    assert_eq!(object(&schema.root).required, Some(vec![]));
}

/// What the test above rests on: the existing validator compiles both.
#[cfg(feature = "validation")]
#[test]
fn should_match_the_existing_validator_on_a_root_holding_bytes() {
    for root in roots_holding_bytes() {
        assert!(existing_validator_compiles(&validating_json(&root)));
    }
}

#[test]
fn should_refuse_a_reference_an_id_may_rescope_before_resolving_it() {
    for root in [
        // The validator resolves `b` through the `$id` above it, to `a`'s own `c`
        platform_value!({
            "type": "object",
            "properties": {
                "a": {
                    "$id": "https://example.com/a",
                    "type": "object",
                    "properties": {
                        "b": { "$ref": "#/properties/c" },
                        "c": { "type": "string" },
                    },
                },
            },
        }),
        // Under an absolute document schema `$id`, a relative `$id` gives a new URL
        platform_value!({
            "$id": "https://example.com/root",
            "type": "object",
            "properties": {
                "p": {
                    "$id": "sub/",
                    "type": "object",
                    "properties": { "u": { "$ref": "#/$defs/l" } },
                },
            },
            "$defs": { "l": { "type": "string" } },
        }),
    ] {
        let message = refusal(root);
        assert!(
            message.contains("may be given another URL by an `$id` above it"),
            "{message}"
        );
    }
}

/// A property named `const` carrying an `$id` that is not a URL reference: the validator,
/// compiling the property with the document schema, fails to enter it.
fn a_property_named_const_with_an_id_that_is_not_a_url() -> Value {
    platform_value!({
        "type": "object",
        "properties": {
            "const": { "$id": "http://[::1", "type": "string", "maxLength": 8 },
        },
    })
}

#[test]
fn should_refuse_a_property_named_const_with_an_id_that_is_not_a_url() {
    let message = refusal(a_property_named_const_with_an_id_that_is_not_a_url());
    assert!(message.contains("may fail to enter"), "{message}");
    assert!(!message.contains("`$ref`"), "{message}");
}

/// What the test above rests on: the existing validator compiles nothing for it.
#[cfg(feature = "validation")]
#[test]
fn should_match_the_existing_validator_on_a_property_named_const_with_an_id_that_is_not_a_url() {
    assert!(!existing_validator_compiles(&validating_json(
        &a_property_named_const_with_an_id_that_is_not_a_url()
    )));
}

#[test]
fn should_name_the_reference_that_reached_a_refused_schema() {
    let message = refusal(platform_value!({
        "type": "object",
        "properties": {
            "name": { "$ref": "#/$defs/bounded" },
            "names": containing_reference("#/$defs/bounded"),
        },
        "$defs": { "bounded": { "type": "string", "maxBytes": 4 } },
    }));
    assert!(
        message.contains("`maxBytes` is read inside `contains`"),
        "{message}"
    );
    assert!(
        message.contains("(reached through `$ref` `#/$defs/bounded`)"),
        "{message}"
    );
}

/// What the comparisons the documentation of [`SchemaNode::constant`],
/// [`SchemaNode::enum_values`], [`ArrayConstraints::unique_items`] and
/// [`NumberConstraints::multiple_of`] describes rest on.
#[cfg(feature = "validation")]
#[test]
fn should_match_the_existing_validator_on_the_comparisons_the_documentation_describes() {
    let accepts = |schema: serde_json::Value, value: serde_json::Value| {
        existing_validator_accepts(
            &serde_json::json!({ "type": "object", "properties": { "value": schema } }),
            &serde_json::json!({ "value": value }),
        )
    };

    // A number `const` matches within an absolute f64::EPSILON
    assert!(accepts(
        serde_json::json!({ "const": 9007199254740993u64 }),
        serde_json::json!(9007199254740992u64)
    ));
    assert!(accepts(
        serde_json::json!({ "const": 0 }),
        serde_json::json!(1e-300)
    ));
    assert!(!accepts(
        serde_json::json!({ "const": 0 }),
        serde_json::json!(1e-10)
    ));

    // A fractional `multipleOf` passes every negative quotient, and decides exactly when the
    // quotient overflows
    assert!(accepts(
        serde_json::json!({ "multipleOf": 0.5 }),
        serde_json::json!(-0.3)
    ));
    assert!(!accepts(
        serde_json::json!({ "multipleOf": 0.5 }),
        serde_json::json!(0.3)
    ));
    assert!(!accepts(
        serde_json::json!({ "multipleOf": 0.3 }),
        serde_json::json!(-8.98846567431158e307) // -2^1023
    ));
    assert!(accepts(
        serde_json::json!({ "multipleOf": 0.1 }),
        serde_json::json!(1.5e308)
    ));

    // `uniqueItems` compares up to 15 elements by value and more by their JSON form
    let elements = |count: u64| {
        let mut elements = vec![serde_json::json!(1), serde_json::json!(1.0)];
        elements.extend((2..count).map(|element| serde_json::json!(element)));
        serde_json::Value::Array(elements)
    };
    assert!(!accepts(
        serde_json::json!({ "uniqueItems": true }),
        elements(15)
    ));
    assert!(accepts(
        serde_json::json!({ "uniqueItems": true }),
        elements(16)
    ));

    // `enum` compares objects key by key in the order the keys are written
    let enumerated = |member: Value| {
        accepts(
            serde_json::json!({ "enum": [validating_json(&member)] }),
            serde_json::json!({ "a": 1, "b": 2 }),
        )
    };
    assert!(enumerated(Value::Map(vec![
        (text_value("a"), Value::U64(1)),
        (text_value("b"), Value::U64(2)),
    ])));
    assert!(!enumerated(Value::Map(vec![
        (text_value("b"), Value::U64(2)),
        (text_value("a"), Value::U64(1)),
    ])));
}

/// Every key a draft 2020-12 vocabulary bundled with DPP checks at the root, written there
/// with values of each JSON type: whenever the existing validator compiles nothing for the
/// document schema, the builder refuses it.
#[cfg(feature = "validation")]
#[test]
fn should_refuse_every_root_the_existing_validator_compiles_nothing_for() {
    let draft = Path::new(env!("CARGO_MANIFEST_DIR")).join("schema/meta_schemas/draft2020-12");
    let mut paths: Vec<_> = fs::read_dir(draft.join("meta"))
        .expect("expected the draft 2020-12 vocabularies")
        .map(|entry| entry.expect("expected a vocabulary").path())
        .collect();
    paths.push(draft.join("schema.json"));
    let mut keys = BTreeSet::new();
    for path in &paths {
        let meta_schema: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(path).expect("expected the vocabulary to read"),
        )
        .expect("expected the vocabulary to be JSON");
        let properties = meta_schema["properties"]
            .as_object()
            .unwrap_or_else(|| panic!("expected root properties in {}", path.display()));
        keys.extend(properties.keys().cloned());
    }
    assert!(
        paths.len() >= 8,
        "expected every vocabulary, found {paths:?}"
    );

    let values = [
        Value::Null,
        platform_value!(true),
        platform_value!(-1),
        platform_value!(0),
        platform_value!(1.5),
        platform_value!("x"),
        platform_value!("1a"),
        platform_value!("a b"),
        platform_value!([]),
        platform_value!(["a", "a"]),
        platform_value!({}),
        platform_value!({ "a": 1 }),
    ];
    let mut uncompiled = 0;
    for key in &keys {
        for value in &values {
            let root = a_root_with(key, value.clone());
            if existing_validator_compiles(&validating_json(&root)) {
                continue;
            }
            uncompiled += 1;
            assert!(
                ValidationSchema::try_from_root(&root).is_err(),
                "the existing validator compiles nothing for `{key}`: {value:?}, which the builder builds"
            );
        }
    }
    assert!(uncompiled > 0);
}
