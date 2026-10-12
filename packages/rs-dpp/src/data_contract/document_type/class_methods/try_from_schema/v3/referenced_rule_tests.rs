//! `propertyConstraints` rules reading the document a reference points at,
//! `{ "referenced": [reference, field] }`, and the `where` entries that are not
//! an equality with a referring value (protocol version 14). The parse reads
//! both into the document type; what a rule reads of a referenced type of the
//! same contract is checked once every type is parsed, and of another
//! contract's type at registration.

use super::immutable_tests::expect_structure_error;
use crate::consensus::basic::document::PropertyConstraintViolation;
use crate::data_contract::config::moderation::{ContractModerationConfig, ContractModerators};
use crate::data_contract::config::DataContractConfig;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::accessors::DocumentTypeV2Getters;
use crate::data_contract::document_type::class_methods::where_condition_literal_error;
use crate::data_contract::document_type::methods::DocumentTypeV0Methods;
use crate::data_contract::document_type::property_constraints::{
    data_with_referenced_documents, referenced_fields, DocumentSystemValues,
    REFERENCED_DOCUMENT_KEY,
};
use crate::data_contract::document_type::property_constraints::{PropertyRead, SystemProperty};
use crate::data_contract::document_type::{
    DocumentPropertyReferenceTarget, DocumentPropertyType, DocumentType, DocumentTypeRef,
    WhereCondition, WhereLiteral, WhereLiteralKind, WhereNotEqualBound, WhereOrdering,
    WhereOrderingBound,
};
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_value::string_encoding::Encoding;
use platform_value::{platform_value, Identifier, Value};
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

const CONTRACT_ID: [u8; 32] = [7; 32];

fn identifier(position: u64, refers_to: Option<Value>) -> Value {
    let mut property = platform_value!({
        "type": "array",
        "byteArray": true,
        "minItems": 32,
        "maxItems": 32,
        "contentMediaType": "application/x.dash.dpp.identifier",
        "position": position,
    });
    if let (Some(refers_to), Value::Map(map)) = (refers_to, &mut property) {
        map.push((Value::Text("refersTo".to_string()), refers_to));
    }
    property
}

/// Pollr's poll: a question and options, whether voters may pick more than one
/// (`multiChoice`), when it closes (`endsAt`, optional), and a `status` with an
/// enum. Its owner may delete it, so a reference to it is a deletableDocument
/// one. `note` is transient.
fn poll() -> Value {
    platform_value!({
        "type": "object",
        "documentsMutable": false,
        "properties": {
            "question": { "type": "string", "minLength": 1, "maxLength": 512, "position": 0 },
            "option0": { "type": "string", "minLength": 1, "maxLength": 100, "position": 1 },
            "option1": { "type": "string", "minLength": 1, "maxLength": 100, "position": 2 },
            "option2": { "type": "string", "minLength": 1, "maxLength": 100, "position": 3 },
            "multiChoice": { "type": "boolean", "position": 4 },
            "endsAt": { "type": "integer", "minimum": 0, "maximum": 9007199254740991u64, "position": 5 },
            "status": { "type": "string", "enum": ["open", "closed"], "maxLength": 10, "position": 6 },
            "note": { "type": "string", "maxLength": 20, "position": 7 },
            "voters": {
                "type": "array",
                "minItems": 0,
                "maxItems": 5,
                "items": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier"
                },
                "position": 8
            },
            "curatorId": identifier(9, None),
            "payload": { "type": "array", "byteArray": true, "maxItems": 34, "position": 10 },
        },
        "transient": ["note"],
        "required": ["question", "option0", "option1", "$createdAt"],
        "additionalProperties": false,
    })
}

/// A post only moderators take down, each removal keeping `hashtag`.
fn moderated_post() -> Value {
    platform_value!({
        "type": "object",
        "documentsMutable": false,
        "canBeDeleted": false,
        "moderatorAbilities": { "delete": true, "deleteKeepsFields": ["hashtag"] },
        "properties": {
            "hashtag": { "type": "string", "minLength": 1, "maxLength": 63, "position": 0 },
            "score": { "type": "integer", "minimum": 0, "maximum": 100, "position": 1 },
        },
        "required": ["hashtag", "$createdAt"],
        "additionalProperties": false,
    })
}

/// A vote on a poll with `rules`, whose `pollId` refers to the poll as
/// `refers_to` says. `extra` adds properties and keywords to the type.
fn vote(refers_to: Value, rules: Value, extra: Vec<(&str, Value)>) -> Value {
    let mut vote = platform_value!({
        "type": "object",
        "documentsMutable": true,
        "canBeDeleted": true,
        "properties": {
            "pollId": identifier(0, Some(refers_to)),
            "pollOwnerId": identifier(1, None),
            "choice": { "type": "integer", "minimum": 0, "maximum": 9, "position": 2 },
            "label": { "type": "string", "maxLength": 20, "position": 3 },
            "postId": identifier(4, Some(platform_value!({
                "type": "moderatedDocument",
                "documentType": "post",
            }))),
            "voterId": identifier(5, Some(platform_value!({ "type": "identity" }))),
        },
        "required": ["pollId", "pollOwnerId", "choice", "$createdAt", "$updatedAt"],
        "propertyConstraints": rules,
        "additionalProperties": false,
    });
    for (key, value) in extra {
        vote.set_value(key, value)
            .expect("the vote takes the keyword");
    }
    vote
}

fn poll_reference() -> Value {
    platform_value!({ "type": "deletableDocument", "documentType": "poll" })
}

