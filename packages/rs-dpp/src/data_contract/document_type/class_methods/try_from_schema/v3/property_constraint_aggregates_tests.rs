//! `countOf` and `sumOf` in `propertyConstraints` rules (protocol version 14):
//! a rule may read a total another document type of the contract keeps in a
//! count or sum tree, which registration checks once every type is parsed. A
//! stored contract is not checked again.

use super::immutable_tests::expect_structure_error;
use super::*;
use crate::consensus::basic::basic_error::BasicError;
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
use crate::data_contract::document_type::accessors::DocumentTypeV2Getters;
use crate::data_contract::document_type::methods::DocumentTypeV0Methods;
use crate::data_contract::document_type::property_constraints::{
    AggregateBinding, AggregateKind, AggregateRead, DocumentSystemValues, EqualityKind,
    PropertyRead, SystemChange,
};
use crate::data_contract::DataContract;
use platform_value::string_encoding::Encoding;
use serde_json::json;

/// A contract of four types:
/// * `listing`, whose trees keep its count and the total of its prices (at
///   most 10^9 each, so that a sum tree takes them), the
///   count of each owner's listings (`byOwner`) and of each owner's listings
///   by status (`byOwnerStatus`), and the total price of each category
///   (`byCategory`);
/// * `pledge`, whose `byCampaign` index keeps the count and the total amount
///   of each campaign's pledges;
/// * `profile`, with a unique index by owner;
/// * `campaign`, declaring `campaign_rules`, with required integer, identifier,
///   string and boolean properties to match by and an optional identifier.
///
/// `listing_rules` go on `listing`.
fn contract(
    campaign_rules: Option<serde_json::Value>,
    listing_rules: Option<serde_json::Value>,
    full_validation: bool,
) -> Result<DataContract, ProtocolError> {
    let identifier = |position: u32| {
        json!({
            "type": "array",
            "byteArray": true,
            "minItems": 32,
            "maxItems": 32,
            "contentMediaType": "application/x.dash.dpp.identifier",
            "position": position
        })
    };
    let mut listing = json!({
        "type": "object",
        "documentsMutable": true,
        "documentsCountable": true,
        "documentsSummable": "price",
        "properties": {
            "price": { "type": "integer", "minimum": 0, "maximum": 1000000000, "position": 0 },
            "status": {
                "type": "string",
                "enum": ["open", "closed"],
                "maxLength": 10,
                "position": 1
            },
            "category": { "type": "integer", "minimum": 0, "maximum": 100, "position": 2 },
            "featured": { "type": "boolean", "position": 3 },
            "sellerRef": identifier(4)
        },
        "required": ["price", "status", "category"],
        "indices": [
            {
                "name": "byOwner",
                "properties": [{ "$ownerId": "asc" }],
                "countable": "countable"
            },
            {
                "name": "byOwnerStatus",
                "properties": [{ "$ownerId": "asc" }, { "status": "asc" }],
                "countable": "countable"
            },
            {
                "name": "byCategory",
                "properties": [{ "category": "asc" }],
                "summable": "price"
            }
        ],
        "additionalProperties": false
    });
    if let Some(rules) = listing_rules {
        listing["propertyConstraints"] = rules;
    }
    let mut campaign = json!({
        "type": "object",
        "documentsMutable": true,
        "properties": {
            "goal": { "type": "integer", "minimum": 0, "position": 0 },
            "campaignRef": identifier(1),
            "tier": { "type": "integer", "minimum": 0, "maximum": 9, "position": 2 },
            "kind": {
                "type": "string",
                "enum": ["open", "closed"],
                "maxLength": 10,
                "position": 3
            },
            "flag": { "type": "boolean", "position": 4 },
            "maybeRef": identifier(5)
        },
        "required": ["goal", "campaignRef", "tier", "kind", "flag"],
        "additionalProperties": false
    });
    if let Some(rules) = campaign_rules {
        campaign["propertyConstraints"] = rules;
    }
    let contract = json!({
        "$formatVersion": "1",
        "id": Identifier::from([7; 32]).to_string(Encoding::Base58),
        "ownerId": Identifier::from([8; 32]).to_string(Encoding::Base58),
        "version": 1,
        "documentSchemas": {
            "listing": listing,
            "pledge": {
                "type": "object",
                "properties": {
                    "campaignId": identifier(0),
                    "amount": { "type": "integer", "minimum": 0, "maximum": 1000000000, "position": 1 },
                    "tier": { "type": "integer", "minimum": 0, "maximum": 9, "position": 2 }
                },
                "required": ["campaignId", "amount", "tier"],
                "indices": [{
                    "name": "byCampaign",
                    "properties": [{ "campaignId": "asc" }],
                    "countable": "countable",
                    "summable": "amount"
                }],
                "additionalProperties": false
            },
            "profile": {
                "type": "object",
                "properties": {
                    "handle": { "type": "string", "maxLength": 20, "position": 0 }
                },
                "required": ["handle"],
                "indices": [{
                    "name": "byOwner",
                    "properties": [{ "$ownerId": "asc" }],
                    "unique": true,
                    "countable": "countable"
                }],
                "additionalProperties": false
            },
            "campaign": campaign
        }
    });
    DataContract::from_value(
        platform_value::to_value(contract).expect("the contract converts"),
        full_validation,
        PlatformVersion::latest(),
    )
}

