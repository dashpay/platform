//! `chooseFirst` (protocol version 14), beside a `findBy` function reading the
//! writer: the parse into the lookup's `LookupChooseFirst`, the checks of the
//! referring side on every parse, the checks of the referenced side at contract
//! level, the update rule, the create structure check and the protocol version
//! gate. The fixture is a name registration whose `preorder` holds two hashes,
//! `ownerNameHash` (of its owner and the name, unique per owner) and `nameHash`
//! (of the name alone, unique per owner too, and indexed with
//! `$createdAtBlockHeight`), and expires; its transferable `domain` reveals the
//! writer's own preorder through `creatorRefersTo`.

use super::reference_test_helpers::{
    assert_refused, assert_refused_by_the_parser, contract, contract_on, CONTRACT_ID,
};
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::document_type::{
    first_unrevealable_lookup_key, ChooseFirstOrder, DocumentReferenceLookup, HashFunction,
    LookupChooseFirst, LookupHashKey, LookupKeyParam, LookupKeySource, PropertyReference,
};
use crate::data_contract::DataContract;
use platform_value::string_encoding::Encoding;
use platform_value::{Identifier, Value};
use platform_version::version::PlatformVersion;
use serde_json::json;
use std::collections::BTreeMap;

/// A 32-byte byte array at `position`.
fn hash_property(position: u32) -> serde_json::Value {
    json!({ "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32, "position": position })
}

/// The preorder: immutable, deletable, expiring after an hour, recording
/// `$createdAt` and `$createdAtBlockHeight`, unique on (`$ownerId`,
/// `ownerNameHash`) and on (`$ownerId`, `nameHash`), and indexed, not uniquely,
/// on (`nameHash`, `$createdAtBlockHeight`).
fn preorder_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "documentsMutable": false,
        "canBeDeleted": true,
        "ttl": 3600,
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
                "name": "ownerAndName",
                "properties": [{ "$ownerId": "asc" }, { "nameHash": "asc" }],
                "unique": true
            },
            {
                "name": "nameHashByHeight",
                "properties": [{ "nameHash": "asc" }, { "$createdAtBlockHeight": "asc" }]
            }
        ],
        "required": ["$createdAt", "$createdAtBlockHeight", "ownerNameHash", "nameHash"],
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
            "salt": hash_property(4),
            "tag": { "type": "string", "maxLength": 63, "position": 5 },
            "claim": hash_property(6),
            "aliases": {
                "type": "array",
                "maxItems": 4,
                "items": {
                    "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier"
                },
                "position": 7
            }
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

/// The `chooseFirst` function: the name alone.
fn name_function() -> serde_json::Value {
    function(json!([
        "normalizedLabel",
        { "const": "." },
        "normalizedParentDomainName"
    ]))
}

/// The `chooseFirst` of the name, ordered by `by`.
fn choose_first_by(by: serde_json::Value) -> serde_json::Value {
    json!({ "property": "nameHash", "value": name_function(), "by": by })
}

/// The domain's reveal: the writer's own preorder, from an earlier block, chosen first
/// among those to the same name by their creation height.
fn reveal() -> serde_json::Value {
    json!({
        "type": "deletableDocument",
        "documentType": "preorder",
        "findBy": { "$ownerId": "$ownerId", "ownerNameHash": owner_name_function() },
        "chooseFirst": choose_first_by(json!(["$createdAtBlockHeight"])),
        "minimumAgeBlocks": 1
    })
}

