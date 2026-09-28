//! Lookups with a computed key (`refersTo.lookup.keys: { "sha256d": [...] }`,
//! protocol version 14), a commit and reveal: the parse of the key, its
//! `minimumAgeSeconds` and `consume`, the checks of the referring side on
//! every parse, the checks of the referenced side at contract level, and the
//! protocol version gate.

use super::reference_test_helpers::{assert_refused, contract, contract_on, CONTRACT_ID};
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::{
    DocumentPropertyReferenceTarget, DocumentPropertyType, DocumentReferenceLookup, LookupHashKey,
    LookupKeyHash, LookupKeySource, LookupPreimagePart, ReferenceHolder,
};
use crate::data_contract::DataContract;
use platform_value::string_encoding::Encoding;
use platform_value::Identifier;
use platform_version::version::PlatformVersion;
use serde_json::json;
use std::collections::BTreeMap;

/// The DPNS preorder hash: `preorderSalt ++ label` for a top-level name,
/// `preorderSalt ++ normalizedLabel ++ "." ++ parentDomainName` otherwise.
fn dpns_preimage() -> serde_json::Value {
    json!([
        { "property": "preorderSalt" },
        {
            "ifEmpty": "parentDomainName",
            "then": [{ "property": "label" }],
            "else": [
                { "property": "normalizedLabel" },
                { "text": "." },
                { "property": "parentDomainName" }
            ]
        }
    ])
}

/// The `creatorRefersTo` of a DPNS-shaped `domain`, with `lookup_extra` merged
/// into its lookup and `declaration_extra` into the declaration.
fn domain_reveal(
    lookup_extra: serde_json::Value,
    declaration_extra: serde_json::Value,
) -> serde_json::Value {
    let mut lookup = json!({
        "index": "saltedHash",
        "keys": { "saltedDomainHash": { "sha256d": dpns_preimage() } }
    });
    merge(&mut lookup, lookup_extra);
    let mut declaration = json!({
        "type": "deletableDocument",
        "documentType": "preorder",
        "lookup": lookup
    });
    merge(&mut declaration, declaration_extra);
    declaration
}

fn merge(target: &mut serde_json::Value, extra: serde_json::Value) {
    if let serde_json::Value::Object(entries) = extra {
        for (key, value) in entries {
            target[key] = value;
        }
    }
}

/// The DPNS-shaped preorder: immutable, deletable by its owner, recording
/// `$createdAt`, unique on its 32-byte `saltedDomainHash`.
fn preorder_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "documentsMutable": false,
        "canBeDeleted": true,
        "properties": {
            "saltedDomainHash": {
                "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32, "position": 0
            }
        },
        "indices": [
            { "name": "saltedHash", "properties": [{ "saltedDomainHash": "asc" }], "unique": true }
        ],
        "required": ["$createdAt", "saltedDomainHash"],
        "additionalProperties": false
    })
}

/// The DPNS-shaped domain: immutable and transferable (so it records creator
/// ids), its salt transient and its parent optional, as in DPNS.
fn domain_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "documentsMutable": false,
        "canBeDeleted": false,
        "transferable": 1,
        "properties": {
            "label": { "type": "string", "maxLength": 63, "position": 0 },
            "normalizedLabel": { "type": "string", "maxLength": 63, "position": 1 },
            "parentDomainName": { "type": "string", "maxLength": 63, "position": 2 },
            "preorderSalt": {
                "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32, "position": 3
            },
            "count": { "type": "integer", "minimum": 0, "maximum": 100, "position": 4 }
        },
        "required": ["label", "normalizedLabel", "preorderSalt"],
        "transient": ["preorderSalt"],
        "additionalProperties": false
    })
}

