//! Reference expressions (`refersTo: { "anyOf": [...] }` / `{ "allOf": [...] }`,
//! nestable, protocol version 14): the parse of the declaration on an
//! identifier property and on the elements of a typed array, the shapes and
//! leaf types it refuses (by the parser on every parse, and by the meta-schema
//! at registration), the registration limits it counts against, the checks
//! each leaf gets as it would alone, the protocol version gate and the
//! platform serialization round trip.

use super::reference_test_helpers::{
    assert_refused, contract, contract_on, identifier, join_request_schema, CONTRACT_ID,
};
use super::typed_array_test_helpers::expect_json_schema_error;
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::{
    DocumentPropertyReferenceTarget, DocumentPropertyType, DocumentReferenceLookup,
    LookupKeySource, PropertyReference, ReferenceOperands,
};
use crate::data_contract::DataContract;
use crate::serialization::{
    PlatformDeserializableWithPotentialValidationFromVersionedStructureUntrusted,
    PlatformSerializableWithPlatformVersion,
};
use platform_value::string_encoding::Encoding;
use platform_value::Identifier;
use platform_version::version::PlatformVersion;
use serde_json::json;
use std::collections::BTreeMap;

/// The moderation charter's two ways in: a `joinRequest` of the member for
/// the charter, found by (`submittedCharterId`, `$ownerId`), or an
/// `addedModerator` document naming the member, found by
/// (`submittedCharterId`, `moderatorId`).
fn join_request_lookup() -> serde_json::Value {
    json!({
        "type": "permanentDocument",
        "documentType": "joinRequest",
        "lookup": {
            "index": "bySubmittedCharter",
            "keys": { "submittedCharterId": "submittedCharterId", "$ownerId": "." }
        }
    })
}

fn added_moderator_lookup() -> serde_json::Value {
    json!({
        "type": "permanentDocument",
        "documentType": "addedModerator",
        "lookup": {
            "index": "byModerator",
            "keys": { "submittedCharterId": "submittedCharterId", "moderatorId": "." }
        }
    })
}

fn any_of(operands: Vec<serde_json::Value>) -> serde_json::Value {
    json!({ "anyOf": operands })
}

fn all_of(operands: Vec<serde_json::Value>) -> serde_json::Value {
    json!({ "allOf": operands })
}

fn identity() -> serde_json::Value {
    json!({ "type": "identity" })
}

fn permanent(document_type: &str) -> serde_json::Value {
    json!({ "type": "permanentDocument", "documentType": document_type })
}

/// An expression `depth` combinators deep, alternating from `allOf` at the
/// innermost level, every list two operands: an `identity` and the level
/// below (a join request lookup at the bottom).
fn nested_to(depth: usize) -> serde_json::Value {
    let mut expression = join_request_lookup();
    for level in 0..depth {
        let operands = vec![identity(), expression];
        expression = if level % 2 == 0 {
            all_of(operands)
        } else {
            any_of(operands)
        };
    }
    expression
}

/// A contract with two permanent, immutable types a member can come from
/// (`joinRequest`, `addedModerator`), a deletable `note`, and a `resignation`
/// whose `memberId` declares `refers_to`, next to a required
/// `submittedCharterId`, a required string `title` and a key id `keyId`.
fn charter_contract(refers_to: serde_json::Value) -> serde_json::Value {
    let mut member_id = identifier(1);
    member_id["refersTo"] = refers_to;
    json!({
        "$formatVersion": "1",
        "id": Identifier::from(CONTRACT_ID).to_string(Encoding::Base58),
        "ownerId": Identifier::from([8; 32]).to_string(Encoding::Base58),
        "version": 1,
        "documentSchemas": {
            "joinRequest": join_request_schema(),
            "addedModerator": {
                "type": "object",
                "canBeDeleted": false,
                "documentsMutable": false,
                "properties": {
                    "submittedCharterId": identifier(0),
                    "moderatorId": identifier(1)
                },
                "indices": [
                    {
                        "name": "byModerator",
                        "properties": [{ "submittedCharterId": "asc" }, { "moderatorId": "asc" }],
                        "unique": true
                    }
                ],
                "required": ["submittedCharterId", "moderatorId"],
                "additionalProperties": false
            },
            "note": {
                "type": "object",
                "properties": {
                    "text": { "type": "string", "maxLength": 63, "position": 0 }
                },
                "additionalProperties": false
            },
            "resignation": {
                "type": "object",
                "properties": {
                    "submittedCharterId": identifier(0),
                    "memberId": member_id,
                    "title": { "type": "string", "maxLength": 63, "position": 2 },
                    "keyId": {
                        "type": "integer",
                        "minimum": 0,
                        "maximum": 4294967295u64,
                        "position": 3
                    },
                    "alternateCharterId": identifier(4)
                },
                "required": ["submittedCharterId", "title"],
                "additionalProperties": false
            }
        }
    })
}