/// A contract whose `domain` declares `creator_refers_to`, with `domain_change`
/// applied to the domain and `preorder_change` to the preorder.
fn contract_changed(
    creator_refers_to: serde_json::Value,
    domain_change: impl FnOnce(&mut serde_json::Value),
    preorder_change: impl FnOnce(&mut serde_json::Value),
) -> serde_json::Value {
    let mut domain = domain_schema();
    domain["creatorRefersTo"] = creator_refers_to;
    domain_change(&mut domain);
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

/// A contract whose `domain` declares `creator_refers_to`, with `preorder_change`
/// applied to the preorder.
fn contract_with(
    creator_refers_to: serde_json::Value,
    preorder_change: impl FnOnce(&mut serde_json::Value),
) -> serde_json::Value {
    contract_changed(creator_refers_to, |_| {}, preorder_change)
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
fn should_parse_choose_first_beside_a_find_by_function_reading_the_writer() {
    for full_validation in [true, false] {
        let parsed = contract_on(
            contract_revealing(reveal()),
            full_validation,
            PlatformVersion::latest(),
        )
        .expect("the reveal should parse");
        let lookup = creator_lookup(&parsed);
        assert_eq!(
            lookup.choose_first,
            Some(LookupChooseFirst {
                property: "nameHash".to_string(),
                key: LookupHashKey {
                    function: HashFunction::Sha256d,
                    params: vec![
                        LookupKeyParam::Property("normalizedLabel".to_string()),
                        LookupKeyParam::Const(".".to_string()),
                        LookupKeyParam::Property("normalizedParentDomainName".to_string()),
                    ],
                },
                by: vec![ChooseFirstOrder {
                    property: "$createdAtBlockHeight".to_string(),
                    ascending: true,
                }],
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

/// The parsed model serializes `chooseFirst` as the schema spells it.
#[test]
fn should_serialize_choose_first_as_the_schema_spells_it() {
    let lookup = creator_lookup(&contract(contract_revealing(reveal())).expect("parses"));
    let serialized = serde_json::to_value(&lookup).expect("the lookup serializes");
    assert_eq!(
        serialized["chooseFirst"],
        json!({
            "property": "nameHash",
            "value": name_function(),
            "by": [{ "$createdAtBlockHeight": "asc" }]
        })
    );
    assert_eq!(
        serialized["keys"]["ownerNameHash"],
        owner_name_function(),
        "the writer param serializes as \"$ownerId\""
    );
}

#[test]
fn should_refuse_choose_first_without_a_find_by_function_reading_the_writer() {
    // A findBy function hashing the name alone: a competitor cannot be judged
    let without_writer = reveal_with(|reveal| {
        reveal["findBy"]["ownerNameHash"] = name_function();
        reveal["chooseFirst"]["value"] = function(json!([
            "normalizedLabel",
            { "const": "/" },
            "normalizedParentDomainName"
        ]));
    });
    assert_refused_by_the_parser_on_every_parse(
        contract_revealing(without_writer),
        "findBy must hold a function reading \"$ownerId\"",
    );
}

/// `findBy` beside `chooseFirst` holds the writer entry and its function and nothing
/// else: without the writer entry the commitment found need not be the writer's own,
/// and another entry (a property or `"."` source) would be a part of the commitment a
/// competitor is never compared on.
#[test]
fn should_refuse_choose_first_beside_a_find_by_other_than_the_writer_and_its_function() {
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
        "needs findBy to hold the entry \"$ownerId\": \"$ownerId\" beside its function and \
         nothing else",
    );

    // The parent read as is beside the function: a commitment under another parent
    // would otherwise precede the create
    let with_parent_entry = reveal_with(|reveal| {
        reveal["findBy"] = json!({
            "$ownerId": "$ownerId",
            "note": "normalizedParentDomainName",
            "ownerNameHash": function(json!(["$ownerId", "normalizedLabel"]))
        });
        reveal["chooseFirst"]["value"] = function(json!(["normalizedLabel"]));
    });
    assert_refused_by_the_parser_on_every_parse(
        contract_with(with_parent_entry, |preorder| {
            preorder["indices"][0] = json!({
                "name": "ownerParentAndNameHash",
                "properties": [{ "$ownerId": "asc" }, { "note": "asc" }, { "ownerNameHash": "asc" }],
                "unique": true
            });
            preorder["required"] = json!([
                "$createdAt",
                "$createdAtBlockHeight",
                "ownerNameHash",
                "nameHash",
                "note"
            ]);
        }),
        "needs findBy to hold the entry \"$ownerId\": \"$ownerId\" beside its function and \
         nothing else",
    );
}

/// A `where` beside `chooseFirst` compares nothing but the writer with the
/// commitment's owner, and holds no condition.
#[test]
fn should_refuse_a_where_entry_beside_choose_first_other_than_the_writer() {
    let reveal_where = |entries: serde_json::Value| {
        reveal_with(|reveal| {
            reveal["where"] = entries;
        })
    };
    contract(contract_revealing(reveal_where(
        json!({ "$ownerId": "$ownerId" }),
    )))
    .expect("the writer entry is the commitment's own owner");
    assert_refused_by_the_parser_on_every_parse(
        contract_revealing(reveal_where(json!({ "note": "label" }))),
        "where \"note\": \"label\" beside chooseFirst",
    );
    // A condition the found commitment meets, which a competitor is never asked to
    assert_refused_by_the_parser_on_every_parse(
        contract_revealing(reveal_where(json!({
            "$ownerId": "$ownerId",
            "note": { "const": "first" }
        }))),
        "where \"note\" holds a condition beside chooseFirst",
    );
}

#[test]
fn should_refuse_a_find_by_function_reading_dot_beside_choose_first() {
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
fn should_refuse_a_choose_first_function_reading_the_writer_or_dot() {
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
            reveal["chooseFirst"]["value"] = function(params.clone());
        });
        assert_refused_by_the_parser_on_every_parse(
            contract_revealing(reveal),
            "may read neither \"$ownerId\" nor \".\"",
        );
    }
}

/// The two functions read the same properties: a salt only the findBy function
/// read would make every competitor look like a copy, a `tag` only the
/// chooseFirst function read would let a later committer pick another one and
/// stand where no competitor exists, and a findBy function over the writer
/// alone would let anyone copying a pending commitment's name hash, without
/// knowing the name, hold a valid competitor.
#[test]
fn should_refuse_functions_reading_different_properties() {
    let owner_only = reveal_with(|reveal| {
        reveal["findBy"]["ownerNameHash"] = function(json!(["$ownerId"]));
    });
    let salted = reveal_with(|reveal| {
        reveal["findBy"]["ownerNameHash"] = function(json!([
            "$ownerId",
            "salt",
            "normalizedLabel",
            { "const": "." },
            "normalizedParentDomainName"
        ]));
    });
    let tagged = reveal_with(|reveal| {
        reveal["chooseFirst"]["value"] = function(json!([
            "normalizedLabel",
            { "const": "." },
            "normalizedParentDomainName",
            { "const": "/" },
            "tag"
        ]));
    });
    for (reveal, only_one_reads) in [
        (salted, "salt"),
        (tagged, "tag"),
        (owner_only, "normalizedLabel"),
    ] {
        assert_refused_by_the_parser_on_every_parse(
            contract_revealing(reveal),
            &format!(
                "must read the properties the findBy function \"ownerNameHash\" reads and no \
                 others, and \"{only_one_reads}\" is read by one of them only"
            ),
        );
    }
}

#[test]
fn should_refuse_choose_first_params_the_referring_type_cannot_supply() {
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
            reveal["chooseFirst"]["value"] = function(params.clone());
        });
        assert_refused_by_the_parser_on_every_parse(
            contract_revealing(reveal),
            &format!("chooseFirst function \"nameHash\" {fragment}"),
        );
    }
}

#[test]
fn should_refuse_choose_first_naming_a_property_find_by_names() {
    let overlapping = reveal_with(|reveal| {
        reveal["chooseFirst"]["property"] = json!("ownerNameHash");
    });
    assert_refused_by_the_parser_on_every_parse(
        contract_revealing(overlapping),
        "chooseFirst \"ownerNameHash\" names a property findBy names",
    );
}

/// A malformed `chooseFirst` is refused naming `chooseFirst`, not `findBy`.
#[test]
fn should_refuse_malformed_choose_first_declarations() {
    let by_height = json!(["$createdAtBlockHeight"]);
    for (choose_first, fragment) in [
        (
            json!({ "property": "nameHash", "value": name_function(), "by": by_height, "extra": 1 }),
            "chooseFirst holds \"property\", \"value\" and \"by\" only",
        ),
        (
            json!({ "value": name_function(), "by": by_height }),
            "chooseFirst names in \"property\" a schema property",
        ),
        (
            json!({ "property": "$createdAt", "value": name_function(), "by": by_height }),
            "chooseFirst names in \"property\" a schema property",
        ),
        (
            json!({ "property": "nameHash", "value": "normalizedLabel", "by": by_height }),
            "chooseFirst \"nameHash\" takes in \"value\" a function",
        ),
        (
            json!({ "property": "nameHash", "value": name_function() }),
            "chooseFirst \"nameHash\" lists in \"by\" the properties",
        ),
        (
            json!({ "property": "nameHash", "value": name_function(), "by": [] }),
            "chooseFirst \"nameHash\" orders by 1 to 10 properties",
        ),
        (
            choose_first_by(json!([{ "$createdAtBlockHeight": "up" }])),
            "chooseFirst \"nameHash\" orders by a property name, ascending",
        ),
        (
            choose_first_by(json!([{ "$createdAtBlockHeight": "asc", "$id": "asc" }])),
            "chooseFirst \"nameHash\" orders by a property name, ascending",
        ),
        (
            choose_first_by(json!(["$createdAtBlockHeight", "$createdAtBlockHeight"])),
            "chooseFirst \"nameHash\" orders by \"$createdAtBlockHeight\" twice",
        ),
        (
            choose_first_by(json!(["nameHash"])),
            "chooseFirst \"nameHash\" orders by \"nameHash\" twice, or by the property it fills",
        ),
        (
            choose_first_by(json!(["$id", "$createdAtBlockHeight"])),
            "chooseFirst \"nameHash\" orders by $id last only",
        ),
        (
            json!({
                "property": "nameHash",
                "value": { "function": "sys.hash.sha256d", "params": [] },
                "by": by_height
            }),
            "a refersTo chooseFirst function lists between 1 and 16 params",
        ),
        (
            json!({
                "property": "nameHash",
                "value": { "function": "sys.hash.sha256d", "params": ["$creatorId"] },
                "by": by_height
            }),
            "a refersTo chooseFirst function param is",
        ),
    ] {
        let reveal = reveal_with(|reveal| reveal["chooseFirst"] = choose_first.clone());
        assert_refused_by_the_parser(contract_revealing(reveal), fragment);
    }
}

/// `chooseFirst` describes the document `findBy` finds, and needs it; beside `inList` it is
/// refused with `minimumAgeBlocks` and `consume` (`list_element_reference_tests`). It reads
/// commitments that expire, so only on a `deletableDocument` reference.
#[test]
fn should_refuse_choose_first_without_find_by_or_on_a_permanent_reference() {
    let without_find_by = json!({
        "type": "deletableDocument",
        "documentType": "preorder",
        "chooseFirst": choose_first_by(json!(["$createdAtBlockHeight"]))
    });
    assert_refused_by_the_parser(
        contract_revealing(without_find_by),
        "does not take chooseFirst without findBy",
    );

    let permanent = reveal_with(|reveal| reveal["type"] = json!("permanentDocument"));
    assert_refused_by_the_parser(
        contract_revealing(permanent),
        "permanentDocument refersTo does not take chooseFirst",
    );
}

/// A create reads the competitors of each `chooseFirst` it judges, so one sits on one
/// single reference per document type: not on the elements of a typed array, not on an
/// operand of a reference expression, and not twice.
#[test]
fn should_refuse_choose_first_on_elements_operands_or_twice() {
    let on_elements = contract_changed(
        reveal(),
        |domain| {
            domain
                .as_object_mut()
                .expect("an object")
                .remove("creatorRefersTo");
            domain["properties"]["aliases"]["items"]["refersTo"] = reveal_with(|reveal| {
                reveal["findBy"]["identityHash"] = json!(".");
            });
        },
        |_| {},
    );
    assert_refused_by_the_parser(on_elements, "chooseFirst may not sit on the elements");

    let on_operand = contract_revealing(json!({
        "anyOf": [{ "type": "identity" }, reveal()]
    }));
    assert_refused_by_the_parser(
        on_operand,
        "chooseFirst may not sit on an operand of a reference expression",
    );

    let twice = contract_changed(
        reveal(),
        |domain| {
            domain["properties"]["claim"]["refersTo"] = reveal_with(|reveal| {
                reveal["findBy"]["ownerNameHash"] =
                    function(json!(["$ownerId", "claim", "normalizedLabel"]));
                reveal["chooseFirst"]["value"] = function(json!(["claim", "normalizedLabel"]));
            });
        },
        |_| {},
    );
    assert_refused_by_the_parser(twice, "a document type declares at most one chooseFirst");
}

/// A create relying on a gas sponsor is judged, state included, before anyone pays for
/// it, which the competitor scan must not be.
#[test]
fn should_refuse_choose_first_on_a_type_whose_creates_offer_gas_sponsorship() {
    let sponsored = contract_changed(
        reveal(),
        |domain| {
            domain["tokenCost"] = json!({
                "create": {
                    "contractId": Identifier::from([9; 32]).to_string(Encoding::Base58),
                    "tokenPosition": 0,
                    "amount": 10,
                    "effect": 0,
                    "gasFeesPaidBy": 1
                }
            });
        },
        |_| {},
    );
    assert_refused_by_the_parser(sponsored, "its creates may not offer gas sponsorship");
}

/// What `chooseFirst` names on the referenced type is checked when the contract
/// enters the chain, with the other referenced-side rules of a `findBy`
/// (`DocumentReferenceLookup::referenced_side_error`, under full validation for
/// a type of the same contract, and at registration for one of another).
#[test]
fn should_refuse_choose_first_a_referenced_type_cannot_answer() {
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
                preorder["indices"][2]["properties"] =
                    json!([{ "$createdAtBlockHeight": "asc" }, { "nameHash": "asc" }])
            }),
            "in order through an index of \"preorder\" over exactly (nameHash, \
             $createdAtBlockHeight), in that order, which it does not have",
        ),
        (
            Box::new(|preorder| {
                preorder["indices"][2]["properties"] = json!([
                    { "nameHash": "asc" },
                    { "$createdAtBlockHeight": "asc" },
                    { "note": "asc" }
                ])
            }),
            "which it does not have",
        ),
        // A unique order index without $ownerId would let a copy landing first keep the
        // commitment out
        (
            Box::new(|preorder| preorder["indices"][2]["unique"] = json!(true)),
            "index \"nameHashByHeight\" of \"preorder\" is unique over \"nameHash\" without \
             $ownerId",
        ),
        (
            Box::new(|preorder| {
                preorder["documentsMutable"] = json!(true);
                preorder["immutable"] = json!(["ownerNameHash"]);
            }),
            "chooseFirst names \"nameHash\" of \"preorder\", which a replace can change",
        ),
        // Without a ttl a commitment never revealed would precede every later one for good
        (
            Box::new(|preorder| {
                preorder.as_object_mut().expect("an object").remove("ttl");
            }),
            "chooseFirst reads the commitments of \"preorder\", whose documents must expire",
        ),
        // A copy of a pending commitment's value landing first would keep it out
        (
            Box::new(|preorder| {
                preorder["indices"][1] = json!({
                    "name": "nameHashAndNote",
                    "properties": [{ "nameHash": "asc" }, { "note": "asc" }],
                    "unique": true
                })
            }),
            "index \"nameHashAndNote\" of \"preorder\" is unique over \"nameHash\" without \
             $ownerId",
        ),
        // So would a copy of its owner-bound hash, which the copier can not compute
        (
            Box::new(|preorder| {
                preorder["indices"]
                    .as_array_mut()
                    .expect("an array")
                    .push(json!({
                        "name": "ownerNameHashAlone",
                        "properties": [{ "ownerNameHash": "asc" }],
                        "unique": true
                    }))
            }),
            "index \"ownerNameHashAlone\" of \"preorder\" is unique over \"ownerNameHash\" \
             without $ownerId",
        ),
        // One identity could hold many commitments to one value
        (
            Box::new(|preorder| {
                preorder["indices"][1]["unique"] = json!(false);
            }),
            "chooseFirst \"nameHash\" needs a unique index of \"preorder\" over exactly \
             ($ownerId, nameHash)",
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
            preorder["required"] = json!(["$createdAt", "ownerNameHash", "nameHash"])
        })),
        "chooseFirst \"nameHash\" orders by \"$createdAtBlockHeight\" of \"preorder\", which is \
         not recorded on every document",
    );
}

