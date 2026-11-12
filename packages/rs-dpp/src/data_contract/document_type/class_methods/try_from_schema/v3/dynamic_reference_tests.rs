//! References whose target the referring document names (`contractId` and `documentType`
//! written `"$self.<property>"`, `documentTypeRequirements`, protocol version 15): the parse of
//! the declaration, where it may sit, the checks of the properties it reads, the protocol
//! version gate, what a document type must look like to meet the requirements, a `type`
//! listing both kinds, a slot's `when`, and the platform serialization round trip.

use super::reference_test_helpers::{
    assert_refused, assert_refused_by_the_parser, contract, contract_on, identifier, CONTRACT_ID,
};
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::config::moderation::{
    ContractModerationConfig, ContractModerators, ModerationAbility,
};
use crate::data_contract::config::DataContractConfig;
use crate::data_contract::document_type::accessors::{
    DocumentTypeV0Getters, DocumentTypeV2Getters,
};
use crate::data_contract::document_type::{
    DocumentPropertyReferenceTarget, DocumentPropertyType, DocumentReferenceKind, DocumentType,
    DocumentTypeRequirements, DynamicDocumentReference, DynamicDocumentReferenceKind,
    RequiredPropertySlot, SlotCondition, SlotConditionList, SlotConditionSetting,
    SlotConditionValue, SlotConditions, SlotPropertyName, SlotRequirement, SlotRequirementTerm,
    SlotValue,
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
        "contractId": "$self.descriptionContractId",
        "documentType": "$self.descriptionDocumentType",
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
        "contractId": "$self.descriptionContractId",
        "documentType": "$self.descriptionDocumentType",
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
        .met_by(post, None, |referring_property| {
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
        when: None,
    }
}

/// A dynamic reference to a document of either kind, with `requirements`.
fn either_kind_requiring(requirements: serde_json::Value) -> serde_json::Value {
    json!({
        "type": ["permanentDocument", "moderatedDocument"],
        "contractId": "$self.descriptionContractId",
        "documentType": "$self.descriptionDocumentType",
        "documentTypeRequirements": requirements
    })
}

/// "When the type is moderated, a removal takes the property and its author writes it."
fn removed_with_the_document() -> serde_json::Value {
    json!({
        "ifThen": [
            { "equal": ["$kind", { "const": "moderatedDocument" }] },
            { "allOf": [
                { "not": { "contains": ["$moderatorAbilities.deleteKeepsFields", "$property"] } },
                { "not": { "contains": ["$moderatorAbilities.changeFields", "$property"] } }
            ] }
        ]
    })
}

/// One slot, `body`, any string property `when` holds for.
fn body_when(when: serde_json::Value) -> serde_json::Value {
    json!({ "properties": { "body": { "name": "$any", "type": "string", "when": when } } })
}

/// The `when` of the slot `body` as the parser reads it.
fn parsed_when(when: serde_json::Value) -> SlotCondition {
    requirements(body_when(when))
        .slots
        .remove("body")
        .expect("body")
        .when
        .expect("when")
}

/// A `post` of a contract its owner moderates, holding the strings `text`, `tags` and
/// `label`, with `extra` merged into its schema.
fn moderated_post(extra: serde_json::Value) -> DocumentType {
    let platform_version = PlatformVersion::latest();
    let config = DataContractConfig::default_for_version(platform_version)
        .expect("default config")
        .with_moderation(Some(ContractModerationConfig {
            banlist: false,
            suspensions: false,
            moderators: ContractModerators::ContractOwner,
            warnings: false,
        }));
    let mut schema = json!({
        "type": "object",
        "properties": {
            "text": { "type": "string", "maxLength": 50, "position": 0 },
            "tags": { "type": "string", "maxLength": 50, "position": 1 },
            "label": { "type": "string", "maxLength": 50, "position": 2 }
        },
        "additionalProperties": false
    });
    for (key, value) in extra.as_object().expect("an object") {
        schema[key] = value.clone();
    }
    DocumentType::try_from_schema(
        Identifier::new([1; 32]),
        1,
        config.version(),
        "post",
        platform_value::to_value(schema).expect("the schema should convert"),
        None,
        &BTreeMap::new(),
        &config,
        true,
        &mut vec![],
        platform_version,
    )
    .expect("the post type parses")
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
                when: None,
            }),
        }
    );
    assert_eq!(reference.wire_names(), ["moderatedDocument"]);
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
        "contractId": "$self.descriptionContractId",
        "documentType": "$self.descriptionDocumentType"
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
                "name": "$self.bodyProperty",
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
    // Registration refuses `documentTypeRequirements`, a key it does not know, and the parser
    // of version 14 a `contractId` that is no contract id: `$self.` reads nothing there
    for full_validation in [true, false] {
        contract_on(
            note_contract(text_or_bytes()),
            full_validation,
            platform_version_14,
        )
        .expect_err("protocol version 14 has no dynamic target");
    }
    // which protocol version 15 reads
    contract(note_contract(text_or_bytes())).expect("protocol version 15 reads it");
}

