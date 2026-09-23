//! References to an element of a list of a referenced document (`refersTo:
//! listElement`, protocol version 14): the parse of the declaration, the checks
//! of its referring side and, for a list in the same contract, of the list at
//! contract level, all under full validation, the protocol version gate, the
//! reference bound and the platform serialization round trip.

use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::{
    DocumentPropertyReferenceTarget, DocumentPropertyType, ListElementReference, PropertyReference,
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
/// one of the `members` of the elected charter `electedCharterId` refers to.
fn members_of_the_charter() -> serde_json::Value {
    list_element("electedCharterId", "members")
}

fn list_element(document_property: &str, list: &str) -> serde_json::Value {
    json!({
        "type": "listElement",
        "documentType": "electedCharter",
        "documentProperty": document_property,
        "list": list
    })
}

/// A contract with a permanent, immutable `electedCharter` type holding the
/// `members` list (and a `tags` list of strings, a `title` and a unique
/// `bySubmittedCharter` index a lookup can find a charter through), a
/// permanent `joinRequest` type, and a `resignation` type whose `memberId`
/// declares `refers_to`. Its other properties are what a declaration can name:
/// `electedCharterId` refers to the charter by id, `lookedUpCharterId` through
/// the index, `joinRequestId` to a join request, `identityId` to an identity,
/// and `plainId` and `note` refer to nothing.
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
                "indices": [
                    {
                        "name": "bySubmittedCharter",
                        "properties": [{ "submittedCharterId": "asc" }],
                        "unique": true
                    }
                ],
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
                    "lookedUpCharterId": identifier_referring_to(2, json!({
                        "type": "permanentDocument",
                        "documentType": "electedCharter",
                        "lookup": {
                            "index": "bySubmittedCharter",
                            "keys": { "submittedCharterId": "." }
                        }
                    })),
                    "joinRequestId": identifier_referring_to(
                        3,
                        json!({ "type": "permanentDocument", "documentType": "joinRequest" })
                    ),
                    "identityId": identifier_referring_to(4, json!({ "type": "identity" })),
                    "plainId": identifier(5),
                    "note": { "type": "string", "maxLength": 63, "position": 6 }
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

fn expected_reference(document_property: &str, list: &str) -> DocumentPropertyReferenceTarget {
    DocumentPropertyReferenceTarget::ListElement(ListElementReference {
        document_type_name: "electedCharter".to_string(),
        document_property: document_property.to_string(),
        list: list.to_string(),
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
            "electedCharterId",
            "members"
        ))
    );
}

#[test]
fn should_parse_a_list_element_reference_read_through_a_lookup_reference() {
    // `documentProperty` may find the charter through a unique index rather
    // than by its id
    let parsed = contract(charter_contract(list_element(
        "lookedUpCharterId",
        "members",
    )))
    .expect("parses");
    assert_eq!(
        resignation_property_type(&parsed, "memberId"),
        DocumentPropertyType::IdentifierWithReference(expected_reference(
            "lookedUpCharterId",
            "members"
        ))
    );
}

#[test]
fn should_parse_a_list_element_reference_on_the_elements_of_a_typed_array() {
    let mut schema = charter_contract(json!({ "type": "identity" }));
    schema["documentSchemas"]["resignation"]["properties"]["witnesses"] =
        identifier_list(7, 4, Some(members_of_the_charter()));
    let parsed = contract(schema).expect("parses");

    let witnesses = resignation_property_type(&parsed, "witnesses");
    assert_eq!(
        witnesses.reference(),
        Some(PropertyReference::Elements {
            target: &expected_reference("electedCharterId", "members"),
            max_items: 4,
        })
    );
}

