//! The member rules of generation 2: an update keeps the members every object of the type
//! already has, in their order and with their requiredness, and adds members only after them.
//! Each case parses the old and the new type under full validation, as a contract update does,
//! and runs the update through the dispatcher.

use crate::consensus::codes::ErrorWithCode;
use crate::data_contract::config::DataContractConfig;
use crate::data_contract::document_type::DocumentType;
use crate::validation::SimpleConsensusValidationResult;
use platform_value::{platform_value, Identifier, Value};
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

/// What a case expects of the update.
#[derive(Clone, Copy, Debug)]
enum Expected {
    Accepted,
    /// Refused with this error code, the message holding the fragment.
    Refused(u32, &'static str),
}

const ORDER: Expected = Expected::Refused(40212, "can only add properties after");

struct Case {
    name: &'static str,
    old: Value,
    new: Value,
    old_defs: Option<Value>,
    new_defs: Option<Value>,
    expected: Expected,
}

fn case(name: &'static str, old: Value, new: Value, expected: Expected) -> Case {
    Case {
        name,
        old,
        new,
        old_defs: None,
        new_defs: None,
        expected,
    }
}

fn boolean(position: u32) -> Value {
    platform_value!({ "type": "boolean", "position": position })
}

fn integer(position: u32) -> Value {
    platform_value!({ "type": "integer", "minimum": 0, "maximum": 10, "position": position })
}

fn string(position: u32, max_length: u32) -> Value {
    platform_value!({ "type": "string", "maxLength": max_length, "position": position })
}

fn with(mut value: Value, key: &str, entry: Value) -> Value {
    value
        .set_value(key, entry)
        .expect("expected a map to set a key on");
    value
}

/// An object property at `position` holding `members` in their map order, with its own
/// `required` when one is given.
fn object(members: Value, position: u32, required: Option<Value>) -> Value {
    let object = platform_value!({
        "type": "object",
        "properties": members,
        "additionalProperties": false,
        "position": position,
    });
    match required {
        Some(required) => with(object, "required", required),
        None => object,
    }
}

fn document_type_schema(properties: Value, required: Value) -> Value {
    platform_value!({
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": false,
    })
}

/// `{ a: string, meta: { y: boolean, z: integer } }`, `meta`'s members optional.
fn meta_yz() -> Value {
    object(
        platform_value!({ "y": boolean(0), "z": integer(1) }),
        1,
        None,
    )
}

fn with_meta(meta: Value) -> Value {
    platform_value!({ "a": string(0, 10), "meta": meta })
}

/// `meta` taking its schema from the definition `m` of `$defs`, with `extra` beside the `$ref`.
fn meta_ref(extra: Option<(&str, Value)>) -> Value {
    let reference = platform_value!({ "$ref": "#/$defs/m", "position": 1 });
    match extra {
        Some((key, entry)) => with(reference, key, entry),
        None => reference,
    }
}

fn definitions(members: Value) -> Option<Value> {
    Some(platform_value!({
        "m": {
            "type": "object",
            "properties": members,
            "additionalProperties": false,
        }
    }))
}

fn parse(schema: &Value, defs: &Option<Value>, platform_version: &PlatformVersion) -> DocumentType {
    let config = DataContractConfig::default_for_version(platform_version)
        .expect("expected a default config");
    let defs: Option<BTreeMap<String, Value>> = defs.clone().map(|defs| {
        defs.into_btree_string_map()
            .expect("expected the definitions to be a map")
    });
    DocumentType::try_from_schema(
        Identifier::new([1; 32]),
        1,
        config.version(),
        "note",
        schema.clone(),
        defs.as_ref(),
        &BTreeMap::new(),
        &config,
        true,
        &mut Vec::new(),
        platform_version,
    )
    .unwrap_or_else(|error| panic!("expected the schema {schema:?} to register: {error}"))
}

fn assert_verdict(name: &str, result: &SimpleConsensusValidationResult, expected: Expected) {
    match expected {
        Expected::Accepted => assert!(
            result.is_valid(),
            "{name}: expected the update to be accepted, got {:?}",
            result.errors
        ),
        Expected::Refused(code, fragment) => {
            let error = result
                .errors
                .first()
                .unwrap_or_else(|| panic!("{name}: expected the update to be refused"));
            assert_eq!(error.code(), code, "{name}: unexpected error {error}");
            assert!(
                error.to_string().contains(fragment),
                "{name}: expected the error to mention {fragment:?}, got {error}"
            );
        }
    }
}

fn run(cases: &[Case], platform_version: &PlatformVersion) {
    for case in cases {
        let old = parse(&case.old, &case.old_defs, platform_version);
        let new = parse(&case.new, &case.new_defs, platform_version);
        let result = old
            .as_ref()
            .validate_update(new.as_ref(), 2, platform_version)
            .unwrap_or_else(|error| panic!("{}: validate_update errored: {error}", case.name));
        assert_verdict(case.name, &result, case.expected);
    }
}

/// The member edits every object of a type is held to, at every depth, through `$ref` too.
fn member_cases() -> Vec<Case> {
    let required = |names: Value| document_type_schema(with_meta(meta_yz()), names);
    vec![
        case(
            "an object's members written in another map order, their positions kept",
            required(platform_value!(["a"])),
            document_type_schema(
                with_meta(object(
                    platform_value!({ "z": integer(1), "y": boolean(0) }),
                    1,
                    None,
                )),
                platform_value!(["a"]),
            ),
            Expected::Accepted,
        ),
        case(
            "a member written first in an object's map, at a position after its existing ones",
            required(platform_value!(["a"])),
            document_type_schema(
                with_meta(object(
                    platform_value!({ "x": boolean(2), "y": boolean(0), "z": integer(1) }),
                    1,
                    None,
                )),
                platform_value!(["a"]),
            ),
            Expected::Accepted,
        ),
        case(
            "a member written last in an object's map, at a position before an existing one",
            document_type_schema(
                with_meta(object(platform_value!({ "z": integer(1) }), 1, None)),
                platform_value!(["a"]),
            ),
            document_type_schema(
                with_meta(object(
                    platform_value!({ "z": integer(1), "x": boolean(0) }),
                    1,
                    None,
                )),
                platform_value!(["a"]),
            ),
            ORDER,
        ),
        case(
            "a member added at the next position of an object whose positions start at 1",
            document_type_schema(
                with_meta(object(platform_value!({ "z": integer(1) }), 1, None)),
                platform_value!(["a"]),
            ),
            document_type_schema(
                with_meta(object(
                    platform_value!({ "z": integer(1), "x": boolean(2) }),
                    1,
                    None,
                )),
                platform_value!(["a"]),
            ),
            Expected::Accepted,
        ),
        case(
            "a member added after an object's existing ones",
            required(platform_value!(["a"])),
            document_type_schema(
                with_meta(object(
                    platform_value!({ "y": boolean(0), "z": integer(1), "x": boolean(2) }),
                    1,
                    None,
                )),
                platform_value!(["a"]),
            ),
            Expected::Accepted,
        ),
        case(
            "a member added to an object inside an object, at a position before its existing one",
            document_type_schema(
                with_meta(object(
                    platform_value!({
                        "y": boolean(0),
                        "inner": object(platform_value!({ "q": integer(1) }), 1, None),
                    }),
                    1,
                    None,
                )),
                platform_value!(["a"]),
            ),
            document_type_schema(
                with_meta(object(
                    platform_value!({
                        "y": boolean(0),
                        "inner": object(platform_value!({ "q": integer(1), "p": boolean(0) }), 1, None),
                    }),
                    1,
                    None,
                )),
                platform_value!(["a"]),
            ),
            Expected::Refused(40212, "meta.inner"),
        ),
        case(
            "a member's string bound raised",
            document_type_schema(
                with_meta(object(
                    platform_value!({ "y": boolean(0), "z": string(1, 10) }),
                    1,
                    None,
                )),
                platform_value!(["a"]),
            ),
            document_type_schema(
                with_meta(object(
                    platform_value!({ "y": boolean(0), "z": string(1, 20) }),
                    1,
                    None,
                )),
                platform_value!(["a"]),
            ),
            Expected::Accepted,
        ),
        case(
            "a top-level property added",
            required(platform_value!(["a"])),
            document_type_schema(
                platform_value!({ "a": string(0, 10), "meta": meta_yz(), "b": boolean(2) }),
                platform_value!(["a"]),
            ),
            Expected::Accepted,
        ),
        case(
            "a new object holding required members of its own",
            document_type_schema(
                platform_value!({ "a": string(0, 10) }),
                platform_value!(["a"]),
            ),
            document_type_schema(
                with_meta(object(
                    platform_value!({ "y": boolean(0), "z": integer(1) }),
                    1,
                    Some(platform_value!(["y"])),
                )),
                platform_value!(["a"]),
            ),
            Expected::Accepted,
        ),
        case(
            "an existing member added to its object's required",
            required(platform_value!(["a"])),
            document_type_schema(
                with_meta(object(
                    platform_value!({ "y": boolean(0), "z": integer(1) }),
                    1,
                    Some(platform_value!(["y"])),
                )),
                platform_value!(["a"]),
            ),
            Expected::Refused(10276, "meta.y"),
        ),
        case(
            "a member removed from its object's required",
            document_type_schema(
                with_meta(object(
                    platform_value!({ "y": boolean(0), "z": integer(1) }),
                    1,
                    Some(platform_value!(["y"])),
                )),
                platform_value!(["a"]),
            ),
            required(platform_value!(["a"])),
            Expected::Refused(10276, "meta.y"),
        ),
        case(
            "an object inside an object added to its object's required",
            document_type_schema(
                with_meta(object(
                    platform_value!({
                        "y": boolean(0),
                        "inner": object(platform_value!({ "p": boolean(0) }), 1, None),
                    }),
                    1,
                    None,
                )),
                platform_value!(["a"]),
            ),
            document_type_schema(
                with_meta(object(
                    platform_value!({
                        "y": boolean(0),
                        "inner": object(platform_value!({ "p": boolean(0) }), 1, None),
                    }),
                    1,
                    Some(platform_value!(["inner"])),
                )),
                platform_value!(["a"]),
            ),
            Expected::Refused(10276, "meta.inner"),
        ),
        case(
            "a dotted name added to a nested required",
            document_type_schema(
                with_meta(object(
                    platform_value!({
                        "y": boolean(0),
                        "inner": object(platform_value!({ "p": boolean(0) }), 1, None),
                    }),
                    1,
                    None,
                )),
                platform_value!(["a"]),
            ),
            document_type_schema(
                with_meta(object(
                    platform_value!({
                        "y": boolean(0),
                        "inner": object(platform_value!({ "p": boolean(0) }), 1, None),
                    }),
                    1,
                    Some(platform_value!(["inner.p"])),
                )),
                platform_value!(["a"]),
            ),
            Expected::Refused(10276, "inner.p"),
        ),
        case(
            "a dotted name added to the top-level required",
            required(platform_value!(["a"])),
            required(platform_value!(["a", "meta.y"])),
            Expected::Refused(10276, "meta.y"),
        ),
        case(
            "a dotted name removed from the top-level required",
            required(platform_value!(["a", "meta.y"])),
            required(platform_value!(["a"])),
            Expected::Refused(10276, "meta.y"),
        ),
        case(
            "a dotted name in the top-level required for an added member",
            required(platform_value!(["a"])),
            document_type_schema(
                with_meta(object(
                    platform_value!({ "y": boolean(0), "z": integer(1), "x": boolean(2) }),
                    1,
                    None,
                )),
                platform_value!(["a", "meta.x"]),
            ),
            Expected::Refused(10276, "meta.x"),
        ),
        case(
            "a top-level property whose name makes an existing member required",
            required(platform_value!(["a"])),
            document_type_schema(
                platform_value!({
                    "a": string(0, 10),
                    "meta": meta_yz(),
                    "meta_y": with(boolean(2), "requiredSince", Value::U32(2)),
                }),
                platform_value!(["a", "meta_y"]),
            ),
            Expected::Refused(10276, "meta.y"),
        ),
        case(
            "a new object whose member an existing property's name makes required",
            document_type_schema(
                platform_value!({ "a": string(0, 10), "meta_y": string(1, 10) }),
                platform_value!(["a", "meta_y"]),
            ),
            document_type_schema(
                platform_value!({
                    "a": string(0, 10),
                    "meta_y": string(1, 10),
                    "meta": object(platform_value!({ "y": boolean(0), "z": integer(1) }), 2, None),
                }),
                platform_value!(["a", "meta_y"]),
            ),
            Expected::Refused(10276, "meta.y"),
        ),
        case(
            "a new object whose name only looks like an existing object's prefix",
            required(platform_value!(["a"])),
            document_type_schema(
                platform_value!({
                    "a": string(0, 10),
                    "meta": meta_yz(),
                    "meta_": object(platform_value!({ "y": boolean(0) }), 2, Some(platform_value!(["y"]))),
                }),
                platform_value!(["a"]),
            ),
            Expected::Accepted,
        ),
        case(
            "an unchanged type",
            required(platform_value!(["a"])),
            required(platform_value!(["a"])),
            Expected::Accepted,
        ),
        Case {
            name: "a definition's members written in another map order, their positions kept",
            old: document_type_schema(
                platform_value!({ "a": string(0, 10), "meta": meta_ref(None) }),
                platform_value!(["a"]),
            ),
            new: document_type_schema(
                platform_value!({ "a": string(0, 10), "meta": meta_ref(None) }),
                platform_value!(["a"]),
            ),
            old_defs: definitions(platform_value!({ "y": boolean(0), "z": integer(1) })),
            new_defs: definitions(platform_value!({ "z": integer(1), "y": boolean(0) })),
            expected: Expected::Accepted,
        },
        Case {
            name: "a member written first in a definition's map, at a position after its existing ones",
            old: document_type_schema(
                platform_value!({ "a": string(0, 10), "meta": meta_ref(None) }),
                platform_value!(["a"]),
            ),
            new: document_type_schema(
                platform_value!({ "a": string(0, 10), "meta": meta_ref(None) }),
                platform_value!(["a"]),
            ),
            old_defs: definitions(platform_value!({ "y": boolean(0), "z": integer(1) })),
            new_defs: definitions(
                platform_value!({ "x": boolean(2), "y": boolean(0), "z": integer(1) }),
            ),
            expected: Expected::Accepted,
        },
        Case {
            name: "a member written last in a definition's map, at a position before an existing one",
            old: document_type_schema(
                platform_value!({ "a": string(0, 10), "meta": meta_ref(None) }),
                platform_value!(["a"]),
            ),
            new: document_type_schema(
                platform_value!({ "a": string(0, 10), "meta": meta_ref(None) }),
                platform_value!(["a"]),
            ),
            old_defs: definitions(platform_value!({ "z": integer(1) })),
            new_defs: definitions(platform_value!({ "z": integer(1), "x": boolean(0) })),
            expected: ORDER,
        },
        Case {
            name: "a member added after a definition's existing ones",
            old: document_type_schema(
                platform_value!({ "a": string(0, 10), "meta": meta_ref(None) }),
                platform_value!(["a"]),
            ),
            new: document_type_schema(
                platform_value!({ "a": string(0, 10), "meta": meta_ref(None) }),
                platform_value!(["a"]),
            ),
            old_defs: definitions(platform_value!({ "y": boolean(0), "z": integer(1) })),
            new_defs: definitions(
                platform_value!({ "y": boolean(0), "z": integer(1), "x": boolean(2) }),
            ),
            expected: Expected::Accepted,
        },
        Case {
            name: "a dotted name added to the top-level required for a definition's member",
            old: document_type_schema(
                platform_value!({ "a": string(0, 10), "meta": meta_ref(None) }),
                platform_value!(["a"]),
            ),
            new: document_type_schema(
                platform_value!({ "a": string(0, 10), "meta": meta_ref(None) }),
                platform_value!(["a", "meta.y"]),
            ),
            old_defs: definitions(platform_value!({ "y": boolean(0), "z": integer(1) })),
            new_defs: definitions(platform_value!({ "y": boolean(0), "z": integer(1) })),
            expected: Expected::Refused(10276, "meta.y"),
        },
        Case {
            name: "a name that is no member added to the required beside a $ref",
            old: document_type_schema(
                platform_value!({
                    "a": string(0, 10),
                    "meta": meta_ref(Some(("required", platform_value!(["y"])))),
                }),
                platform_value!(["a"]),
            ),
            new: document_type_schema(
                platform_value!({
                    "a": string(0, 10),
                    "meta": meta_ref(Some(("required", platform_value!(["y", "q"])))),
                }),
                platform_value!(["a"]),
            ),
            old_defs: definitions(platform_value!({ "y": boolean(0), "z": integer(1) })),
            new_defs: definitions(platform_value!({ "y": boolean(0), "z": integer(1) })),
            expected: Expected::Refused(10276, "beside the $ref"),
        },
    ]
}

#[test]
fn should_hold_every_object_to_its_existing_members_on_update() {
    run(&member_cases(), PlatformVersion::latest());
}

/// A private type is held to the same rules as any other: its entries are read with the type
/// as it is now.
#[test]
fn should_hold_a_private_type_s_objects_to_their_requiredness_on_update() {
    let private = |properties: Value, required: Value| {
        platform_value!({
            "type": "object",
            "private": { "entrySize": 704 },
            "documentsMutable": false,
            "canBeDeleted": false,
            "properties": properties,
            "required": required,
            "additionalProperties": false,
        })
    };
    let nested = || {
        with_meta(object(
            platform_value!({
                "y": boolean(0),
                "inner": object(platform_value!({ "p": boolean(0) }), 1, None),
            }),
            1,
            None,
        ))
    };
    let cases = vec![
        case(
            "a dotted name added to a private type's required",
            private(nested(), platform_value!(["a"])),
            private(nested(), platform_value!(["a", "meta.y"])),
            Expected::Refused(10276, "meta.y"),
        ),
        case(
            "a dotted name removed from a private type's required",
            private(nested(), platform_value!(["a", "meta.y"])),
            private(nested(), platform_value!(["a"])),
            Expected::Refused(10276, "meta.y"),
        ),
        case(
            "a new object of a private type whose name makes an existing member required",
            private(nested(), platform_value!(["a"])),
            private(
                platform_value!({
                    "a": string(0, 10),
                    "meta": object(
                        platform_value!({
                            "y": boolean(0),
                            "inner": object(platform_value!({ "p": boolean(0) }), 1, None),
                        }),
                        1,
                        None,
                    ),
                    "meta_inner": object(
                        platform_value!({ "p": boolean(0) }),
                        2,
                        Some(platform_value!(["p"])),
                    ),
                }),
                platform_value!(["a"]),
            ),
            Expected::Refused(10276, "meta.inner.p"),
        ),
        case(
            "a member added after a private type's object's existing ones",
            private(nested(), platform_value!(["a"])),
            private(
                with_meta(object(
                    platform_value!({
                        "y": boolean(0),
                        "inner": object(platform_value!({ "p": boolean(0) }), 1, None),
                        "x": boolean(2),
                    }),
                    1,
                    None,
                )),
                platform_value!(["a"]),
            ),
            Expected::Accepted,
        ),
    ];
    run(&cases, PlatformVersion::latest());
}

/// Protocol version 14 keeps the verdicts its update validation always gave these edits.
#[test]
fn should_keep_the_protocol_version_14_verdicts_of_member_edits() {
    let platform_version = PlatformVersion::get(14).expect("expected protocol version 14");
    let schema_refusal = Expected::Refused(10246, "required");
    let required = |names: Value| document_type_schema(with_meta(meta_yz()), names);
    let cases = vec![
        case(
            "a member added after an object's existing ones",
            required(platform_value!(["a"])),
            document_type_schema(
                with_meta(object(
                    platform_value!({ "y": boolean(0), "z": integer(1), "x": boolean(2) }),
                    1,
                    None,
                )),
                platform_value!(["a"]),
            ),
            Expected::Accepted,
        ),
        case(
            "a new object holding required members of its own",
            document_type_schema(
                platform_value!({ "a": string(0, 10) }),
                platform_value!(["a"]),
            ),
            document_type_schema(
                with_meta(object(
                    platform_value!({ "y": boolean(0), "z": integer(1) }),
                    1,
                    Some(platform_value!(["y"])),
                )),
                platform_value!(["a"]),
            ),
            Expected::Accepted,
        ),
        case(
            "an existing member added to its object's required",
            required(platform_value!(["a"])),
            document_type_schema(
                with_meta(object(
                    platform_value!({ "y": boolean(0), "z": integer(1) }),
                    1,
                    Some(platform_value!(["y"])),
                )),
                platform_value!(["a"]),
            ),
            schema_refusal,
        ),
        case(
            "a member removed from its object's required",
            document_type_schema(
                with_meta(object(
                    platform_value!({ "y": boolean(0), "z": integer(1) }),
                    1,
                    Some(platform_value!(["y"])),
                )),
                platform_value!(["a"]),
            ),
            required(platform_value!(["a"])),
            schema_refusal,
        ),
        case(
            "a new member added with its object's required",
            required(platform_value!(["a"])),
            document_type_schema(
                with_meta(object(
                    platform_value!({ "y": boolean(0), "z": integer(1), "x": boolean(2) }),
                    1,
                    Some(platform_value!(["x"])),
                )),
                platform_value!(["a"]),
            ),
            schema_refusal,
        ),
    ];
    run(&cases, platform_version);
}

/// A member an object gains may be required from the contract version the update creates, as a
/// top-level property may: through its object's own `required`, at any depth, or through the
/// `required` beside a `$ref`.
#[test]
fn should_admit_a_member_required_from_the_version_the_update_creates() {
    let since = |version: u32| with(boolean(2), "requiredSince", Value::U32(version));
    let appended = |x: Value, required: Value| {
        document_type_schema(
            with_meta(object(
                platform_value!({ "y": boolean(0), "z": integer(1), "x": x }),
                1,
                Some(required),
            )),
            platform_value!(["a"]),
        )
    };
    let old = || document_type_schema(with_meta(meta_yz()), platform_value!(["a"]));
    let nested = |inner: Value| {
        document_type_schema(
            with_meta(object(
                platform_value!({ "y": boolean(0), "inner": inner }),
                1,
                None,
            )),
            platform_value!(["a"]),
        )
    };
    let cases = vec![
        case(
            "a member its object's required names from the new version",
            old(),
            appended(since(2), platform_value!(["x"])),
            Expected::Accepted,
        ),
        case(
            "a member of an object inside an object required from the new version",
            nested(object(platform_value!({ "p": boolean(0) }), 1, None)),
            nested(object(
                platform_value!({ "p": boolean(0), "q": with(boolean(1), "requiredSince", Value::U32(2)) }),
                1,
                Some(platform_value!(["q"])),
            )),
            Expected::Accepted,
        ),
        case(
            "a member required from an earlier version",
            old(),
            appended(since(1), platform_value!(["x"])),
            Expected::Refused(10276, "must carry requiredSince 2"),
        ),
        case(
            "an existing member required from the new version",
            old(),
            document_type_schema(
                with_meta(object(
                    platform_value!({
                        "y": with(boolean(0), "requiredSince", Value::U32(2)),
                        "z": integer(1),
                    }),
                    1,
                    Some(platform_value!(["y"])),
                )),
                platform_value!(["a"]),
            ),
            Expected::Refused(10276, "meta.y"),
        ),
        case(
            "a member required from the new version through a dotted name",
            old(),
            document_type_schema(
                with_meta(object(
                    platform_value!({ "y": boolean(0), "z": integer(1), "x": since(2) }),
                    1,
                    None,
                )),
                platform_value!(["a", "meta.x"]),
            ),
            Expected::Refused(10276, "meta.x"),
        ),
        Case {
            name: "a definition's member required beside its $ref, whose list stays as it is",
            old: document_type_schema(
                platform_value!({
                    "a": string(0, 10),
                    "meta": meta_ref(Some(("required", platform_value!(["y"])))),
                }),
                platform_value!(["a"]),
            ),
            new: document_type_schema(
                platform_value!({
                    "a": string(0, 10),
                    "meta": meta_ref(Some(("required", platform_value!(["y", "x"])))),
                }),
                platform_value!(["a"]),
            ),
            old_defs: definitions(platform_value!({ "y": boolean(0), "z": integer(1) })),
            new_defs: definitions(
                platform_value!({ "y": boolean(0), "z": integer(1), "x": since(2) }),
            ),
            expected: Expected::Refused(10276, "beside the $ref"),
        },
        case(
            "a member named required whose type changes",
            document_type_schema(
                with_meta(object(
                    platform_value!({ "y": boolean(0), "required": integer(1) }),
                    1,
                    None,
                )),
                platform_value!(["a"]),
            ),
            document_type_schema(
                with_meta(object(
                    platform_value!({ "y": boolean(0), "required": string(1, 10) }),
                    1,
                    None,
                )),
                platform_value!(["a"]),
            ),
            Expected::Refused(10246, "required"),
        ),
    ];
    run(&cases, PlatformVersion::latest());
}

/// The edits that keep, or break, what the parse and the document validator agree on, beyond
/// the order of the members and the members an update adds.
#[test]
fn should_keep_what_the_parse_and_the_validator_agree_on() {
    let meta_y_required = || {
        object(
            platform_value!({ "y": boolean(0), "z": integer(1) }),
            1,
            Some(platform_value!(["y"])),
        )
    };
    let below_ref = |inner_required: Option<Value>| {
        document_type_schema(
            platform_value!({
                "a": string(0, 10),
                "meta": meta_ref(Some((
                    "properties",
                    platform_value!({
                        "q": object(platform_value!({ "p": boolean(0) }), 0, inner_required),
                    }),
                ))),
            }),
            platform_value!(["a"]),
        )
    };
    let q_definition = || {
        Some(platform_value!({
            "m": {
                "type": "object",
                "properties": {
                    "q": object(platform_value!({ "p": boolean(0) }), 0, None),
                },
                "additionalProperties": false,
            }
        }))
    };
    let cases = vec![
        case(
            "a member its object's required names without requiredSince",
            document_type_schema(with_meta(meta_yz()), platform_value!(["a"])),
            document_type_schema(
                with_meta(object(
                    platform_value!({ "y": boolean(0), "z": integer(1), "x": boolean(2) }),
                    1,
                    Some(platform_value!(["x"])),
                )),
                platform_value!(["a"]),
            ),
            Expected::Refused(10276, "must carry requiredSince 2"),
        ),
        case(
            "a member left out of its object's required while another property's name requires it",
            document_type_schema(
                platform_value!({ "a": string(0, 10), "meta": meta_y_required(), "meta_y": string(2, 10) }),
                platform_value!(["a", "meta_y"]),
            ),
            document_type_schema(
                platform_value!({ "a": string(0, 10), "meta": meta_yz(), "meta_y": string(2, 10) }),
                platform_value!(["a", "meta_y"]),
            ),
            Expected::Refused(10276, "removed required field 'meta.y'"),
        ),
        case(
            "a dotted name dropped while its object's required still names the member",
            document_type_schema(
                with_meta(meta_y_required()),
                platform_value!(["a", "meta.y"]),
            ),
            document_type_schema(with_meta(meta_y_required()), platform_value!(["a"])),
            Expected::Accepted,
        ),
        case(
            "a member removed after an object's other members",
            document_type_schema(with_meta(meta_yz()), platform_value!(["a"])),
            document_type_schema(
                with_meta(object(platform_value!({ "y": boolean(0) }), 1, None)),
                platform_value!(["a"]),
            ),
            Expected::Refused(10246, "remove"),
        ),
        case(
            "a top-level property removed after the others",
            document_type_schema(with_meta(meta_yz()), platform_value!(["a"])),
            document_type_schema(
                platform_value!({ "a": string(0, 10) }),
                platform_value!(["a"]),
            ),
            Expected::Refused(10246, "remove"),
        ),
        Case {
            name: "a name that is no member added to a required list below a $ref",
            old: below_ref(None),
            new: below_ref(Some(platform_value!(["ghost"]))),
            old_defs: q_definition(),
            new_defs: q_definition(),
            expected: Expected::Refused(10246, "required"),
        },
    ];
    run(&cases, PlatformVersion::latest());
}

/// An object the update adds is held to the dotted-name rule too: the document validator reads
/// a dotted name in its `required` as a property named with a dot, which no document can hold.
/// Its own member requirements, written on the object that holds the member, are accepted.
#[test]
fn should_refuse_a_dotted_name_in_the_required_of_an_object_the_update_adds() {
    let inner = || object(platform_value!({ "p": boolean(0) }), 1, None);
    let cases = vec![
        case(
            "a new object whose required names a member of its own object member",
            document_type_schema(
                platform_value!({ "a": string(0, 10) }),
                platform_value!(["a"]),
            ),
            document_type_schema(
                with_meta(object(
                    platform_value!({ "y": boolean(0), "inner": inner() }),
                    1,
                    Some(platform_value!(["inner.p"])),
                )),
                platform_value!(["a"]),
            ),
            Expected::Refused(10276, "inner.p"),
        ),
        case(
            "a member added to an existing object, whose required names a member of its member",
            document_type_schema(
                with_meta(object(platform_value!({ "y": boolean(0) }), 1, None)),
                platform_value!(["a"]),
            ),
            document_type_schema(
                with_meta(object(
                    platform_value!({
                        "y": boolean(0),
                        "added": object(
                            platform_value!({ "inner": object(platform_value!({ "p": boolean(0) }), 0, None) }),
                            1,
                            Some(platform_value!(["inner.p"])),
                        ),
                    }),
                    1,
                    None,
                )),
                platform_value!(["a"]),
            ),
            Expected::Refused(10276, "inner.p"),
        ),
        case(
            "a new object whose member object requires its own member",
            document_type_schema(
                platform_value!({ "a": string(0, 10) }),
                platform_value!(["a"]),
            ),
            document_type_schema(
                with_meta(object(
                    platform_value!({
                        "y": boolean(0),
                        "inner": object(platform_value!({ "p": boolean(0) }), 1, Some(platform_value!(["p"]))),
                    }),
                    1,
                    None,
                )),
                platform_value!(["a"]),
            ),
            Expected::Accepted,
        ),
    ];
    run(&cases, PlatformVersion::latest());
}

/// The `required` written beside a `$ref` stays as it is, at any depth of the type's own schema:
/// the object takes its members from a definition other types share, so it gains members only as
/// optional. On a `$ref` to a scalar the list stays frozen too.
#[test]
fn should_keep_the_required_beside_a_ref_as_it_is() {
    let with_q = |q_required: Value| {
        document_type_schema(
            with_meta(object(
                platform_value!({
                    "y": boolean(0),
                    "q": with(
                        platform_value!({ "$ref": "#/$defs/m", "position": 1 }),
                        "required",
                        q_required,
                    ),
                }),
                1,
                None,
            )),
            platform_value!(["a"]),
        )
    };
    let scalar = |required: Option<Value>| {
        let reference = platform_value!({ "$ref": "#/$defs/s", "position": 1 });
        document_type_schema(
            platform_value!({
                "a": string(0, 10),
                "label": match required {
                    Some(required) => with(reference, "required", required),
                    None => reference,
                },
            }),
            platform_value!(["a"]),
        )
    };
    let scalar_definition =
        || Some(platform_value!({ "s": { "type": "string", "maxLength": 10 } }));
    let cases = vec![
        Case {
            name: "a definition's member required beside a $ref inside an object",
            old: with_q(platform_value!(["y"])),
            new: with_q(platform_value!(["y", "x"])),
            old_defs: definitions(platform_value!({ "y": boolean(0), "z": integer(1) })),
            new_defs: definitions(platform_value!({
                "y": boolean(0),
                "z": integer(1),
                "x": with(boolean(2), "requiredSince", Value::U32(2)),
            })),
            expected: Expected::Refused(10276, "beside the $ref"),
        },
        Case {
            name: "a definition's member added as optional beside a $ref inside an object",
            old: with_q(platform_value!(["y"])),
            new: with_q(platform_value!(["y"])),
            old_defs: definitions(platform_value!({ "y": boolean(0), "z": integer(1) })),
            new_defs: definitions(
                platform_value!({ "y": boolean(0), "z": integer(1), "x": boolean(2) }),
            ),
            expected: Expected::Accepted,
        },
        Case {
            name: "a required list added beside a $ref to a scalar",
            old: scalar(None),
            new: scalar(Some(platform_value!(["x"]))),
            old_defs: scalar_definition(),
            new_defs: scalar_definition(),
            expected: Expected::Refused(10246, "required"),
        },
    ];
    run(&cases, PlatformVersion::latest());
}

/// An object holds at most `maxProperties` members, which an update can not change: its
/// `required` may not grow past that, or no document could hold the object.
#[test]
fn should_refuse_a_required_list_growing_past_its_object_s_max_properties() {
    let capped = |max_properties: u32, with_x: bool| {
        let mut members = platform_value!({ "y": boolean(0) });
        let mut required = vec![Value::Text("y".to_string())];
        if with_x {
            members
                .set_value("x", with(boolean(1), "requiredSince", Value::U32(2)))
                .expect("expected a map");
            required.push(Value::Text("x".to_string()));
        }
        let meta = with(
            object(members, 1, Some(Value::Array(required))),
            "maxProperties",
            Value::U32(max_properties),
        );
        document_type_schema(with_meta(meta), platform_value!(["a", "meta"]))
    };
    let cases = vec![
        case(
            "a required list growing past maxProperties",
            capped(1, false),
            capped(1, true),
            Expected::Refused(10276, "maxProperties"),
        ),
        case(
            "a required list growing within maxProperties",
            capped(2, false),
            capped(2, true),
            Expected::Accepted,
        ),
    ];
    run(&cases, PlatformVersion::latest());
}

/// A replace sends the whole document again, so a document stored before an object gained a
/// required member must add it; under an immutable property it can not change the object, so
/// such a member is refused there. An optional member may still be added.
#[test]
fn should_refuse_a_member_required_from_the_new_version_under_an_immutable_property() {
    let immutable_meta = |members: Value, required: Option<Value>| {
        with(
            document_type_schema(
                with_meta(object(members, 1, required)),
                platform_value!(["a"]),
            ),
            "immutable",
            platform_value!(["meta"]),
        )
    };
    let cases = vec![
        case(
            "a member required from the new version under an immutable property",
            immutable_meta(platform_value!({ "y": boolean(0) }), None),
            immutable_meta(
                platform_value!({ "y": boolean(0), "x": with(boolean(1), "requiredSince", Value::U32(2)) }),
                Some(platform_value!(["x"])),
            ),
            Expected::Refused(10276, "immutable"),
        ),
        case(
            "an optional member under an immutable property",
            immutable_meta(platform_value!({ "y": boolean(0) }), None),
            immutable_meta(platform_value!({ "y": boolean(0), "x": boolean(1) }), None),
            Expected::Accepted,
        ),
    ];
    run(&cases, PlatformVersion::latest());
}

/// A member required from the new version is refused where a stored document could not be
/// shown to add it on replace: inside an object declaring a whole-object `const` or `enum`, at
/// any depth. An update that drops the `const` may add it.
#[test]
fn should_refuse_a_member_required_from_the_new_version_that_no_document_could_meet() {
    let since = |version: u32| with(boolean(1), "requiredSince", Value::U32(version));
    let constrained = |keyword: &str, constraint: Value, with_x: bool| {
        let members = if with_x {
            platform_value!({ "y": boolean(0), "x": since(2) })
        } else {
            platform_value!({ "y": boolean(0) })
        };
        let required = if with_x {
            platform_value!(["y", "x"])
        } else {
            platform_value!(["y"])
        };
        document_type_schema(
            with_meta(with(
                object(members, 1, Some(required)),
                keyword,
                constraint,
            )),
            platform_value!(["a"]),
        )
    };
    let cases = vec![
        case(
            "a member required from the new version inside an object declaring const, one level down",
            document_type_schema(
                with_meta(object(
                    platform_value!({
                        "y": boolean(0),
                        "inner": with(
                            object(platform_value!({ "p": boolean(0) }), 1, Some(platform_value!(["p"]))),
                            "const",
                            platform_value!({ "p": true }),
                        ),
                    }),
                    1,
                    None,
                )),
                platform_value!(["a"]),
            ),
            document_type_schema(
                with_meta(object(
                    platform_value!({
                        "y": boolean(0),
                        "inner": with(
                            object(
                                platform_value!({ "p": boolean(0), "x": since(2) }),
                                1,
                                Some(platform_value!(["p", "x"])),
                            ),
                            "const",
                            platform_value!({ "p": true }),
                        ),
                    }),
                    1,
                    None,
                )),
                platform_value!(["a"]),
            ),
            Expected::Refused(10276, "'meta.inner' declares const"),
        ),
        case(
            "a member required from the new version under an object declaring const",
            constrained("const", platform_value!({ "y": true }), false),
            constrained("const", platform_value!({ "y": true }), true),
            Expected::Refused(10276, "const"),
        ),
        case(
            "a member required from the new version under an object declaring enum",
            constrained("enum", platform_value!([{ "y": true }]), false),
            constrained("enum", platform_value!([{ "y": true }]), true),
            Expected::Refused(10276, "enum"),
        ),
        case(
            "a member required from the new version, the same update dropping the object's const",
            constrained("const", platform_value!({ "y": true }), false),
            document_type_schema(
                with_meta(object(
                    platform_value!({ "y": boolean(0), "x": since(2) }),
                    1,
                    Some(platform_value!(["y", "x"])),
                )),
                platform_value!(["a"]),
            ),
            Expected::Accepted,
        ),
    ];
    run(&cases, PlatformVersion::latest());
}

/// An object only moderators write is like an immutable one for its owner: a replace that adds
/// a member required from the new version would change it, so such a member is refused there.
#[test]
fn should_refuse_a_member_required_from_the_new_version_under_a_field_only_moderators_write() {
    use crate::data_contract::config::moderation::{ContractModerationConfig, ContractModerators};

    let platform_version = PlatformVersion::latest();
    let config = DataContractConfig::default_for_version(platform_version)
        .expect("expected a default config")
        .with_moderation(Some(ContractModerationConfig {
            banlist: true,
            suspensions: false,
            moderators: ContractModerators::ContractOwner,
            warnings: false,
        }));
    let make = |members: Value, required: Option<Value>| {
        let schema = with(
            document_type_schema(
                with_meta(object(members, 1, required)),
                platform_value!(["a"]),
            ),
            "moderatorAbilities",
            platform_value!({ "changeFields": ["meta"] }),
        );
        DocumentType::try_from_schema(
            Identifier::new([1; 32]),
            1,
            config.version(),
            "note",
            schema,
            None,
            &BTreeMap::new(),
            &config,
            false,
            &mut Vec::new(),
            platform_version,
        )
        .expect("expected the type to parse")
    };
    let old = make(platform_value!({ "y": boolean(0) }), None);
    let required = make(
        platform_value!({ "y": boolean(0), "x": with(boolean(1), "requiredSince", Value::U32(2)) }),
        Some(platform_value!(["x"])),
    );
    let optional = make(platform_value!({ "y": boolean(0), "x": boolean(1) }), None);

    let result = old
        .as_ref()
        .validate_update(required.as_ref(), 2, platform_version)
        .expect("expected the update to be judged");
    assert_verdict(
        "a member required from the new version under a field only moderators write",
        &result,
        Expected::Refused(10276, "moderators"),
    );
    let result = old
        .as_ref()
        .validate_update(optional.as_ref(), 2, platform_version)
        .expect("expected the update to be judged");
    assert_verdict(
        "an optional member under a field only moderators write",
        &result,
        Expected::Accepted,
    );
}

/// Through a `$ref` members are added only as optional, whatever lists the schema writes
/// beside the `$ref`; and the lists written beside a `$ref` an update adds hold no dotted name.
#[test]
fn should_hold_additions_through_a_ref_to_optional_members() {
    let overlay = |entries: Value| {
        document_type_schema(
            platform_value!({
                "a": string(0, 10),
                "meta": meta_ref(Some(("properties", entries))),
            }),
            platform_value!(["a"]),
        )
    };
    let r_entry = || object(platform_value!({ "b": boolean(0) }), 1, None);
    let definition = |with_p: bool| {
        let q_members = if with_p {
            platform_value!({
                "a": boolean(0),
                "p": with(boolean(1), "requiredSince", Value::U32(2)),
            })
        } else {
            platform_value!({ "a": boolean(0) })
        };
        definitions(platform_value!({
            "q": object(q_members, 0, None),
            "r": r_entry(),
        }))
    };
    let new_shared = |q_required: Value| {
        document_type_schema(
            platform_value!({
                "a": string(0, 10),
                "meta": meta_ref(None),
                "shared": with(
                    platform_value!({ "$ref": "#/$defs/n", "position": 2 }),
                    "properties",
                    platform_value!({
                        "q": object(platform_value!({ "a": boolean(0) }), 0, Some(q_required)),
                    }),
                ),
            }),
            platform_value!(["a"]),
        )
    };
    // A second definition, for the property an update adds beside `meta`: one type may not
    // reference a definition twice
    let with_n = |defs: Option<Value>| {
        let mut defs = defs.expect("expected definitions");
        defs.set_value(
            "n",
            platform_value!({
                "type": "object",
                "properties": { "q": object(platform_value!({ "a": boolean(0) }), 0, None) },
                "additionalProperties": false,
            }),
        )
        .expect("expected a map");
        Some(defs)
    };
    let cases = vec![
        Case {
            name: "a definition member made required through properties written beside a $ref",
            old: overlay(platform_value!({ "r": r_entry() })),
            new: overlay(platform_value!({
                "r": r_entry(),
                "q": object(
                    platform_value!({
                        "a": boolean(0),
                        "p": with(boolean(1), "requiredSince", Value::U32(2)),
                    }),
                    0,
                    Some(platform_value!(["p"])),
                ),
            })),
            old_defs: definition(false),
            new_defs: definition(true),
            expected: Expected::Refused(10276, "$ref"),
        },
        Case {
            name: "a dotted name in a list beside a $ref the update adds",
            old: document_type_schema(
                platform_value!({ "a": string(0, 10), "meta": meta_ref(None) }),
                platform_value!(["a"]),
            ),
            new: new_shared(platform_value!(["x.y"])),
            old_defs: with_n(definition(false)),
            new_defs: with_n(definition(false)),
            expected: Expected::Refused(10276, "x.y"),
        },
    ];
    run(&cases, PlatformVersion::latest());
}