/// [`charter_contract`] with a `members` typed array of at most `max_items`
/// identifiers on `resignation`, its items declaring `refers_to`, and a plain
/// `memberId`.
fn charter_contract_with_members(
    refers_to: serde_json::Value,
    max_items: u16,
) -> serde_json::Value {
    let mut contract = charter_contract(json!({ "type": "identity" }));
    let resignation = &mut contract["documentSchemas"]["resignation"];
    resignation["properties"]["memberId"] = identifier(1);
    let mut items = identifier(0);
    items.as_object_mut().expect("an object").remove("position");
    items["refersTo"] = refers_to;
    resignation["properties"]["members"] = json!({
        "type": "array",
        "maxItems": max_items,
        "items": items,
        "position": 5
    });
    contract
}

fn property_type(contract: &DataContract, property: &str) -> DocumentPropertyType {
    contract
        .document_type_for_name("resignation")
        .expect("the resignation document type")
        .flattened_properties()
        .get(property)
        .expect("the property")
        .property_type
        .clone()
}

/// Refused by the parser on every parse with `fragment`, and by the
/// meta-schema itself, before the parser runs, when the contract registers.
fn assert_refused_by_parser_and_meta_schema(schema: serde_json::Value, fragment: &str) {
    assert_refused(
        contract_on(schema.clone(), false, PlatformVersion::latest()),
        fragment,
    );
    expect_json_schema_error(contract(schema));
}

fn expected_join_request_lookup() -> DocumentPropertyReferenceTarget {
    DocumentPropertyReferenceTarget::PermanentDocumentLookup {
        contract_id: None,
        document_type_name: "joinRequest".to_string(),
        property_agreement: BTreeMap::new(),
        lookup: DocumentReferenceLookup {
            index: "bySubmittedCharter".to_string(),
            keys: [
                (
                    "submittedCharterId".to_string(),
                    LookupKeySource::Property("submittedCharterId".to_string()),
                ),
                ("$ownerId".to_string(), LookupKeySource::ReferenceValue),
            ]
            .into(),
        },
    }
}

fn expected_added_moderator_lookup() -> DocumentPropertyReferenceTarget {
    DocumentPropertyReferenceTarget::PermanentDocumentLookup {
        contract_id: None,
        document_type_name: "addedModerator".to_string(),
        property_agreement: BTreeMap::new(),
        lookup: DocumentReferenceLookup {
            index: "byModerator".to_string(),
            keys: [
                (
                    "submittedCharterId".to_string(),
                    LookupKeySource::Property("submittedCharterId".to_string()),
                ),
                ("moderatorId".to_string(), LookupKeySource::ReferenceValue),
            ]
            .into(),
        },
    }
}

fn any_of_targets(
    operands: Vec<DocumentPropertyReferenceTarget>,
) -> DocumentPropertyReferenceTarget {
    DocumentPropertyReferenceTarget::AnyOf(ReferenceOperands::new(operands))
}

fn all_of_targets(
    operands: Vec<DocumentPropertyReferenceTarget>,
) -> DocumentPropertyReferenceTarget {
    DocumentPropertyReferenceTarget::AllOf(ReferenceOperands::new(operands))
}

