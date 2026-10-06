//! `findBy` functions
//! (`"<referenced property>": { "function": "sys.hash.sha256d", "params": [...] }`,
//! protocol version 14), a commit and reveal: the parse of the function into
//! the computed key the reference is found by, the `minimumAgeBlocks` and
//! `consume` declared beside `findBy`, a string or byte array property whose
//! value the function reads, the checks of the referring side on every parse,
//! the checks of the referenced side at contract level, and the protocol
//! version gate.

use super::reference_test_helpers::{assert_refused, contract, contract_on, CONTRACT_ID};
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::document_type::accessors::{
    DocumentTypeV0Getters, DocumentTypeV2Getters,
};
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
/// named by its path.
fn dpns_function() -> serde_json::Value {
    json!({
        "function": "sys.hash.sha256d",
        "params": ["preorderSalt", "normalizedLabel", { "const": "." }, "parentDomainName"]
    })
}

/// A preorder reference found by `findBy` holding `function` for
/// `saltedDomainHash` (so through the `saltedHash` index), with `find_by_extra`
/// merged into its `findBy`, `where_extra` as its `where` (left out when empty)
/// and `declaration_extra` merged into the declaration.
fn reveal_with(
    function: serde_json::Value,
    find_by_extra: serde_json::Value,
    where_extra: serde_json::Value,
    declaration_extra: serde_json::Value,
) -> serde_json::Value {
    let mut find_by = json!({ "saltedDomainHash": function });
    merge(&mut find_by, find_by_extra);
    let mut declaration = json!({
        "type": "deletableDocument",
        "documentType": "preorder",
        "findBy": find_by
    });
    if where_extra
        .as_object()
        .is_some_and(|entries| !entries.is_empty())
    {
        declaration["where"] = where_extra;
    }
    merge(&mut declaration, declaration_extra);
    declaration
}

/// The `refersTo` of a DPNS-shaped `domain`'s `preorderSalt`, with
/// `declaration_extra` (its `minimumAgeBlocks` and `consume`) merged into the
/// declaration and `where_extra` as its `where`.
fn salt_reveal(
    declaration_extra: serde_json::Value,
    where_extra: serde_json::Value,
) -> serde_json::Value {
    reveal_with(dpns_function(), json!({}), where_extra, declaration_extra)
}