fn config() -> DataContractConfig {
    DataContractConfig::default_for_version(PlatformVersion::latest())
        .expect("default config available")
        .with_moderation(Some(ContractModerationConfig {
            banlist: false,
            suspensions: false,
            moderators: ContractModerators::ContractOwner,
            warnings: false,
        }))
}

fn parse_types(
    types: Vec<(&str, Value)>,
    full_validation: bool,
) -> Result<BTreeMap<String, DocumentType>, ProtocolError> {
    let config = config();
    DocumentType::create_document_types_from_document_schemas(
        Identifier::new(CONTRACT_ID),
        1,
        config.version(),
        types
            .into_iter()
            .map(|(name, schema)| (name.to_string(), schema))
            .collect(),
        None,
        &BTreeMap::new(),
        &config,
        full_validation,
        false,
        &mut vec![],
        PlatformVersion::latest(),
    )
}

fn parse(vote: Value) -> Result<BTreeMap<String, DocumentType>, ProtocolError> {
    parse_types(
        vec![("poll", poll()), ("post", moderated_post()), ("vote", vote)],
        true,
    )
}

fn parse_rules(rules: Value) -> Result<BTreeMap<String, DocumentType>, ProtocolError> {
    parse(vote(poll_reference(), rules, vec![]))
}

fn pollr_rules() -> Value {
    platform_value!({
        "castBeforeTheEnd": {
            "lessThan": [
                "$createdAt",
                { "ifAbsent": [{ "referenced": ["pollId", "endsAt"] }, 9007199254740991u64] }
            ]
        },
        "singleChoicePoll": { "equal": [{ "referenced": ["pollId", "multiChoice"] }, 0] },
        "choiceIsAnOption": {
            "lessThan": [
                "choice",
                { "countPresent": [
                    { "referenced": ["pollId", "option0"] },
                    { "referenced": ["pollId", "option1"] },
                    { "referenced": ["pollId", "option2"] }
                ] }
            ]
        },
        "pollIsOpen": {
            "notEqual": [{ "referenced": ["pollId", "status"] }, { "const": "closed" }]
        },
        "ownerIsTheDenormalizedOne": {
            "equal": [{ "referenced": ["pollId", "$ownerId"] }, "pollOwnerId"]
        },
        "labelMatchesStatus": {
            "anyOf": [
                { "absent": "label" },
                { "equal": ["label", { "referenced": ["pollId", "status"] }] }
            ]
        },
        "voterListed": {
            "ifThen": [
                { "present": "voterId" },
                { "contains": [{ "referenced": ["pollId", "voters"] }, "voterId"] }
            ]
        }
    })
}

#[test]
fn should_parse_rules_reading_the_referenced_document() {
    let types = parse_rules(pollr_rules()).expect("the rules parse");
    let vote = types.get("vote").expect("the vote type").as_ref();
    let fields = referenced_fields(vote.property_constraints());
    assert_eq!(
        fields
            .get("pollId")
            .expect("the rules read through pollId")
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        vec![
            "$ownerId",
            "endsAt",
            "multiChoice",
            "option0",
            "option1",
            "option2",
            "status",
            "voters"
        ]
    );
    assert!(vote
        .property_constraints()
        .values()
        .all(|rule| rule.reads_referenced()));
}

/// The data a vote with `vote_data` is judged against, with the poll's
/// `poll_values` under the reference.
fn judged(vote_data: Value, poll_values: Value) -> Value {
    let data: BTreeMap<String, Value> = vote_data
        .into_btree_string_map()
        .expect("the vote data is a map");
    data_with_referenced_documents(&data, BTreeMap::from([("pollId".to_string(), poll_values)]))
        .expect("the values are put under the reference")
}

/// Judges `vote_data`, a vote of `vote`, by the rules reading the poll it
/// names, whose values are `poll_values`, with `system`.
fn judge(
    vote: DocumentTypeRef,
    vote_data: Value,
    poll_values: Value,
    system: &DocumentSystemValues,
) -> SimpleConsensusValidationResult {
    let data: BTreeMap<String, Value> = vote_data
        .into_btree_string_map()
        .expect("the vote data is a map");
    vote.validate_referencing_property_constraints(
        &data,
        BTreeMap::from([("pollId".to_string(), poll_values)]),
        system.clone(),
        PlatformVersion::latest(),
    )
    .expect("the rules evaluate")
}

#[test]
fn should_judge_a_rule_by_the_values_of_the_referenced_document() {
    let types = parse_rules(pollr_rules()).expect("the rules parse");
    let vote = types.get("vote").expect("the vote type").as_ref();
    let owner = Identifier::new([3; 32]);
    let poll_owner = Value::Identifier([4; 32]);
    let system = DocumentSystemValues {
        created_at: Some(1_000),
        updated_at: Some(1_000),
        referenced_documents: true,
        ..DocumentSystemValues::owned_by(owner)
    };
    let open_poll = platform_value!({
        "$ownerId": poll_owner.clone(),
        "endsAt": 2_000u64,
        "option0": "a",
        "option1": "b",
        "status": "open",
    });
    let poll_id = Value::Identifier([5; 32]);
    let vote_data = platform_value!({ "pollId": poll_id.clone(), "pollOwnerId": poll_owner.clone(), "choice": 1u8 });
    let result = judge(vote, vote_data.clone(), open_poll.clone(), &system);
    assert!(result.is_valid(), "{:?}", result.errors);

    // The poll has two options, so choice 2 is none of them
    let result = judge(
        vote,
        platform_value!({ "pollId": poll_id.clone(), "pollOwnerId": poll_owner.clone(), "choice": 2u8 }),
        open_poll.clone(),
        &system,
    );
    assert!(format!("{:?}", result.errors).contains("choiceIsAnOption"));

    // Past the end
    let mut ended = open_poll.clone();
    ended.set_value("endsAt", Value::U64(900)).expect("set");
    let result = judge(vote, vote_data.clone(), ended, &system);
    assert!(format!("{:?}", result.errors).contains("castBeforeTheEnd"));

    // A multiple-choice poll is no single-choice vote's
    let mut multi = open_poll.clone();
    multi
        .set_value("multiChoice", Value::Bool(true))
        .expect("set");
    let result = judge(vote, vote_data.clone(), multi, &system);
    assert!(format!("{:?}", result.errors).contains("singleChoicePoll"));

    // A poll without an end is open for good: the default stands in for endsAt
    let mut endless = open_poll;
    endless.remove("endsAt").expect("remove");
    let result = judge(vote, vote_data.clone(), endless, &system);
    assert!(result.is_valid(), "{:?}", result.errors);
}

