//! `refersTo` on the elements of a typed array: an identifier element's
//! `items` schema may carry the reference declaration a single identifier
//! property takes, and the element then parses to
//! `IdentifierWithReference(target)` inside the array's `item_type`.
//!
//! The declaration is read by `apply_property_reference`, the function a
//! scalar identifier goes through, from `parse_typed_array` 0 (protocol
//! version 14). The checks that need other contracts (the referenced
//! document type, the `propertyAgreement` sides and value kinds) run at
//! registration in drive-abci and are tested there.

use super::typed_array_test_helpers::{
    expect_json_schema_error, expect_structure_error, parse_dispatched,
};
use super::*;
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::array::TypedArrayProperty;
use crate::data_contract::document_type::{
    ContractReferenceModeration, ContractReferenceRequirements, DocumentPropertyReferenceTarget,
    DocumentPropertyType, PropertyReference,
};
use crate::data_contract::serialized_version::v0::DataContractInSerializationFormatV0;
use crate::data_contract::DataContract;
use crate::serialization::{
    PlatformDeserializableWithPotentialValidationFromVersionedStructureUntrusted,
    PlatformSerializableWithPlatformVersion,
};
use platform_value::platform_value;
use platform_value::string_encoding::Encoding;

/// An identifier element schema, carrying `refers_to` when given.
fn identifier_items(refers_to: Option<Value>) -> Value {
    let mut items = platform_value!({
        "type": "array",
        "byteArray": true,
        "minItems": 32,
        "maxItems": 32,
        "contentMediaType": "application/x.dash.dpp.identifier"
    });
    if let Some(refers_to) = refers_to {
        items
            .set_value("refersTo", refers_to)
            .expect("refersTo applies");
    }
    items
}

/// The `reasons` declaration of the moderation charters contract with the
/// given `items`: up to 64 distinct elements.
fn reasons_with_items(items: Value) -> Value {
    platform_value!({
        "type": "array",
        "minItems": 0,
        "maxItems": 64,
        "uniqueItems": true,
        "items": items,
        "position": 0
    })
}

/// A document type with `reasons` and a string `topic` a propertyAgreement
/// can name.
fn schema_with_reasons(reasons: Value) -> Value {
    platform_value!({
        "type": "object",
        "properties": {
            "reasons": reasons,
            "topic": { "type": "string", "maxLength": 32, "position": 1 }
        },
        "additionalProperties": false
    })
}

fn reasons_type(document_type: &DocumentType) -> DocumentPropertyType {
    document_type
        .as_ref()
        .flattened_properties()
        .get("reasons")
        .map(|property| property.property_type.clone())
        .expect("the reasons property is parsed")
}

