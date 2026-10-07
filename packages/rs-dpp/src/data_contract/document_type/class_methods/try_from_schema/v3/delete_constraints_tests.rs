//! `deleteConstraints` (protocol version 14): named rules, in the grammar of
//! `propertyConstraints`, the stored document must meet for its owner to delete it,
//! and `$id`, the document's own id, as the value a `countOf` or `sumOf` filter
//! matches by. What the parser records, what it refuses, and the limits.

use super::immutable_tests::expect_structure_error;
use super::*;
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
use crate::data_contract::document_type::accessors::DocumentTypeV2Getters;
use crate::data_contract::document_type::property_constraints::{
    AggregateBinding, AggregateKind, AggregateRead, PropertyRead,
};
use crate::data_contract::DataContract;
use platform_value::string_encoding::Encoding;
use serde_json::json;

fn identifier(position: u32) -> serde_json::Value {
    json!({
        "type": "array",
        "byteArray": true,
        "minItems": 32,
        "maxItems": 32,
        "contentMediaType": "application/x.dash.dpp.identifier",
        "position": position
    })
}

/// A poll its owner may delete, recording `$createdAt`, with `poll_extra` merged in
/// (its `deleteConstraints`, or a `canBeDeleted` to test).
fn poll_schema(poll_extra: serde_json::Value) -> serde_json::Value {
    let mut poll = json!({
        "type": "object",
        "documentsMutable": true,
        "canBeDeleted": true,
        "properties": {
            "question": { "type": "string", "maxLength": 280, "position": 0 },
            "endsAt": { "type": "integer", "minimum": 0, "position": 1 },
            "status": {
                "type": "string",
                "enum": ["draft", "open"],
                "maxLength": 10,
                "position": 2
            },
            "optionCount": { "type": "integer", "minimum": 2, "maximum": 10, "position": 3 },
            "note": { "type": "string", "maxLength": 63, "position": 4 }
        },
        "required": ["$createdAt", "question", "endsAt", "optionCount"],
        "transient": ["note"],
        "additionalProperties": false
    });
    if let serde_json::Value::Object(entries) = poll_extra {
        for (key, value) in entries {
            poll[key] = value;
        }
    }
    poll
}

/// A contract of a `poll` (`poll_extra` merged in) and a `vote` pointing at one by
/// `pollId`, whose `byPoll` index counts the votes of each poll and totals their
/// `weight`, and whose `byPollChoice` counts them by poll and choice.
fn poll_contract(
    poll_extra: serde_json::Value,
    full_validation: bool,
) -> Result<DataContract, ProtocolError> {
    let contract = json!({
        "$formatVersion": "1",
        "id": Identifier::from([7; 32]).to_string(Encoding::Base58),
        "ownerId": Identifier::from([8; 32]).to_string(Encoding::Base58),
        "version": 1,
        "documentSchemas": {
            "poll": poll_schema(poll_extra),
            "vote": {
                "type": "object",
                "documentsMutable": true,
                "canBeDeleted": false,
                "properties": {
                    "pollId": identifier(0),
                    "choice": { "type": "integer", "minimum": 0, "maximum": 9, "position": 1 },
                    "weight": { "type": "integer", "minimum": 0, "maximum": 100, "position": 2 }
                },
                "required": ["pollId", "choice", "weight"],
                "indices": [
                    {
                        "name": "byPoll",
                        "properties": [{ "pollId": "asc" }],
                        "countable": "countable",
                        "summable": "weight"
                    },
                    {
                        "name": "byPollChoice",
                        "properties": [{ "pollId": "asc" }, { "choice": "asc" }],
                        "countable": "countable"
                    }
                ],
                "additionalProperties": false
            }
        }
    });
    DataContract::from_value(
        platform_value::to_value(contract).expect("the contract converts"),
        full_validation,
        PlatformVersion::latest(),
    )
}

/// A contract whose poll declares the delete rule `rule`.
fn poll_delete_rule(rule: serde_json::Value) -> Result<DataContract, ProtocolError> {
    poll_contract(json!({ "deleteConstraints": { "rule": rule } }), true)
}

/// `{ "countOf": ["vote", { "pollId": "$id" }] }`: the votes pointing at the poll.
fn votes_of_this_poll() -> serde_json::Value {
    json!({ "countOf": ["vote", { "pollId": "$id" }] })
}

