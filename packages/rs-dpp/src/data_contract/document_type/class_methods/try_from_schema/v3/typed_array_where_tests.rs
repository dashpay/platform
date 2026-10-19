//! `where` entries over typed arrays (protocol version 14): an equality
//! between two typed arrays, kept with the other equalities, and the
//! `contains` and `containsAll` conditions. The parse reads their shape; what
//! the names resolve to, and whether the two sides compare as lists, is
//! checked at registration.

use super::reference_test_helpers::{
    assert_refused_by_the_parser, contract, contract_on, identifier, CONTRACT_ID,
};
use super::typed_array_test_helpers::expect_json_schema_error;
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::{
    DocumentPropertyType, DocumentReferenceDeclaration, WhereCondition, WhereContainsBound,
};
use crate::data_contract::DataContract;
use platform_value::string_encoding::Encoding;
use platform_value::Identifier;
use platform_version::version::PlatformVersion;
use serde_json::json;
use std::collections::BTreeMap;

/// A post's tags, as Yappr declares them: up to 4 distinct strings, never
/// changed once written.
fn tags(position: u32) -> serde_json::Value {
    json!({
        "type": "array",
        "items": { "type": "string", "minLength": 1, "maxLength": 63 },
        "maxItems": 4,
        "uniqueItems": true,
        "position": position
    })
}

/// A contract with a `post` holding `tags` and `members`, and a `like` of a
/// post whose reference declares `where`.
fn contract_with_where(where_value: serde_json::Value) -> serde_json::Value {
    let mut post_id = identifier(0);
    post_id["refersTo"] = json!({
        "type": "permanentDocument",
        "documentType": "post",
        "where": where_value
    });
    json!({
        "$formatVersion": "1",
        "id": Identifier::from(CONTRACT_ID).to_string(Encoding::Base58),
        "ownerId": Identifier::from([8; 32]).to_string(Encoding::Base58),
        "version": 1,
        "documentSchemas": {
            "post": {
                "type": "object",
                "canBeDeleted": false,
                "documentsMutable": false,
                "properties": {
                    "tags": tags(0),
                    "members": {
                        "type": "array",
                        "items": {
                            "type": "array",
                            "byteArray": true,
                            "minItems": 32,
                            "maxItems": 32,
                            "contentMediaType": "application/x.dash.dpp.identifier"
                        },
                        "maxItems": 4,
                        "position": 1
                    }
                },
                "additionalProperties": false
            },
            "like": {
                "type": "object",
                "documentsMutable": false,
                "properties": {
                    "postId": post_id,
                    "tags": tags(1),
                    "tag": { "type": "string", "minLength": 1, "maxLength": 63, "position": 2 }
                },
                "required": ["postId"],
                "additionalProperties": false
            }
        }
    })
}

/// The `where` of the like's reference, as registration parses it.
fn parsed_where(
    where_value: serde_json::Value,
) -> (BTreeMap<String, String>, BTreeMap<String, WhereCondition>) {
    where_of(&contract(contract_with_where(where_value)).expect("the contract parses"))
}

/// The `where` of the like's reference in `parsed`.
fn where_of(parsed: &DataContract) -> (BTreeMap<String, String>, BTreeMap<String, WhereCondition>) {
    let like = parsed
        .document_type_for_name("like")
        .expect("the like type");
    let Some(DocumentPropertyType::IdentifierWithReference(target)) = like
        .flattened_properties()
        .get("postId")
        .map(|property| &property.property_type)
    else {
        panic!("postId carries a reference");
    };
    let Some(DocumentReferenceDeclaration {
        property_agreement,
        where_conditions,
        ..
    }) = target.as_document_reference()
    else {
        panic!("postId refers to a post by id");
    };
    (property_agreement.clone(), where_conditions.clone())
}

#[test]
fn should_parse_an_equality_between_typed_arrays_as_an_equality() {
    let (property_agreement, where_conditions) = parsed_where(json!({ "tags": "tags" }));
    assert_eq!(
        property_agreement,
        BTreeMap::from([("tags".to_string(), "tags".to_string())])
    );
    assert!(where_conditions.is_empty());
}

