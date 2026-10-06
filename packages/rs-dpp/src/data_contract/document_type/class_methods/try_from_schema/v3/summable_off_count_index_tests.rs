//! `summableOffCountIndex` indexes (protocol version 14): an index that keeps, per group, the
//! number of its source index's entries in that group as a sum item, admitted only when that
//! count is lossless. Its count and sum read the source's entries, and its average divides them
//! by the groups.

use super::refusal_test_support::assert_refused;
use crate::data_contract::config::moderation::{ContractModerationConfig, ContractModerators};
use crate::data_contract::config::DataContractConfig;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::DocumentType;
use crate::ProtocolError;
use platform_value::{platform_value, Identifier, Value};
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

const CONTRACT_ID: [u8; 32] = [9; 32];

fn identifier(position: u64, refers_to: Option<Value>) -> Value {
    let mut property = platform_value!({
        "type": "array",
        "byteArray": true,
        "minItems": 32,
        "maxItems": 32,
        "contentMediaType": "application/x.dash.dpp.identifier",
        "position": position,
    });
    if let (Some(refers_to), Value::Map(map)) = (refers_to, &mut property) {
        map.push((Value::Text("refersTo".to_string()), refers_to));
    }
    property
}

/// A post only the moderators take down, whose hashtag never changes and stays on the
/// record of a removal.
fn post() -> Value {
    platform_value!({
        "type": "object",
        "documentsMutable": true,
        "canBeDeleted": false,
        "immutable": ["hashtag"],
        "moderatorAbilities": { "delete": true, "deleteKeepsFields": ["hashtag"] },
        "properties": {
            "hashtag": { "type": "string", "minLength": 1, "maxLength": 59, "position": 0 },
            "text": { "type": "string", "maxLength": 280, "position": 1 },
        },
        "required": ["$createdAt", "$updatedAt"],
        "additionalProperties": false,
    })
}

fn author_post() -> Value {
    platform_value!({
        "name": "byAuthorPost",
        "properties": [{ "postAuthor": "asc" }, { "postId": "asc" }],
        "summableOffCountIndex": "byPost",
        "rangeCountable": true,
        "rangeSummable": true,
        "rankedSummable": { "at": ["postAuthor", "postId"] },
        "rankedAverageable": { "at": ["postAuthor"] },
        "preallocated": true,
    })
}

fn hashtag_post() -> Value {
    platform_value!({
        "name": "byHashtagPost",
        "properties": [{ "hashtag": "asc" }, { "postId": "asc" }],
        "summableOffCountIndex": "byPost",
        "rangeCountable": true,
        "rangeSummable": true,
        "rankedSummable": { "at": ["hashtag", "postId"] },
        "rankedAverageable": { "at": ["hashtag"] },
        "skipIfAbsent": true,
        "preallocated": true,
    })
}

/// Yappr's indexOnly like: `byPost` keeps one entry per like, and the author and hashtag
/// indexes keep only a counter per post of it, through the reference's `where`.
fn like(indices: Vec<Value>) -> Value {
    platform_value!({
        "type": "object",
        "indexOnly": true,
        "documentsMutable": false,
        "canBeDeleted": true,
        "properties": {
            "postId": identifier(0, Some(platform_value!({
                "type": "moderatedDocument",
                "documentType": "post",
                "where": { "hashtag": "hashtag", "$ownerId": "postAuthor" },
            }))),
            // An average ranking at the hashtag level keys its secondary by a 16-byte
            // sort key and the hashtag, which caps the hashtag at 59 characters
            "hashtag": { "type": "string", "minLength": 1, "maxLength": 59, "position": 1 },
            "postAuthor": identifier(2, None),
        },
        "indices": indices,
        "required": ["postId", "postAuthor"],
        "additionalProperties": false,
    })
}

fn by_post() -> Value {
    platform_value!({
        "name": "byPost",
        "properties": [{ "postId": "asc" }],
        "terminal": "$ownerId",
        "rangeCountable": true,
        "rankedCountable": true,
        "preallocated": true,
    })
}

fn config() -> DataContractConfig {
    DataContractConfig::default_for_version(PlatformVersion::latest())
        .expect("default config available")
        .with_moderation(Some(ContractModerationConfig {
            banlist: false,
            suspensions: false,
            moderators: ContractModerators::ContractOwner,
            warnings: false,
        }))
}

