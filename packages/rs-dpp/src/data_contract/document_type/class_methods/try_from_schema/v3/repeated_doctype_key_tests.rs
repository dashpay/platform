//! A document type schema naming a key twice, at its root or in an object below it, which
//! registration refuses from protocol version 15 (`check_document_type_schema` 1).

use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::data_contract::config::DataContractConfig;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::schema::validate_max_depth;
use crate::data_contract::document_type::{DocumentPropertyType, DocumentType};
use crate::data_contract::errors::DataContractError;
use crate::data_contract::serialized_version::v0::DataContractInSerializationFormatV0;
use crate::data_contract::serialized_version::DataContractInSerializationFormat;
use crate::data_contract::DataContract;
use crate::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructureUntrusted;
use crate::validation::operations::ProtocolValidationOperation;
use crate::ProtocolError;
use platform_serialization::PLATFORM_BINCODE_CONFIG;
use platform_value::{platform_value, Identifier, Value};
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;
use std::sync::OnceLock;

/// The latest protocol version with the registration check of protocol version 14 and
/// earlier, `check_document_type_schema` 0.
fn latest_with_the_meta_schema_check() -> &'static PlatformVersion {
    static VERSION: OnceLock<PlatformVersion> = OnceLock::new();
    VERSION.get_or_init(|| {
        let mut version = PlatformVersion::latest().clone();
        version
            .dpp
            .contract_versions
            .document_type_versions
            .schema
            .check_document_type_schema = 0;
        version
    })
}

fn parse_on(
    schema: Value,
    schema_defs: Option<&BTreeMap<String, Value>>,
    full_validation: bool,
    platform_version: &PlatformVersion,
) -> Result<DocumentType, ProtocolError> {
    let config = DataContractConfig::default_for_version(platform_version)
        .expect("expected a default config");
    DocumentType::try_from_schema(
        Identifier::new([7; 32]),
        1,
        config.version(),
        "note",
        schema,
        schema_defs,
        &BTreeMap::new(),
        &config,
        full_validation,
        &mut vec![],
        platform_version,
    )
}

fn parse(schema: Value, full_validation: bool) -> Result<DocumentType, ProtocolError> {
    parse_on(schema, None, full_validation, PlatformVersion::latest())
}

/// Requires `error` to be the paid basic error `InvalidContractStructure` saying `key` is
/// declared more than once at `place`, "its root" or the path of the object repeating it.
fn assert_repeat_refused(error: ProtocolError, key: &str, place: &str) {
    let ProtocolError::ConsensusError(consensus_error) = &error else {
        panic!("expected a consensus error, got {error:?}");
    };
    let ConsensusError::BasicError(BasicError::ContractError(
        DataContractError::InvalidContractStructure(message),
    )) = consensus_error.as_ref()
    else {
        panic!("expected InvalidContractStructure, got {consensus_error:?}");
    };
    assert!(
        message.contains(&format!("declares `{key}` more than once at {place}")),
        "{message}"
    );
}

fn text(value: &str) -> Value {
    Value::Text(value.to_string())
}

/// An object property `note` whose schema names `properties` twice: first with a `child`
/// requiring `leaf`, then with a `child` that does not and holds `extra` too. The parser's
/// `required` walk reads the first entry, its property walk and every check the last.
fn nested_repeating_properties() -> Value {
    let leaf = platform_value!({ "type": "string", "maxLength": 10, "position": 0 });
    let extra = platform_value!({ "type": "string", "maxLength": 10, "position": 1 });
    let requiring_leaf = platform_value!({
        "child": {
            "type": "object",
            "position": 0,
            "properties": { "leaf": leaf.clone() },
            "required": ["leaf"],
            "additionalProperties": false
        }
    });
    let not_requiring_leaf = platform_value!({
        "child": {
            "type": "object",
            "position": 0,
            "properties": { "leaf": leaf, "extra": extra },
            "additionalProperties": false
        }
    });
    schema_of(vec![(
        "properties",
        platform_value!({
            "note": Value::Map(vec![
                (text("type"), text("object")),
                (text("position"), Value::U64(0)),
                (text("properties"), requiring_leaf),
                (text("properties"), not_requiring_leaf),
                (text("additionalProperties"), Value::Bool(false)),
            ])
        }),
    )])
}

