//! A string or byte array property whose `refersTo` `findBy` reads its value as
//! is, `"."` with no function (protocol version 14): the parse, the kind of
//! index property its value must fill, and the rules it shares with an
//! identifier's lookup.

use super::reference_test_helpers::{assert_refused, contract, identifier, CONTRACT_ID};
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::document_type::{
    DocumentReferenceLookup, LookupKeySource, PropertyReference, ReferenceHolder,
};
use crate::data_contract::DataContract;
use platform_value::string_encoding::Encoding;
use platform_value::Identifier;
use serde_json::json;
use std::collections::BTreeMap;

/// A contract with a `handle`, never changed nor deleted, unique by its string
/// `name`, by its 20-byte `tag` and by its identifier `account`, beside a
/// mutable `post` whose `author`, `author_schema`, declares `refers_to`.
fn handle_contract(
    author_schema: serde_json::Value,
    refers_to: serde_json::Value,
) -> serde_json::Value {
    let mut author = author_schema;
    author["refersTo"] = refers_to;
    json!({
        "$formatVersion": "1",
        "id": Identifier::from(CONTRACT_ID).to_string(Encoding::Base58),
        "ownerId": Identifier::from([8; 32]).to_string(Encoding::Base58),
        "version": 1,
        "documentSchemas": {
            "handle": {
                "type": "object",
                "documentsMutable": false,
                "canBeDeleted": false,
                "properties": {
                    "name": { "type": "string", "maxLength": 63, "position": 0 },
                    "tag": {
                        "type": "array", "byteArray": true, "minItems": 20, "maxItems": 20,
                        "position": 1
                    },
                    "account": identifier(2)
                },
                "indices": [
                    { "name": "name", "properties": [{ "name": "asc" }], "unique": true },
                    { "name": "tag", "properties": [{ "tag": "asc" }], "unique": true },
                    { "name": "account", "properties": [{ "account": "asc" }], "unique": true }
                ],
                "required": ["name", "tag", "account"],
                "additionalProperties": false
            },
            "post": {
                "type": "object",
                "documentsMutable": true,
                "canBeDeleted": true,
                "properties": {
                    "author": author,
                    "body": { "type": "string", "maxLength": 100, "position": 1 }
                },
                "required": ["author", "body"],
                "additionalProperties": false
            }
        }
    })
}

fn string_author() -> serde_json::Value {
    json!({ "type": "string", "maxLength": 63, "position": 0 })
}

fn byte_array_author() -> serde_json::Value {
    json!({
        "type": "array", "byteArray": true, "minItems": 20, "maxItems": 20, "position": 0
    })
}

/// A `kind` reference to a `handle` found by `index_property`, which the
/// value fills as is.
fn found_by(kind: &str, index_property: &str) -> serde_json::Value {
    let mut find_by = serde_json::Map::new();
    find_by.insert(index_property.to_string(), json!("."));
    json!({ "type": kind, "documentType": "handle", "findBy": find_by })
}

/// The lookup the `post`'s `author` declares, which must be its one reference,
/// carried by the property itself.
fn author_lookup(contract: &DataContract) -> DocumentReferenceLookup {
    let post = contract
        .document_type_for_name("post")
        .expect("the post document type");
    let declarations = post.reference_declarations().collect::<Vec<_>>();
    let [(ReferenceHolder::Property("author"), PropertyReference::Revealed(target))] =
        declarations.as_slice()
    else {
        panic!("expected the author's reference alone, read from its value");
    };
    target
        .as_any_document_reference()
        .and_then(|declaration| declaration.lookup)
        .expect("the author's lookup")
        .clone()
}

fn value_as_is(index_property: &str) -> DocumentReferenceLookup {
    DocumentReferenceLookup::new(BTreeMap::from([(
        index_property.to_string(),
        LookupKeySource::ReferenceValue,
    )]))
}

#[test]
fn should_parse_a_string_or_byte_array_property_found_by_its_value_as_is() {
    for (author_schema, index_property) in [(string_author(), "name"), (byte_array_author(), "tag")]
    {
        let parsed = contract(handle_contract(
            author_schema,
            found_by("permanentDocument", index_property),
        ))
        .expect("a value read as is into an index property of its kind should parse");
        let lookup = author_lookup(&parsed);
        assert_eq!(lookup, value_as_is(index_property));
        // No function: judged again on a replace, as an identifier's lookup is
        assert!(!lookup.is_checked_on_create_only(true));
    }
}

#[test]
fn should_refuse_a_value_read_as_is_into_an_index_property_of_another_kind() {
    for (author_schema, index_property) in [
        (string_author(), "account"),
        (string_author(), "tag"),
        (byte_array_author(), "name"),
        (byte_array_author(), "account"),
    ] {
        assert_refused(
            contract(handle_contract(
                author_schema,
                found_by("permanentDocument", index_property),
            )),
            "which holds a different kind of value",
        );
    }
}

/// Every replace asks again for a `deletableDocument` found by a value read as
/// is, as for one by id, so an immutable property can not hold one
#[test]
fn should_refuse_a_deletable_document_found_by_a_string_value_on_an_immutable_property() {
    let mut schema = handle_contract(string_author(), found_by("deletableDocument", "name"));
    schema["documentSchemas"]["handle"]["canBeDeleted"] = json!(true);
    contract(schema.clone()).expect("a deletable handle found by name on a mutable property");
    schema["documentSchemas"]["post"]["immutable"] = json!(["author"]);
    assert_refused(
        contract(schema),
        "\"author\" is a deletableDocument reference found by findBy",
    );
}

/// The value fills its key once: as `"."`, its kind, or by its path as a param
/// of a function, never both
#[test]
fn should_refuse_a_value_read_as_is_and_by_a_function() {
    let mut refers_to = found_by("permanentDocument", "name");
    refers_to["findBy"]["account"] = json!({
        "function": "sys.hash.sha256d",
        "params": ["author"]
    });
    assert_refused(
        contract(handle_contract(string_author(), refers_to)),
        "findBy reads the value of \"author\" 2 times",
    );
}
