//! `firstAmong` (protocol version 14), beside a `findBy` function reading the
//! writer: the parse into the lookup's `LookupFirstAmong`, the checks of the
//! referring side on every parse, the checks of the referenced side at contract
//! level, the update rule and the protocol version gate. The fixture is a name
//! registration whose `preorder` holds two hashes, `ownerNameHash` (of its
//! owner and the name, unique per owner) and `nameHash` (of the name alone,
//! under a non-unique index with `$createdAtBlockHeight`), and whose transferable
//! `domain` reveals the writer's own preorder through `creatorRefersTo`.

use super::reference_test_helpers::{assert_refused, contract, contract_on, CONTRACT_ID};
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::document_type::{
    DocumentReferenceLookup, HashFunction, LookupFirstAmong, LookupHashKey, LookupKeyParam,
    LookupKeySource, PropertyReference,
};
use crate::data_contract::DataContract;
use platform_value::string_encoding::Encoding;
use platform_value::Identifier;
use platform_version::version::PlatformVersion;
use serde_json::json;

/// A 32-byte byte array at `position`.
fn hash_property(position: u32) -> serde_json::Value {
    json!({ "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32, "position": position })
}

/// The preorder: immutable, deletable, recording `$createdAtBlockHeight`, unique
/// on (`$ownerId`, `ownerNameHash`) and indexed, not uniquely, on (`nameHash`,
/// `$createdAtBlockHeight`).
fn preorder_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "documentsMutable": false,
        "canBeDeleted": true,
        "properties": {
            "ownerNameHash": hash_property(0),
            "nameHash": hash_property(1),
            "note": { "type": "string", "maxLength": 63, "position": 2 }
        },
        "indices": [
            {
                "name": "ownerAndNameHash",
                "properties": [{ "$ownerId": "asc" }, { "ownerNameHash": "asc" }],
                "unique": true
            },
            {
                "name": "nameHashByHeight",
                "properties": [{ "nameHash": "asc" }, { "$createdAtBlockHeight": "asc" }]
            }
        ],
        "required": ["$createdAtBlockHeight", "ownerNameHash", "nameHash"],
        "additionalProperties": false
    })
}

/// The domain: immutable and transferable, so its reveal sits on its creator.
fn domain_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "documentsMutable": false,
        "canBeDeleted": false,
        "transferable": 1,
        "properties": {
            "label": { "type": "string", "maxLength": 63, "position": 0 },
            "normalizedLabel": { "type": "string", "maxLength": 63, "position": 1 },
            "normalizedParentDomainName": { "type": "string", "maxLength": 63, "position": 2 },
            "count": { "type": "integer", "minimum": 0, "maximum": 100, "position": 3 },
            "salt": hash_property(4)
        },
        "required": ["label", "normalizedLabel", "normalizedParentDomainName"],
        "transient": ["salt"],
        "additionalProperties": false
    })
}

/// A function over `params`.
fn function(params: serde_json::Value) -> serde_json::Value {
    json!({ "function": "sys.hash.sha256d", "params": params })
}

/// The `findBy` function: the writer's id and the name.
fn owner_name_function() -> serde_json::Value {
    function(json!([
        "$ownerId",
        "normalizedLabel",
        { "const": "." },
        "normalizedParentDomainName"
    ]))
}

/// The `firstAmong` function: the name alone.
fn name_function() -> serde_json::Value {
    function(json!([
        "normalizedLabel",
        { "const": "." },
        "normalizedParentDomainName"
    ]))
}

/// The domain's reveal: the writer's own preorder, from an earlier block, the
/// first among those to the same name.
fn reveal() -> serde_json::Value {
    json!({
        "type": "deletableDocument",
        "documentType": "preorder",
        "findBy": { "$ownerId": "$ownerId", "ownerNameHash": owner_name_function() },
        "firstAmong": { "nameHash": name_function() },
        "minimumAgeBlocks": 1
    })
}