#[test]
fn should_parse_an_any_of_of_two_lookups_in_declared_order() {
    let parsed = contract(charter_contract(any_of(vec![
        join_request_lookup(),
        added_moderator_lookup(),
    ])))
    .expect("parses");

    assert_eq!(
        property_type(&parsed, "memberId"),
        DocumentPropertyType::IdentifierWithReference(any_of_targets(vec![
            expected_join_request_lookup(),
            expected_added_moderator_lookup(),
        ]))
    );

    // The order is the author's: it decides which error a writer is shown
    let reversed = contract(charter_contract(any_of(vec![
        added_moderator_lookup(),
        join_request_lookup(),
    ])))
    .expect("parses");
    assert_eq!(
        property_type(&reversed, "memberId"),
        DocumentPropertyType::IdentifierWithReference(any_of_targets(vec![
            expected_added_moderator_lookup(),
            expected_join_request_lookup(),
        ]))
    );
}

/// The same value must meet every operand of an `allOf`: an identity that
/// also asked to join.
#[test]
fn should_parse_an_all_of_of_a_lookup_and_an_identity() {
    let parsed = contract(charter_contract(all_of(vec![
        join_request_lookup(),
        identity(),
    ])))
    .expect("parses");

    assert_eq!(
        property_type(&parsed, "memberId"),
        DocumentPropertyType::IdentifierWithReference(all_of_targets(vec![
            expected_join_request_lookup(),
            DocumentPropertyReferenceTarget::Identity,
        ]))
    );
}

/// A `deletableDocument` found through a lookup is an operand like a permanent one: the
/// leader takes an added moderator off by deleting the addition, and the expression is then
/// re-validated on every replace. An immutable property may not hold it, since once the
/// document is gone the property could never change to pass again.
#[test]
fn should_parse_a_deletable_document_lookup_operand_and_refuse_it_in_an_immutable_property() {
    let mut deletable_added_moderator = added_moderator_lookup();
    deletable_added_moderator["type"] = json!("deletableDocument");
    let mut schema = charter_contract(any_of(vec![
        join_request_lookup(),
        deletable_added_moderator,
    ]));
    schema["documentSchemas"]["addedModerator"]["canBeDeleted"] = json!(true);

    let parsed = contract(schema.clone()).expect("parses");
    let DocumentPropertyReferenceTarget::PermanentDocumentLookup {
        contract_id,
        document_type_name,
        property_agreement,
        lookup,
    } = expected_added_moderator_lookup()
    else {
        unreachable!("a permanent lookup")
    };
    assert_eq!(
        property_type(&parsed, "memberId"),
        DocumentPropertyType::IdentifierWithReference(any_of_targets(vec![
            expected_join_request_lookup(),
            DocumentPropertyReferenceTarget::DeletableDocumentLookup {
                contract_id,
                document_type_name,
                property_agreement,
                lookup,
            },
        ]))
    );

    let mut immutable = schema;
    immutable["documentSchemas"]["resignation"]["immutable"] = json!(["memberId"]);
    assert_refused(
        contract(immutable),
        "lists \"memberId\" as immutable, but \"memberId\" is a deletableDocument reference \
         through a lookup",
    );
}

/// Each leaf is an ordinary declaration with its own keys: an agreement
/// belongs to the leaf it is declared on.
#[test]
fn should_parse_an_any_of_of_an_identity_and_a_document_with_its_own_agreement() {
    let parsed = contract(charter_contract(any_of(vec![
        identity(),
        json!({
            "type": "permanentDocument",
            "documentType": "joinRequest",
            "propertyAgreement": { "title": "message" }
        }),
    ])))
    .expect("parses");

    assert_eq!(
        property_type(&parsed, "memberId"),
        DocumentPropertyType::IdentifierWithReference(any_of_targets(vec![
            DocumentPropertyReferenceTarget::Identity,
            DocumentPropertyReferenceTarget::PermanentDocument {
                contract_id: None,
                document_type_name: "joinRequest".to_string(),
                property_agreement: [("title".to_string(), "message".to_string())].into(),
            },
        ]))
    );
}

