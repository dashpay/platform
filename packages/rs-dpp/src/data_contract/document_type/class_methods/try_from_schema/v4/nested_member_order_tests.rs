//! An object's members as generation 4 parses them: stored in the schema's map order, as every
//! generation stores them, with positions that follow one another from the object's lowest
//! under full validation.
use super::*;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::DocumentPropertyType;
use platform_value::platform_value;

fn parse_at(
    schema: Value,
    defs: Option<BTreeMap<String, Value>>,
    platform_version: &PlatformVersion,
    full_validation: bool,
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

#[test]
fn should_hold_an_objects_positions_to_one_after_another_from_its_lowest() {
    let latest = PlatformVersion::latest();
    let v14 = PlatformVersion::get(14).expect("expected protocol version 14");

    for (case, members) in [
        (
            "positions from 0",
            platform_value!({ "a": boolean(0.into()), "b": boolean(1.into()) }),
        ),
        (
            "a single member at 1, as DPNS records",
            platform_value!({ "identity": boolean(1.into()) }),
        ),
        (
            "positions from 1, listed out of position order, which registration judges",
            platform_value!({ "b": boolean(2.into()), "a": boolean(1.into()) }),
        ),
    ] {
        parse_at(with_meta(members), None, latest, true)
            .unwrap_or_else(|error| panic!("{case}: expected it to register: {error}"));
    }

    let inner = |members: Value| {
        platform_value!({
            "inner": {
                "type": "object",
                "position": 0,
                "properties": members,
                "additionalProperties": false,
            },
        })
    };
    for (case, members, refusal) in [
        (
            "a gap",
            platform_value!({ "a": boolean(0.into()), "b": boolean(2.into()) }),
            "position 1 is missing",
        ),
        (
            "a position taken twice",
            platform_value!({ "a": boolean(0.into()), "b": boolean(0.into()) }),
            "position 0 is taken twice",
        ),
        (
            "a gap in an object inside an object",
            inner(platform_value!({ "p": boolean(3.into()), "q": boolean(5.into()) })),
            "position 4 is missing",
        ),
    ] {
        let error = parse_at(with_meta(members.clone()), None, latest, true)
            .expect_err(&format!("{case}: expected the type to be refused"));
        assert!(
            error.to_string().contains(refusal),
            "{case}: expected '{refusal}' in {error}"
        );
        // Protocol version 14 never read an object's positions
        parse_at(with_meta(members), None, v14, true)
            .unwrap_or_else(|error| panic!("{case}: expected it to register at 14: {error}"));
    }

    let error = parse_at(
        with_meta(platform_value!({ "shared": { "$ref": "#/$defs/d", "position": 0 } })),
        Some(BTreeMap::from([(
            "d".to_string(),
            platform_value!({
                "type": "object",
                "properties": { "a": boolean(0.into()), "b": boolean(2.into()) },
                "additionalProperties": false,
            }),
        )])),
        latest,
        true,
    )
    .expect_err("a gap in the members a $ref brings must be refused");
    assert!(
        error.to_string().contains("position 1 is missing"),
        "{error}"
    );
}

/// Full validation refuses a position that is not an integer from 0, which a stored contract's
/// nested members were never held to: the meta-schema counts `1.0` as an integer.
#[test]
fn should_refuse_a_member_position_that_is_not_an_integer_under_full_validation() {
    let latest = PlatformVersion::latest();
    let with_position = |position: Value| {
        with_meta(platform_value!({ "a": boolean(0.into()), "b": boolean(position) }))
    };

    let error = parse_at(with_position(Value::Float(1.0)), None, latest, true)
        .expect_err("a float position must be refused");
    assert!(error.to_string().contains("needs a position"), "{error}");
    // Protocol version 14 never read an object's positions, and its meta-schema counts `1.0`
    // as an integer
    parse_at(
        with_position(Value::Float(1.0)),
        None,
        PlatformVersion::get(14).expect("expected protocol version 14"),
        true,
    )
    .expect("a float position registers at protocol version 14");

    // The schema check refuses these before the order is read
    for (case, position) in [
        ("a negative integer", Value::I64(-1)),
        ("a string", Value::Text("1".to_string())),
    ] {
        assert!(
            parse_at(with_position(position), None, latest, true).is_err(),
            "{case}: expected the type to be refused"
        );
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
        let document_type = parse_at(with_meta(members), None, latest, false)
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
        let document_type = parse_at(with_meta(members.clone()), None, platform_version, false)
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

/// The positions full validation checks are the ones written beside a member's `$ref`, never one
/// its definition carries: the definition's would leave a gap.
#[test]
fn should_check_the_position_written_beside_a_members_ref() {
    let document_type = parse_at(
        with_meta(platform_value!({
            "x": { "$ref": "#/$defs/flag", "position": 0 },
            "y": boolean(1.into()),
        })),
        Some(BTreeMap::from([(
            "flag".to_string(),
            platform_value!({ "type": "boolean", "position": 5 }),
        )])),
        PlatformVersion::latest(),
        true,
    )
    .expect("positions 0 and 1, written beside the members, follow one another");
    assert_eq!(stored_order(&document_type, &[]), ["x", "y"]);
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
            parse_at(with_meta(members.clone()), None, platform_version, false).is_err(),
            "protocol version {}",
            platform_version.protocol_version
        );
    }
}