/// A contract whose `domain` declares `creator_refers_to`, with `preorder_change`
/// applied to the preorder.
fn contract_with(
    creator_refers_to: serde_json::Value,
    preorder_change: impl FnOnce(&mut serde_json::Value),
) -> serde_json::Value {
    let mut domain = domain_schema();
    domain["creatorRefersTo"] = creator_refers_to;
    let mut preorder = preorder_schema();
    preorder_change(&mut preorder);
    json!({
        "$formatVersion": "1",
        "id": Identifier::from(CONTRACT_ID).to_string(Encoding::Base58),
        "ownerId": Identifier::from([8; 32]).to_string(Encoding::Base58),
        "version": 1,
        "documentSchemas": { "preorder": preorder, "domain": domain }
    })
}

fn contract_revealing(creator_refers_to: serde_json::Value) -> serde_json::Value {
    contract_with(creator_refers_to, |_| {})
}

/// The reveal with `change` applied.
fn reveal_with(change: impl FnOnce(&mut serde_json::Value)) -> serde_json::Value {
    let mut reveal = reveal();
    change(&mut reveal);
    reveal
}

/// The lookup of the domain's `creatorRefersTo`.
fn creator_lookup(contract: &DataContract) -> DocumentReferenceLookup {
    let domain = contract
        .document_type_for_name("domain")
        .expect("the domain document type");
    let declarations = domain.reference_declarations().collect::<Vec<_>>();
    let [(holder, PropertyReference::Value(target))] = declarations.as_slice() else {
        panic!("expected the creator's reference alone, got {declarations:?}");
    };
    assert_eq!(holder.path(), "$creatorId");
    target
        .as_any_document_reference()
        .and_then(|declaration| declaration.lookup)
        .cloned()
        .expect("the creator's lookup")
}

/// Both parses refuse `contract_value`, the validating one with the meta-schema
/// or the parser, the one that skips the meta-schema (a stored contract) with
/// the parser's `fragment`: the rule holds on every parse.
fn assert_refused_on_every_parse(contract_value: serde_json::Value, fragment: &str) {
    contract(contract_value.clone()).expect_err("a validating parse should refuse it");
    assert_refused(
        contract_on(contract_value, false, PlatformVersion::latest()),
        fragment,
    );
}

/// Both parses refuse `contract_value` with the parser's `fragment`.
fn assert_refused_by_the_parser_on_every_parse(contract_value: serde_json::Value, fragment: &str) {
    for full_validation in [true, false] {
        assert_refused(
            contract_on(
                contract_value.clone(),
                full_validation,
                PlatformVersion::latest(),
            ),
            fragment,
        );
    }
}

#[test]
fn should_parse_first_among_beside_a_find_by_function_reading_the_writer() {
    for full_validation in [true, false] {
        let parsed = contract_on(
            contract_revealing(reveal()),
            full_validation,
            PlatformVersion::latest(),
        )
        .expect("the reveal should parse");
        let lookup = creator_lookup(&parsed);
        assert_eq!(
            lookup.first_among,
            Some(LookupFirstAmong {
                property: "nameHash".to_string(),
                key: LookupHashKey {
                    function: HashFunction::Sha256d,
                    params: vec![
                        LookupKeyParam::Property("normalizedLabel".to_string()),
                        LookupKeyParam::Const(".".to_string()),
                        LookupKeyParam::Property("normalizedParentDomainName".to_string()),
                    ],
                },
            })
        );
        assert_eq!(lookup.keys.get("$ownerId"), Some(&LookupKeySource::OwnerId));
        assert!(matches!(
            lookup.keys.get("ownerNameHash"),
            Some(LookupKeySource::Hash(key)) if key.params[0] == LookupKeyParam::Writer
        ));
        assert!(lookup.is_checked_on_create_only(true));
        assert_eq!(lookup.minimum_age_blocks, Some(1));
    }
}

/// The parsed model serializes `firstAmong` as the schema spells it.
#[test]
fn should_serialize_first_among_as_the_schema_spells_it() {
    let lookup = creator_lookup(&contract(contract_revealing(reveal())).expect("parses"));
    let serialized = serde_json::to_value(&lookup).expect("the lookup serializes");
    assert_eq!(
        serialized["firstAmong"],
        json!({ "nameHash": name_function() })
    );
    assert_eq!(
        serialized["keys"]["ownerNameHash"],
        owner_name_function(),
        "the writer param serializes as \"$ownerId\""
    );
}

