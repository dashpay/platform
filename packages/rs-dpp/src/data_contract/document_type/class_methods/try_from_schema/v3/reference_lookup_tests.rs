//! Document references found by `findBy` through a unique index (protocol
//! version 14): the parse of the declaration, the checks of its
//! referring side on every parse, the checks of its referenced side at contract
//! level for a document type of the same contract, the protocol version gate and
//! the platform serialization round trip.

use super::reference_test_helpers::{
    assert_refused, contract, contract_on, identifier, join_request_schema, CONTRACT_ID,
};
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::config::DataContractConfig;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::{
    DocumentPropertyReferenceTarget, DocumentPropertyType, DocumentReferenceLookup, DocumentType,
    LookupKeySource, PropertyReference,
};
use crate::data_contract::DataContract;
use crate::serialization::{
    PlatformDeserializableWithPotentialValidationFromVersionedStructureUntrusted,
    PlatformSerializableWithPlatformVersion,
};
use platform_value::string_encoding::Encoding;
use platform_value::{Identifier, Value};
use platform_version::version::PlatformVersion;
use serde_json::json;
use std::collections::BTreeMap;

/// The `findBy` of the moderation charter's `members`: the member is the owner
/// of a `joinRequest` for the same submitted charter, found through its unique
/// index over (`submittedCharterId`, `$ownerId`).
fn members_find_by() -> serde_json::Value {
    json!({ "submittedCharterId": "submittedCharterId", "$ownerId": "." })
}

/// A contract with a permanent, immutable `joinRequest` type, unique on
/// (`submittedCharterId`, `$ownerId`) and with a non-unique `byMessage` index,
/// and an `electedCharter` type whose `memberId` declares `refers_to`, next to
/// a required `submittedCharterId`, an optional `alternateCharterId` and a
/// required string `title`.
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
            "electedCharter": {
                "type": "object",
                "properties": {
                    "submittedCharterId": identifier(0),
                    "memberId": member_id,
                    "alternateCharterId": identifier(2),
                    "title": { "type": "string", "maxLength": 63, "position": 3 }
                },
                "required": ["submittedCharterId", "title"],
                "additionalProperties": false
            }
        }
    })
}

fn permanent_join_request(find_by: serde_json::Value) -> serde_json::Value {
    json!({ "type": "permanentDocument", "documentType": "joinRequest", "findBy": find_by })
}

fn member_id_type(contract: &DataContract) -> DocumentPropertyType {
    contract
        .document_type_for_name("electedCharter")
        .expect("the electedCharter document type")
        .flattened_properties()
        .get("memberId")
        .expect("the memberId property")
        .property_type
        .clone()
}

fn expected_lookup(keys: &[(&str, LookupKeySource)]) -> DocumentReferenceLookup {
    DocumentReferenceLookup {
        keys: keys
            .iter()
            .map(|(index_property, source)| (index_property.to_string(), source.clone()))
            .collect(),
        minimum_age_blocks: None,
        consume: false,
    }
}

#[test]
fn should_parse_find_by_with_a_property_source_an_owner_source_and_the_reference_value() {
    let parsed =
        contract(charter_contract(permanent_join_request(members_find_by()))).expect("parses");
    assert_eq!(
        member_id_type(&parsed),
        DocumentPropertyType::IdentifierWithReference(
            DocumentPropertyReferenceTarget::PermanentDocumentLookup {
                contract_id: None,
                document_type_name: "joinRequest".to_string(),
                property_agreement: BTreeMap::new(),
                lookup: expected_lookup(&[
                    (
                        "submittedCharterId",
                        LookupKeySource::Property("submittedCharterId".to_string())
                    ),
                    ("$ownerId", LookupKeySource::ReferenceValue),
                ]),
            }
        )
    );

    // The writer can fill a key part too, and the reference's own value can fill
    // a schema property of the index
    let parsed = contract(charter_contract(permanent_join_request(json!({
        "submittedCharterId": ".",
        "$ownerId": "$ownerId"
    }))))
    .expect("parses");
    let DocumentPropertyType::IdentifierWithReference(
        DocumentPropertyReferenceTarget::PermanentDocumentLookup { lookup, .. },
    ) = member_id_type(&parsed)
    else {
        panic!("expected a permanentDocument lookup reference");
    };
    assert_eq!(
        lookup,
        expected_lookup(&[
            ("submittedCharterId", LookupKeySource::ReferenceValue),
            ("$ownerId", LookupKeySource::OwnerId),
        ])
    );
}

