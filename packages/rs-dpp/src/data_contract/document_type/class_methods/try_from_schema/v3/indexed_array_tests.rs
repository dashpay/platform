//! Indexes on typed arrays (protocol version 14): an index naming a typed
//! array keys a document once by each of its distinct elements. These tests
//! cover what registration admits and refuses, and what the parse resolves on
//! every parse: the array in the index's skip set and the element stamp on its
//! index level.

use super::typed_array_test_helpers::{expect_structure_error, parse_dispatched};
use super::*;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use platform_value::platform_value;

fn hashtags(max_items: u16) -> Value {
    platform_value!({
        "type": "array",
        "maxItems": max_items,
        "items": { "type": "string", "minLength": 1, "maxLength": 61 },
        "position": 0
    })
}

/// A `post` type holding `hashtags`, an optional `language` and an `author`
/// identifier, with `indices`.
fn post_schema(hashtags: Value, indices: Value) -> Value {
    platform_value!({
        "type": "object",
        "documentsMutable": true,
        "properties": {
            "hashtags": hashtags,
            "language": { "type": "string", "maxLength": 2, "position": 1 },
            "author": {
                "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32,
                "contentMediaType": "application/x.dash.dpp.identifier",
                "position": 2
            }
        },
        "required": ["$createdAt"],
        "indices": indices,
        "additionalProperties": false
    })
}

fn parse(schema: Value) -> Result<DocumentType, ProtocolError> {
    parse_dispatched(schema, PlatformVersion::latest(), true)
}

/// The constraint an `InvalidIndexedPropertyConstraintError` names, as
/// `"<property>.<constraint>"`.
fn expect_constraint_error(result: Result<DocumentType, ProtocolError>) -> String {
    match result {
        Err(ProtocolError::ConsensusError(boxed)) => match *boxed {
            ConsensusError::BasicError(BasicError::InvalidIndexedPropertyConstraintError(
                error,
            )) => format!("{}.{}", error.property_name(), error.constraint_name()),
            other => panic!("expected an indexed property constraint error, got {other:?}"),
        },
        other => panic!("expected an indexed property constraint error, got {other:?}"),
    }
}

#[test]
fn should_index_a_typed_array_at_any_position_and_skip_documents_without_elements() {
    let document_type = parse(post_schema(
        hashtags(10),
        platform_value!([
            { "name": "byHashtagTime", "properties": [{ "hashtags": "asc" }, { "$createdAt": "asc" }] },
            { "name": "byAuthorHashtag", "properties": [{ "author": "asc" }, { "hashtags": "asc" }] },
            {
                "name": "byHashtagLanguage",
                "properties": [{ "hashtags": "asc" }, { "language": "asc" }],
                "skipIfAbsent": ["language"]
            }
        ]),
    ))
    .expect("an index may name a typed array at any position");

    let indexes = document_type.indexes();
    let skip_set = |name: &str| {
        let index = indexes.get(name).expect("the index is parsed");
        (
            index.skip_if_absent,
            index.skip_if_absent_properties.clone(),
        )
    };
    // The array is in every skip set, in index-property order with the
    // declared skip properties, without declaring the keyword.
    assert_eq!(
        skip_set("byHashtagTime"),
        (false, vec!["hashtags".to_string()])
    );
    assert_eq!(
        skip_set("byAuthorHashtag"),
        (false, vec!["hashtags".to_string()])
    );
    assert_eq!(
        skip_set("byHashtagLanguage"),
        (true, vec!["hashtags".to_string(), "language".to_string()])
    );
    // Each index names which member of its skip set the array is.
    for name in ["byHashtagTime", "byAuthorHashtag", "byHashtagLanguage"] {
        assert_eq!(
            indexes
                .get(name)
                .expect("the index")
                .indexed_array
                .as_deref(),
            Some("hashtags"),
            "{name}"
        );
    }

    // The levels named after the array carry its maxItems, wherever an index
    // reaches them; no other level does.
    let structure = document_type.index_structure();
    let top = structure
        .sub_levels()
        .get("hashtags")
        .expect("a first level");
    assert_eq!(top.indexed_array_max_items(), Some(10));
    assert!(!top
        .sub_levels()
        .get("$createdAt")
        .expect("a second level")
        .indexes_array_elements());
    let below_author = structure
        .sub_levels()
        .get("author")
        .and_then(|level| level.sub_levels().get("hashtags"))
        .expect("the array below the author");
    assert_eq!(below_author.indexed_array_max_items(), Some(10));
    assert!(!structure
        .sub_levels()
        .get("author")
        .expect("the author level")
        .indexes_array_elements());
}