#[test]
fn should_leave_rules_reading_a_referenced_document_unjudged_without_its_values() {
    let types = parse_rules(pollr_rules()).expect("the rules parse");
    let vote = types.get("vote").expect("the vote type").as_ref();
    let rule = vote
        .property_constraints()
        .get("castBeforeTheEnd")
        .expect("the rule");
    // A client, or consensus before the references are checked, gives none
    let system = DocumentSystemValues {
        created_at: Some(5_000),
        ..DocumentSystemValues::owned_by(Identifier::new([3; 32]))
    };
    let data = platform_value!({ "choice": 0u8 });
    assert_eq!(rule.violation(&data, &system), None);
    let result = vote
        .validate_property_constraints(&data, &system, PlatformVersion::latest())
        .expect("the rules evaluate");
    assert!(result.is_valid());
    // With the values given, a missing endsAt reads as the default and holds;
    // an endsAt before the creation does not
    let with = DocumentSystemValues {
        referenced_documents: true,
        ..system
    };
    let data = judged(
        platform_value!({ "choice": 0u8 }),
        platform_value!({ "endsAt": 4_000u64 }),
    );
    assert_eq!(
        rule.violation(&data, &with),
        Some(PropertyConstraintViolation::NotMet)
    );
    assert!(data.get_optional_value(REFERENCED_DOCUMENT_KEY).is_ok());
}

#[test]
fn should_refuse_a_referenced_read_through_a_property_without_a_single_document_reference() {
    for (rule, needle) in [
        (
            platform_value!({ "lessThan": [{ "referenced": ["choice", "endsAt"] }, 5] }),
            "carries no refersTo",
        ),
        (
            platform_value!({ "lessThan": [{ "referenced": ["missing", "endsAt"] }, 5] }),
            "is not a property of the document type",
        ),
        (
            platform_value!({ "equal": [{ "referenced": ["voterId", "$ownerId"] }, "pollOwnerId"] }),
            "names an identity",
        ),
    ] {
        expect_structure_error(parse_rules(platform_value!({ "rule": rule })), needle);
    }
}

#[test]
fn should_refuse_a_referenced_read_through_elements_an_expression_or_a_list() {
    let mut item = identifier(0, Some(poll_reference()));
    item.remove("position").expect("an item has no position");
    let mut elements = vote(
        poll_reference(),
        platform_value!({ "rule": { "lessThan": [{ "referenced": ["polls", "endsAt"] }, 5] } }),
        vec![],
    );
    elements
        .get_mut("properties")
        .expect("properties accessible")
        .expect("properties present")
        .set_value(
            "polls",
            platform_value!({
                "type": "array",
                "minItems": 0,
                "maxItems": 3,
                "items": item,
                "position": 6
            }),
        )
        .expect("the property applies");
    expect_structure_error(parse(elements), "elements of a typed array");

    // A topic that never leaves state, so a permanentDocument reference may
    // be an operand of an expression
    let topic = platform_value!({
        "type": "object",
        "documentsMutable": false,
        "canBeDeleted": false,
        "properties": {
            "rank": { "type": "integer", "minimum": 0, "maximum": 9, "position": 0 },
        },
        "required": ["rank"],
        "additionalProperties": false,
    });
    let any_of = vote(
        platform_value!({ "anyOf": [
            { "type": "permanentDocument", "documentType": "topic" },
            { "type": "identity" }
        ] }),
        platform_value!({ "rule": { "lessThan": [{ "referenced": ["pollId", "rank"] }, 5] } }),
        vec![],
    );
    expect_structure_error(
        parse_types(
            vec![
                ("poll", poll()),
                ("post", moderated_post()),
                ("topic", topic),
                ("vote", any_of),
            ],
            true,
        ),
        "anyOf or allOf",
    );
}

#[test]
fn should_refuse_a_read_the_referenced_type_can_not_answer() {
    for (rule, needle) in [
        // A string read as a number
        (
            platform_value!({ "lessThan": [{ "referenced": ["pollId", "question"] }, 5] }),
            "has type string, not integer",
        ),
        // No such property
        (
            platform_value!({ "lessThan": [{ "referenced": ["pollId", "nothing"] }, 5] }),
            "is not an integer or boolean property",
        ),
        // Transient on the poll
        (
            platform_value!({ "equal": [{ "referenced": ["pollId", "note"] }, { "const": "x" }] }),
            "transient",
        ),
        // A time the poll does not record
        (
            platform_value!({ "lessThan": [{ "referenced": ["pollId", "$updatedAt"] }, "$createdAt"] }),
            "does not record",
        ),
        // The poll records no creator
        (
            platform_value!({ "equal": [{ "referenced": ["pollId", "$creatorId"] }, "pollOwnerId"] }),
            "does not record $creatorId",
        ),
        // A constant the poll's enum does not list
        (
            platform_value!({ "equal": [{ "referenced": ["pollId", "status"] }, { "const": "paused" }] }),
            "not one of its enum values",
        ),
        // An identifier is compared, not counted
        (
            platform_value!({ "lessThan": [{ "referenced": ["pollId", "$ownerId"] }, 5] }),
            "an identifier",
        ),
    ] {
        expect_structure_error(parse_rules(platform_value!({ "rule": rule })), needle);
    }
}

