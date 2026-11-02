//! `preallocated` and `summableOffCountIndex` indexes of an indexOnly type keyed by a
//! typed array (protocol version 14): admitted when a `where` equality makes the array
//! equal to a typed array of the referenced document, the shape of Yappr's tag likes,
//! where every like of a post carries the post's `tags`, so each tag's group of a post
//! holds every like of it.

use super::refusal_test_support::assert_refused;
use super::summable_off_count_index_tests::{by_post, identifier, parse, with, CONTRACT_ID};
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::PreallocatedKeySource;
use platform_value::{platform_value, Identifier, Value};

/// Up to 4 distinct tags, as a post and a like of it carry them.
fn tags(position: u64) -> Value {
    platform_value!({
        "type": "array",
        "items": { "type": "string", "pattern": "^[a-z0-9_]{1,61}$", "maxLength": 61 },
        "maxItems": 4,
        "uniqueItems": true,
        "position": position,
    })
}

/// A post only the moderators take down, whose tags never change and stay on the record
/// of a removal.
fn post() -> Value {
    platform_value!({
        "type": "object",
        "documentsMutable": true,
        "canBeDeleted": false,
        "immutable": ["tags"],
        "moderatorAbilities": { "delete": true, "deleteKeepsFields": ["tags"] },
        "properties": {
            "content": { "type": "string", "maxLength": 500, "position": 0 },
            "tags": tags(1),
        },
        "required": ["$createdAt", "$updatedAt"],
        "additionalProperties": false,
    })
}

/// Yappr's indexOnly like of a post, binding its `tags` and `postAuthor` to the post's
/// through `where`.
fn like(where_tags: Value, indices: Vec<Value>) -> Value {
    platform_value!({
        "type": "object",
        "indexOnly": true,
        "documentsMutable": false,
        "canBeDeleted": true,
        "properties": {
            "postId": identifier(0, Some(platform_value!({
                "type": "moderatedDocument",
                "documentType": "post",
                "where": { "tags": where_tags, "$ownerId": "postAuthor" },
            }))),
            "tags": tags(1),
            "postAuthor": identifier(2, None),
        },
        "indices": indices,
        "required": ["$createdAt", "postId", "postAuthor"],
        "additionalProperties": false,
    })
}

/// One counter per tag of a post, created at zero with the post. The array
/// is in the skip set of every index naming it, so the index takes no
/// `skipIfAbsent` of its own (a type naming the array alone there is refused).
fn by_tag_post() -> Value {
    platform_value!({
        "name": "byTagPost",
        "properties": [{ "tags": "asc" }, { "postId": "asc" }],
        "summableOffCountIndex": "byPost",
        "rangeCountable": true,
        "rangeSummable": true,
        "rankedCountable": { "at": ["tags", "postId"] },
        "preallocated": true,
    })
}

/// One counter per post of an author, which keeps `postAuthor`.
fn by_author_post() -> Value {
    platform_value!({
        "name": "byAuthorPost",
        "properties": [{ "postAuthor": "asc" }, { "postId": "asc" }],
        "summableOffCountIndex": "byPost",
        "rangeCountable": true,
        "rangeSummable": true,
        "rankedCountable": { "at": ["postAuthor", "postId"] },
        "preallocated": true,
    })
}

fn by_trend_tag_post() -> Value {
    platform_value!({
        "name": "byTrendTagPost",
        "properties": [{ "$createdAt": "asc" }, { "tags": "asc" }, { "postId": "asc" }],
        "terminal": "$ownerId",
        "rangeCountable": true,
        "rankedCountable": { "at": ["tags", "postId"] },
        "timeRange": { "on": "$createdAt", "range": 86400, "step": 21600, "ttl": 604800 },
        "outlivesDelete": true,
    })
}

fn yappr_like() -> Value {
    like(
        Value::Text("tags".to_string()),
        vec![
            by_post(),
            by_tag_post(),
            by_author_post(),
            by_trend_tag_post(),
        ],
    )
}

fn without(mut index: Value, key: &str) -> Value {
    index.remove(key).expect("index key removed");
    index
}

/// The whole Yappr contract parses, validating or not: `byTagPost` is preallocated
/// through the post's tags, one key per tag, and its counters are fixed by `byPost`
/// through the same equality, so `tags` needs no index of its own.
#[test]
fn should_parse_a_preallocated_counter_per_tag_of_a_post() {
    for full_validation in [true, false] {
        let document_types =
            parse(post(), yappr_like(), full_validation).expect("the contract should parse");
        let like = document_types.get("like").expect("the like type");
        let index = like.indexes().get("byTagPost").expect("byTagPost");
        assert_eq!(index.indexed_array.as_deref(), Some("tags"));
        assert_eq!(index.summable_off_count_index.as_deref(), Some("byPost"));
        assert!(index.preallocated);
        assert_eq!(index.skip_if_absent_properties, ["tags"]);

        let bindings = index.preallocation_bindings_for_target(
            like.flattened_properties(),
            Identifier::new(CONTRACT_ID),
            document_types.get("post").expect("the post type").as_ref(),
        );
        assert_eq!(bindings.len(), 1);
        assert_eq!(
            bindings[0].key_sources,
            vec![
                PreallocatedKeySource::ReferencedDocumentListElements("tags"),
                PreallocatedKeySource::ReferencedDocumentId,
            ]
        );

        let source = like.indexes().get("byPost").expect("byPost");
        let derivations = index
            .count_index_derivations(
                source,
                like.flattened_properties(),
                Identifier::new(CONTRACT_ID),
            )
            .expect("the tags are fixed by byPost");
        assert_eq!(
            derivations
                .iter()
                .map(|derivation| (derivation.property, derivation.referenced))
                .collect::<Vec<_>>(),
            vec![("tags", "tags")]
        );
        assert!(like
            .index_structure()
            .sub_levels()
            .get("tags")
            .expect("the tag level")
            .sub_levels()
            .get("postId")
            .expect("the post level under a tag")
            .summable_off_count_index_info()
            .is_some());
    }
}