#[test]
fn should_resolve_the_same_indexes_without_full_validation() {
    let schema = post_schema(
        hashtags(10),
        platform_value!([
            { "name": "byHashtagTime", "properties": [{ "hashtags": "asc" }, { "$createdAt": "asc" }] }
        ]),
    );
    let validated = parse_dispatched(schema.clone(), PlatformVersion::latest(), true)
        .expect("parses with full validation");
    let loaded = parse_dispatched(schema, PlatformVersion::latest(), false)
        .expect("parses as a stored contract");
    assert_eq!(validated.indexes(), loaded.indexes());
    assert_eq!(validated.index_structure(), loaded.index_structure());
}

#[test]
fn should_hold_an_indexed_array_to_16_elements_and_4_under_a_window() {
    let plain = |max_items: u16| {
        parse(post_schema(
            hashtags(max_items),
            platform_value!([{ "name": "byHashtag", "properties": [{ "hashtags": "asc" }] }]),
        ))
    };
    plain(16).expect("16 elements may be indexed");
    assert_eq!(expect_constraint_error(plain(17)), "hashtags.maxItems");

    let windowed = |max_items: u16| {
        parse(post_schema(
            hashtags(max_items),
            platform_value!([{
                "name": "trending",
                "properties": [{ "$createdAt": "asc" }, { "hashtags": "asc" }],
                "timeRange": { "on": "$createdAt", "range": 86400, "step": 86400 },
                "rangeCountable": true,
                "rankedCountable": true
            }]),
        ))
    };
    windowed(4).expect("4 elements may be indexed under a window");
    assert_eq!(expect_constraint_error(windowed(5)), "hashtags.maxItems");
}

/// At 16 elements, and 10 indexes to a type, plain indexes of arrays stay
/// under the budget (160 entries); windows reach it. Two indexes of a
/// 4-element array under 24 overlapping hour windows of a day write 96 entries
/// each, and `count` indexes of a 16-element array, each after a property of
/// its own, 16 each.
#[test]
fn should_hold_a_document_to_256_entries_across_the_indexes_of_its_type() {
    let schema = |count: usize| {
        let mut properties = vec![
            (Value::Text("hashtags".to_string()), hashtags(16)),
            (
                Value::Text("topics".to_string()),
                platform_value!({
                    "type": "array",
                    "maxItems": 4,
                    "items": { "type": "string", "minLength": 1, "maxLength": 61 },
                    "position": 1
                }),
            ),
            (
                Value::Text("region".to_string()),
                platform_value!({ "type": "string", "maxLength": 8, "position": 2 }),
            ),
        ];
        let hourly = || platform_value!({ "on": "$createdAt", "range": 86400, "step": 3600 });
        let mut indices = vec![
            platform_value!({
                "name": "trending",
                "properties": [{ "$createdAt": "asc" }, { "topics": "asc" }],
                "timeRange": hourly()
            }),
            platform_value!({
                "name": "trendingByRegion",
                "properties": [{ "$createdAt": "asc" }, { "topics": "asc" }, { "region": "asc" }],
                "timeRange": hourly()
            }),
        ];
        for n in 0..count {
            let name = format!("p{n}");
            properties.push((
                Value::Text(name.clone()),
                platform_value!({ "type": "integer", "minimum": 0, "maximum": 255, "position": n + 3 }),
            ));
            indices.push(platform_value!({
                "name": format!("byHashtag{n}"),
                "properties": [
                    Value::Map(vec![(Value::Text(name), Value::Text("asc".to_string()))]),
                    { "hashtags": "asc" }
                ]
            }));
        }
        platform_value!({
            "type": "object",
            "properties": Value::Map(properties),
            "required": ["$createdAt"],
            "indices": Value::Array(indices),
            "additionalProperties": false
        })
    };
    parse(schema(4)).expect("2 windowed indexes and 4 plain ones write 256 entries");
    expect_structure_error(parse(schema(5)), "can write 272 index entries");
}