/// A contract whose `campaign` type declares one rule, `rule`.
fn campaign_rule(rule: serde_json::Value) -> Result<DataContract, ProtocolError> {
    contract(Some(json!({ "rule": rule })), None, true)
}

#[test]
fn should_register_totals_a_count_or_sum_tree_keeps() {
    let contract = contract(
        Some(json!({
            "pledgedWithinGoal": {
                "lessThanOrEqual": [
                    { "sumOf": ["pledge", "amount", { "campaignId": "campaignRef" }] },
                    "goal"
                ]
            },
            "fewPledges": {
                "lessThan": [{ "countOf": ["pledge", { "campaignId": "campaignRef" }] }, 100]
            },
            "ownerHasListingsOrThereAreMany": {
                "greaterThan": [
                    { "add": [
                        { "countOf": ["listing", { "$ownerId": "$ownerId" }] },
                        { "countOf": ["listing"] }
                    ] },
                    0
                ]
            }
        })),
        Some(json!({
            "atMostTenPerOwner": {
                "lessThanOrEqual": [{ "countOf": ["listing", { "$ownerId": "$ownerId" }] }, 10]
            },
            "openListingsOfOwner": {
                "lessThan": [
                    {
                        "countOf": [
                            "listing",
                            { "$ownerId": "$ownerId", "status": { "const": "open" } }
                        ]
                    },
                    5
                ]
            },
            "categoryTotal": {
                "lessThan": [
                    { "sumOf": ["listing", "price", { "category": "category" }] },
                    1000000
                ]
            },
            "allListingPrices": {
                "lessThan": [{ "sumOf": ["listing", "price"] }, 1000000000]
            }
        })),
        true,
    )
    .expect("every total is kept by a tree");

    let campaign = contract
        .document_type_for_name("campaign")
        .expect("campaign");
    let rules = campaign.property_constraints();
    assert_eq!(rules.len(), 3);
    assert_eq!(
        rules["pledgedWithinGoal"].aggregate_reads(),
        [&AggregateRead {
            kind: AggregateKind::Sum {
                property: "amount".to_string()
            },
            document_type: "pledge".to_string(),
            filter: [(
                "campaignId".to_string(),
                AggregateBinding::Property {
                    path: "campaignRef".to_string(),
                    kind: Some(EqualityKind::Identifier),
                }
            )]
            .into(),
            of_own_type: false,
        }]
    );
    assert_eq!(
        rules["pledgedWithinGoal"].property_reads(),
        [
            ("campaignRef", PropertyRead::Identifier),
            ("goal", PropertyRead::Value)
        ]
    );
    assert!(rules["ownerHasListingsOrThereAreMany"].reads_owner());
    assert!(!rules["fewPledges"].reads_owner());

    let listing = contract.document_type_for_name("listing").expect("listing");
    let own = listing.property_constraints();
    assert_eq!(own.len(), 4);
    assert!(own["atMostTenPerOwner"].aggregate_reads()[0].of_own_type);
    assert!(own["atMostTenPerOwner"].reads_owner());
    // The category the document counts toward is its own, whoever owns it
    assert!(!own["categoryTotal"].reads_owner());
}