#[test]
fn should_parse_contains_and_contains_all() {
    let (property_agreement, where_conditions) = parsed_where(json!({
        "members": { "contains": "$ownerId" },
        "tags": { "containsAll": "tags" }
    }));
    assert!(property_agreement.is_empty());
    assert_eq!(
        where_conditions,
        BTreeMap::from([
            (
                "members".to_string(),
                WhereCondition::Contains(WhereContainsBound::Writer)
            ),
            (
                "tags".to_string(),
                WhereCondition::ContainsAll("tags".to_string())
            ),
        ])
    );

    let (_, where_conditions) = parsed_where(json!({ "tags": { "contains": "tag" } }));
    assert_eq!(
        where_conditions.get("tags"),
        Some(&WhereCondition::Contains(WhereContainsBound::Property(
            "tag".to_string()
        )))
    );
}

/// A `contains` and a `containsAll` read a list, which the referenced
/// document's system identifiers are not; the writer is one identifier a list
/// may hold, never a list itself; and the bound is a property path, never a
/// literal, `"."` or another system property. The meta-schema refuses most of
/// these too; the parser refuses each on its own.
#[test]
fn should_refuse_a_malformed_contains_or_contains_all() {
    for (where_value, fragment) in [
        (
            json!({ "$ownerId": { "contains": "tag" } }),
            "reads contains of an identifier",
        ),
        (
            json!({ "$id": { "containsAll": "tags" } }),
            "reads containsAll of an identifier",
        ),
        (
            json!({ "tags": { "containsAll": "$ownerId" } }),
            "containsAll reads $ownerId",
        ),
        (
            json!({ "tags": { "contains": { "const": "news" } } }),
            "contains must be a property path of the referring document type, or",
        ),
        (
            json!({ "tags": { "containsAll": ["tags"] } }),
            "containsAll must be a property path of the referring document type",
        ),
        (json!({ "tags": { "contains": "." } }), "reads \".\""),
        (
            json!({ "tags": { "containsAll": "$createdAt" } }),
            "its $ownerId",
        ),
        (
            json!({ "tags": { "containsAny": "tags" } }),
            "contains or containsAll",
        ),
    ] {
        assert_refused_by_the_parser(contract_with_where(where_value), fragment);
    }
}

/// The meta-schema admits both keys with a property path, and `contains` with
/// `"$ownerId"`, before the parser sees them.
#[test]
fn should_admit_the_list_keys_in_the_meta_schema() {
    for where_value in [
        json!({ "tags": { "contains": "tag" } }),
        json!({ "members": { "contains": "$ownerId" } }),
        json!({ "tags": { "containsAll": "tags" } }),
    ] {
        let parsed = contract(contract_with_where(where_value.clone()));
        assert!(parsed.is_ok(), "{where_value}: {parsed:?}");
    }
    let error = expect_json_schema_error(contract(contract_with_where(
        json!({ "tags": { "contains": 5 } }),
    )));
    // The entry matches none of an equality's path, the writer and a condition
    assert_eq!(
        error.instance_path(),
        "/properties/postId/refersTo/where/tags",
        "the meta-schema refuses the entry: {error:?}"
    );
}

/// A stored contract read back parses the same entries, without the
/// meta-schema.
#[test]
fn should_read_back_a_stored_list_where() {
    let stored = contract_on(
        contract_with_where(json!({
            "tags": "tags",
            "members": { "contains": "$ownerId" }
        })),
        false,
        PlatformVersion::latest(),
    )
    .expect("the stored contract reads");
    assert_eq!(stored.id(), Identifier::from(CONTRACT_ID));
    let (property_agreement, where_conditions) = where_of(&stored);
    assert_eq!(
        property_agreement,
        BTreeMap::from([("tags".to_string(), "tags".to_string())])
    );
    assert_eq!(
        where_conditions,
        BTreeMap::from([(
            "members".to_string(),
            WhereCondition::Contains(WhereContainsBound::Writer)
        )])
    );
}