/// The combinators alternate down to the depth limit, which registers; one
/// level more is refused under full validation (a stored contract was checked
/// when it was registered, so its parse does not re-apply the limit).
#[test]
fn should_parse_expressions_nested_to_the_depth_limit_and_refuse_one_deeper() {
    let platform_version = PlatformVersion::latest();
    let limit = usize::from(
        platform_version
            .system_limits
            .max_reference_expression_depth,
    );

    let deepest = contract(charter_contract(nested_to(limit))).expect("the limit registers");
    let DocumentPropertyType::IdentifierWithReference(expression) =
        property_type(&deepest, "memberId")
    else {
        panic!("expected a reference");
    };
    assert_eq!(expression.expression_depth(), limit);
    assert_eq!(expression.leaves().len(), limit + 1);

    assert_refused(
        contract(charter_contract(nested_to(limit + 1))),
        &format!(
            "property \"memberId\" of document type \"resignation\" declares a refersTo \
             expression nested {} deep, above the maximum of {limit}",
            limit + 1
        ),
    );
    let stored = contract_on(
        charter_contract(nested_to(limit + 1)),
        false,
        platform_version,
    )
    .expect("the stored path does not re-apply a registration limit");
    assert!(matches!(
        property_type(&stored, "memberId"),
        DocumentPropertyType::IdentifierWithReference(expression)
            if expression.expression_depth() == limit + 1
    ));
}

#[test]
fn should_parse_an_expression_on_the_elements_of_a_typed_array() {
    let parsed = contract(charter_contract_with_members(
        any_of(vec![
            added_moderator_lookup(),
            all_of(vec![join_request_lookup(), identity()]),
        ]),
        15,
    ))
    .expect("parses");

    let members = property_type(&parsed, "members");
    assert_eq!(
        members.reference(),
        Some(PropertyReference::Elements {
            target: &any_of_targets(vec![
                expected_added_moderator_lookup(),
                all_of_targets(vec![
                    expected_join_request_lookup(),
                    DocumentPropertyReferenceTarget::Identity,
                ]),
            ]),
            max_items: 15,
        })
    );
}

#[test]
fn should_refuse_a_list_of_fewer_than_two_operands() {
    for (schema, fragment) in [
        (
            charter_contract(any_of(vec![identity()])),
            "refersTo anyOf must list at least two operands",
        ),
        (
            charter_contract(all_of(vec![])),
            "refersTo allOf must list at least two operands",
        ),
        (
            charter_contract(any_of(vec![identity(), all_of(vec![identity()])])),
            "refersTo anyOf[1].allOf must list at least two operands",
        ),
        (
            charter_contract(json!({ "anyOf": { "type": "identity" } })),
            "refersTo anyOf must be a list of operands",
        ),
    ] {
        assert_refused_by_parser_and_meta_schema(schema, fragment);
    }
}

/// A pool of operands no two of which are alike.
fn distinct_operands() -> Vec<serde_json::Value> {
    vec![
        identity(),
        permanent("joinRequest"),
        permanent("addedModerator"),
        join_request_lookup(),
        added_moderator_lookup(),
        all_of(vec![identity(), join_request_lookup()]),
    ]
}

/// The operands one list holds are a registration limit,
/// `max_reference_operands`, at the top and inside a nested list alike.
#[test]
fn should_refuse_a_list_above_the_operand_limit_under_full_validation() {
    let platform_version = PlatformVersion::latest();
    let limit = usize::from(platform_version.system_limits.max_reference_operands);
    let pool = distinct_operands();
    assert!(pool.len() > limit, "the pool must outgrow the limit");

    contract(charter_contract(any_of(pool[..limit].to_vec()))).expect("the maximum registers");
    assert_refused(
        contract(charter_contract(any_of(pool[..=limit].to_vec()))),
        &format!(
            "property \"memberId\" of document type \"resignation\" declares a refersTo anyOf \
             of {} operands, above the maximum of {limit}",
            limit + 1
        ),
    );
    // Inside a nested list (its operands leaves, so no anyOf directly in it)
    let leaves: Vec<serde_json::Value> = pool[..pool.len() - 1].to_vec();
    assert!(leaves.len() > limit);
    assert_refused(
        contract(charter_contract(any_of(vec![
            identity(),
            all_of(leaves[..=limit].to_vec()),
        ]))),
        &format!("refersTo anyOf[1].allOf of {} operands", limit + 1),
    );
    let stored = contract_on(
        charter_contract(any_of(pool[..=limit].to_vec())),
        false,
        platform_version,
    )
    .expect("the stored path does not re-apply a registration limit");
    assert!(matches!(
        property_type(&stored, "memberId"),
        DocumentPropertyType::IdentifierWithReference(DocumentPropertyReferenceTarget::AnyOf(
            operands
        )) if operands.operands().len() == limit + 1
    ));
}