/// A total no tree keeps, or a filter whose keys and values do not match in
/// kind, is refused when the contract registers.
#[test]
fn should_refuse_a_total_no_tree_keeps() {
    let refused = |rule: serde_json::Value, needle: &str| {
        expect_structure_error(campaign_rule(rule), needle);
    };
    let at_most = |operand: serde_json::Value| json!({ "lessThanOrEqual": [operand, 10] });

    refused(
        at_most(json!({ "countOf": ["order"] })),
        "document type \"campaign\" propertyConstraints rule \"rule\" counts \"order\", which is \
         no document type of this contract",
    );
    refused(
        at_most(json!({ "countOf": ["pledge"] })),
        "counts every \"pledge\" document, which needs documentsCountable on \"pledge\"",
    );
    refused(
        at_most(json!({ "sumOf": ["listing", "category"] })),
        "totals \"category\" over every \"listing\" document, which needs documentsSummable: \
         \"category\" on \"listing\"",
    );
    // `byOwnerStatus` keeps the count by owner and status, not by status alone
    refused(
        at_most(json!({ "countOf": ["listing", { "status": "kind" }] })),
        "counts \"listing\" by \"status\", which no countable index of \"listing\" whose \
         properties are exactly those keys answers",
    );
    refused(
        at_most(json!({ "sumOf": ["listing", "price", { "$ownerId": "$ownerId" }] })),
        "totals \"price\" of \"listing\" by \"$ownerId\", which no index with summable: \
         \"price\" of \"listing\"",
    );
    // A unique index keeps no count
    refused(
        at_most(json!({ "countOf": ["profile", { "$ownerId": "$ownerId" }] })),
        "counts \"profile\" by \"$ownerId\", which no countable index of \"profile\"",
    );
    refused(
        at_most(json!({ "countOf": ["listing", { "shade": "tier" }] })),
        "counts \"listing\" by \"shade\", which is not a property of \"listing\"",
    );
    refused(
        at_most(json!({ "countOf": ["listing", { "featured": "flag" }] })),
        "counts \"listing\" by \"featured\", which has type boolean",
    );
    refused(
        at_most(json!({ "countOf": ["listing", { "$ownerId": 3 }] })),
        "counts \"listing\" with \"$ownerId\", an identifier, at 3, an integer",
    );
    refused(
        at_most(json!({ "countOf": ["listing", { "$ownerId": "tier" }] })),
        "counts \"listing\" with \"$ownerId\", an identifier, at \"tier\", an integer",
    );
    refused(
        at_most(json!({ "sumOf": ["listing", "price", { "category": "kind" }] })),
        "totals \"price\" of \"listing\" with \"category\", an integer, at \"kind\", a string",
    );
    refused(
        at_most(json!({ "sumOf": ["listing", "price", { "category": { "const": "3" } }] })),
        "with \"category\" at the constant \"3\", but \"category\" is an integer: write the \
         integer bare",
    );
    refused(
        at_most(json!({ "countOf": ["listing", { "$ownerId": { "const": "not-base58!" } }] })),
        "with \"$ownerId\" at \"not-base58!\", which is not a base58 identifier",
    );
    refused(
        at_most(json!({
            "countOf": ["listing", { "$ownerId": "$ownerId", "status": { "const": "opne" } }]
        })),
        "with \"status\" at \"opne\", which is not one of its enum values",
    );
    refused(
        at_most(json!({ "sumOf": ["listing", "price", { "category": "flag" }] })),
        "with \"category\" at \"flag\", which has type boolean",
    );
}