fn integer_properties() -> Value {
    platform_value!({ "a": { "type": "integer", "minimum": 0, "maximum": 100, "position": 0 } })
}

fn string_properties() -> Value {
    platform_value!({ "a": { "type": "string", "maxLength": 10, "position": 0 } })
}

/// A schema whose root holds `entries` in order, with `additionalProperties: false`.
fn schema_of(entries: Vec<(&str, Value)>) -> Value {
    let mut map: Vec<(Value, Value)> = vec![("type".into(), "object".into())];
    map.extend(
        entries
            .into_iter()
            .map(|(key, value)| (Value::Text(key.to_string()), value)),
    );
    map.push(("additionalProperties".into(), false.into()));
    Value::Map(map)
}

fn repeating_properties() -> Value {
    schema_of(vec![
        ("properties", string_properties()),
        ("properties", integer_properties()),
    ])
}

/// The parser reads the first entry of a key repeated at the root, every schema check the
/// last, so registration refuses the repeat, whichever key it is.
#[test]
fn should_refuse_a_key_repeated_at_the_root_of_a_document_schema_under_full_validation() {
    for (key, schema) in [
        ("properties", repeating_properties()),
        (
            "documentsMutable",
            schema_of(vec![
                ("properties", integer_properties()),
                ("documentsMutable", false.into()),
                ("documentsMutable", true.into()),
            ]),
        ),
    ] {
        assert_repeat_refused(
            parse(schema, true).expect_err("expected the repeat to be refused"),
            key,
            "its root",
        );
    }
}

/// Below the root the parser still reads the first entry of some repeated keys (the
/// `required` walk down a nested `properties`, an index's keys) where every check reads
/// the last, so registration refuses a repeat in any object of the schema, naming where.
#[test]
fn should_refuse_a_key_repeated_below_the_root_of_a_document_schema_under_full_validation() {
    let repeating_unique = schema_of(vec![
        ("properties", string_properties()),
        (
            "indices",
            Value::Array(vec![Value::Map(vec![
                (text("name"), text("byA")),
                (text("properties"), platform_value!([{ "a": "asc" }])),
                (text("unique"), Value::Bool(false)),
                (text("unique"), Value::Bool(true)),
            ])]),
        ),
    ]);
    // The schema is read as sent: expanding a property type shorthand keeps the last of a
    // repeated `type` or `size` and drops the rest, which would hide the repeat from every
    // check while the contract stores both entries
    let property_repeating = |entries: Vec<(&str, Value)>| {
        let mut property = vec![(text("position"), Value::U64(0))];
        property.extend(entries.into_iter().map(|(key, value)| (text(key), value)));
        schema_of(vec![(
            "properties",
            platform_value!({ "a": Value::Map(property) }),
        )])
    };
    let repeating_type = property_repeating(vec![
        ("type", text("string")),
        ("maxLength", Value::U64(10)),
        ("type", text("identifier")),
    ]);
    let repeating_size = property_repeating(vec![
        ("type", text("bytes")),
        ("size", Value::U64(4)),
        ("size", Value::U64(8)),
    ]);
    for (key, place, schema) in [
        (
            "properties",
            "`properties.note`",
            nested_repeating_properties(),
        ),
        ("unique", "`indices[0]`", repeating_unique),
        ("type", "`properties.a`", repeating_type),
        ("size", "`properties.a`", repeating_size),
    ] {
        assert_repeat_refused(
            parse(schema, true).expect_err("expected the repeat to be refused"),
            key,
            place,
        );
    }
}

