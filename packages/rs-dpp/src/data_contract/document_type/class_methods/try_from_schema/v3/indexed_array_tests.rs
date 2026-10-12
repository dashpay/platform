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
        "hashtags.maxLength"
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

#[test]
fn should_refuse_an_indexed_array_on_an_index_only_type() {
    let schema = platform_value!({
        "type": "object",
        "indexOnly": true,
        "documentsMutable": false,
        "properties": { "hashtags": hashtags(10) },
        "required": ["hashtags"],
        "indices": [{ "name": "byHashtag", "properties": [{ "hashtags": "asc" }] }],
        "additionalProperties": false
    });
    expect_structure_error(
        parse(schema),
        "an indexOnly type's indexes are uniqueness rules",
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
