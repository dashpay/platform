//! `propertyConstraints` rules reading the stored version of a document
//! through `$old.`, and `$revision`, under generation 3 (protocol version 14):
//! what registration admits on which document types, and how the parsed type
//! judges a write with the stored version, without it, and for a moderator's
//! change of fields.

use super::immutable_tests::{expect_structure_error, parse_dispatched};
use super::*;
use crate::data_contract::document_type::accessors::DocumentTypeV2Getters;
use crate::data_contract::document_type::methods::DocumentTypeV0Methods;
use crate::data_contract::document_type::property_constraints::{
    DocumentSystemValues, StoredDocument, StoredSystemValues, StoredVersion,
};
use platform_value::platform_value;

/// A type, parsed as `post`: two integers, a string `status`, an identifier `buyerId`,
/// and what `doctype` adds or replaces at the type's top level (its
/// mutability, `required`, `propertyConstraints` and the rest).
fn listing_schema(doctype: Value) -> Value {
    let mut schema = platform_value!({
        "type": "object",
        "documentsMutable": true,
        "properties": {
            "price": { "type": "integer", "minimum": 0, "position": 0 },
            "fee": { "type": "integer", "minimum": 0, "position": 1 },
            "status": {
                "type": "string",
                "enum": ["open", "closed"],
                "maxLength": 10,
                "position": 2
            },
            "buyerId": {
                "type": "array",
                "byteArray": true,
                "minItems": 32,
                "maxItems": 32,
                "contentMediaType": "application/x.dash.dpp.identifier",
                "position": 3
            }
        },
        "required": ["price", "fee", "$updatedAt"],
        "additionalProperties": false
    });
    let Value::Map(entries) = doctype else {
        panic!("the doctype keys are an object");
    };
    for (key, value) in entries {
        schema
            .set_value(key.as_text().expect("a text key"), value)
            .expect("the doctype key applies");
    }
    schema
}

fn parse_listing(doctype: Value, full_validation: bool) -> Result<DocumentType, ProtocolError> {
    parse_dispatched(
        listing_schema(doctype),
        PlatformVersion::latest(),
        full_validation,
    )
}

/// A mutable type registers rules reading every kind of stored value and
/// `$revision`, on both paths.
#[test]
fn should_register_stored_reads_and_the_revision_on_a_type_with_a_later_write() {
    let rules = platform_value!({
        "priceNeverRises": {
            "lessThanOrEqual": ["price", { "ifAbsent": ["$old.price", 1000000] }]
        },
        "closedStaysClosed": {
            "ifThen": [
                { "equal": ["$old.status", { "const": "closed" }] },
                { "equal": ["status", { "const": "closed" }] }
            ]
        },
        "buyerKept": {
            "anyOf": [{ "absent": "$old.buyerId" }, { "equal": ["buyerId", "$old.buyerId"] }]
        },
        "editsAnHourApart": {
            "greaterThanOrEqual": [{ "subtract": ["$updatedAt", "$old.$updatedAt"] }, 3600000]
        },
        "atMostFiveEdits": { "lessThanOrEqual": ["$revision", 6] },
        "sameOwner": { "equal": ["$ownerId", "$old.$ownerId"] },
        "revisionStepsByOne": { "equal": ["$revision", { "add": ["$old.$revision", 1] }] }
    });
    for full_validation in [true, false] {
        let document_type = parse_listing(
            platform_value!({ "propertyConstraints": rules.clone() }),
            full_validation,
        )
        .unwrap_or_else(|error| {
            panic!("the rules register (full_validation: {full_validation}): {error}")
        });
        let parsed = document_type.property_constraints();
        assert_eq!(parsed.len(), 7);
        assert!(!parsed["atMostFiveEdits"].reads_stored());
        assert!(parsed
            .iter()
            .filter(|(name, _)| name.as_str() != "atMostFiveEdits")
            .all(|(_, rule)| rule.reads_stored()));
    }

    // A transferable type keeps a revision too, however immutable its documents
    let transferable = parse_listing(
        platform_value!({
            "documentsMutable": false,
            "transferable": 1,
            "propertyConstraints": {
                "resoldAfterADay": {
                    "ifThen": [
                        { "notEqual": ["$ownerId", "$old.$ownerId"] },
                        { "greaterThanOrEqual": ["$revision", 2] }
                    ]
                }
            }
        }),
        true,
    );
    assert!(transferable.is_ok(), "{transferable:?}");
}