#[test]
fn should_refuse_first_among_without_a_find_by_function_reading_the_writer() {
    // A findBy function hashing the name alone: a competitor cannot be judged
    let without_writer = reveal_with(|reveal| {
        reveal["findBy"]["ownerNameHash"] = name_function();
        reveal["firstAmong"] = json!({
            "nameHash": function(json!(["normalizedLabel", { "const": "/" }, "normalizedParentDomainName"]))
        });
    });
    assert_refused_by_the_parser_on_every_parse(
        contract_revealing(without_writer),
        "findBy must hold a function reading \"$ownerId\"",
    );
}

#[test]
fn should_refuse_first_among_without_the_writer_find_by_entry() {
    // The writer hashed but not keyed: the commitment found need not be the
    // writer's own, whose owner a competitor's is judged against
    let without_owner_entry = reveal_with(|reveal| {
        reveal["findBy"] = json!({ "ownerNameHash": owner_name_function() });
    });
    assert_refused_by_the_parser_on_every_parse(
        contract_with(without_owner_entry, |preorder| {
            preorder["indices"][0] = json!({
                "name": "ownerNameHash",
                "properties": [{ "ownerNameHash": "asc" }],
                "unique": true
            });
        }),
        "needs the findBy entry \"$ownerId\": \"$ownerId\"",
    );
}

#[test]
fn should_refuse_a_find_by_function_reading_dot_beside_first_among() {
    // On the creator "." is the creator, which a competitor's owner can not
    // stand in for
    let reads_dot = reveal_with(|reveal| {
        reveal["findBy"]["ownerNameHash"] = function(json!([
            "$ownerId",
            ".",
            "normalizedLabel",
            { "const": "." },
            "normalizedParentDomainName"
        ]));
    });
    assert_refused_by_the_parser_on_every_parse(
        contract_revealing(reads_dot),
        "so the function may not read \".\"",
    );
}

#[test]
fn should_refuse_a_first_among_function_reading_the_writer_or_dot() {
    for params in [
        json!([
            "$ownerId",
            "normalizedLabel",
            { "const": "." },
            "normalizedParentDomainName"
        ]),
        json!([
            ".",
            "normalizedLabel",
            { "const": "." },
            "normalizedParentDomainName"
        ]),
    ] {
        let reveal = reveal_with(|reveal| {
            reveal["firstAmong"]["nameHash"] = function(params.clone());
        });
        assert_refused_by_the_parser_on_every_parse(
            contract_revealing(reveal),
            "may read neither \"$ownerId\" nor \".\"",
        );
    }
}

#[test]
fn should_refuse_a_first_among_function_leaving_out_a_value_the_find_by_function_reads() {
    // A salt in the findBy hash alone would make every competitor look like a
    // copy: the check would never refuse
    let salted = reveal_with(|reveal| {
        reveal["findBy"]["ownerNameHash"] = function(json!([
            "$ownerId",
            "salt",
            "normalizedLabel",
            { "const": "." },
            "normalizedParentDomainName"
        ]));
    });
    assert_refused_by_the_parser_on_every_parse(
        contract_revealing(salted),
        "must read every property the findBy function \"ownerNameHash\" reads, and does not \
         read \"salt\"",
    );
}

#[test]
fn should_refuse_first_among_params_the_referring_type_cannot_supply() {
    for (params, fragment) in [
        (
            json!(["count", "normalizedLabel", { "const": "." }, "normalizedParentDomainName"]),
            "param \"count\" is not a string, a byte array or an identifier",
        ),
        (
            json!(["normalizedLabel", { "const": "." }, "ghost", "normalizedParentDomainName"]),
            "param \"ghost\" is not a property of the referring document type",
        ),
        (
            json!(["normalizedLabel", "normalizedParentDomainName"]),
            "param \"normalizedLabel\" has no fixed length and another such param follows it",
        ),
    ] {
        let reveal = reveal_with(|reveal| {
            reveal["firstAmong"]["nameHash"] = function(params.clone());
        });
        assert_refused_by_the_parser_on_every_parse(
            contract_revealing(reveal),
            &format!("firstAmong function \"nameHash\" {fragment}"),
        );
    }
}