#[test]
fn should_parse_find_by_beside_where_and_resolve_its_unique_index() {
    let parsed = contract(charter_contract(json!({
        "type": "permanentDocument",
        "documentType": "joinRequest",
        "findBy": members_find_by(),
        "where": { "message": "title" }
    })))
    .expect("parses");
    let DocumentPropertyType::IdentifierWithReference(
        DocumentPropertyReferenceTarget::PermanentDocumentLookup {
            property_agreement,
            lookup,
            ..
        },
    ) = member_id_type(&parsed)
    else {
        panic!("expected a permanentDocument lookup reference");
    };
    // The parsed model keys the comparison by the referring property
    assert_eq!(
        property_agreement,
        BTreeMap::from([("title".to_string(), "message".to_string())])
    );
    // The index is the unique one over exactly the properties findBy names
    let join_request = parsed
        .document_type_for_name("joinRequest")
        .expect("the joinRequest document type");
    assert_eq!(
        lookup.resolve_index(join_request),
        Ok("bySubmittedCharter".to_string())
    );
}

#[test]
fn should_refuse_find_by_on_any_reference_but_a_document_one() {
    for reference_type in ["identity", "contract", "token", "identityPublicKey"] {
        let mut refers_to = json!({ "type": reference_type, "findBy": members_find_by() });
        if reference_type == "identityPublicKey" {
            refers_to["keyIdProperty"] = json!("title");
        }
        let schema = charter_contract(refers_to);

        // The meta-schema refuses it under full validation, and the parser on
        // its own without it
        contract(schema.clone()).expect_err("the meta-schema should refuse it");
        assert_refused(
            contract_on(schema, false, PlatformVersion::latest()),
            &format!("{reference_type} refersTo does not take findBy"),
        );
    }
}

#[test]
fn should_refuse_find_by_naming_no_unique_index_exactly() {
    for (find_by, fragment) in [
        // Fewer properties than the unique index, or more: no unique index is
        // over exactly them, and the error lists the ones there are
        (
            json!({ "$ownerId": "." }),
            "\"joinRequest\" has no unique index over exactly ($ownerId): findBy must name \
             every property of one of its unique indexes and nothing else (bySubmittedCharter \
             (submittedCharterId, $ownerId))",
        ),
        (
            json!({ "submittedCharterId": "submittedCharterId", "$ownerId": ".", "message": "title" }),
            "has no unique index over exactly ($ownerId, message, submittedCharterId)",
        ),
        // Exactly the properties of an index that is not unique
        (
            json!({ "message": "." }),
            "index \"byMessage\" of \"joinRequest\" over (message) is not unique",
        ),
    ] {
        assert_refused(
            contract(charter_contract(permanent_join_request(find_by))),
            fragment,
        );
    }
}

#[test]
fn should_refuse_find_by_reading_the_reference_value_twice_or_not_at_all() {
    for (keys, found) in [
        (json!({ "submittedCharterId": ".", "$ownerId": "." }), 2),
        (
            json!({ "submittedCharterId": "submittedCharterId", "$ownerId": "$ownerId" }),
            0,
        ),
    ] {
        let schema = charter_contract(permanent_join_request(keys));
        for full_validation in [true, false] {
            assert_refused(
                contract_on(schema.clone(), full_validation, PlatformVersion::latest()),
                &format!(
                    "findBy must read \".\", the reference's own value, exactly once, found \
                     {found}"
                ),
            );
        }
    }
}

#[test]
fn should_refuse_a_find_by_source_of_the_wrong_value_kind() {
    assert_refused(
        contract(charter_contract(permanent_join_request(json!({
            "submittedCharterId": "title",
            "$ownerId": "."
        })))),
        "findBy \"submittedCharterId\" is filled from \"title\", which holds a different kind \
         of value",
    );
}

