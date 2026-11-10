//! References whose target the referring document names (`contractIdProperty`,
//! `documentTypeProperty`, `documentTypeRequirements`, protocol version 15): the parse of
//! the declaration, where it may sit, the checks of the properties it reads, the protocol
//! version gate, what a document type must look like to meet the requirements, and the
//! platform serialization round trip.

use super::reference_test_helpers::{
    assert_refused, assert_refused_by_the_parser, contract, contract_on, identifier, CONTRACT_ID,
};
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::config::moderation::{ContractModerationConfig, ContractModerators};
use crate::data_contract::config::DataContractConfig;
use crate::data_contract::document_type::accessors::{
    DocumentTypeV0Getters, DocumentTypeV2Getters,
};
use crate::data_contract::document_type::{
    DocumentPropertyReferenceTarget, DocumentPropertyType, DocumentType, DocumentTypeRequirements,
    DynamicDocumentReference, DynamicDocumentReferenceKind, RequiredPropertySlot, SlotPropertyName,
    SlotRequirement, SlotRequirementTerm, SlotValue,
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
use std::collections::{BTreeMap, BTreeSet};

fn string(position: u32) -> serde_json::Value {
    json!({ "type": "string", "maxLength": 64, "position": position })
}

/// The declaration the decision proposals design was sketched with: a moderated document
/// of a type holding text, as a string or as bytes.
fn text_or_bytes() -> serde_json::Value {
    json!({
        "type": "moderatedDocument",
        "contractIdProperty": "descriptionContractId",
        "documentTypeProperty": "descriptionDocumentType",
        "documentTypeRequirements": {
            "properties": {
                "body": { "name": "$any", "type": "string" },
                "bodyBytes": { "name": "$any", "type": "array", "byteArray": true }
            },
            "require": { "anyOf": ["body", "bodyBytes"] }
        }
    })
}

/// A dynamic `moderatedDocument` reference with `requirements`.
fn requiring(requirements: serde_json::Value) -> serde_json::Value {
    json!({
        "type": "moderatedDocument",
        "contractIdProperty": "descriptionContractId",
        "documentTypeProperty": "descriptionDocumentType",
        "documentTypeRequirements": requirements
    })
}

/// A contract with a `note` type whose `descriptionId` declares `refers_to`, beside the
/// properties a dynamic reference reads (`descriptionContractId`, an identifier, and
/// `descriptionDocumentType` and `bodyProperty`, strings), an optional `label`, and a
/// permanent `post` type with properties of every kind a slot describes: `title` (a string
/// of at most 252 bytes), `body` (at most 4096 bytes), `summary` (unbounded), `blob` (at
/// most 1024 bytes), `authorId`, `count` and `pinned`.
fn note_contract(refers_to: serde_json::Value) -> serde_json::Value {
    let mut description_id = identifier(0);
    description_id["refersTo"] = refers_to;
    json!({
        "$formatVersion": "1",
        "id": Identifier::from(CONTRACT_ID).to_string(Encoding::Base58),
        "ownerId": Identifier::from([8; 32]).to_string(Encoding::Base58),
        "version": 1,
        "documentSchemas": {
            "note": {
                "type": "object",
                "properties": {
                    "descriptionId": description_id,
                    "descriptionContractId": identifier(1),
                    "descriptionDocumentType": string(2),
                    "bodyProperty": string(3),
                    "label": string(4)
                },
                "required": [
                    "descriptionId",
                    "descriptionContractId",
                    "descriptionDocumentType",
                    "bodyProperty"
                ],
                "additionalProperties": false
            },
            "post": {
                "type": "object",
                "canBeDeleted": false,
                "documentsMutable": false,
                "properties": {
                    "title": { "type": "string", "maxLength": 63, "position": 0 },
                    "body": { "type": "string", "maxLength": 4096, "maxBytes": 4096, "position": 1 },
                    "summary": { "type": "string", "position": 2 },
                    "blob": { "type": "array", "byteArray": true, "maxItems": 1024, "position": 3 },
                    "authorId": identifier(4),
                    "count": { "type": "integer", "minimum": 0, "position": 5 },
                    "pinned": { "type": "boolean", "position": 6 }
                },
                "required": ["title"],
                "additionalProperties": false
            }
        }
    })
}

/// The dynamic reference `note.descriptionId` parsed to.
fn dynamic_reference(contract: &DataContract) -> DynamicDocumentReference {
    let note = contract.document_type_for_name("note").expect("note");
    match &note
        .properties()
        .get("descriptionId")
        .expect("descriptionId")
        .property_type
    {
        DocumentPropertyType::IdentifierWithReference(
            DocumentPropertyReferenceTarget::DynamicDocument(reference),
        ) => reference.clone(),
        other => panic!("expected a dynamic reference, got {other:?}"),
    }
}

/// `requirements` as the parser reads them.
fn requirements(requirements: serde_json::Value) -> DocumentTypeRequirements {
    dynamic_reference(&contract(note_contract(requiring(requirements))).expect("parses"))
        .requirements
        .expect("requirements")
}

/// Whether the fixture's `post` type meets `requirements`, a slot naming its property
/// through the referring document's `bodyProperty` reading `body_property` there.
fn post_meets(requirements_value: serde_json::Value, body_property: Option<&str>) -> bool {
    let parsed = contract(note_contract(requiring(requirements_value))).expect("parses");
    let post = parsed.document_type_for_name("post").expect("post");
    dynamic_reference(&parsed)
        .requirements
        .expect("requirements")
        .met_by(post, |referring_property| {
            assert_eq!(referring_property, "bodyProperty");
            body_property.map(str::to_string)
        })
}

fn slot(name: SlotPropertyName, value: SlotValue) -> RequiredPropertySlot {
    RequiredPropertySlot {
        name,
        value,
        max_bytes: None,
        distinct_from: BTreeSet::new(),
    }
}

#[test]
fn should_parse_a_reference_naming_its_target_through_the_referring_document() {
    let parsed = contract(note_contract(text_or_bytes())).expect("parses");
    let reference = dynamic_reference(&parsed);
    assert_eq!(
        reference,
        DynamicDocumentReference {
            kind: DynamicDocumentReferenceKind::Moderated,
            contract_id_property: "descriptionContractId".to_string(),
            document_type_property: "descriptionDocumentType".to_string(),
            requirements: Some(DocumentTypeRequirements {
                slots: BTreeMap::from([
                    (
                        "body".to_string(),
                        slot(SlotPropertyName::Any, SlotValue::String)
                    ),
                    (
                        "bodyBytes".to_string(),
                        slot(SlotPropertyName::Any, SlotValue::ByteArray)
                    ),
                ]),
                require: Some(SlotRequirement::AnyOf(vec![
                    SlotRequirementTerm::Slot("body".to_string()),
                    SlotRequirementTerm::Slot("bodyBytes".to_string()),
                ])),
            }),
        }
    );
    assert_eq!(reference.wire_name(), "moderatedDocument");
    assert_eq!(
        reference.target_properties().collect::<Vec<_>>(),
        ["descriptionContractId", "descriptionDocumentType"]
    );
    // It names no contract and no document type, so it is no declaration for the code that
    // resolves a target from the schema
    let target = DocumentPropertyReferenceTarget::DynamicDocument(reference);
    assert!(target.as_any_document_reference().is_none());
    assert!(target.as_document_reference().is_none());
    assert_eq!(
        target.to_string(),
        "moderated document (contract named by descriptionContractId, document type named by \
         descriptionDocumentType)"
    );

    // A permanent document, with no requirements
    let parsed = contract(note_contract(json!({
        "type": "permanentDocument",
        "contractIdProperty": "descriptionContractId",
        "documentTypeProperty": "descriptionDocumentType"
    })))
    .expect("parses");
    let reference = dynamic_reference(&parsed);
    assert_eq!(reference.kind, DynamicDocumentReferenceKind::Permanent);
    assert_eq!(reference.requirements, None);
}

#[test]
fn should_parse_every_form_of_a_slot_and_of_require() {
    let parsed = requirements(json!({
        "properties": {
            "title": { "name": "$any", "type": "string", "maxBytes": 256 },
            "body": {
                "name": { "property": "bodyProperty" },
                "type": "string",
                "distinctFrom": "title"
            },
            "author": { "name": "authorId", "type": "identifier" },
            "extra": {
                "name": "$any",
                "type": "array",
                "byteArray": true,
                "maxBytes": 2048,
                "distinctFrom": ["title", "body"]
            }
        },
        "require": { "allOf": ["title", "author", { "anyOf": ["body", "extra"] }] }
    }));
    assert_eq!(
        parsed.slots["title"],
        RequiredPropertySlot {
            max_bytes: Some(256),
            ..slot(SlotPropertyName::Any, SlotValue::String)
        }
    );
    assert_eq!(
        parsed.slots["body"],
        RequiredPropertySlot {
            distinct_from: BTreeSet::from(["title".to_string()]),
            ..slot(
                SlotPropertyName::FromProperty("bodyProperty".to_string()),
                SlotValue::String
            )
        }
    );
    assert_eq!(
        parsed.slots["author"],
        slot(
            SlotPropertyName::Named("authorId".to_string()),
            SlotValue::Identifier
        )
    );
    assert_eq!(
        parsed.slots["extra"],
        RequiredPropertySlot {
            max_bytes: Some(2048),
            distinct_from: BTreeSet::from(["title".to_string(), "body".to_string()]),
            ..slot(SlotPropertyName::Any, SlotValue::ByteArray)
        }
    );
    assert_eq!(
        parsed.require,
        Some(SlotRequirement::AllOf(vec![
            SlotRequirementTerm::Slot("title".to_string()),
            SlotRequirementTerm::Slot("author".to_string()),
            SlotRequirementTerm::Group(vec!["body".to_string(), "extra".to_string()]),
        ]))
    );
    assert_eq!(
        parsed.name_properties().collect::<Vec<_>>(),
        ["bodyProperty"]
    );

    // One slot, required by name
    let parsed = requirements(json!({
        "properties": { "count": { "name": "count", "type": "integer" } },
        "require": "count"
    }));
    assert_eq!(
        parsed.require,
        Some(SlotRequirement::Slot("count".to_string()))
    );
}

#[test]
fn should_not_read_a_dynamic_target_before_protocol_version_15() {
    let platform_version_14 = PlatformVersion::get(14).expect("protocol version 14");
    // Registration refuses the keys it does not know, and the parser of version 14, which
    // does not read them, refuses the document reference they leave without a documentType
    for full_validation in [true, false] {
        contract_on(
            note_contract(text_or_bytes()),
            full_validation,
            platform_version_14,
        )
        .expect_err("protocol version 14 has no dynamic target");
    }
    // Beside a documentType, version 14's parser reads the reference it always read
    let mut refers_to = text_or_bytes();
    refers_to["documentType"] = json!("post");
    let parsed = contract_on(note_contract(refers_to.clone()), false, platform_version_14)
        .expect("version 14 does not read the keys");
    let note = parsed.document_type_for_name("note").expect("note");
    assert!(matches!(
        note.properties()["descriptionId"].property_type,
        DocumentPropertyType::IdentifierWithReference(
            DocumentPropertyReferenceTarget::ModeratedDocument { .. }
        )
    ));
    // where protocol version 15 refuses the two together
    assert_refused_by_the_parser(note_contract(refers_to), "does not take documentType");
}

#[test]
fn should_refuse_what_cannot_sit_beside_a_dynamic_target() {
    let with = |key: &str, value: serde_json::Value| {
        let mut refers_to = text_or_bytes();
        refers_to[key] = value;
        note_contract(refers_to)
    };
    for (key, value) in [
        ("documentType", json!("post")),
        (
            "contractId",
            json!(Identifier::from(CONTRACT_ID).to_string(Encoding::Base58)),
        ),
        ("where", json!({ "$ownerId": "$ownerId" })),
        ("findBy", json!({ "title": "." })),
    ] {
        assert_refused_by_the_parser(with(key, value), &format!("does not take {key}"));
    }

    // Only a permanent or a moderated document
    let mut deletable = text_or_bytes();
    deletable["type"] = json!("deletableDocument");
    assert_refused_by_the_parser(
        note_contract(deletable),
        "only a permanentDocument or a moderatedDocument reference names its target",
    );

    // Both properties, and two different ones
    let mut missing = text_or_bytes();
    missing
        .as_object_mut()
        .expect("an object")
        .remove("documentTypeProperty");
    assert_refused_by_the_parser(
        note_contract(missing),
        "needs both contractIdProperty and documentTypeProperty",
    );
    let mut same = text_or_bytes();
    same["documentTypeProperty"] = json!("descriptionContractId");
    assert_refused_by_the_parser(note_contract(same), "name the same property");
}

#[test]
fn should_refuse_a_dynamic_target_anywhere_but_on_an_identifier_property() {
    // On a string property
    let mut on_a_string = note_contract(text_or_bytes());
    on_a_string["documentSchemas"]["note"]["properties"]["descriptionId"] =
        json!({ "type": "string", "maxLength": 64, "position": 0, "refersTo": text_or_bytes() });
    assert_refused_by_the_parser(on_a_string, "only allowed on identifier properties");

    // On the elements of a typed array
    let mut on_elements = note_contract(text_or_bytes());
    let mut items = identifier(0);
    items.as_object_mut().expect("an object").remove("position");
    items["refersTo"] = text_or_bytes();
    on_elements["documentSchemas"]["note"]["properties"]["descriptionId"] =
        json!({ "type": "array", "maxItems": 4, "items": items, "position": 0 });
    assert_refused_by_the_parser(on_elements, "on the elements of a typed array");

    // As an operand of a reference expression
    let in_an_expression = note_contract(json!({
        "anyOf": [{ "type": "identity" }, text_or_bytes()]
    }));
    assert_refused_by_the_parser(in_an_expression, "in a reference expression");

    // As the writer's reference
    let mut as_owner = note_contract(json!({ "type": "identity" }));
    as_owner["documentSchemas"]["note"]["ownerRefersTo"] = text_or_bytes();
    assert_refused_by_the_parser(as_owner, "in ownerRefersTo");
}

#[test]
fn should_hold_the_properties_a_dynamic_target_reads_to_the_declaring_type() {
    let naming = |key: &str, property: &str| {
        let mut refers_to = text_or_bytes();
        refers_to[key] = json!(property);
        note_contract(refers_to)
    };
    // The contract id is a top-level identifier property, other than the reference
    assert_refused_by_the_parser(
        naming("contractIdProperty", "label"),
        "which is not an identifier property",
    );
    assert_refused_by_the_parser(
        naming("contractIdProperty", "missing"),
        "which is not a top-level property",
    );
    assert_refused_by_the_parser(
        naming("contractIdProperty", "descriptionId"),
        "the property carrying the reference",
    );
    // The document type name is a string property
    assert_refused_by_the_parser(
        naming("documentTypeProperty", "descriptionId"),
        "the property carrying the reference",
    );
    let mut type_from_an_identifier = note_contract(text_or_bytes());
    type_from_an_identifier["documentSchemas"]["note"]["properties"]["descriptionDocumentType"] =
        identifier(2);
    assert_refused_by_the_parser(type_from_an_identifier, "which is not a string property");
    // and each is required where the reference is
    assert_refused_by_the_parser(
        naming("documentTypeProperty", "label"),
        "which is not required while the reference is",
    );
    // A slot's name property too
    let named_by = |property: &str| {
        note_contract(requiring(json!({
            "properties": {
                "body": { "name": { "property": property }, "type": "string" }
            }
        })))
    };
    assert_refused_by_the_parser(
        named_by("descriptionContractId"),
        "which is not a string property",
    );
    assert_refused_by_the_parser(
        named_by("label"),
        "which is not required while the reference is",
    );
    contract(named_by("bodyProperty")).expect("a required string property names the slot's");

    // An optional reference reads optional properties
    let mut optional = naming("documentTypeProperty", "label");
    optional["documentSchemas"]["note"]["required"] = json!(["descriptionContractId"]);
    contract(optional).expect("nothing is required where the reference is not");
}

#[test]
fn should_refuse_malformed_document_type_requirements() {
    let refused = |requirements: serde_json::Value, fragment: &str| {
        assert_refused_by_the_parser(note_contract(requiring(requirements)), fragment);
    };
    let any_string = || json!({ "name": "$any", "type": "string" });

    refused(json!({ "require": "body" }), "needs properties");
    refused(json!({ "properties": {} }), "must hold 1 to 4 slots");
    refused(
        json!({ "properties": {
            "a": any_string(), "b": any_string(), "c": any_string(),
            "d": any_string(), "e": any_string()
        } }),
        "must hold 1 to 4 slots, got 5",
    );
    refused(
        json!({ "properties": { "body": any_string() }, "allOf": ["body"] }),
        "does not take allOf",
    );

    // A slot
    refused(
        json!({ "properties": { "body": { "type": "string" } } }),
        "needs a name",
    );
    refused(
        json!({ "properties": { "body": { "name": "$all", "type": "string" } } }),
        "needs a name",
    );
    refused(
        json!({ "properties": { "body": { "name": "$any" } } }),
        "needs a type",
    );
    refused(
        json!({ "properties": { "body": { "name": "$any", "type": "array" } } }),
        "needs a type",
    );
    refused(
        json!({ "properties": { "body": { "name": "$any", "type": "string", "byteArray": true } } }),
        "needs a type",
    );
    refused(
        json!({ "properties": { "body": { "name": "$any", "type": "object" } } }),
        "needs a type",
    );
    refused(
        json!({ "properties": { "n": { "name": "$any", "type": "integer", "maxBytes": 8 } } }),
        "maxBytes bounds a string or a byte array",
    );
    refused(
        json!({ "properties": { "body": { "name": "$any", "type": "string", "maxBytes": 0 } } }),
        "maxBytes must be an integer from 1",
    );
    refused(
        json!({ "properties": { "body": { "name": "$any", "type": "string", "present": true } } }),
        "does not take present",
    );

    // distinctFrom names other slots
    refused(
        json!({ "properties": {
            "body": { "name": "$any", "type": "string", "distinctFrom": "body" }
        } }),
        "must name another slot",
    );
    refused(
        json!({ "properties": {
            "body": { "name": "$any", "type": "string", "distinctFrom": ["title"] }
        } }),
        "must name another slot",
    );
    refused(
        json!({ "properties": {
            "title": any_string(),
            "body": { "name": "$any", "type": "string", "distinctFrom": ["title", "title"] }
        } }),
        "names a slot twice",
    );

    // require names every slot, and only slots
    refused(
        json!({ "properties": { "body": any_string() }, "require": "title" }),
        "which is not a slot",
    );
    refused(
        json!({ "properties": { "title": any_string(), "body": any_string() }, "require": "body" }),
        "does not name the slot \"title\"",
    );
    refused(
        json!({ "properties": { "title": any_string(), "body": any_string() },
                "require": { "anyOf": ["title"] } }),
        "require must be a slot name",
    );
    refused(
        json!({ "properties": { "title": any_string(), "body": any_string() },
                "require": { "anyOf": ["title", { "anyOf": ["title", "body"] }] } }),
        "require must be a slot name",
    );
}

#[test]
fn should_fill_a_slot_with_a_property_of_the_described_kind() {
    let one = |slot: serde_json::Value| json!({ "properties": { "slot": slot } });
    // Any property of the kind
    for value_type in ["string", "identifier", "integer", "boolean"] {
        assert!(
            post_meets(one(json!({ "name": "$any", "type": value_type })), None),
            "{value_type}"
        );
    }
    assert!(post_meets(
        one(json!({ "name": "$any", "type": "array", "byteArray": true })),
        None
    ));
    // A named one, of that kind
    assert!(post_meets(
        one(json!({ "name": "body", "type": "string" })),
        None
    ));
    assert!(!post_meets(
        one(json!({ "name": "body", "type": "integer" })),
        None
    ));
    assert!(!post_meets(
        one(json!({ "name": "text", "type": "string" })),
        None
    ));
    // An identifier is not a byte array slot's, nor a byte array an identifier's
    assert!(!post_meets(
        one(json!({ "name": "authorId", "type": "array", "byteArray": true })),
        None
    ));
    assert!(!post_meets(
        one(json!({ "name": "blob", "type": "identifier" })),
        None
    ));
    // One the referring document names
    let named = one(json!({ "name": { "property": "bodyProperty" }, "type": "string" }));
    assert!(post_meets(named.clone(), Some("body")));
    assert!(!post_meets(named.clone(), Some("count")));
    assert!(!post_meets(named.clone(), Some("missing")));
    assert!(!post_meets(named, None));
}

#[test]
fn should_hold_a_slot_to_its_byte_bound() {
    let string_of = |max_bytes: u32| json!({ "properties": { "text": { "name": "$any", "type": "string", "maxBytes": max_bytes } } });
    // `title` holds at most 63 characters, 252 bytes; `body` 4096 bytes; `summary` declares
    // no bound, which no bound admits
    assert!(post_meets(string_of(252), None));
    assert!(!post_meets(string_of(251), None));
    let named = |name: &str, max_bytes: u32| json!({ "properties": { "text": { "name": name, "type": "string", "maxBytes": max_bytes } } });
    assert!(post_meets(named("body", 4096), None));
    assert!(!post_meets(named("body", 4095), None));
    assert!(!post_meets(named("summary", u32::MAX), None));
    assert!(post_meets(
        json!({ "properties": { "text": { "name": "summary", "type": "string" } } }),
        None
    ));
    let bytes_of = |max_bytes: u32| {
        json!({ "properties": {
            "bytes": { "name": "$any", "type": "array", "byteArray": true, "maxBytes": max_bytes }
        } })
    };
    assert!(post_meets(bytes_of(1024), None));
    assert!(!post_meets(bytes_of(1023), None));
}

#[test]
fn should_fill_distinct_slots_with_different_properties() {
    let short = || json!({ "name": "$any", "type": "string", "maxBytes": 252 });
    // One property is short enough: it fills one slot, or both where they need not differ
    assert!(post_meets(
        json!({ "properties": { "a": short(), "b": short() } }),
        None
    ));
    let mut distinct = short();
    distinct["distinctFrom"] = json!("a");
    assert!(!post_meets(
        json!({ "properties": { "a": short(), "b": distinct } }),
        None
    ));
    // Stated on either side
    let mut first = short();
    first["distinctFrom"] = json!(["b"]);
    assert!(!post_meets(
        json!({ "properties": { "a": first, "b": short() } }),
        None
    ));
    // Three strings fill three distinct slots, whichever order the candidates come in: the
    // slot only `title` fills takes it, though every other slot admits it too
    let any = |distinct_from: serde_json::Value| json!({ "name": "$any", "type": "string", "distinctFrom": distinct_from });
    assert!(post_meets(
        json!({ "properties": {
            "a": { "name": "$any", "type": "string" },
            "b": any(json!("a")),
            "c": { "name": "$any", "type": "string", "maxBytes": 252, "distinctFrom": ["a", "b"] }
        } }),
        None
    ));
    // and there is no fourth
    assert!(!post_meets(
        json!({ "properties": {
            "a": { "name": "$any", "type": "string" },
            "b": any(json!("a")),
            "c": any(json!(["a", "b"])),
            "d": any(json!(["a", "b", "c"]))
        } }),
        None
    ));
}

#[test]
fn should_require_the_slots_require_names() {
    let slots = json!({
        "text": { "name": "text", "type": "string" },
        "bytes": { "name": "$any", "type": "array", "byteArray": true },
        "count": { "name": "count", "type": "integer" }
    });
    let requiring_slots = |require: Option<serde_json::Value>| {
        let mut requirements = json!({ "properties": slots.clone() });
        if let Some(require) = require {
            requirements["require"] = require;
        }
        requirements
    };
    // The type has no `text`: every slot is required where nothing says otherwise
    assert!(!post_meets(requiring_slots(None), None));
    // Either the text or the bytes, and the count
    assert!(post_meets(
        requiring_slots(Some(
            json!({ "allOf": ["count", { "anyOf": ["text", "bytes"] }] })
        )),
        None
    ));
    // The text and the count, or the bytes
    assert!(post_meets(
        requiring_slots(Some(
            json!({ "anyOf": ["bytes", { "allOf": ["text", "count"] }] })
        )),
        None
    ));
    // The text, with or without the rest
    assert!(!post_meets(
        requiring_slots(Some(
            json!({ "allOf": ["text", { "anyOf": ["bytes", "count"] }] })
        )),
        None
    ));
    assert!(!post_meets(
        requiring_slots(Some(
            json!({ "anyOf": [{ "allOf": ["text", "bytes"] }, { "allOf": ["text", "count"] }] })
        )),
        None
    ));

    // A slot left empty frees its property for a slot that needs it: `first` and `second`
    // both want `title` alone and must differ, and `require` asks for `second` only
    let contended = json!({
        "properties": {
            "first": { "name": "title", "type": "string" },
            "second": { "name": "title", "type": "string", "distinctFrom": "first" }
        },
        "require": { "anyOf": ["second", { "allOf": ["first", "second"] }] }
    });
    assert!(post_meets(contended, None));
}

#[test]
fn should_round_trip_a_contract_through_platform_serialization_with_a_dynamic_reference() {
    let platform_version = PlatformVersion::latest();
    let original = contract(note_contract(requiring(json!({
        "properties": {
            "title": { "name": "$any", "type": "string", "maxBytes": 256 },
            "body": {
                "name": { "property": "bodyProperty" },
                "type": "string",
                "distinctFrom": ["title"]
            },
            "bytes": { "name": "blob", "type": "array", "byteArray": true }
        },
        "require": { "allOf": ["title", { "anyOf": ["body", "bytes"] }] }
    }))))
    .expect("parses");
    let bytes = original
        .serialize_to_bytes_with_platform_version(platform_version)
        .expect("the contract should serialize");
    let recovered = DataContract::versioned_deserialize_untrusted(&bytes, false, platform_version)
        .expect("the contract should deserialize");
    assert_eq!(original, recovered);
    assert_eq!(dynamic_reference(&original), dynamic_reference(&recovered));
}

#[test]
fn should_refuse_a_rule_or_an_index_reading_through_a_dynamic_reference() {
    // A rule reads the values of a document a reference names in the schema
    let mut with_a_rule = note_contract(text_or_bytes());
    with_a_rule["documentSchemas"]["note"]["propertyConstraints"] = json!({
        "ownDescription": {
            "equal": [{ "referenced": ["descriptionId", "$ownerId"] }, "$ownerId"]
        }
    });
    // The refusal says why: the reference is to a document, of a type the schema does not name
    assert_refused(
        contract(with_a_rule),
        "\"descriptionId\", whose refersTo names its target through the referring document",
    );

    // and an index holds a value of one
    let mut with_an_index = note_contract(text_or_bytes());
    with_an_index["documentSchemas"]["note"]["indices"] = json!([
        { "name": "byDescriptionOwner", "properties": [{ "descriptionId.$ownerId": "asc" }] }
    ]);
    contract(with_an_index).expect_err("an index reads no value through a dynamic reference");
}

/// A `note` of a contract its owner moderates, describing itself with a document of a type
/// it names whose text is the string property the note names. Nothing of the description
/// is required, so a moderator could be given any of its properties to write. `abilities`
/// is the type's `moderatorAbilities`.
fn moderated_note(abilities: serde_json::Value) -> Result<DocumentType, crate::ProtocolError> {
    let platform_version = PlatformVersion::latest();
    let config = DataContractConfig::default_for_version(platform_version)
        .expect("default config")
        .with_moderation(Some(ContractModerationConfig {
            banlist: false,
            suspensions: false,
            moderators: ContractModerators::ContractOwner,
            warnings: false,
        }));
    let mut description_id = identifier(0);
    description_id["refersTo"] = requiring(json!({
        "properties": {
            "body": { "name": { "property": "bodyProperty" }, "type": "string" }
        }
    }));
    let schema = json!({
        "type": "object",
        "properties": {
            "descriptionId": description_id,
            "descriptionContractId": identifier(1),
            "descriptionDocumentType": string(2),
            "bodyProperty": string(3),
            "label": string(4)
        },
        "additionalProperties": false,
        "moderatorAbilities": abilities
    });
    DocumentType::try_from_schema(
        Identifier::new([1; 32]),
        1,
        config.version(),
        "note",
        platform_value::to_value(schema).expect("the schema should convert"),
        None,
        &BTreeMap::new(),
        &config,
        true,
        &mut vec![],
        platform_version,
    )
}

/// A moderator's change of a field is not a replace: no reference is checked again. So a
/// moderator is given none of the properties a reference reads its target from, or it could
/// point a stored reference at a document type, or a property of one, nobody checked.
#[test]
fn should_not_let_a_moderator_write_what_a_dynamic_target_is_read_from() {
    // A property no reference reads is the moderators' to write
    let note = moderated_note(json!({ "changeFields": ["label"] })).expect("parses");
    assert_eq!(
        note.moderator_changeable_fields(),
        &BTreeSet::from(["label".to_string()])
    );

    for source in [
        // The reference, the contract id, the document type name, and a slot's property name
        "descriptionId",
        "descriptionContractId",
        "descriptionDocumentType",
        "bodyProperty",
    ] {
        let error = moderated_note(json!({ "changeFields": [source] }))
            .expect_err("a moderator must not write what the reference reads");
        let message = error.to_string();
        assert!(
            message.contains(&format!("can not list \"{source}\"")),
            "{source}: {message}"
        );
        assert!(
            message.contains("it holds a reference or is read by one"),
            "{source}: {message}"
        );
    }
}

/// The requirements are written back as a schema declares them, in one place, and the
/// parser reads what is written back to the same requirements: what a client is shown of a
/// declaration is a declaration.
#[test]
fn should_write_requirements_as_a_schema_declares_them() {
    let declared = json!({
        "properties": {
            "title": { "name": "$any", "type": "string", "maxBytes": 256 },
            "body": {
                "name": { "property": "bodyProperty" },
                "type": "string",
                "distinctFrom": ["title"]
            },
            "bytes": { "name": "blob", "type": "array", "byteArray": true },
            "count": { "name": "$any", "type": "integer" }
        },
        "require": { "allOf": ["title", { "anyOf": ["body", "bytes"] }, "count"] }
    });
    let parsed = requirements(declared.clone());
    let written = parsed
        .to_schema_value()
        .try_into_validating_json()
        .expect("the schema value converts to JSON");
    assert_eq!(written, declared);
    assert_eq!(requirements(written), parsed);

    // Every other form: one slot name, `anyOf` over groups, a `distinctFrom` written as
    // one name (which is written back as a list), and no `require`
    for declared in [
        json!({
            "properties": { "body": { "name": "$any", "type": "boolean" } },
            "require": "body"
        }),
        json!({
            "properties": {
                "a": { "name": "$any", "type": "string" },
                "b": { "name": "$any", "type": "string", "distinctFrom": "a" },
                "c": { "name": "$any", "type": "identifier" }
            },
            "require": { "anyOf": [{ "allOf": ["a", "b"] }, "c"] }
        }),
        json!({ "properties": { "body": { "name": "body", "type": "string" } } }),
    ] {
        let parsed = requirements(declared);
        let written = parsed
            .to_schema_value()
            .try_into_validating_json()
            .expect("the schema value converts to JSON");
        assert_eq!(requirements(written), parsed);
    }
}