/// Every target type a single identifier property takes, `identityPublicKey`
/// aside, with the same keys, folded into the element exactly as a scalar
/// reference is folded into its property.
#[test]
fn should_parse_an_element_reference_of_each_target_type() {
    let foreign_contract = Identifier::new([9; 32]);
    for (refers_to, expected) in [
        (
            platform_value!({ "type": "identity" }),
            DocumentPropertyReferenceTarget::Identity,
        ),
        (
            platform_value!({
                "type": "contract",
                "contractRequirements": { "moderation": "elected", "minimumAgeSeconds": 60 }
            }),
            DocumentPropertyReferenceTarget::Contract {
                contract_requirements: ContractReferenceRequirements {
                    moderation: Some(ContractReferenceModeration::Elected),
                    minimum_age_seconds: Some(60),
                    ..Default::default()
                },
            },
        ),
        (
            platform_value!({ "type": "token" }),
            DocumentPropertyReferenceTarget::Token,
        ),
        (
            platform_value!({
                "type": "permanentDocument",
                "documentType": "reason",
                "propertyAgreement": { "topic": "topic", "$ownerId": "$ownerId" }
            }),
            DocumentPropertyReferenceTarget::PermanentDocument {
                contract_id: None,
                document_type_name: "reason".to_string(),
                property_agreement: BTreeMap::from([
                    ("$ownerId".to_string(), "$ownerId".to_string()),
                    ("topic".to_string(), "topic".to_string()),
                ]),
            },
        ),
        (
            platform_value!({
                "type": "deletableDocument",
                "contractId": foreign_contract.to_string(Encoding::Base58),
                "documentType": "draft"
            }),
            DocumentPropertyReferenceTarget::DeletableDocument {
                contract_id: Some(foreign_contract),
                document_type_name: "draft".to_string(),
                property_agreement: BTreeMap::new(),
            },
        ),
    ] {
        let schema = schema_with_reasons(reasons_with_items(identifier_items(Some(
            refers_to.clone(),
        ))));
        let expected_type = DocumentPropertyType::TypedArray(TypedArrayProperty {
            item_type: Box::new(DocumentPropertyType::IdentifierWithReference(
                expected.clone(),
            )),
            item_constraints: Default::default(),
            min_items: Some(0),
            max_items: 64,
            unique_items: true,
        });
        // The meta-schema admits it, and the stored path reads it the same
        for full_validation in [true, false] {
            let document_type =
                parse_dispatched(schema.clone(), PlatformVersion::latest(), full_validation)
                    .unwrap_or_else(|error| {
                        panic!("{refers_to:?} (full validation {full_validation}): {error}")
                    });
            let parsed = reasons_type(&document_type);
            assert_eq!(parsed, expected_type, "{refers_to:?}");
            assert_eq!(
                parsed.reference(),
                Some(PropertyReference::Elements {
                    target: &expected,
                    max_items: 64
                }),
                "{refers_to:?}"
            );
        }
    }
}

/// `keyIdProperty` names one sibling key id, which cannot pair with many
/// elements.
#[test]
fn should_refuse_an_identity_public_key_reference_on_an_element() {
    let schema = platform_value!({
        "type": "object",
        "properties": {
            "reasons": reasons_with_items(identifier_items(Some(platform_value!({
                "type": "identityPublicKey",
                "keyIdProperty": "keyId"
            })))),
            "keyId": { "type": "integer", "minimum": 0, "maximum": 4294967295u64, "position": 1 }
        },
        "additionalProperties": false
    });

    let error = expect_json_schema_error(parse_dispatched(
        schema.clone(),
        PlatformVersion::latest(),
        true,
    ));
    assert!(
        error.instance_path().contains("/reasons/items/refersTo"),
        "the meta-schema refuses the element declaration, got {}",
        error.instance_path()
    );
    expect_structure_error(
        parse_dispatched(schema, PlatformVersion::latest(), false),
        "identityPublicKey refersTo is not allowed on the elements of a typed array",
    );

    // Nor the form declared on the key id itself, which names whose key it is
    let schema = schema_with_reasons(reasons_with_items(identifier_items(Some(
        platform_value!({ "type": "identityPublicKey", "identityProperty": "$ownerId" }),
    ))));
    expect_json_schema_error(parse_dispatched(
        schema.clone(),
        PlatformVersion::latest(),
        true,
    ));
    expect_structure_error(
        parse_dispatched(schema, PlatformVersion::latest(), false),
        "identityPublicKey refersTo is not allowed on the elements of a typed array",
    );
}

#[test]
fn should_refuse_refers_to_on_a_non_identifier_element() {
    for items in [
        platform_value!({ "type": "integer", "refersTo": { "type": "identity" } }),
        platform_value!({ "type": "string", "maxLength": 8, "refersTo": { "type": "identity" } }),
        // A byte array that is not an identifier
        platform_value!({
            "type": "array",
            "byteArray": true,
            "minItems": 32,
            "maxItems": 32,
            "refersTo": { "type": "identity" }
        }),
    ] {
        let schema = schema_with_reasons(reasons_with_items(items.clone()));
        let error = expect_json_schema_error(parse_dispatched(
            schema.clone(),
            PlatformVersion::latest(),
            true,
        ));
        // The identifier-only rule names the element keyword it fails on
        assert!(
            error.instance_path().contains("/reasons/items"),
            "{items:?}: the meta-schema refuses it on the element, got {}",
            error.instance_path()
        );
        expect_structure_error(
            parse_dispatched(schema, PlatformVersion::latest(), false),
            "refersTo is only allowed on identifier elements of a typed array",
        );
    }
}