/// A contract with `preorder` and a `domain` declaring `creator_refers_to`,
/// with `domain_extra` merged into the domain and `preorder_extra` into the
/// preorder.
fn dpns_contract_with(
    creator_refers_to: serde_json::Value,
    domain_extra: serde_json::Value,
    preorder_extra: serde_json::Value,
) -> serde_json::Value {
    let mut domain = domain_schema();
    domain["creatorRefersTo"] = creator_refers_to;
    merge(&mut domain, domain_extra);
    let mut preorder = preorder_schema();
    merge(&mut preorder, preorder_extra);
    json!({
        "$formatVersion": "1",
        "id": Identifier::from(CONTRACT_ID).to_string(Encoding::Base58),
        "ownerId": Identifier::from([8; 32]).to_string(Encoding::Base58),
        "version": 1,
        "documentSchemas": { "preorder": preorder, "domain": domain }
    })
}

fn dpns_contract(creator_refers_to: serde_json::Value) -> serde_json::Value {
    dpns_contract_with(creator_refers_to, json!({}), json!({}))
}

fn creator_reference(contract: &DataContract) -> DocumentPropertyReferenceTarget {
    contract
        .document_type_for_name("domain")
        .expect("the domain document type")
        .reference_declarations()
        .find_map(|(holder, reference)| {
            (holder == ReferenceHolder::Creator).then(|| reference.target().cloned())
        })
        .flatten()
        .expect("the domain's creator reference")
}

#[test]
fn should_parse_the_dpns_preorder_as_a_computed_key_with_its_age_consumption_and_owner_pair() {
    let parsed = contract(dpns_contract(domain_reveal(
        json!({ "minimumAgeSeconds": 60, "consume": true }),
        json!({ "propertyAgreement": { "$ownerId": "$ownerId" } }),
    )))
    .expect("the DPNS reveal should parse");

    let preimage = vec![
        LookupPreimagePart::Property("preorderSalt".to_string()),
        LookupPreimagePart::IfEmpty {
            property: "parentDomainName".to_string(),
            then: vec![LookupPreimagePart::Property("label".to_string())],
            otherwise: vec![
                LookupPreimagePart::Property("normalizedLabel".to_string()),
                LookupPreimagePart::Text(".".to_string()),
                LookupPreimagePart::Property("parentDomainName".to_string()),
            ],
        },
    ];
    assert_eq!(
        creator_reference(&parsed),
        DocumentPropertyReferenceTarget::DeletableDocumentLookup {
            contract_id: None,
            document_type_name: "preorder".to_string(),
            property_agreement: BTreeMap::from([("$ownerId".to_string(), "$ownerId".to_string())]),
            lookup: DocumentReferenceLookup {
                index: "saltedHash".to_string(),
                keys: BTreeMap::from([(
                    "saltedDomainHash".to_string(),
                    LookupKeySource::Hash(LookupHashKey {
                        hash: LookupKeyHash::Sha256d,
                        preimage,
                    }),
                )]),
                minimum_age_seconds: Some(60),
                consume: true,
            },
        }
    );
}

#[test]
fn should_parse_a_computed_key_that_demands_nothing_more() {
    // No owner pair, no minimum age, no consume: each is optional
    let parsed = contract(dpns_contract(domain_reveal(json!({}), json!({}))))
        .expect("a bare computed key should parse");
    let DocumentPropertyReferenceTarget::DeletableDocumentLookup { lookup, .. } =
        creator_reference(&parsed)
    else {
        panic!("expected a deletable lookup");
    };
    assert!(lookup.is_checked_on_create_only());
    assert_eq!(lookup.minimum_age_seconds, None);
    assert!(!lookup.consume);
}

#[test]
fn should_refuse_a_deletable_creator_lookup_without_a_computed_key() {
    // The creator's gate on a deletable document would outlive a transfer:
    // only a computed key, judged on the create alone, is admitted
    let contract_value = dpns_contract(json!({
        "type": "deletableDocument",
        "documentType": "preorder",
        "lookup": { "index": "saltedHash", "keys": { "saltedDomainHash": "." } }
    }));
    assert_refused(
        contract(contract_value),
        "does not take a deletableDocument reference unless its lookup key is computed",
    );
}