/// The schema checked is the enriched one, so a repeat in the contract's `$defs`, which a
/// property's `$ref` reaches, is refused as well.
#[test]
fn should_refuse_a_key_repeated_in_a_definition_under_full_validation() {
    let defs = BTreeMap::from([(
        "word".to_string(),
        Value::Map(vec![
            (text("type"), text("string")),
            (text("maxLength"), Value::U64(10)),
            (text("maxLength"), Value::U64(20)),
        ]),
    )]);
    let schema = schema_of(vec![(
        "properties",
        platform_value!({ "a": { "$ref": "#/$defs/word", "position": 0 } }),
    )]);
    assert_repeat_refused(
        parse_on(schema.clone(), Some(&defs), true, PlatformVersion::latest())
            .expect_err("expected the repeat to be refused"),
        "maxLength",
        "`$defs.word`",
    );

    let defs = BTreeMap::from([(
        "word".to_string(),
        platform_value!({ "type": "string", "maxLength": 20 }),
    )]);
    parse_on(schema, Some(&defs), true, PlatformVersion::latest())
        .expect("expected the schema to register once the repeat is gone");
}

/// The refusal belongs to `check_document_type_schema` 1: the registration check of
/// protocol version 14 and earlier still admits the repeat, at the root or below it.
#[test]
fn should_leave_the_meta_schema_check_admitting_a_repeated_key() {
    for schema in [repeating_properties(), nested_repeating_properties()] {
        parse_on(schema, None, true, latest_with_the_meta_schema_check())
            .expect("expected check_document_type_schema 0 to admit the schema");
    }
}

/// What the rule keeps out of the chain: parsed without validation, as a contract stored
/// before protocol version 15 is, the parser's two walks of a nested repeated `properties`
/// read different entries, so the type requires `note.child.leaf` of the first entry while
/// its properties hold the `extra` of the last.
#[test]
fn should_still_parse_a_nested_repeated_key_without_validation_reading_both_entries() {
    let document_type =
        parse(nested_repeating_properties(), false).expect("expected a stored schema to load");
    assert!(document_type.required_fields().contains("note.child.leaf"));
    let DocumentPropertyType::Object(note) = &document_type.properties()["note"].property_type
    else {
        panic!("expected `note` to be an object");
    };
    let DocumentPropertyType::Object(child) = &note["child"].property_type else {
        panic!("expected `child` to be an object");
    };
    assert!(child["leaf"].required);
    assert!(child.contains_key("extra"));
}

/// The rule holds at registration only: a contract read back from state, parsed without
/// validation, still loads, reading the first entry as it always has.
#[test]
fn should_still_parse_a_repeated_key_without_validation() {
    let document_type =
        parse(repeating_properties(), false).expect("expected a stored schema to load");
    assert!(matches!(
        document_type.flattened_properties()["a"].property_type,
        DocumentPropertyType::String(_)
    ));
}

/// The same schema without the repeat registers.
#[test]
fn should_register_the_schema_once_the_repeat_is_gone() {
    parse(schema_of(vec![("properties", integer_properties())]), true)
        .expect("expected the schema to register");
}

/// A contract decoded from the bytes a create or update transition carries keeps both
/// entries through bincode, and the full validation the transition runs refuses it.
#[test]
fn should_refuse_a_contract_decoded_with_a_repeated_key_under_full_validation() {
    let platform_version = PlatformVersion::latest();
    let contract_bytes = |schema: Value| {
        let format: DataContractInSerializationFormat = DataContractInSerializationFormatV0 {
            id: Identifier::new([7; 32]),
            config: DataContractConfig::default_for_version(platform_version)
                .expect("expected a default config"),
            version: 2,
            owner_id: Identifier::new([8; 32]),
            schema_defs: None,
            document_schemas: BTreeMap::from([("note".to_string(), schema)]),
        }
        .into();
        bincode::encode_to_vec(format, PLATFORM_BINCODE_CONFIG)
            .expect("expected to encode the contract")
    };

    DataContract::versioned_deserialize_untrusted(
        &contract_bytes(schema_of(vec![("properties", integer_properties())])),
        true,
        platform_version,
    )
    .expect("expected the contract to register");

    let error = DataContract::versioned_deserialize_untrusted(
        &contract_bytes(repeating_properties()),
        true,
        platform_version,
    )
    .expect_err("expected the contract to be refused");
    assert_repeat_refused(error, "properties", "its root");
}

