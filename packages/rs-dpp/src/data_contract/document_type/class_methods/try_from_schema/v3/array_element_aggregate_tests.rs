//! `countOf` and `sumOf` matching their documents by any element of a typed array of the
//! document written, `{ "in": path }` (`parse_property_constraints` 1, protocol version 15):
//! "no tag of this post is banned". Protocol version 14 refuses the form.

use super::immutable_tests::expect_structure_error;
use super::*;
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
use crate::data_contract::document_type::accessors::DocumentTypeV2Getters;
use crate::data_contract::document_type::property_constraints::{
    AggregateBinding, AggregateKind, AggregateRead, ElementKind, EqualityKind, PropertyRead,
};
use crate::data_contract::DataContract;
use platform_value::string_encoding::Encoding;
use serde_json::json;
use std::collections::BTreeMap;

/// A contract of two types parsed under `platform_version`: `bannedTag`, holding a string
/// `tag` and an identifier `bannedBy`, each kept counted by an index of its own, and `post`,
/// holding up to four string `tags`, up to `max_authors` identifier `authors` and a `title`,
/// and declaring `rules` under `keyword` (`propertyConstraints` or `deleteConstraints`).
/// `banned_tag_rules` are `bannedTag`'s own `propertyConstraints`.
fn tagged_posts(
    keyword: &str,
    rules: serde_json::Value,
    banned_tag_rules: Option<serde_json::Value>,
    max_authors: u16,
    platform_version: &PlatformVersion,
) -> Result<DataContract, ProtocolError> {
    let tag = json!({ "type": "string", "pattern": "^[a-z0-9_]{1,61}$", "maxLength": 61 });
    let mut banned_tag = json!({
        "type": "object",
        "documentsMutable": false,
        "properties": {
            "tag": { "type": "string", "pattern": "^[a-z0-9_]{1,61}$", "maxLength": 61, "position": 0 },
            "bannedBy": { "type": "identifier", "position": 1 },
            "aliases": { "type": "array", "items": tag, "maxItems": 2, "position": 2 }
        },
        "required": ["tag", "bannedBy"],
        "indices": [
            { "name": "byTag", "properties": [{ "tag": "asc" }], "countable": "countable" },
            {
                "name": "byBannedBy",
                "properties": [{ "bannedBy": "asc" }],
                "countable": "countable"
            }
        ],
        "additionalProperties": false
    });
    if let Some(banned_tag_rules) = banned_tag_rules {
        banned_tag["propertyConstraints"] = banned_tag_rules;
    }
    let mut post = json!({
        "type": "object",
        "properties": {
            "title": { "type": "string", "maxLength": 61, "position": 0 },
            "tags": { "type": "array", "items": tag, "maxItems": 4, "position": 1 },
            "authors": {
                "type": "array",
                "items": { "type": "identifier" },
                "maxItems": max_authors,
                "position": 2
            }
        },
        "required": ["title"],
        "additionalProperties": false
    });
    post[keyword] = rules;
    let contract = json!({
        "$formatVersion": "1",
        "id": Identifier::from([7; 32]).to_string(Encoding::Base58),
        "ownerId": Identifier::from([8; 32]).to_string(Encoding::Base58),
        "version": 1,
        "documentSchemas": { "bannedTag": banned_tag, "post": post }
    });
    DataContract::from_value(
        platform_value::to_value(contract).expect("the contract converts"),
        true,
        platform_version,
    )
}

/// No tag of the post is banned.
fn no_banned_tags() -> serde_json::Value {
    json!({
        "noBannedTags": {
            "equal": [{ "countOf": ["bannedTag", { "tag": { "in": "tags" } }] }, 0]
        }
    })
}

fn banned_among_tags() -> AggregateRead {
    AggregateRead {
        kind: AggregateKind::Count,
        document_type: "bannedTag".to_string(),
        filter: [(
            "tag".to_string(),
            AggregateBinding::Elements {
                path: "tags".to_string(),
                kind: Some(EqualityKind::Text),
            },
        )]
        .into(),
        of_own_type: false,
    }
}

/// A post holding `tags`, or none when `None`.
fn post_tagged(tags: Option<&[&str]>) -> Value {
    let mut properties = BTreeMap::from([("title".to_string(), Value::Text("hello".to_string()))]);
    if let Some(tags) = tags {
        properties.insert(
            "tags".to_string(),
            Value::Array(
                tags.iter()
                    .map(|tag| Value::Text(tag.to_string()))
                    .collect(),
            ),
        );
    }
    Value::from(properties)
}