/// The value a key is matched by must always be there, so that every write
/// reads the total of the documents matching it.
#[test]
fn should_refuse_a_value_the_document_may_leave_out() {
    expect_structure_error(
        campaign_rule(json!({
            "lessThan": [{ "countOf": ["listing", { "sellerRef": "maybeRef" }] }, 3]
        })),
        "rule \"rule\" matches a countOf by \"maybeRef\", but the document type does not \
         require \"maybeRef\"",
    );
}

/// At most `max_property_constraint_aggregates` distinct totals per type, a
/// total read twice counting once; checked under full validation only.
#[test]
fn should_cap_the_distinct_totals_a_type_reads() {
    let limit = PlatformVersion::latest()
        .system_limits
        .max_property_constraint_aggregates;
    assert_eq!(limit, 4);
    let total = |tier: u32| json!({ "sumOf": ["listing", "price", { "category": tier }] });
    let rules = |count: u32| {
        (0..count)
            .map(|tier| {
                (
                    format!("rule{tier}"),
                    json!({ "lessThan": [total(tier), 1000] }),
                )
            })
            .collect::<serde_json::Map<_, _>>()
    };

    contract(Some(rules(4).into()), None, true).expect("four totals are within the limit");
    let mut twice = rules(4);
    twice.insert("again".to_string(), json!({ "greaterThan": [total(0), 0] }));
    contract(Some(twice.into()), None, true).expect("a total read twice counts once");
    expect_structure_error(
        contract(Some(rules(5).into()), None, true),
        "reads 5 distinct countOf and sumOf totals, above the maximum of 4",
    );
    contract(Some(rules(5).into()), None, false).expect("a stored contract is not re-checked");
}

/// A stored contract is not checked again: the checks ran when it registered,
/// and nothing they rely on changes on an update.
#[test]
fn should_not_check_a_stored_contract_again() {
    let stored = contract(
        Some(json!({ "rule": { "lessThan": [{ "countOf": ["order"] }, 3] } })),
        None,
        false,
    )
    .expect("a stored contract parses");
    assert_eq!(
        stored
            .document_type_for_name("campaign")
            .expect("campaign")
            .property_constraints()["rule"]
            .aggregate_reads()[0]
            .document_type,
        "order"
    );
}

/// The meta-schema checks the shapes of `countOf` and `sumOf` when a contract
/// registers.
#[test]
fn should_refuse_malformed_totals_through_the_meta_schema() {
    for operand in [
        json!({ "countOf": [] }),
        json!({ "countOf": ["listing", {}] }),
        json!({ "countOf": ["listing", { "$ownerId": "$ownerId" }, 1] }),
        json!({ "sumOf": ["pledge"] }),
        json!({ "sumOf": ["pledge", "amount", { "campaignId": true }] }),
        json!({ "countOf": ["listing", { "$createdAt": 1 }] }),
        json!({ "countOf": ["listing", { "status": { "const": "open", "extra": 1 } }] }),
    ] {
        let registered = campaign_rule(json!({ "lessThan": [operand.clone(), 3] }));
        assert!(
            matches!(
                &registered,
                Err(ProtocolError::ConsensusError(boxed))
                    if matches!(**boxed, ConsensusError::BasicError(BasicError::JsonSchemaError(_)))
            ),
            "{operand}: the meta-schema should refuse it, got {registered:?}"
        );
    }
}