fn votes_of_this_poll_read() -> AggregateRead {
    AggregateRead {
        kind: AggregateKind::Count,
        document_type: "vote".to_string(),
        filter: [("pollId".to_string(), AggregateBinding::Id)].into(),
        of_own_type: false,
    }
}

#[test]
fn should_parse_delete_rules_on_both_paths() {
    for full_validation in [true, false] {
        let contract = poll_contract(
            json!({
                "deleteConstraints": {
                    "noVotes": { "equal": [votes_of_this_poll(), 0] },
                    "stillDraft": { "equal": ["status", { "const": "draft" }] },
                    "beforeItEnds": { "lessThan": ["$createdAt", "endsAt"] }
                }
            }),
            full_validation,
        )
        .expect("delete rules over the stored poll and the votes pointing at it parse");
        let poll = contract.document_type_for_name("poll").expect("poll");
        let rules = poll.delete_constraints();
        // In name order, the order they are judged in
        assert_eq!(
            rules.keys().map(String::as_str).collect::<Vec<_>>(),
            ["beforeItEnds", "noVotes", "stillDraft"]
        );
        assert_eq!(
            rules["noVotes"].aggregate_reads(),
            [&votes_of_this_poll_read()]
        );
        // The id is no property of the poll: the rule reads none
        assert!(rules["noVotes"].property_reads().is_empty());
        assert!(!rules["noVotes"].reads_owner());
        assert_eq!(
            rules["stillDraft"].property_reads(),
            [("status", PropertyRead::Text)]
        );
        // The rules gate deletes only: creates and replaces are not held to them
        assert!(poll.property_constraints().is_empty());
        assert!(poll.documents_can_be_deleted());
    }
}

/// `$id` is a value a filter matches by in `propertyConstraints` too: a poll whose
/// replace is refused once a vote points at it.
#[test]
fn should_match_a_property_constraints_total_by_the_document_id() {
    let contract = poll_contract(
        json!({
            "propertyConstraints": {
                "editableBeforeTheFirstVote": { "equal": [votes_of_this_poll(), 0] }
            }
        }),
        true,
    )
    .expect("a rule matching a total by $id parses");
    let poll = contract.document_type_for_name("poll").expect("poll");
    assert_eq!(
        poll.property_constraints()["editableBeforeTheFirstVote"].aggregate_reads(),
        [&votes_of_this_poll_read()]
    );
    assert!(poll.delete_constraints().is_empty());
}

/// A type whose owner never deletes a document has no delete to gate, and an indexOnly
/// type's delete names no stored document: refused on every parse.
#[test]
fn should_refuse_delete_rules_where_the_owner_deletes_no_stored_document() {
    let rules = json!({ "noVotes": { "equal": [votes_of_this_poll(), 0] } });
    for full_validation in [true, false] {
        expect_structure_error(
            poll_contract(
                json!({ "canBeDeleted": false, "deleteConstraints": rules.clone() }),
                full_validation,
            ),
            "document type \"poll\" deleteConstraints gate the delete of a document by its \
             owner, but the type sets `canBeDeleted: false`, so there is no delete to gate",
        );
        expect_structure_error(
            poll_contract(
                json!({
                    "canBeDeleted": "onlyWhenConsumed",
                    "deleteConstraints": rules.clone()
                }),
                full_validation,
            ),
            "the type sets `canBeDeleted: \"onlyWhenConsumed\"`: its owner never deletes one",
        );
    }

    let index_only = json!({
        "$formatVersion": "1",
        "id": Identifier::from([7; 32]).to_string(Encoding::Base58),
        "ownerId": Identifier::from([8; 32]).to_string(Encoding::Base58),
        "version": 1,
        "documentSchemas": {
            "like": {
                "type": "object",
                "indexOnly": true,
                "documentsMutable": false,
                "properties": {
                    "postId": identifier(0),
                    "weight": { "type": "integer", "minimum": 0, "maximum": 9, "position": 1 }
                },
                "required": ["postId", "weight"],
                "indices": [{
                    "name": "byPost",
                    "properties": [{ "postId": "asc" }, { "weight": "asc" }],
                    "terminal": "$ownerId"
                }],
                "deleteConstraints": { "light": { "lessThan": ["weight", 5] } },
                "additionalProperties": false
            }
        }
    });
    expect_structure_error(
        DataContract::from_value(
            platform_value::to_value(index_only).expect("the contract converts"),
            true,
            PlatformVersion::latest(),
        ),
        "document type \"like\" deleteConstraints gate the delete of a stored document, but \
         the type is indexOnly",
    );
}

