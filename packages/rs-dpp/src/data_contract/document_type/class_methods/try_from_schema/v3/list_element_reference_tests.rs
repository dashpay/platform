//! References to an element of a list of a referenced document (`refersTo:
//! listElement`, protocol version 14): the parse of the declaration, the checks
//! of its `$id` pair under full validation, the check of its list at contract
//! level for a document type of the same contract, the protocol version gate,
//! the reference bound and the platform serialization round trip.

use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
use crate::data_contract::document_type::accessors::{
    DocumentTypeV0Getters, DocumentTypeV2Getters,
};
use crate::data_contract::document_type::{
    DocumentPropertyReferenceTarget, DocumentPropertyType, DocumentReferenceDeclaration,
    ListElementReference, PropertyReference,
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

fn identifier_referring_to(position: u32, refers_to: serde_json::Value) -> serde_json::Value {
    let mut property = identifier(position);
    property["refersTo"] = refers_to;
    property
}

/// A typed array of at most `max_items` identifiers, whose elements declare
/// `refers_to` when given.
fn identifier_list(
    position: u32,
    max_items: u32,
    refers_to: Option<serde_json::Value>,
) -> serde_json::Value {
    let mut items = json!({
        "type": "array",
        "byteArray": true,
        "minItems": 32,
        "maxItems": 32,
        "contentMediaType": "application/x.dash.dpp.identifier"
    });
    if let Some(refers_to) = refers_to {
        items["refersTo"] = refers_to;
    }
    json!({ "type": "array", "maxItems": max_items, "items": items, "position": position })
}

/// The declaration of the moderation charters' resignations: the value must be
/// one of the `members` of the elected charter whose id `electedCharterId`
/// holds.
fn members_of_the_charter() -> serde_json::Value {
    list_element(json!({ "electedCharterId": "$id" }), "members")
}

fn list_element(property_agreement: serde_json::Value, in_list: &str) -> serde_json::Value {
    json!({
        "type": "listElement",
        "documentType": "electedCharter",
        "propertyAgreement": property_agreement,
        "inList": in_list
    })
}

/// A contract with a permanent, immutable `electedCharter` type holding the
/// `members` list (and a `tags` list of strings and a `title`), a permanent
/// `joinRequest` type, and a `resignation` type whose `memberId` declares
/// `refers_to`. Its other properties are what a `$id` pair can read:
/// `electedCharterId` refers to the charter by id, `plainCharterId` is a
/// plain identifier, `identityId` refers to an identity, and `note` is a
/// string; `charterTitle` is a string another pair can agree on.
fn charter_contract(refers_to: serde_json::Value) -> serde_json::Value {
    json!({
        "$formatVersion": "1",
        "id": Identifier::from(CONTRACT_ID).to_string(Encoding::Base58),
        "ownerId": Identifier::from([8; 32]).to_string(Encoding::Base58),
        "version": 1,
        "documentSchemas": {
            "electedCharter": {
                "type": "object",
                "canBeDeleted": false,
                "documentsMutable": false,
                "properties": {
                    "submittedCharterId": identifier(0),
                    "members": identifier_list(1, 15, None),
                    "tags": {
                        "type": "array",
                        "maxItems": 4,
                        "items": { "type": "string", "maxLength": 16 },
                        "position": 2
                    },
                    "title": { "type": "string", "maxLength": 63, "position": 3 }
                },
                "required": ["submittedCharterId", "members"],
                "additionalProperties": false
            },
            "joinRequest": {
                "type": "object",
                "canBeDeleted": false,
                "documentsMutable": false,
                "properties": {
                    "message": { "type": "string", "maxLength": 63, "position": 0 }
                },
                "additionalProperties": false
            },
            "resignation": {
                "type": "object",
                "properties": {
                    "electedCharterId": identifier_referring_to(
                        0,
                        json!({ "type": "permanentDocument", "documentType": "electedCharter" })
                    ),
                    "memberId": identifier_referring_to(1, refers_to),
                    "plainCharterId": identifier(2),
                    "identityId": identifier_referring_to(3, json!({ "type": "identity" })),
                    "note": { "type": "string", "maxLength": 63, "position": 4 },
                    "charterTitle": { "type": "string", "maxLength": 63, "position": 5 }
                },
                "required": ["electedCharterId"],
                "additionalProperties": false
            }
        }
    })
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

fn resignation_property_type(contract: &DataContract, property: &str) -> DocumentPropertyType {
    contract
        .document_type_for_name("resignation")
        .expect("the resignation document type")
        .flattened_properties()
        .get(property)
        .expect("the property")
        .property_type
        .clone()
}

fn expected_reference(pairs: &[(&str, &str)], in_list: &str) -> DocumentPropertyReferenceTarget {
    DocumentPropertyReferenceTarget::ListElement(ListElementReference {
        contract_id: None,
        document_type_name: "electedCharter".to_string(),
        property_agreement: pairs
            .iter()
            .map(|(referring, referenced)| (referring.to_string(), referenced.to_string()))
            .collect(),
        in_list: in_list.to_string(),
    })
}

fn assert_refused(result: Result<DataContract, ProtocolError>, fragment: &str) {
    let error = result.expect_err("the contract should be refused");
    assert!(
        error.to_string().contains(fragment),
        "expected {fragment:?} in: {error}"
    );
}

/// `schema` with the `electedCharter` type's `key` set to `value`.
fn with_elected_charter(
    mut schema: serde_json::Value,
    key: &str,
    value: serde_json::Value,
) -> serde_json::Value {
    schema["documentSchemas"]["electedCharter"][key] = value;
    schema
}

#[test]
fn should_parse_a_list_element_reference_on_an_identifier_property() {
    let parsed = contract(charter_contract(members_of_the_charter())).expect("parses");
    assert_eq!(
        resignation_property_type(&parsed, "memberId"),
        DocumentPropertyType::IdentifierWithReference(expected_reference(
            &[("electedCharterId", "$id")],
            "members"
        ))
    );
    let reference = expected_reference(&[("electedCharterId", "$id")], "members");
    let DocumentPropertyReferenceTarget::ListElement(reference) = &reference else {
        unreachable!()
    };
    assert_eq!(reference.document_id_property(), Some("electedCharterId"));
    // A document reference, whose value is not the document's id
    let member_id = resignation_property_type(&parsed, "memberId");
    let declaration = reference_declaration(&member_id);
    assert_eq!(declaration.document_type_name, "electedCharter");
    assert!(declaration.permanent);
    assert_eq!(declaration.in_list, Some("members"));
}

fn reference_declaration(property_type: &DocumentPropertyType) -> DocumentReferenceDeclaration<'_> {
    let DocumentPropertyType::IdentifierWithReference(target) = property_type else {
        panic!("expected a reference");
    };
    assert!(
        target.as_document_reference().is_none(),
        "a list element's value is not a document id"
    );
    target
        .as_any_document_reference()
        .expect("a list element is a document reference")
}

#[test]
fn should_parse_a_list_element_reference_through_a_plain_identifier_and_with_more_pairs() {
    // The `$id` property needs no reference of its own: the list element
    // fetches the document itself
    let parsed = contract(charter_contract(list_element(
        json!({ "plainCharterId": "$id" }),
        "members",
    )))
    .expect("parses");
    assert_eq!(
        resignation_property_type(&parsed, "memberId"),
        DocumentPropertyType::IdentifierWithReference(expected_reference(
            &[("plainCharterId", "$id")],
            "members"
        ))
    );

    // Other pairs are ordinary agreements, checked against the same document
    let parsed = contract(charter_contract(list_element(
        json!({ "electedCharterId": "$id", "charterTitle": "title" }),
        "members",
    )))
    .expect("parses");
    assert_eq!(
        resignation_property_type(&parsed, "memberId"),
        DocumentPropertyType::IdentifierWithReference(expected_reference(
            &[("electedCharterId", "$id"), ("charterTitle", "title")],
            "members"
        ))
    );
}

#[test]
fn should_parse_a_list_element_reference_on_the_elements_of_a_typed_array() {
    let mut schema = charter_contract(json!({ "type": "identity" }));
    schema["documentSchemas"]["resignation"]["properties"]["witnesses"] =
        identifier_list(6, 4, Some(members_of_the_charter()));
    let parsed = contract(schema).expect("parses");

    let witnesses = resignation_property_type(&parsed, "witnesses");
    assert_eq!(
        witnesses.reference(),
        Some(PropertyReference::Elements {
            target: &expected_reference(&[("electedCharterId", "$id")], "members"),
            max_items: 4,
        })
    );
}

/// A list element is an existence check against a document that is never
/// deleted with a list that never changes, so it composes with the other
/// permanent targets in a reference expression, checked as a leaf as it would
/// be alone.
#[test]
fn should_parse_a_list_element_as_a_leaf_of_a_reference_expression() {
    let parsed = contract(charter_contract(json!({
        "anyOf": [
            members_of_the_charter(),
            { "type": "permanentDocument", "documentType": "electedCharter" }
        ]
    })))
    .expect("parses");
    let DocumentPropertyType::IdentifierWithReference(DocumentPropertyReferenceTarget::AnyOf(
        operands,
    )) = resignation_property_type(&parsed, "memberId")
    else {
        panic!("expected an anyOf");
    };
    assert_eq!(
        operands.operands()[0],
        expected_reference(&[("electedCharterId", "$id")], "members")
    );

    // The leaf's own checks still run, naming the leaf
    assert_refused(
        contract(charter_contract(json!({
            "anyOf": [
                list_element(json!({ "electedCharterId": "$id" }), "title"),
                { "type": "permanentDocument", "documentType": "electedCharter" }
            ]
        }))),
        "refersTo anyOf[0] listElement: \"title\" of \"electedCharter\" is not a typed array",
    );
}

/// The moderation charters' owner rule: the writer must be one of the
/// charter's members. A list element is a target an identity can be (an
/// element of a list of identities), so `ownerRefersTo` takes it, alone or as a
/// leaf of an expression, and its `$id` pair is checked as a property's is.
#[test]
fn should_accept_a_list_element_on_the_writer() {
    let mut schema = charter_contract(json!({ "type": "identity" }));
    schema["documentSchemas"]["resignation"]["ownerRefersTo"] = members_of_the_charter();
    let parsed = contract(schema).expect("parses");
    assert_eq!(
        parsed
            .document_type_for_name("resignation")
            .expect("the resignation document type")
            .owner_reference(),
        Some(&expected_reference(
            &[("electedCharterId", "$id")],
            "members"
        ))
    );

    let mut expression = charter_contract(json!({ "type": "identity" }));
    expression["documentSchemas"]["resignation"]["ownerRefersTo"] = json!({
        "anyOf": [members_of_the_charter(), { "type": "identity" }]
    });
    contract(expression).expect("an expression with a list element leaf parses");

    let mut bad_id_property = charter_contract(json!({ "type": "identity" }));
    bad_id_property["documentSchemas"]["resignation"]["ownerRefersTo"] =
        list_element(json!({ "note": "$id" }), "members");
    assert_refused(
        contract(bad_id_property),
        "ownerRefersTo listElement: the $id pair reads \"note\", which is not an identifier property",
    );
}

#[test]
fn should_accept_an_optional_id_property() {
    // Only a value set while the `$id` property is not is refused, when the
    // document is written
    let mut schema = charter_contract(members_of_the_charter());
    schema["documentSchemas"]["resignation"]["required"] = json!([]);
    contract(schema).expect("an optional $id property is accepted");
}

#[test]
fn should_refuse_a_list_element_reference_on_a_non_identifier_property() {
    let mut schema = charter_contract(json!({ "type": "identity" }));
    schema["documentSchemas"]["resignation"]["properties"]["note"]["refersTo"] =
        members_of_the_charter();
    contract(schema.clone()).expect_err("the meta-schema should refuse it");
    assert_refused(
        contract_on(schema, false, PlatformVersion::latest()),
        "refersTo is only allowed on identifier properties",
    );
}

#[test]
fn should_refuse_an_id_pair_reading_a_missing_or_non_identifier_property_or_the_writer() {
    for (id_property, fragment) in [
        (
            "nothing",
            "the $id pair reads \"nothing\", which is not a property of the referring document type",
        ),
        (
            "note",
            "the $id pair reads \"note\", which is not an identifier property",
        ),
    ] {
        let schema = charter_contract(list_element(json!({ id_property: "$id" }), "members"));
        assert_refused(contract(schema.clone()), fragment);
        // Without full validation (a contract read back from state) the
        // referring side is not re-checked
        contract_on(schema, false, PlatformVersion::latest())
            .expect("a contract read back from state is not re-checked");
    }

    // The writer's id is no document's id: refused on every parse
    let schema = charter_contract(list_element(json!({ "$ownerId": "$id" }), "members"));
    for full_validation in [true, false] {
        assert_refused(
            contract_on(schema.clone(), full_validation, PlatformVersion::latest()),
            "listElement refersTo $id pair must read a property of the referring document type",
        );
    }
}

/// The `$id` property needs no reference, but one it carries must name the
/// list's document type by id in the list's contract, or its value could
/// never be the id of the document holding the list.
#[test]
fn should_refuse_an_id_property_whose_reference_names_something_else() {
    let refused = |refers_to: serde_json::Value| {
        let mut schema = charter_contract(list_element(json!({ "otherId": "$id" }), "members"));
        schema["documentSchemas"]["resignation"]["properties"]["otherId"] =
            identifier_referring_to(6, refers_to);
        assert_refused(
            contract(schema),
            "the $id pair reads \"otherId\", whose refersTo is not a reference by id to \
             \"electedCharter\" in the list's contract",
        );
    };
    // An identity, another document type, another contract, a lookup key
    // part and an expression hold values no charter has as its id
    refused(json!({ "type": "identity" }));
    refused(json!({ "type": "permanentDocument", "documentType": "joinRequest" }));
    refused(json!({
        "type": "permanentDocument",
        "contractId": Identifier::from([9; 32]).to_string(Encoding::Base58),
        "documentType": "electedCharter"
    }));
    refused(json!({
        "type": "permanentDocument",
        "documentType": "electedCharter",
        "lookup": { "index": "bySubmittedCharter", "keys": { "submittedCharterId": "." } }
    }));
    refused(json!({ "anyOf": [
        { "type": "permanentDocument", "documentType": "electedCharter" },
        { "type": "identity" }
    ] }));

    // Naming the declaring contract explicitly is the same contract
    let mut own_contract = charter_contract(list_element(json!({ "otherId": "$id" }), "members"));
    own_contract["documentSchemas"]["resignation"]["properties"]["otherId"] =
        identifier_referring_to(
            6,
            json!({
                "type": "permanentDocument",
                "contractId": Identifier::from(CONTRACT_ID).to_string(Encoding::Base58),
                "documentType": "electedCharter"
            }),
        );
    contract(own_contract).expect("the declaring contract named explicitly is the same contract");
}

#[test]
fn should_refuse_an_agreement_without_exactly_one_id_pair() {
    for (agreement, found) in [
        (json!({ "charterTitle": "title" }), 0),
        (
            json!({ "electedCharterId": "$id", "plainCharterId": "$id" }),
            2,
        ),
    ] {
        let schema = charter_contract(list_element(agreement, "members"));
        for full_validation in [true, false] {
            assert_refused(
                contract_on(schema.clone(), full_validation, PlatformVersion::latest()),
                &format!("exactly one pair with $id on the referenced side, naming the property whose value is the id of the document holding the list, found {found}"),
            );
        }
    }
}

#[test]
fn should_refuse_a_transient_id_property_or_one_inside_a_transient_object() {
    let mut schema = charter_contract(members_of_the_charter());
    schema["documentSchemas"]["resignation"]["transient"] = json!(["electedCharterId"]);
    assert_refused(
        contract(schema),
        "the $id pair reads \"electedCharterId\", which is transient",
    );

    // An object listed as transient is never stored, and neither is anything
    // in it, on either side of the declaration
    let seats = json!({
        "type": "object",
        "properties": { "members": identifier_list(0, 15, None) },
        "additionalProperties": false,
        "position": 4
    });
    let meta = json!({
        "type": "object",
        "properties": {
            "charterId": identifier_referring_to(
                0,
                json!({ "type": "permanentDocument", "documentType": "electedCharter" })
            )
        },
        "additionalProperties": false,
        "position": 6
    });
    let with_objects = || {
        let mut schema = charter_contract(list_element(
            json!({ "meta.charterId": "$id" }),
            "seats.members",
        ));
        schema["documentSchemas"]["electedCharter"]["properties"]["seats"] = seats.clone();
        schema["documentSchemas"]["resignation"]["properties"]["meta"] = meta.clone();
        schema
    };
    contract(with_objects()).expect("nested stored paths are accepted");

    let mut transient_id_property = with_objects();
    transient_id_property["documentSchemas"]["resignation"]["transient"] = json!(["meta"]);
    assert_refused(
        contract(transient_id_property),
        "the $id pair reads \"meta.charterId\", which is transient",
    );

    let mut transient_list = with_objects();
    transient_list["documentSchemas"]["electedCharter"]["transient"] = json!(["seats"]);
    assert_refused(
        contract(transient_list),
        "\"seats.members\" of \"electedCharter\" is transient",
    );
}

#[test]
fn should_refuse_a_list_that_is_missing_or_not_a_typed_array_of_identifiers() {
    for (list, fragment) in [
        ("absent", "\"electedCharter\" has no property \"absent\""),
        (
            "title",
            "\"title\" of \"electedCharter\" is not a typed array of identifiers",
        ),
        (
            "tags",
            "\"tags\" of \"electedCharter\" is not a typed array of identifiers",
        ),
        (
            "submittedCharterId",
            "\"submittedCharterId\" of \"electedCharter\" is not a typed array of identifiers",
        ),
    ] {
        let schema = charter_contract(list_element(json!({ "electedCharterId": "$id" }), list));
        assert_refused(contract(schema.clone()), fragment);
        contract_on(schema, false, PlatformVersion::latest())
            .expect("a contract read back from state is not re-checked");
    }
}

#[test]
fn should_refuse_a_list_held_by_a_deletable_or_mutable_document_type() {
    // A document holding the list that could be deleted
    assert_refused(
        contract(with_elected_charter(
            charter_contract(members_of_the_charter()),
            "canBeDeleted",
            json!(true),
        )),
        "documents of \"electedCharter\" can be deleted",
    );

    // A list a replace could change
    let mutable = with_elected_charter(
        charter_contract(members_of_the_charter()),
        "documentsMutable",
        json!(true),
    );
    assert_refused(
        contract(mutable.clone()),
        "property \"memberId\" refersTo listElement: \"members\" of \"electedCharter\" can be \
         changed by a replace",
    );

    // Freezing the list on the mutable type is enough
    contract(with_elected_charter(
        mutable.clone(),
        "immutable",
        json!(["members"]),
    ))
    .expect("an immutable list on a mutable type is accepted");

    // Freezing another property is not
    assert_refused(
        contract(with_elected_charter(mutable, "immutable", json!(["title"]))),
        "\"members\" of \"electedCharter\" can be changed by a replace",
    );
}

#[test]
fn should_leave_a_list_in_another_contract_to_registration() {
    // The list's document type is in another contract, so the parse cannot
    // see it: registration checks it against that contract in state. The `$id`
    // property is a plain identifier, as `electedCharterId` refers to this
    // contract's charter
    let mut schema = charter_contract(list_element(json!({ "plainCharterId": "$id" }), "anything"));
    schema["documentSchemas"]["resignation"]["properties"]["memberId"]["refersTo"]["contractId"] =
        json!(Identifier::from([9; 32]).to_string(Encoding::Base58));
    let parsed = contract(schema).expect("parses");
    let DocumentPropertyType::IdentifierWithReference(
        DocumentPropertyReferenceTarget::ListElement(reference),
    ) = resignation_property_type(&parsed, "memberId")
    else {
        panic!("expected a list element reference");
    };
    assert_eq!(reference.contract_id, Some(Identifier::from([9; 32])));
    assert_eq!(reference.in_list, "anything");
}

#[test]
fn should_refuse_a_list_element_below_protocol_version_14_and_accept_it_at_14() {
    let schema = charter_contract(members_of_the_charter());
    let platform_version_13 = PlatformVersion::get(13).expect("platform version 13 should exist");

    // Meta-schema v2 knows no refersTo, so a registering parse refuses it
    contract_on(schema.clone(), true, platform_version_13)
        .expect_err("protocol version 13 should refuse the declaration");
    // A parse predating refersTo ignores the declaration. It predates typed
    // arrays too, so the lists are left out: no list element can exist there
    let mut without_lists = schema.clone();
    let elected_charter = &mut without_lists["documentSchemas"]["electedCharter"];
    for list in ["members", "tags"] {
        elected_charter["properties"]
            .as_object_mut()
            .expect("the properties")
            .remove(list);
    }
    elected_charter["required"] = json!(["submittedCharterId"]);
    let ignored = contract_on(without_lists, false, platform_version_13)
        .expect("protocol version 13 should parse it as a plain identifier");
    assert_eq!(
        resignation_property_type(&ignored, "memberId"),
        DocumentPropertyType::Identifier
    );

    let accepted = contract_on(schema, true, PlatformVersion::latest()).expect("parses");
    assert_eq!(
        resignation_property_type(&accepted, "memberId"),
        DocumentPropertyType::IdentifierWithReference(expected_reference(
            &[("electedCharterId", "$id")],
            "members"
        ))
    );
}

#[test]
fn should_count_list_elements_against_the_reference_bound() {
    let limit = PlatformVersion::latest()
        .system_limits
        .max_references_per_document;
    // The type's other references: `electedCharterId`, `memberId` and
    // `identityId`
    let others = 3;
    let witnesses_up_to = |max_items: u16| {
        let mut schema = charter_contract(members_of_the_charter());
        schema["documentSchemas"]["resignation"]["properties"]["witnesses"] =
            identifier_list(6, u32::from(max_items), Some(members_of_the_charter()));
        schema
    };

    contract(witnesses_up_to(limit - others)).expect("exactly at the bound parses");
    assert_refused(
        contract(witnesses_up_to(limit - others + 1)),
        &format!("above the maximum of {limit}"),
    );
}

#[test]
fn should_round_trip_a_contract_through_platform_serialization_with_a_list_element() {
    let platform_version = PlatformVersion::latest();
    let mut schema = charter_contract(list_element(
        json!({ "electedCharterId": "$id", "charterTitle": "title" }),
        "members",
    ));
    schema["documentSchemas"]["resignation"]["properties"]["witnesses"] = identifier_list(
        6,
        4,
        Some(list_element(json!({ "plainCharterId": "$id" }), "members")),
    );

    let original = contract(schema).expect("parses");
    let bytes = original
        .serialize_to_bytes_with_platform_version(platform_version)
        .expect("the contract should serialize");
    let recovered = DataContract::versioned_deserialize_untrusted(&bytes, false, platform_version)
        .expect("the contract should deserialize");

    assert_eq!(original, recovered);
    for property in ["memberId", "witnesses"] {
        assert_eq!(
            resignation_property_type(&original, property),
            resignation_property_type(&recovered, property)
        );
    }
    assert_eq!(
        resignation_property_type(&recovered, "memberId"),
        DocumentPropertyType::IdentifierWithReference(expected_reference(
            &[("electedCharterId", "$id"), ("charterTitle", "title")],
            "members"
        ))
    );
}

#[test]
fn should_refuse_malformed_list_element_declarations_in_the_parser() {
    for (refers_to, fragment) in [
        (
            json!({ "type": "listElement", "documentType": "electedCharter", "inList": "members" }),
            "exactly one pair with $id on the referenced side",
        ),
        (
            json!({ "type": "listElement", "documentType": "electedCharter", "propertyAgreement": { "electedCharterId": "$id" } }),
            "inList",
        ),
        (
            json!({ "type": "listElement", "documentType": "electedCharter", "propertyAgreement": { "electedCharterId": "$id" }, "inList": "$members" }),
            "listElement refersTo inList must be a property path",
        ),
        (
            json!({ "type": "listElement", "documentType": "electedCharter", "propertyAgreement": { "electedCharterId": "$id" }, "inList": "members", "lookup": { "index": "x", "keys": { "a": "." } } }),
            "listElement refersTo does not take lookup",
        ),
        (
            json!({ "type": "identity", "inList": "members" }),
            "identity refersTo does not take inList",
        ),
        (
            json!({ "type": "permanentDocument", "documentType": "electedCharter", "inList": "members" }),
            "permanentDocument refersTo does not take inList",
        ),
        (
            json!({ "type": "identity", "propertyAgreement": { "electedCharterId": "$id" } }),
            "propertyAgreement is only allowed on permanentDocument, deletableDocument and listElement",
        ),
    ] {
        let schema = charter_contract(refers_to.clone());
        assert_refused(
            contract_on(schema.clone(), false, PlatformVersion::latest()),
            fragment,
        );
        contract(schema).expect_err("the meta-schema should refuse it too");
    }
}

/// The declaration's own rules, apart from any contract.
#[test]
fn should_find_a_value_only_in_the_list_the_referenced_document_holds() {
    let reference = ListElementReference {
        contract_id: None,
        document_type_name: "electedCharter".to_string(),
        property_agreement: BTreeMap::from([("meta.charterId".to_string(), "$id".to_string())]),
        in_list: "seats.members".to_string(),
    };
    let listed = [1u8; 32];
    let unlisted = [2u8; 32];
    let properties = platform_value::platform_value!({
        "seats": { "members": [Identifier::from(listed), Identifier::from([3u8; 32])] }
    })
    .into_btree_string_map()
    .expect("a map");

    let listed_values = reference.listed_values(&properties);
    assert!(listed_values.contains(&listed));
    assert!(!listed_values.contains(&unlisted));
    assert_eq!(listed_values.len(), 2);
    // An absent list holds nothing
    assert!(reference.listed_values(&Default::default()).is_empty());
    assert!(reference
        .listed_values(
            &platform_value::platform_value!({ "seats": {} })
                .into_btree_string_map()
                .expect("a map")
        )
        .is_empty());
    assert_eq!(reference.document_id_property(), Some("meta.charterId"));
    assert_eq!(
        DocumentPropertyReferenceTarget::ListElement(reference).to_string(),
        "list element (seats.members of the electedCharter document meta.charterId names)"
    );
}