#[test]
fn should_refuse_an_optional_transient_or_missing_find_by_source_property() {
    let lookup_reading = |source: &str| {
        permanent_join_request(json!({ "submittedCharterId": source, "$ownerId": "." }))
    };

    // Refused on every parse, validating or not: the rule belongs to the type
    for full_validation in [true, false] {
        assert_refused(
            contract_on(
                charter_contract(lookup_reading("alternateCharterId")),
                full_validation,
                PlatformVersion::latest(),
            ),
            "which is not required",
        );
        assert_refused(
            contract_on(
                charter_contract(lookup_reading("ghostId")),
                full_validation,
                PlatformVersion::latest(),
            ),
            "which is not a property of the referring document type",
        );
        assert_refused(
            contract_on(
                charter_contract(lookup_reading("memberId")),
                full_validation,
                PlatformVersion::latest(),
            ),
            "names the reference property itself",
        );
    }

    let mut transient = charter_contract(lookup_reading("submittedCharterId"));
    transient["documentSchemas"]["electedCharter"]["transient"] = json!(["submittedCharterId"]);
    assert_refused(contract(transient), "which is transient");

    // A required leaf inside an optional object is not always present
    let mut nested = charter_contract(lookup_reading("meta.charterId"));
    nested["documentSchemas"]["electedCharter"]["properties"]["meta"] = json!({
        "type": "object",
        "position": 4,
        "properties": { "charterId": identifier(0) },
        "required": ["charterId"],
        "additionalProperties": false
    });
    assert_refused(contract(nested.clone()), "which is not required");
    nested["documentSchemas"]["electedCharter"]["required"] =
        json!(["submittedCharterId", "title", "meta"]);
    contract(nested).expect("a required leaf of a required object is a valid source");
}

/// `transient` names the object, not the leaf the key reads, and the whole
/// object is stripped before storage, so a reader could never reassemble the
/// key from the stored document.
#[test]
fn should_refuse_a_find_by_source_inside_a_transient_required_object() {
    let mut schema = charter_contract(permanent_join_request(json!({
        "submittedCharterId": "meta.charterId",
        "$ownerId": "."
    })));
    let elected_charter = &mut schema["documentSchemas"]["electedCharter"];
    elected_charter["properties"]["meta"] = json!({
        "type": "object",
        "position": 4,
        "properties": { "charterId": identifier(0) },
        "required": ["charterId"],
        "additionalProperties": false
    });
    elected_charter["required"] = json!(["submittedCharterId", "title", "meta"]);
    contract(schema.clone()).expect("a stored, required leaf of a required object is a source");

    schema["documentSchemas"]["electedCharter"]["transient"] = json!(["meta"]);
    for full_validation in [true, false] {
        assert_refused(
            contract_on(schema.clone(), full_validation, PlatformVersion::latest()),
            "document type \"electedCharter\" property \"memberId\" refersTo findBy: findBy \
             \"submittedCharterId\" reads \"meta.charterId\", which is transient or inside a \
             transient object",
        );
    }
}

#[test]
fn should_refuse_find_by_into_a_document_type_that_can_move_the_key() {
    let mutable = |schema: &mut serde_json::Value| {
        schema["documentSchemas"]["joinRequest"]["documentsMutable"] = json!(true);
    };

    let mut moving = charter_contract(permanent_join_request(members_find_by()));
    mutable(&mut moving);
    assert_refused(
        contract(moving.clone()),
        "findBy names \"submittedCharterId\" of \"joinRequest\", which a replace can change",
    );

    // Freezing the key's schema property is enough
    let mut frozen = moving;
    frozen["documentSchemas"]["joinRequest"]["immutable"] = json!(["submittedCharterId"]);
    contract(frozen).expect("an immutable key property holds the key");
}

/// A key part that is an immutable `deletableDocument` reference by id moves
/// when it is optional, since a replace may clear it once its document is
/// deleted, and holds when it is required, since no replace can drop it.
#[test]
fn should_refuse_find_by_keyed_by_an_optional_deletable_reference_only() {
    let mut required = charter_contract(permanent_join_request(members_find_by()));
    required["documentSchemas"]["charter"] = json!({
        "type": "object",
        "canBeDeleted": true,
        "properties": { "name": { "type": "string", "maxLength": 63, "position": 0 } },
        "additionalProperties": false
    });
    let join_request = &mut required["documentSchemas"]["joinRequest"];
    join_request["documentsMutable"] = json!(true);
    join_request["immutable"] = json!(["submittedCharterId"]);
    join_request["properties"]["submittedCharterId"]["refersTo"] =
        json!({ "type": "deletableDocument", "documentType": "charter" });

    let mut optional = required.clone();
    optional["documentSchemas"]["joinRequest"]["required"] = json!(["message"]);
    assert_refused(
        contract(optional),
        "findBy names \"submittedCharterId\" of \"joinRequest\", which a replace can clear once \
         its document is deleted",
    );

    contract(required).expect("a required reference is never cleared, so the key holds");
}