/// Whether the type registers under full validation, and the sizes of the
/// `DocumentTypeSchemaValidationForSize` operations its parse records.
fn registration_and_schema_sizes_billed(
    schema: Value,
    schema_defs: Option<&BTreeMap<String, Value>>,
) -> (bool, Vec<u64>) {
    let platform_version = PlatformVersion::latest();
    let config = DataContractConfig::default_for_version(platform_version)
        .expect("expected a default config");
    let mut operations = Vec::new();
    let registered = DocumentType::try_from_schema(
        Identifier::new([7; 32]),
        1,
        config.version(),
        "note",
        schema,
        schema_defs,
        &BTreeMap::new(),
        &config,
        true,
        &mut operations,
        platform_version,
    )
    .is_ok();
    let sizes = operations
        .iter()
        .filter_map(|operation| match operation {
            ProtocolValidationOperation::DocumentTypeSchemaValidationForSize(size) => Some(*size),
            _ => None,
        })
        .collect();
    (registered, sizes)
}

/// The size a successful parse bills for reading `schema` with `schema_defs`: what
/// `validate_max_depth` measures of the root the base schema and `$defs` are added to.
fn billed_size(schema: &Value, schema_defs: Option<&BTreeMap<String, Value>>) -> u64 {
    let platform_version = PlatformVersion::latest();
    let root = DocumentType::enrich_with_base_schema(
        schema.clone(),
        schema_defs.map(|schema_defs| Value::from(schema_defs.clone())),
        platform_version,
    )
    .expect("expected a root");
    validate_max_depth(&root, platform_version)
        .expect("expected a measure")
        .into_data()
        .expect("expected a size")
        .size
}

/// Sixty string properties, read before any repeat that follows them.
fn many_properties() -> Value {
    Value::Map(
        (0..60u32)
            .map(|position| {
                (
                    Value::Text(format!("p{position}")),
                    platform_value!({ "type": "string", "maxLength": 10, "position": position }),
                )
            })
            .collect(),
    )
}

/// The size `validate_max_depth` measures of `value`, a schema without references.
fn measured_size(value: &Value) -> u64 {
    validate_max_depth(value, PlatformVersion::latest())
        .expect("expected a measure")
        .into_data()
        .expect("expected a size")
        .size
}

/// Refusing a repeat bills what the walk read, the schema and the contract's `$defs` up to
/// the repeat, in one operation; a schema that registers is billed once, by the parse.
#[test]
fn should_bill_the_schema_read_when_a_repeated_key_is_refused() {
    // A repeat after a large subtree of the schema
    let (registered, sizes) = registration_and_schema_sizes_billed(
        schema_of(vec![
            ("properties", many_properties()),
            ("documentsMutable", false.into()),
            ("documentsMutable", true.into()),
        ]),
        None,
    );
    assert!(!registered);
    assert!(
        matches!(sizes.as_slice(), [read] if *read > measured_size(&many_properties())),
        "{sizes:?}"
    );

    // A repeat in a definition, after the schema and a large definition
    let large_definition = platform_value!({ "type": "object", "properties": many_properties() });
    let definitions = BTreeMap::from([
        ("a".to_string(), large_definition.clone()),
        (
            "b".to_string(),
            Value::Map(vec![
                (text("type"), text("string")),
                (text("maxLength"), Value::U64(10)),
                (text("maxLength"), Value::U64(20)),
            ]),
        ),
    ]);
    let (registered, sizes) = registration_and_schema_sizes_billed(
        schema_of(vec![("properties", integer_properties())]),
        Some(&definitions),
    );
    assert!(!registered);
    assert!(
        matches!(sizes.as_slice(), [read] if *read > measured_size(&large_definition)),
        "{sizes:?}"
    );

    // A schema that registers records the parse's one operation
    let schema = schema_of(vec![("properties", many_properties())]);
    let read = billed_size(&schema, None);
    assert_eq!(
        registration_and_schema_sizes_billed(schema, None),
        (true, vec![read])
    );
}