#[test]
fn should_hold_the_elements_to_the_key_limits_of_their_type() {
    let unbounded = platform_value!({
        "type": "array",
        "maxItems": 10,
        "items": { "type": "string", "minLength": 1 },
        "position": 0
    });
    assert_eq!(
        expect_constraint_error(parse(post_schema(
            unbounded,
            platform_value!([{ "name": "byHashtag", "properties": [{ "hashtags": "asc" }] }]),
        ))),
        "hashtags.items.maxLength"
    );

    // An empty byte array is keyed like a missing value.
    let maybe_empty = platform_value!({
        "type": "array",
        "maxItems": 10,
        "items": { "type": "array", "byteArray": true, "maxItems": 20 },
        "position": 0
    });
    assert_eq!(
        expect_constraint_error(parse(post_schema(
            maybe_empty,
            platform_value!([{ "name": "byKey", "properties": [{ "hashtags": "asc" }] }]),
        ))),
        "hashtags.items.minItems"
    );

    // A ranking leaves 61 characters for its group key.
    let long = platform_value!({
        "type": "array",
        "maxItems": 10,
        "items": { "type": "string", "maxLength": 63 },
        "position": 0
    });
    assert_eq!(
        expect_constraint_error(parse(post_schema(
            long,
            platform_value!([{
                "name": "byHashtag",
                "properties": [{ "hashtags": "asc" }],
                "rangeCountable": true,
                "rankedCountable": true
            }]),
        ))),
        "hashtags.items.maxLength"
    );
}

#[test]
fn should_refuse_what_an_element_index_can_not_be() {
    let refused = |indices: Value, needle: &str| {
        expect_structure_error(parse(post_schema(hashtags(10), indices)), needle)
    };
    refused(
        platform_value!([{ "name": "byHashtag", "properties": [{ "hashtags": "asc" }], "unique": true }]),
        "a unique index can not key by the elements",
    );
    refused(
        platform_value!([{
            "name": "byHashtag",
            "properties": [{ "hashtags": "asc" }],
            "nullSearchable": false
        }]),
        "nullSearchable must stay true",
    );
    refused(
        platform_value!([{
            "name": "byAuthorHashtag",
            "properties": [{ "author": "asc" }, { "hashtags": "asc" }],
            "countable": "countable",
            "rangeCountable": true,
            "rankedCountable": { "at": ["author"] }
        }]),
        "above the array, would count a document once per element",
    );
}

#[test]
fn should_refuse_two_arrays_in_one_index() {
    let schema = platform_value!({
        "type": "object",
        "properties": {
            "hashtags": hashtags(10),
            "scores": {
                "type": "array",
                "maxItems": 4,
                "items": { "type": "integer", "minimum": 0, "maximum": 100 },
                "position": 1
            }
        },
        "indices": [{
            "name": "byBoth",
            "properties": [{ "hashtags": "asc" }, { "scores": "asc" }]
        }],
        "additionalProperties": false
    });
    expect_structure_error(parse(schema), "also names \"scores\"");
}

#[test]
fn should_not_judge_a_required_array_by_the_declared_skip_rules() {
    let mut schema = post_schema(
        hashtags(10),
        platform_value!([{
            "name": "byHashtagLanguage",
            "properties": [{ "hashtags": "asc" }, { "language": "asc" }],
            "skipIfAbsent": ["language"]
        }]),
    );
    schema
        .set_value("required", platform_value!(["$createdAt", "hashtags"]))
        .expect("required");
    parse(schema).expect("a required array still skips a document with an empty list");
}