#[test]
fn should_refuse_malformed_computed_keys_in_the_parser() {
    let with_key = |key: serde_json::Value| {
        dpns_contract(json!({
            "type": "deletableDocument",
            "documentType": "preorder",
            "lookup": { "index": "saltedHash", "keys": { "saltedDomainHash": key } }
        }))
    };
    for (key, fragment) in [
        (json!({ "sha256": [{ "property": "label" }] }), "sha256"),
        (
            json!({ "sha256d": [{ "property": "label" }], "sha512": [] }),
            "saltedDomainHash",
        ),
        (json!({ "sha256d": [] }), "sha256d"),
        (json!({ "sha256d": [{ "text": "" }] }), "text"),
        (
            json!({ "sha256d": [{ "property": "$ownerId" }] }),
            "property",
        ),
        (json!({ "sha256d": [{ "value": "." }] }), "value"),
        (
            json!({ "sha256d": [
                { "ifEmpty": "parentDomainName", "then": [{ "property": "label" }], "else": [{ "property": "label" }] },
                { "ifEmpty": "parentDomainName", "then": [{ "property": "label" }], "else": [{ "property": "label" }] }
            ] }),
            "ifEmpty",
        ),
        (
            json!({ "sha256d": [
                { "ifEmpty": "parentDomainName", "then": [
                    { "ifEmpty": "label", "then": [{ "property": "label" }], "else": [{ "property": "label" }] }
                ], "else": [{ "property": "label" }] }
            ] }),
            "then",
        ),
    ] {
        assert_refused(contract(with_key(key.clone())), fragment);
    }
}

#[test]
fn should_refuse_age_and_consume_without_a_computed_key() {
    let mut contract_value = dpns_contract(json!({}));
    contract_value["documentSchemas"]["domain"]
        .as_object_mut()
        .expect("an object")
        .remove("creatorRefersTo");
    contract_value["documentSchemas"]["domain"]["properties"]["referrerId"] = json!({
        "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32,
        "contentMediaType": "application/x.dash.dpp.identifier", "position": 5,
        "refersTo": {
            "type": "deletableDocument",
            "documentType": "preorder",
            "lookup": {
                "index": "saltedHash",
                "keys": { "saltedDomainHash": "." },
                "minimumAgeSeconds": 60
            }
        }
    });
    assert_refused(contract(contract_value), "need a computed key");
}

#[test]
fn should_refuse_consume_without_the_writer_owner_pair_or_on_a_permanent_reference() {
    assert_refused(
        contract(dpns_contract(domain_reveal(
            json!({ "consume": true }),
            json!({}),
        ))),
        "\"$ownerId\": \"$ownerId\"",
    );
    assert_refused(
        contract(dpns_contract(domain_reveal(
            json!({ "consume": true }),
            json!({ "propertyAgreement": { "$ownerId": "$creatorId" } }),
        ))),
        "\"$ownerId\": \"$ownerId\"",
    );
    let mut permanent = dpns_contract(domain_reveal(
        json!({ "consume": true }),
        json!({ "type": "permanentDocument", "propertyAgreement": { "$ownerId": "$ownerId" } }),
    ));
    permanent["documentSchemas"]["preorder"]["canBeDeleted"] = json!(false);
    assert_refused(
        contract(permanent),
        "permanentDocument refersTo lookup cannot consume",
    );
}

#[test]
fn should_refuse_a_computed_key_whose_index_property_cannot_hold_the_hash() {
    for (hash_property, fragment) in [
        (
            json!({ "type": "array", "byteArray": true, "minItems": 20, "maxItems": 20, "position": 0 }),
            "exactly 32 bytes",
        ),
        (
            json!({ "type": "string", "maxLength": 63, "position": 0 }),
            "exactly 32 bytes",
        ),
    ] {
        assert_refused(
            contract(dpns_contract_with(
                domain_reveal(json!({}), json!({})),
                json!({}),
                json!({ "properties": { "saltedDomainHash": hash_property } }),
            )),
            fragment,
        );
    }
}

#[test]
fn should_refuse_a_minimum_age_on_a_commitment_type_recording_no_creation_time() {
    assert_refused(
        contract(dpns_contract_with(
            domain_reveal(json!({ "minimumAgeSeconds": 60 }), json!({})),
            json!({}),
            json!({ "required": ["saltedDomainHash"] }),
        )),
        "does not record",
    );
}