/// The declaration applies to every element, so it goes on the items.
#[test]
fn should_refuse_refers_to_on_the_typed_array_itself() {
    let mut reasons = reasons_with_items(identifier_items(None));
    reasons
        .set_value("refersTo", platform_value!({ "type": "identity" }))
        .expect("refersTo applies");
    let schema = schema_with_reasons(reasons);

    expect_json_schema_error(parse_dispatched(
        schema.clone(),
        PlatformVersion::latest(),
        true,
    ));
    expect_structure_error(
        parse_dispatched(schema, PlatformVersion::latest(), false),
        "refersTo on a typed array belongs on its items",
    );

    // An identityPublicKey declaration is refused on the elements as well,
    // so the error does not send the author there
    let mut reasons = reasons_with_items(identifier_items(None));
    reasons
        .set_value(
            "refersTo",
            platform_value!({ "type": "identityPublicKey", "keyIdProperty": "topic" }),
        )
        .expect("refersTo applies");
    expect_structure_error(
        parse_dispatched(
            schema_with_reasons(reasons),
            PlatformVersion::latest(),
            false,
        ),
        "identityPublicKey refersTo is not allowed on a typed array or on its elements",
    );
}

/// The rules the parse itself holds for a `propertyAgreement`, identical for
/// an element declaration: only `$ownerId` among the referring document's
/// system properties, only `$ownerId`, `$creatorId` and `$id` among the
/// referenced document's. Whether a named schema property exists on either side is
/// checked at registration against the referenced contract (drive-abci).
#[test]
fn should_refuse_an_element_property_agreement_naming_an_unusable_system_property() {
    for (agreement, fragment) in [
        (
            platform_value!({ "$createdAt": "topic" }),
            "propertyAgreement keys must name a schema property",
        ),
        (
            platform_value!({ "topic": "$createdAt" }),
            "propertyAgreement values must name a schema property",
        ),
    ] {
        let schema = schema_with_reasons(reasons_with_items(identifier_items(Some(
            platform_value!({
                "type": "permanentDocument",
                "documentType": "reason",
                "propertyAgreement": agreement.clone()
            }),
        ))));
        expect_structure_error(
            parse_dispatched(schema, PlatformVersion::latest(), false),
            fragment,
        );
    }
}

/// Typed arrays, and so their element references, are protocol version 14
/// grammar: meta-schema v2 refuses an array that is not a byte array.
#[test]
fn should_refuse_an_element_reference_before_protocol_version_14_and_accept_it_at_14() {
    let schema = schema_with_reasons(reasons_with_items(identifier_items(Some(
        platform_value!({ "type": "permanentDocument", "documentType": "reason" }),
    ))));

    let platform_version_13 = PlatformVersion::get(13).expect("protocol version 13 exists");
    expect_json_schema_error(parse_dispatched(schema.clone(), platform_version_13, true));

    let document_type = parse_dispatched(schema, PlatformVersion::latest(), true)
        .expect("protocol version 14 admits an element reference");
    assert!(matches!(
        reasons_type(&document_type).reference(),
        Some(PropertyReference::Elements {
            target: DocumentPropertyReferenceTarget::PermanentDocument { .. },
            ..
        })
    ));
}