#[test]
fn should_refuse_first_among_naming_a_property_find_by_names() {
    let overlapping = reveal_with(|reveal| {
        reveal["firstAmong"] = json!({ "ownerNameHash": name_function() });
    });
    assert_refused_by_the_parser_on_every_parse(
        contract_revealing(overlapping),
        "firstAmong \"ownerNameHash\" names a property findBy names",
    );
}

#[test]
fn should_refuse_malformed_first_among_declarations() {
    for (first_among, fragment) in [
        (
            json!({ "nameHash": name_function(), "otherHash": name_function() }),
            "firstAmong holds exactly one entry",
        ),
        (json!({}), "firstAmong holds exactly one entry"),
        (
            json!({ "nameHash": "normalizedLabel" }),
            "firstAmong \"nameHash\" maps to a function",
        ),
        (
            json!({ "$createdAt": name_function() }),
            "firstAmong names a schema property of the referenced document type",
        ),
        (
            json!({ "nameHash": { "function": "sys.hash.sha256d", "params": [] } }),
            "lists between 1 and 16 params",
        ),
    ] {
        let reveal = reveal_with(|reveal| reveal["firstAmong"] = first_among.clone());
        assert_refused_on_every_parse(contract_revealing(reveal), fragment);
    }
}

/// `firstAmong` describes the document `findBy` finds, and needs it; beside `inList` it is
/// refused with `minimumAgeBlocks` and `consume` (`list_element_reference_tests`).
#[test]
fn should_refuse_first_among_without_find_by() {
    let without_find_by = json!({
        "type": "deletableDocument",
        "documentType": "preorder",
        "firstAmong": { "nameHash": name_function() }
    });
    assert_refused_on_every_parse(
        contract_revealing(without_find_by),
        "does not take firstAmong without findBy",
    );
}

/// What `firstAmong` names on the referenced type is checked when the contract
/// enters the chain, with the other referenced-side rules of a `findBy`
/// (`DocumentReferenceLookup::referenced_side_error`, under full validation for
/// a type of the same contract, and at registration for one of another).
#[test]
fn should_refuse_first_among_a_referenced_type_cannot_answer() {
    type PreorderChange = Box<dyn FnOnce(&mut serde_json::Value)>;
    let cases: Vec<(PreorderChange, &str)> = vec![
        (
            Box::new(|preorder| {
                preorder["properties"]["nameHash"] =
                    json!({ "type": "string", "maxLength": 63, "position": 1 })
            }),
            "\"nameHash\" must be a byte array of exactly 32 bytes of \"preorder\"",
        ),
        (
            Box::new(|preorder| {
                preorder["properties"]["nameHash"] = json!({
                    "type": "array", "byteArray": true, "minItems": 20, "maxItems": 32,
                    "position": 1
                })
            }),
            "\"nameHash\" must be a byte array of exactly 32 bytes of \"preorder\"",
        ),
        (
            Box::new(|preorder| {
                preorder["indices"][1]["properties"] =
                    json!([{ "$createdAtBlockHeight": "asc" }, { "nameHash": "asc" }])
            }),
            "through a non-unique index of \"preorder\" over exactly (nameHash, \
             $createdAtBlockHeight), in that order, which it does not have",
        ),
        (
            Box::new(|preorder| {
                preorder["indices"][1]["properties"] = json!([
                    { "nameHash": "asc" },
                    { "$createdAtBlockHeight": "asc" },
                    { "note": "asc" }
                ])
            }),
            "which it does not have",
        ),
        (
            Box::new(|preorder| {
                preorder["indices"][1]["properties"] = json!([{ "nameHash": "asc" }])
            }),
            "which it does not have",
        ),
        (
            Box::new(|preorder| preorder["indices"][1]["unique"] = json!(true)),
            "index \"nameHashByHeight\" of \"preorder\" over (nameHash, $createdAtBlockHeight) is \
             unique",
        ),
        (
            Box::new(|preorder| {
                preorder["documentsMutable"] = json!(true);
                preorder["immutable"] = json!(["ownerNameHash"]);
            }),
            "firstAmong names \"nameHash\" of \"preorder\", which a replace can change",
        ),
    ];
    for (change, fragment) in cases {
        assert_refused(contract(contract_with(reveal(), change)), fragment);
    }

    // Without `$createdAtBlockHeight` the commitments have no order (and no
    // minimum age to meet, so the reveal declares none here)
    let without_age = reveal_with(|reveal| {
        reveal
            .as_object_mut()
            .expect("an object")
            .remove("minimumAgeBlocks");
    });
    assert_refused(
        contract(contract_with(without_age, |preorder| {
            preorder["required"] = json!(["ownerNameHash", "nameHash"])
        })),
        "firstAmong orders commitments by their $createdAtBlockHeight, which \"preorder\" does \
         not record",
    );
}

