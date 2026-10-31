//! A document type schema naming a key twice at its root, which registration refuses from
//! protocol version 15 (`check_document_type_schema` 1).

use crate::data_contract::config::DataContractConfig;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::{DocumentPropertyType, DocumentType};
use crate::ProtocolError;
use platform_value::{platform_value, Identifier, Value};
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

fn parse(schema: Value, full_validation: bool) -> Result<DocumentType, ProtocolError> {
    let platform_version = PlatformVersion::latest();
    let config = DataContractConfig::default_for_version(platform_version)
        .expect("expected a default config");
    DocumentType::try_from_schema(
        Identifier::new([7; 32]),
        1,
        config.version(),
        "note",
        schema,
        None,
        &BTreeMap::new(),
        &config,
        full_validation,
        &mut vec![],
        platform_version,
    )
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

/// The parser reads the first entry of a key repeated at the root, every schema check the
/// last, so registration refuses the repeat, whichever key it is.
#[test]
fn should_refuse_a_key_repeated_at_the_root_of_a_document_schema_under_full_validation() {
    for (key, schema) in [
        (
            "properties",
            schema_of(vec![
                ("properties", string_properties()),
                ("properties", integer_properties()),
            ]),
        ),
        (
            "documentsMutable",
            schema_of(vec![
                ("properties", integer_properties()),
                ("documentsMutable", false.into()),
                ("documentsMutable", true.into()),
            ]),
        ),
    ] {
        let message = format!(
            "{:?}",
            parse(schema, true).expect_err("expected the repeat to be refused")
        );
        assert!(
            message.contains(&format!("declares `{key}` more than once")),
            "{message}"
        );
    }
}

/// The rule holds at registration only: a contract read back from state, parsed without
/// validation, still loads, reading the first entry as it always has.
#[test]
fn should_still_parse_a_repeated_key_without_validation() {
    let document_type = parse(
        schema_of(vec![
            ("properties", string_properties()),
            ("properties", integer_properties()),
        ]),
        false,
    )
    .expect("expected a stored schema to load");
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