#[test]
fn should_refuse_every_leaf_type_an_expression_does_not_take() {
    for (refused, reason) in [
        (
            json!({ "type": "contract" }),
            "reference of type contract, which a reference expression does not take: its \
             requirements are gates judged against the block time and the writer",
        ),
        (
            json!({ "type": "token" }),
            "reference of type token, which a reference expression does not take",
        ),
        (
            json!({ "type": "deletableDocument", "documentType": "note" }),
            "reference of type deletableDocument, which a reference expression does not take: \
             by id it is re-validated on every replace",
        ),
        (
            json!({ "type": "identityPublicKey", "keyIdProperty": "keyId" }),
            "reference of type identityPublicKey, which a reference expression does not take: \
             it pairs the value with a key id property",
        ),
        (
            json!({ "type": "identityPublicKey", "identityProperty": "$ownerId" }),
            "reference of type identityPublicKey, which a reference expression does not take",
        ),
    ] {
        // As a direct operand, and as a leaf of a nested allOf
        assert_refused_by_parser_and_meta_schema(
            charter_contract(any_of(vec![identity(), refused.clone()])),
            &format!("refersTo anyOf[1] is a {reason}"),
        );
        assert_refused_by_parser_and_meta_schema(
            charter_contract(any_of(vec![
                identity(),
                all_of(vec![join_request_lookup(), refused]),
            ])),
            &format!("refersTo anyOf[1].allOf[1] is a {reason}"),
        );
    }

    // The key id form is refused whatever type it claims
    assert_refused_by_parser_and_meta_schema(
        charter_contract(all_of(vec![
            identity(),
            json!({ "type": "identity", "identityProperty": "$ownerId" }),
        ])),
        "refersTo allOf[1]: identity refersTo does not take identityProperty",
    );
}

/// An `anyOf` directly inside an `anyOf` (or an `allOf` inside an `allOf`)
/// says what one flat list says, so it is refused; the other combinator
/// nests.
#[test]
fn should_refuse_a_combinator_directly_inside_the_same_combinator() {
    assert_refused_by_parser_and_meta_schema(
        charter_contract(any_of(vec![
            identity(),
            any_of(vec![join_request_lookup(), added_moderator_lookup()]),
        ])),
        "refersTo anyOf[1] is an anyOf directly inside an anyOf",
    );
    assert_refused_by_parser_and_meta_schema(
        charter_contract(all_of(vec![
            identity(),
            any_of(vec![
                join_request_lookup(),
                all_of(vec![
                    identity(),
                    all_of(vec![identity(), join_request_lookup()]),
                ]),
            ]),
        ])),
        "refersTo allOf[1].anyOf[1].allOf[1] is an allOf directly inside an allOf",
    );
    contract(charter_contract(all_of(vec![
        identity(),
        any_of(vec![join_request_lookup(), added_moderator_lookup()]),
    ])))
    .expect("alternating combinators nest");
}

#[test]
fn should_refuse_keys_beside_a_combinator() {
    let mut beside = any_of(vec![identity(), join_request_lookup()]);
    beside["type"] = json!("identity");
    assert_refused_by_parser_and_meta_schema(
        charter_contract(beside),
        "refersTo anyOf declares nothing beside anyOf",
    );

    let mut both = any_of(vec![identity(), join_request_lookup()]);
    both["allOf"] = json!([identity(), added_moderator_lookup()]);
    assert_refused_by_parser_and_meta_schema(
        charter_contract(both),
        "refersTo anyOf declares nothing beside anyOf",
    );

    let mut agreement_beside = all_of(vec![identity(), join_request_lookup()]);
    agreement_beside["propertyAgreement"] = json!({ "title": "message" });
    assert_refused_by_parser_and_meta_schema(
        charter_contract(any_of(vec![added_moderator_lookup(), agreement_beside])),
        "refersTo anyOf[1].allOf declares nothing beside allOf",
    );
}