/// How a referenced value compares follows the rule, since the referenced type
/// may be another contract's: beside a `const` it reads as a string. So a
/// referenced identifier schema property is compared with an identifier
/// operand of the type, while the referenced document's own identifiers
/// (`$id`, `$ownerId`, `$creatorId`), always identifiers, take a base58 `const`
/// too.
/// The parse of a rule does not know what a field of a referenced document
/// holds, so a string constant compared with one reads as the field holds: an
/// identifier written base58, a byte array hex. Registration, with the
/// referenced type in hand, checks the constant spells one.
#[test]
fn should_compare_a_referenced_field_as_what_it_holds() {
    let base58 = Identifier::new([4; 32]).to_string(Encoding::Base58);
    parse_rules(platform_value!({
        "rule": { "equal": [{ "referenced": ["pollId", "curatorId"] }, "pollOwnerId"] }
    }))
    .expect("an identifier property of the type compares with it");
    parse_rules(platform_value!({
        "rule": { "notEqual": [{ "referenced": ["pollId", "$ownerId"] }, { "const": base58.clone() }] }
    }))
    .expect("the referenced document's owner takes a base58 const");
    let types = parse_rules(platform_value!({
        "curated": { "equal": [{ "referenced": ["pollId", "curatorId"] }, { "const": base58.clone() }] },
        "listed": { "in": [{ "referenced": ["pollId", "curatorId"] }, [base58.clone(), Identifier::new([5; 32]).to_string(Encoding::Base58)]] },
        "multihash": { "startsWith": [{ "referenced": ["pollId", "payload"] }, { "const": "1220" }] },
        "voterListed": { "contains": [{ "referenced": ["pollId", "voters"] }, { "const": base58.clone() }] }
    }))
    .expect("an identifier takes base58 constants and a byte array a hex affix");
    let vote = types.get("vote").expect("the vote type").as_ref();
    let system = DocumentSystemValues::owned_by(Identifier::new([3; 32]));
    let vote_data = platform_value!({ "pollId": Value::Identifier([5; 32]), "pollOwnerId": Value::Identifier([4; 32]), "choice": 0u8 });
    // The values as a stored poll decodes them: identifiers as identifiers, a
    // byte array as bytes
    let poll = |curator: u8, payload: Vec<u8>, voter: u8| {
        Value::Map(vec![
            (
                Value::Text("curatorId".to_string()),
                Value::Identifier([curator; 32]),
            ),
            (Value::Text("payload".to_string()), Value::Bytes(payload)),
            (
                Value::Text("voters".to_string()),
                Value::Array(vec![Value::Identifier([voter; 32])]),
            ),
        ])
    };
    let result = judge(
        vote,
        vote_data.clone(),
        poll(4, vec![0x12, 0x20, 0xab], 4),
        &system,
    );
    assert!(result.is_valid(), "{:?}", result.errors);
    for (poll_values, broken) in [
        (poll(6, vec![0x12, 0x20], 4), "curated"),
        (poll(4, vec![0x12, 0x21], 4), "multihash"),
        (poll(4, vec![0x12, 0x20], 7), "voterListed"),
    ] {
        let result = judge(vote, vote_data.clone(), poll_values, &system);
        assert!(
            format!("{:?}", result.errors).contains(broken),
            "expected {broken} to break: {:?}",
            result.errors
        );
    }

    for (rule, needle) in [
        (
            platform_value!({ "equal": [{ "referenced": ["pollId", "curatorId"] }, { "const": "not base58 0OIl" }] }),
            "which spells none",
        ),
        (
            platform_value!({ "startsWith": [{ "referenced": ["pollId", "payload"] }, { "const": "zz" }] }),
            "which is not hex",
        ),
        (
            platform_value!({ "startsWith": [{ "referenced": ["pollId", "curatorId"] }, { "const": "12" }] }),
            "which has no start or end",
        ),
        (
            platform_value!({ "equal": [{ "referenced": ["pollId", "curatorId"] }, "label"] }),
            "a string property of the type",
        ),
        (
            platform_value!({ "equal": [{ "referenced": ["pollId", "endsAt"] }, { "const": "soon" }] }),
            "which spells none",
        ),
    ] {
        expect_structure_error(parse_rules(platform_value!({ "rule": rule })), needle);
    }
}