/// The DPNS reveal as DPNS would declare it: the writer's own preorder, from
/// an earlier block, deleted by the create.
fn dpns_salt_reveal() -> serde_json::Value {
    salt_reveal(
        json!({ "minimumAgeBlocks": 1, "consume": true }),
        json!({ "$ownerId": "$ownerId" }),
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

/// A DPNS-shaped contract whose `domain` declares `creator_refers_to` as its
/// `creatorRefersTo`, the salt carrying nothing.
fn dpns_contract_with_creator_reference(creator_refers_to: serde_json::Value) -> serde_json::Value {
    let mut contract_value = dpns_contract_with(json!({}), json!({}), json!({}));
    let domain = &mut contract_value["documentSchemas"]["domain"];
    domain["properties"]["preorderSalt"]
        .as_object_mut()
        .expect("an object")
        .remove("refersTo");
    domain["creatorRefersTo"] = creator_refers_to;
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
        keys: BTreeMap::from([(
            "saltedDomainHash".to_string(),
            LookupKeySource::Hash(LookupHashKey {
                function: HashFunction::Sha256d,
                params: vec![
                    LookupKeyParam::Property("preorderSalt".to_string()),
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
fn should_parse_the_dpns_salt_as_revealing_the_preorder_with_its_age_consumption_and_owner_pair() {
    let parsed = contract(dpns_contract(dpns_salt_reveal())).expect("the DPNS reveal should parse");

    // The function is the lookup's computed key; the where entries are the
    // agreement
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
fn should_parse_a_function_pair_that_demands_nothing_more() {
    // No owner entry, no minimum age, no consume: each is optional, and
    // findBy may hold the function alone
    let parsed = contract(dpns_contract(salt_reveal(json!({}), json!({}))))
        .expect("a bare function pair should parse");
    let DocumentPropertyReferenceTarget::DeletableDocumentLookup {
        lookup,
        property_agreement,
        ..
    } = salt_reference(&parsed)
    else {
        panic!("expected a deletable lookup");
    };
    assert_eq!(lookup, dpns_lookup(None, false));
    assert!(property_agreement.is_empty());
    assert!(lookup.is_checked_on_create_only());
}

#[test]
fn should_parse_a_string_carrier_revealed_before_a_separator() {
    // A string's value has no fixed length, so a one-byte const must follow it
    let reveal = |params: serde_json::Value| {
        dpns_contract_carried_by(
            "secret",
            json!({ "type": "string", "maxLength": 63, "position": 5 }),
            reveal_with(
                json!({ "function": "sys.hash.sha256d", "params": params }),
                json!({}),
                json!({}),
                json!({}),
            ),
        )
    };
    assert_refused(
        contract(reveal(json!(["secret", "normalizedLabel"]))),
        "param \"secret\" has no fixed length and another such param follows it",
    );
    let parsed = contract(reveal(
        json!(["secret", { "const": ":" }, "normalizedLabel"]),
    ))
    .expect("a separated string reveal should parse");
    assert!(matches!(
        domain_references(&parsed).as_slice(),
        [(path, PropertyReference::Revealed(_))] if path == "secret"
    ));
}

#[test]
fn should_parse_a_creator_function_reading_the_creator_or_nothing() {
    // The creator has no path: "." reads its id, and beside a computed key it
    // may be left out, the key being the document's own
    for params in [
        json!([".", "preorderSalt", "normalizedLabel"]),
        json!(["preorderSalt", "normalizedLabel"]),
    ] {
        let parsed = contract(dpns_contract_with_creator_reference(reveal_with(
            json!({ "function": "sys.hash.sha256d", "params": params }),
            json!({}),
            json!({}),
            json!({}),
        )))
        .expect("a creator's function pair should parse");
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
    assert_refused(
        contract(dpns_contract_with_creator_reference(json!({
            "type": "deletableDocument",
            "documentType": "preorder",
            "findBy": {
                "saltedDomainHash": "."
            }
        }))),
        "does not take a deletableDocument reference unless its findBy key is computed",
    );
}

#[test]
fn should_refuse_malformed_functions_in_the_parser() {
    let with_function = |function: serde_json::Value| {
        dpns_contract(reveal_with(function, json!({}), json!({}), json!({})))
    };
    for (function, fragment) in [
        (
            json!({ "function": "sys.hash.sha256", "params": ["preorderSalt"] }),
            "function \"sys.hash.sha256\" is not a hash",
        ),
        // A string transformation is a system function, but not a hash
        (
            json!({ "function": "sys.stringTransformations.lowercase", "params": ["preorderSalt"] }),
            "is not a hash",
        ),
        (json!({ "params": ["preorderSalt"] }), "names its function"),
        (
            json!({ "function": "sys.hash.sha256d" }),
            "lists its params",
        ),
        (
            json!({ "function": "sys.hash.sha256d", "params": ["preorderSalt"], "extra": 1 }),
            "takes function and params, not \"extra\"",
        ),
        (
            json!({ "function": "sys.hash.sha256d", "params": [] }),
            "between 1 and 16 params, found 0",
        ),
        (
            json!({ "function": "sys.hash.sha256d", "params": ["preorderSalt", { "const": "" }] }),
            "const holds between 1 and 64 bytes",
        ),
        (
            json!({ "function": "sys.hash.sha256d", "params": ["preorderSalt", "$ownerId"] }),
            "not \"$ownerId\"",
        ),
        (
            json!({ "function": "sys.hash.sha256d", "params": ["preorderSalt", { "value": "." }] }),
            "takes const, not \"value\"",
        ),
        (
            json!({ "function": "sys.hash.sha256d", "params": ["preorderSalt", { "const": 7 }] }),
            "const must be a string",
        ),
        (
            json!({ "function": "sys.hash.sha256d", "params": ["preorderSalt", 7] }),
            "a property path or { \"const\": text }",
        ),
    ] {
        assert_refused_by_the_parser(with_function(function), fragment);
    }
}

#[test]
fn should_refuse_a_function_that_computes_no_find_by_key() {
    // Keyed by a system property, which no function computes
    let mut system_key = dpns_contract(salt_reveal(json!({}), json!({})));
    let find_by = &mut system_key["documentSchemas"]["domain"]["properties"]["preorderSalt"]
        ["refersTo"]["findBy"];
    find_by["$ownerId"] = dpns_function();
    find_by
        .as_object_mut()
        .expect("an object")
        .remove("saltedDomainHash");
    assert_refused_by_the_parser(
        system_key,
        "findBy function \"$ownerId\" must fill a schema property of the referenced",
    );

    // Two functions
    assert_refused_by_the_parser(
        dpns_contract(reveal_with(
            dpns_function(),
            json!({ "other": dpns_function() }),
            json!({}),
            json!({}),
        )),
        "findBy holds at most one function",
    );

    // A function in where, which compares the document found and finds none
    assert_refused_by_the_parser(
        dpns_contract_carried_by(
            "referrerId",
            json!({
                "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32,
                "contentMediaType": "application/x.dash.dpp.identifier", "position": 6
            }),
            json!({
                "type": "deletableDocument",
                "documentType": "preorder",
                "findBy": { "saltedDomainHash": "." },
                "where": { "saltedDomainHash": dpns_function() }
            }),
        ),
        "holds a function, which computes a key that finds the document: declare it in findBy",
    );
}

#[test]
fn should_refuse_age_and_consume_without_a_function() {
    let referrer = |refers_to: serde_json::Value| {
        dpns_contract_carried_by(
            "referrerId",
            json!({
                "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32,
                "contentMediaType": "application/x.dash.dpp.identifier", "position": 6
            }),
            refers_to,
        )
    };
    // A key read as is finds no commitment
    for extra in [json!({ "minimumAgeBlocks": 1 }), json!({ "consume": true })] {
        let mut refers_to = json!({
            "type": "deletableDocument",
            "documentType": "preorder",
            "findBy": { "saltedDomainHash": "." },
            "where": { "$ownerId": "$ownerId" }
        });
        merge(&mut refers_to, extra);
        assert_refused_by_the_parser(
            referrer(refers_to),
            "need a findBy function computing the key",
        );
    }
    // Nor does a reference without findBy
    assert_refused_by_the_parser(
        referrer(json!({
            "type": "deletableDocument",
            "documentType": "preorder",
            "minimumAgeBlocks": 1
        })),
        "deletableDocument refersTo does not take minimumAgeBlocks without findBy",
    );
    assert_refused_by_the_parser(
        referrer(json!({ "type": "identity", "consume": true })),
        "identity refersTo does not take consume without findBy",
    );
}

#[test]
fn should_refuse_a_minimum_age_of_zero_or_a_consume_of_false() {
    assert_refused_by_the_parser(
        dpns_contract(salt_reveal(json!({ "minimumAgeBlocks": 0 }), json!({}))),
        "minimumAgeBlocks must be at least 1",
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
            json!({ "$creatorId": "$ownerId" }),
        ))),
        "\"$ownerId\": \"$ownerId\"",
    );
    let mut permanent = dpns_contract(reveal_with(
        dpns_function(),
        json!({}),
        json!({ "$ownerId": "$ownerId" }),
        json!({ "type": "permanentDocument", "consume": true }),
    ));
    permanent["documentSchemas"]["preorder"]["canBeDeleted"] = json!(false);
    assert_refused_by_the_parser(permanent, "permanentDocument refersTo cannot consume");
}

#[test]
fn should_refuse_a_function_whose_index_property_cannot_hold_the_hash() {
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
            "findBy function \"saltedDomainHash\" is a sys.hash.sha256d hash, so the index \
             property must be a byte array of exactly 32 bytes",
        );
    }
}

#[test]
fn should_refuse_a_function_naming_a_property_outside_the_index() {
    let mut contract_value =
        dpns_contract_with(salt_reveal(json!({}), json!({})), json!({}), json!({}));
    let preorder = &mut contract_value["documentSchemas"]["preorder"];
    preorder["properties"]["other"] = json!({ "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32, "position": 1 });
    let find_by = &mut contract_value["documentSchemas"]["domain"]["properties"]["preorderSalt"]
        ["refersTo"]["findBy"];
    find_by["other"] = dpns_function();
    find_by
        .as_object_mut()
        .expect("an object")
        .remove("saltedDomainHash");
    assert_refused(
        contract(contract_value),
        "\"preorder\" has no unique index over exactly (other)",
    );
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
                json!({ "$ownerId": "$ownerId" }),
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
fn should_consume_a_commitment_only_a_consume_deletes() {
    // `canBeDeleted: "onlyWhenConsumed"`: its owner can not withdraw the preorder with a
    // delete, and the reveal consumes it
    let parsed = contract(dpns_contract_with(
        dpns_salt_reveal(),
        json!({}),
        json!({ "canBeDeleted": "onlyWhenConsumed" }),
    ))
    .expect("a reveal consuming a commitment only a consume deletes should parse");
    assert_eq!(
        salt_reference(&parsed),
        DocumentPropertyReferenceTarget::DeletableDocumentLookup {
            contract_id: None,
            document_type_name: "preorder".to_string(),
            property_agreement: BTreeMap::from([("$ownerId".to_string(), "$ownerId".to_string())]),
            lookup: dpns_lookup(Some(1), true),
        }
    );
    let preorder = parsed
        .document_type_for_name("preorder")
        .expect("the preorder");
    assert!(!preorder.documents_can_be_deleted());
    assert!(preorder.documents_deleted_only_when_consumed());
}

#[test]
fn should_refuse_params_the_referring_type_cannot_supply() {
    let with_params = |params: serde_json::Value| {
        dpns_contract(reveal_with(
            json!({ "function": "sys.hash.sha256d", "params": params }),
            json!({}),
            json!({}),
            json!({}),
        ))
    };
    for (params, fragment) in [
        (
            json!(["preorderSalt", "missing"]),
            "param \"missing\" is not a property of the referring document type",
        ),
        (
            json!(["preorderSalt", "count"]),
            "param \"count\" is not a string, a byte array or an identifier",
        ),
        // Two variable-length params with nothing between them split more than one way
        (
            json!(["preorderSalt", "normalizedLabel", "parentDomainName"]),
            "param \"normalizedLabel\" has no fixed length",
        ),
        // A two-byte separator is not one byte
        (
            json!(["preorderSalt", "normalizedLabel", { "const": ".." }, "parentDomainName"]),
            "one-byte const",
        ),
        // The salt has a path, which names it: "." is for values without one
        (
            json!([".", "normalizedLabel", { "const": "." }, "parentDomainName"]),
            "a param names the property carrying the reference by its path, \"preorderSalt\"",
        ),
        // The function reads the value carrying the reference exactly once
        (
            json!(["normalizedLabel", { "const": "." }, "parentDomainName"]),
            "reads the value of \"preorderSalt\" 0 times",
        ),
        (
            json!(["preorderSalt", "preorderSalt"]),
            "reads the value of \"preorderSalt\" 2 times",
        ),
    ] {
        assert_refused(contract(with_params(params)), fragment);
    }
    // The salt is always 32 bytes and the last param ends the preimage: no
    // separator is needed
    contract(with_params(json!(["preorderSalt", "normalizedLabel"])))
        .expect("a fixed-length param followed by a variable one splits one way");
}

#[test]
fn should_refuse_a_revealed_reference_without_a_function_pair() {
    for (refers_to, fragment) in [
        // A plain key would compare the salt with the index property as is
        (
            json!({
                "type": "deletableDocument",
                "documentType": "preorder",
                "findBy": {
                    "saltedDomainHash": "."
                }
            }),
            "found by a findBy function reading the value",
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
        // Only a document is found by findBy, and the value is no key id
        (
            json!({
                "type": "identity",
                "findBy": { "saltedDomainHash": dpns_function() }
            }),
            "must be a permanentDocument or deletableDocument reference",
        ),
        (
            reveal_with(
                dpns_function(),
                json!({}),
                json!({}),
                json!({ "identityProperty": "$ownerId" }),
            ),
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
        "\"preorderSalt\" carries a reference found by a computed key",
    );
    assert_refused(
        not_transient(json!({
            "immutable": [
                "normalizedLabel",
                "parentDomainName",
                { "property": "preorderSalt", "when": { "present": "$old.preorderSalt" } }
            ]
        })),
        "\"preorderSalt\" carries a reference found by a computed key",
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

/// A stored param that is an immutable `deletableDocument` reference by id is
/// fixed only when required: a replace may clear an optional one once its
/// document is deleted, and the stored reveal would then no longer hold the
/// value its commitment was revealed for.
#[test]
fn should_refuse_a_param_a_replace_can_clear_once_its_document_is_deleted() {
    let option_reveal = |required: bool| {
        let mut domain_extra = json!({
            "documentsMutable": true,
            "immutable": ["normalizedLabel", "parentDomainName", "optionId"]
        });
        if required {
            domain_extra["required"] = json!([
                "label",
                "normalizedLabel",
                "parentDomainName",
                "preorderSalt",
                "optionId"
            ]);
        }
        let mut contract_value = dpns_contract_with(
            reveal_with(
                json!({ "function": "sys.hash.sha256d", "params": ["preorderSalt", "optionId"] }),
                json!({}),
                json!({}),
                json!({}),
            ),
            domain_extra,
            json!({}),
        );
        contract_value["documentSchemas"]["domain"]["properties"]["optionId"] = json!({
            "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32,
            "contentMediaType": "application/x.dash.dpp.identifier", "position": 6,
            "refersTo": { "type": "deletableDocument", "documentType": "option" }
        });
        contract_value["documentSchemas"]["option"] = json!({
            "type": "object",
            "canBeDeleted": true,
            "properties": { "name": { "type": "string", "maxLength": 63, "position": 0 } },
            "additionalProperties": false
        });
        contract(contract_value)
    };
    assert_refused(
        option_reveal(false),
        "param \"optionId\" is an optional `deletableDocument` reference a replace can clear",
    );
    option_reveal(true).expect("a required reference is never cleared");
}

/// A contract whose domain's `committerId` identifier refers to a
/// `committed` document unique on (`committerId`, `hash`), found by `keys`
/// with `function` merged in, with `domain_extra` merged into the domain.
fn committer_reveal(
    keys: serde_json::Value,
    function: serde_json::Value,
    domain_extra: serde_json::Value,
) -> serde_json::Value {
    let mut find_by = keys;
    merge(&mut find_by, function);
    let mut contract_value = dpns_contract_carried_by(
        "committerId",
        json!({
            "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32,
            "contentMediaType": "application/x.dash.dpp.identifier", "position": 6
        }),
        json!({
            "type": "deletableDocument",
            "documentType": "committed",
            "findBy": find_by
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
}

#[test]
fn should_hold_an_identifier_reference_with_a_function_pair_beside_its_value() {
    let hash = json!({ "function": "sys.hash.sha256d", "params": ["preorderSalt", "label"] });

    // On a property the value must fill the key
    assert_refused(
        contract(committer_reveal(
            json!({ "committerId": "preorderSalt" }),
            json!({ "hash": hash.clone() }),
            json!({}),
        )),
        "reads the value of \"committerId\" 0 times",
    );
    // ... and the property carrying it must be set when the document is created
    assert_refused(
        contract(committer_reveal(
            json!({ "committerId": "." }),
            json!({ "hash": hash.clone() }),
            json!({
                "documentsMutable": true,
                "immutable": [
                    "label",
                    { "property": "committerId", "when": { "present": "$old.committerId" } }
                ]
            }),
        )),
        "under `immutable` without a condition",
    );
    // On a mutable type the carrier is listed under immutable, which a
    // deletable lookup re-validated on every replace could not be, but a
    // function's lookup is judged on the create alone
    contract(committer_reveal(
        json!({ "committerId": "." }),
        json!({ "hash": hash.clone() }),
        json!({
            "documentsMutable": true,
            "immutable": ["label", "committerId"]
        }),
    ))
    .expect("an immutable identifier carrier on a mutable type holds a function's lookup");
    let parsed = contract(committer_reveal(
        json!({ "committerId": "." }),
        json!({ "hash": hash }),
        json!({}),
    ))
    .expect("the value and a function pair make the key");
    assert!(matches!(
        domain_references(&parsed).as_slice(),
        [(path, PropertyReference::Value(
            DocumentPropertyReferenceTarget::DeletableDocumentLookup { .. }
        ))] if path == "committerId"
    ));
}

#[test]
fn should_read_each_element_of_a_typed_array_as_dot() {
    // An element has no path of its own: "." reads it
    let with_params = |params: serde_json::Value| {
        let mut contract_value = committer_reveal(json!({}), json!({}), json!({}));
        contract_value["documentSchemas"]["committed"]["canBeDeleted"] = json!(false);
        let domain = &mut contract_value["documentSchemas"]["domain"];
        domain["properties"]
            .as_object_mut()
            .expect("an object")
            .remove("committerId");
        domain["properties"]["sponsorId"] = json!({
            "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32,
            "contentMediaType": "application/x.dash.dpp.identifier", "position": 6
        });
        domain["properties"]["committers"] = json!({
            "type": "array", "minItems": 0, "maxItems": 3, "position": 7,
            "items": {
                "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32,
                "contentMediaType": "application/x.dash.dpp.identifier",
                "refersTo": {
                    "type": "permanentDocument",
                    "documentType": "committed",
                    "findBy": {
                        "committerId": "sponsorId",
                        "hash": { "function": "sys.hash.sha256d", "params": params }
                    }
                }
            }
        });
        domain["required"] = json!([
            "label",
            "normalizedLabel",
            "parentDomainName",
            "preorderSalt",
            "sponsorId"
        ]);
        contract_value
    };
    assert_refused(
        contract(with_params(json!(["preorderSalt", "label"]))),
        "reads the value of \"committers\" 0 times",
    );
    contract(with_params(json!([".", "label"])))
        .expect("each element read as \".\" makes the key its own");
}

#[test]
fn should_refuse_a_function_below_protocol_version_14_and_accept_it_at_14() {
    let schema = dpns_contract(dpns_salt_reveal());
    let platform_version_13 = PlatformVersion::get(13).expect("platform version 13 should exist");

    // Meta-schema v2 knows no refersTo on a byte array, so a registering parse
    // refuses it
    contract_on(schema.clone(), true, platform_version_13)
        .expect_err("protocol version 13 should refuse the declaration");
    contract_on(schema, true, PlatformVersion::latest()).expect("protocol version 14 parses it");
}

#[test]
fn should_refuse_an_update_changing_a_function_or_what_it_demands() {
    use crate::consensus::basic::BasicError;
    use crate::consensus::ConsensusError;

    let platform_version = PlatformVersion::latest();
    let old = contract(dpns_contract(dpns_salt_reveal())).expect("the DPNS reveal should parse");
    let old_domain = old.document_type_for_name("domain").expect("the domain");

    let mut reordered = salt_reveal(
        json!({ "minimumAgeBlocks": 1, "consume": true }),
        json!({ "$ownerId": "$ownerId" }),
    );
    reordered["findBy"]["saltedDomainHash"]["params"] =
        json!(["preorderSalt", "parentDomainName", { "const": "." }, "normalizedLabel"]);

    // Documents were revealed under the declaration as written, so any change to
    // the function, the age or the consumption is an incompatible schema change
    for changed in [
        reordered,
        salt_reveal(
            json!({ "minimumAgeBlocks": 2, "consume": true }),
            json!({ "$ownerId": "$ownerId" }),
        ),
        salt_reveal(
            json!({ "minimumAgeBlocks": 1 }),
            json!({ "$ownerId": "$ownerId" }),
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
                        if e.property_path().starts_with("/properties/preorderSalt/refersTo")
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

#[test]
fn should_refuse_a_function_leaf_under_an_any_of_on_a_type_whose_documents_can_be_replaced() {
    // On a replace a function's leaf holds without a read, so an anyOf holding
    // one would hold on every replace, whichever operand held on the create
    let creator_any_of = |domain_extra: serde_json::Value| {
        let mut contract_value = dpns_contract_with_creator_reference(json!({
            "anyOf": [
                { "type": "identity" },
                reveal_with(
                    json!({ "function": "sys.hash.sha256d", "params": ["preorderSalt", "normalizedLabel"] }),
                    json!({}),
                    json!({}),
                    json!({}),
                )
            ]
        }));
        merge(
            &mut contract_value["documentSchemas"]["domain"],
            domain_extra,
        );
        contract_value
    };
    assert_refused(
        contract(creator_any_of(json!({
            "documentsMutable": true,
            "immutable": ["normalizedLabel", "parentDomainName"]
        }))),
        "cannot be an operand of an anyOf",
    );
    contract(creator_any_of(json!({})))
        .expect("on a type whose documents are never replaced the anyOf is judged once");
}

#[test]
fn should_refuse_a_plain_pair_beside_a_function_reading_a_property_a_replace_can_change() {
    // The pair is judged on the create alone, so its referring side must stay
    // what the create held
    let with_label_pair = |domain_extra: serde_json::Value| {
        dpns_contract_with(
            salt_reveal(json!({}), json!({ "tier": "label" })),
            domain_extra,
            json!({
                "properties": {
                    "saltedDomainHash": {
                        "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32,
                        "position": 0
                    },
                    "tier": { "type": "string", "maxLength": 63, "position": 1 }
                }
            }),
        )
    };
    assert_refused(
        contract(with_label_pair(json!({
            "documentsMutable": true,
            "immutable": ["normalizedLabel", "parentDomainName"]
        }))),
        "where reads \"label\" beside a findBy function",
    );
    contract(with_label_pair(json!({
        "documentsMutable": true,
        "immutable": ["label", "normalizedLabel", "parentDomainName"]
    })))
    .expect("a pair whose referring property is fixed once written holds for good");
}

#[test]
fn should_refuse_consuming_a_type_whose_delete_costs_something_or_asks_a_stricter_key() {
    let consuming = |preorder_extra: serde_json::Value| {
        contract(dpns_contract_with(
            dpns_salt_reveal(),
            json!({}),
            preorder_extra,
        ))
    };
    // A delete token cost is never charged when a create consumes
    assert_refused(
        consuming(json!({
            "tokenCost": {
                "delete": {
                    "contractId": vec![9u8; 32],
                    "tokenPosition": 0,
                    "amount": 1000
                }
            }
        })),
        "may declare no delete token cost and no delete action fee",
    );
    // Nor is a key stricter than the creating type's asked for
    assert_refused(
        consuming(json!({ "signatureSecurityLevelRequirement": 1 })),
        "may not require a stricter signature security level",
    );
    consuming(json!({})).expect("a plain commitment type may be consumed");
}

#[test]
fn should_list_the_types_a_create_may_consume() {
    let parsed = contract(dpns_contract(dpns_salt_reveal())).expect("the DPNS reveal should parse");
    let names = |type_name: &str| {
        parsed
            .document_type_for_name(type_name)
            .expect("the type")
            .consumable_document_type_names()
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>()
    };
    assert_eq!(names("domain"), vec!["preorder".to_string()]);
    assert!(names("preorder").is_empty());
}