#[test]
fn should_refuse_find_by_reading_the_writer_on_a_type_that_can_change_owner() {
    let reads_the_writer = charter_contract(permanent_join_request(json!({
        "submittedCharterId": ".",
        "$ownerId": "$ownerId"
    })));
    for (keyword, value) in [("transferable", json!(1)), ("tradeMode", json!(1))] {
        let mut changes_owner = reads_the_writer.clone();
        changes_owner["documentSchemas"]["electedCharter"][keyword] = value;
        assert_refused(
            contract(changes_owner),
            "findBy \"$ownerId\" reads \"$ownerId\", which a transfer or a purchase of the \
             referring document changes",
        );
    }

    // A transferable type may still refer by a findBy that does not read the
    // writer: its value and properties change only with a replace
    let mut transferable = charter_contract(permanent_join_request(members_find_by()));
    transferable["documentSchemas"]["electedCharter"]["transferable"] = json!(1);
    contract(transferable).expect("a key that does not read the writer holds");
}

/// A `deletableDocument` reference may find its document by `findBy` too: a key into a
/// deletable type may find a later document once the one it found is deleted, so the reference
/// means a document with this key exists now, and every replace re-validates it. The
/// referenced side is checked as it is for a permanent one.
#[test]
fn should_parse_find_by_on_a_deletable_document_reference_and_check_its_referenced_side() {
    let deletable_join_request = json!({
        "type": "deletableDocument",
        "documentType": "joinRequest",
        "findBy": members_find_by()
    });
    let mut schema = charter_contract(deletable_join_request);
    refers_to_deletable_join_requests(&mut schema);

    let parsed = contract(schema.clone()).expect("parses");
    assert_eq!(
        member_id_type(&parsed),
        DocumentPropertyType::IdentifierWithReference(
            DocumentPropertyReferenceTarget::DeletableDocumentLookup {
                contract_id: None,
                document_type_name: "joinRequest".to_string(),
                property_agreement: BTreeMap::new(),
                lookup: expected_lookup(&[
                    (
                        "submittedCharterId",
                        LookupKeySource::Property("submittedCharterId".to_string()),
                    ),
                    ("$ownerId", LookupKeySource::ReferenceValue),
                ]),
            }
        )
    );

    let mut moving = schema.clone();
    moving["documentSchemas"]["joinRequest"]["documentsMutable"] = json!(true);
    assert_refused(
        contract(moving),
        "findBy names \"submittedCharterId\" of \"joinRequest\", which a replace can change",
    );

    // Once its document is gone the property would have to change to pass again
    let mut immutable = schema;
    immutable["documentSchemas"]["electedCharter"]["immutable"] = json!(["memberId"]);
    assert_refused(
        contract(immutable),
        "\"memberId\" is a deletableDocument reference found by findBy",
    );
}

/// Makes the fixture's `joinRequest` deletable, the target a
/// `deletableDocument` reference needs.
fn refers_to_deletable_join_requests(schema: &mut serde_json::Value) {
    schema["documentSchemas"]["joinRequest"]["canBeDeleted"] = json!(true);
}

/// The moderation charter's `members` (#4898), verbatim: a typed array whose
/// elements must each be the owner of a join request for the charter.
fn members(refers_to: serde_json::Value) -> serde_json::Value {
    let mut items = identifier(0);
    items.as_object_mut().expect("an object").remove("position");
    items["distinctFrom"] = json!("$ownerId");
    items["refersTo"] = refers_to;
    json!({
        "type": "array", "minItems": 0, "maxItems": 15, "uniqueItems": true,
        "items": items,
        "position": 4
    })
}

fn with_members(refers_to: serde_json::Value) -> serde_json::Value {
    let mut schema = charter_contract(json!({ "type": "identity" }));
    schema["documentSchemas"]["electedCharter"]["properties"]["members"] = members(refers_to);
    schema
}