/// A document type whose `reasons` carries `max_items` references, next to
/// `scalar_references` identifier properties with their own `refersTo`.
fn schema_with_references(max_items: u16, scalar_references: u32) -> Value {
    let mut properties = BTreeMap::from([(
        "reasons".to_string(),
        platform_value!({
            "type": "array",
            "maxItems": max_items,
            "items": identifier_items(Some(platform_value!({ "type": "identity" }))),
            "position": 0
        }),
    )]);
    for position in 1..=scalar_references {
        let mut property = identifier_items(Some(platform_value!({ "type": "identity" })));
        property
            .set_value("position", Value::U32(position))
            .expect("position applies");
        properties.insert(format!("ref{position}"), property);
    }
    platform_value!({
        "type": "object",
        "properties": Value::from(properties),
        "additionalProperties": false
    })
}

/// Each element is a billed read when a document is written, so the
/// references one document can carry are bounded at registration: a typed
/// array counts its `maxItems`, a single reference one.
#[test]
fn should_bound_the_references_one_document_can_carry() {
    let platform_version = PlatformVersion::latest();
    let limit = platform_version.system_limits.max_references_per_document;
    assert!(
        limit < platform_version.system_limits.max_typed_array_items,
        "the bound is below one maximal typed array"
    );

    parse_dispatched(schema_with_references(limit - 2, 2), platform_version, true)
        .expect("a type carrying exactly the maximum registers");

    expect_structure_error(
        parse_dispatched(schema_with_references(limit - 1, 2), platform_version, true),
        &format!(
            "declares references for up to {} values",
            u32::from(limit) + 1
        ),
    );
    expect_structure_error(
        parse_dispatched(schema_with_references(limit + 1, 0), platform_version, true),
        &format!("above the maximum of {limit}"),
    );

    // A key reference declared on the key id itself is one read too
    let mut schema = schema_with_references(limit, 0);
    schema
        .set_value_at_full_path(
            "properties.senderKeyId",
            platform_value!({
                "type": "integer",
                "minimum": 0,
                "maximum": 4294967295u64,
                "position": 1,
                "refersTo": { "type": "identityPublicKey", "identityProperty": "$ownerId" }
            }),
        )
        .expect("the key id property applies");
    expect_structure_error(
        parse_dispatched(schema, platform_version, true),
        &format!(
            "declares references for up to {} values",
            u32::from(limit) + 1
        ),
    );

    // A stored contract was checked when it was registered
    parse_dispatched(
        schema_with_references(limit + 1, 0),
        platform_version,
        false,
    )
    .expect("the stored path does not re-apply a registration limit");
}

/// Every replace re-validates a `deletableDocument` element, and an
/// immutable array could not drop a dead one, so the pair is refused.
#[test]
fn should_refuse_an_immutable_typed_array_of_deletable_document_references() {
    let schema_with = |refers_to: Value, immutable: Value| {
        let mut schema = schema_with_reasons(reasons_with_items(identifier_items(Some(refers_to))));
        schema
            .set_value("documentsMutable", Value::Bool(true))
            .expect("documentsMutable applies");
        schema
            .set_value("immutable", immutable)
            .expect("immutable applies");
        schema
    };

    expect_structure_error(
        parse_dispatched(
            schema_with(
                platform_value!({ "type": "deletableDocument", "documentType": "draft" }),
                platform_value!(["reasons"]),
            ),
            PlatformVersion::latest(),
            true,
        ),
        "is a typed array of deletableDocument references",
    );

    // Permanent targets never die, and a mutable array can drop a dead one
    parse_dispatched(
        schema_with(
            platform_value!({ "type": "permanentDocument", "documentType": "reason" }),
            platform_value!(["reasons"]),
        ),
        PlatformVersion::latest(),
        true,
    )
    .expect("an immutable array of permanentDocument references registers");
    parse_dispatched(
        schema_with(
            platform_value!({ "type": "deletableDocument", "documentType": "draft" }),
            platform_value!(["topic"]),
        ),
        PlatformVersion::latest(),
        true,
    )
    .expect("a mutable array of deletableDocument references registers");

    // Inside an immutable object, a list and a single reference alike: the
    // replace could only clear either by changing the object
    let deletable = platform_value!({ "type": "deletableDocument", "documentType": "draft" });
    let mut single_reference = identifier_items(Some(deletable.clone()));
    single_reference
        .set_value("position", Value::U32(1))
        .expect("position applies");
    let mut drafts = reasons_with_items(identifier_items(Some(deletable.clone())));
    drafts
        .set_value("position", Value::U32(0))
        .expect("position applies");
    for (member, member_schema, held_as) in [
        (
            "drafts",
            drafts,
            "a typed array of deletableDocument references",
        ),
        (
            "lead",
            single_reference,
            "a deletableDocument reference inside an object",
        ),
    ] {
        let schema = platform_value!({
            "type": "object",
            "documentsMutable": true,
            "immutable": ["team"],
            "properties": {
                "team": {
                    "type": "object",
                    "position": 0,
                    "properties": { member: member_schema },
                    "additionalProperties": false
                }
            },
            "additionalProperties": false
        });
        expect_structure_error(
            parse_dispatched(schema, PlatformVersion::latest(), true),
            &format!("\"team.{member}\" is {held_as}"),
        );
    }

    // A single reference that is itself the immutable property can be
    // cleared once its target is gone, so it registers
    let mut single_reference = identifier_items(Some(deletable));
    single_reference
        .set_value("position", Value::U32(0))
        .expect("position applies");
    parse_dispatched(
        platform_value!({
            "type": "object",
            "documentsMutable": true,
            "immutable": ["draftId"],
            "properties": { "draftId": single_reference },
            "additionalProperties": false
        }),
        PlatformVersion::latest(),
        true,
    )
    .expect("an immutable top-level deletableDocument reference registers");
}