fn parse(
    post: Value,
    like: Value,
    full_validation: bool,
) -> Result<BTreeMap<String, DocumentType>, ProtocolError> {
    parse_schemas(
        BTreeMap::from([("post".to_string(), post), ("like".to_string(), like)]),
        full_validation,
    )
}

fn parse_schemas(
    schemas: BTreeMap<String, Value>,
    full_validation: bool,
) -> Result<BTreeMap<String, DocumentType>, ProtocolError> {
    let config = config();
    DocumentType::create_document_types_from_document_schemas(
        Identifier::new(CONTRACT_ID),
        1,
        config.version(),
        schemas,
        None,
        &BTreeMap::new(),
        &config,
        full_validation,
        false,
        &mut vec![],
        PlatformVersion::latest(),
    )
}

fn with(mut index: Value, key: &str, value: Value) -> Value {
    index.set_value(key, value).expect("index key applies");
    index
}

#[test]
fn should_parse_the_summable_off_count_author_and_hashtag_indexes_of_a_like() {
    for full_validation in [false, true] {
        let document_types = parse(
            post(),
            like(vec![by_post(), author_post(), hashtag_post()]),
            full_validation,
        )
        .expect("the contract should parse");
        let like = document_types.get("like").expect("the like type");
        for (name, first) in [("byAuthorPost", "postAuthor"), ("byHashtagPost", "hashtag")] {
            let index = like.indexes().get(name).expect("the index");
            assert_eq!(
                index.summable_off_count_index.as_deref(),
                Some("byPost"),
                "{name}"
            );
            assert!(index.is_summable_off_count_index(), "{name}");
            assert_eq!(index.summed_value_name(), Some("byPost"), "{name}");
            assert_eq!(index.summable, None, "{name} names no summed property");
            assert_eq!(index.terminal, None, "{name} keeps no member key");
            // Naming the last property in `at` is the terminal ranking
            assert!(index.ranked_summable, "{name}");
            assert_eq!(index.ranked_summable_at, vec![first.to_string()], "{name}");
            assert!(!index.ranked_averageable, "{name}");
            assert_eq!(
                index.ranked_averageable_at,
                vec![first.to_string()],
                "{name}"
            );
            assert_eq!(index.shallowest_count_chain_position(), Some(0), "{name}");
            assert_eq!(index.shallowest_sum_chain_position(), Some(0), "{name}");
        }
        let by_post = like.indexes().get("byPost").expect("byPost");
        assert!(!by_post.is_summable_off_count_index());
        assert!(by_post.terminal.is_some());
    }
}

#[test]
fn should_parse_a_count_ranking_of_a_counter_index_as_its_sum_ranking() {
    // A document count is the counters' sums, so `rankedCountable` declares
    // the ranking `rankedSummable` does, and needs no `rangeCountable`.
    let mut counted = author_post();
    counted
        .remove_value_at_path("rankedSummable")
        .expect("the sum ranking");
    let counted = with(
        counted,
        "rankedCountable",
        platform_value!({ "at": ["postAuthor", "postId"] }),
    );
    let mut unaveraged = author_post();
    for keyword in ["rankedSummable", "rankedAverageable", "rangeCountable"] {
        unaveraged
            .remove_value_at_path(keyword)
            .expect("the keyword is declared");
    }
    let unaveraged = with(
        unaveraged,
        "rankedCountable",
        platform_value!({ "at": "postAuthor" }),
    );
    for full_validation in [false, true] {
        let summed = parse(
            post(),
            like(vec![by_post(), author_post(), hashtag_post()]),
            full_validation,
        )
        .expect("the sum ranking parses");
        let summed = summed["like"].indexes()["byAuthorPost"].clone();
        let document_types = parse(
            post(),
            like(vec![by_post(), counted.clone(), hashtag_post()]),
            full_validation,
        )
        .expect("the count ranking parses");
        let index = &document_types["like"].indexes()["byAuthorPost"];
        assert!(!index.ranked_countable);
        assert!(index.ranked_countable_at.is_empty());
        assert_eq!(index, &summed, "the same index as the sum ranking");

        let document_types = parse(
            post(),
            like(vec![by_post(), unaveraged.clone(), hashtag_post()]),
            full_validation,
        )
        .expect("a count ranking needs no rangeCountable here");
        let index = &document_types["like"].indexes()["byAuthorPost"];
        assert!(!index.ranked_summable, "no ranking at the last property");
        assert_eq!(index.ranked_summable_at, vec!["postAuthor".to_string()]);
        assert!(index.ranked_countable_at.is_empty());
        assert_eq!(index.shallowest_sum_chain_position(), Some(0));
        assert_eq!(index.shallowest_count_chain_position(), None);
    }
}