#[test]
fn should_refuse_what_cannot_sit_beside_a_dynamic_target() {
    let with = |key: &str, value: serde_json::Value| {
        let mut refers_to = text_or_bytes();
        refers_to[key] = value;
        note_contract(refers_to)
    };
    for (key, value) in [
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

    // The contract and the document type both come from the document: not one of them
    // from the schema, whether it is given there or left out
    for (key, value) in [
        ("documentType", json!("post")),
        (
            "contractId",
            json!(Identifier::from(CONTRACT_ID).to_string(Encoding::Base58)),
        ),
    ] {
        assert_refused_by_the_parser(
            with(key, value),
            &format!("reads both contractId and documentType from it: {key} must be"),
        );
        let mut missing = text_or_bytes();
        missing.as_object_mut().expect("an object").remove(key);
        assert_refused_by_the_parser(
            note_contract(missing),
            &format!("reads both contractId and documentType from it: {key} must be"),
        );
    }
    // Requirements describe a type the schema does not name: registration refuses them
    // beside one it names (what a stored contract carrying them reads as is tested apart)
    contract(note_contract(json!({
        "type": "permanentDocument",
        "documentType": "post",
        "documentTypeRequirements": {
            "properties": { "body": { "name": "$any", "type": "string" } }
        }
    })))
    .expect_err("documentTypeRequirements beside a target the schema names");
    // What is read is a top-level property, and two different ones
    for read in [
        "$self.",
        "$self.description.type",
        "$self.$ownerId",
        "$self.a-b",
    ] {
        assert_refused_by_the_parser(
            with("documentType", json!(read)),
            "documentType must read a top-level property",
        );
    }
    assert_refused_by_the_parser(
        with("documentType", json!("$self.descriptionContractId")),
        "read the same property",
    );
}

#[test]
fn should_refuse_a_dynamic_target_anywhere_but_on_an_identifier_property() {
    // On a string property
    let mut on_a_string = note_contract(text_or_bytes());
    on_a_string["documentSchemas"]["note"]["properties"]["descriptionId"] =
        json!({ "type": "string", "maxLength": 64, "position": 0, "refersTo": text_or_bytes() });
    // The whole sentence, which travels in a consensus error: one space between its lines
    assert_refused_by_the_parser(
        on_a_string,
        "a refersTo reading its contractId and documentType from the referring document is \
         only allowed on identifier properties: its value is the referenced document's id",
    );

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
        refers_to[key] = json!(format!("$self.{property}"));
        note_contract(refers_to)
    };
    // The contract id is a top-level identifier property, other than the reference
    assert_refused_by_the_parser(
        naming("contractId", "label"),
        "which is not an identifier property",
    );
    assert_refused_by_the_parser(
        naming("contractId", "missing"),
        "which is not a top-level property",
    );
    assert_refused_by_the_parser(
        naming("contractId", "descriptionId"),
        "the property carrying the reference",
    );
    // The document type name is a string property
    assert_refused_by_the_parser(
        naming("documentType", "descriptionId"),
        "the property carrying the reference",
    );
    let mut type_from_an_identifier = note_contract(text_or_bytes());
    type_from_an_identifier["documentSchemas"]["note"]["properties"]["descriptionDocumentType"] =
        identifier(2);
    assert_refused_by_the_parser(type_from_an_identifier, "which is not a string property");
    // and each is required where the reference is
    assert_refused_by_the_parser(
        naming("documentType", "label"),
        "which is not required while the reference is",
    );
    // A slot's name property too
    let named_by = |property: &str| {
        note_contract(requiring(json!({
            "properties": {
                "body": { "name": format!("$self.{property}"), "type": "string" }
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
    let mut optional = naming("documentType", "label");
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

    // require is judged on every write that checks the reference, so it names nothing
    // twice: a term, a slot within a group, or a group with its slots in another order.
    // With at most four slots that leaves it at most fifteen terms
    let three = || json!({ "a": any_string(), "b": any_string(), "c": any_string() });
    refused(
        json!({ "properties": three(), "require": { "allOf": ["a", "b", "c", "a"] } }),
        "names the slot \"a\" twice",
    );
    refused(
        json!({ "properties": three(),
                "require": { "anyOf": ["c", { "allOf": ["a", "b", "a"] }] } }),
        "names a slot in one group twice",
    );
    refused(
        json!({ "properties": three(),
                "require": { "anyOf": ["c", { "allOf": ["a", "b"] }, { "allOf": ["b", "a"] }] } }),
        "names a group of slots twice",
    );
    // and one repeated thousands of times is refused like one repeated once
    let mut many = vec![json!("b"), json!("c")];
    many.extend(std::iter::repeat_n(json!("a"), 5_000));
    refused(
        json!({ "properties": three(), "require": { "allOf": many } }),
        "names the slot \"a\" twice",
    );
    // A slot may still stand alone and in a group
    contract(note_contract(requiring(json!({
        "properties": three(),
        "require": { "anyOf": ["a", { "allOf": ["a", "b"] }, { "allOf": ["b", "c"] }] }
    }))))
    .expect("a slot named alone and in a group parses");
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
    let named = one(json!({ "name": "$self.bodyProperty", "type": "string" }));
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

    // A string bounded both ways is held to the tighter bound, whichever it is: `code`
    // holds at most two characters, eight bytes, though its `maxBytes` allows a hundred,
    // and `tag` at most six bytes, though its `maxLength` allows a hundred characters
    let bounded_both_ways = |name: &str, max_bytes: u32| {
        let mut posts = note_contract(requiring(named(name, max_bytes)));
        let properties = &mut posts["documentSchemas"]["post"]["properties"];
        properties["code"] =
            json!({ "type": "string", "maxLength": 2, "maxBytes": 100, "position": 7 });
        properties["tag"] =
            json!({ "type": "string", "maxLength": 100, "maxBytes": 6, "position": 8 });
        let parsed = contract(posts).expect("parses");
        let post = parsed.document_type_for_name("post").expect("post");
        dynamic_reference(&parsed)
            .requirements
            .expect("requirements")
            .met_by(post, None, |_| None)
    };
    assert!(bounded_both_ways("code", 8));
    assert!(!bounded_both_ways("code", 7));
    assert!(bounded_both_ways("tag", 6));
    assert!(!bounded_both_ways("tag", 5));
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

    // Slots that differ from some of the others, not all: `title` and `body` are the two
    // strings of at most 4096 bytes, so each of four slots takes one of them. With `a`
    // differing from `b`, and `d` from `b` and from `c`, giving them out in turn (`title`,
    // `body`, `title`) leaves `d` nothing, and only taking `c`'s back finds what fills all
    // four: `title`, `body`, `body`, `title`
    let bounded = |distinct_from: serde_json::Value| json!({ "name": "$any", "type": "string", "maxBytes": 4096, "distinctFrom": distinct_from });
    assert!(post_meets(
        json!({ "properties": {
            "a": { "name": "$any", "type": "string", "maxBytes": 4096 },
            "b": bounded(json!("a")),
            "c": { "name": "$any", "type": "string", "maxBytes": 4096 },
            "d": bounded(json!(["b", "c"]))
        } }),
        None
    ));
    // while one more edge, `a` from `d`, leaves three slots that all differ and two strings
    assert!(!post_meets(
        json!({ "properties": {
            "a": bounded(json!("d")),
            "b": bounded(json!("a")),
            "c": { "name": "$any", "type": "string", "maxBytes": 4096 },
            "d": bounded(json!(["b", "c"]))
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
                "name": "$self.bodyProperty",
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

#[test]
fn should_parse_a_type_listing_both_kinds() {
    for kinds in [
        json!(["permanentDocument", "moderatedDocument"]),
        json!(["moderatedDocument", "permanentDocument"]),
    ] {
        let parsed = contract(note_contract(json!({
            "type": kinds,
            "contractId": "$self.descriptionContractId",
            "documentType": "$self.descriptionDocumentType"
        })))
        .expect("parses");
        let reference = dynamic_reference(&parsed);
        assert_eq!(
            reference.kind,
            DynamicDocumentReferenceKind::PermanentOrModerated
        );
        assert_eq!(
            reference.wire_names(),
            ["permanentDocument", "moderatedDocument"]
        );
        assert_eq!(
            DocumentPropertyReferenceTarget::DynamicDocument(reference).to_string(),
            "permanent or moderated document (contract named by descriptionContractId, document \
             type named by descriptionDocumentType)"
        );
    }
    assert_eq!(
        DynamicDocumentReferenceKind::Moderated.wire_names(),
        ["moderatedDocument"]
    );

    // A list is the two kinds, once each
    for kinds in [
        json!(["permanentDocument"]),
        json!(["permanentDocument", "permanentDocument"]),
        json!(["permanentDocument", "deletableDocument"]),
        json!([
            "permanentDocument",
            "moderatedDocument",
            "deletableDocument"
        ]),
        json!(["permanentDocument", 1]),
        json!([]),
    ] {
        assert_refused_by_the_parser(
            note_contract(json!({
                "type": kinds,
                "contractId": "$self.descriptionContractId",
                "documentType": "$self.descriptionDocumentType"
            })),
            "holds permanentDocument and moderatedDocument, once each",
        );
    }
    // and only a reference naming its target through the referring document lists them
    for full_validation in [true, false] {
        contract_on(
            note_contract(json!({
                "type": ["permanentDocument", "moderatedDocument"],
                "documentType": "post"
            })),
            full_validation,
            PlatformVersion::latest(),
        )
        .expect_err("a reference naming its target in the schema is of one kind");
    }
}

#[test]
fn should_check_a_reference_as_the_kind_the_named_type_is() {
    let (permanent, moderated, deletable) = (
        DocumentReferenceKind::Permanent,
        DocumentReferenceKind::Moderated,
        DocumentReferenceKind::Deletable,
    );
    // A declaration of one kind is that kind, whatever the type
    for actual in [permanent, moderated, deletable] {
        assert_eq!(
            DynamicDocumentReferenceKind::Permanent.checked_as(actual),
            permanent
        );
        assert_eq!(
            DynamicDocumentReferenceKind::Moderated.checked_as(actual),
            moderated
        );
    }
    // A declaration of either kind is the kind of the type, and a permanent reference into
    // a type whose documents can be deleted, which the kind check then refuses
    let either = DynamicDocumentReferenceKind::PermanentOrModerated;
    assert_eq!(either.checked_as(permanent), permanent);
    assert_eq!(either.checked_as(moderated), moderated);
    assert_eq!(either.checked_as(deletable), permanent);
}

#[test]
fn should_parse_every_form_of_a_slot_condition() {
    let constant = |text: &str| SlotConditionValue::Constant(text.to_string());
    let list = SlotConditions::new;
    let not_listed = |listed: SlotConditionList| {
        SlotCondition::Not(list(vec![SlotCondition::Contains(
            listed,
            SlotConditionValue::PropertyName,
        )]))
    };

    assert_eq!(
        parsed_when(removed_with_the_document()),
        SlotCondition::IfThen(list(vec![
            SlotCondition::Equal(SlotConditionValue::Kind, constant("moderatedDocument")),
            SlotCondition::AllOf(list(vec![
                not_listed(SlotConditionList::DeletionKeptFields),
                not_listed(SlotConditionList::ModeratorChangeableFields),
            ])),
        ]))
    );

    // A comparison reads either way round
    assert_eq!(
        parsed_when(json!({ "notEqual": [{ "const": "permanentDocument" }, "$kind"] })),
        SlotCondition::NotEqual(constant("permanentDocument"), SlotConditionValue::Kind)
    );
    assert_eq!(
        parsed_when(json!({ "in": ["$property", ["body", "summary"]] })),
        SlotCondition::In(
            SlotConditionValue::PropertyName,
            BTreeSet::from(["body".to_string(), "summary".to_string()])
        )
    );
    assert_eq!(
        parsed_when(json!({ "notIn": ["$kind", ["moderatedDocument", "permanentDocument"]] })),
        SlotCondition::NotIn(
            SlotConditionValue::Kind,
            BTreeSet::from([
                "moderatedDocument".to_string(),
                "permanentDocument".to_string()
            ])
        )
    );
    assert_eq!(
        parsed_when(json!({
            "contains": ["$moderatorAbilities.changeFields", { "const": "label" }]
        })),
        SlotCondition::Contains(
            SlotConditionList::ModeratorChangeableFields,
            constant("label")
        )
    );
    assert_eq!(
        parsed_when(json!({ "ifThenElse": [
            { "equal": ["$kind", { "const": "permanentDocument" }] },
            { "anyOf": [
                { "equal": ["$property", { "const": "body" }] },
                { "equal": ["$property", { "const": "summary" }] }
            ] },
            { "notEqual": ["$property", { "const": "title" }] }
        ] })),
        SlotCondition::IfThenElse(list(vec![
            SlotCondition::Equal(SlotConditionValue::Kind, constant("permanentDocument")),
            SlotCondition::AnyOf(list(vec![
                SlotCondition::Equal(SlotConditionValue::PropertyName, constant("body")),
                SlotCondition::Equal(SlotConditionValue::PropertyName, constant("summary")),
            ])),
            SlotCondition::NotEqual(SlotConditionValue::PropertyName, constant("title")),
        ]))
    );
}

#[test]
fn should_refuse_a_malformed_slot_condition() {
    let refused = |when: serde_json::Value, fragment: &str| {
        assert_refused_by_the_parser(note_contract(requiring(body_when(when))), fragment);
    };
    let is_body = || json!({ "equal": ["$property", { "const": "body" }] });

    // The shape of a condition
    refused(json!("$property"), "is an object of one key");
    refused(json!({}), "is an object of one key");
    refused(
        json!({ "equal": ["$property", { "const": "a" }], "not": is_body() }),
        "is an object of one key",
    );
    refused(json!({ "present": "$property" }), "when present tests");
    refused(
        json!({ "startsWith": ["$property", { "const": "a" }] }),
        "does not take startsWith",
    );
    refused(
        json!({ "greaterThan": ["$property", { "const": "a" }] }),
        "does not take greaterThan",
    );

    // What a comparison reads
    refused(json!({ "equal": ["$property"] }), "equal takes two strings");
    refused(
        json!({ "equal": ["title", { "const": "title" }] }),
        "when compares \"$kind\", \"$property\" or",
    );
    // `$self` is the referring document, which a condition on the referenced type does not
    // read: the property tried for the slot is `$property`
    refused(
        json!({ "equal": ["$self", { "const": "body" }] }),
        "when compares \"$kind\", \"$property\" or",
    );
    refused(
        json!({ "contains": ["$moderatorAbilities.changeFields", "$self.bodyProperty"] }),
        "when compares \"$kind\", \"$property\" or",
    );
    refused(
        json!({ "equal": ["$property", { "const": "" }] }),
        "when compares \"$kind\", \"$property\" or",
    );
    refused(
        json!({ "equal": ["$property", { "const": 1 }] }),
        "when compares \"$kind\", \"$property\" or",
    );
    refused(
        json!({ "equal": ["$property", { "const": "a", "other": "b" }] }),
        "when compares \"$kind\", \"$property\" or",
    );
    refused(
        json!({ "equal": ["$property", "$kind"] }),
        "equal compares \"$kind\" or \"$property\" with a",
    );
    refused(
        json!({ "notEqual": [{ "const": "a" }, { "const": "b" }] }),
        "notEqual compares \"$kind\" or \"$property\" with a",
    );
    refused(
        json!({ "equal": ["$property", "$property"] }),
        "equal compares \"$kind\" or \"$property\" with a",
    );
    // The kind is one a reference of this sort admits
    refused(
        json!({ "equal": ["$kind", { "const": "moderated" }] }),
        "the kind of the referenced document type is permanentDocument or moderatedDocument",
    );
    refused(
        json!({ "equal": ["$kind", { "const": "deletableDocument" }] }),
        "the kind of the referenced document type is permanentDocument or moderatedDocument",
    );
    refused(
        json!({ "in": ["$kind", ["permanentDocument", "post"]] }),
        "the kind of the referenced document type is permanentDocument or moderatedDocument",
    );

    // `in` and `notIn`
    refused(
        json!({ "in": ["$property", ["body"]] }),
        "in takes a string and a list of two or more distinct strings",
    );
    refused(
        json!({ "notIn": ["$property", ["body", "body"]] }),
        "notIn takes a string and a list of two or more distinct strings",
    );
    refused(
        json!({ "in": ["$property", ["body", 1]] }),
        "in takes a string and a list of two or more distinct strings",
    );
    refused(
        json!({ "in": [{ "const": "body" }, ["body", "title"]] }),
        "not for a constant",
    );

    // `contains`
    refused(
        json!({ "contains": ["$property", "$property"] }),
        "contains looks into \"$moderatorAbilities.deleteKeepsFields\",",
    );
    refused(
        json!({ "contains": ["$moderatorAbilities.delete", "$property"] }),
        "contains looks into \"$moderatorAbilities.deleteKeepsFields\",",
    );
    refused(
        json!({ "contains": ["$moderatorAbilities.changeFields", "$kind"] }),
        "not for the kind of the type",
    );

    // The combinators
    refused(
        json!({ "not": { "not": is_body() } }),
        "not holds a not or a notIn",
    );
    refused(
        json!({ "not": { "notIn": ["$property", ["a", "b"]] } }),
        "not holds a not or a notIn",
    );
    refused(
        json!({ "anyOf": [is_body()] }),
        "anyOf takes two or more conditions",
    );
    refused(
        json!({ "allOf": [is_body(), is_body()] }),
        "allOf lists the same condition twice",
    );
    refused(
        json!({ "anyOf": [is_body(), { "anyOf": [
            { "equal": ["$property", { "const": "a" }] },
            { "equal": ["$property", { "const": "b" }] }
        ] }] }),
        "anyOf holds another anyOf",
    );
    refused(
        json!({ "ifThen": [is_body()] }),
        "ifThen takes two conditions",
    );
    refused(
        json!({ "ifThenElse": [is_body(), is_body()] }),
        "ifThenElse takes three conditions",
    );
}

#[test]
fn should_hold_a_slot_condition_to_the_node_limit() {
    let max_nodes = usize::from(
        PlatformVersion::latest()
            .system_limits
            .max_document_type_requirement_condition_nodes,
    );
    // `allOf` over `count - 1` comparisons: `count` conditions
    let of = |count: usize| {
        json!({ "allOf": (1..count)
            .map(|index| json!({ "notEqual": ["$property", { "const": format!("p{index}") }] }))
            .collect::<Vec<_>>() })
    };
    assert_eq!(parsed_when(of(max_nodes)).node_count(), max_nodes);
    assert_refused_by_the_parser(
        note_contract(requiring(body_when(of(max_nodes + 1)))),
        &format!("when is made of more than {max_nodes} conditions"),
    );

    // The limit also holds what nests: `not` and `ifThen` in turn, as deep as it allows
    let mut deepest = json!({ "equal": ["$property", { "const": "body" }] });
    let mut nodes = 1;
    while nodes * 2 + 2 <= max_nodes {
        deepest = json!({ "not": { "ifThen": [deepest.clone(), deepest] } });
        nodes = nodes * 2 + 2;
    }
    assert!(parsed_when(deepest.clone()).node_count() <= max_nodes);
    let too_many = json!({ "ifThen": [deepest.clone(), deepest] });
    assert_refused_by_the_parser(
        note_contract(requiring(body_when(too_many))),
        &format!("when is made of more than {max_nodes} conditions"),
    );
}

#[test]
fn should_fill_a_slot_only_with_a_property_its_condition_holds_for() {
    // The fixture's `post` is permanent: `title`, `body` and `summary` are its strings
    assert!(post_meets(
        body_when(json!({ "in": ["$property", ["body", "content"]] })),
        None
    ));
    assert!(!post_meets(
        body_when(json!({ "in": ["$property", ["content", "text"]] })),
        None
    ));
    // A condition on the kind decides for every property at once
    assert!(post_meets(
        body_when(json!({ "equal": ["$kind", { "const": "permanentDocument" }] })),
        None
    ));
    assert!(!post_meets(
        body_when(json!({ "equal": ["$kind", { "const": "moderatedDocument" }] })),
        None
    ));
    // The condition narrows the candidates distinct slots are filled from: three strings,
    // of which the two slots may not take `summary`
    let two_of = |excluded: &[&str]| {
        let when = json!({ "notIn": ["$property", excluded] });
        json!({ "properties": {
            "first": { "name": "$any", "type": "string", "when": when },
            "second": { "name": "$any", "type": "string", "when": when, "distinctFrom": "first" }
        } })
    };
    assert!(post_meets(two_of(&["summary", "none"]), None));
    assert!(!post_meets(two_of(&["summary", "body"]), None));
    // A named property is held to it too
    assert!(!post_meets(
        json!({ "properties": { "body": {
            "name": "title",
            "type": "string",
            "when": { "notEqual": ["$property", { "const": "title" }] }
        } } }),
        None
    ));
    assert!(!post_meets(
        json!({ "properties": { "body": {
            "name": "$self.bodyProperty",
            "type": "string",
            "when": { "notEqual": ["$property", { "const": "title" }] }
        } } }),
        Some("title")
    ));
}

#[test]
fn should_ask_of_a_moderated_type_what_it_does_not_ask_of_a_permanent_one() {
    let slot_requirements = |name: serde_json::Value| {
        dynamic_reference(
            &contract(note_contract(either_kind_requiring(
                json!({ "properties": { "body": {
                "name": name,
                "type": "string",
                "when": removed_with_the_document()
            } } }),
            )))
            .expect("parses"),
        )
        .requirements
        .expect("requirements")
    };
    let met = |name: serde_json::Value, post: &DocumentType| {
        slot_requirements(name).met_by(post.as_ref(), None, |_| None)
    };

    // Moderators delete posts, the removal record keeping `tags`, and they alone write
    // `label`: `text` is the one string a removal takes and the author writes
    let moderated = moderated_post(json!({
        "canBeDeleted": false,
        "moderatorAbilities": {
            "delete": true,
            "deleteKeepsFields": ["tags"],
            "changeFields": ["label"]
        }
    }));
    assert_eq!(
        moderated.document_reference_kind(),
        DocumentReferenceKind::Moderated
    );
    assert!(met(json!("$any"), &moderated));
    assert!(met(json!("text"), &moderated));
    assert!(!met(json!("tags"), &moderated));
    assert!(!met(json!("label"), &moderated));

    // Every string is kept or is the moderators': nothing fills the slot
    let all_kept = moderated_post(json!({
        "canBeDeleted": false,
        "moderatorAbilities": {
            "delete": true,
            "deleteKeepsFields": ["text", "tags"],
            "changeFields": ["label"]
        }
    }));
    assert!(!met(json!("$any"), &all_kept));

    // A permanent type whose moderators write `label`: the condition asks nothing of it
    let permanent = moderated_post(json!({
        "canBeDeleted": false,
        "documentsMutable": false,
        "moderatorAbilities": { "changeFields": ["label"] }
    }));
    assert_eq!(
        permanent.document_reference_kind(),
        DocumentReferenceKind::Permanent
    );
    assert!(met(json!("label"), &permanent));
    assert!(met(json!("tags"), &permanent));
}

#[test]
fn should_round_trip_a_contract_through_platform_serialization_with_a_slot_condition() {
    let platform_version = PlatformVersion::latest();
    let original = contract(note_contract(either_kind_requiring(json!({
        "properties": {
            "body": { "name": "$any", "type": "string", "when": removed_with_the_document() },
            "bytes": {
                "name": "$any",
                "type": "array",
                "byteArray": true,
                "when": { "ifThenElse": [
                    { "in": ["$kind", ["permanentDocument", "moderatedDocument"]] },
                    { "notIn": ["$property", ["blob", "raw"]] },
                    { "notEqual": ["$property", { "const": "blob" }] }
                ] }
            }
        },
        "require": { "anyOf": ["body", "bytes"] }
    }))))
    .expect("parses");
    let bytes = original
        .serialize_to_bytes_with_platform_version(platform_version)
        .expect("the contract should serialize");
    let recovered = DataContract::versioned_deserialize_untrusted(&bytes, false, platform_version)
        .expect("the contract should deserialize");
    assert_eq!(original, recovered);
    let reference = dynamic_reference(&recovered);
    assert_eq!(
        reference.kind,
        DynamicDocumentReferenceKind::PermanentOrModerated
    );
    assert_eq!(dynamic_reference(&original), reference);
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
            "body": { "name": "$self.bodyProperty", "type": "string" }
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
                "name": "$self.bodyProperty",
                "type": "string",
                "distinctFrom": ["title"]
            },
            "bytes": {
                "name": "blob",
                "type": "array",
                "byteArray": true,
                "when": { "ifThenElse": [
                    { "in": ["$kind", ["moderatedDocument", "permanentDocument"]] },
                    { "notIn": ["$property", ["blob", "raw"]] },
                    { "notEqual": [{ "const": "blob" }, "$property"] }
                ] }
            },
            "count": {
                "name": "$any",
                "type": "integer",
                "when": { "ifThen": [
                    { "equal": ["$kind", { "const": "moderatedDocument" }] },
                    { "allOf": [
                        { "not": { "contains": ["$moderatorAbilities.deleteKeepsFields", "$property"] } },
                        { "anyOf": [
                            { "contains": ["$moderatorAbilities.changeFields", { "const": "label" }] },
                            { "notEqual": ["$property", { "const": "count" }] }
                        ] }
                    ] }
                ] }
            }
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

/// A contract stored before protocol version 15 is read as the version that stored it read
/// it. That version did not read `documentTypeRequirements`, and through a key its schema
/// check did not see (a root `properties` given twice: the check read the last entry, the
/// parser the first) a stored declaration naming its target in the schema may carry one.
/// It is still that reference: only `$self.` selects the reading this version adds, and no
/// stored declaration is written with it.
#[test]
fn should_read_a_stored_reference_naming_its_target_in_the_schema_as_it_was_registered() {
    let platform_version_14 = PlatformVersion::get(14).expect("protocol version 14");
    let text = |value: &str| platform_value::Value::Text(value.to_string());
    let reference = |stray: bool| {
        let mut refers_to = json!({ "type": "permanentDocument", "documentType": "post" });
        if stray {
            refers_to["documentTypeRequirements"] = json!({
                "properties": { "body": { "name": "$any", "type": "string" } }
            });
        }
        let mut description_id = identifier(0);
        description_id["refersTo"] = refers_to;
        platform_value::to_value(json!({ "descriptionId": description_id }))
            .expect("the properties should convert")
    };
    // `properties` twice: the first entry carries the stray key, the last does not
    let schema = platform_value::Value::Map(vec![
        (text("type"), text("object")),
        (text("properties"), reference(true)),
        (text("properties"), reference(false)),
        (
            text("additionalProperties"),
            platform_value::Value::Bool(false),
        ),
    ]);
    let parse = |full_validation: bool, platform_version: &PlatformVersion| {
        let config =
            DataContractConfig::default_for_version(platform_version).expect("default config");
        DocumentType::try_from_schema(
            Identifier::new([1; 32]),
            1,
            config.version(),
            "note",
            schema.clone(),
            None,
            &BTreeMap::new(),
            &config,
            full_validation,
            &mut vec![],
            platform_version,
        )
    };
    let target = |document_type: &DocumentType| {
        document_type
            .properties()
            .get("descriptionId")
            .expect("descriptionId")
            .property_type
            .clone()
    };

    // Protocol version 14 registers it, reading the reference of the first entry
    let registered = parse(true, platform_version_14).expect("protocol version 14 registers it");
    assert!(matches!(
        target(&registered),
        DocumentPropertyType::IdentifierWithReference(
            DocumentPropertyReferenceTarget::PermanentDocument { .. }
        )
    ));
    // and this version, loading what was stored, reads the same reference
    let loaded = parse(false, PlatformVersion::latest()).expect("the stored schema still loads");
    assert_eq!(target(&loaded), target(&registered));
    // though it registers no such schema itself
    parse(true, PlatformVersion::latest())
        .expect_err("registration refuses the repeated key and the stray one");

    // A whole contract stored that way loads too
    let stored = contract_on(
        note_contract(json!({
            "type": "permanentDocument",
            "documentType": "post",
            "documentTypeRequirements": {
                "properties": { "body": { "name": "$any", "type": "string" } }
            }
        })),
        false,
        platform_version_14,
    )
    .expect("parsed as protocol version 14 stored it");
    let bytes = stored
        .serialize_to_bytes_with_platform_version(platform_version_14)
        .expect("the contract should serialize");
    let recovered =
        DataContract::versioned_deserialize_untrusted(&bytes, false, PlatformVersion::latest())
            .expect("the stored contract loads at the latest protocol version");
    let note = |contract: &DataContract| {
        contract
            .document_type_for_name("note")
            .expect("note")
            .properties()["descriptionId"]
            .property_type
            .clone()
    };
    assert_eq!(note(&recovered), note(&stored));
    assert!(matches!(
        note(&recovered),
        DocumentPropertyType::IdentifierWithReference(
            DocumentPropertyReferenceTarget::PermanentDocument { .. }
        )
    ));
}

/// A `when` is held to its nodes and to the length of its strings, so what one adds to a
/// stored contract is bounded: each string an `in` or a `notIn` lists counts as a node, and
/// no string is longer than a property path.
#[test]
fn should_bound_what_a_slot_condition_lists() {
    let limits = &PlatformVersion::latest().system_limits;
    let max_nodes = usize::from(limits.max_document_type_requirement_condition_nodes);
    let max_length = usize::from(limits.max_document_type_requirement_condition_text_length);
    let listing = |key: &str, count: usize| json!({ key: ["$property", (0..count).map(|index| format!("p{index}")).collect::<Vec<_>>()] });
    let refused = |when: serde_json::Value, fragment: &str| {
        assert_refused_by_the_parser(note_contract(requiring(body_when(when))), fragment);
    };

    // The list and the condition holding it: as many strings as the limit less one
    for key in ["in", "notIn"] {
        assert_eq!(
            parsed_when(listing(key, max_nodes - 1)).node_count(),
            max_nodes
        );
        refused(
            listing(key, max_nodes),
            &format!("when is made of more than {max_nodes} conditions, counting each string"),
        );
        // however many: the list is not read past the limit
        refused(
            listing(key, 50_000),
            &format!("when is made of more than {max_nodes} conditions, counting each string"),
        );
    }
    // The strings count with the conditions around them
    refused(
        json!({ "allOf": [
            listing("in", max_nodes / 2),
            listing("notIn", max_nodes / 2)
        ] }),
        &format!("when is made of more than {max_nodes} conditions"),
    );

    // A string is as long as a property path may be, and no longer
    let long = |length: usize| "p".repeat(length);
    parsed_when(json!({ "equal": ["$property", { "const": long(max_length) }] }));
    refused(
        json!({ "equal": ["$property", { "const": long(max_length + 1) }] }),
        &format!("a string of 1 to {max_length} characters"),
    );
    parsed_when(json!({ "in": ["$property", [long(max_length), "body"]] }));
    refused(
        json!({ "in": ["$property", [long(max_length + 1), "body"]] }),
        &format!("strings of 1 to {max_length} characters"),
    );
    refused(
        json!({ "contains": ["$moderatorAbilities.changeFields", { "const": long(max_length + 1) }] }),
        &format!("a string of 1 to {max_length} characters"),
    );
}

/// Judging a type is billed by the comparisons it may take: each property a slot tries,
/// and each node of the slot's `when` for it. The count reads the declaration and the size
/// of the type, never what the type's properties are.
#[test]
fn should_count_the_comparisons_judging_a_type_takes() {
    let parsed = |requirements_value: serde_json::Value| {
        let parsed = contract(note_contract(requiring(requirements_value))).expect("parses");
        let requirements = dynamic_reference(&parsed)
            .requirements
            .expect("requirements");
        let post = parsed.document_type_for_name("post").expect("post");
        (
            requirements.comparisons(post),
            post.properties().len() as u64,
        )
    };

    // Any property: each of the type's is tried, whatever it holds
    let (comparisons, properties) =
        parsed(json!({ "properties": { "body": { "name": "$any", "type": "boolean" } } }));
    assert_eq!(properties, 7);
    assert_eq!(comparisons, properties);
    // A named one: one, and one read from the document: one
    let (comparisons, _) = parsed(json!({ "properties": {
        "body": { "name": "body", "type": "string" },
        "other": { "name": "$self.bodyProperty", "type": "string" }
    } }));
    assert_eq!(comparisons, 2);
    // A `when` adds its nodes for each property tried, listed strings among them
    let (comparisons, properties) = parsed(json!({ "properties": {
        "body": {
            "name": "$any",
            "type": "string",
            "when": { "notIn": ["$property", ["title", "summary", "label"]] }
        },
        "named": {
            "name": "blob",
            "type": "array",
            "byteArray": true,
            "when": { "not": { "equal": ["$property", { "const": "raw" }] } }
        }
    } }));
    assert_eq!(comparisons, properties * (1 + 4) + (1 + 2));
}

/// The requirements may carry a `when` of their own, judged once for the named type as a
/// whole: the grammar of a slot's `when` with no property to read, over what the type's
/// contract lets its elected team do on it (`$electedAbilities`) and the settings the type
/// declares (`present`, `absent`).
#[test]
fn should_parse_a_condition_on_the_type_as_a_whole() {
    let removable = || {
        json!({ "allOf": [
            { "contains": ["$electedAbilities", { "const": "deleteDocuments" }] },
            { "anyOf": [
                { "absent": "$moderatorAbilities.deleteWithin" },
                { "present": "$moderatorAbilities.deleteSettled" }
            ] }
        ] })
    };
    let with_when = |when: serde_json::Value| {
        json!({
            "when": when,
            "properties": { "body": { "name": "$any", "type": "string" } }
        })
    };
    let parsed = requirements(with_when(removable()));
    assert_eq!(
        parsed.when,
        Some(SlotCondition::AllOf(SlotConditions::new(vec![
            SlotCondition::Contains(
                SlotConditionList::ElectedAbilities,
                SlotConditionValue::Constant("deleteDocuments".to_string())
            ),
            SlotCondition::AnyOf(SlotConditions::new(vec![
                SlotCondition::Absent(SlotConditionSetting::DeleteWithin),
                SlotCondition::Present(SlotConditionSetting::DeleteSettled),
            ])),
        ])))
    );
    // Written back as declared, which parses to the same requirements
    let written = parsed
        .to_schema_value()
        .try_into_validating_json()
        .expect("the schema value converts to JSON");
    assert_eq!(written, with_when(removable()));
    assert_eq!(requirements(written), parsed);
    // and it survives the platform serialization of its contract
    let platform_version = PlatformVersion::latest();
    let original = contract(note_contract(requiring(with_when(removable())))).expect("parses");
    let bytes = original
        .serialize_to_bytes_with_platform_version(platform_version)
        .expect("the contract should serialize");
    let recovered = DataContract::versioned_deserialize_untrusted(&bytes, false, platform_version)
        .expect("the contract should deserialize");
    assert_eq!(dynamic_reference(&original), dynamic_reference(&recovered));

    // The same facts are a slot's to read too, beside the property
    requirements(json!({ "properties": { "body": {
        "name": "$any",
        "type": "string",
        "when": { "ifThen": [
            { "present": "$moderatorAbilities.deleteWithin" },
            { "notEqual": ["$property", { "const": "title" }] }
        ] }
    } } }));

    let refused = |when: serde_json::Value, fragment: &str| {
        assert_refused_by_the_parser(note_contract(requiring(with_when(when))), fragment);
    };
    // No property is in hand for the type as a whole
    refused(
        json!({ "notEqual": ["$property", { "const": "title" }] }),
        "is judged for the document type as a whole and reads no \"$property\"",
    );
    refused(
        json!({ "anyOf": [
            removable(),
            { "not": { "contains": ["$moderatorAbilities.changeFields", "$property"] } }
        ] }),
        "is judged for the document type as a whole and reads no \"$property\"",
    );
    // An ability is one a contract can give, named by a constant
    refused(
        json!({ "contains": ["$electedAbilities", { "const": "delete" }] }),
        "for a { \"const\": ... } naming an ability: deleteDocuments, ban, suspend, warn, \
         changeDocumentFields",
    );
    refused(
        json!({ "contains": ["$electedAbilities", "$kind"] }),
        "naming an ability",
    );
    // and a setting is one of the two that say how long a document stays deletable
    refused(
        json!({ "absent": "$moderatorAbilities.delete" }),
        "when absent tests \"$moderatorAbilities.deleteWithin\" or",
    );
    refused(
        json!({ "present": ["$moderatorAbilities.deleteWithin"] }),
        "when present tests",
    );
    // It is held to the limits of a slot's
    let max_nodes = usize::from(
        platform_version
            .system_limits
            .max_document_type_requirement_condition_nodes,
    );
    refused(
        json!({ "anyOf": (0..max_nodes)
            .map(|index| json!({ "notEqual": ["$kind", { "const": if index % 2 == 0 { "permanentDocument" } else { "moderatedDocument" } }] }))
            .collect::<Vec<_>>() }),
        &format!("documentTypeRequirements when is made of more than {max_nodes} conditions"),
    );
    refused(
        json!({ "in": ["$kind", ["permanentDocument", "moderatedDocument"]], "extra": 1 }),
        "is an object of one key",
    );
}

/// A type meets a condition on it by what its contract and its own moderation say: the
/// elected team's abilities come from the contract, the window and the settled deletion
/// from the type. The judging counts the condition's nodes once.
#[test]
fn should_judge_the_named_type_by_its_moderation() {
    let parsed = requirements(json!({
        "when": { "allOf": [
            { "contains": ["$electedAbilities", { "const": "deleteDocuments" }] },
            { "absent": "$moderatorAbilities.deleteWithin" }
        ] },
        "properties": { "body": { "name": "$any", "type": "string" } }
    }));
    let deleting = BTreeSet::from([ModerationAbility::DeleteDocuments]);
    let banning = BTreeSet::from([ModerationAbility::Ban, ModerationAbility::Suspend]);
    let met = |post: &DocumentType, abilities: Option<&BTreeSet<ModerationAbility>>| {
        parsed.met_by(post.as_ref(), abilities, |_| None)
    };

    // Moderators delete posts, at any time
    let durable = moderated_post(json!({
        "canBeDeleted": false,
        "moderatorAbilities": { "delete": true }
    }));
    assert!(met(&durable, Some(&deleting)));
    // The contract elects no team, leaves the type out, or gives the team other abilities
    assert!(!met(&durable, None));
    assert!(!met(&durable, Some(&BTreeSet::new())));
    assert!(!met(&durable, Some(&banning)));

    // Moderators delete posts for an hour after their last change, and nobody after
    let windowed = moderated_post(json!({
        "canBeDeleted": false,
        "required": ["$createdAt", "$updatedAt"],
        "moderatorAbilities": { "delete": true, "deleteWithin": 3600 }
    }));
    assert!(windowed
        .documents_can_be_deleted_by_moderators_for()
        .is_some());
    assert!(!met(&windowed, Some(&deleting)));

    // Three string properties tried for the slot, and the three nodes of the condition
    assert_eq!(parsed.comparisons(durable.as_ref()), 3 + 3);
}
