//! Lookups with a computed key
//! (`refersTo.lookup.keys: { "<index property>": { "function": "sys.hash.sha256d", "params": [...] } }`,
//! protocol version 14), a commit and reveal: the parse of the key, its
//! `minimumAgeBlocks` and `consume`, a string or byte array property whose
//! value the key reveals, the checks of the referring side on every parse,
//! the checks of the referenced side at contract level, and the protocol
//! version gate.

use super::reference_test_helpers::{assert_refused, contract, contract_on, CONTRACT_ID};
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::{
    DocumentPropertyReferenceTarget, DocumentPropertyType, DocumentReferenceLookup, HashFunction,
    LookupHashKey, LookupKeyParam, LookupKeySource, PropertyReference,
};
use crate::data_contract::DataContract;
use platform_value::string_encoding::Encoding;
use platform_value::Identifier;
use platform_version::version::PlatformVersion;
use serde_json::json;
use std::collections::BTreeMap;

/// The DPNS preorder hash of a name under a parent:
/// `preorderSalt ++ normalizedLabel ++ "." ++ parentDomainName`, the salt
/// being the value carrying the reference.
fn dpns_key() -> serde_json::Value {
    json!({
        "function": "sys.hash.sha256d",
        "params": [".", "normalizedLabel", { "const": "." }, "parentDomainName"]
    })
}

/// The `refersTo` of a DPNS-shaped `domain`'s `preorderSalt`, with
/// `lookup_extra` merged into its lookup and `declaration_extra` into the
/// declaration.
fn salt_reveal(
    lookup_extra: serde_json::Value,
    declaration_extra: serde_json::Value,
) -> serde_json::Value {
    let mut lookup = json!({
        "index": "saltedHash",
        "keys": { "saltedDomainHash": dpns_key() }
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

/// The DPNS reveal as DPNS would declare it: the writer's own preorder, from
/// an earlier block, deleted by the create.
fn dpns_salt_reveal() -> serde_json::Value {
    salt_reveal(
        json!({ "minimumAgeBlocks": 1, "consume": true }),
        json!({ "propertyAgreement": { "$ownerId": "$ownerId" } }),
    )
}

fn merge(target: &mut serde_json::Value, extra: serde_json::Value) {
    if let serde_json::Value::Object(entries) = extra {
        for (key, value) in entries {
            target[key] = value;
        }
    }
}

/// The DPNS-shaped preorder: immutable, deletable by its owner, recording
/// `$createdAtBlockHeight`, unique on its 32-byte `saltedDomainHash`.
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
        "required": ["$createdAtBlockHeight", "saltedDomainHash"],
        "additionalProperties": false
    })
}

/// The DPNS-shaped domain: immutable and transferable, its salt transient, as
/// in DPNS.
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
            "count": { "type": "integer", "minimum": 0, "maximum": 100, "position": 4 },
            "secret": { "type": "string", "maxLength": 63, "position": 5 }
        },
        "required": ["label", "normalizedLabel", "parentDomainName", "preorderSalt"],
        "transient": ["preorderSalt", "secret"],
        "additionalProperties": false
    })
}