/// `by` orders through the index over the property followed by the `by` properties, each
/// recorded on every commitment and fixed once written, in either direction; a last `$id`
/// stands outside that index, which must then not be unique.
#[test]
fn should_order_choose_first_through_its_index() {
    let ordered_by =
        |by: serde_json::Value| reveal_with(move |reveal| reveal["chooseFirst"]["by"] = by);
    for by in [
        json!([{ "$createdAtBlockHeight": "desc" }]),
        json!(["$createdAtBlockHeight", "$id"]),
        json!([{ "$createdAtBlockHeight": "asc" }, { "$id": "desc" }]),
    ] {
        contract(contract_revealing(ordered_by(by.clone())))
            .unwrap_or_else(|error| panic!("{by} should order the commitments: {error}"));
    }
    // A second index level, $ownerId always recorded and fixed on an immutable,
    // non-transferable type
    contract(contract_with(
        ordered_by(json!(["$createdAtBlockHeight", { "$ownerId": "desc" }])),
        |preorder| {
            preorder["indices"][2]["properties"] = json!([
                { "nameHash": "asc" },
                { "$createdAtBlockHeight": "asc" },
                { "$ownerId": "asc" }
            ])
        },
    ))
    .expect("two ordered index properties");

    // $id alone orders through an index over the property alone
    assert_refused(
        contract(contract_revealing(ordered_by(json!(["$id"])))),
        "over exactly (nameHash), in that order, which it does not have",
    );
    // A unique index keeps no entries by $id
    assert_refused(
        contract(contract_with(
            ordered_by(json!(["$createdAtBlockHeight", "$id"])),
            |preorder| {
                preorder["indices"][2]["properties"] = json!([
                    { "nameHash": "asc" },
                    { "$createdAtBlockHeight": "asc" },
                    { "$ownerId": "asc" }
                ]);
                preorder["indices"][2]["unique"] = json!(true);
            },
        )),
        "which it does not have",
    );
    assert_refused(
        contract(contract_with(
            ordered_by(json!(["$createdAtBlockHeight", "$id"])),
            |preorder| preorder["indices"][2]["unique"] = json!(true),
        )),
        "index \"nameHashByHeight\" of \"preorder\" over (nameHash, $createdAtBlockHeight) is \
         unique, so it keeps no entries by $id",
    );
    // An optional property is not on every commitment
    assert_refused(
        contract(contract_with(ordered_by(json!(["note"])), |preorder| {
            preorder["indices"][2]["properties"] = json!([{ "nameHash": "asc" }, { "note": "asc" }])
        })),
        "chooseFirst \"nameHash\" orders by \"note\" of \"preorder\", which is not recorded on \
         every document",
    );
    // A property a replace can change orders nothing for good
    assert_refused(
        contract(contract_with(ordered_by(json!(["note"])), |preorder| {
            preorder["documentsMutable"] = json!(true);
            preorder["immutable"] = json!(["ownerNameHash", "nameHash"]);
            preorder["required"] = json!([
                "$createdAt",
                "$createdAtBlockHeight",
                "ownerNameHash",
                "nameHash",
                "note"
            ]);
            preorder["indices"][2]["properties"] = json!([{ "nameHash": "asc" }, { "note": "asc" }])
        })),
        "chooseFirst \"nameHash\" orders by \"note\" of \"preorder\", which a replace can change",
    );
}

