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
use crate::data_contract::document_type::methods::DocumentTypeV0Methods;
use crate::data_contract::document_type::property_constraints::SystemProperty;
use crate::data_contract::document_type::property_constraints::{
    data_with_referenced_documents, referenced_fields, DocumentSystemValues,
    REFERENCED_DOCUMENT_KEY,
};
use crate::data_contract::document_type::{
    DocumentPropertyReferenceTarget, DocumentPropertyType, DocumentType, WhereBound,
    WhereCondition, WhereLiteral, WhereOrdering,
};
use crate::ProtocolError;
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
    let data = judged(vote_data.clone(), open_poll.clone());
    let result = vote
        .validate_referencing_property_constraints(&data, &system, PlatformVersion::latest())
        .expect("the rules evaluate");
    assert!(result.is_valid(), "{:?}", result.errors);

    // The poll has two options, so choice 2 is none of them
    let data = judged(
        platform_value!({ "pollId": poll_id.clone(), "pollOwnerId": poll_owner.clone(), "choice": 2u8 }),
        open_poll.clone(),
    );
    let result = vote
        .validate_referencing_property_constraints(&data, &system, PlatformVersion::latest())
        .expect("the rules evaluate");
    assert!(format!("{:?}", result.errors).contains("choiceIsAnOption"));

    // Past the end
    let mut ended = open_poll.clone();
    ended.set_value("endsAt", Value::U64(900)).expect("set");
    let result = vote
        .validate_referencing_property_constraints(
            &judged(vote_data.clone(), ended),
            &system,
            PlatformVersion::latest(),
        )
        .expect("the rules evaluate");
    assert!(format!("{:?}", result.errors).contains("castBeforeTheEnd"));

    // A multiple-choice poll is no single-choice vote's
    let mut multi = open_poll.clone();
    multi
        .set_value("multiChoice", Value::Bool(true))
        .expect("set");
    let result = vote
        .validate_referencing_property_constraints(
            &judged(vote_data.clone(), multi),
            &system,
            PlatformVersion::latest(),
        )
        .expect("the rules evaluate");
    assert!(format!("{:?}", result.errors).contains("singleChoicePoll"));

    // A poll without an end is open for good: the default stands in for endsAt
    let mut endless = open_poll;
    endless.remove("endsAt").expect("remove");
    let result = vote
        .validate_referencing_property_constraints(
            &judged(vote_data.clone(), endless),
            &system,
            PlatformVersion::latest(),
        )
        .expect("the rules evaluate");
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
            platform_value!({ "lessThan": [{ "referenced": ["meta.pollId", "endsAt"] }, 5] }),
            "a reference property is a top-level property",
        ),
        (
            platform_value!({ "lessThan": [{ "referenced": ["pollId", "$revision"] }, 5] }),
            "a field of the referenced document",
        ),
        (
            platform_value!({ "lessThan": ["$referenced.pollId.endsAt", 5] }),
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
            bound: WhereBound::System(SystemProperty::CreatedAt)
        })
    );
    assert_eq!(
        conditions.get("option0"),
        Some(&WhereCondition::NotEqual(WhereBound::Property(
            "label".to_string()
        )))
    );
    assert_eq!(
        conditions.get("$id"),
        Some(&WhereCondition::NotEqual(WhereBound::Writer))
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