#[test]
fn should_refuse_consuming_a_commitment_its_owner_may_not_delete() {
    // Deletable only by the platform (`ttl`), so a deletableDocument target, but
    // not by its owner, which consuming would stand in for
    assert_refused(
        contract(dpns_contract_with(
            domain_reveal(
                json!({ "consume": true }),
                json!({ "propertyAgreement": { "$ownerId": "$ownerId" } }),
            ),
            json!({}),
            json!({ "canBeDeleted": false, "ttl": 3600 }),
        )),
        "canBeDeleted: false",
    );
}

#[test]
fn should_refuse_preimage_parts_the_referring_type_cannot_supply() {
    let with_preimage = |preimage: serde_json::Value| {
        dpns_contract(json!({
            "type": "deletableDocument",
            "documentType": "preorder",
            "lookup": { "index": "saltedHash", "keys": { "saltedDomainHash": { "sha256d": preimage } } }
        }))
    };
    for (preimage, fragment) in [
        (json!([{ "property": "missing" }]), "not a property"),
        (
            json!([{ "property": "count" }]),
            "is not a string, a byte array or an identifier",
        ),
        // Two variable-length parts with nothing between them split more than one way
        (
            json!([{ "property": "normalizedLabel" }, { "property": "parentDomainName" }]),
            "one-byte text part",
        ),
        // A two-byte separator is not one byte
        (
            json!([{ "property": "normalizedLabel" }, { "text": ".." }, { "property": "parentDomainName" }]),
            "one-byte text part",
        ),
    ] {
        assert_refused(contract(with_preimage(preimage)), fragment);
    }
    // A fixed-length part needs no separator: the salt is always 32 bytes
    contract(with_preimage(json!([
        { "property": "preorderSalt" },
        { "property": "normalizedLabel" }
    ])))
    .expect("a fixed-length part followed by a variable one splits one way");
}

#[test]
fn should_refuse_a_stored_preimage_part_a_replace_could_change() {
    // On a mutable type, a stored part must be listed under `immutable`: the
    // reveal is judged when the document is created only
    let mutable = json!({ "documentsMutable": true });
    assert_refused(
        contract(dpns_contract_with(
            domain_reveal(json!({}), json!({})),
            mutable.clone(),
            json!({}),
        )),
        "which a replace can change",
    );
    contract(dpns_contract_with(
        domain_reveal(json!({}), json!({})),
        json!({
            "documentsMutable": true,
            "immutable": ["label", "normalizedLabel", "parentDomainName"]
        }),
        json!({}),
    ))
    .expect("stored parts listed under immutable, and the transient salt, are fixed");
}