/// An indexOnly type neither reads a total, which its delete is not given,
/// nor is totalled.
#[test]
fn should_keep_totals_away_from_index_only_types() {
    let rows = |rules: Option<serde_json::Value>| {
        let mut rows = json!({
            "type": "object",
            "indexOnly": true,
            "documentsMutable": false,
            "properties": {
                "day": { "type": "integer", "minimum": 0, "position": 0 }
            },
            "required": ["day"],
            "indices": [{
                "name": "byDay",
                "properties": [{ "day": "asc" }],
                "countable": "countable"
            }],
            "additionalProperties": false
        });
        if let Some(rules) = rules {
            rows["propertyConstraints"] = rules;
        }
        let contract = json!({
            "$formatVersion": "1",
            "id": Identifier::from([7; 32]).to_string(Encoding::Base58),
            "ownerId": Identifier::from([8; 32]).to_string(Encoding::Base58),
            "version": 1,
            "documentSchemas": {
                "row": rows,
                "note": {
                    "type": "object",
                    "properties": {
                        "day": { "type": "integer", "minimum": 0, "position": 0 }
                    },
                    "required": ["day"],
                    "propertyConstraints": {
                        "rule": {
                            "lessThan": [{ "countOf": ["row", { "day": "day" }] }, 3]
                        }
                    },
                    "additionalProperties": false
                }
            }
        });
        DataContract::from_value(
            platform_value::to_value(contract).expect("the contract converts"),
            true,
            PlatformVersion::latest(),
        )
    };
    expect_structure_error(
        rows(None),
        "rule \"rule\" counts \"row\", an indexOnly type, whose rows a countOf or sumOf does not \
         total",
    );
    expect_structure_error(
        rows(Some(json!({
            "own": { "lessThan": [{ "countOf": ["row", { "day": "day" }] }, 3] }
        }))),
        "rule \"own\" reads a countOf, which a delete of an indexOnly document is not given",
    );
}

/// The values a filter takes are typed as the counted type keys its index, so
/// that an integer, a string constant, an identifier constant, a property or
/// the owner each match a document holding the same value; the index the read
/// is answered from is the one whose properties are exactly its keys, and a
/// document adds to a total of its own type only when it matches.
#[test]
fn should_match_a_document_by_the_values_its_filter_takes() {
    let platform_version = PlatformVersion::latest();
    let contract = contract(None, None, true).expect("the contract registers");
    let listing = contract.document_type_for_name("listing").expect("listing");
    let owner = Identifier::from([3; 32]);
    let other = Identifier::from([4; 32]);
    let document = |category: u64, status: &str| {
        Value::from(std::collections::BTreeMap::from([
            ("price".to_string(), Value::U64(40)),
            ("category".to_string(), Value::U64(category)),
            ("status".to_string(), Value::Text(status.to_string())),
        ]))
    };
    let read = |kind: AggregateKind, filter: Vec<(&str, AggregateBinding)>| AggregateRead {
        kind,
        document_type: "listing".to_string(),
        filter: filter
            .into_iter()
            .map(|(key, binding)| (key.to_string(), binding))
            .collect(),
        of_own_type: true,
    };
    let price = || AggregateKind::Sum {
        property: "price".to_string(),
    };

    // An integer constant, against the category's own integer type
    let by_category = read(price(), vec![("category", AggregateBinding::Integer(3))]);
    assert_eq!(
        by_category
            .answering_index(&listing)
            .map(|index| index.name.as_str()),
        Some("byCategory")
    );
    let values = by_category
        .filter_values(listing, &document(9, "open"), owner, platform_version)
        .expect("reads")
        .expect("an integer the key holds");
    assert_eq!(
        by_category
            .contribution(
                listing,
                &values,
                &document(3, "open"),
                owner,
                platform_version
            )
            .expect("reads"),
        40
    );
    assert_eq!(
        by_category
            .contribution(
                listing,
                &values,
                &document(4, "open"),
                owner,
                platform_version
            )
            .expect("reads"),
        0
    );

    // The owner and a string constant, answered by the two-key index
    let open_of_owner = read(
        AggregateKind::Count,
        vec![
            ("$ownerId", AggregateBinding::Owner),
            ("status", AggregateBinding::Constant("open".to_string())),
        ],
    );
    assert_eq!(
        open_of_owner
            .answering_index(&listing)
            .map(|index| index.name.as_str()),
        Some("byOwnerStatus")
    );
    let values = open_of_owner
        .filter_values(listing, &document(1, "closed"), owner, platform_version)
        .expect("reads")
        .expect("the owner and a string");
    let counted = |data: Value, owner_id: Identifier| {
        open_of_owner
            .contribution(listing, &values, &data, owner_id, platform_version)
            .expect("reads")
    };
    assert_eq!(counted(document(1, "open"), owner), 1);
    assert_eq!(counted(document(1, "closed"), owner), 0);
    assert_eq!(counted(document(1, "open"), other), 0);

    // An identifier constant for the owner key
    let of_one_owner = read(
        AggregateKind::Count,
        vec![(
            "$ownerId",
            AggregateBinding::Constant(owner.to_string(Encoding::Base58)),
        )],
    );
    let values = of_one_owner
        .filter_values(listing, &document(1, "open"), other, platform_version)
        .expect("reads")
        .expect("a base58 identifier");
    assert_eq!(
        values,
        [("$ownerId".to_string(), Value::Identifier([3; 32]))]
    );

    // A property the document leaves out matches nothing
    let by_bound_category = read(
        price(),
        vec![(
            "category",
            AggregateBinding::Property {
                path: "tier".to_string(),
                kind: None,
            },
        )],
    );
    assert_eq!(
        by_bound_category
            .filter_values(listing, &document(1, "open"), owner, platform_version)
            .expect("reads"),
        None
    );

    // Another type's documents never count toward it
    let not_own = AggregateRead {
        of_own_type: false,
        ..by_category.clone()
    };
    let values = not_own
        .filter_values(listing, &document(3, "open"), owner, platform_version)
        .expect("reads")
        .expect("an integer");
    assert_eq!(
        not_own
            .contribution(
                listing,
                &values,
                &document(3, "open"),
                owner,
                platform_version
            )
            .expect("reads"),
        0
    );
}