/// Documents were revealed under the declaration as written, so adding,
/// removing or changing `chooseFirst` is an incompatible schema change.
#[test]
fn should_refuse_an_update_adding_removing_or_changing_choose_first() {
    let platform_version = PlatformVersion::latest();
    let without_choose_first = reveal_with(|reveal| {
        reveal
            .as_object_mut()
            .expect("an object")
            .remove("chooseFirst");
    });
    let reordered = reveal_with(|reveal| {
        reveal["chooseFirst"]["value"] = function(json!([
            "normalizedParentDomainName",
            { "const": "." },
            "normalizedLabel"
        ]));
    });
    for (old, new) in [
        (without_choose_first.clone(), reveal()),
        (reveal(), without_choose_first),
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
fn should_refuse_choose_first_below_protocol_version_14_and_accept_it_at_14() {
    let schema = contract_revealing(reveal());
    let platform_version_13 = PlatformVersion::get(13).expect("platform version 13 should exist");
    contract_on(schema.clone(), true, platform_version_13)
        .expect_err("protocol version 13 should refuse the declaration");
    contract_on(schema, true, PlatformVersion::latest()).expect("protocol version 14 parses it");
}

/// The two functions read the same properties but may join them with other separators, so
/// a create can hold a value the `findBy` function takes and the `chooseFirst` function
/// refuses, which is refused before any read, naming the `chooseFirst` function.
#[test]
fn should_refuse_a_create_holding_a_separator_of_the_choose_first_function() {
    let slashed = reveal_with(|reveal| {
        reveal["chooseFirst"]["value"] = function(json!([
            "normalizedLabel",
            { "const": "/" },
            "normalizedParentDomainName"
        ]));
    });
    let parsed = contract(contract_revealing(slashed)).expect("the reveal should parse");
    let domain = parsed
        .document_type_for_name("domain")
        .expect("the domain document type");
    let data = |label: &str| {
        BTreeMap::from([
            ("label".to_string(), Value::Text(label.to_string())),
            (
                "normalizedLabel".to_string(),
                Value::Text(label.to_string()),
            ),
            (
                "normalizedParentDomainName".to_string(),
                Value::Text("dash".to_string()),
            ),
        ])
    };
    let writer = Identifier::from([3; 32]);

    assert!(matches!(
        first_unrevealable_lookup_key(domain, &data("a/b"), writer),
        Some((path, error))
            if path == "$creatorId"
                && error.param == "normalizedLabel"
                && error.reason.contains("the chooseFirst function")
    ));
    assert_eq!(
        first_unrevealable_lookup_key(domain, &data("al1ce"), writer),
        None
    );
}