/// A contract with `reason` (never deleted) and `submittedCharter`, whose
/// `reasons` elements refer to reasons of the same contract.
fn charter_contract(platform_version: &PlatformVersion) -> DataContract {
    let config = DataContractConfig::default_for_version(platform_version)
        .expect("default config available on this platform version");
    let reason = platform_value!({
        "type": "object",
        "canBeDeleted": false,
        "properties": {
            "topic": { "type": "string", "maxLength": 32, "position": 0 }
        },
        "additionalProperties": false
    });
    let submitted_charter = schema_with_reasons(reasons_with_items(identifier_items(Some(
        platform_value!({
            "type": "permanentDocument",
            "documentType": "reason",
            "propertyAgreement": { "topic": "topic" }
        }),
    ))));

    DataContract::try_from_platform_versioned(
        DataContractInSerializationFormatV0 {
            id: Identifier::new([7; 32]),
            config,
            version: 1,
            owner_id: Identifier::new([8; 32]),
            schema_defs: None,
            document_schemas: BTreeMap::from([
                ("reason".to_string(), reason),
                ("submittedCharter".to_string(), submitted_charter),
            ]),
        }
        .into(),
        true,
        &mut vec![],
        platform_version,
    )
    .expect("the charter contract registers")
}

#[test]
fn should_round_trip_a_contract_with_an_element_reference_through_platform_serialization() {
    let platform_version = PlatformVersion::latest();
    let contract = charter_contract(platform_version);

    let bytes = contract
        .serialize_to_bytes_with_platform_version(platform_version)
        .expect("the contract serializes");
    for full_validation in [true, false] {
        let restored = DataContract::versioned_deserialize_untrusted(
            &bytes,
            full_validation,
            platform_version,
        )
        .expect("the contract deserializes");

        assert_eq!(restored, contract);
        let charter = restored
            .document_type_for_name("submittedCharter")
            .expect("the charter type exists");
        let reasons = charter
            .flattened_properties()
            .get("reasons")
            .expect("reasons is parsed");
        assert_eq!(
            reasons.property_type.reference(),
            Some(PropertyReference::Elements {
                target: &DocumentPropertyReferenceTarget::PermanentDocument {
                    contract_id: None,
                    document_type_name: "reason".to_string(),
                    property_agreement: BTreeMap::from([(
                        "topic".to_string(),
                        "topic".to_string()
                    )]),
                },
                max_items: 64,
            })
        );
    }
}