/// `skipIfAbsent` whose only skip property would be the array adds nothing:
/// the index skips a document without elements anyway. It is refused with a
/// reason naming the array, not as a keyword on required properties.
#[test]
fn should_refuse_a_declared_skip_whose_only_skip_property_is_the_array() {
    for skip_if_absent in [platform_value!(true), platform_value!(["hashtags"])] {
        let schema = post_schema(
            hashtags(10),
            platform_value!([{
                "name": "byHashtag",
                "properties": [{ "hashtags": "asc" }, { "$createdAt": "asc" }],
                "skipIfAbsent": skip_if_absent
            }]),
        );
        expect_structure_error(
            parse(schema),
            "its only skip property is the typed array \"hashtags\"",
        );
    }
    // Beside another optional property the keyword skips on that one.
    let schema = post_schema(
        hashtags(10),
        platform_value!([{
            "name": "byHashtagLanguage",
            "properties": [{ "hashtags": "asc" }, { "language": "asc" }],
            "skipIfAbsent": true
        }]),
    );
    let document_type = parse(schema).expect("language is an optional skip property");
    let index = document_type
        .indexes()
        .get("byHashtagLanguage")
        .expect("the index");
    assert_eq!(index.skip_if_absent_properties, ["hashtags", "language"]);
}

/// `tags` of an indexOnly `like`: at most 4 strings, `uniqueItems` as given.
fn like_tags(unique_items: bool) -> Value {
    platform_value!({
        "type": "array",
        "items": { "type": "string", "maxLength": 61 },
        "maxItems": 4,
        "uniqueItems": unique_items,
        "position": 1
    })
}

fn identifier(position: u32) -> Value {
    platform_value!({
        "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32,
        "contentMediaType": "application/x.dash.dpp.identifier",
        "position": position
    })
}

/// An indexOnly `like` of a post carrying the post's `tags` and author, with
/// `indices` beside `byAuthorPostTime`, which holds `postAuthor`.
fn like_schema(tags: Value, indices: Value) -> Value {
    let mut indices = match indices {
        Value::Array(indices) => indices,
        other => panic!("indices are a list, got {other:?}"),
    };
    indices.push(platform_value!({
        "name": "byAuthorPostTime",
        "properties": [{ "postAuthor": "asc" }, { "postId": "asc" }, { "$createdAt": "asc" }]
    }));
    platform_value!({
        "type": "object",
        "indexOnly": true,
        "documentsMutable": false,
        "properties": {
            "postId": identifier(0),
            "tags": tags,
            "postAuthor": identifier(2)
        },
        "required": ["$createdAt", "postId", "postAuthor"],
        "indices": Value::Array(indices),
        "additionalProperties": false
    })
}

/// The `byPost` index of a like: one like per (post, owner).
fn by_post() -> Value {
    platform_value!({
        "name": "byPost", "properties": [{ "postId": "asc" }], "terminal": "$ownerId",
        "rangeCountable": true
    })
}

/// The per-tag index of a like.
fn by_tag_post() -> Value {
    platform_value!({
        "name": "byTagPost", "properties": [{ "tags": "asc" }, { "postId": "asc" }],
        "rangeCountable": true
    })
}

/// The trending per-tag index of a like: a day's window every 6 hours, kept a
/// week past its end, outliving a delete.
fn by_trend_tag_post() -> Value {
    platform_value!({
        "name": "byTrendTagPost",
        "properties": [{ "$createdAt": "asc" }, { "tags": "asc" }, { "postId": "asc" }],
        "terminal": "$ownerId",
        "rangeCountable": true,
        "timeRange": { "on": "$createdAt", "range": 86400, "step": 21600, "ttl": 604800 },
        "outlivesDelete": true
    })
}

/// An indexOnly type may index the elements of a typed array when another
/// index keeps each document once under part of the array index's key: there
/// `byPost`, whose key (`postId`, `$ownerId`) is inside each tag index's key
/// without the array. The array is in each tag index's skip set, so a like
/// without tags is in neither, and no tag index is ever the proof index.
#[test]
fn should_admit_an_index_only_array_index_beside_an_index_keeping_each_document_once() {
    let schema = like_schema(
        like_tags(true),
        platform_value!([by_post(), by_tag_post(), by_trend_tag_post()]),
    );
    for full_validation in [true, false] {
        let document_type =
            parse_dispatched(schema.clone(), PlatformVersion::latest(), full_validation)
                .expect("the like parses");
        let indexes = document_type.indexes();
        for name in ["byTagPost", "byTrendTagPost"] {
            let index = indexes.get(name).expect("the tag index");
            assert_eq!(index.indexed_array.as_deref(), Some("tags"), "{name}");
            assert_eq!(index.skip_if_absent_properties, ["tags"], "{name}");
            assert!(!index.keys_each_live_document_by_its_values(), "{name}");
            // The terminal defaults to the owner, as on every indexOnly index
            assert_eq!(index.terminal_components(), ["$ownerId"], "{name}");
        }
        assert!(indexes
            .get("byPost")
            .expect("byPost")
            .keys_each_live_document_by_its_values());
        assert_eq!(
            document_type
                .index_structure()
                .sub_levels()
                .get("tags")
                .expect("the tag level")
                .indexed_array_max_items(),
            Some(4)
        );
    }
}