/// Two fields of referenced documents compared with each other compare by
/// value, whatever they hold, one left out equalling nothing; registration
/// refuses two that hold different kinds of value, which never agree.
#[test]
fn should_compare_two_referenced_fields_by_value() {
    let types = parse_rules(platform_value!({
        "sameStatus": { "equal": [{ "referenced": ["pollId", "status"] }, { "referenced": ["postId", "hashtag"] }] }
    }))
    .expect("two strings compare");
    let vote = types.get("vote").expect("the vote type").as_ref();
    let system = DocumentSystemValues::owned_by(Identifier::new([3; 32]));
    let data: BTreeMap<String, Value> = platform_value!({ "pollId": Value::Identifier([5; 32]), "postId": Value::Identifier([6; 32]), "choice": 0u8 })
        .into_btree_string_map()
        .expect("a map");
    let judge_with = |status: Option<&str>, hashtag: Option<&str>| {
        let poll = status.map_or_else(
            || platform_value!({}),
            |status| platform_value!({ "status": status }),
        );
        let post = hashtag.map_or_else(
            || platform_value!({}),
            |hashtag| platform_value!({ "hashtag": hashtag }),
        );
        vote.validate_referencing_property_constraints(
            &data,
            BTreeMap::from([("pollId".to_string(), poll), ("postId".to_string(), post)]),
            system.clone(),
            PlatformVersion::latest(),
        )
        .expect("the rules evaluate")
        .is_valid()
    };
    assert!(judge_with(Some("open"), Some("open")));
    assert!(!judge_with(Some("open"), Some("closed")));
    assert!(!judge_with(None, None));

    expect_structure_error(
        parse_rules(platform_value!({
            "rule": { "equal": [{ "referenced": ["pollId", "endsAt"] }, { "referenced": ["postId", "hashtag"] }] }
        })),
        "which hold different kinds of value",
    );
    expect_structure_error(
        parse_rules(platform_value!({
            "rule": { "notEqual": [{ "referenced": ["pollId", "endsAt"] }, { "referenced": ["pollId", "status"] }] }
        })),
        "which does not",
    );
}

/// A reference inside an object is read through by its dotted path.
#[test]
fn should_read_through_a_reference_inside_an_object() {
    let mut nested = vote(
        poll_reference(),
        platform_value!({
            "beforeTheEnd": { "lessThan": ["$createdAt", { "referenced": ["meta.pollId", "endsAt"] }] }
        }),
        vec![],
    );
    nested
        .set_value_at_full_path(
            "properties.meta",
            platform_value!({
                "type": "object",
                "properties": { "pollId": identifier(0, Some(poll_reference())) },
                "required": ["pollId"],
                "additionalProperties": false,
                "position": 6,
            }),
        )
        .expect("the vote takes the object");
    let types = parse(nested).expect("a nested reference reads");
    let vote = types.get("vote").expect("the vote type").as_ref();
    let rule = vote
        .property_constraints()
        .get("beforeTheEnd")
        .expect("the rule");
    assert_eq!(
        rule.referenced_reads(),
        vec![("meta.pollId", "endsAt", PropertyRead::Value)]
    );
    let system = DocumentSystemValues {
        created_at: Some(1_000),
        ..DocumentSystemValues::owned_by(Identifier::new([3; 32]))
    };
    let data: BTreeMap<String, Value> = platform_value!({ "pollId": Value::Identifier([5; 32]), "pollOwnerId": Value::Identifier([4; 32]), "choice": 0u8, "meta": { "pollId": Value::Identifier([5; 32]) } })
        .into_btree_string_map()
        .expect("a map");
    for (ends_at, holds) in [(2_000u64, true), (500, false)] {
        let result = vote
            .validate_referencing_property_constraints(
                &data,
                BTreeMap::from([(
                    "meta.pollId".to_string(),
                    platform_value!({ "endsAt": ends_at }),
                )]),
                system.clone(),
                PlatformVersion::latest(),
            )
            .expect("the rules evaluate");
        assert_eq!(
            result.is_valid(),
            holds,
            "endsAt {ends_at}: {:?}",
            result.errors
        );
    }
}

#[test]
fn should_read_only_what_a_removal_record_keeps_through_a_moderated_reference() {
    // The record keeps $ownerId and the hashtag
    parse_rules(platform_value!({
        "rule": { "notEqual": [{ "referenced": ["postId", "hashtag"] }, { "const": "spam" }] }
    }))
    .expect("a kept field reads");
    parse_rules(platform_value!({
        "rule": { "notEqual": [{ "referenced": ["postId", "$ownerId"] }, "$ownerId"] }
    }))
    .expect("the owner reads");
    expect_structure_error(
        parse_rules(platform_value!({
            "rule": { "lessThan": [{ "referenced": ["postId", "score"] }, 50] }
        })),
        "deleteKeepsFields",
    );
    expect_structure_error(
        parse_rules(platform_value!({
            "rule": { "lessThan": [{ "referenced": ["postId", "$createdAt"] }, "$createdAt"] }
        })),
        "deleteKeepsFields",
    );
}

#[test]
fn should_refuse_a_referenced_read_outside_property_constraints() {
    let delete_rule = vote(
        poll_reference(),
        platform_value!({ "rule": { "greaterThan": ["choice", -1] } }),
        vec![(
            "deleteConstraints",
            platform_value!({ "rule": { "lessThan": [{ "referenced": ["pollId", "endsAt"] }, 5] } }),
        )],
    );
    expect_structure_error(parse(delete_rule), "only a propertyConstraints rule");

    let immutable = vote(
        poll_reference(),
        platform_value!({ "rule": { "greaterThan": ["choice", -1] } }),
        vec![(
            "immutable",
            platform_value!([{
                "property": "choice",
                "when": { "lessThan": [{ "referenced": ["pollId", "endsAt"] }, "$updatedAt"] }
            }]),
        )],
    );
    expect_structure_error(parse(immutable), "only a propertyConstraints rule");
}

#[test]
fn should_refuse_a_referenced_read_beside_what_a_transfer_changes() {
    let transferable = vote(
        poll_reference(),
        platform_value!({
            "rule": {
                "allOf": [
                    { "lessThan": ["$createdAt", { "referenced": ["pollId", "endsAt"] }] },
                    { "notEqual": [{ "referenced": ["pollId", "$ownerId"] }, "$ownerId"] }
                ]
            }
        }),
        vec![("transferable", Value::U8(1))],
    );
    expect_structure_error(
        parse(transferable),
        "a transfer or a purchase checks no reference",
    );

    // Reading neither the owner nor a transfer time, the rule stands on a transferable type
    let transferable = vote(
        poll_reference(),
        platform_value!({
            "rule": { "lessThan": ["$createdAt", { "referenced": ["pollId", "endsAt"] }] }
        }),
        vec![("transferable", Value::U8(1))],
    );
    parse(transferable).expect("the rule reads nothing a transfer changes");
}