#[test]
fn should_stamp_the_sum_and_average_chain_down_to_the_counter() {
    let document_types = parse(
        post(),
        like(vec![by_post(), author_post(), hashtag_post()]),
        true,
    )
    .expect("the contract should parse");
    let like = document_types.get("like").expect("the like type");
    for first in ["postAuthor", "hashtag"] {
        let grouping = like
            .index_structure()
            .sub_levels()
            .get(first)
            .expect("the first level");
        // Ranked by likes and by likes per post: the level carries both
        assert!(grouping.ranked_sum_grouping(), "{first}");
        assert!(grouping.ranked_average_grouping(), "{first}");
        assert!(!grouping.ranked_count_grouping(), "{first}");
        assert!(
            !grouping.count_propagating() && !grouping.sum_propagating(),
            "{first}"
        );
        assert!(
            grouping.chain_carries_counts() && grouping.chain_carries_sums(),
            "{first}"
        );
        assert!(grouping.has_index_with_type().is_none(), "{first}");
        let counter_level = grouping
            .sub_levels()
            .get("postId")
            .expect("the postId level");
        let info = counter_level
            .has_index_with_type()
            .expect("the index ends at postId");
        assert!(info.is_summable_off_count_index(), "{first}");
        assert!(info.ranked_summable && !info.ranked_averageable, "{first}");
        assert_eq!(info.terminal, None, "{first}");
        assert!(!counter_level.is_ranked_chain_level(), "{first}");
        assert!(!counter_level.count_exempt_branch(), "{first}");
    }
}

#[test]
fn should_carry_sums_through_a_level_ranking_by_neither() {
    // Ranked by likes at the first level only: the second level carries the sums up
    let index = platform_value!({
        "name": "byHashtagAuthorPost",
        "properties": [{ "hashtag": "asc" }, { "postAuthor": "asc" }, { "postId": "asc" }],
        "summableOffCountIndex": "byPost",
        "rangeSummable": true,
        "rankedSummable": { "at": "hashtag" },
        "skipIfAbsent": true,
    });
    let document_types =
        parse(post(), like(vec![by_post(), index]), false).expect("the contract should parse");
    let like = document_types.get("like").expect("the like type");
    let hashtag = like
        .index_structure()
        .sub_levels()
        .get("hashtag")
        .expect("the hashtag level");
    assert!(hashtag.ranked_sum_grouping());
    assert!(hashtag.chain_carries_sums() && !hashtag.chain_carries_counts());
    let author = hashtag
        .sub_levels()
        .get("postAuthor")
        .expect("the author level");
    assert!(author.sum_propagating() && !author.count_propagating());
    assert!(!author.ranked_sum_grouping());
}

#[test]
fn should_refuse_a_sum_ranking_at_an_earlier_level_of_another_index() {
    let index = platform_value!({
        "name": "byAuthorPost",
        "properties": [{ "postAuthor": "asc" }, { "postId": "asc" }],
        "terminal": "$ownerId",
        "rangeCountable": true,
        "rangeSummable": true,
        "rankedSummable": { "at": "postAuthor" },
    });
    assert_refused(
        parse(post(), like(vec![by_post(), index]), false),
        "only allowed on a summableOffCountIndex index",
    );
}