/// Without an index keeping each document once under part of the array
/// index's key, two likes sharing a tag would collide on its entry: refused on
/// every parse. An index that skips documents, outlives deletes or involves
/// `$createdAt` keeps no document apart, and an index outside the array
/// index's key does not either.
#[test]
fn should_refuse_an_index_only_array_index_no_other_index_keeps_apart() {
    let outside_the_key = like_schema(
        like_tags(true),
        platform_value!([
            {
                "name": "byAuthor", "properties": [{ "postAuthor": "asc" }],
                "terminal": "$ownerId"
            },
            by_tag_post()
        ]),
    );
    let with_created_at = like_schema(
        like_tags(true),
        platform_value!([
            {
                "name": "byAuthorPost", "properties": [{ "postAuthor": "asc" }, { "postId": "asc" }],
                "terminal": "$ownerId"
            },
            {
                "name": "byTimePost", "properties": [{ "$createdAt": "asc" }, { "postId": "asc" }],
                "terminal": "$ownerId"
            },
            {
                "name": "byTimeTagPost",
                "properties": [{ "$createdAt": "asc" }, { "tags": "asc" }, { "postId": "asc" }]
            }
        ]),
    );
    for schema in [outside_the_key, with_created_at] {
        for full_validation in [true, false] {
            expect_structure_error(
                parse_dispatched(schema.clone(), PlatformVersion::latest(), full_validation),
                "holds the whole key of no index that keeps one entry for every document",
            );
        }
    }
}

/// An index of a typed array has no entry for a like without tags, so it can not
/// be the only index keeping a required property: `postAuthor`, held only by
/// `byTagPostAuthor`, would be committed and stored nowhere for an untagged
/// like. It still keeps the array itself.
#[test]
fn should_refuse_a_required_property_only_an_index_only_array_index_holds() {
    let schema = platform_value!({
        "type": "object",
        "indexOnly": true,
        "documentsMutable": false,
        "properties": {
            "postId": identifier(0),
            "tags": like_tags(true),
            "postAuthor": identifier(2)
        },
        "required": ["postId", "postAuthor"],
        "indices": [
            by_post(),
            {
                "name": "byTagPostAuthor",
                "properties": [{ "tags": "asc" }, { "postId": "asc" }, { "postAuthor": "asc" }]
            }
        ],
        "additionalProperties": false
    });
    for full_validation in [true, false] {
        expect_structure_error(
            parse_dispatched(schema.clone(), PlatformVersion::latest(), full_validation),
            "property \"postAuthor\" on indexOnly document type \"charter\" does not appear \
             in any index that neither sets skipIfAbsent nor outlivesDelete, nor names a typed \
             array other than the property",
        );
    }

    // A required array is kept by its own index
    let mut required_tags = schema;
    required_tags
        .set_value(
            "required",
            platform_value!(["postId", "postAuthor", "tags"]),
        )
        .expect("keyword applies");
    required_tags
        .set_value(
            "indices",
            platform_value!([
                by_post(),
                by_tag_post(),
                {
                    "name": "byAuthorPost",
                    "properties": [{ "postAuthor": "asc" }, { "postId": "asc" }]
                }
            ]),
        )
        .expect("keyword applies");
    for full_validation in [true, false] {
        parse_dispatched(
            required_tags.clone(),
            PlatformVersion::latest(),
            full_validation,
        )
        .expect("a required array its own index holds parses");
    }
}