#[test]
fn should_parse_find_by_on_the_elements_of_a_typed_array_as_the_charter_declares() {
    let parsed = contract(with_members(permanent_join_request(members_find_by()))).expect("parses");
    let members = parsed
        .document_type_for_name("electedCharter")
        .expect("the electedCharter document type")
        .flattened_properties()
        .get("members")
        .expect("the members property")
        .property_type
        .clone();
    assert_eq!(
        members.reference(),
        Some(PropertyReference::Elements {
            target: &DocumentPropertyReferenceTarget::PermanentDocumentLookup {
                contract_id: None,
                document_type_name: "joinRequest".to_string(),
                property_agreement: BTreeMap::new(),
                lookup: expected_lookup(&[
                    (
                        "submittedCharterId",
                        LookupKeySource::Property("submittedCharterId".to_string())
                    ),
                    ("$ownerId", LookupKeySource::ReferenceValue),
                ]),
            },
            max_items: 15,
        })
    );
}

#[test]
fn should_check_an_element_find_by_as_a_single_one_is_checked() {
    // The referring side, on every parse
    assert_refused(
        contract_on(
            with_members(permanent_join_request(json!({
                "submittedCharterId": "alternateCharterId",
                "$ownerId": "."
            }))),
            false,
            PlatformVersion::latest(),
        ),
        "document type \"electedCharter\" property \"members\" refersTo findBy: findBy \
         \"submittedCharterId\" reads \"alternateCharterId\", which is not required",
    );
    // The referenced side, at contract level
    assert_refused(
        contract(with_members(permanent_join_request(
            json!({ "message": "." }),
        ))),
        "index \"byMessage\" of \"joinRequest\" over (message) is not unique",
    );
    // And on a deletableDocument reference, whose elements every replace re-validates
    let mut deletable = with_members(json!({
        "type": "deletableDocument",
        "documentType": "joinRequest",
        "findBy": members_find_by()
    }));
    refers_to_deletable_join_requests(&mut deletable);
    let parsed = contract(deletable).expect("parses");
    assert!(matches!(
        parsed
            .document_type_for_name("electedCharter")
            .expect("the electedCharter document type")
            .flattened_properties()
            .get("members")
            .expect("the members property")
            .property_type
            .reference(),
        Some(PropertyReference::Elements {
            target: DocumentPropertyReferenceTarget::DeletableDocumentLookup { .. },
            ..
        })
    ));
}

#[test]
fn should_refuse_find_by_below_protocol_version_14_and_accept_it_at_14() {
    let schema = charter_contract(permanent_join_request(members_find_by()));
    let platform_version_13 = PlatformVersion::get(13).expect("platform version 13 should exist");

    // Meta-schema v2 knows no refersTo, so a registering parse refuses it
    contract_on(schema.clone(), true, platform_version_13)
        .expect_err("protocol version 13 should refuse the declaration");
    // A parse predating refersTo ignores the whole declaration, findBy and all
    let ignored = contract_on(schema.clone(), false, platform_version_13)
        .expect("protocol version 13 should parse it as a plain identifier");
    assert_eq!(member_id_type(&ignored), DocumentPropertyType::Identifier);

    let accepted = contract_on(schema, true, PlatformVersion::latest()).expect("parses");
    assert!(matches!(
        member_id_type(&accepted),
        DocumentPropertyType::IdentifierWithReference(
            DocumentPropertyReferenceTarget::PermanentDocumentLookup { .. }
        )
    ));
}

#[test]
fn should_leave_find_by_into_another_contract_to_registration() {
    // The referenced type is not in this contract, so the parse cannot see its
    // indexes: registration checks them against the other contract in state
    let parsed = contract(charter_contract(json!({
        "type": "permanentDocument",
        "contractId": Identifier::from([9; 32]).to_string(Encoding::Base58),
        "documentType": "joinRequest",
        "findBy": { "anything": "." }
    })))
    .expect("parses");
    assert!(matches!(
        member_id_type(&parsed),
        DocumentPropertyType::IdentifierWithReference(
            DocumentPropertyReferenceTarget::PermanentDocumentLookup {
                contract_id: Some(_),
                ..
            }
        )
    ));
}