/// A filter key whose schema is a `$ref` to one of the contract's `$defs` is
/// held to the definition's `enum`, as one declared inline is: a constant the
/// enum lists registers, and a misspelt one is refused.
#[test]
fn should_hold_a_constant_to_the_enum_of_a_key_declared_by_reference() {
    let counting = |status: &str| {
        let contract = json!({
            "$formatVersion": "1",
            "id": Identifier::from([7; 32]).to_string(Encoding::Base58),
            "ownerId": Identifier::from([8; 32]).to_string(Encoding::Base58),
            "version": 1,
            "schemaDefs": {
                "status": { "type": "string", "enum": ["open", "closed"], "maxLength": 10 }
            },
            "documentSchemas": {
                "listing": {
                    "type": "object",
                    "documentsCountable": true,
                    "properties": {
                        "status": { "$ref": "#/$defs/status", "position": 0 }
                    },
                    "required": ["status"],
                    "indices": [{
                        "name": "byOwnerStatus",
                        "properties": [{ "$ownerId": "asc" }, { "status": "asc" }],
                        "countable": "countable"
                    }],
                    "additionalProperties": false
                },
                "seller": {
                    "type": "object",
                    "properties": {
                        "note": { "type": "string", "maxLength": 20, "position": 0 }
                    },
                    "additionalProperties": false,
                    "propertyConstraints": {
                        "rule": {
                            "lessThanOrEqual": [
                                {
                                    "countOf": [
                                        "listing",
                                        { "$ownerId": "$ownerId", "status": { "const": status } }
                                    ]
                                },
                                10
                            ]
                        }
                    }
                }
            }
        });
        DataContract::from_value(
            platform_value::to_value(contract).expect("the contract converts"),
            true,
            PlatformVersion::latest(),
        )
    };

    counting("open").expect("a constant the referenced enum lists registers");
    expect_structure_error(
        counting("opne"),
        "with \"status\" at \"opne\", which is not one of its enum values",
    );
}