/// Two alike operands of one list are refused at registration, a leaf naming
/// the declaring contract explicitly included, which means the same as one
/// omitting it; the meta-schema catches only the identical spelling.
#[test]
fn should_refuse_an_operand_repeated_in_a_list() {
    let repeated = charter_contract(any_of(vec![
        join_request_lookup(),
        identity(),
        join_request_lookup(),
    ]));
    expect_json_schema_error(contract(repeated.clone()));

    let mut own_contract_named = join_request_lookup();
    own_contract_named["contractId"] =
        json!(Identifier::from(CONTRACT_ID).to_string(Encoding::Base58));
    let respelled = charter_contract(all_of(vec![
        identity(),
        join_request_lookup(),
        own_contract_named,
    ]));
    assert_refused(
        contract(respelled.clone()),
        "declares a refersTo allOf whose operand 2 repeats an earlier one",
    );

    // Registration limits: a stored contract was checked when it registered
    for schema in [repeated, respelled] {
        contract_on(schema, false, PlatformVersion::latest())
            .expect("the stored path does not re-apply a registration rule");
    }
}

#[test]
fn should_refuse_an_expression_on_a_property_that_is_not_an_identifier() {
    for (expression, combinator) in [
        (any_of(vec![identity(), join_request_lookup()]), "anyOf"),
        (all_of(vec![identity(), join_request_lookup()]), "allOf"),
    ] {
        let mut schema = charter_contract(identity());
        schema["documentSchemas"]["resignation"]["properties"]["keyId"]["refersTo"] = expression;
        assert_refused_by_parser_and_meta_schema(
            schema,
            &format!("refersTo {combinator} is only allowed on identifier properties"),
        );
    }
}

/// Every check a leaf gets alone, it gets inside an expression, and the error
/// names the leaf: a malformed leaf is refused by the parser, the referring
/// side of a lookup on every parse, and its referenced side against a type of
/// the same contract when the contract registers.
#[test]
fn should_check_each_leaf_as_it_would_be_checked_alone() {
    let malformed = charter_contract(any_of(vec![
        identity(),
        json!({ "type": "permanentDocument" }),
    ]));
    assert_refused(
        contract_on(malformed.clone(), false, PlatformVersion::latest()),
        "unable to get str property documentType",
    );
    expect_json_schema_error(contract(malformed));

    let mut optional_source = added_moderator_lookup();
    optional_source["lookup"]["keys"]["submittedCharterId"] = json!("alternateCharterId");
    assert_refused(
        contract_on(
            charter_contract(any_of(vec![join_request_lookup(), optional_source])),
            false,
            PlatformVersion::latest(),
        ),
        "document type \"resignation\" property \"memberId\" refersTo anyOf[1] lookup: key \
         \"submittedCharterId\" reads \"alternateCharterId\", which is not required",
    );

    let mut not_unique = join_request_lookup();
    not_unique["lookup"] = json!({ "index": "byMessage", "keys": { "message": "." } });
    let not_unique_nested = charter_contract(any_of(vec![
        added_moderator_lookup(),
        all_of(vec![identity(), not_unique]),
    ]));
    assert_refused(
        contract(not_unique_nested.clone()),
        "property \"memberId\" refersTo anyOf[1].allOf[1] lookup: index \"byMessage\" of \
         \"joinRequest\" is not unique",
    );
    // The referenced side needs the whole contract, so a stored parse leaves it
    contract_on(not_unique_nested, false, PlatformVersion::latest())
        .expect("a stored contract was checked when it registered");

    // A single declaration's error names no leaf, as before expressions
    let mut single_optional = join_request_lookup();
    single_optional["lookup"]["keys"]["submittedCharterId"] = json!("alternateCharterId");
    assert_refused(
        contract_on(
            charter_contract(single_optional),
            false,
            PlatformVersion::latest(),
        ),
        "property \"memberId\" refersTo lookup: key",
    );
}