/// Each element of a like's tags has an entry of its own: an array that may
/// repeat an element is refused.
#[test]
fn should_refuse_an_index_only_array_index_without_unique_items() {
    expect_structure_error(
        parse(like_schema(
            like_tags(false),
            platform_value!([by_post(), by_tag_post()]),
        )),
        "must declare `uniqueItems: true`",
    );
}

/// An indexOnly type's array index names one top-level array on every parse,
/// not only under full validation: the entry probes and the row commitment
/// key the one array the index stamps, by its top-level name.
#[test]
fn should_refuse_a_second_or_nested_array_on_an_index_only_type_on_every_parse() {
    let two_arrays = platform_value!({
        "type": "object",
        "indexOnly": true,
        "documentsMutable": false,
        "properties": {
            "postId": identifier(0),
            "tags": like_tags(true),
            "postAuthor": identifier(2),
            "topics": {
                "type": "array",
                "items": { "type": "string", "maxLength": 61 },
                "maxItems": 4,
                "uniqueItems": true,
                "position": 3
            }
        },
        "required": ["$createdAt", "postId", "postAuthor"],
        "indices": [
            by_post(),
            {
                "name": "byTagTopicPost",
                "properties": [{ "tags": "asc" }, { "topics": "asc" }, { "postId": "asc" }]
            },
            {
                "name": "byAuthorPostTime",
                "properties": [{ "postAuthor": "asc" }, { "postId": "asc" }, { "$createdAt": "asc" }]
            }
        ],
        "additionalProperties": false
    });
    let nested = platform_value!({
        "type": "object",
        "indexOnly": true,
        "documentsMutable": false,
        "properties": {
            "postId": identifier(0),
            "meta": {
                "type": "object",
                "properties": { "tags": like_tags(true) },
                "additionalProperties": false,
                "position": 1
            },
            "postAuthor": identifier(2)
        },
        "required": ["$createdAt", "postId", "postAuthor", "meta"],
        "indices": [
            by_post(),
            {
                "name": "byTagPost",
                "properties": [{ "meta.tags": "asc" }, { "postId": "asc" }]
            },
            {
                "name": "byAuthorPostTime",
                "properties": [{ "postAuthor": "asc" }, { "postId": "asc" }, { "$createdAt": "asc" }]
            }
        ],
        "additionalProperties": false
    });
    for full_validation in [true, false] {
        expect_structure_error(
            parse_dispatched(
                two_arrays.clone(),
                PlatformVersion::latest(),
                full_validation,
            ),
            "an index keys by the elements of one typed array",
        );
        expect_structure_error(
            parse_dispatched(nested.clone(), PlatformVersion::latest(), full_validation),
            if full_validation {
                "an indexed typed array must be a top-level property"
            } else {
                "which is inside an object: an indexed typed array is a top-level property"
            },
        );
    }
    // The second array is the indexOnly rule's own finding without full validation
    expect_structure_error(
        parse_dispatched(two_arrays, PlatformVersion::latest(), false),
        "and also \"topics\"",
    );
}

/// `preallocated` and `summableOffCountIndex` stay refused on an index of a
/// typed array no `where` equality binds to a referenced typed array (here the
/// like refers to nothing): a preallocated path is fixed by one referenced
/// document, and a counter's group is one element of one list. The bound shape
/// is admitted (`typed_array_counter_index_tests`).
#[test]
fn should_refuse_a_preallocated_or_counting_index_only_array_index() {
    let mut preallocated = by_tag_post();
    preallocated
        .set_value("preallocated", Value::Bool(true))
        .expect("keyword applies");
    expect_structure_error(
        parse(like_schema(
            like_tags(true),
            platform_value!([by_post(), preallocated]),
        )),
        "declares `preallocated`, but its path is not determined by a reference",
    );

    let counter = platform_value!({
        "name": "tagLikes",
        "properties": [{ "tags": "asc" }, { "postId": "asc" }],
        "summableOffCountIndex": "byPost",
        "rangeCountable": true,
        "rangeSummable": true
    });
    expect_structure_error(
        parse(like_schema(
            like_tags(true),
            platform_value!([by_post(), counter]),
        )),
        "has \"tags\", which is neither a property of its source \"byPost\" nor fixed by it",
    );
}

