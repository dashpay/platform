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
            "an object's members reordered",
            required(platform_value!(["a"])),
            document_type_schema(
                with_meta(object(
                    platform_value!({ "z": integer(1), "y": boolean(0) }),
                    1,
                    None,
                )),
                platform_value!(["a"]),
            ),
            ORDER,
        ),
        case(
            "a member put before an object's existing ones",
            required(platform_value!(["a"])),
            document_type_schema(
                with_meta(object(
                    platform_value!({ "x": boolean(2), "y": boolean(0), "z": integer(1) }),
                    1,
                    None,
                )),
                platform_value!(["a"]),
            ),
            ORDER,
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
            "an object inside an object with its members reordered",
            document_type_schema(
                with_meta(object(
                    platform_value!({
                        "y": boolean(0),
                        "inner": object(platform_value!({ "p": boolean(0), "q": integer(1) }), 1, None),
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
            name: "a definition's members reordered",
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
            expected: ORDER,
        },
        Case {
            name: "a member put before a definition's existing ones",
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
            expected: Expected::Refused(10276, "meta.q"),
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
            name: "a definition's member required from the new version beside its $ref",
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
            expected: Expected::Accepted,
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