/// A type with a contested index cannot total its own documents: a contested
/// create waits in its contest's storage, outside the count and sum trees, and
/// the document a contest awards is stored without the rules being judged, so
/// the rule could be passed. Another type may still total it, as it totals any
/// type whose writes it does not judge.
#[test]
fn should_refuse_a_total_of_its_own_type_with_a_contested_index() {
    let contract = |name_rules: Option<serde_json::Value>,
                    note_rules: Option<serde_json::Value>| {
        let mut name = json!({
            "type": "object",
            "documentsMutable": false,
            "indices": [
                {
                    "name": "byLabel",
                    "properties": [{ "normalizedLabel": "asc" }],
                    "unique": true,
                    "contested": {
                        "fieldMatches": [
                            { "field": "normalizedLabel", "regexPattern": "^[a-z]{3,}$" }
                        ],
                        "resolution": 0
                    }
                },
                {
                    "name": "byOwner",
                    "properties": [{ "$ownerId": "asc" }],
                    "countable": "countable"
                }
            ],
            "properties": {
                "normalizedLabel": { "type": "string", "maxLength": 50, "position": 0 }
            },
            "required": ["normalizedLabel"],
            "additionalProperties": false
        });
        if let Some(rules) = name_rules {
            name["propertyConstraints"] = rules;
        }
        let mut note = json!({
            "type": "object",
            "properties": {
                "text": { "type": "string", "maxLength": 50, "position": 0 }
            },
            "additionalProperties": false
        });
        if let Some(rules) = note_rules {
            note["propertyConstraints"] = rules;
        }
        let contract = json!({
            "$formatVersion": "1",
            "id": Identifier::from([7; 32]).to_string(Encoding::Base58),
            "ownerId": Identifier::from([8; 32]).to_string(Encoding::Base58),
            "version": 1,
            "documentSchemas": { "name": name, "note": note }
        });
        DataContract::from_value(
            platform_value::to_value(contract).expect("the contract converts"),
            true,
            PlatformVersion::latest(),
        )
    };
    let one_per_owner = json!({
        "onePerOwner": {
            "lessThanOrEqual": [{ "countOf": ["name", { "$ownerId": "$ownerId" }] }, 1]
        }
    });

    expect_structure_error(
        contract(Some(one_per_owner.clone()), None),
        "rule \"onePerOwner\" counts \"name\", its own type, which has a contested index",
    );
    contract(None, Some(one_per_owner)).expect("another type may count it");
}

/// A client gives no totals, and a rule reading one is not judged; consensus
/// gives the totals of every rule it judges, so one missing there is an error
/// in the code building the write rather than a rule silently skipped.
#[test]
fn should_refuse_to_skip_a_rule_whose_total_consensus_did_not_read() {
    let platform_version = PlatformVersion::latest();
    let contract = contract(
        None,
        Some(json!({
            "atMostTenPerOwner": {
                "lessThanOrEqual": [{ "countOf": ["listing", { "$ownerId": "$ownerId" }] }, 10]
            }
        })),
        true,
    )
    .expect("the contract registers");
    let listing = contract.document_type_for_name("listing").expect("listing");
    let data = Value::from(std::collections::BTreeMap::from([
        ("price".to_string(), Value::U64(40)),
        ("category".to_string(), Value::U64(1)),
        ("status".to_string(), Value::Text("open".to_string())),
    ]));
    let owner = Identifier::from([3; 32]);
    let read = listing.property_constraints()["atMostTenPerOwner"].aggregate_reads()[0].clone();
    let with = |aggregates| DocumentSystemValues {
        aggregates,
        ..DocumentSystemValues::owned_by(owner)
    };

    let client = listing
        .validate_property_constraints(&data, &with(None), platform_version)
        .expect("a client's check runs");
    assert!(client.is_valid(), "a client skips the rule");

    let refused = listing
        .validate_property_constraints(
            &data,
            &with(Some([(read.clone(), 11)].into())),
            platform_version,
        )
        .expect("consensus judges the rule");
    assert!(!refused.is_valid(), "11 is above the cap");

    let missing = listing.validate_property_constraints(
        &data,
        &with(Some(Default::default())),
        platform_version,
    );
    assert!(
        matches!(
            &missing,
            Err(ProtocolError::CorruptedCodeExecution(message))
                if message.contains("rule atMostTenPerOwner of document type listing reads a countOf of listing that consensus did not read")
        ),
        "{missing:?}"
    );
}

