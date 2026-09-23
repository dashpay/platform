//! `anyOf` reference targets (`refersTo: { "anyOf": [target, ...] }`, protocol
//! version 14): the parse of the declaration on an identifier property and on
//! the elements of a typed array, the targets it refuses, the registration
//! limits it counts against, the checks each target gets as it would alone,
//! the protocol version gate and the platform serialization round trip.

use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::{
    AnyOfReferenceTargets, DocumentPropertyReferenceTarget, DocumentPropertyType,
    DocumentReferenceLookup, LookupKeySource, PropertyReference,
};
use crate::data_contract::DataContract;
use crate::serialization::{
    PlatformDeserializableWithPotentialValidationFromVersionedStructureUntrusted,
    PlatformSerializableWithPlatformVersion,
};
use crate::ProtocolError;
use platform_value::string_encoding::Encoding;
use platform_value::Identifier;
use platform_version::version::PlatformVersion;
use serde_json::json;
use std::collections::BTreeMap;

const CONTRACT_ID: [u8; 32] = [7; 32];

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

fn any_of(targets: Vec<serde_json::Value>) -> serde_json::Value {
    json!({ "anyOf": targets })
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
            "joinRequest": {
                "type": "object",
                "canBeDeleted": false,
                "documentsMutable": false,
                "properties": {
                    "submittedCharterId": identifier(0),
                    "message": { "type": "string", "maxLength": 63, "position": 1 }
                },
                "indices": [
                    {
                        "name": "bySubmittedCharter",
                        "properties": [{ "submittedCharterId": "asc" }, { "$ownerId": "asc" }],
                        "unique": true
                    },
                    { "name": "byMessage", "properties": [{ "message": "asc" }] }
                ],
                "required": ["submittedCharterId", "message"],
                "additionalProperties": false
            },
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

fn contract_on(
    contract: serde_json::Value,
    full_validation: bool,
    platform_version: &PlatformVersion,
) -> Result<DataContract, ProtocolError> {
    let value = platform_value::to_value(contract).expect("the contract should convert");
    DataContract::from_value(value, full_validation, platform_version)
}

fn contract(contract: serde_json::Value) -> Result<DataContract, ProtocolError> {
    contract_on(contract, true, PlatformVersion::latest())
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

fn assert_refused(result: Result<DataContract, ProtocolError>, fragment: &str) {
    let error = result.expect_err("the contract should be refused");
    assert!(
        error.to_string().contains(fragment),
        "expected {fragment:?} in: {error}"
    );
}

/// Refused by the parser on every parse, and by the meta-schema (or the
/// parser behind it) when the contract registers.
fn assert_refused_by_parser_and_registration(schema: serde_json::Value, fragment: &str) {
    assert_refused(
        contract_on(schema.clone(), false, PlatformVersion::latest()),
        fragment,
    );
    contract(schema).expect_err("registration should refuse it too");
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

#[test]
fn should_parse_an_any_of_of_two_lookups_in_declared_order() {
    let parsed = contract(charter_contract(any_of(vec![
        join_request_lookup(),
        added_moderator_lookup(),
    ])))
    .expect("parses");

    assert_eq!(
        property_type(&parsed, "memberId"),
        DocumentPropertyType::IdentifierWithReference(DocumentPropertyReferenceTarget::AnyOf(
            AnyOfReferenceTargets::new(vec![
                expected_join_request_lookup(),
                expected_added_moderator_lookup(),
            ])
        ))
    );

    // The order is the author's: it decides which error a writer is shown
    let reversed = contract(charter_contract(any_of(vec![
        added_moderator_lookup(),
        join_request_lookup(),
    ])))
    .expect("parses");
    assert_eq!(
        property_type(&reversed, "memberId")
            .reference()
            .and_then(|reference| reference.target())
            .map(|target| target.targets().to_vec()),
        Some(vec![
            expected_added_moderator_lookup(),
            expected_join_request_lookup(),
        ])
    );
}

/// Each target is an ordinary declaration with its own keys: an agreement
/// belongs to the target it is declared on.
#[test]
fn should_parse_an_any_of_of_an_identity_and_a_document_with_its_own_agreement() {
    let parsed = contract(charter_contract(any_of(vec![
        json!({ "type": "identity" }),
        json!({
            "type": "permanentDocument",
            "documentType": "joinRequest",
            "propertyAgreement": { "title": "message" }
        }),
    ])))
    .expect("parses");

    assert_eq!(
        property_type(&parsed, "memberId"),
        DocumentPropertyType::IdentifierWithReference(DocumentPropertyReferenceTarget::AnyOf(
            AnyOfReferenceTargets::new(vec![
                DocumentPropertyReferenceTarget::Identity,
                DocumentPropertyReferenceTarget::PermanentDocument {
                    contract_id: None,
                    document_type_name: "joinRequest".to_string(),
                    property_agreement: [("title".to_string(), "message".to_string())].into(),
                },
            ])
        ))
    );
}

#[test]
fn should_parse_an_any_of_on_the_elements_of_a_typed_array() {
    let parsed = contract(charter_contract_with_members(
        any_of(vec![join_request_lookup(), added_moderator_lookup()]),
        15,
    ))
    .expect("parses");

    let members = property_type(&parsed, "members");
    assert_eq!(
        members.reference(),
        Some(PropertyReference::Elements {
            target: &DocumentPropertyReferenceTarget::AnyOf(AnyOfReferenceTargets::new(vec![
                expected_join_request_lookup(),
                expected_added_moderator_lookup(),
            ])),
            max_items: 15,
        })
    );
}

#[test]
fn should_refuse_an_any_of_of_fewer_than_two_targets() {
    assert_refused_by_parser_and_registration(
        charter_contract(any_of(vec![json!({ "type": "identity" })])),
        "refersTo anyOf must list at least two targets",
    );
    assert_refused_by_parser_and_registration(
        charter_contract(any_of(vec![])),
        "refersTo anyOf must list at least two targets",
    );
    assert_refused_by_parser_and_registration(
        charter_contract(json!({ "anyOf": { "type": "identity" } })),
        "refersTo anyOf must be a list of targets",
    );
}

/// The count is a registration limit, `max_any_of_reference_targets`: a
/// stored contract was checked when it was registered.
#[test]
fn should_refuse_an_any_of_above_the_target_limit_under_full_validation() {
    let platform_version = PlatformVersion::latest();
    let limit = platform_version.system_limits.max_any_of_reference_targets;
    assert_eq!(limit, 4);
    let permanent =
        |document_type: &str| json!({ "type": "permanentDocument", "documentType": document_type });
    let at_limit = vec![
        json!({ "type": "identity" }),
        permanent("joinRequest"),
        permanent("addedModerator"),
        join_request_lookup(),
    ];
    let mut above_limit = at_limit.clone();
    above_limit.push(added_moderator_lookup());

    contract(charter_contract(any_of(at_limit))).expect("the maximum registers");
    assert_refused(
        contract(charter_contract(any_of(above_limit.clone()))),
        "property \"memberId\" of document type \"resignation\" declares a refersTo anyOf of 5 \
         targets, above the maximum of 4",
    );
    let stored = contract_on(
        charter_contract(any_of(above_limit)),
        false,
        platform_version,
    )
    .expect("the stored path does not re-apply a registration limit");
    assert!(matches!(
        property_type(&stored, "memberId"),
        DocumentPropertyType::IdentifierWithReference(DocumentPropertyReferenceTarget::AnyOf(
            targets
        )) if targets.targets().len() == 5
    ));
}

#[test]
fn should_refuse_every_target_type_an_any_of_does_not_take() {
    for (refused, fragment) in [
        (
            json!({ "type": "contract" }),
            "refersTo anyOf[1] is a reference of type contract, which anyOf does not take: its \
             requirements are judged against the block time and the writer",
        ),
        (
            json!({ "type": "token" }),
            "refersTo anyOf[1] is a reference of type token, which anyOf does not take",
        ),
        (
            json!({ "type": "deletableDocument", "documentType": "note" }),
            "refersTo anyOf[1] is a reference of type deletableDocument, which anyOf does not \
             take: it is re-validated on every replace",
        ),
        (
            json!({ "type": "identityPublicKey", "keyIdProperty": "keyId" }),
            "refersTo anyOf[1] is a reference of type identityPublicKey, which anyOf does not \
             take: it pairs the value with a key id property",
        ),
        (
            json!({ "type": "identityPublicKey", "identityProperty": "$ownerId" }),
            "refersTo anyOf[1] is a reference of type identityPublicKey, which anyOf does not take",
        ),
    ] {
        assert_refused_by_parser_and_registration(
            charter_contract(any_of(vec![json!({ "type": "identity" }), refused])),
            fragment,
        );
    }

    // The key id form is refused whatever type it claims
    assert_refused_by_parser_and_registration(
        charter_contract(any_of(vec![
            json!({ "type": "identity" }),
            json!({ "type": "identity", "identityProperty": "$ownerId" }),
        ])),
        "refersTo anyOf[1]: identity refersTo does not take identityProperty",
    );
}

#[test]
fn should_refuse_an_any_of_nested_in_an_any_of() {
    assert_refused_by_parser_and_registration(
        charter_contract(any_of(vec![
            json!({ "type": "identity" }),
            any_of(vec![join_request_lookup(), added_moderator_lookup()]),
        ])),
        "refersTo anyOf[1] is itself an anyOf: the targets of an anyOf do not nest",
    );
}

#[test]
fn should_refuse_keys_beside_an_any_of() {
    let mut beside = any_of(vec![json!({ "type": "identity" }), join_request_lookup()]);
    beside["type"] = json!("identity");
    assert_refused_by_parser_and_registration(
        charter_contract(beside),
        "a refersTo anyOf declares nothing beside anyOf",
    );

    let mut agreement_beside = any_of(vec![json!({ "type": "identity" }), join_request_lookup()]);
    agreement_beside["propertyAgreement"] = json!({ "title": "message" });
    assert_refused_by_parser_and_registration(
        charter_contract(agreement_beside),
        "a refersTo anyOf declares nothing beside anyOf",
    );
}

#[test]
fn should_refuse_a_target_repeated_in_an_any_of() {
    assert_refused_by_parser_and_registration(
        charter_contract(any_of(vec![
            join_request_lookup(),
            json!({ "type": "identity" }),
            join_request_lookup(),
        ])),
        "refersTo anyOf[2] repeats an earlier target",
    );
}

#[test]
fn should_refuse_an_any_of_on_a_property_that_is_not_an_identifier() {
    let mut schema = charter_contract(json!({ "type": "identity" }));
    schema["documentSchemas"]["resignation"]["properties"]["keyId"]["refersTo"] =
        any_of(vec![json!({ "type": "identity" }), join_request_lookup()]);
    assert_refused_by_parser_and_registration(
        schema,
        "refersTo anyOf is only allowed on identifier properties",
    );
}

/// Every check a target gets alone, it gets inside an `anyOf`: a malformed
/// target is refused by the parser, the referring side of a lookup on every
/// parse, and its referenced side against a type of the same contract when
/// the contract registers.
#[test]
fn should_check_each_target_of_an_any_of_as_it_would_be_checked_alone() {
    assert_refused_by_parser_and_registration(
        charter_contract(any_of(vec![
            json!({ "type": "identity" }),
            json!({ "type": "permanentDocument" }),
        ])),
        "documentType",
    );

    let mut optional_source = added_moderator_lookup();
    optional_source["lookup"]["keys"]["submittedCharterId"] = json!("alternateCharterId");
    assert_refused(
        contract_on(
            charter_contract(any_of(vec![join_request_lookup(), optional_source])),
            false,
            PlatformVersion::latest(),
        ),
        "document type \"resignation\" property \"memberId\" refersTo lookup: key \
         \"submittedCharterId\" reads \"alternateCharterId\", which is not required",
    );

    let mut not_unique = join_request_lookup();
    not_unique["lookup"] = json!({ "index": "byMessage", "keys": { "message": "." } });
    let not_unique_second = charter_contract(any_of(vec![added_moderator_lookup(), not_unique]));
    assert_refused(
        contract(not_unique_second.clone()),
        "index \"byMessage\" of \"joinRequest\" is not unique",
    );
    // The referenced side needs the whole contract, so a stored parse leaves it
    contract_on(not_unique_second, false, PlatformVersion::latest())
        .expect("a stored contract was checked when it registered");
}

/// Every target may be read for every value when a document is written, so
/// each counts against `max_references_per_document`: a typed array of 128
/// elements holding two targets carries 256 references, one of three 384.
#[test]
fn should_count_every_target_of_an_any_of_against_the_reference_limit() {
    let platform_version = PlatformVersion::latest();
    assert_eq!(
        platform_version.system_limits.max_references_per_document,
        256
    );

    contract(charter_contract_with_members(
        any_of(vec![join_request_lookup(), added_moderator_lookup()]),
        128,
    ))
    .expect("two targets for 128 elements are exactly the maximum");

    assert_refused(
        contract(charter_contract_with_members(
            any_of(vec![
                json!({ "type": "identity" }),
                join_request_lookup(),
                added_moderator_lookup(),
            ]),
            128,
        )),
        "declares references for up to 384 values",
    );

    // A single property's anyOf counts its targets too: 255 more references
    // from the list, and two from memberId
    let mut schema = charter_contract_with_members(json!({ "type": "identity" }), 255);
    schema["documentSchemas"]["resignation"]["properties"]["memberId"]["refersTo"] =
        any_of(vec![json!({ "type": "identity" }), join_request_lookup()]);
    assert_refused(contract(schema), "declares references for up to 257 values");
}

#[test]
fn should_refuse_an_any_of_below_protocol_version_14_and_accept_it_at_14() {
    let schema = charter_contract(any_of(vec![
        join_request_lookup(),
        added_moderator_lookup(),
    ]));
    let platform_version_13 = PlatformVersion::get(13).expect("platform version 13 should exist");

    // Meta-schema v2 knows no refersTo, so a registering parse refuses it
    contract_on(schema.clone(), true, platform_version_13)
        .expect_err("protocol version 13 should refuse the declaration");
    // A parse predating refersTo ignores the whole declaration, anyOf and all
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

/// A contract is serialized as its schemas, so an `anyOf` round trips as
/// the declaration it was registered with, and a contract without one
/// serializes exactly as it did before the form existed.
#[test]
fn should_round_trip_a_contract_with_an_any_of_through_platform_serialization() {
    let platform_version = PlatformVersion::latest();

    for schema in [
        charter_contract(join_request_lookup()),
        charter_contract(any_of(vec![
            json!({ "type": "identity" }),
            join_request_lookup(),
            added_moderator_lookup(),
        ])),
        charter_contract_with_members(
            any_of(vec![join_request_lookup(), added_moderator_lookup()]),
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

    // Without an anyOf, the parsed reference is exactly the lookup it was
    let without = contract(charter_contract(join_request_lookup())).expect("parses");
    assert_eq!(
        property_type(&without, "memberId"),
        DocumentPropertyType::IdentifierWithReference(expected_join_request_lookup())
    );
}