/// A contract update may raise an indexed array's `maxItems` within the index
/// limits: the index itself does not change. The updated type is parsed with
/// full validation, so a raise past the limits is refused there.
#[test]
fn should_let_an_update_raise_an_indexed_array_within_the_limits() {
    let indices =
        || platform_value!([{ "name": "byHashtag", "properties": [{ "hashtags": "asc" }] }]);
    let old = parse(post_schema(hashtags(10), indices())).expect("the registered type");
    let raised = parse(post_schema(hashtags(16), indices())).expect("16 elements may be indexed");
    let result = old
        .as_ref()
        .validate_update(raised.as_ref(), 2, PlatformVersion::latest())
        .expect("validate_update runs");
    assert!(result.is_valid(), "{:?}", result.errors);

    assert_eq!(
        expect_constraint_error(parse(post_schema(hashtags(17), indices()))),
        "hashtags.maxItems"
    );
}

/// A type with a time to live prepaid each document's cleanup by its indexed
/// arrays' `maxItems`, so an update may not raise one there; a bound that
/// stays, or a type without a time to live, is unaffected.
#[test]
fn should_hold_an_indexed_array_bound_on_a_type_with_a_time_to_live() {
    let expiring = |max_items: u16| {
        let mut schema = post_schema(
            hashtags(max_items),
            platform_value!([{ "name": "byHashtag", "properties": [{ "hashtags": "asc" }] }]),
        );
        schema
            .insert("ttl".to_string(), 86_400u32.into())
            .expect("a time to live");
        parse(schema).expect("an expiring post type")
    };
    let old = expiring(4);
    let update = |new: &DocumentType| {
        old.as_ref()
            .validate_update(new.as_ref(), 2, PlatformVersion::latest())
            .expect("validate_update runs")
    };

    let raised = update(&expiring(8));
    assert!(!raised.is_valid(), "a raised bound is refused");
    assert!(
        format!("{:?}", raised.errors)
            .contains("can not raise the maxItems of indexed typed array 'hashtags' from 4 to 8"),
        "{:?}",
        raised.errors
    );
    let kept = update(&expiring(4));
    assert!(kept.is_valid(), "{:?}", kept.errors);
}

/// The key codec by protocol version, through its dispatchers: generation 1
/// (protocol version 14) keys a scalar exactly as generation 0 does, so no
/// stored key moves, and keys a typed array property as one element, which
/// generation 0 refuses. A document names its element keys only from
/// protocol version 14.
#[test]
fn should_key_scalars_alike_and_elements_only_from_protocol_version_14() {
    use crate::data_contract::document_type::methods::DocumentTypeV0Methods;
    use crate::document::document_methods::DocumentMethodsV0;
    use crate::document::{Document, DocumentV0};
    use platform_value::Identifier;

    let document_type = parse(post_schema(
        hashtags(10),
        platform_value!([{ "name": "byHashtag", "properties": [{ "hashtags": "asc" }] }]),
    ))
    .expect("the post type");
    let document_type = document_type.as_ref();
    let protocol_version_13 = PlatformVersion::get(13).expect("protocol version 13 exists");
    let latest = PlatformVersion::latest();

    let author = [5u8; 32];
    for (key, value, frozen) in [
        ("language", platform_value!("en"), b"en".to_vec()),
        ("author", Value::Identifier(author), author.to_vec()),
    ] {
        for platform_version in [protocol_version_13, latest] {
            let encoded = document_type
                .serialize_value_for_key(key, &value, platform_version)
                .expect("a scalar keys at every version");
            assert_eq!(
                encoded, frozen,
                "{key} at {}",
                platform_version.protocol_version
            );
            assert_eq!(
                document_type
                    .deserialize_value_for_key(key, &encoded, platform_version)
                    .expect("the key decodes"),
                document_type
                    .deserialize_value_for_key(key, &encoded, latest)
                    .expect("the key decodes"),
                "{key} decodes alike at {}",
                platform_version.protocol_version
            );
        }
    }

    let element = platform_value!("dash");
    document_type
        .serialize_value_for_key("hashtags", &element, protocol_version_13)
        .expect_err("generation 0 keys no typed array");
    assert_eq!(
        document_type
            .serialize_value_for_key("hashtags", &element, latest)
            .expect("generation 1 keys an element"),
        b"dash".to_vec()
    );

    let document = Document::V0(DocumentV0 {
        id: Identifier::from([1; 32]),
        owner_id: Identifier::from([2; 32]),
        properties: [(
            "hashtags".to_string(),
            platform_value!(["news", "dash", "news"]),
        )]
        .into_iter()
        .collect(),
        ..Default::default()
    });
    document
        .get_raw_array_elements_for_document_type("hashtags", document_type, protocol_version_13)
        .expect_err("no document names its elements before protocol version 14");
    assert_eq!(
        document
            .get_raw_array_elements_for_document_type("hashtags", document_type, latest)
            .expect("the elements, sorted and distinct"),
        vec![b"dash".to_vec(), b"news".to_vec()]
    );
}

