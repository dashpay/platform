use super::*;
use crate::tests::fixtures::get_dashpay_contract_fixture;
use platform_value::platform_value;
use std::collections::BTreeSet;

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
    assert_eq!(format(key).byte_array, Some(true));
    assert_eq!(
        format(key).content_media_type.as_deref(),
        Some("application/x.dash.dpp.identifier")
    );

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
            "slash": { "$ref": "#/$defs/a~1b" },
            "tilde": { "$ref": "#/$defs/c~0d" },
            "space": { "$ref": "#/$defs/e%20f" },
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
            "encodedSeparators": { "$ref": "#/$defs/wrapper%2Fproperties%2Flabel" },
            "encodedLeadingSlash": { "$ref": "#%2F$defs%2Fwrapper" },
            "lonePercent": { "$ref": "#/$defs/100%" },
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
        "properties": { "child": { "$ref": "#" } },
        "$defs": { "unused": { "type": "string" } },
    }));

    assert_eq!(schema.references.get("#"), Some(&schema.root));
}

#[test]
fn should_read_a_repeated_key_by_its_last_entry_in_the_place_of_its_first() {
    let text = |text: &str| Value::Text(text.to_string());
    let schema = build(Value::Map(vec![
        (text("type"), text("object")),
        (
            text("properties"),
            Value::Map(vec![
                // Shadowed by the last `value`: an unknown type the validator never reads.
                (text("value"), platform_value!({ "type": "bytes" })),
                (text("other"), platform_value!({ "type": "boolean" })),
                (
                    text("value"),
                    platform_value!({ "type": "string", "maxLength": 10 }),
                ),
                (
                    text("label"),
                    Value::Map(vec![
                        (text("$ref"), text("#/$defs/missing")),
                        (text("$ref"), text("#/$defs/label")),
                    ]),
                ),
            ]),
        ),
        (
            text("$defs"),
            Value::Map(vec![
                (text("label"), platform_value!({ "maxLength": 1 })),
                (text("label"), platform_value!({ "maxLength": 2 })),
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
    assert_eq!(
        property(&schema.root, "label").reference.as_deref(),
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
fn should_refuse_a_reference_in_a_schema_that_names_a_subschema() {
    let root = platform_value!({
        "type": "object",
        "properties": {
            "named": { "$id": "#/$defs/label", "type": "string", "maxLength": 1 },
            "user": { "$ref": "#/$defs/label" },
        },
        "$defs": { "label": { "type": "string", "maxLength": 10 } },
    });
    assert!(ValidationSchema::try_from_root(&root).is_err());
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
fn should_refuse_a_reference_in_a_schema_holding_a_fragment_id_anywhere() {
    let root = platform_value!({
        "type": "object",
        "ownerRefersTo": { "where": { "$id": "#/$defs/id" } },
        "properties": { "repoId": { "$ref": "#/$defs/id", "position": 0 } },
        "$defs": { "id": { "type": "string" } },
    });
    assert!(ValidationSchema::try_from_root(&root).is_err());
}

#[test]
fn should_refuse_a_reference_an_absolute_id_names() {
    // An `examples` value is searched for identifiers too, and an absolute `$id` resolving to
    // the reference's URL names the schema the reference reaches.
    let root = platform_value!({
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
    assert!(ValidationSchema::try_from_root(&root).is_err());
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
fn should_refuse_a_reference_an_id_above_it_rescopes() {
    let root = platform_value!({
        "type": "object",
        "properties": {
            "scoped": {
                "$id": "elsewhere",
                "type": "object",
                "properties": { "user": { "$ref": "#/$defs/label" } },
            },
        },
        "$defs": { "label": { "type": "string" } },
    });
    assert!(ValidationSchema::try_from_root(&root).is_err());
}

/// The divergence the refusal above guards: the existing validator checks a `$ref` against
/// the subschema an `$id` names, not the one its pointer reaches.
#[cfg(feature = "validation")]
#[test]
fn should_match_the_existing_validator_in_resolving_a_reference_through_an_id() {
    use crate::validation::JsonSchemaValidator;

    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "named": { "$id": "#/$defs/label", "type": "string", "maxLength": 1 },
            "user": { "$ref": "#/$defs/label" },
        },
        "$defs": { "label": { "type": "string", "maxLength": 10 } },
    });
    let result = JsonSchemaValidator::new()
        .compile_and_validate(
            &schema,
            &serde_json::json!({ "user": "hello" }),
            PlatformVersion::latest(),
        )
        .expect("expected the schema to compile");
    assert!(
        !result.is_valid(),
        "the existing validator holds `user` to the named schema's maxLength of 1"
    );

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
    let result = JsonSchemaValidator::new()
        .compile_and_validate(
            &absolute_alias,
            &serde_json::json!({ "user": "hello" }),
            PlatformVersion::latest(),
        )
        .expect("expected the schema to compile");
    assert!(
        !result.is_valid(),
        "the existing validator holds `user` to the schema the absolute `$id` names"
    );

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
        let result = JsonSchemaValidator::new()
            .compile_and_validate(
                &unredirected,
                &serde_json::json!({ "scoped": { "user": user } }),
                PlatformVersion::latest(),
            )
            .expect("expected the schema to compile");
        assert_eq!(result.is_valid(), valid, "{user}");
    }
}

#[test]
fn should_refuse_a_reference_that_does_not_resolve() {
    let root = platform_value!({
        "type": "object",
        "properties": { "a": { "$ref": "#/$defs/missing" } },
    });
    assert!(ValidationSchema::try_from_root(&root).is_err());
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
fn should_refuse_a_keyword_no_document_meta_schema_admits_below_the_root() {
    let root = platform_value!({
        "type": "object",
        "properties": {
            "a": { "type": "string", "oneOf": [{ "maxLength": 1 }, { "minLength": 3 }] },
        },
    });
    assert!(ValidationSchema::try_from_root(&root).is_err());
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
            SchemaKeyword::Format(FormatKeyword::ContentMediaType),
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
    let meta_schemas = [
        include_str!("../../../../../schema/meta_schemas/document/v0/document-meta.json"),
        include_str!("../../../../../schema/meta_schemas/document/v1/document-meta.json"),
        include_str!("../../../../../schema/meta_schemas/document/v2/document-meta.json"),
        include_str!("../../../../../schema/meta_schemas/document/v3/document-meta.json"),
    ];
    let mut admitted = BTreeSet::new();
    for meta_schema in meta_schemas {
        let meta_schema: serde_json::Value =
            serde_json::from_str(meta_schema).expect("expected the meta-schema to be JSON");
        for node in ["documentSchema", "documentArrayItem"] {
            if let Some(properties) = meta_schema["$defs"][node]["properties"].as_object() {
                admitted.extend(properties.keys().cloned());
            }
        }
    }

    let unclassified: Vec<_> = admitted
        .iter()
        .filter(|keyword| SchemaKeyword::from_name(keyword).is_none())
        .collect();
    assert!(
        unclassified.is_empty(),
        "keywords a document meta-schema admits that the validation schema neither reads nor ignores: {unclassified:?}"
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