/// A schema the walk passes, refused by a later stage before the parse records the schema's
/// size, is billed for what the walk read: here `immutable` is not a list, which the parse
/// refuses before its depth check.
#[test]
fn should_bill_the_walk_of_a_schema_refused_before_its_size_is_billed() {
    let (registered, sizes) = registration_and_schema_sizes_billed(
        schema_of(vec![
            ("properties", many_properties()),
            ("immutable", true.into()),
        ]),
        None,
    );
    assert!(!registered);
    assert!(
        matches!(sizes.as_slice(), [read] if *read > measured_size(&many_properties())),
        "{sizes:?}"
    );
}

/// A depth check that stops early, at a reference it can not resolve after a large subtree,
/// measures less than the walk read before it: the one size the parse records is raised to
/// the walk's count.
#[test]
fn should_bill_the_walk_when_the_depth_check_stops_short_of_it() {
    let (registered, sizes) = registration_and_schema_sizes_billed(
        schema_of(vec![
            ("properties", many_properties()),
            ("$ref", text("#/missing")),
        ]),
        None,
    );
    assert!(!registered);
    assert!(
        matches!(sizes.as_slice(), [read] if *read > measured_size(&many_properties())),
        "{sizes:?}"
    );
}

/// Every map entry the walk reads is billed, one whose key is not text included: a wider
/// map of such entries before a repeat costs more.
#[test]
fn should_bill_the_entries_with_keys_that_are_not_text_the_walk_read() {
    let billed_with = |entries: u64| {
        let description = Value::Map(
            (0..entries)
                .map(|index| (Value::U64(index), Value::Null))
                .collect(),
        );
        let (registered, sizes) = registration_and_schema_sizes_billed(
            schema_of(vec![
                ("properties", integer_properties()),
                ("description", description),
                ("documentsMutable", false.into()),
                ("documentsMutable", true.into()),
            ]),
            None,
        );
        assert!(!registered);
        let [size] = sizes.as_slice() else {
            panic!("expected one size, got {sizes:?}");
        };
        *size
    };
    assert!(
        billed_with(100) >= billed_with(10) + 90,
        "{} {}",
        billed_with(100),
        billed_with(10)
    );
}

/// The walk reads every value whatever a reference in it says: a reference the schema as
/// sent can not resolve, and only the expansion of a property type shorthand would make
/// resolvable, does not keep a nested repeat from being refused.
#[test]
fn should_refuse_a_nested_repeat_beside_a_reference_only_the_expansion_resolves() {
    let mut schema = nested_repeating_properties();
    let Value::Map(root) = &mut schema else {
        panic!("a schema is a map");
    };
    let Some((_, Value::Map(properties))) = root
        .iter_mut()
        .find(|(key, _)| key.as_text() == Some("properties"))
    else {
        panic!("the schema declares properties");
    };
    properties.push((
        text("a"),
        platform_value!({
            "type": "identifier",
            "position": 1,
            "examples": [{ "$ref": "#/properties/a/byteArray" }]
        }),
    ));
    assert_repeat_refused(
        parse(schema, true).expect_err("expected the nested repeat to be refused"),
        "properties",
        "`properties.note`",
    );
}