/// A stored `tip` type whose one index is `index`.
fn parse_tip(
    index: Value,
    full_validation: bool,
    platform_version: &PlatformVersion,
) -> Result<BTreeMap<String, DocumentType>, ProtocolError> {
    let indices = vec![index];
    let tip = platform_value!({
        "type": "object",
        "properties": {
            "recipient": { "type": "string", "minLength": 1, "maxLength": 32, "position": 0 },
            "amount": { "type": "integer", "minimum": 1, "maximum": 1000, "position": 1 },
        },
        "indices": indices,
        "required": ["recipient", "amount"],
        "additionalProperties": false,
    });
    let config = DataContractConfig::default_for_version(platform_version).expect("default config");
    DocumentType::create_document_types_from_document_schemas(
        Identifier::new(CONTRACT_ID),
        1,
        config.version(),
        BTreeMap::from([("tip".to_string(), tip)]),
        None,
        &BTreeMap::new(),
        &config,
        full_validation,
        false,
        &mut vec![],
        platform_version,
    )
}

#[test]
fn should_refuse_the_object_form_of_a_sum_ranking_naming_the_last_property_of_another_index() {
    // Naming the last property folds into the boolean, but the form itself is
    // what only a summableOffCountIndex index admits
    for index in [
        platform_value!({
            "name": "byRecipient",
            "properties": [{ "recipient": "asc" }],
            "summable": "amount",
            "rangeSummable": true,
            "rankedSummable": { "at": "recipient" },
        }),
        platform_value!({
            "name": "byRecipient",
            "properties": [{ "recipient": "asc" }],
            "averageable": "amount",
            "rangeAverageable": true,
            "rankedAverageable": { "at": ["recipient"] },
        }),
    ] {
        assert_refused(
            parse_tip(index.clone(), false, PlatformVersion::latest()),
            "only allowed on a summableOffCountIndex index",
        );
        assert!(parse_tip(index, true, PlatformVersion::latest()).is_err());
    }
    let boolean = platform_value!({
        "name": "byRecipient",
        "properties": [{ "recipient": "asc" }],
        "summable": "amount",
        "rangeSummable": true,
        "rankedSummable": true,
    });
    for full_validation in [false, true] {
        parse_tip(boolean.clone(), full_validation, PlatformVersion::latest())
            .expect("the boolean form parses on any index");
    }
}

#[test]
fn should_refuse_the_keywords_before_protocol_version_14() {
    let counter = platform_value!({
        "name": "byRecipient",
        "properties": [{ "recipient": "asc" }],
        "summableOffCountIndex": "byAmount",
        "rangeSummable": true,
    });
    let ranked = platform_value!({
        "name": "byRecipient",
        "properties": [{ "recipient": "asc" }],
        "summable": "amount",
        "rangeSummable": true,
        "rankedSummable": { "at": "recipient" },
    });
    // Protocol version 13 knows neither: both fall to the unknown-key arm, as
    // on a node without them
    let platform_version_13 = PlatformVersion::get(13).expect("PV13 exists");
    for index in [counter.clone(), ranked.clone()] {
        let error = parse_tip(index.clone(), false, platform_version_13)
            .expect_err("PV13 refuses the keyword");
        assert!(
            format!("{error:?}").contains("unexpected property name"),
            "{error:?}"
        );
        assert!(parse_tip(index, true, platform_version_13).is_err());
    }
    // Protocol version 14 parses them, and refuses them here for the type's
    // own reasons
    assert_refused(
        parse_tip(counter, false, PlatformVersion::latest()),
        "only allowed on indexOnly document types",
    );
    assert_refused(
        parse_tip(ranked, false, PlatformVersion::latest()),
        "only allowed on a summableOffCountIndex index",
    );
}

#[test]
fn should_refuse_summable_off_count_index_without_range_summable() {
    let index = platform_value!({
        "name": "byAuthorPost",
        "properties": [{ "postAuthor": "asc" }, { "postId": "asc" }],
        "summableOffCountIndex": "byPost",
        "rangeCountable": true,
    });
    // A count ranking is the sum ranking here, and the refusal still names
    // the keyword the author left out.
    for index in [
        index.clone(),
        with(index.clone(), "rankedCountable", Value::Bool(true)),
        with(
            index,
            "rankedCountable",
            platform_value!({ "at": ["postAuthor"] }),
        ),
    ] {
        assert_refused(
            parse(post(), like(vec![by_post(), index]), false),
            "needs rangeSummable: true",
        );
    }
}

#[test]
fn should_refuse_summable_off_count_index_with_a_terminal() {
    assert_refused(
        parse(
            post(),
            like(vec![
                by_post(),
                with(author_post(), "terminal", Value::Text("$ownerId".into())),
            ]),
            false,
        ),
        "takes no terminal",
    );
}