/// From protocol version 15 a `countOf` matches its documents by any element of a typed array
/// of the document, read as a `contains` reads the array; protocol version 14 refuses the
/// form.
#[test]
fn should_count_by_the_elements_of_an_array_from_protocol_version_15() {
    let contract = tagged_posts(
        "propertyConstraints",
        no_banned_tags(),
        None,
        2,
        PlatformVersion::latest(),
    )
    .expect("a key matched by the elements of an array registers");
    let post = contract.document_type_for_name("post").expect("post");
    let rule = &post.property_constraints()["noBannedTags"];
    assert_eq!(rule.aggregate_reads(), [&banned_among_tags()]);
    assert_eq!(
        rule.property_reads(),
        [("tags", PropertyRead::Elements(ElementKind::Text))]
    );
    let banned_tag = contract
        .document_type_for_name("bannedTag")
        .expect("bannedTag");
    assert_eq!(
        banned_among_tags()
            .answering_index(&banned_tag)
            .map(|index| index.name.as_str()),
        Some("byTag")
    );

    tagged_posts(
        "propertyConstraints",
        no_banned_tags(),
        None,
        2,
        PlatformVersion::get(14).expect("protocol version 14"),
    )
    .expect_err("protocol version 14 has no key matched by the elements of an array");
}

/// A `deleteConstraints` rule parses with the same grammar.
#[test]
fn should_count_by_the_elements_of_an_array_in_a_delete_rule() {
    let contract = tagged_posts(
        "deleteConstraints",
        no_banned_tags(),
        None,
        2,
        PlatformVersion::latest(),
    )
    .expect("a delete rule may count by the elements of an array");
    let post = contract.document_type_for_name("post").expect("post");
    assert_eq!(
        post.delete_constraints()["noBannedTags"].aggregate_reads(),
        [&banned_among_tags()]
    );
}

/// The read takes one total from state per distinct element of the list, and none for a list
/// the document leaves out or empties; it has no single set of values.
#[test]
fn should_give_one_set_of_values_per_distinct_element() {
    let platform_version = PlatformVersion::latest();
    let contract = tagged_posts(
        "propertyConstraints",
        no_banned_tags(),
        None,
        2,
        platform_version,
    )
    .expect("the contract registers");
    let banned_tag = contract
        .document_type_for_name("bannedTag")
        .expect("bannedTag");
    let read = banned_among_tags();
    let sets = |tags: Option<&[&str]>| {
        read.filter_value_sets(
            banned_tag,
            Identifier::from([5; 32]),
            &post_tagged(tags),
            Identifier::from([3; 32]),
            platform_version,
        )
        .expect("reads")
    };
    let of = |tag: &str| vec![("tag".to_string(), Value::Text(tag.to_string()))];

    assert_eq!(
        sets(Some(&["dash", "news", "dash"])),
        [of("dash"), of("news")]
    );
    assert_eq!(sets(Some(&["dash"])), [of("dash")]);
    assert!(sets(Some(&[])).is_empty());
    assert!(sets(None).is_empty());
    assert_eq!(
        read.filter_values(
            banned_tag,
            Identifier::from([5; 32]),
            &post_tagged(Some(&["dash"])),
            Identifier::from([3; 32]),
            platform_version,
        )
        .expect("reads"),
        None
    );
}

/// A filter binding every key to one value has exactly the one set of values
/// `filter_values` gives, or none when no document can match, under protocol version 14,
/// whose totals consensus now reads through `filter_value_sets`, as under the latest.
#[test]
fn should_give_the_one_set_of_values_of_a_filter_without_an_array() {
    for platform_version in [
        PlatformVersion::get(14).expect("protocol version 14"),
        PlatformVersion::latest(),
    ] {
        let contract = tagged_posts(
            "propertyConstraints",
            json!({
                "titleNotBanned": {
                    "equal": [{ "countOf": ["bannedTag", { "tag": "title" }] }, 0]
                }
            }),
            None,
            2,
            platform_version,
        )
        .expect("the contract registers");
        let banned_tag = contract
            .document_type_for_name("bannedTag")
            .expect("bannedTag");
        let post = contract.document_type_for_name("post").expect("post");
        let read = post.property_constraints()["titleNotBanned"].aggregate_reads()[0].clone();
        for data in [
            post_tagged(None),
            Value::from(BTreeMap::<String, Value>::new()),
        ] {
            let one = read
                .filter_values(
                    banned_tag,
                    Identifier::from([5; 32]),
                    &data,
                    Identifier::from([3; 32]),
                    platform_version,
                )
                .expect("reads");
            let sets = read
                .filter_value_sets(
                    banned_tag,
                    Identifier::from([5; 32]),
                    &data,
                    Identifier::from([3; 32]),
                    platform_version,
                )
                .expect("reads");
            assert_eq!(sets, one.into_iter().collect::<Vec<_>>());
        }
        assert_eq!(
            read.filter_value_sets(
                banned_tag,
                Identifier::from([5; 32]),
                &post_tagged(None),
                Identifier::from([3; 32]),
                platform_version,
            )
            .expect("reads"),
            [vec![("tag".to_string(), Value::Text("hello".to_string()))]]
        );
    }
}