/// A transfer, a purchase or a price update judges only the rules it can
/// break, so consensus reads only their totals: a missing total of a rule it
/// judges is an error, one of a rule it does not judge is not, and a client
/// skips both.
#[test]
fn should_refuse_to_skip_a_rule_a_system_change_judges_without_its_total() {
    let platform_version = PlatformVersion::latest();
    let contract = json!({
        "$formatVersion": "1",
        "id": Identifier::from([7; 32]).to_string(Encoding::Base58),
        "ownerId": Identifier::from([8; 32]).to_string(Encoding::Base58),
        "version": 1,
        "documentSchemas": {
            "offer": {
                "type": "object",
                "documentsMutable": true,
                "transferable": 1,
                "tradeMode": 1,
                "documentsCountable": true,
                "properties": {
                    "category": { "type": "integer", "minimum": 0, "maximum": 100, "position": 0 },
                    "price": { "type": "integer", "minimum": 0, "maximum": 1000000000, "position": 1 },
                    "endsAt": { "type": "integer", "minimum": 0, "position": 2 }
                },
                "required": ["category", "price", "endsAt", "$updatedAt"],
                "indices": [
                    {
                        "name": "byOwner",
                        "properties": [{ "$ownerId": "asc" }],
                        "countable": "countable"
                    },
                    {
                        "name": "byCategory",
                        "properties": [{ "category": "asc" }],
                        "summable": "price"
                    }
                ],
                "propertyConstraints": {
                    "ownerCap": {
                        "lessThanOrEqual": [{ "countOf": ["offer", { "$ownerId": "$ownerId" }] }, 10]
                    },
                    "openWindow": {
                        "allOf": [
                            { "lessThanOrEqual": ["$updatedAt", "endsAt"] },
                            { "lessThanOrEqual": [{ "countOf": ["offer"] }, 100] }
                        ]
                    },
                    "categoryCap": {
                        "lessThanOrEqual": [
                            { "sumOf": ["offer", "price", { "category": "category" }] },
                            1000
                        ]
                    }
                },
                "additionalProperties": false
            }
        }
    });
    let contract = DataContract::from_value(
        platform_value::to_value(contract).expect("the contract converts"),
        true,
        platform_version,
    )
    .expect("the contract registers");
    let offer = contract.document_type_for_name("offer").expect("offer");
    let rules = offer.property_constraints();
    let owner_cap = rules["ownerCap"].aggregate_reads()[0].clone();
    let open_window = rules["openWindow"].aggregate_reads()[0].clone();
    let data = BTreeMap::from([
        ("category".to_string(), Value::U64(1)),
        ("price".to_string(), Value::U64(40)),
        ("endsAt".to_string(), Value::U64(1000)),
    ]);
    let judge = |change: SystemChange, aggregates: Option<Vec<(AggregateRead, i128)>>| {
        let system = DocumentSystemValues {
            updated_at: Some(500),
            aggregates: aggregates.map(|aggregates| aggregates.into_iter().collect()),
            ..DocumentSystemValues::owned_by(Identifier::from([3; 32]))
        };
        offer.validate_property_constraints_for_system_change(
            &data,
            &system,
            change,
            platform_version,
        )
    };
    let is_unread = |result: &Result<_, ProtocolError>, rule: &str| {
        matches!(
            result,
            Err(ProtocolError::CorruptedCodeExecution(message))
                if message.starts_with(&format!("rule {rule} of document type offer reads a countOf"))
        )
    };

    // A transfer judges the owner's cap alone
    let transfer_without = judge(SystemChange::Transfer, Some(vec![]));
    assert!(
        is_unread(&transfer_without, "ownerCap"),
        "{transfer_without:?}"
    );
    let transfer = judge(SystemChange::Transfer, Some(vec![(owner_cap.clone(), 3)]))
        .expect("the transfer's rules are judged");
    assert!(
        transfer.is_valid(),
        "openWindow and categoryCap are not judged"
    );

    // A price update judges the window alone
    let price_update_without = judge(SystemChange::PriceUpdate, Some(vec![]));
    assert!(
        is_unread(&price_update_without, "openWindow"),
        "{price_update_without:?}"
    );
    let price_update = judge(SystemChange::PriceUpdate, Some(vec![(open_window, 101)]))
        .expect("the price update's rules are judged");
    assert!(
        !price_update.is_valid(),
        "101 offers is above the window's 100"
    );

    // A client skips the rules reading totals
    for change in [SystemChange::Transfer, SystemChange::PriceUpdate] {
        let skipped = judge(change, None).expect("a client's check runs");
        assert!(skipped.is_valid());
    }
}