#[test]
fn should_refuse_summable_off_count_index_with_a_summed_property() {
    assert_refused(
        parse(
            post(),
            like(vec![
                by_post(),
                with(author_post(), "summable", Value::Text("postAuthor".into())),
            ]),
            false,
        ),
        "names no summable or averageable property",
    );
}

#[test]
fn should_refuse_summable_off_count_index_naming_no_index() {
    assert_refused(
        parse(
            post(),
            like(vec![
                by_post(),
                with(
                    author_post(),
                    "summableOffCountIndex",
                    Value::Text("byNothing".into()),
                ),
            ]),
            false,
        ),
        "which is not an index of the document type",
    );
}

#[test]
fn should_refuse_a_summable_off_count_index_source() {
    // Named to sort before byAuthorPost, so its own rule is the one checked first
    let index = platform_value!({
        "name": "byAuthorCountPerPost",
        "properties": [{ "postId": "asc" }, { "postAuthor": "asc" }],
        "summableOffCountIndex": "byAuthorPost",
        "rangeSummable": true,
    });
    assert_refused(
        parse(post(), like(vec![by_post(), author_post(), index]), false),
        "which is a summableOffCountIndex index itself",
    );
}

#[test]
fn should_refuse_two_sources_on_one_document_type() {
    let by_post_again = platform_value!({
        "name": "byPostAgain",
        "properties": [{ "postId": "asc" }, { "postAuthor": "asc" }],
        "terminal": "$ownerId",
    });
    let hashtag = with(
        hashtag_post(),
        "summableOffCountIndex",
        Value::Text("byPostAgain".into()),
    );
    assert_refused(
        parse(
            post(),
            like(vec![by_post(), by_post_again, author_post(), hashtag]),
            false,
        ),
        "a document type keeps one summed value",
    );
}

#[test]
fn should_refuse_a_source_named_like_a_property() {
    let source = with(by_post(), "name", Value::Text("hashtag".into()));
    let index = with(
        author_post(),
        "summableOffCountIndex",
        Value::Text("hashtag".into()),
    );
    assert_refused(
        parse(post(), like(vec![source, index]), false),
        "must not share its name with a property",
    );
}

#[test]
fn should_refuse_a_source_that_skips_documents() {
    // A source keeping entries but skipping untagged likes would undercount the posts
    let tagged = platform_value!({
        "name": "byTaggedPost",
        "properties": [{ "postId": "asc" }, { "hashtag": "asc" }],
        "terminal": "$ownerId",
        "skipIfAbsent": true,
    });
    let index = with(
        hashtag_post(),
        "summableOffCountIndex",
        Value::Text("byTaggedPost".into()),
    );
    assert_refused(
        parse(post(), like(vec![by_post(), tagged, index]), false),
        "must hold every document exactly once",
    );
}

#[test]
fn should_refuse_index_shapes_a_counter_cannot_keep() {
    let window = platform_value!({ "on": "$createdAt", "range": 86400, "step": 86400 });
    for (indices, fragment) in [
        (
            vec![
                by_post(),
                with(
                    author_post(),
                    "countable",
                    Value::Text("countableAllowingOffset".into()),
                ),
            ],
            "cannot be countableAllowingOffset",
        ),
        (
            vec![
                by_post(),
                with(
                    with(
                        author_post(),
                        "properties",
                        platform_value!([
                            { "$createdAt": "asc" },
                            { "postAuthor": "asc" },
                            { "postId": "asc" },
                        ]),
                    ),
                    "timeRange",
                    window,
                ),
            ],
            "cannot declare timeRange or integerRange",
        ),
        (
            vec![
                by_post(),
                with(author_post(), "outlivesDelete", Value::Bool(true)),
            ],
            "cannot outlive deletes",
        ),
        (
            vec![
                by_post(),
                platform_value!({
                    "name": "byAuthorPost",
                    "properties": [{ "postAuthor": "asc" }, { "postId": "asc" }],
                    "summableOffCountIndex": "byPost",
                    "rangeSummable": true,
                    "unique": true,
                }),
            ],
            "cannot be unique or contested",
        ),
        (
            vec![with(author_post(), "name", Value::Text("byPost".into()))],
            "cannot name itself",
        ),
        (
            vec![
                by_post(),
                platform_value!({
                    "name": "byNothing",
                    "summableOffCountIndex": "byPost",
                    "rangeSummable": true,
                }),
            ],
            "needs properties",
        ),
    ] {
        assert_refused(parse(post(), like(indices), false), fragment);
    }
}