/// A delete rule reads what a `propertyConstraints` rule may read: a property of the
/// right kind that is stored, and a time or height the type records. Nor may it read
/// the stored document through `$old.`, which a replace's conditions alone have.
#[test]
fn should_refuse_delete_rules_reading_what_no_rule_may() {
    expect_structure_error(
        poll_delete_rule(json!({ "equal": [{ "length": "note" }, 0] })),
        "document type \"poll\" deleteConstraints rule \"rule\" measures \"note\", which is \
         transient or inside a transient object",
    );
    expect_structure_error(
        poll_delete_rule(json!({ "lessThan": ["$updatedAt", "endsAt"] })),
        "document type \"poll\" deleteConstraints rule \"rule\" reads $updatedAt, which the \
         document type does not record: list it in required",
    );
    expect_structure_error(
        poll_delete_rule(json!({ "lessThan": ["question", 3] })),
        "document type \"poll\" deleteConstraints rule \"rule\" reads \"question\", which has \
         type string, not integer or boolean",
    );
    // The meta-schema refuses the path first; a stored contract meets the parser
    poll_delete_rule(json!({ "present": "$old.status" }))
        .expect_err("the meta-schema refuses a path through $old.");
    expect_structure_error(
        poll_contract(
            json!({ "deleteConstraints": { "rule": { "present": "$old.status" } } }),
            false,
        ),
        "only a condition judging a replace",
    );
}

/// A total a delete rule reads is checked with the contract's other types, as a
/// `propertyConstraints` rule's is, `$id` matching only an identifier key.
#[test]
fn should_refuse_a_delete_total_no_tree_keeps() {
    // `byPollChoice` keeps the count by poll and choice, not by choice alone
    expect_structure_error(
        poll_delete_rule(json!({
            "equal": [{ "countOf": ["vote", { "choice": "optionCount" }] }, 0]
        })),
        "document type \"poll\" deleteConstraints rule \"rule\" counts \"vote\" by \"choice\", \
         which no countable index of \"vote\" whose properties are exactly those keys answers",
    );
    expect_structure_error(
        poll_delete_rule(json!({
            "equal": [{ "countOf": ["vote", { "choice": "$id" }] }, 0]
        })),
        "document type \"poll\" deleteConstraints rule \"rule\" counts \"vote\" with \"choice\", \
         an integer, at $id, an identifier",
    );
    expect_structure_error(
        poll_delete_rule(json!({ "equal": [{ "countOf": ["ballot", { "pollId": "$id" }] }, 0] })),
        "document type \"poll\" deleteConstraints rule \"rule\" counts \"ballot\", which is no \
         document type of this contract",
    );
    // `$id` is a value a key is matched against, never a key itself
    poll_delete_rule(json!({
        "equal": [{ "countOf": ["vote", { "$id": "$id" }] }, 0]
    }))
    .expect_err("$id is no key a filter matches by");

    poll_delete_rule(json!({
        "lessThanOrEqual": [{ "sumOf": ["vote", "weight", { "pollId": "$id" }] }, 10]
    }))
    .expect("a sum of the votes pointing at the poll is kept by byPoll");
}

/// The two keywords are each held to the limits of `SystemLimits` on their own: a type
/// may declare as many delete rules as rules for its writes.
#[test]
fn should_hold_delete_rules_to_the_rule_limits_apart_from_property_constraints() {
    let limits = &PlatformVersion::latest().system_limits;
    let rules = |count: usize| {
        serde_json::Value::Object(
            (0..count)
                .map(|index| {
                    (
                        format!("rule{index:02}"),
                        json!({ "lessThan": ["optionCount", index + 2] }),
                    )
                })
                .collect(),
        )
    };
    let max = usize::from(limits.max_property_constraints);
    poll_contract(
        json!({ "propertyConstraints": rules(max), "deleteConstraints": rules(max) }),
        true,
    )
    .expect("the most rules of each keyword parse together");
    expect_structure_error(
        poll_contract(json!({ "deleteConstraints": rules(max + 1) }), true),
        &format!(
            "document type \"poll\" deleteConstraints declares {} rules, above the maximum of \
             {max}",
            max + 1
        ),
    );
    // A stored contract is not held to the limits again
    poll_contract(json!({ "deleteConstraints": rules(max + 1) }), false)
        .expect("a stored contract over the limit still parses");
}
