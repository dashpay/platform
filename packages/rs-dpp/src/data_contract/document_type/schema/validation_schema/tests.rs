use super::*;
use crate::tests::fixtures::get_dashpay_contract_fixture;
use platform_value::platform_value;
use std::collections::BTreeSet;

fn build(root: Value) -> ValidationSchema {
    ValidationSchema::try_from_root(&root).expect("expected the schema to build")
}

fn property<'a>(node: &'a SchemaNode, name: &str) -> &'a SchemaNode {
    node.properties
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
    assert_eq!(root.required, Some(vec!["name".to_string()]));
    assert_eq!(root.additional_properties, Some(false));

    let name = property(root, "name");
    assert_eq!(name.min_length, Some(1));
    assert_eq!(name.max_length, Some(63));
    assert_eq!(name.max_bytes, Some(126));
    assert_eq!(name.pattern.as_deref(), Some("^[a-z]+$"));
    assert_eq!(name.format.as_deref(), Some("uri"));
    assert_eq!(
        name.enum_values,
        Some(vec![Value::Text("a".into()), Value::Text("b".into())])
    );

    let score = property(root, "score");
    assert_eq!(
        score.types,
        Some(JsonTypes::AnyOf(vec![JsonType::Integer, JsonType::Null]))
    );
    assert!(score.minimum.as_ref().is_some_and(Value::is_integer));
    assert_eq!(score.maximum, Some(Value::Float(100.5)));
    assert!(score.exclusive_minimum.is_some());
    assert!(score.exclusive_maximum.is_some());
    assert!(score.multiple_of.is_some());
    assert!(score.constant.is_some());

    let tags = property(root, "tags");
    assert_eq!(
        tags.items.as_deref().and_then(|items| items.max_length),
        Some(8)
    );
    assert!(tags
        .contains
        .as_deref()
        .is_some_and(|contains| contains.constant.is_some()));
    assert_eq!((tags.min_items, tags.max_items), (Some(1), Some(4)));
    assert_eq!(tags.unique_items, Some(true));

    let key = property(root, "key");
    assert_eq!(key.byte_array, Some(true));
    assert_eq!(
        key.content_media_type.as_deref(),
        Some("application/x.dash.dpp.identifier")
    );

    let message = property(root, "message");
    let encrypted_for = message
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
        (meta.min_properties, meta.max_properties),
        (Some(1), Some(2))
    );
    assert_eq!(
        meta.dependent_required,
        Some(BTreeMap::from([("a".to_string(), vec!["b".to_string()])]))
    );
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
    assert_eq!(label.max_length, Some(5));
    assert_eq!(
        schema
            .references
            .get("#/$defs/label")
            .map(|node| node.min_length),
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
                .and_then(|node| node.max_length),
            Some(max_length),
            "{reference}"
        );
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
        schema.root.required,
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
        (SchemaKeyword::Annotation(AnnotationKeyword::Id), "$id"),
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
        let property_names: BTreeSet<&str> = schema
            .root
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