#[test]
fn should_refuse_a_source_that_outlives_deletes_or_involves_created_at() {
    let timed = platform_value!({
        "name": "byPostTime",
        "properties": [{ "postId": "asc" }, { "$createdAt": "asc" }],
        "terminal": "$ownerId",
    });
    let outliving = platform_value!({
        "name": "byTrendPost",
        "properties": [{ "$createdAt": "asc" }, { "postId": "asc" }],
        "terminal": "$ownerId",
        "timeRange": { "on": "$createdAt", "range": 86400, "step": 86400, "ttl": 604800 },
        "outlivesDelete": true,
    });
    for (source_name, source) in [("byPostTime", timed), ("byTrendPost", outliving)] {
        let index = with(
            author_post(),
            "summableOffCountIndex",
            Value::Text(source_name.into()),
        );
        let mut schema = like(vec![by_post(), source, index]);
        schema
            .set_value(
                "required",
                platform_value!(["postId", "postAuthor", "$createdAt"]),
            )
            .expect("required applies");
        assert_refused(
            parse(post(), schema, false),
            "must hold every document exactly once",
        );
    }
}

#[test]
fn should_refuse_an_index_lacking_a_source_property() {
    let index = platform_value!({
        "name": "byAuthor",
        "properties": [{ "postAuthor": "asc" }],
        "summableOffCountIndex": "byPost",
        "rangeSummable": true,
    });
    assert_refused(
        parse(post(), like(vec![by_post(), index]), false),
        "lacks \"postId\", a property of its source",
    );
}

#[test]
fn should_refuse_a_property_the_source_does_not_fix() {
    // `mood` is the like's own value, which no reference fixes: one post's likes would spread
    // over several moods
    let mut like = like(vec![
        by_post(),
        platform_value!({
            "name": "byMoodPost",
            "properties": [{ "mood": "asc" }, { "postId": "asc" }],
            "summableOffCountIndex": "byPost",
            "rangeSummable": true,
        }),
        platform_value!({
            "name": "byMood",
            "properties": [{ "mood": "asc" }, { "postId": "asc" }, { "postAuthor": "asc" }],
            "terminal": "$ownerId",
        }),
    ]);
    let properties = like
        .get_mut("properties")
        .expect("properties accessible")
        .expect("properties present");
    properties
        .set_value(
            "mood",
            platform_value!({ "type": "string", "minLength": 1, "maxLength": 16, "position": 3 }),
        )
        .expect("property added");
    like.set_value(
        "required",
        platform_value!(["postId", "postAuthor", "mood"]),
    )
    .expect("required set");
    assert_refused(
        parse(post(), like, false),
        "has \"mood\", which is neither a property of its source",
    );
}

#[test]
fn should_refuse_an_index_continuing_below_the_counter() {
    // An index otherwise admitted (its terminal outside its properties, its
    // optional hashtag skipped), continuing below the counter's postId
    let continuing = platform_value!({
        "name": "byAuthorPostHashtag",
        "properties": [{ "postAuthor": "asc" }, { "postId": "asc" }, { "hashtag": "asc" }],
        "terminal": "$ownerId",
        "skipIfAbsent": true,
    });
    assert_refused(
        parse(
            post(),
            like(vec![by_post(), author_post(), continuing]),
            false,
        ),
        "is continued by index \"byAuthorPostHashtag\"",
    );
}

#[test]
fn should_refuse_a_referenced_value_that_can_change() {
    // Without `immutable`, a post's hashtag may change: likes before and after would land in
    // two hashtag groups of one post
    let mut post = post();
    post.remove("immutable").expect("immutable removed");
    assert_refused(
        parse(
            post,
            like(vec![by_post(), author_post(), hashtag_post()]),
            true,
        ),
        "can change after a document is written",
    );
}