/// Documents were revealed under the declaration as written, so adding,
/// removing or changing `firstAmong` is an incompatible schema change.
#[test]
fn should_refuse_an_update_adding_removing_or_changing_first_among() {
    use crate::consensus::basic::BasicError;
    use crate::consensus::ConsensusError;

    let platform_version = PlatformVersion::latest();
    let without_first_among = reveal_with(|reveal| {
        reveal
            .as_object_mut()
            .expect("an object")
            .remove("firstAmong");
    });
    let reordered = reveal_with(|reveal| {
        reveal["firstAmong"]["nameHash"] = function(json!([
            "normalizedParentDomainName",
            { "const": "." },
            "normalizedLabel"
        ]));
    });
    for (old, new) in [
        (without_first_among.clone(), reveal()),
        (reveal(), without_first_among),
        (reveal(), reordered),
    ] {
        let old = contract(contract_revealing(old)).expect("the old contract parses");
        let new = contract(contract_revealing(new)).expect("the new contract parses");
        let result = old
            .document_type_for_name("domain")
            .expect("the domain")
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
                        if e.property_path().starts_with("/creatorRefersTo")
                )),
            "{:?}",
            result.errors
        );
    }
}

#[test]
fn should_refuse_first_among_below_protocol_version_14_and_accept_it_at_14() {
    let schema = contract_revealing(reveal());
    let platform_version_13 = PlatformVersion::get(13).expect("platform version 13 should exist");
    contract_on(schema.clone(), true, platform_version_13)
        .expect_err("protocol version 13 should refuse the declaration");
    contract_on(schema, true, PlatformVersion::latest()).expect("protocol version 14 parses it");
}

/// A create must carry every value the `firstAmong` function reads, as for a `findBy`
/// function: one missing is refused before any read, here a `tag` only `firstAmong` reads.
#[test]
fn should_refuse_a_create_missing_a_param_of_the_first_among_function() {
    use crate::data_contract::document_type::first_unrevealable_lookup_key;
    use platform_value::Value;
    use std::collections::BTreeMap;

    let tagged = reveal_with(|reveal| {
        reveal["firstAmong"]["nameHash"] = function(json!([
            "normalizedLabel",
            { "const": "." },
            "normalizedParentDomainName",
            { "const": "/" },
            "tag"
        ]));
    });
    let mut contract_value = contract_revealing(tagged);
    contract_value["documentSchemas"]["domain"]["properties"]["tag"] =
        json!({ "type": "string", "maxLength": 63, "position": 5 });
    let parsed = contract(contract_value).expect("the tagged reveal should parse");
    let domain = parsed
        .document_type_for_name("domain")
        .expect("the domain document type");
    let mut data = BTreeMap::from([
        ("label".to_string(), Value::Text("Al1ce".to_string())),
        (
            "normalizedLabel".to_string(),
            Value::Text("al1ce".to_string()),
        ),
        (
            "normalizedParentDomainName".to_string(),
            Value::Text("dash".to_string()),
        ),
    ]);
    let writer = Identifier::from([3; 32]);

    assert!(matches!(
        first_unrevealable_lookup_key(domain, &data, writer),
        Some((path, error)) if path == "$creatorId" && error.param == "tag"
    ));
    data.insert("tag".to_string(), Value::Text("t".to_string()));
    assert_eq!(first_unrevealable_lookup_key(domain, &data, writer), None);
}
