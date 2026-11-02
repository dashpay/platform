//! Tests of what every generation of the builder shares: the keyword and type tables, the
//! version dispatch, and the generated schemas and existing validator the generations'
//! tests compare with.

use super::*;
use crate::tests::fixtures::get_dashpay_contract_fixture;
#[cfg(feature = "validation")]
use crate::validation::JsonSchemaValidator;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use strum::IntoEnumIterator;

fn object(node: &SchemaNode) -> &ObjectConstraints {
    node.object.as_deref().expect("expected object keywords")
}

/// Whether the existing validator, compiling `schema` as the document validator compiles a
/// document schema, accepts `document`.
#[cfg(feature = "validation")]
pub(super) fn existing_validator_accepts(
    schema: &serde_json::Value,
    document: &serde_json::Value,
) -> bool {
    JsonSchemaValidator::new()
        .compile_and_validate(schema, document, PlatformVersion::latest())
        .expect("expected the schema to compile")
        .is_valid()
}

#[test]
fn should_build_only_at_a_protocol_version_selecting_the_builder() {
    for protocol_version in [13, 14] {
        let platform_version =
            PlatformVersion::get(protocol_version).expect("expected the version");
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
            .expect_err("expected the protocol version to be refused");
        assert!(
            error
                .to_string()
                .contains("checks no document against a validation schema"),
            "{error}"
        );
    }

    let platform_version = PlatformVersion::latest();
    let contract = get_dashpay_contract_fixture(None, 0, platform_version.protocol_version)
        .data_contract_owned();
    let name = contract
        .document_types()
        .keys()
        .next()
        .expect("expected a document type")
        .clone();
    contract
        .document_type_validation_schema(&name, platform_version)
        .expect("expected the latest protocol version to be accepted");
}

#[test]
fn should_name_every_json_type_and_read_it_back() {
    for json_type in JsonType::iter() {
        assert_eq!(JsonType::from_name(json_type.name()), Some(json_type));
    }
    assert_eq!(JsonType::Integer.name(), "integer");
    assert_eq!(JsonType::from_name("Integer"), None);
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
        (SchemaKeyword::Resolution(ResolutionKeyword::Id), "$id"),
        (
            SchemaKeyword::Object(ObjectKeyword::AdditionalProperties),
            "additionalProperties",
        ),
        (
            SchemaKeyword::Number(NumberKeyword::MultipleOf),
            "multipleOf",
        ),
        (
            SchemaKeyword::Storage(StorageKeyword::ContentMediaType),
            "contentMediaType",
        ),
        (
            SchemaKeyword::Storage(StorageKeyword::RequiredSince),
            "requiredSince",
        ),
        (
            SchemaKeyword::Relationship(RelationshipKeyword::DistinctFrom),
            "distinctFrom",
        ),
        (SchemaKeyword::Array(ArrayKeyword::ByteArray), "byteArray"),
    ] {
        assert_eq!(keyword.name(), name);
    }
}