/// The array may sit at any position of the index: a counter per tag under each post.
#[test]
fn should_parse_a_preallocated_counter_index_ending_at_the_array() {
    let tags_under_post = platform_value!({
        "name": "byPostTag",
        "properties": [{ "postId": "asc" }, { "tags": "asc" }],
        "summableOffCountIndex": "byPost",
        "rangeSummable": true,
        "preallocated": true,
    });
    let document_types = parse(
        post(),
        like(
            Value::Text("tags".to_string()),
            vec![by_post(), by_author_post(), tags_under_post],
        ),
        true,
    )
    .expect("the contract should parse");
    let like = document_types.get("like").expect("the like type");
    let index = like.indexes().get("byPostTag").expect("byPostTag");
    assert_eq!(
        index.preallocation_bindings(like.flattened_properties(), Identifier::new(CONTRACT_ID))[0]
            .key_sources,
        vec![
            PreallocatedKeySource::ReferencedDocumentId,
            PreallocatedKeySource::ReferencedDocumentListElements("tags"),
        ]
    );
}

/// `containsAll` and `contains` hold for likes whose tags only share some of the post's,
/// which would spread the likes of one post over different groups: neither binds the
/// array, so neither keyword is admitted on its index, on every parse.
#[test]
fn should_refuse_an_array_index_bound_only_by_a_contains() {
    let contains_all = || platform_value!({ "containsAll": "tags" });
    for full_validation in [true, false] {
        assert_refused(
            parse(
                post(),
                like(
                    contains_all(),
                    vec![by_post(), without(by_tag_post(), "preallocated")],
                ),
                full_validation,
            ),
            "a counter's group is one element, and only an equality enters every document",
        );
        let tag_likes = platform_value!({
            "name": "byTagPostLiker",
            "properties": [{ "tags": "asc" }, { "postId": "asc" }],
            "terminal": "$ownerId",
            "preallocated": true,
        });
        assert_refused(
            parse(
                post(),
                like(contains_all(), vec![by_post(), tag_likes]),
                full_validation,
            ),
            "the preallocated path is fixed by one referenced document, and only an equality",
        );
    }
}

/// A counter per tag needs tags that never change: likes of a post before and after a
/// change of its tags would land in different groups.
#[test]
fn should_refuse_a_counter_over_tags_a_post_can_change() {
    let mut post = post();
    post.remove("immutable").expect("immutable removed");
    assert_refused(
        parse(post, yappr_like(), true),
        "can change after a document is written",
    );
}

/// Through a moderatedDocument reference the removal record must keep the tags, for the
/// preallocated trees and for the counters alike.
#[test]
fn should_refuse_tags_a_removal_drops() {
    let mut post = post();
    post.set_value("moderatorAbilities", platform_value!({ "delete": true }))
        .expect("moderator abilities set");
    assert_refused(
        parse(post.clone(), yappr_like(), true),
        "but \"tags\" of \"post\" is not kept by a moderator's removal",
    );
    let indexes = vec![
        without(by_post(), "preallocated"),
        without(by_tag_post(), "preallocated"),
        without(by_author_post(), "preallocated"),
        by_trend_tag_post(),
    ];
    let error = parse(post, like(Value::Text("tags".to_string()), indexes), true)
        .expect_err("the contract should be refused");
    let message = error.to_string();
    assert!(
        message.contains("summableOffCountIndex index \"byTagPost\"")
            && message.contains("could no longer be read back"),
        "expected the counter's removal refusal in: {message}"
    );
}

/// Each tag of a like still has a group of its own: an array that may repeat an element
/// is refused on a counter index too.
#[test]
fn should_refuse_a_counter_over_tags_without_unique_items() {
    let mut like = yappr_like();
    like.set_value(
        "properties",
        platform_value!({
            "postId": identifier(0, Some(platform_value!({
                "type": "moderatedDocument",
                "documentType": "post",
                "where": { "tags": "tags", "$ownerId": "postAuthor" },
            }))),
            "tags": with(tags(1), "uniqueItems", Value::Bool(false)),
            "postAuthor": identifier(2, None),
        }),
    )
    .expect("properties set");
    assert_refused(
        parse(post(), like, true),
        "index \"byTagPost\" on indexOnly document type \"like\" names the typed array \"tags\", \
         which must declare `uniqueItems: true`",
    );
}