/// On a type no write of which replaces a stored version, the stored version
/// is always absent and the revision always the first: a rule reading either
/// is refused, in `deleteConstraints` too for `$revision`.
#[test]
fn should_refuse_stored_reads_and_the_revision_on_a_type_without_a_later_write() {
    let no_later_write = "its documents are immutable, not transferable, not sold at a price \
                          their owner sets, and keep no fields for their moderators";
    for (doctype, needle) in [
        (
            platform_value!({
                "documentsMutable": false,
                "propertyConstraints": { "rule": { "greaterThan": ["price", "$old.price"] } }
            }),
            format!(
                "document type \"post\" propertyConstraints rule \"rule\" reads the stored \
                 version through $old., but no write of the type replaces one: {no_later_write}"
            ),
        ),
        (
            platform_value!({
                "documentsMutable": false,
                "propertyConstraints": { "rule": { "notEqual": ["$ownerId", "$old.$ownerId"] } }
            }),
            "propertyConstraints rule \"rule\" reads the stored version through $old., but no \
             write of the type replaces one"
                .to_string(),
        ),
        (
            platform_value!({
                "documentsMutable": false,
                "propertyConstraints": { "rule": { "lessThan": ["$revision", 3] } }
            }),
            format!(
                "document type \"post\" propertyConstraints rule \"rule\" reads $revision, but \
                 the type keeps no revision: {no_later_write}"
            ),
        ),
        (
            platform_value!({
                "documentsMutable": false,
                "canBeDeleted": true,
                "deleteConstraints": { "rule": { "lessThan": ["$revision", 3] } }
            }),
            "document type \"post\" deleteConstraints rule \"rule\" reads $revision, but the \
             type keeps no revision"
                .to_string(),
        ),
    ] {
        for full_validation in [true, false] {
            expect_structure_error(parse_listing(doctype.clone(), full_validation), &needle);
        }
    }
}

/// A stored time or height is one the type records, as a written one is.
#[test]
fn should_refuse_a_stored_time_the_type_does_not_record() {
    for full_validation in [true, false] {
        expect_structure_error(
            parse_listing(
                platform_value!({
                    "propertyConstraints": {
                        "rule": { "greaterThan": ["$createdAt", "$old.$transferredAt"] }
                    }
                }),
                full_validation,
            ),
            "rule \"rule\" reads $createdAt, which the document type does not record: list it in \
             required",
        );
        expect_structure_error(
            parse_listing(
                platform_value!({
                    "required": ["price", "fee", "$createdAt"],
                    "propertyConstraints": {
                        "rule": { "greaterThan": ["$createdAt", "$old.$transferredAt"] }
                    }
                }),
                full_validation,
            ),
            "rule \"rule\" reads $old.$transferredAt, which the document type does not record: \
             list $transferredAt in required",
        );
    }
}

/// An indexOnly type keeps no stored documents to replace.
#[test]
fn should_refuse_a_stored_read_on_an_index_only_type() {
    // A stored property, and the stored owner, which is told it is the stored
    // version rather than the owner the rule never names
    let stored_reads = [
        platform_value!({ "anyOf": [{ "absent": "$old.hashtag" }, { "present": "hashtag" }] }),
        platform_value!({
            "anyOf": [
                { "notEqual": ["$old.$ownerId", { "const": "CXH2kZCATjvDTnQAPVg28EgPg9WySUvwvnR5ZkmNqY5i" }] },
                { "present": "hashtag" }
            ]
        }),
    ];
    for rule in stored_reads {
        let schema = platform_value!({
            "type": "object",
            "indexOnly": true,
            "documentsMutable": false,
            "properties": {
                "hashtag": { "type": "string", "maxLength": 63, "position": 0 }
            },
            "required": ["hashtag"],
            "indices": [
                { "name": "byHashtag", "properties": [{ "hashtag": "asc" }] }
            ],
            "additionalProperties": false,
            "propertyConstraints": { "rule": rule }
        });
        for full_validation in [true, false] {
            expect_structure_error(
                parse_dispatched(schema.clone(), PlatformVersion::latest(), full_validation),
                "rule \"rule\" reads the stored version through $old., which an indexOnly type \
                 does not keep: its documents are never replaced",
            );
        }
    }
}

fn listing(price: u64, fee: u64) -> Value {
    platform_value!({ "price": price, "fee": fee })
}