#[test]
fn should_refuse_a_referenced_value_a_replace_can_clear() {
    // An optional `immutable` `deletableDocument` reference may still be cleared by a replace
    // once its target is deleted: likes before and after would land in two groups of one post
    let post = platform_value!({
        "type": "object",
        "documentsMutable": true,
        "canBeDeleted": false,
        "immutable": ["hashtag", "draftId"],
        "moderatorAbilities": { "delete": true, "deleteKeepsFields": ["hashtag", "draftId"] },
        "properties": {
            "hashtag": { "type": "string", "minLength": 1, "maxLength": 59, "position": 0 },
            "text": { "type": "string", "maxLength": 280, "position": 1 },
            "draftId": identifier(2, Some(platform_value!({
                "type": "deletableDocument",
                "documentType": "draft",
            }))),
        },
        "required": ["$createdAt", "$updatedAt"],
        "additionalProperties": false,
    });
    let draft = platform_value!({
        "type": "object",
        "documentsMutable": false,
        "canBeDeleted": true,
        "properties": {
            "body": { "type": "string", "maxLength": 280, "position": 0 },
        },
        "additionalProperties": false,
    });
    let draft_post = platform_value!({
        "name": "byDraftPost",
        "properties": [{ "postDraft": "asc" }, { "postId": "asc" }],
        "summableOffCountIndex": "byPost",
        "rangeSummable": true,
        "skipIfAbsent": true,
    });
    let like = platform_value!({
        "type": "object",
        "indexOnly": true,
        "documentsMutable": false,
        "canBeDeleted": true,
        "properties": {
            "postId": identifier(0, Some(platform_value!({
                "type": "moderatedDocument",
                "documentType": "post",
                "where": { "hashtag": "hashtag", "$ownerId": "postAuthor", "draftId": "postDraft" },
            }))),
            "hashtag": { "type": "string", "minLength": 1, "maxLength": 59, "position": 1 },
            "postAuthor": identifier(2, None),
            "postDraft": identifier(3, None),
        },
        "indices": [by_post(), author_post(), hashtag_post(), draft_post],
        "required": ["postId", "postAuthor"],
        "additionalProperties": false,
    });
    let schemas = |post: Value| {
        BTreeMap::from([
            ("post".to_string(), post),
            ("draft".to_string(), draft.clone()),
            ("like".to_string(), like.clone()),
        ])
    };
    assert_refused(
        parse_schemas(schemas(post.clone()), true),
        "\"draftId\", which a replace can clear once its document is deleted, can change after \
         a document is written",
    );

    // A required reference is never cleared: no replace may drop a required property
    let mut required = post;
    required
        .set_value(
            "required",
            platform_value!(["$createdAt", "$updatedAt", "draftId"]),
        )
        .expect("required set");
    parse_schemas(schemas(required), true).expect("a required reference fixes the group");
}

#[test]
fn should_refuse_an_owner_a_transfer_changes() {
    let mut post = post();
    post.set_value("transferable", Value::U8(1))
        .expect("transferable set");
    post.remove("moderatorAbilities")
        .expect("moderator abilities removed");
    post.set_value("canBeDeleted", Value::Bool(false))
        .expect("canBeDeleted set");
    let mut like = like(vec![by_post(), author_post(), hashtag_post()]);
    // A permanentDocument reference: the post can never leave state, but changes owner
    like.get_mut("properties")
        .expect("properties accessible")
        .expect("properties present")
        .get_mut("postId")
        .expect("postId accessible")
        .expect("postId present")
        .get_mut("refersTo")
        .expect("refersTo accessible")
        .expect("refersTo present")
        .set_value("type", Value::Text("permanentDocument".into()))
        .expect("reference kind set");
    assert_refused(
        parse(post, like, true),
        "can change after a document is written",
    );
}