#[test]
fn should_say_what_checks_each_keyword() {
    // The validator's keyword table is tied to the pinned validator by the probes below;
    // `byteArray` is the keyword Platform registers with it
    for keyword in SchemaKeyword::iter() {
        let name = keyword.name();
        let expected = if VALIDATOR_KEYWORDS.contains(&name) || name == "byteArray" {
            CheckedBy::JsonSchemaValidator
        } else if matches!(name, "maxBytes" | "encryptedFor") {
            CheckedBy::DocumentTypeParser
        } else if matches!(keyword, SchemaKeyword::Relationship(_)) {
            CheckedBy::Relationship
        } else {
            CheckedBy::Nothing
        };
        assert_eq!(keyword.checked_by(), expected, "{name}");
        assert_eq!(
            keyword.constrains_a_value(),
            matches!(
                expected,
                CheckedBy::JsonSchemaValidator | CheckedBy::DocumentTypeParser
            ),
            "{name}"
        );
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

/// A small deterministic generator, so that the generated cases are the same on every run.
#[cfg(feature = "validation")]
pub(super) struct Cases(pub(super) u64);

#[cfg(feature = "validation")]
impl Cases {
    /// xorshift64*.
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    pub(super) fn below(&mut self, bound: usize) -> usize {
        (self.next() % bound as u64) as usize
    }

    fn chance(&mut self, percent: u64) -> bool {
        self.next() % 100 < percent
    }

    fn pick(&mut self, values: &[serde_json::Value]) -> serde_json::Value {
        values[self.below(values.len())].clone()
    }
}

/// The values the generated schemas compare with and the generated documents carry.
#[cfg(feature = "validation")]
pub(super) fn generated_values() -> Vec<serde_json::Value> {
    use serde_json::json;
    vec![
        json!(null),
        json!(true),
        json!(false),
        json!(0),
        json!(1),
        json!(-1),
        json!(2),
        json!(1.5),
        json!(2.0),
        json!(10),
        json!(255),
        json!(256),
        json!(-0.5),
        json!(""),
        json!("a"),
        json!("abc"),
        json!("b1"),
        json!("2026-10-20"),
        json!("a@b.co"),
        json!("10.0.0.1"),
        json!("http://x"),
        json!([]),
        json!([1]),
        json!([1, 1]),
        json!([1, 2, 3]),
        json!(["a", "b"]),
        json!([0, 255]),
        json!([256]),
        json!({}),
        json!({ "a": 1 }),
        json!({ "a": "x", "b": 2 }),
        json!({ "b": true }),
        json!({ "a": [1], "c": null }),
    ]
}

/// A schema of keywords the validation schema holds, with keys the validator ignores among
/// them, `depth` levels deep, whose `$ref`s reach the first `definitions` of the generated
/// definitions.
#[cfg(feature = "validation")]
pub(super) fn generated_schema(
    cases: &mut Cases,
    values: &[serde_json::Value],
    depth: usize,
    definitions: usize,
) -> serde_json::Value {
    use serde_json::json;
    let mut schema = serde_json::Map::new();
    let mut insert = |key: &str, value: serde_json::Value| {
        schema.insert(key.to_string(), value);
    };
    if definitions > 0 && cases.chance(15) {
        insert(
            "$ref",
            json!(format!("#/$defs/d{}", cases.below(definitions))),
        );
    }
    let types = [
        json!("string"),
        json!("integer"),
        json!("number"),
        json!("boolean"),
        json!("object"),
        json!("array"),
        json!("null"),
    ];
    if cases.chance(60) {
        let one = cases.pick(&types);
        insert(
            "type",
            if cases.chance(80) {
                one
            } else {
                json!([one, cases.pick(&types)])
            },
        );
    }
    if cases.chance(20) {
        insert("minLength", json!(cases.below(4)));
    }
    if cases.chance(20) {
        insert("maxLength", json!(cases.below(6)));
    }
    if cases.chance(12) {
        let patterns = [
            json!("^a"),
            json!("b$"),
            json!("^[a-z]+$"),
            json!("\\d"),
            json!("^\\w+$"),
        ];
        insert("pattern", cases.pick(&patterns));
    }
    if cases.chance(10) {
        let formats = [
            json!("date"),
            json!("email"),
            json!("ipv4"),
            json!("uri"),
            json!("time"),
        ];
        insert("format", cases.pick(&formats));
    }
    let bounds = [
        json!(-1),
        json!(0),
        json!(1),
        json!(1.5),
        json!(10),
        json!(255),
    ];
    for keyword in ["minimum", "maximum", "exclusiveMinimum", "exclusiveMaximum"] {
        if cases.chance(10) {
            insert(keyword, cases.pick(&bounds));
        }
    }
    if cases.chance(12) {
        let divisors = [json!(1), json!(2), json!(2.0), json!(0.5), json!(3)];
        insert("multipleOf", cases.pick(&divisors));
    }
    if cases.chance(10) {
        insert("enum", json!([cases.pick(values), cases.pick(values)]));
    }
    if cases.chance(8) {
        insert("const", cases.pick(values));
    }
    if depth > 0 && cases.chance(25) {
        insert(
            "items",
            generated_schema(cases, values, depth - 1, definitions),
        );
    }
    if depth > 0 && cases.chance(10) {
        insert(
            "contains",
            generated_schema(cases, values, depth - 1, definitions),
        );
    }
    if cases.chance(12) {
        insert("minItems", json!(cases.below(3)));
    }
    if cases.chance(12) {
        insert("maxItems", json!(cases.below(4)));
    }
    if cases.chance(10) {
        insert("uniqueItems", json!(cases.chance(70)));
    }
    if cases.chance(6) {
        insert("byteArray", json!(true));
    }
    if depth > 0 && cases.chance(25) {
        let mut properties = serde_json::Map::new();
        for name in ["a", "b"] {
            if cases.chance(70) {
                properties.insert(
                    name.to_string(),
                    generated_schema(cases, values, depth - 1, definitions),
                );
            }
        }
        insert("properties", serde_json::Value::Object(properties));
    }
    if cases.chance(15) {
        let names = [json!("a"), json!("b"), json!("c")];
        insert("required", json!([cases.pick(&names)]));
    }
    if cases.chance(10) {
        insert("additionalProperties", json!(cases.chance(50)));
    }
    if cases.chance(8) {
        insert("minProperties", json!(cases.below(3)));
    }
    if cases.chance(8) {
        insert("maxProperties", json!(cases.below(3)));
    }
    if cases.chance(8) {
        insert("dependentRequired", json!({ "a": ["b"] }));
    }
    // Keys the validator ignores, some of them only where they stand alone
    if cases.chance(10) {
        insert("title", json!("t"));
    }
    if cases.chance(10) {
        insert("examples", json!([cases.pick(values)]));
    }
    if cases.chance(8) {
        insert("default", cases.pick(values));
    }
    if cases.chance(8) {
        insert("x-note", json!(1));
    }
    if cases.chance(6) {
        insert("then", json!(false));
    }
    if cases.chance(6) {
        insert("minContains", json!(2));
    }
    if cases.chance(6) {
        insert("additionalItems", json!(false));
    }
    serde_json::Value::Object(schema)
}