#[test]
fn should_refuse_an_index_on_a_typed_array_inside_an_object() {
    let schema = platform_value!({
        "type": "object",
        "properties": {
            "team": {
                "type": "object",
                "position": 0,
                "properties": {
                    "leads": {
                        "type": "array",
                        "maxItems": 4,
                        "items": { "type": "string", "minLength": 1, "maxLength": 20 },
                        "position": 0
                    }
                },
                "additionalProperties": false
            }
        },
        "indices": [{ "name": "byLead", "properties": [{ "team.leads": "asc" }] }],
        "additionalProperties": false
    });
    expect_structure_error(
        parse(schema),
        "must be a top-level property of the document type",
    );
}

#[test]
fn should_refuse_a_contested_index_on_a_typed_array() {
    let schema = platform_value!({
        "type": "object",
        "documentsMutable": false,
        "properties": { "hashtags": hashtags(10) },
        "indices": [{
            "name": "byHashtag",
            "properties": [{ "hashtags": "asc" }],
            "unique": true,
            "contested": {
                "fieldMatches": [{ "field": "hashtags", "regexPattern": "^[a-z]{3,}$" }],
                "resolution": 0
            }
        }],
        "additionalProperties": false
    });
    expect_structure_error(parse(schema), "a contested index is over one value");
}

#[test]
fn should_key_an_unsigned_element_in_value_order() {
    use crate::data_contract::document_type::methods::DocumentTypeV0Methods;
    use crate::document::document_methods::DocumentMethodsV0;
    use crate::document::{Document, DocumentV0};
    use platform_value::Identifier;

    let schema = platform_value!({
        "type": "object",
        "properties": {
            "scores": {
                "type": "array",
                "maxItems": 4,
                "items": { "type": "integer", "minimum": 0, "maximum": 1000 },
                "position": 0
            }
        },
        "indices": [{ "name": "byScore", "properties": [{ "scores": "asc" }] }],
        "additionalProperties": false
    });
    let document_type = parse(schema).expect("the scored type");
    let document_type = document_type.as_ref();
    let latest = PlatformVersion::latest();

    // An element is keyed as a property of its type is: from protocol version 14 an
    // unsigned integer by its plain big-endian bytes, so the keys sort as the values do.
    let key = document_type
        .serialize_value_for_key("scores", &Value::U16(300), latest)
        .expect("an element keys");
    assert_eq!(key, 300u16.to_be_bytes().to_vec());
    assert_eq!(
        document_type
            .deserialize_value_for_key("scores", &key, latest)
            .expect("the key decodes"),
        Value::U16(300)
    );
    let document = Document::V0(DocumentV0 {
        id: Identifier::from([1; 32]),
        owner_id: Identifier::from([2; 32]),
        properties: [(
            "scores".to_string(),
            Value::Array(vec![Value::U16(300), Value::U16(2), Value::U16(128)]),
        )]
        .into_iter()
        .collect(),
        ..Default::default()
    });
    assert_eq!(
        document
            .get_raw_array_elements_for_document_type("scores", document_type, latest)
            .expect("the elements"),
        vec![
            2u16.to_be_bytes().to_vec(),
            128u16.to_be_bytes().to_vec(),
            300u16.to_be_bytes().to_vec()
        ],
        "the element keys sort as the scores do"
    );
}