#[test]
fn should_accept_a_property_any_source_reference_fixes() {
    // `postAuthor` is fixed through two references: the post's `$ownerId`,
    // which a transfer changes, and the thread's `author`, which never
    // changes. The count stays lossless through the second.
    let mut post = post();
    post.set_value("transferable", Value::U8(1))
        .expect("transferable set");
    post.remove("moderatorAbilities")
        .expect("moderator abilities removed");
    let thread = platform_value!({
        "type": "object",
        "documentsMutable": false,
        "canBeDeleted": false,
        "properties": { "author": identifier(0, None) },
        "required": ["author"],
        "additionalProperties": false,
    });
    let like = platform_value!({
        "type": "object",
        "indexOnly": true,
        "documentsMutable": false,
        "canBeDeleted": true,
        "properties": {
            "postId": identifier(0, Some(platform_value!({
                "type": "permanentDocument",
                "documentType": "post",
                "where": { "$ownerId": "postAuthor" },
            }))),
            "threadId": identifier(1, Some(platform_value!({
                "type": "permanentDocument",
                "documentType": "thread",
                "where": { "author": "postAuthor" },
            }))),
            "postAuthor": identifier(2, None),
        },
        "indices": [
            {
                "name": "byPostThread",
                "properties": [{ "postId": "asc" }, { "threadId": "asc" }],
                "terminal": "$ownerId",
            },
            {
                "name": "byAuthorPostThread",
                "properties": [
                    { "postAuthor": "asc" },
                    { "postId": "asc" },
                    { "threadId": "asc" },
                ],
                "summableOffCountIndex": "byPostThread",
                "rangeSummable": true,
            },
        ],
        "required": ["postId", "threadId", "postAuthor"],
        "additionalProperties": false,
    });
    for full_validation in [false, true] {
        parse_schemas(
            BTreeMap::from([
                ("post".to_string(), post.clone()),
                ("thread".to_string(), thread.clone()),
                ("like".to_string(), like.clone()),
            ]),
            full_validation,
        )
        .expect("the thread reference keeps the count lossless");
    }
}

#[test]
fn should_refuse_a_value_a_removal_drops() {
    let mut post = post();
    post.set_value("moderatorAbilities", platform_value!({ "delete": true }))
        .expect("moderator abilities set");
    // Without preallocation, so the preallocation rule, which refuses the same
    // shape first, stays out of the way and the counter's own rule refuses it
    let indexes = [by_post(), author_post(), hashtag_post()]
        .into_iter()
        .map(|mut index| {
            index.remove("preallocated").expect("preallocated removed");
            index
        })
        .collect();
    let error = parse(post, like(indexes), true).expect_err("the contract should be refused");
    let message = error.to_string();
    assert!(
        message.contains("summableOffCountIndex index \"byHashtagPost\"")
            && message.contains("could no longer be read back"),
        "expected the counter's removal refusal in: {message}"
    );
}

#[test]
fn should_bound_each_ranked_level_by_the_axes_ranked_at_it() {
    // A hashtag of 60 characters takes up to 240 bytes: over the 239 an
    // average ranking's tree admits, within the 247 of a sum ranking's
    let like_with = |hashtag_index: Value| {
        let mut like = like(vec![by_post(), author_post(), hashtag_index]);
        like.set_value_at_full_path("properties.hashtag.maxLength", Value::U32(60))
            .expect("maxLength set");
        like
    };
    // The average ranking at `hashtag` caps the hashtag (a full-validation rule)
    assert_refused(parse(post(), like_with(hashtag_post()), true), "239");
    // Ranked by sum only at `hashtag`, with the average at `postId`, the
    // hashtag's tree carries the sum axis alone
    let sum_only_at_hashtag = with(
        hashtag_post(),
        "rankedAverageable",
        platform_value!({ "at": ["postId"] }),
    );
    for full_validation in [false, true] {
        let document_types = parse(
            post(),
            like_with(sum_only_at_hashtag.clone()),
            full_validation,
        )
        .expect("the sum-only hashtag level admits 60 characters");
        let index = document_types
            .get("like")
            .expect("the like type")
            .indexes()
            .get("byHashtagPost")
            .expect("the index");
        assert!(
            index.ranked_averageable,
            "the average ranks the last property"
        );
        assert!(index.ranked_averageable_at.is_empty());
    }
}

#[test]
fn should_refuse_summable_off_count_index_on_a_stored_type() {
    let mut like = like(vec![
        platform_value!({ "name": "byPost", "properties": [{ "postId": "asc" }] }),
        platform_value!({
            "name": "byAuthorPost",
            "properties": [{ "postAuthor": "asc" }, { "postId": "asc" }],
            "summableOffCountIndex": "byPost",
            "rangeSummable": true,
        }),
    ]);
    like.remove("indexOnly").expect("indexOnly removed");
    assert_refused(
        parse(post(), like, false),
        "only allowed on indexOnly document types",
    );
}