/// An entry on a poll whose moderators approve it (`status`), the poll's
/// entries counted in all and by approval, with `rules`.
fn moderated_entry(rules: Value) -> Value {
    platform_value!({
        "type": "object",
        "documentsMutable": true,
        "canBeDeleted": true,
        "moderatorAbilities": { "changeFields": ["status"] },
        "properties": {
            "pollId": identifier(0, Some(poll_reference())),
            "status": { "type": "string", "enum": ["approved"], "maxLength": 10, "position": 1 },
        },
        "required": ["pollId", "$createdAt"],
        "indices": [
            {
                "name": "byPollStatus",
                "properties": [{ "pollId": "asc" }, { "status": "asc" }],
                "countable": "countable"
            },
            {
                "name": "byPoll",
                "properties": [{ "pollId": "asc" }],
                "countable": "countable"
            }
        ],
        "propertyConstraints": rules,
        "additionalProperties": false,
    })
}

fn parse_entry(rules: Value) -> Result<BTreeMap<String, DocumentType>, ProtocolError> {
    parse_types(
        vec![("poll", poll()), ("entry", moderated_entry(rules))],
        true,
    )
}

/// A moderator's change judges no rule reading a referenced document, so such
/// a rule may not read what their change is judged against: a field moderators
/// change, read or keying a total of the type's own documents (approving an
/// entry moves the count of the poll's approved entries, though the rule does
/// not read `status` itself), or any total, which may have moved since the
/// document was last written.
#[test]
fn should_refuse_a_referenced_read_beside_a_total_moderators_move() {
    let approved_cap = platform_value!({
        "cap": {
            "lessThanOrEqual": [
                { "countOf": ["entry", { "pollId": "pollId", "status": { "const": "approved" } }] },
                { "referenced": ["pollId", "endsAt"] }
            ]
        }
    });
    let reads_status = platform_value!({
        "cap": {
            "anyOf": [
                { "absent": "status" },
                { "lessThan": ["$createdAt", { "referenced": ["pollId", "endsAt"] }] }
            ]
        }
    });
    for rules in [approved_cap, reads_status] {
        expect_structure_error(parse_entry(rules), "depends on \"status\"");
    }

    // The count of all the poll's entries, which no approval moves, may have
    // moved by other writes since the entry was last written, and a moderator's
    // change is judged against every rule reading a total
    expect_structure_error(
        parse_entry(platform_value!({
            "cap": {
                "lessThanOrEqual": [
                    { "countOf": ["entry", { "pollId": "pollId" }] },
                    { "referenced": ["pollId", "endsAt"] }
                ]
            }
        })),
        "and a total, which a moderator's change",
    );

    // Reading neither a moderator's field, `$revision` nor a total, a rule stands
    parse_entry(platform_value!({
        "cap": { "lessThan": ["$createdAt", { "referenced": ["pollId", "endsAt"] }] }
    }))
    .expect("a moderator's change can break nothing the rule reads");
}

/// A moderator's change writes the fields it changes and the revision, so a
/// rule reading a referenced document reads neither the stored version of such
/// a field, through `$old.`, nor `$revision`, of either version. The stored
/// version of what the change leaves alone stands.
#[test]
fn should_refuse_a_referenced_read_beside_a_stored_field_or_revision_moderators_move() {
    let stored_status = platform_value!({
        "cap": {
            "anyOf": [
                { "absent": "$old.status" },
                { "lessThan": ["$createdAt", { "referenced": ["pollId", "endsAt"] }] }
            ]
        }
    });
    expect_structure_error(parse_entry(stored_status), "depends on \"status\"");
    for revision in ["$revision", "$old.$revision"] {
        let rules = platform_value!({
            "cap": {
                "anyOf": [
                    { "lessThanOrEqual": [revision, 3] },
                    { "lessThan": ["$createdAt", { "referenced": ["pollId", "endsAt"] }] }
                ]
            }
        });
        expect_structure_error(
            parse_entry(rules),
            "and $revision, which a moderator's change",
        );
    }

    parse_entry(platform_value!({
        "cap": {
            "lessThan": ["$old.$createdAt", { "referenced": ["pollId", "endsAt"] }]
        }
    }))
    .expect("a moderator's change leaves the stored creation as it was");
}

/// A vote on a poll, the poll's votes counted.
fn counted_vote() -> Value {
    platform_value!({
        "type": "object",
        "documentsMutable": false,
        "canBeDeleted": false,
        "properties": {
            "pollId": identifier(0, Some(poll_reference())),
        },
        "required": ["pollId"],
        "indices": [
            {
                "name": "byPoll",
                "properties": [{ "pollId": "asc" }],
                "countable": "countable"
            }
        ],
        "additionalProperties": false,
    })
}

/// An entry on a poll moderators take down, its removal keeping a record a
/// restore brings it back from when `keeps_record`, the poll's entries
/// counted, with `rules`.
fn removable_entry(keeps_record: bool, rules: Value) -> Value {
    platform_value!({
        "type": "object",
        "documentsMutable": true,
        "canBeDeleted": true,
        "moderatorAbilities": { "delete": true, "deleteKeepsRecord": keeps_record },
        "properties": {
            "pollId": identifier(0, Some(poll_reference())),
        },
        "required": ["pollId", "$createdAt"],
        "indices": [
            {
                "name": "byPoll",
                "properties": [{ "pollId": "asc" }],
                "countable": "countable"
            }
        ],
        "propertyConstraints": rules,
        "additionalProperties": false,
    })
}

