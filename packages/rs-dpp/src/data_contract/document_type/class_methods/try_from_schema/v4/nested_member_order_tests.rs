//! An object's members as generation 4 parses them: stored in the schema's map order, as every
//! generation stores them, whatever their positions.
use super::*;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::DocumentPropertyType;
use platform_value::platform_value;

fn parse_at(
    schema: Value,
    defs: Option<BTreeMap<String, Value>>,
    full_validation: bool,
    platform_version: &PlatformVersion,
) -> Result<DocumentType, ProtocolError> {
    let config = DataContractConfig::default_for_version(platform_version)
        .expect("default config available");
    DocumentType::try_from_schema(
        Identifier::new([1; 32]),
        1,
        config.version(),
        "test_doc",
        schema,
        defs.as_ref(),
        &BTreeMap::new(),
        &config,
        full_validation,
        &mut vec![],
        platform_version,
    )
}

/// A type whose only property is the object `meta`, holding `members` as written.
fn with_meta(members: Value) -> Value {
    platform_value!({
        "type": "object",
        "properties": {
            "meta": {
                "type": "object",
                "position": 0,
                "properties": members,
                "additionalProperties": false,
            },
        },
        "additionalProperties": false,
    })
}

fn boolean(position: Value) -> Value {
    platform_value!({ "type": "boolean", "position": position })
}

/// The members of `meta`, at `path` below it, in the order the type stores them.
fn stored_order(document_type: &DocumentType, path: &[&str]) -> Vec<String> {
    let mut properties = document_type.properties();
    for name in std::iter::once(&"meta").chain(path) {
        let Some(DocumentPropertyType::Object(members)) = properties
            .get(*name)
            .map(|property| &property.property_type)
        else {
            panic!("expected an object at {name}");
        };
        properties = members;
    }
    properties.keys().cloned().collect()
}

/// The parse reads no member position, at protocol version 14 and from 15 alike, with or
/// without full validation: an object a contract already has is stored in map order whatever its
/// positions, and stays updatable. Registration and an update hold the objects a contract gains
/// to consecutive positions (`MemberPositionOrder`).
#[test]
fn should_leave_an_objects_member_positions_to_registration() {
    for (case, members) in [
        (
            "a gap",
            platform_value!({ "a": boolean(0.into()), "b": boolean(2.into()) }),
        ),
        (
            "a position taken twice",
            platform_value!({ "a": boolean(0.into()), "b": boolean(0.into()) }),
        ),
        (
            "a float",
            platform_value!({ "a": boolean(0.into()), "b": boolean(Value::Float(1.0)) }),
        ),
        (
            "out of position order",
            platform_value!({ "b": boolean(1.into()), "a": boolean(0.into()) }),
        ),
    ] {
        for platform_version in [
            PlatformVersion::get(14).expect("expected protocol version 14"),
            PlatformVersion::latest(),
        ] {
            let document_type = parse_at(with_meta(members.clone()), None, true, platform_version)
                .unwrap_or_else(|error| {
                    panic!(
                        "{case} at protocol version {}: expected the type to parse: {error}",
                        platform_version.protocol_version
                    )
                });
            let order: Vec<String> = members
                .as_map()
                .expect("expected a map")
                .iter()
                .filter_map(|(name, _)| name.as_text().map(str::to_string))
                .collect();
            assert_eq!(stored_order(&document_type, &[]), order, "{case}");
        }
    }
}

/// A stored contract is read without full validation, and reads as it always did whatever its
/// positions: an object's members keep the schema's map order, out of position order, with a
/// gap, a repeated, float or missing position alike.
#[test]
fn should_read_every_stored_objects_members_in_map_order() {
    let latest = PlatformVersion::latest();
    for (case, members, order) in [
        (
            "out of position order",
            platform_value!({ "b": boolean(1.into()), "a": boolean(0.into()) }),
            ["b", "a"],
        ),
        (
            "a float position",
            platform_value!({ "b": boolean(Value::Float(1.0)), "a": boolean(0.into()) }),
            ["b", "a"],
        ),
        (
            "a member without a position",
            platform_value!({ "b": { "type": "boolean" }, "a": boolean(0.into()) }),
            ["b", "a"],
        ),
        (
            "a position taken twice",
            platform_value!({ "b": boolean(1.into()), "a": boolean(1.into()) }),
            ["b", "a"],
        ),
        (
            "a gap",
            platform_value!({ "b": boolean(5.into()), "a": boolean(2.into()) }),
            ["b", "a"],
        ),
    ] {
        let document_type = parse_at(with_meta(members), None, false, latest)
            .unwrap_or_else(|error| panic!("{case}: expected the type to parse: {error}"));
        assert_eq!(stored_order(&document_type, &[]), order, "{case}");
    }
}

/// A member a stored schema writes twice is the one the schema's map order resolves, its first
/// place with its last schema, at protocol version 14 and from 15 alike.
#[test]
fn should_resolve_a_member_written_twice_as_the_schema_map_order_does() {
    let members = Value::Map(vec![
        (
            Value::Text("a".to_string()),
            platform_value!({ "type": "string", "maxLength": 10, "position": 2 }),
        ),
        (Value::Text("b".to_string()), boolean(1.into())),
        (
            Value::Text("a".to_string()),
            platform_value!({ "type": "integer", "position": 0 }),
        ),
    ]);
    for platform_version in [
        PlatformVersion::get(14).expect("expected protocol version 14"),
        PlatformVersion::latest(),
    ] {
        let document_type = parse_at(with_meta(members.clone()), None, false, platform_version)
            .expect("expected the type to parse");
        let Some(DocumentPropertyType::Object(meta)) = document_type
            .properties()
            .get("meta")
            .map(|property| &property.property_type)
        else {
            panic!("expected the object meta");
        };
        assert!(
            matches!(
                meta.get("a").map(|property| &property.property_type),
                Some(DocumentPropertyType::I64)
            ),
            "protocol version {}: {:?}",
            platform_version.protocol_version,
            meta.get("a")
        );
    }
}

/// A member written twice is parsed in both its schemas, by the flattened walk, whichever one
/// the order keeps: an invalid first schema is refused at protocol version 14 and from 15.
#[test]
fn should_refuse_a_member_written_twice_whose_first_schema_is_invalid() {
    let members = Value::Map(vec![
        (
            Value::Text("a".to_string()),
            platform_value!({ "type": "nonsense", "position": 0 }),
        ),
        (Value::Text("a".to_string()), boolean(0.into())),
    ]);
    for platform_version in [
        PlatformVersion::get(14).expect("expected protocol version 14"),
        PlatformVersion::latest(),
    ] {
        assert!(
            parse_at(with_meta(members.clone()), None, false, platform_version).is_err(),
            "protocol version {}",
            platform_version.protocol_version
        );
    }
}