#[test]
fn should_round_trip_a_contract_through_platform_serialization_with_and_without_find_by() {
    let platform_version = PlatformVersion::latest();

    for refers_to in [
        json!({ "type": "permanentDocument", "documentType": "joinRequest" }),
        permanent_join_request(members_find_by()),
    ] {
        let original = contract(charter_contract(refers_to.clone())).expect("parses");
        let bytes = original
            .serialize_to_bytes_with_platform_version(platform_version)
            .expect("the contract should serialize");
        let recovered =
            DataContract::versioned_deserialize_untrusted(&bytes, false, platform_version)
                .expect("the contract should deserialize");

        assert_eq!(original, recovered, "refersTo {refers_to}");
        assert_eq!(member_id_type(&original), member_id_type(&recovered));
    }

    // Without findBy, the parsed reference is exactly the id reference it was
    let without = contract(charter_contract(
        json!({ "type": "permanentDocument", "documentType": "joinRequest" }),
    ))
    .expect("parses");
    assert_eq!(
        member_id_type(&without),
        DocumentPropertyType::IdentifierWithReference(
            DocumentPropertyReferenceTarget::PermanentDocument {
                contract_id: None,
                document_type_name: "joinRequest".to_string(),
                property_agreement: BTreeMap::new(),
            }
        )
    );
}

#[test]
fn should_refuse_malformed_find_by_declarations_in_the_parser() {
    for (find_by, fragment) in [
        (json!({}), "findBy must name between 1 and 10 properties"),
        (
            json!({ "$ownerId": 1 }),
            "findBy maps each property to \".\", \"$ownerId\", a property path",
        ),
        (
            json!({ "submittedCharterId": "$createdAt", "$ownerId": "." }),
            "not system property \"$createdAt\"",
        ),
        // The value being the document's id is what leaving findBy out says
        (json!({ "$id": "." }), "findBy $id names the document by id"),
    ] {
        let schema = charter_contract(permanent_join_request(find_by.clone()));
        assert_refused(
            contract_on(schema.clone(), false, PlatformVersion::latest()),
            fragment,
        );
        contract(schema).expect_err("the meta-schema should refuse it too");
    }
}

/// A protocol version 14 beta spelled these references with `lookup`,
/// `propertyAgreement` and the `listElement` type. Every parse refuses them,
/// validating or not, so a contract written with them never loads with another
/// meaning, and the error names what replaced each.
#[test]
fn should_refuse_the_keywords_find_by_and_where_replaced() {
    for (refers_to, fragment) in [
        (
            json!({
                "type": "permanentDocument",
                "documentType": "joinRequest",
                "lookup": {
                    "index": "bySubmittedCharter",
                    "keys": { "submittedCharterId": "submittedCharterId", "$ownerId": "." }
                }
            }),
            "refersTo lookup was replaced by findBy",
        ),
        (
            json!({
                "type": "permanentDocument",
                "documentType": "joinRequest",
                "propertyAgreement": { "title": "message" }
            }),
            "refersTo propertyAgreement was replaced by where, keyed by the referenced \
             document's property",
        ),
        (
            json!({
                "type": "listElement",
                "documentType": "joinRequest",
                "propertyAgreement": { "submittedCharterId": "$id" },
                "inList": "members"
            }),
            "refersTo type listElement was replaced",
        ),
    ] {
        let schema = charter_contract(refers_to);
        assert_refused(
            contract_on(schema.clone(), false, PlatformVersion::latest()),
            fragment,
        );
        contract(schema).expect_err("the meta-schema should refuse it too");
    }
}

/// The document type parse alone, without the rest of the contract, as a
/// client building one document type sees it.
#[test]
fn should_check_only_the_referring_side_when_a_document_type_is_parsed_alone() {
    let platform_version = PlatformVersion::latest();
    let config =
        DataContractConfig::default_for_version(platform_version).expect("config should build");
    let schema = charter_contract(permanent_join_request(json!({ "anything": "." })));
    let elected_charter: Value =
        platform_value::to_value(schema["documentSchemas"]["electedCharter"].clone())
            .expect("the schema should convert");
    DocumentType::try_from_schema(
        Identifier::from(CONTRACT_ID),
        1,
        config.version(),
        "electedCharter",
        elected_charter,
        None,
        &BTreeMap::new(),
        &config,
        true,
        &mut vec![],
        platform_version,
    )
    .expect("the referenced type is out of sight, so only the sources are checked");
}