/// A moderator's restore judges no rule reading a referenced document, yet is
/// judged against every rule reading a total, which may have moved while the
/// document was gone: a rule capping the poll's entries by a value of the poll
/// would let a restore past the cap, and so would one capping the votes the
/// entry received meanwhile.
#[test]
fn should_refuse_a_referenced_read_beside_a_total_a_restore_moves() {
    let cap = platform_value!({
        "cap": {
            "lessThanOrEqual": [
                { "countOf": ["entry", { "pollId": "pollId" }] },
                { "referenced": ["pollId", "endsAt"] }
            ]
        }
    });
    expect_structure_error(
        parse_types(
            vec![
                ("poll", poll()),
                ("entry", removable_entry(true, cap.clone())),
            ],
            true,
        ),
        "and a total, which a moderator's restore is judged against",
    );
    // A total over another type: the poll's votes, cast while the entry was gone
    let votes_cap = platform_value!({
        "cap": {
            "lessThanOrEqual": [
                { "countOf": ["vote", { "pollId": "pollId" }] },
                { "referenced": ["pollId", "endsAt"] }
            ]
        }
    });
    expect_structure_error(
        parse_types(
            vec![
                ("poll", poll()),
                ("vote", counted_vote()),
                ("entry", removable_entry(true, votes_cap)),
            ],
            true,
        ),
        "and a total, which a moderator's restore is judged against",
    );

    // Deletions keeping no record are final, so no restore moves the count
    parse_types(
        vec![("poll", poll()), ("entry", removable_entry(false, cap))],
        true,
    )
    .expect("no restore moves a total the rule reads");
}