/// A contract with `preorder` and a `domain` whose `preorderSalt` declares
/// `salt_refers_to`, with `domain_extra` merged into the domain and
/// `preorder_extra` into the preorder.
fn dpns_contract_with(
    salt_refers_to: serde_json::Value,
    domain_extra: serde_json::Value,
    preorder_extra: serde_json::Value,
) -> serde_json::Value {
    let mut domain = domain_schema();
    domain["properties"]["preorderSalt"]["refersTo"] = salt_refers_to;
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

fn dpns_contract(salt_refers_to: serde_json::Value) -> serde_json::Value {
    dpns_contract_with(salt_refers_to, json!({}), json!({}))
}

/// A DPNS-shaped contract whose `domain` declares `refers_to` on the
/// property `carrier` of `carrier_schema` instead of on its salt.
fn dpns_contract_carried_by(
    carrier: &str,
    carrier_schema: serde_json::Value,
    refers_to: serde_json::Value,
) -> serde_json::Value {
    let mut contract_value = dpns_contract_with(json!({}), json!({}), json!({}));
    let domain = &mut contract_value["documentSchemas"]["domain"];
    domain["properties"]["preorderSalt"]
        .as_object_mut()
        .expect("an object")
        .remove("refersTo");
    let mut carrier_schema = carrier_schema;
    carrier_schema["refersTo"] = refers_to;
    domain["properties"][carrier] = carrier_schema;
    contract_value
}

/// The domain's references, by holder.
fn domain_references(contract: &DataContract) -> Vec<(String, PropertyReference<'_>)> {
    contract
        .document_type_for_name("domain")
        .expect("the domain document type")
        .reference_declarations()
        .map(|(holder, reference)| (holder.path().to_string(), reference))
        .collect()
}

fn salt_reference(contract: &DataContract) -> DocumentPropertyReferenceTarget {
    match domain_references(contract).as_slice() {
        [(path, PropertyReference::Revealed(target))] if path == "preorderSalt" => {
            (*target).clone()
        }
        other => panic!("expected the salt's revealed reference alone, got {other:?}"),
    }
}

fn dpns_lookup(minimum_age_blocks: Option<u32>, consume: bool) -> DocumentReferenceLookup {
    DocumentReferenceLookup {
        index: "saltedHash".to_string(),
        keys: BTreeMap::from([(
            "saltedDomainHash".to_string(),
            LookupKeySource::Hash(LookupHashKey {
                function: HashFunction::Sha256d,
                params: vec![
                    LookupKeyParam::ReferenceValue,
                    LookupKeyParam::Property("normalizedLabel".to_string()),
                    LookupKeyParam::Const(".".to_string()),
                    LookupKeyParam::Property("parentDomainName".to_string()),
                ],
            }),
        )]),
        minimum_age_blocks,
        consume,
    }
}

#[test]
fn should_parse_the_dpns_salt_as_revealing_the_preorder_with_its_age_consumption_and_owner_pair() {
    let parsed = contract(dpns_contract(dpns_salt_reveal())).expect("the DPNS reveal should parse");

    assert_eq!(
        salt_reference(&parsed),
        DocumentPropertyReferenceTarget::DeletableDocumentLookup {
            contract_id: None,
            document_type_name: "preorder".to_string(),
            property_agreement: BTreeMap::from([("$ownerId".to_string(), "$ownerId".to_string())]),
            lookup: dpns_lookup(Some(1), true),
        }
    );
    // The salt stays a byte array: its value is revealed, not an id
    let salt = parsed
        .document_type_for_name("domain")
        .expect("the domain")
        .flattened_properties()
        .get("preorderSalt")
        .cloned()
        .expect("the salt");
    assert!(matches!(
        salt.property_type,
        DocumentPropertyType::ByteArray(_)
    ));
    assert!(salt.revealed_reference.is_some());
}

#[test]
fn should_parse_a_computed_key_that_demands_nothing_more() {
    // No owner pair, no minimum age, no consume: each is optional
    let parsed = contract(dpns_contract(salt_reveal(json!({}), json!({}))))
        .expect("a bare computed key should parse");
    let DocumentPropertyReferenceTarget::DeletableDocumentLookup { lookup, .. } =
        salt_reference(&parsed)
    else {
        panic!("expected a deletable lookup");
    };
    assert_eq!(lookup, dpns_lookup(None, false));
    assert!(lookup.is_checked_on_create_only());
}

#[test]
fn should_parse_a_string_carrier_revealed_before_a_separator() {
    // A string's value has no fixed length, so a one-byte const must follow it
    let reveal = |params: serde_json::Value| {
        dpns_contract_carried_by(
            "secret",
            json!({ "type": "string", "maxLength": 63, "position": 5 }),
            json!({
                "type": "deletableDocument",
                "documentType": "preorder",
                "lookup": {
                    "index": "saltedHash",
                    "keys": { "saltedDomainHash": { "function": "sys.hash.sha256d", "params": params } }
                }
            }),
        )
    };
    assert_refused(
        contract(reveal(json!([".", "normalizedLabel"]))),
        "param \".\" has no fixed length and another such param follows it",
    );
    let parsed = contract(reveal(json!([".", { "const": ":" }, "normalizedLabel"])))
        .expect("a separated string reveal should parse");
    assert!(matches!(
        domain_references(&parsed).as_slice(),
        [(path, PropertyReference::Revealed(_))] if path == "secret"
    ));
}

#[test]
fn should_parse_a_creator_lookup_whose_key_reads_the_creator_or_nothing() {
    // On creatorRefersTo "." is the creator's id, and beside a computed key it
    // may be left out: the key is the document's own
    for params in [
        json!([".", "preorderSalt", "normalizedLabel"]),
        json!(["preorderSalt", "normalizedLabel"]),
    ] {
        let mut contract_value = dpns_contract_with(json!({}), json!({}), json!({}));
        let domain = &mut contract_value["documentSchemas"]["domain"];
        domain["properties"]["preorderSalt"]
            .as_object_mut()
            .expect("an object")
            .remove("refersTo");
        domain["creatorRefersTo"] = json!({
            "type": "deletableDocument",
            "documentType": "preorder",
            "lookup": {
                "index": "saltedHash",
                "keys": { "saltedDomainHash": { "function": "sys.hash.sha256d", "params": params } }
            }
        });
        let parsed = contract(contract_value).expect("a creator's computed key should parse");
        assert!(matches!(
            domain_references(&parsed).as_slice(),
            [(path, PropertyReference::Value(_))] if path == "$creatorId"
        ));
    }
}

#[test]
fn should_refuse_a_deletable_creator_lookup_without_a_computed_key() {
    // The creator's gate on a deletable document would outlive a transfer:
    // only a computed key, judged on the create alone, is admitted
    let mut contract_value = dpns_contract_with(json!({}), json!({}), json!({}));
    let domain = &mut contract_value["documentSchemas"]["domain"];
    domain["properties"]["preorderSalt"]
        .as_object_mut()
        .expect("an object")
        .remove("refersTo");
    domain["creatorRefersTo"] = json!({
        "type": "deletableDocument",
        "documentType": "preorder",
        "lookup": { "index": "saltedHash", "keys": { "saltedDomainHash": "." } }
    });
    assert_refused(
        contract(contract_value),
        "does not take a deletableDocument reference unless its lookup key is computed",
    );
}

/// Both parses refuse `contract_value`: the validating one with the
/// meta-schema or the parser, the one that skips the meta-schema with the
/// parser's `fragment`.
fn assert_refused_by_the_parser(contract_value: serde_json::Value, fragment: &str) {
    contract(contract_value.clone()).expect_err("a validating parse should refuse it");
    assert_refused(
        contract_on(contract_value, false, PlatformVersion::latest()),
        fragment,
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
        (
            json!({ "function": "sys.hash.sha256", "params": ["."] }),
            "function \"sys.hash.sha256\" is not a hash",
        ),
        // A string transformation is a system function, but not a hash
        (
            json!({ "function": "sys.stringTransformations.lowercase", "params": ["."] }),
            "is not a hash",
        ),
        (json!({ "params": ["."] }), "names its function"),
        (
            json!({ "function": "sys.hash.sha256d" }),
            "lists its params",
        ),
        (
            json!({ "function": "sys.hash.sha256d", "params": ["."], "extra": 1 }),
            "takes function and params, not \"extra\"",
        ),
        (
            json!({ "function": "sys.hash.sha256d", "params": [] }),
            "between 1 and 16 params, found 0",
        ),
        (
            json!({ "function": "sys.hash.sha256d", "params": [".", { "const": "" }] }),
            "const holds between 1 and 64 bytes",
        ),
        (
            json!({ "function": "sys.hash.sha256d", "params": [".", "$ownerId"] }),
            "not \"$ownerId\"",
        ),
        (
            json!({ "function": "sys.hash.sha256d", "params": [".", { "value": "." }] }),
            "takes const, not \"value\"",
        ),
        (
            json!({ "function": "sys.hash.sha256d", "params": [".", { "const": 7 }] }),
            "const must be a string",
        ),
        (
            json!({ "function": "sys.hash.sha256d", "params": [".", 7] }),
            "a property path or { \"const\": text }",
        ),
    ] {
        assert_refused_by_the_parser(with_key(key), fragment);
    }
}

#[test]
fn should_refuse_age_and_consume_without_a_computed_key() {
    let contract_value = dpns_contract_carried_by(
        "referrerId",
        json!({
            "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32,
            "contentMediaType": "application/x.dash.dpp.identifier", "position": 6
        }),
        json!({
            "type": "deletableDocument",
            "documentType": "preorder",
            "lookup": {
                "index": "saltedHash",
                "keys": { "saltedDomainHash": "." },
                "minimumAgeBlocks": 1
            }
        }),
    );
    assert_refused(contract(contract_value), "need a computed key");
}

#[test]
fn should_refuse_a_minimum_age_of_zero_or_a_consume_of_false() {
    assert_refused_by_the_parser(
        dpns_contract(salt_reveal(json!({ "minimumAgeBlocks": 0 }), json!({}))),
        "minimumAgeBlocks must be at least 1",
    );
    assert_refused_by_the_parser(
        dpns_contract(salt_reveal(json!({ "minimumAgeSeconds": 60 }), json!({}))),
        "refersTo lookup \"minimumAgeSeconds\" is unknown",
    );
    assert_refused_by_the_parser(
        dpns_contract(salt_reveal(json!({ "consume": false }), json!({}))),
        "consume may only be declared true",
    );
}

#[test]
fn should_refuse_consume_without_the_writer_owner_pair_or_on_a_permanent_reference() {
    assert_refused(
        contract(dpns_contract(salt_reveal(
            json!({ "consume": true }),
            json!({}),
        ))),
        "\"$ownerId\": \"$ownerId\"",
    );
    assert_refused(
        contract(dpns_contract(salt_reveal(
            json!({ "consume": true }),
            json!({ "propertyAgreement": { "$ownerId": "$creatorId" } }),
        ))),
        "\"$ownerId\": \"$ownerId\"",
    );
    let mut permanent = dpns_contract(salt_reveal(
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
    for hash_property in [
        json!({ "type": "array", "byteArray": true, "minItems": 20, "maxItems": 20, "position": 0 }),
        json!({ "type": "string", "maxLength": 63, "position": 0 }),
    ] {
        assert_refused(
            contract(dpns_contract_with(
                salt_reveal(json!({}), json!({})),
                json!({}),
                json!({ "properties": { "saltedDomainHash": hash_property } }),
            )),
            "is a sys.hash.sha256d hash, so the index property must be a byte array of exactly \
             32 bytes",
        );
    }
}

#[test]
fn should_refuse_a_minimum_age_on_a_commitment_type_recording_no_creation_block() {
    // The creation time is not the creation block: the age is counted in blocks
    assert_refused(
        contract(dpns_contract_with(
            salt_reveal(json!({ "minimumAgeBlocks": 1 }), json!({})),
            json!({}),
            json!({ "required": ["$createdAt", "saltedDomainHash"] }),
        )),
        "minimumAgeBlocks is judged against the found document's $createdAtBlockHeight, which \
         \"preorder\" does not record",
    );
}

#[test]
fn should_refuse_consuming_a_commitment_its_owner_may_not_delete() {
    // Deletable only by the platform (`ttl`), so a deletableDocument target, but
    // not by its owner, which consuming would stand in for
    assert_refused(
        contract(dpns_contract_with(
            salt_reveal(
                json!({ "consume": true }),
                json!({ "propertyAgreement": { "$ownerId": "$ownerId" } }),
            ),
            json!({}),
            json!({
                "canBeDeleted": false,
                "ttl": 3600,
                "required": ["$createdAt", "$createdAtBlockHeight", "saltedDomainHash"]
            }),
        )),
        "canBeDeleted: false",
    );
}

#[test]
fn should_refuse_params_the_referring_type_cannot_supply() {
    let with_params = |params: serde_json::Value| {
        dpns_contract(json!({
            "type": "deletableDocument",
            "documentType": "preorder",
            "lookup": {
                "index": "saltedHash",
                "keys": { "saltedDomainHash": { "function": "sys.hash.sha256d", "params": params } }
            }
        }))
    };
    for (params, fragment) in [
        (
            json!([".", "missing"]),
            "param \"missing\" is not a property of the referring document type",
        ),
        (
            json!([".", "count"]),
            "param \"count\" is not a string, a byte array or an identifier",
        ),
        // Two variable-length params with nothing between them split more than one way
        (
            json!([".", "normalizedLabel", "parentDomainName"]),
            "param \"normalizedLabel\" has no fixed length",
        ),
        // A two-byte separator is not one byte
        (
            json!([".", "normalizedLabel", { "const": ".." }, "parentDomainName"]),
            "one-byte const",
        ),
        // The value carrying the reference fills the key exactly once
        (
            json!([".", "normalizedLabel", { "const": "." }, "."]),
            "exactly once, found 2",
        ),
    ] {
        assert_refused(contract(with_params(params)), fragment);
    }
    // The salt is always 32 bytes and the last param ends the preimage: no
    // separator is needed
    contract(with_params(json!([".", "normalizedLabel"])))
        .expect("a fixed-length param followed by a variable one splits one way");
}

#[test]
fn should_refuse_a_revealed_reference_that_is_not_a_computed_key_reading_the_value() {
    for (refers_to, fragment) in [
        // A plain key would compare the salt with the index property as is
        (
            json!({
                "type": "deletableDocument",
                "documentType": "preorder",
                "lookup": { "index": "saltedHash", "keys": { "saltedDomainHash": "." } }
            }),
            "whose lookup reveals the value in a computed key",
        ),
        // A computed key that never reads the salt reveals nothing
        (
            json!({
                "type": "deletableDocument",
                "documentType": "preorder",
                "lookup": {
                    "index": "saltedHash",
                    "keys": { "saltedDomainHash": {
                        "function": "sys.hash.sha256d",
                        "params": ["normalizedLabel", { "const": "." }, "parentDomainName"]
                    } }
                }
            }),
            "whose lookup reveals the value in a computed key",
        ),
        // Without a lookup there is nothing to reveal the value into
        (
            json!({ "type": "identity" }),
            "refersTo is only allowed on identifier properties",
        ),
        (
            json!({ "anyOf": [salt_reveal(json!({}), json!({})), salt_reveal(json!({ "consume": true }), json!({}))] }),
            "refersTo anyOf is only allowed on identifier properties",
        ),
        // Only a document is found through a lookup, and the value is no key id
        (
            json!({
                "type": "identity",
                "lookup": { "index": "saltedHash", "keys": { "saltedDomainHash": dpns_key() } }
            }),
            "must be a permanentDocument or deletableDocument reference",
        ),
        (
            json!({
                "type": "deletableDocument",
                "documentType": "preorder",
                "identityProperty": "$ownerId",
                "lookup": { "index": "saltedHash", "keys": { "saltedDomainHash": dpns_key() } }
            }),
            "must be a permanentDocument or deletableDocument reference",
        ),
    ] {
        assert_refused_by_the_parser(dpns_contract(refers_to), fragment);
    }
}

#[test]
fn should_refuse_a_carrier_or_stored_param_a_replace_could_change() {
    // On a mutable type, the salt must be transient or fixed once written, and a
    // stored param listed under `immutable`: the reveal is judged when the
    // document is created only
    let not_transient = |extra: serde_json::Value| {
        let mut domain_extra = json!({ "documentsMutable": true, "transient": ["secret"] });
        merge(&mut domain_extra, extra);
        contract(dpns_contract_with(
            salt_reveal(json!({}), json!({})),
            domain_extra,
            json!({}),
        ))
    };
    assert_refused(
        not_transient(json!({ "immutable": ["normalizedLabel", "parentDomainName"] })),
        "\"preorderSalt\" carries a lookup with a computed key",
    );
    assert_refused(
        not_transient(json!({
            "immutable": ["normalizedLabel", "parentDomainName", "preorderSalt"],
            "immutableAllowSetting": ["preorderSalt"]
        })),
        "\"preorderSalt\" carries a lookup with a computed key",
    );
    assert_refused(
        contract(dpns_contract_with(
            salt_reveal(json!({}), json!({})),
            json!({ "documentsMutable": true }),
            json!({}),
        )),
        "is a property a replace can change",
    );
    not_transient(json!({
        "immutable": ["normalizedLabel", "parentDomainName", "preorderSalt"]
    }))
    .expect("a stored salt fixed once written can carry the reveal");
    contract(dpns_contract_with(
        salt_reveal(json!({}), json!({})),
        json!({
            "documentsMutable": true,
            "immutable": ["label", "normalizedLabel", "parentDomainName"]
        }),
        json!({}),
    ))
    .expect("stored params listed under immutable, and the transient salt, are fixed");
}

#[test]
fn should_hold_an_identifier_reference_with_a_computed_key_beside_its_value() {
    let committer_reveal = |keys: serde_json::Value, domain_extra: serde_json::Value| {
        let mut contract_value = dpns_contract_carried_by(
            "committerId",
            json!({
                "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32,
                "contentMediaType": "application/x.dash.dpp.identifier", "position": 6
            }),
            json!({
                "type": "deletableDocument",
                "documentType": "committed",
                "lookup": { "index": "byCommitter", "keys": keys }
            }),
        );
        merge(
            &mut contract_value["documentSchemas"]["domain"],
            domain_extra,
        );
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
    let hash = json!({ "function": "sys.hash.sha256d", "params": ["preorderSalt", "label"] });

    // On a property the value must fill the key
    assert_refused(
        contract(committer_reveal(
            json!({ "committerId": "preorderSalt", "hash": hash.clone() }),
            json!({}),
        )),
        "the key reads \".\", the value of \"committerId\", 0 times",
    );
    // At most one key is computed
    assert_refused_by_the_parser(
        committer_reveal(
            json!({ "committerId": hash.clone(), "hash": hash.clone() }),
            json!({}),
        ),
        "at most one computed key",
    );
    // ... and the property carrying it must be set when the document is created
    assert_refused(
        contract(committer_reveal(
            json!({ "committerId": ".", "hash": hash.clone() }),
            json!({
                "documentsMutable": true,
                "immutable": ["label", "committerId"],
                "immutableAllowSetting": ["committerId"]
            }),
        )),
        "immutableAllowSetting",
    );
    let parsed = contract(committer_reveal(
        json!({ "committerId": ".", "hash": hash }),
        json!({}),
    ))
    .expect("the value and a computed key make the key");
    assert!(matches!(
        domain_references(&parsed).as_slice(),
        [(path, PropertyReference::Value(
            DocumentPropertyReferenceTarget::DeletableDocumentLookup { .. }
        ))] if path == "committerId"
    ));
}

#[test]
fn should_refuse_a_computed_key_below_protocol_version_14_and_accept_it_at_14() {
    let schema = dpns_contract(dpns_salt_reveal());
    let platform_version_13 = PlatformVersion::get(13).expect("platform version 13 should exist");

    // Meta-schema v2 knows no refersTo on a byte array, so a registering parse
    // refuses it
    contract_on(schema.clone(), true, platform_version_13)
        .expect_err("protocol version 13 should refuse the declaration");
    contract_on(schema, true, PlatformVersion::latest()).expect("protocol version 14 parses it");
}

#[test]
fn should_refuse_an_update_changing_a_computed_key_or_what_it_demands() {
    use crate::consensus::basic::BasicError;
    use crate::consensus::ConsensusError;

    let platform_version = PlatformVersion::latest();
    let old = contract(dpns_contract(dpns_salt_reveal())).expect("the DPNS reveal should parse");
    let old_domain = old.document_type_for_name("domain").expect("the domain");

    let mut reordered = dpns_salt_reveal();
    reordered["lookup"]["keys"]["saltedDomainHash"]["params"] =
        json!([".", "parentDomainName", { "const": "." }, "normalizedLabel"]);

    // Documents were revealed under the declaration as written, so any change to
    // the key, its age or its consumption is an incompatible schema change
    for changed in [
        reordered,
        salt_reveal(
            json!({ "minimumAgeBlocks": 2, "consume": true }),
            json!({ "propertyAgreement": { "$ownerId": "$ownerId" } }),
        ),
        salt_reveal(
            json!({ "minimumAgeBlocks": 1 }),
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
            !result.errors.is_empty()
                && result.errors.iter().all(|error| matches!(
                    error,
                    ConsensusError::BasicError(BasicError::IncompatibleDocumentTypeSchemaError(e))
                        if e.property_path().starts_with("/properties/preorderSalt/refersTo/lookup")
                )),
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