#[test]
fn should_hold_a_property_reference_with_a_computed_key_to_the_value_and_a_fixed_property() {
    let property_reveal = |keys: serde_json::Value, domain_extra: serde_json::Value| {
        let mut contract_value = dpns_contract(json!({}));
        let domain = &mut contract_value["documentSchemas"]["domain"];
        domain
            .as_object_mut()
            .expect("an object")
            .remove("creatorRefersTo");
        domain["properties"]["committerId"] = json!({
            "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32,
            "contentMediaType": "application/x.dash.dpp.identifier", "position": 5,
            "refersTo": {
                "type": "deletableDocument",
                "documentType": "committed",
                "lookup": { "index": "byCommitter", "keys": keys }
            }
        });
        merge(domain, domain_extra);
        contract_value["documentSchemas"]["committed"] = json!({
            "type": "object",
            "documentsMutable": false,
            "canBeDeleted": true,
            "properties": {
                "committerId": {
                    "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier", "position": 0
                },
                "hash": {
                    "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32, "position": 1
                }
            },
            "indices": [{
                "name": "byCommitter",
                "properties": [{ "committerId": "asc" }, { "hash": "asc" }],
                "unique": true
            }],
            "required": ["committerId", "hash"],
            "additionalProperties": false
        });
        contract_value
    };
    let hash = json!({ "sha256d": [{ "property": "preorderSalt" }, { "property": "label" }] });

    // On a property the value must fill a key part
    assert_refused(
        contract(property_reveal(
            json!({ "committerId": "preorderSalt", "hash": hash.clone() }),
            json!({}),
        )),
        "fill no index property from",
    );
    // ... and the property carrying it must be set when the document is created
    assert_refused(
        contract(property_reveal(
            json!({ "committerId": ".", "hash": hash.clone() }),
            json!({
                "documentsMutable": true,
                "immutable": ["label", "committerId"],
                "immutableAllowSetting": ["committerId"]
            }),
        )),
        "immutableAllowSetting",
    );
    let parsed = contract(property_reveal(
        json!({ "committerId": ".", "hash": hash }),
        json!({}),
    ))
    .expect("the value and a computed key make the key");
    assert!(matches!(
        parsed
            .document_type_for_name("domain")
            .expect("the domain")
            .flattened_properties()
            .get("committerId")
            .expect("the committer")
            .property_type,
        DocumentPropertyType::IdentifierWithReference(
            DocumentPropertyReferenceTarget::DeletableDocumentLookup { .. }
        )
    ));
}

#[test]
fn should_refuse_a_computed_key_below_protocol_version_14_and_accept_it_at_14() {
    let schema = dpns_contract(domain_reveal(json!({ "minimumAgeSeconds": 60 }), json!({})));
    let platform_version_13 = PlatformVersion::get(13).expect("platform version 13 should exist");

    // Meta-schema v2 knows no creatorRefersTo, so a registering parse refuses it
    contract_on(schema.clone(), true, platform_version_13)
        .expect_err("protocol version 13 should refuse the declaration");
    contract_on(schema, true, PlatformVersion::latest()).expect("protocol version 14 parses it");
}

#[test]
fn should_refuse_an_update_changing_a_computed_key_or_what_it_demands() {
    use crate::consensus::basic::BasicError;
    use crate::consensus::ConsensusError;

    let platform_version = PlatformVersion::latest();
    let old = contract(dpns_contract(domain_reveal(
        json!({ "minimumAgeSeconds": 60, "consume": true }),
        json!({ "propertyAgreement": { "$ownerId": "$ownerId" } }),
    )))
    .expect("the DPNS reveal should parse");
    let old_domain = old.document_type_for_name("domain").expect("the domain");

    let mut reordered_branch = domain_reveal(
        json!({ "minimumAgeSeconds": 60, "consume": true }),
        json!({ "propertyAgreement": { "$ownerId": "$ownerId" } }),
    );
    reordered_branch["lookup"]["keys"]["saltedDomainHash"]["sha256d"][1]["then"] =
        json!([{ "property": "normalizedLabel" }]);

    // Documents were revealed under the declaration as written, so any change to
    // the key, its age or its consumption is an incompatible schema change
    for changed in [
        reordered_branch,
        domain_reveal(
            json!({ "minimumAgeSeconds": 61, "consume": true }),
            json!({ "propertyAgreement": { "$ownerId": "$ownerId" } }),
        ),
        domain_reveal(
            json!({ "minimumAgeSeconds": 60 }),
            json!({ "propertyAgreement": { "$ownerId": "$ownerId" } }),
        ),
    ] {
        let new = contract(dpns_contract(changed.clone())).expect("the change parses");
        let result = old_domain
            .validate_update(
                new.document_type_for_name("domain").expect("the domain"),
                2,
                platform_version,
            )
            .expect("validate_update should not error");
        assert!(
            matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(BasicError::IncompatibleDocumentTypeSchemaError(e))]
                    if e.property_path().starts_with("/creatorRefersTo")
            ),
            "{changed}: {:?}",
            result.errors
        );
    }

    // An unchanged declaration is no change
    let result = old_domain
        .validate_update(old_domain, 2, platform_version)
        .expect("validate_update should not error");
    assert!(result.is_valid(), "{:?}", result.errors);
}