/// What an `in` may name, and where it may stand.
#[test]
fn should_refuse_an_array_element_key_it_cannot_match() {
    let refused = |rules: serde_json::Value, needle: &str| {
        expect_structure_error(
            tagged_posts(
                "propertyConstraints",
                rules,
                None,
                2,
                PlatformVersion::latest(),
            ),
            needle,
        );
    };
    let counting = |filter: serde_json::Value| json!({ "rule": { "equal": [{ "countOf": ["bannedTag", filter] }, 0] } });
    // No array
    refused(
        counting(json!({ "tag": { "in": "title" } })),
        "looks in \"title\", which has type string, not an array with items",
    );
    // No property
    refused(
        counting(json!({ "tag": { "in": "nothing" } })),
        "looks in \"nothing\", which is not an array property of the document type",
    );
    // Elements of another kind than the key
    refused(
        counting(json!({ "tag": { "in": "authors" } })),
        "counts \"bannedTag\" with \"tag\", a string, at an element of \"authors\", an identifier",
    );
    refused(
        counting(json!({ "bannedBy": { "in": "tags" } })),
        "counts \"bannedTag\" with \"bannedBy\", an identifier, at an element of \"tags\", a string",
    );
}

/// A total of the rule's own type matches every key by a single value.
#[test]
fn should_refuse_an_array_element_key_in_a_total_of_the_own_type() {
    expect_structure_error(
        tagged_posts(
            "propertyConstraints",
            json!({ "titled": { "present": "title" } }),
            Some(json!({
                "noAliasBanned": {
                    "equal": [{ "countOf": ["bannedTag", { "tag": { "in": "aliases" } }] }, 0]
                }
            })),
            2,
            PlatformVersion::latest(),
        ),
        "counts \"bannedTag\", its own type, with \"tag\" at any element of \"aliases\"",
    );
}

/// A key matched by the elements of an array reads one total per element, so it counts as the
/// array's `maxItems` toward `max_property_constraint_aggregates`.
#[test]
fn should_count_an_array_element_key_as_its_max_items_toward_the_limit() {
    let platform_version = PlatformVersion::latest();
    let max_aggregates = platform_version
        .system_limits
        .max_property_constraint_aggregates;
    assert_eq!(max_aggregates, 4, "the arithmetic below reads a limit of 4");
    let rules = || {
        json!({
            "noBannedTags": {
                "equal": [{ "countOf": ["bannedTag", { "tag": { "in": "tags" } }] }, 0]
            },
            "noBannedAuthors": {
                "equal": [{ "countOf": ["bannedTag", { "bannedBy": { "in": "authors" } }] }, 0]
            }
        })
    };
    // Four tags alone are the whole limit
    tagged_posts(
        "propertyConstraints",
        no_banned_tags(),
        None,
        2,
        platform_version,
    )
    .expect("four reads are within the limit");
    // And one author more is past it
    expect_structure_error(
        tagged_posts("propertyConstraints", rules(), None, 1, platform_version),
        "reads 5 distinct countOf and sumOf totals, above the maximum of 4",
    );
}

/// A filter takes one `in`.
#[test]
fn should_refuse_two_array_element_keys_in_one_filter() {
    let contract = json!({
        "$formatVersion": "1",
        "id": Identifier::from([7; 32]).to_string(Encoding::Base58),
        "ownerId": Identifier::from([8; 32]).to_string(Encoding::Base58),
        "version": 1,
        "documentSchemas": {
            "pair": {
                "type": "object",
                "documentsMutable": false,
                "properties": {
                    "left": { "type": "string", "maxLength": 61, "position": 0 },
                    "right": { "type": "string", "maxLength": 61, "position": 1 }
                },
                "required": ["left", "right"],
                "indices": [{
                    "name": "byBoth",
                    "properties": [{ "left": "asc" }, { "right": "asc" }],
                    "countable": "countable"
                }],
                "additionalProperties": false
            },
            "post": {
                "type": "object",
                "properties": {
                    "lefts": {
                        "type": "array",
                        "items": { "type": "string", "maxLength": 61 },
                        "maxItems": 2,
                        "position": 0
                    },
                    "rights": {
                        "type": "array",
                        "items": { "type": "string", "maxLength": 61 },
                        "maxItems": 2,
                        "position": 1
                    }
                },
                "propertyConstraints": {
                    "noPair": {
                        "equal": [{
                            "countOf": ["pair", {
                                "left": { "in": "lefts" },
                                "right": { "in": "rights" }
                            }]
                        }, 0]
                    }
                },
                "additionalProperties": false
            }
        }
    });
    expect_structure_error(
        DataContract::from_value(
            platform_value::to_value(contract).expect("the contract converts"),
            true,
            PlatformVersion::latest(),
        ),
        "matches two keys by the elements of an array, but a total takes one \"in\"",
    );
}