#[test]
fn should_accept_an_optional_document_property() {
    // Only a value set while `documentProperty` is not is refused, when the
    // document is written
    let mut schema = charter_contract(members_of_the_charter());
    schema["documentSchemas"]["resignation"]["required"] = json!([]);
    contract(schema).expect("an optional documentProperty is accepted");
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
fn should_refuse_a_document_property_that_is_missing_or_carries_no_permanent_document_reference() {
    for document_property in ["nothing", "plainId", "identityId", "note"] {
        let schema = charter_contract(list_element(document_property, "members"));
        let fragment = if document_property == "nothing" {
            format!("documentProperty \"{document_property}\" is not a property")
        } else {
            format!(
                "documentProperty \"{document_property}\" must be an identifier property \
                 carrying a permanentDocument refersTo"
            )
        };
        assert_refused(contract(schema.clone()), &fragment);
        // Without full validation (a contract read back from state) the
        // referring side is not re-checked
        contract_on(schema, false, PlatformVersion::latest())
            .expect("a contract read back from state is not re-checked");
    }

    // A deletableDocument reference does not qualify: the document holding the
    // list could be deleted
    let mut schema = charter_contract(members_of_the_charter());
    schema["documentSchemas"]["resignation"]["properties"]["electedCharterId"]["refersTo"]
        ["type"] = json!("deletableDocument");
    schema = with_elected_charter(schema, "canBeDeleted", json!(true));
    assert_refused(
        contract(schema),
        "documentProperty \"electedCharterId\" must be an identifier property carrying a \
         permanentDocument refersTo",
    );
}

#[test]
fn should_refuse_a_document_property_that_refers_to_another_document_type() {
    assert_refused(
        contract(charter_contract(list_element("joinRequestId", "members"))),
        "documentProperty \"joinRequestId\" refers to document type \"joinRequest\", not \
         \"electedCharter\"",
    );
}

#[test]
fn should_refuse_a_transient_document_property() {
    let mut schema = charter_contract(members_of_the_charter());
    schema["documentSchemas"]["resignation"]["transient"] = json!(["electedCharterId"]);
    assert_refused(
        contract(schema),
        "documentProperty \"electedCharterId\" is transient",
    );
}

/// An object listed as transient is never stored, and neither is anything in
/// it, on either side of the declaration.
#[test]
fn should_refuse_a_document_property_or_a_list_inside_a_transient_object() {
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
        "position": 7
    });
    let with_objects = |refers_to: serde_json::Value| {
        let mut schema = charter_contract(refers_to);
        schema["documentSchemas"]["electedCharter"]["properties"]["seats"] = seats.clone();
        schema["documentSchemas"]["resignation"]["properties"]["meta"] = meta.clone();
        schema
    };

    // Stored, the nested paths are accepted
    contract(with_objects(list_element(
        "meta.charterId",
        "seats.members",
    )))
    .expect("nested stored paths are accepted");

    let mut transient_document_property =
        with_objects(list_element("meta.charterId", "seats.members"));
    transient_document_property["documentSchemas"]["resignation"]["transient"] = json!(["meta"]);
    assert_refused(
        contract(transient_document_property),
        "documentProperty \"meta.charterId\" is transient",
    );

    let mut transient_list = with_objects(list_element("meta.charterId", "seats.members"));
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
        let schema = charter_contract(list_element("electedCharterId", list));
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

    // A list a replace could change (the key `lookedUpCharterId` finds a
    // charter by stays frozen, so the lookup beside it is not what refuses)
    let mutable = with_elected_charter(
        charter_contract(members_of_the_charter()),
        "documentsMutable",
        json!(true),
    );
    let frozen_except =
        |frozen: &[&str]| with_elected_charter(mutable.clone(), "immutable", json!(frozen));
    assert_refused(
        contract(frozen_except(&["submittedCharterId"])),
        "property \"memberId\" refersTo listElement: \"members\" of \"electedCharter\" can be \
         changed by a replace",
    );

    // Freezing the list on the mutable type is enough
    contract(frozen_except(&["submittedCharterId", "members"]))
        .expect("an immutable list on a mutable type is accepted");

    // Freezing another property is not
    assert_refused(
        contract(frozen_except(&["submittedCharterId", "title"])),
        "\"members\" of \"electedCharter\" can be changed by a replace",
    );
}

#[test]
fn should_leave_a_list_in_another_contract_to_registration() {
    // `documentProperty` refers to a charter of another contract, so the parse
    // cannot see its list: registration checks it against that contract in state
    let mut schema = charter_contract(list_element("electedCharterId", "anything"));
    schema["documentSchemas"]["resignation"]["properties"]["electedCharterId"]["refersTo"]
        ["contractId"] = json!(Identifier::from([9; 32]).to_string(Encoding::Base58));
    let parsed = contract(schema).expect("parses");
    assert_eq!(
        resignation_property_type(&parsed, "memberId"),
        DocumentPropertyType::IdentifierWithReference(expected_reference(
            "electedCharterId",
            "anything"
        ))
    );
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
            "electedCharterId",
            "members"
        ))
    );
}

#[test]
fn should_count_list_elements_against_the_reference_bound() {
    let limit = PlatformVersion::latest()
        .system_limits
        .max_references_per_document;
    // The type's other references: `electedCharterId`, `memberId`,
    // `lookedUpCharterId`, `joinRequestId` and `identityId`
    let others = 5;
    let witnesses_up_to = |max_items: u16| {
        let mut schema = charter_contract(members_of_the_charter());
        schema["documentSchemas"]["resignation"]["properties"]["witnesses"] =
            identifier_list(7, u32::from(max_items), Some(members_of_the_charter()));
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
    let mut schema = charter_contract(members_of_the_charter());
    schema["documentSchemas"]["resignation"]["properties"]["witnesses"] =
        identifier_list(7, 4, Some(list_element("lookedUpCharterId", "members")));

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
            "electedCharterId",
            "members"
        ))
    );
}

#[test]
fn should_refuse_malformed_list_element_declarations_in_the_parser() {
    for (refers_to, fragment) in [
        (
            json!({
                "type": "listElement",
                "contractId": Identifier::from([9; 32]).to_string(Encoding::Base58),
                "documentType": "electedCharter",
                "documentProperty": "electedCharterId",
                "list": "members"
            }),
            "listElement refersTo does not take contractId",
        ),
        (
            json!({ "type": "listElement", "documentType": "electedCharter", "list": "members" }),
            "documentProperty",
        ),
        (
            json!({ "type": "listElement", "documentType": "electedCharter", "documentProperty": "electedCharterId" }),
            "list",
        ),
        (
            list_element("$ownerId", "members"),
            "listElement refersTo documentProperty must be a property path",
        ),
        (
            json!({
                "type": "listElement",
                "documentType": "electedCharter",
                "documentProperty": "electedCharterId",
                "list": "members",
                "propertyAgreement": { "note": "title" }
            }),
            "propertyAgreement is only allowed on permanentDocument and deletableDocument",
        ),
        (
            json!({ "type": "identity", "list": "members" }),
            "identity refersTo does not take list",
        ),
        (
            json!({
                "type": "permanentDocument",
                "documentType": "electedCharter",
                "documentProperty": "electedCharterId"
            }),
            "permanentDocument refersTo does not take documentProperty",
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
        document_type_name: "electedCharter".to_string(),
        document_property: "electedCharterId".to_string(),
        list: "seats.members".to_string(),
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
    assert_eq!(
        DocumentPropertyReferenceTarget::ListElement(reference).to_string(),
        "list element (seats.members of the electedCharter document electedCharterId refers to)"
    );
}