#[test]
fn should_refuse_a_referenced_read_through_a_find_by_function() {
    let commitment = platform_value!({
        "type": "object",
        "documentsMutable": false,
        "canBeDeleted": true,
        "properties": {
            "hash": { "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32, "position": 0 },
            "weight": { "type": "integer", "minimum": 0, "maximum": 9, "position": 1 },
        },
        "indices": [{ "name": "byHash", "properties": [{ "hash": "asc" }], "unique": true }],
        "required": ["hash", "weight", "$createdAtBlockHeight"],
        "additionalProperties": false,
    });
    let reveal = platform_value!({
        "type": "object",
        "documentsMutable": false,
        "canBeDeleted": true,
        "properties": {
            "secret": {
                "type": "string",
                "minLength": 1,
                "maxLength": 63,
                "position": 0,
                "refersTo": {
                    "type": "deletableDocument",
                    "documentType": "commitment",
                    "findBy": {
                        "hash": {
                            "function": "sys.hash.sha256d",
                            "params": ["secret"]
                        }
                    }
                }
            },
            "weight": { "type": "integer", "minimum": 0, "maximum": 9, "position": 1 },
        },
        "required": ["secret", "weight"],
        "propertyConstraints": {
            "rule": { "equal": ["weight", { "referenced": ["secret", "weight"] }] }
        },
        "additionalProperties": false,
    });
    expect_structure_error(
        parse_types(vec![("commitment", commitment), ("reveal", reveal)], true),
        "computes its key with a function",
    );
}

#[test]
fn should_refuse_a_malformed_referenced_operand() {
    for (rule, needle) in [
        (
            platform_value!({ "lessThan": [{ "referenced": ["pollId"] }, 5] }),
            "must list a reference property",
        ),
        (
            platform_value!({ "lessThan": [{ "referenced": ["meta..pollId", "endsAt"] }, 5] }),
            "a reference property is the dotted path",
        ),
        (
            platform_value!({ "lessThan": [{ "referenced": ["pollId", "$revision"] }, 5] }),
            "a field of the referenced document",
        ),
        (
            platform_value!({ "lessThan": ["$referenced.pollId.$.endsAt", 5] }),
            "is written { \"referenced\"",
        ),
    ] {
        // Without full validation the meta-schema does not run, so the parser
        // refuses them itself
        let result = parse_types(
            vec![
                ("poll", poll()),
                ("post", moderated_post()),
                (
                    "vote",
                    vote(poll_reference(), platform_value!({ "rule": rule }), vec![]),
                ),
            ],
            false,
        );
        expect_structure_error(result, needle);
    }
}

fn where_conditions(types: &BTreeMap<String, DocumentType>) -> BTreeMap<String, WhereCondition> {
    let vote = types.get("vote").expect("the vote type").as_ref();
    let Some(DocumentPropertyType::IdentifierWithReference(
        DocumentPropertyReferenceTarget::DeletableDocument {
            where_conditions, ..
        },
    )) = vote
        .flattened_properties()
        .get("pollId")
        .map(|property| &property.property_type)
    else {
        panic!("pollId refers to a deletable poll by id");
    };
    where_conditions.clone()
}

#[test]
fn should_parse_where_entries_that_are_not_equalities() {
    let types = parse(vote(
        platform_value!({
            "type": "deletableDocument",
            "documentType": "poll",
            "where": {
                "$ownerId": "pollOwnerId",
                "multiChoice": { "const": false },
                "status": { "in": ["open", "closed"] },
                "endsAt": { "greaterThan": "$createdAt" },
                "option0": { "notEqual": "label" },
                "$id": { "notEqual": "$ownerId" }
            }
        }),
        platform_value!({ "rule": { "greaterThan": ["choice", -1] } }),
        vec![],
    ))
    .expect("the where parses");
    let conditions = where_conditions(&types);
    assert_eq!(
        conditions.get("multiChoice"),
        Some(&WhereCondition::Constant(WhereLiteral::Boolean(false)))
    );
    assert_eq!(
        conditions.get("status"),
        Some(&WhereCondition::In(
            [
                WhereLiteral::Text("open".to_string()),
                WhereLiteral::Text("closed".to_string())
            ]
            .into_iter()
            .collect()
        ))
    );
    assert_eq!(
        conditions.get("endsAt"),
        Some(&WhereCondition::Ordering {
            ordering: WhereOrdering::GreaterThan,
            bound: WhereOrderingBound::System(SystemProperty::CreatedAt)
        })
    );
    assert_eq!(
        conditions.get("option0"),
        Some(&WhereCondition::NotEqual(WhereNotEqualBound::Property(
            "label".to_string()
        )))
    );
    assert_eq!(
        conditions.get("$id"),
        Some(&WhereCondition::NotEqual(WhereNotEqualBound::Writer))
    );
    // The equality stays an equality
    assert_eq!(conditions.len(), 5);
}

#[test]
fn should_refuse_a_malformed_where_entry() {
    for (entry, needle) in [
        (
            platform_value!({ "$ownerId": { "greaterThan": "choice" } }),
            "orders an identifier",
        ),
        (
            platform_value!({ "endsAt": { "greaterThan": "$ownerId" } }),
            "orders by $ownerId",
        ),
        (
            platform_value!({ "endsAt": { "lessThan": { "const": "soon" } } }),
            "must be an integer",
        ),
        (
            platform_value!({ "status": { "in": ["open", 1] } }),
            "of another kind",
        ),
        (
            platform_value!({ "status": { "in": ["open", "open"] } }),
            "repeats an earlier value",
        ),
        (
            platform_value!({ "status": { "in": ["open"] } }),
            "two or more values",
        ),
        (
            platform_value!({ "status": { "sha256": ["label"] } }),
            "belongs in findBy",
        ),
        (
            platform_value!({ "status": { "notEqual": "$createdAt" } }),
            "its $ownerId",
        ),
        (
            platform_value!({ "status": { "const": "open", "in": ["a", "b"] } }),
            "one key",
        ),
    ] {
        let result = parse_types(
            vec![
                ("poll", poll()),
                ("post", moderated_post()),
                (
                    "vote",
                    vote(
                        platform_value!({
                            "type": "deletableDocument",
                            "documentType": "poll",
                            "where": entry
                        }),
                        platform_value!({ "rule": { "greaterThan": ["choice", -1] } }),
                        vec![],
                    ),
                ),
            ],
            false,
        );
        expect_structure_error(result, needle);
    }
}

#[test]
fn should_hold_a_moderated_writer_inequality_to_what_every_record_keeps() {
    let mut refused = vote(
        poll_reference(),
        platform_value!({ "rule": { "greaterThan": ["choice", -1] } }),
        vec![],
    );
    refused
        .get_mut("properties")
        .expect("properties accessible")
        .expect("properties present")
        .set_value(
            "postId",
            identifier(
                4,
                Some(platform_value!({
                    "type": "moderatedDocument",
                    "documentType": "post",
                    "where": { "hashtag": { "notEqual": "$ownerId" } }
                })),
            ),
        )
        .expect("the property applies");
    expect_structure_error(parse(refused), "compares the writer");
}

/// A `where` literal no value of the referenced property can equal, or an
/// ordering no value of it meets, would refuse every write or hold for every
/// one, so registration refuses it, the referenced type in hand.
#[test]
fn should_refuse_a_where_literal_the_referenced_property_never_holds() {
    let types = parse_rules(pollr_rules()).expect("the types parse");
    let poll = types.get("poll").expect("the poll type").as_ref();
    let text = |text: &str| WhereLiteral::Text(text.to_string());
    let error = |property: &str, kind: WhereLiteralKind, condition: WhereCondition| {
        where_condition_literal_error(poll, None, property, kind, &condition)
            .expect("the schema reads")
    };
    for (property, kind, condition, reason) in [
        (
            "status",
            WhereLiteralKind::Text,
            WhereCondition::Constant(text("paused")),
            "enum values",
        ),
        (
            "option0",
            WhereLiteralKind::Text,
            WhereCondition::NotEqual(WhereNotEqualBound::Constant(text(&"x".repeat(101)))),
            "of a length",
        ),
        (
            "payload",
            WhereLiteralKind::Bytes,
            WhereCondition::In(vec![text("cafe"), text("CAFE")]),
            "an earlier literal",
        ),
        (
            "endsAt",
            WhereLiteralKind::Integer,
            WhereCondition::Constant(WhereLiteral::Integer(-1)),
            "outside the values",
        ),
        (
            "endsAt",
            WhereLiteralKind::Integer,
            WhereCondition::Ordering {
                ordering: WhereOrdering::GreaterThan,
                bound: WhereOrderingBound::Constant(9007199254740991),
            },
            "no value the referenced property holds",
        ),
    ] {
        let refused = error(property, kind, condition);
        assert!(
            refused
                .as_deref()
                .is_some_and(|refused| refused.contains(reason)),
            "{property}: {refused:?}"
        );
    }
    for (property, kind, condition) in [
        (
            "status",
            WhereLiteralKind::Text,
            WhereCondition::In(vec![text("open"), text("closed")]),
        ),
        (
            "payload",
            WhereLiteralKind::Bytes,
            WhereCondition::In(vec![text("cafe"), text("beef")]),
        ),
        (
            "endsAt",
            WhereLiteralKind::Integer,
            WhereCondition::Ordering {
                ordering: WhereOrdering::GreaterThanOrEqual,
                bound: WhereOrderingBound::Constant(9007199254740991),
            },
        ),
    ] {
        assert_eq!(error(property, kind, condition), None, "{property}");
    }
}