fn stored_listing(price: u64, fee: u64, revision: u64) -> StoredVersion {
    StoredVersion::Present(Box::new(StoredDocument {
        properties: Some(
            [
                ("price".to_string(), Value::U64(price)),
                ("fee".to_string(), Value::U64(fee)),
            ]
            .into(),
        ),
        system: StoredSystemValues {
            revision: Some(revision),
            updated_at: Some(0),
            ..Default::default()
        },
    }))
}

/// The parsed type judges a rule reading `$old.` with the stored version the
/// write gives, skips it for a client not giving one, and reports consensus
/// not giving one as an error. A moderator's change of fields is judged only
/// against the rules reading what it changes.
#[test]
fn should_judge_stored_reads_through_the_parsed_type() {
    let platform_version = PlatformVersion::latest();
    let document_type = parse_listing(
        platform_value!({
            "propertyConstraints": {
                "priceStrictlyDrops": { "lessThan": ["price", "$old.price"] },
                "feeCapped": { "lessThanOrEqual": ["fee", 100] },
                "atMostFiveEdits": { "lessThanOrEqual": ["$revision", 6] }
            }
        }),
        true,
    )
    .expect("the rules register");
    let judged = |data: &Value, system: &DocumentSystemValues| {
        document_type
            .validate_property_constraints(data, system, platform_version)
            .expect("the rules are judged")
    };
    let broken_rule = |data: &Value, system: &DocumentSystemValues| {
        judged(data, system)
            .errors
            .first()
            .map(|error| error.to_string())
    };

    // A replace: the price must drop
    let replace = |stored: StoredVersion, revision: u64| DocumentSystemValues {
        revision: Some(revision),
        stored: Some(stored),
        ..Default::default()
    };
    assert!(judged(&listing(90, 10), &replace(stored_listing(100, 10, 1), 2)).is_valid());
    assert!(
        broken_rule(&listing(100, 10), &replace(stored_listing(100, 10, 1), 2))
            .is_some_and(|message| message.contains("priceStrictlyDrops")),
        "an unchanged price breaks the rule"
    );
    // A create: an absent stored price reads 0, which no price is below
    let create = DocumentSystemValues {
        revision: Some(1),
        stored: Some(StoredVersion::Absent),
        ..Default::default()
    };
    assert!(broken_rule(&listing(90, 10), &create)
        .is_some_and(|message| message.contains("priceStrictlyDrops")));

    // A client not given the stored version does not judge the rule
    let client = DocumentSystemValues {
        revision: Some(2),
        ..Default::default()
    };
    assert!(judged(&listing(100, 10), &client).is_valid());

    // Consensus not giving it is a fault in the code building the write
    let consensus = DocumentSystemValues {
        revision: Some(2),
        aggregates: Some(Default::default()),
        consensus: true,
        ..Default::default()
    };
    assert!(matches!(
        document_type.validate_property_constraints(&listing(100, 10), &consensus, platform_version),
        Err(ProtocolError::CorruptedCodeExecution(message))
            if message.contains("rule priceStrictlyDrops of document type post reads the stored version through $old., which consensus did not give")
    ));
    // Nor is giving the stored version without the properties a rule reads
    let without_properties = DocumentSystemValues {
        stored: Some(StoredVersion::without_properties(StoredSystemValues {
            revision: Some(1),
            ..Default::default()
        })),
        ..consensus.clone()
    };
    assert!(matches!(
        document_type.validate_property_constraints(&listing(100, 10), &without_properties, platform_version),
        Err(ProtocolError::CorruptedCodeExecution(message))
            if message.contains("rule priceStrictlyDrops of document type post reads a property of the stored version through $old., which consensus did not give")
    ));

    // A moderator's change of `fee`: the price rule, which reads only what the
    // change leaves alone, is not judged, the fee rule and `$revision` are
    let moderator_change = |fields: &[&str], revision: u64| DocumentSystemValues {
        revision: Some(revision),
        stored: Some(stored_listing(100, 10, revision - 1)),
        changed_fields: Some(fields.iter().map(|field| field.to_string()).collect()),
        ..Default::default()
    };
    assert!(judged(&listing(100, 20), &moderator_change(&["fee"], 2)).is_valid());
    assert!(
        broken_rule(&listing(100, 200), &moderator_change(&["fee"], 2))
            .is_some_and(|message| message.contains("feeCapped"))
    );
    assert!(
        broken_rule(&listing(100, 20), &moderator_change(&["fee"], 7))
            .is_some_and(|message| message.contains("atMostFiveEdits"))
    );
    assert!(
        broken_rule(&listing(100, 20), &moderator_change(&["price"], 2))
            .is_some_and(|message| message.contains("priceStrictlyDrops"))
    );
}