/// Every leaf may be read for every value when a document is written, so each
/// counts against `max_references_per_document`.
#[test]
fn should_count_every_leaf_against_the_reference_limit() {
    let platform_version = PlatformVersion::latest();
    let limit = u32::from(platform_version.system_limits.max_references_per_document);
    let half = u16::try_from(limit / 2).expect("the limit fits a typed array's maxItems");

    contract(charter_contract_with_members(
        any_of(vec![join_request_lookup(), added_moderator_lookup()]),
        half,
    ))
    .expect("two leaves for half the limit's elements are exactly the maximum");

    // Three leaves, one of them inside a nested allOf
    assert_refused(
        contract(charter_contract_with_members(
            any_of(vec![
                added_moderator_lookup(),
                all_of(vec![identity(), join_request_lookup()]),
            ]),
            half,
        )),
        &format!(
            "declares references for up to {} values",
            3 * u32::from(half)
        ),
    );

    // A single property's expression counts its leaves too: the list's
    // references and two from memberId
    let list = u16::try_from(limit - 1).expect("fits");
    let mut schema = charter_contract_with_members(identity(), list);
    schema["documentSchemas"]["resignation"]["properties"]["memberId"]["refersTo"] =
        all_of(vec![identity(), join_request_lookup()]);
    assert_refused(
        contract(schema),
        &format!("declares references for up to {} values", limit + 1),
    );
}

#[test]
fn should_refuse_an_expression_below_protocol_version_14_and_accept_it_at_14() {
    let schema = charter_contract(any_of(vec![
        join_request_lookup(),
        all_of(vec![identity(), added_moderator_lookup()]),
    ]));
    let platform_version_13 = PlatformVersion::get(13).expect("platform version 13 should exist");

    // Meta-schema v2 knows no refersTo, so a registering parse refuses it
    contract_on(schema.clone(), true, platform_version_13)
        .expect_err("protocol version 13 should refuse the declaration");
    // A parse predating refersTo ignores the whole declaration, expression and all
    let ignored = contract_on(schema.clone(), false, platform_version_13)
        .expect("protocol version 13 should parse it as a plain identifier");
    assert_eq!(
        property_type(&ignored, "memberId"),
        DocumentPropertyType::Identifier
    );

    let accepted = contract_on(schema, true, PlatformVersion::latest()).expect("parses");
    assert!(matches!(
        property_type(&accepted, "memberId"),
        DocumentPropertyType::IdentifierWithReference(DocumentPropertyReferenceTarget::AnyOf(_))
    ));
}

/// A contract is serialized as its schemas, so an expression round trips as
/// the declaration it was registered with, and a contract without one
/// serializes exactly as it did before the form existed.
#[test]
fn should_round_trip_a_contract_with_an_expression_through_platform_serialization() {
    let platform_version = PlatformVersion::latest();
    let depth = usize::from(
        platform_version
            .system_limits
            .max_reference_expression_depth,
    );

    for schema in [
        charter_contract(join_request_lookup()),
        charter_contract(any_of(vec![
            identity(),
            join_request_lookup(),
            added_moderator_lookup(),
        ])),
        charter_contract(nested_to(depth)),
        charter_contract_with_members(
            all_of(vec![
                join_request_lookup(),
                any_of(vec![identity(), added_moderator_lookup()]),
            ]),
            15,
        ),
    ] {
        let original = contract(schema.clone()).expect("parses");
        let bytes = original
            .serialize_to_bytes_with_platform_version(platform_version)
            .expect("the contract should serialize");
        let recovered =
            DataContract::versioned_deserialize_untrusted(&bytes, false, platform_version)
                .expect("the contract should deserialize");

        assert_eq!(original, recovered, "contract {schema}");
        let resignation = |contract: &DataContract| {
            contract
                .document_type_for_name("resignation")
                .expect("the resignation document type")
                .flattened_properties()
                .clone()
        };
        assert_eq!(resignation(&original), resignation(&recovered));
    }

    // Without an expression, the parsed reference is exactly the lookup it was
    let without = contract(charter_contract(join_request_lookup())).expect("parses");
    assert_eq!(
        property_type(&without, "memberId"),
        DocumentPropertyType::IdentifierWithReference(expected_join_request_lookup())
    );
}
