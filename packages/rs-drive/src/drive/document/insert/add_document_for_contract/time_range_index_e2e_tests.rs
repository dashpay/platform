//! End-to-end coverage for time-range index fan-out: a single document is
//! indexed under every overlapping range bucket its `$createdAt` falls
//! into, those buckets are queryable by exact bucket start, and deletion
//! removes every entry.
//!
//! Also covers the other half of that contract: a bucket-start equality
//! only means "bucket" when it came from `IN_TIME_RANGE` resolution, so
//! index selection is pinned by
//! [`DriveDocumentQuery::resolved_time_ranges`] rather than left to
//! whichever index happens to cover the fields.
use crate::config::DriveConfig;
use crate::drive::Drive;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use crate::query::{
    resolve_time_range_bucket_clause, DriveDocumentQuery, ResolvedTimeRange, TimeRangeGridSpec,
    TimeRangeSelector,
};
use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::util::storage_flags::StorageFlags;
use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::DataContractFactory;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::{Document, DocumentV0, DocumentV0Getters, DocumentV0Setters};
use dpp::fee::fee_result::FeeResult;
use dpp::platform_value::{platform_value, Identifier, Value};
use dpp::prelude::DataContract;
use dpp::version::PlatformVersion;
use std::borrow::Cow;
use std::collections::BTreeMap;

/// One hour in each of the two units these tests deal in: `*_SECONDS`
/// declares a contract's window, `*_MS` is a document timestamp, a bucket
/// start or an index key. Scaling the wrong one silently shifts the
/// buckets by a factor of a thousand, so they are kept apart by name.
const HOUR_SECONDS: u64 = 3_600;
const HOUR_MS: u64 = 3_600_000;

/// Deterministic 32-byte fixture identifier derived from the document's
/// own fixture inputs. Identifiers here are plumbing, not test inputs:
/// fixed bytes keep a failing GroveDB fixture reproducible run-to-run and
/// avoid an OS-entropy dependency (and its unwrap) in
/// consensus-sensitive tests. `marker` separates namespaces (document id
/// vs owner) and same-timestamp siblings.
fn fixture_bytes(marker: u8, created_at: u64, tag: &str) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    bytes[0] = marker;
    bytes[1..9].copy_from_slice(&created_at.to_be_bytes());
    for (i, byte) in tag.bytes().take(23).enumerate() {
        bytes[9 + i] = byte;
    }
    bytes
}

/// A latest-protocol `post` document type with a `(timeRange($createdAt, range=6h,
/// step=2h), hashtag)` countable index — i.e. trending hashtags over a
/// 6-hour window refreshed every 2 hours (overlap factor 3).
fn build_trending_contract() -> DataContract {
    let factory =
        DataContractFactory::new(PlatformVersion::latest().protocol_version).expect("factory");
    let index_map = vec![
        (
            Value::Text("name".to_string()),
            Value::Text("trending".to_string()),
        ),
        (
            Value::Text("properties".to_string()),
            Value::Array(vec![
                platform_value!({"$createdAt": "asc"}),
                platform_value!({"hashtag": "asc"}),
            ]),
        ),
        (
            Value::Text("timeRange".to_string()),
            Value::Map(vec![
                (
                    Value::Text("on".to_string()),
                    Value::Text("$createdAt".to_string()),
                ),
                (
                    Value::Text("range".to_string()),
                    Value::U64(6 * HOUR_SECONDS),
                ),
                (
                    Value::Text("step".to_string()),
                    Value::U64(2 * HOUR_SECONDS),
                ),
            ]),
        ),
        (
            Value::Text("countable".to_string()),
            Value::Text("countable".to_string()),
        ),
    ];

    let document_schema = platform_value!({
        "type": "object",
        "properties": {
            "hashtag": {"type": "string", "maxLength": 63, "position": 0},
        },
        "required": ["hashtag", "$createdAt"],
        "indices": Value::Array(vec![Value::Map(index_map)]),
        "additionalProperties": false,
    });
    let schemas = platform_value!({ "post": document_schema });
    let owner_id = Identifier::from([201u8; 32]);
    factory
        .create_with_value_config(owner_id, 0, schemas, None, None)
        .expect("create contract")
        .data_contract_owned()
}

/// Number of documents the `trending` index returns for an exact
/// `$createdAt == bucket` lookup.
///
/// The equality is what `IN_TIME_RANGE` resolution produces, so the query
/// is marked as such: without that provenance index selection refuses to
/// bind a bare `$createdAt` equality to a bucketed index, exactly as it
/// refuses a client-written one.
fn count_in_bucket(
    drive: &Drive,
    contract: &DataContract,
    bucket: u64,
    platform_version: &PlatformVersion,
) -> usize {
    let document_type = contract.document_type_for_name("post").expect("post");
    let query = build_created_at_query(
        contract,
        document_type,
        bucket,
        None,
        created_at_resolution(document_type),
    );
    query
        .execute_raw_results_no_proof(drive, None, None, platform_version)
        .expect("query")
        .0
        .len()
}

/// The provenance a real `IN_TIME_RANGE` resolution against this document
/// type's (single) `$createdAt` grid would have produced: the field plus
/// the exact transform, which is what pins index selection to the grid.
fn created_at_resolution(document_type: DocumentTypeRef) -> Vec<ResolvedTimeRange> {
    let transform = document_type
        .indexes()
        .values()
        .find_map(|index| index.time_range.clone())
        .expect("the fixture declares a time-range index");
    vec![ResolvedTimeRange { transform }]
}

/// A `$createdAt == created_at` query, optionally ANDed with
/// `hashtag == <hashtag>`, carrying `resolved_time_ranges` verbatim
/// so tests can drive both the resolved and the raw (empty) provenance.
fn build_created_at_query<'a>(
    contract: &'a DataContract,
    document_type: DocumentTypeRef<'a>,
    created_at: u64,
    hashtag: Option<&str>,
    resolved_time_ranges: Vec<ResolvedTimeRange>,
) -> DriveDocumentQuery<'a> {
    let mut clauses = vec![Value::Array(vec![
        Value::Text("$createdAt".to_string()),
        Value::Text("==".to_string()),
        Value::U64(created_at),
    ])];
    if let Some(hashtag) = hashtag {
        clauses.push(Value::Array(vec![
            Value::Text("hashtag".to_string()),
            Value::Text("==".to_string()),
            Value::Text(hashtag.to_string()),
        ]));
    }
    let query_value = Value::Map(vec![(
        Value::Text("where".to_string()),
        Value::Array(clauses),
    )]);
    let mut query = DriveDocumentQuery::from_value(
        query_value,
        contract,
        document_type,
        &DriveConfig::default(),
        PlatformVersion::latest(),
    )
    .expect("build query");
    query.resolved_time_ranges = resolved_time_ranges;
    query
}

#[test]
fn time_range_insert_fans_out_to_overlapping_buckets_and_delete_removes_them() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = build_trending_contract();

    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            platform_version,
        )
        .expect("apply contract");

    let document_type = contract.document_type_for_name("post").expect("post");
    let transform = document_type
        .indexes()
        .get("trending")
        .expect("trending index")
        .time_range
        .clone()
        .expect("time range transform");
    assert_eq!(transform.overlap_factor(), 3);

    // A document created at 7h+ falls into the ranges starting at 6h, 4h, 2h.
    let created_at = 7 * HOUR_MS + 123_456;
    let expected_buckets = transform.containing_buckets(created_at);
    assert_eq!(
        expected_buckets,
        vec![6 * HOUR_MS, 4 * HOUR_MS, 2 * HOUR_MS]
    );

    let owner_bytes = fixture_bytes(1, created_at, "ibiza");
    let document = Document::V0(DocumentV0 {
        id: Identifier::from(fixture_bytes(2, created_at, "ibiza")),
        owner_id: Identifier::from(owner_bytes),
        properties: BTreeMap::from([("hashtag".to_string(), Value::Text("ibiza".to_string()))]),
        created_at: Some(created_at),
        ..Default::default()
    });
    let document_id = document.id();

    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((
                        &document,
                        StorageFlags::optional_default_as_cow(),
                    )),
                    owner_id: Some(owner_bytes),
                },
                contract: &contract,
                document_type,
            },
            false,
            BlockInfo::default(),
            true,
            None,
            platform_version,
            None,
        )
        .expect("add document");

    // The document is queryable under each of its 3 overlapping buckets.
    for bucket in &expected_buckets {
        assert_eq!(
            count_in_bucket(&drive, &contract, *bucket, platform_version),
            1,
            "document should be indexed under bucket {bucket}"
        );
    }
    // It is NOT stored under the raw timestamp (only under bucket starts)…
    assert_eq!(
        count_in_bucket(&drive, &contract, created_at, platform_version),
        0,
        "document must be indexed under bucket starts, not the raw timestamp"
    );
    // …nor under a range that does not contain it.
    assert_eq!(
        count_in_bucket(&drive, &contract, 0, platform_version),
        0,
        "an unrelated bucket must be empty"
    );

    // Deleting the document removes every bucket entry.
    drive
        .delete_document_for_contract(
            document_id,
            &contract,
            "post",
            BlockInfo::default(),
            true,
            None,
            platform_version,
            None,
        )
        .expect("delete document");

    for bucket in &expected_buckets {
        assert_eq!(
            count_in_bucket(&drive, &contract, *bucket, platform_version),
            0,
            "bucket {bucket} should be empty after deletion"
        );
    }
}

/// Update-path set-diff coverage: moving a timestamp between bucket sets
/// must delete the stale entries and insert the new ones, and a
/// sub-property change at an unchanged timestamp must reinsert under the
/// new suffix without duplicating entries. (Null transitions are
/// unreachable through a valid contract: the transform's system-timestamp
/// source must be a required field, so documents always carry it; the
/// walkers' null-entry handling is defense-in-depth covered by
/// `TimeRangeTransform::entry_keys_for_raw`'s unit tests.)
#[test]
fn time_range_update_moves_between_buckets_and_suffix_changes() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = build_trending_contract();

    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            platform_version,
        )
        .expect("apply contract");

    let document_type = contract.document_type_for_name("post").expect("post");
    let transform = document_type
        .indexes()
        .get("trending")
        .expect("trending index")
        .time_range
        .clone()
        .expect("time range transform");

    let first_created_at = 7 * HOUR_MS + 123_456;
    let first_buckets = transform.containing_buckets(first_created_at);
    assert_eq!(first_buckets, vec![6 * HOUR_MS, 4 * HOUR_MS, 2 * HOUR_MS]);
    let second_created_at = 13 * HOUR_MS + 42;
    let second_buckets = transform.containing_buckets(second_created_at);
    assert_eq!(
        second_buckets,
        vec![12 * HOUR_MS, 10 * HOUR_MS, 8 * HOUR_MS]
    );

    let owner_bytes = fixture_bytes(1, first_created_at, "ibiza");
    let mut document = Document::V0(DocumentV0 {
        id: Identifier::from(fixture_bytes(2, first_created_at, "ibiza")),
        owner_id: Identifier::from(owner_bytes),
        properties: BTreeMap::from([("hashtag".to_string(), Value::Text("ibiza".to_string()))]),
        created_at: Some(first_created_at),
        revision: Some(1),
        ..Default::default()
    });

    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((
                        &document,
                        StorageFlags::optional_default_as_cow(),
                    )),
                    owner_id: Some(owner_bytes),
                },
                contract: &contract,
                document_type,
            },
            false,
            BlockInfo::default(),
            true,
            None,
            platform_version,
            None,
        )
        .expect("add document");

    let update = |document: &Document, step: &str| {
        drive
            .update_document_for_contract(
                document,
                &contract,
                document_type,
                Some(owner_bytes),
                BlockInfo::default(),
                true,
                None,
                None,
                platform_version,
                None,
            )
            .unwrap_or_else(|e| panic!("update document ({step}): {e:?}"));
    };

    // Move the timestamp to a disjoint bucket set: the stale entries must
    // be deleted and the new ones inserted.
    document.set_created_at(Some(second_created_at));
    document.set_revision(Some(2));
    update(&document, "move buckets");
    for bucket in &first_buckets {
        assert_eq!(
            count_in_bucket(&drive, &contract, *bucket, platform_version),
            0,
            "old bucket {bucket} should be empty after the timestamp moved"
        );
    }
    for bucket in &second_buckets {
        assert_eq!(
            count_in_bucket(&drive, &contract, *bucket, platform_version),
            1,
            "new bucket {bucket} should hold the document after the timestamp moved"
        );
    }

    // A sub-property change at an unchanged timestamp reinserts under the
    // new suffix without duplicating bucket entries.
    document.set("hashtag", Value::Text("mykonos".to_string()));
    document.set_revision(Some(3));
    update(&document, "suffix change");
    for bucket in &second_buckets {
        assert_eq!(
            count_in_bucket(&drive, &contract, *bucket, platform_version),
            1,
            "bucket {bucket} should still hold exactly one entry after a suffix change"
        );
    }

    // Deleting the document (now holding bucket entries created by the
    // update path) removes every entry.
    drive
        .delete_document_for_contract(
            document.id(),
            &contract,
            "post",
            BlockInfo::default(),
            true,
            None,
            platform_version,
            None,
        )
        .expect("delete document");
    for bucket in &second_buckets {
        assert_eq!(
            count_in_bucket(&drive, &contract, *bucket, platform_version),
            0,
            "bucket {bucket} should be empty after deletion"
        );
    }
}

/// The `post` type of [`build_trending_contract`] plus two plain indexes
/// over the same fields, storing raw timestamps:
///
/// - `byHashtag` — `(hashtag, $createdAt)`. Covers exactly the fields the
///   bucketed `trending` index covers, and sorts before it, so a search
///   that only scores field coverage — ties broken by the index map's name
///   order — always prefers it. Which of the two is correct depends
///   entirely on where the `$createdAt` value came from.
/// - `byHashtagAndAuthor` — `(hashtag, $createdAt, author)`. The only
///   index that can also serve an ordering by `author`, so it is what an
///   unpinned search falls back to for a time-range query that orders by
///   a property the bucketed index does not carry.
///
/// Both plain indexes start with `hashtag` rather than `$createdAt` so
/// their `$createdAt` entries hold raw timestamps at a non-leading
/// position — the competing-coverage shape these tests need. (A plain
/// index MAY lead with a bucketed field: grids fork into sibling
/// subtrees via grid-qualified level keys, so no cross-index agreement
/// rule exists; safety comes from provenance-pinned index selection.)
fn build_competing_index_trending_contract() -> DataContract {
    let factory =
        DataContractFactory::new(PlatformVersion::latest().protocol_version).expect("factory");
    let trending_index = vec![
        (
            Value::Text("name".to_string()),
            Value::Text("trending".to_string()),
        ),
        (
            Value::Text("properties".to_string()),
            Value::Array(vec![
                platform_value!({"$createdAt": "asc"}),
                platform_value!({"hashtag": "asc"}),
            ]),
        ),
        (
            Value::Text("timeRange".to_string()),
            Value::Map(vec![
                (
                    Value::Text("on".to_string()),
                    Value::Text("$createdAt".to_string()),
                ),
                (
                    Value::Text("range".to_string()),
                    Value::U64(6 * HOUR_SECONDS),
                ),
                (
                    Value::Text("step".to_string()),
                    Value::U64(2 * HOUR_SECONDS),
                ),
            ]),
        ),
        (
            Value::Text("countable".to_string()),
            Value::Text("countable".to_string()),
        ),
    ];
    let by_hashtag_index = vec![
        (
            Value::Text("name".to_string()),
            Value::Text("byHashtag".to_string()),
        ),
        (
            Value::Text("properties".to_string()),
            Value::Array(vec![
                platform_value!({"hashtag": "asc"}),
                platform_value!({"$createdAt": "asc"}),
            ]),
        ),
    ];
    let by_hashtag_and_author_index = vec![
        (
            Value::Text("name".to_string()),
            Value::Text("byHashtagAndAuthor".to_string()),
        ),
        (
            Value::Text("properties".to_string()),
            Value::Array(vec![
                platform_value!({"hashtag": "asc"}),
                platform_value!({"$createdAt": "asc"}),
                platform_value!({"author": "asc"}),
            ]),
        ),
    ];

    let document_schema = platform_value!({
        "type": "object",
        "properties": {
            "hashtag": {"type": "string", "maxLength": 63, "position": 0},
            "author": {"type": "string", "maxLength": 63, "position": 1},
        },
        "required": ["hashtag", "$createdAt"],
        "indices": Value::Array(vec![
            Value::Map(by_hashtag_index),
            Value::Map(by_hashtag_and_author_index),
            Value::Map(trending_index),
        ]),
        "additionalProperties": false,
    });
    let schemas = platform_value!({ "post": document_schema });
    let owner_id = Identifier::from([201u8; 32]);
    factory
        .create_with_value_config(owner_id, 0, schemas, None, None)
        .expect("create contract")
        .data_contract_owned()
}

/// Stores one `post` with the given timestamp and hashtag.
fn insert_post(
    drive: &Drive,
    contract: &DataContract,
    created_at: u64,
    hashtag: &str,
    platform_version: &PlatformVersion,
) {
    let document_type = contract.document_type_for_name("post").expect("post");
    let owner_bytes = fixture_bytes(1, created_at, hashtag);
    let document = Document::V0(DocumentV0 {
        id: Identifier::from(fixture_bytes(2, created_at, hashtag)),
        owner_id: Identifier::from(owner_bytes),
        properties: BTreeMap::from([("hashtag".to_string(), Value::Text(hashtag.to_string()))]),
        created_at: Some(created_at),
        ..Default::default()
    });
    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((
                        &document,
                        StorageFlags::optional_default_as_cow(),
                    )),
                    owner_id: Some(owner_bytes),
                },
                contract,
                document_type,
            },
            false,
            BlockInfo::default(),
            true,
            None,
            platform_version,
            None,
        )
        .expect("add document");
}

/// With two indexes covering the same fields, the value's provenance —
/// not the index map's name order — decides which index serves the query:
/// a resolved bucket start goes to the bucketed index, a raw timestamp to
/// the plain one. Getting this wrong returns a validly-proven empty result
/// in either direction, so both halves are asserted.
#[test]
fn resolved_time_range_equality_pins_the_bucketed_index_while_a_raw_one_uses_the_plain_index() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = build_competing_index_trending_contract();
    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            platform_version,
        )
        .expect("apply contract");

    let document_type = contract.document_type_for_name("post").expect("post");
    assert_eq!(
        document_type.indexes().keys().next().map(String::as_str),
        Some("byHashtag"),
        "the plain index must sort first for this to reproduce the tie-break the \
         pinning fixes"
    );

    let created_at = 7 * HOUR_MS + 123_456;
    let bucket = 6 * HOUR_MS;
    insert_post(&drive, &contract, created_at, "ibiza", platform_version);
    // A second post in the same bucket under a different hashtag: the
    // hashtag equality must still narrow the result to one document.
    insert_post(&drive, &contract, created_at, "mykonos", platform_version);

    let resolved = build_created_at_query(
        &contract,
        document_type,
        bucket,
        Some("ibiza"),
        created_at_resolution(document_type),
    );
    assert_eq!(
        resolved
            .find_best_index(platform_version)
            .expect("the bucketed index covers the resolved query")
            .name,
        "trending"
    );
    assert_eq!(
        resolved
            .execute_raw_results_no_proof(&drive, None, None, platform_version)
            .expect("resolved query executes")
            .0
            .len(),
        1,
        "the resolved bucket equality must find the document stored under that bucket"
    );

    let raw = build_created_at_query(&contract, document_type, created_at, Some("ibiza"), vec![]);
    assert_eq!(
        raw.find_best_index(platform_version)
            .expect("the plain index covers the raw query")
            .name,
        "byHashtag",
        "a raw `$createdAt` equality must never bind to bucket keys"
    );
    assert_eq!(
        raw.execute_raw_results_no_proof(&drive, None, None, platform_version)
            .expect("raw query executes")
            .0
            .len(),
        1,
        "the plain index stores raw timestamps, so the raw equality finds the document"
    );

    // The converse of the raw case: a bucket start is not a timestamp any
    // document carries, so the plain index legitimately matches nothing.
    let raw_on_bucket =
        build_created_at_query(&contract, document_type, bucket, Some("ibiza"), vec![]);
    assert_eq!(
        raw_on_bucket
            .execute_raw_results_no_proof(&drive, None, None, platform_version)
            .expect("raw query executes")
            .0
            .len(),
        0
    );
}

/// A contract whose only index buckets `$createdAt` alone — the shape
/// where a resolved bucket equality is the query's *last* clause with no
/// left-over index properties, so cursor pagination has nothing below the
/// transformed level except the document-id terminal.
///
/// `unique` flips the index's uniqueness: the grid already satisfies the
/// unique shape (`range == step` on `$createdAt`), and the unique terminal
/// layout stores the reference AT the value tree rather than under an id
/// terminal — the other arm the in-bucket cursor rule must handle.
fn build_single_property_trending_contract(unique: bool) -> DataContract {
    let factory =
        DataContractFactory::new(PlatformVersion::latest().protocol_version).expect("factory");
    let index_map = vec![
        (
            Value::Text("name".to_string()),
            Value::Text("byBucket".to_string()),
        ),
        (
            Value::Text("properties".to_string()),
            Value::Array(vec![platform_value!({"$createdAt": "asc"})]),
        ),
        (
            Value::Text("timeRange".to_string()),
            Value::Map(vec![
                (
                    Value::Text("on".to_string()),
                    Value::Text("$createdAt".to_string()),
                ),
                (
                    Value::Text("range".to_string()),
                    Value::U64(6 * HOUR_SECONDS),
                ),
                (
                    Value::Text("step".to_string()),
                    Value::U64(6 * HOUR_SECONDS),
                ),
            ]),
        ),
        (Value::Text("unique".to_string()), Value::Bool(unique)),
    ];

    let document_schema = platform_value!({
        "type": "object",
        "properties": {
            "hashtag": {"type": "string", "maxLength": 63, "position": 0},
        },
        "required": ["hashtag", "$createdAt"],
        "indices": Value::Array(vec![Value::Map(index_map)]),
        "additionalProperties": false,
    });
    let schemas = platform_value!({ "post": document_schema });
    let owner_id = Identifier::from([202u8; 32]);
    factory
        .create_with_value_config(owner_id, 0, schemas, None, None)
        .expect("create contract")
        .data_contract_owned()
}

/// The ids of raw query results, in returned order.
fn result_ids(
    results: &[Vec<u8>],
    document_type: DocumentTypeRef,
    platform_version: &PlatformVersion,
) -> Vec<[u8; 32]> {
    results
        .iter()
        .map(|serialized| {
            Document::from_bytes(serialized, document_type, platform_version)
                .expect("deserialize result")
                .id()
                .to_buffer()
        })
        .collect()
}

/// The transformed level stores bucket starts, so a cursor document's raw
/// timestamp must never be compared against this level's keys: an
/// included cursor created *inside* the selected bucket (07:10 in a
/// bucket starting at 06:00) orders after the bucket-start key, and
/// applying it at the transformed level suppresses the only key — a
/// validly-proven empty page while later document ids still exist in the
/// bucket. The cursor belongs to the document-id terminal instead.
#[test]
fn included_cursor_inside_the_bucket_continues_a_single_property_time_range_query() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = build_single_property_trending_contract(false);
    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            platform_version,
        )
        .expect("apply contract");
    let document_type = contract.document_type_for_name("post").expect("post");

    // Three posts inside the [06:00, 12:00) bucket, none at its start.
    let bucket = 6 * HOUR_MS;
    let timestamps: [(u64, &str); 3] = [
        (6 * HOUR_MS + 600_000, "early"),
        (7 * HOUR_MS + 123_456, "mid"),
        (8 * HOUR_MS, "late"),
    ];
    for (created_at, hashtag) in timestamps {
        insert_post(&drive, &contract, created_at, hashtag, platform_version);
    }

    // The document-id terminal walks ids ascending; the cursor is the
    // mid-bucket post, so the expected continuation is every id at or
    // after it in that order.
    let mut ids: Vec<[u8; 32]> = timestamps
        .iter()
        .map(|(created_at, hashtag)| fixture_bytes(2, *created_at, hashtag))
        .collect();
    ids.sort();
    let cursor_id = fixture_bytes(2, 7 * HOUR_MS + 123_456, "mid");
    let cursor_position = ids.iter().position(|id| *id == cursor_id).expect("cursor");

    let mut query = build_created_at_query(
        &contract,
        document_type,
        bucket,
        None,
        created_at_resolution(document_type),
    );
    query.start_at = Some(cursor_id);
    query.start_at_included = true;

    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, None, platform_version)
        .expect("query with an in-bucket cursor");
    assert_eq!(
        result_ids(&results, document_type, platform_version),
        &ids[cursor_position..],
        "an included in-bucket cursor must return itself and every later id \
         in the bucket, in id order"
    );

    query.start_at_included = false;
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, None, platform_version)
        .expect("query with an excluded in-bucket cursor");
    assert_eq!(
        result_ids(&results, document_type, platform_version),
        &ids[cursor_position + 1..],
        "an excluded in-bucket cursor must return every id after it in the \
         bucket, in id order"
    );
}

/// The unique arm of the in-bucket cursor rule: a unique single-property
/// time-range index (`range == step` on `$createdAt`) stores the bucket's
/// sole reference AT the value tree, with no document-id terminal below
/// it. An included cursor must retain the bucket's one document; an
/// excluded cursor must produce an empty page — never an error, and never
/// the full page again.
#[test]
fn in_bucket_cursor_on_a_unique_single_property_time_range_index() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = build_single_property_trending_contract(true);
    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            platform_version,
        )
        .expect("apply contract");
    let document_type = contract.document_type_for_name("post").expect("post");

    // One document in the [06:00, 12:00) bucket — the most a unique
    // single-property grid admits.
    let created_at = 7 * HOUR_MS + 123_456;
    insert_post(&drive, &contract, created_at, "solo", platform_version);
    let document_id = fixture_bytes(2, created_at, "solo");
    let bucket = 6 * HOUR_MS;

    let mut query = build_created_at_query(
        &contract,
        document_type,
        bucket,
        None,
        created_at_resolution(document_type),
    );
    query.start_at = Some(document_id);
    query.start_at_included = true;

    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, None, platform_version)
        .expect("included cursor on the unique layout");
    assert_eq!(
        result_ids(&results, document_type, platform_version),
        vec![document_id],
        "an included cursor must retain the bucket's sole document"
    );

    query.start_at_included = false;
    let (results, _, _) = query
        .execute_raw_results_no_proof(&drive, None, None, platform_version)
        .expect("excluded cursor on the unique layout");
    assert!(
        results.is_empty(),
        "an excluded cursor must empty the page — the bucket holds exactly the cursor"
    );
}

/// One index can bucket only one field (a transform's source must be its
/// index's first property), so a query resolving two time ranges has no
/// servable shape and is refused rather than routed to whichever index
/// happens to cover the fields.
#[test]
fn two_resolved_time_ranges_are_rejected() {
    let contract = build_competing_index_trending_contract();
    let document_type = contract.document_type_for_name("post").expect("post");
    let query = build_created_at_query(&contract, document_type, 6 * HOUR_MS, Some("ibiza"), {
        let mut resolutions = created_at_resolution(document_type);
        let mut second = resolutions[0].clone();
        second.transform.source = "hashtag".to_string();
        resolutions.push(second);
        resolutions
    });
    let error = query
        .find_best_index(PlatformVersion::latest())
        .expect_err("two resolved time-range fields cannot be served");
    assert!(
        matches!(error, Error::Query(QuerySyntaxError::Unsupported(_))),
        "expected an Unsupported rejection, got {error:?}"
    );
}

/// Ordering by a property the bucketed index does not carry must fail
/// loudly. `byHashtagAndAuthor` covers the same where fields *and* the
/// ordering, so an unpinned search would take it and match the bucket
/// start against raw timestamps — a validly-proven empty result.
#[test]
fn resolved_time_range_query_ordering_off_the_bucketed_index_errors_rather_than_falling_back() {
    let contract = build_competing_index_trending_contract();
    let document_type = contract.document_type_for_name("post").expect("post");
    let query_value = Value::Map(vec![
        (
            Value::Text("where".to_string()),
            Value::Array(vec![
                Value::Array(vec![
                    Value::Text("hashtag".to_string()),
                    Value::Text("==".to_string()),
                    Value::Text("ibiza".to_string()),
                ]),
                Value::Array(vec![
                    Value::Text("$createdAt".to_string()),
                    Value::Text("==".to_string()),
                    Value::U64(6 * HOUR_MS),
                ]),
            ]),
        ),
        (
            Value::Text("orderBy".to_string()),
            Value::Array(vec![Value::Array(vec![
                Value::Text("author".to_string()),
                Value::Text("asc".to_string()),
            ])]),
        ),
    ]);
    let mut query = DriveDocumentQuery::from_value(
        query_value,
        &contract,
        document_type,
        &DriveConfig::default(),
        PlatformVersion::latest(),
    )
    .expect("build query");
    // Sanity: without the provenance this query has a covering index, so
    // the rejection below is the pinning talking and not a query that
    // nothing could serve.
    assert_eq!(
        query
            .find_best_index(PlatformVersion::latest())
            .expect("a plain index covers the where fields and the ordering")
            .name,
        "byHashtagAndAuthor"
    );

    query.resolved_time_ranges = created_at_resolution(document_type);
    let error = query
        .find_best_index(PlatformVersion::latest())
        .expect_err("no bucketed index covers the ordering");
    assert!(
        matches!(
            error,
            Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(_))
        ),
        "expected a no-covering-index rejection, got {error:?}"
    );
}

const DAY_SECONDS: u64 = 24 * HOUR_SECONDS;
const DAY_MS: u64 = 24 * HOUR_MS;

/// A `report` document type with a **unique**
/// `(timeRange($createdAt, range = step = 1 day), author)` index — one
/// report per author per calendar day.
///
/// `range == step` makes the windows a partition (overlap factor 1), which
/// is what lets uniqueness mean anything here, and `$createdAt` is
/// immutable so a document's bucket never moves. Both index properties are
/// required, so the terminator always takes the unique layout (the
/// reference stored AT `[0]`, with no per-document subtree).
fn build_unique_daily_report_contract() -> DataContract {
    let factory =
        DataContractFactory::new(PlatformVersion::latest().protocol_version).expect("factory");
    let index_map = vec![
        (
            Value::Text("name".to_string()),
            Value::Text("dailyReport".to_string()),
        ),
        (
            Value::Text("properties".to_string()),
            Value::Array(vec![
                platform_value!({"$createdAt": "asc"}),
                platform_value!({"author": "asc"}),
            ]),
        ),
        (Value::Text("unique".to_string()), Value::Bool(true)),
        (
            Value::Text("timeRange".to_string()),
            Value::Map(vec![
                (
                    Value::Text("on".to_string()),
                    Value::Text("$createdAt".to_string()),
                ),
                (Value::Text("range".to_string()), Value::U64(DAY_SECONDS)),
                (Value::Text("step".to_string()), Value::U64(DAY_SECONDS)),
            ]),
        ),
    ];

    let document_schema = platform_value!({
        "type": "object",
        "properties": {
            "author": {"type": "string", "maxLength": 63, "position": 0},
        },
        "required": ["author", "$createdAt"],
        "indices": Value::Array(vec![Value::Map(index_map)]),
        "additionalProperties": false,
    });
    let schemas = platform_value!({ "report": document_schema });
    let owner_id = Identifier::from([201u8; 32]);
    factory
        .create_with_value_config(owner_id, 0, schemas, None, None)
        .expect("create contract")
        .data_contract_owned()
}

/// Number of `report`s stored under the exact `(bucket, author)` tuple of
/// the unique index. Carries the `IN_TIME_RANGE` provenance for the same
/// reason [`count_in_bucket`] does.
fn count_reports_for(
    drive: &Drive,
    contract: &DataContract,
    bucket: u64,
    author: &str,
    platform_version: &PlatformVersion,
) -> usize {
    let document_type = contract.document_type_for_name("report").expect("report");
    let query_value = Value::Map(vec![(
        Value::Text("where".to_string()),
        Value::Array(vec![
            Value::Array(vec![
                Value::Text("$createdAt".to_string()),
                Value::Text("==".to_string()),
                Value::U64(bucket),
            ]),
            Value::Array(vec![
                Value::Text("author".to_string()),
                Value::Text("==".to_string()),
                Value::Text(author.to_string()),
            ]),
        ]),
    )]);
    let mut query = DriveDocumentQuery::from_value(
        query_value,
        contract,
        document_type,
        &DriveConfig::default(),
        PlatformVersion::latest(),
    )
    .expect("build query");
    query.resolved_time_ranges = created_at_resolution(document_type);
    query
        .execute_raw_results_no_proof(drive, None, None, platform_version)
        .expect("query")
        .0
        .len()
}

/// A suffix change under a **unique** bucketed index exercises the update
/// walker's unique terminator layout end to end: the old `(bucket, author)`
/// slot must be vacated and the new one occupied. Under the non-unique
/// layout the walker would delete a doc-id key that does not exist and
/// write the reference one level too deep, leaving the old entry in place
/// and the new one unfindable.
#[test]
fn unique_time_range_index_update_moves_the_entry_between_suffixes() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = build_unique_daily_report_contract();
    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            platform_version,
        )
        .expect("apply contract");

    let document_type = contract.document_type_for_name("report").expect("report");
    let index = document_type
        .indexes()
        .get("dailyReport")
        .expect("dailyReport index");
    assert!(index.unique, "the index under test must be unique");
    let transform = index
        .time_range
        .clone()
        .expect("dailyReport buckets $createdAt");
    assert_eq!(transform.overlap_factor(), 1);

    let created_at = 100 * DAY_MS + 3 * HOUR_MS;
    let bucket = *transform
        .containing_buckets(created_at)
        .first()
        .expect("a post-origin timestamp has exactly one bucket");
    assert_eq!(bucket, 100 * DAY_MS);

    let owner_bytes = fixture_bytes(1, created_at, "alice");
    let mut document = Document::V0(DocumentV0 {
        id: Identifier::from(fixture_bytes(2, created_at, "alice")),
        owner_id: Identifier::from(owner_bytes),
        properties: BTreeMap::from([("author".to_string(), Value::Text("alice".to_string()))]),
        created_at: Some(created_at),
        revision: Some(1),
        ..Default::default()
    });

    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((
                        &document,
                        StorageFlags::optional_default_as_cow(),
                    )),
                    owner_id: Some(owner_bytes),
                },
                contract: &contract,
                document_type,
            },
            false,
            BlockInfo::default(),
            true,
            None,
            platform_version,
            None,
        )
        .expect("add document");

    // The insert walker's unique terminator is readable through the
    // bucketed index.
    assert_eq!(
        count_reports_for(&drive, &contract, bucket, "alice", platform_version),
        1,
        "the inserted report must be found under its (bucket, author) tuple"
    );

    // Change the suffix. `$createdAt` is untouched — it cannot change —
    // so the bucket stays and only the author component of the tuple moves.
    document.set("author", Value::Text("bob".to_string()));
    document.set_revision(Some(2));
    drive
        .update_document_for_contract(
            &document,
            &contract,
            document_type,
            Some(owner_bytes),
            BlockInfo::default(),
            true,
            None,
            None,
            platform_version,
            None,
        )
        .expect("update document");

    assert_eq!(
        count_reports_for(&drive, &contract, bucket, "alice", platform_version),
        0,
        "the old (bucket, author) slot must be vacated by the update"
    );
    assert_eq!(
        count_reports_for(&drive, &contract, bucket, "bob", platform_version),
        1,
        "the new (bucket, author) slot must hold the document after the update"
    );

    // The vacated slot is genuinely free again: a second document may take
    // it, which only holds if the update actually removed the reference
    // rather than leaving a stale one behind.
    let second_owner = fixture_bytes(3, created_at, "alice");
    let second = Document::V0(DocumentV0 {
        id: Identifier::from(fixture_bytes(4, created_at, "alice")),
        owner_id: Identifier::from(second_owner),
        properties: BTreeMap::from([("author".to_string(), Value::Text("alice".to_string()))]),
        created_at: Some(created_at + HOUR_MS),
        revision: Some(1),
        ..Default::default()
    });
    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((
                        &second,
                        StorageFlags::optional_default_as_cow(),
                    )),
                    owner_id: Some(second_owner),
                },
                contract: &contract,
                document_type,
            },
            false,
            BlockInfo::default(),
            true,
            None,
            platform_version,
            None,
        )
        .expect("add second document into the vacated slot");
    assert_eq!(
        count_reports_for(&drive, &contract, bucket, "alice", platform_version),
        1
    );

    // Deleting the updated document clears its slot too — the delete
    // walker and the update walker must agree on where the reference is.
    drive
        .delete_document_for_contract(
            document.id(),
            &contract,
            "report",
            BlockInfo::default(),
            true,
            None,
            platform_version,
            None,
        )
        .expect("delete document");
    assert_eq!(
        count_reports_for(&drive, &contract, bucket, "bob", platform_version),
        0,
        "the updated document's slot must be empty after deletion"
    );
}

/// The mirror case: when every index covering the query buckets the
/// field, a raw query has nowhere to go and must be refused instead of
/// silently matching a timestamp against bucket starts.
#[test]
fn raw_query_on_a_doctype_whose_only_covering_index_is_bucketed_errors() {
    let contract = build_trending_contract();
    let document_type = contract.document_type_for_name("post").expect("post");
    let query = build_created_at_query(
        &contract,
        document_type,
        7 * HOUR_MS + 123_456,
        None,
        vec![],
    );
    let error = query
        .find_best_index(PlatformVersion::latest())
        .expect_err("a raw equality cannot be served by a bucketed index");
    assert!(
        matches!(
            error,
            Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(_))
        ),
        "expected a no-covering-index rejection, got {error:?}"
    );
}
/// The multi-grid contract: one timestamp, two grids, sibling subtrees.
/// A 6h/2h "trending" grid and a 24h/24h "daily" grid both bucket
/// `$createdAt`; each level is keyed by the grid-qualified storage key,
/// so the two coexist — including bucket starts that are numerically
/// identical across grids (every daily start is also a trending start).
fn build_two_grid_contract() -> DataContract {
    let factory =
        DataContractFactory::new(PlatformVersion::latest().protocol_version).expect("factory");
    let grid_index = |name: &str, range_seconds: u64, step_seconds: u64| {
        Value::Map(vec![
            (
                Value::Text("name".to_string()),
                Value::Text(name.to_string()),
            ),
            (
                Value::Text("properties".to_string()),
                Value::Array(vec![
                    platform_value!({"$createdAt": "asc"}),
                    platform_value!({"hashtag": "asc"}),
                ]),
            ),
            (
                Value::Text("timeRange".to_string()),
                Value::Map(vec![
                    (
                        Value::Text("on".to_string()),
                        Value::Text("$createdAt".to_string()),
                    ),
                    (Value::Text("range".to_string()), Value::U64(range_seconds)),
                    (Value::Text("step".to_string()), Value::U64(step_seconds)),
                ]),
            ),
            (
                Value::Text("countable".to_string()),
                Value::Text("countable".to_string()),
            ),
        ])
    };
    let document_schema = platform_value!({
        "type": "object",
        "properties": {
            "hashtag": {"type": "string", "maxLength": 63, "position": 0},
        },
        "required": ["hashtag", "$createdAt"],
        "indices": Value::Array(vec![
            grid_index("trending", 6 * HOUR_SECONDS, 2 * HOUR_SECONDS),
            grid_index("daily", 24 * HOUR_SECONDS, 24 * HOUR_SECONDS),
        ]),
        "additionalProperties": false,
    });
    let schemas = platform_value!({ "post": document_schema });
    factory
        .create_with_value_config(Identifier::from([202u8; 32]), 0, schemas, None, None)
        .expect("a contract may bucket one timestamp with several grids")
        .data_contract_owned()
}

/// The provenance of a resolution against one named grid of a
/// multi-grid document type.
fn grid_resolution(contract: &DataContract, index_name: &str) -> Vec<ResolvedTimeRange> {
    let transform = contract
        .document_type_for_name("post")
        .expect("post")
        .indexes()
        .get(index_name)
        .expect("the fixture declares this index")
        .time_range
        .clone()
        .expect("the index carries a transform");
    vec![ResolvedTimeRange { transform }]
}

/// Two grids over `$createdAt`: a document fans out into each grid's own
/// subtree, a resolution against one grid reads only that grid's bucket
/// — even when the two grids' bucket starts are the same number — and
/// deletion empties both. The bucket start chosen here (24h) is
/// deliberately a start on BOTH grids: without grid-qualified level keys
/// the two entry sets would interleave in one keyspace and the counts
/// below would be wrong in both directions.
#[test]
fn two_grids_over_one_timestamp_write_and_read_independently() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = build_two_grid_contract();

    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            platform_version,
        )
        .expect("apply contract");

    let document_type = contract.document_type_for_name("post").expect("post");

    // 25h10m: the daily grid buckets it at 24h; the trending grid at
    // [24h, 22h, 20h]. 24h is a bucket start on BOTH grids.
    let created_at = 25 * HOUR_MS + 10 * 60_000;
    let shared_bucket = 24 * HOUR_MS;

    let trending = grid_resolution(&contract, "trending");
    let daily = grid_resolution(&contract, "daily");
    assert_eq!(
        trending[0]
            .transform
            .containing_buckets(created_at)
            .first()
            .copied(),
        Some(shared_bucket)
    );
    assert_eq!(
        daily[0].transform.containing_buckets(created_at),
        vec![shared_bucket],
        "the same numeric start on both grids is the point of this fixture"
    );

    let owner_bytes = fixture_bytes(1, created_at, "ibiza");
    let document = Document::V0(DocumentV0 {
        id: Identifier::from(fixture_bytes(2, created_at, "ibiza")),
        owner_id: Identifier::from(owner_bytes),
        properties: BTreeMap::from([("hashtag".to_string(), Value::Text("ibiza".to_string()))]),
        created_at: Some(created_at),
        ..Default::default()
    });
    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((
                        &document,
                        StorageFlags::optional_default_as_cow(),
                    )),
                    owner_id: Some(owner_bytes),
                },
                contract: &contract,
                document_type,
            },
            false,
            BlockInfo::default(),
            true,
            None,
            platform_version,
            None,
        )
        .expect("add document");

    let count_for = |resolutions: &Vec<ResolvedTimeRange>, bucket: u64| -> usize {
        let query = build_created_at_query(
            &contract,
            document_type,
            bucket,
            Some("ibiza"),
            resolutions.clone(),
        );
        query
            .execute_raw_results_no_proof(&drive, None, None, platform_version)
            .expect("query")
            .0
            .len()
    };

    // Selection pins to the resolved grid's index.
    let trending_query = build_created_at_query(
        &contract,
        document_type,
        shared_bucket,
        Some("ibiza"),
        trending.clone(),
    );
    assert_eq!(
        trending_query
            .find_best_index(platform_version)
            .expect("the trending grid's index serves its own resolution")
            .name,
        "trending"
    );
    let daily_query = build_created_at_query(
        &contract,
        document_type,
        shared_bucket,
        Some("ibiza"),
        daily.clone(),
    );
    assert_eq!(
        daily_query
            .find_best_index(platform_version)
            .expect("the daily grid's index serves its own resolution")
            .name,
        "daily"
    );

    // Each grid's subtree holds the document under the shared start, and
    // the trending grid additionally holds it under its two older
    // overlapping starts — which the daily grid must NOT see.
    assert_eq!(count_for(&trending, shared_bucket), 1);
    assert_eq!(count_for(&daily, shared_bucket), 1);
    assert_eq!(count_for(&trending, 22 * HOUR_MS), 1);
    assert_eq!(
        count_for(&daily, 22 * HOUR_MS),
        0,
        "22h is a trending fan-out entry only; leaking it into the daily \
         grid would mean the levels share a keyspace again"
    );

    // Deletion empties both grids' subtrees.
    drive
        .delete_document_for_contract(
            document.id(),
            &contract,
            "post",
            BlockInfo::default(),
            true,
            None,
            platform_version,
            None,
        )
        .expect("delete document");
    assert_eq!(count_for(&trending, shared_bucket), 0);
    assert_eq!(count_for(&trending, 22 * HOUR_MS), 0);
    assert_eq!(count_for(&daily, shared_bucket), 0);
}

/// Resolution over a multi-grid field: the bare selector is ambiguous
/// and refused; a grid spec picks exactly the named grid; a spec no
/// index declares is refused. This is the query-language half of the
/// storage fork the previous test pins.
#[test]
fn multi_grid_resolution_requires_and_honors_a_grid_spec() {
    let contract = build_two_grid_contract();
    let document_type = contract.document_type_for_name("post").expect("post");
    let now_ms = 25 * HOUR_MS;

    let error = resolve_time_range_bucket_clause(
        "$createdAt",
        TimeRangeSelector::Newest,
        None,
        document_type,
        now_ms,
    )
    .expect_err("two grids on the field make the bare selector ambiguous");
    assert!(
        matches!(error, Error::Query(QuerySyntaxError::Unsupported(_))),
        "expected the ambiguity rejection, got {error:?}"
    );

    let (clause, resolution) = resolve_time_range_bucket_clause(
        "$createdAt",
        TimeRangeSelector::Newest,
        Some(TimeRangeGridSpec {
            range_seconds: 24 * HOUR_SECONDS,
            step_seconds: 24 * HOUR_SECONDS,
            phase_seconds: 0,
        }),
        document_type,
        now_ms,
    )
    .expect("naming the daily grid resolves against it");
    assert_eq!(clause.value, Value::U64(24 * HOUR_MS));
    assert_eq!(resolution.transform.range_seconds, 24 * HOUR_SECONDS);

    let (clause, resolution) = resolve_time_range_bucket_clause(
        "$createdAt",
        TimeRangeSelector::Newest,
        Some(TimeRangeGridSpec {
            range_seconds: 6 * HOUR_SECONDS,
            step_seconds: 2 * HOUR_SECONDS,
            phase_seconds: 0,
        }),
        document_type,
        now_ms,
    )
    .expect("naming the trending grid resolves against it");
    assert_eq!(
        clause.value,
        Value::U64(24 * HOUR_MS),
        "at 25h both grids' newest start is 24h — same number, different \
         subtree, which is exactly why provenance carries the grid"
    );
    assert_eq!(resolution.transform.step_seconds, 2 * HOUR_SECONDS);

    let error = resolve_time_range_bucket_clause(
        "$createdAt",
        TimeRangeSelector::Newest,
        Some(TimeRangeGridSpec {
            range_seconds: 12 * HOUR_SECONDS,
            step_seconds: 12 * HOUR_SECONDS,
            phase_seconds: 0,
        }),
        document_type,
        now_ms,
    )
    .expect_err("a grid no index declares must be refused");
    assert!(
        matches!(error, Error::Query(QuerySyntaxError::Unsupported(_))),
        "expected the unknown-grid rejection, got {error:?}"
    );
}

/// The multiple-`In` execution lowering picks its index directly
/// (without `find_best_index`), so it must run the shared
/// resolved-source shape guard itself: a direct caller pairing
/// fabricated provenance with an `In` clause ON the bucketed source
/// would otherwise have its raw `In` values serialized as bucket
/// keys — a validly-proven answer over arbitrary buckets.
#[test]
fn multiple_in_route_refuses_an_in_clause_on_the_bucketed_source() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = build_trending_contract();
    let document_type = contract.document_type_for_name("post").expect("post");

    let query_value = Value::Map(vec![(
        Value::Text("where".to_string()),
        Value::Array(vec![
            Value::Array(vec![
                Value::Text("$createdAt".to_string()),
                Value::Text("in".to_string()),
                Value::Array(vec![Value::U64(2 * HOUR_MS), Value::U64(4 * HOUR_MS)]),
            ]),
            Value::Array(vec![
                Value::Text("hashtag".to_string()),
                Value::Text("in".to_string()),
                Value::Array(vec![
                    Value::Text("dash".to_string()),
                    Value::Text("evo".to_string()),
                ]),
            ]),
        ]),
    )]);
    let mut query = DriveDocumentQuery::from_value(
        query_value,
        &contract,
        document_type,
        &DriveConfig::default(),
        platform_version,
    )
    .expect("two In clauses are a valid protocol-version-14 query shape");
    query.resolved_time_ranges = created_at_resolution(document_type);

    let error = query
        .execute_raw_results_no_proof(&drive, None, None, platform_version)
        .expect_err("an In on the bucketed source must not reach bucket keys");
    assert!(
        matches!(error, Error::Query(QuerySyntaxError::Unsupported(_))),
        "expected the source-shape rejection, got {error:?}"
    );
}

/// The update walker's bucketed branch materializes ranking-chain
/// continuations exactly as the insert walker does — the consensus
/// property behind the chain-inversion dispatch it shares with the
/// non-bucketed branch. An update that moves a document to a NEW group
/// value inside its (unchanged) buckets materializes that group's value
/// tree and its chain continuation through the update path alone; a
/// wrapped (zero-contributing) continuation there would pin the new
/// group's whole-subtree count at zero, and the per-window ranking would
/// then disagree between a document that arrived by insert and one that
/// arrived by update — a layout divergence, not just a wrong answer.
#[test]
fn ranked_chain_below_bucket_update_materializes_like_insert() {
    use crate::query::drive_document_ranked_query::index_picker::resolve_ranked_query_for_mode;
    use crate::query::drive_document_ranked_query::PrefixPin;
    use crate::query::{DocumentRankedMode, RankedAxis, RankedEntryValue};

    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));

    let factory =
        DataContractFactory::new(PlatformVersion::latest().protocol_version).expect("factory");
    let index_map = vec![
        (
            Value::Text("name".to_string()),
            Value::Text("trendingChain".to_string()),
        ),
        (
            Value::Text("properties".to_string()),
            Value::Array(vec![
                platform_value!({"$createdAt": "asc"}),
                platform_value!({"category": "asc"}),
                platform_value!({"name": "asc"}),
            ]),
        ),
        (
            Value::Text("timeRange".to_string()),
            Value::Map(vec![
                (
                    Value::Text("on".to_string()),
                    Value::Text("$createdAt".to_string()),
                ),
                (
                    Value::Text("range".to_string()),
                    Value::U64(6 * HOUR_SECONDS),
                ),
                (
                    Value::Text("step".to_string()),
                    Value::U64(2 * HOUR_SECONDS),
                ),
            ]),
        ),
        (
            Value::Text("countable".to_string()),
            Value::Text("countable".to_string()),
        ),
        (Value::Text("rangeCountable".to_string()), Value::Bool(true)),
        (
            Value::Text("rankedCountable".to_string()),
            Value::Map(vec![(
                Value::Text("at".to_string()),
                Value::Array(vec![Value::Text("category".to_string())]),
            )]),
        ),
    ];
    let document_schema = platform_value!({
        "type": "object",
        "properties": {
            "category": {"type": "string", "maxLength": 61, "position": 0},
            "name": {"type": "string", "maxLength": 61, "position": 1},
        },
        "required": ["category", "name", "$createdAt"],
        "indices": Value::Array(vec![Value::Map(index_map)]),
        "additionalProperties": false,
    });
    let schemas = platform_value!({ "post": document_schema });
    let contract = factory
        .create_with_value_config(Identifier::from([202u8; 32]), 0, schemas, None, None)
        .expect("a ranked at-chain below a bucketed level registers")
        .data_contract_owned();

    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            platform_version,
        )
        .expect("apply contract");

    let document_type = contract.document_type_for_name("post").expect("post");
    let transform = document_type
        .indexes()
        .get("trendingChain")
        .expect("trendingChain index")
        .time_range
        .clone()
        .expect("time range transform");

    let created_at = 7 * HOUR_MS + 123_456;
    let buckets = transform.containing_buckets(created_at);
    assert_eq!(buckets, vec![6 * HOUR_MS, 4 * HOUR_MS, 2 * HOUR_MS]);

    let owner_bytes = fixture_bytes(3, created_at, "chain");
    let mut document = Document::V0(DocumentV0 {
        id: Identifier::from(fixture_bytes(4, created_at, "chain")),
        owner_id: Identifier::from(owner_bytes),
        properties: BTreeMap::from([
            ("category".to_string(), Value::Text("a".to_string())),
            ("name".to_string(), Value::Text("x".to_string())),
        ]),
        created_at: Some(created_at),
        revision: Some(1),
        ..Default::default()
    });

    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((
                        &document,
                        StorageFlags::optional_default_as_cow(),
                    )),
                    owner_id: Some(owner_bytes),
                },
                contract: &contract,
                document_type,
            },
            false,
            BlockInfo::default(),
            true,
            None,
            platform_version,
            None,
        )
        .expect("add document");

    // The update materializes category "b" — a value tree that does not
    // exist yet — inside every containing bucket, together with the
    // "name" chain continuation below it. This is the exact tree set the
    // buggy dispatch would have zero-wrapped.
    document.set("category", Value::Text("b".to_string()));
    document.set_revision(Some(2));
    drive
        .update_document_for_contract(
            &document,
            &contract,
            document_type,
            Some(owner_bytes),
            BlockInfo::default(),
            true,
            None,
            None,
            platform_version,
            None,
        )
        .expect("update document to a new group value");

    for bucket in &buckets {
        let mode = DocumentRankedMode {
            axis: RankedAxis::Count,
            descending: true,
            k: 10,
            offset: 0,
            group_by_property: "category".to_string(),
            aggregate_field: String::new(),
            prefix_pins: vec![PrefixPin {
                field: "$createdAt".to_string(),
                values: vec![Value::U64(*bucket)],
            }],
        };
        let ranked_query = resolve_ranked_query_for_mode(
            contract.id().to_buffer(),
            document_type,
            "post".to_string(),
            document_type.indexes(),
            &mode,
            &created_at_resolution(document_type),
            platform_version,
        )
        .expect("the bucketed at-chain index covers the pinned request");
        let page = ranked_query
            .execute_top_k_no_proof(&drive, None, platform_version)
            .expect("the per-window group ranking reads");
        assert_eq!(
            page.entries.len(),
            1,
            "bucket {bucket}: the stale group must be pruned and the new one present"
        );
        assert_eq!(
            page.entries[0].key,
            b"b".to_vec(),
            "bucket {bucket}: the group key is the update's new category"
        );
        assert_eq!(
            page.entries[0].value,
            RankedEntryValue::Count(1),
            "bucket {bucket}: an update-materialized chain continuation must count \
             exactly like an insert-created one — zero here means the continuation \
             was zero-wrapped on the update path"
        );
    }
}

/// The TTL lifecycle end to end — `book/src/drive/time-range-ttl.md`
/// exercised through the real walkers:
///
/// * a bucket-creating write drops buckets behind the horizon (and only
///   those: a bucket starting exactly AT the horizon survives);
/// * the per-write cap amortizes catch-up instead of dumping a backlog
///   on one writer;
/// * deleting and updating a document whose buckets were dropped
///   succeeds — the removal side skips exactly the dropped buckets, and
///   an update never resurrects one;
/// * ranked per-window leaderboards below the bucket ride along: live
///   windows keep serving, dropped windows take their secondaries with
///   them (the recursive-delete placeholder sweeps indexed axes).
#[test]
fn ttl_drops_expired_buckets_and_walkers_skip_them() {
    use crate::drive::document::paths::contract_document_type_path_vec;
    use crate::fees::op::LowLevelDriveOperation;
    use crate::util::grove_operations::DirectQueryType;
    use dpp::data_contract::document_type::DocumentPropertyType;
    use grovedb_path::SubtreePath;

    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));

    // Tumbling 2h windows, TTL 4h, ranked hashtags per window.
    let factory =
        DataContractFactory::new(PlatformVersion::latest().protocol_version).expect("factory");
    let index_map = vec![
        (
            Value::Text("name".to_string()),
            Value::Text("trendingTtl".to_string()),
        ),
        (
            Value::Text("properties".to_string()),
            Value::Array(vec![
                platform_value!({"$createdAt": "asc"}),
                platform_value!({"hashtag": "asc"}),
            ]),
        ),
        (
            Value::Text("timeRange".to_string()),
            Value::Map(vec![
                (
                    Value::Text("on".to_string()),
                    Value::Text("$createdAt".to_string()),
                ),
                (
                    Value::Text("range".to_string()),
                    Value::U64(2 * HOUR_SECONDS),
                ),
                (
                    Value::Text("step".to_string()),
                    Value::U64(2 * HOUR_SECONDS),
                ),
                (Value::Text("ttl".to_string()), Value::U64(4 * HOUR_SECONDS)),
            ]),
        ),
        (
            Value::Text("countable".to_string()),
            Value::Text("countable".to_string()),
        ),
        (Value::Text("rangeCountable".to_string()), Value::Bool(true)),
        (
            Value::Text("rankedCountable".to_string()),
            Value::Bool(true),
        ),
    ];
    let document_schema = platform_value!({
        "type": "object",
        "properties": {
            "hashtag": {"type": "string", "maxLength": 61, "position": 0},
        },
        "required": ["hashtag", "$createdAt"],
        "indices": Value::Array(vec![Value::Map(index_map)]),
        "additionalProperties": false,
    });
    let schemas = platform_value!({ "post": document_schema });
    let contract = factory
        .create_with_value_config(Identifier::from([203u8; 32]), 0, schemas, None, None)
        .expect("a TTL'd ranked windowed index registers")
        .data_contract_owned();

    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            platform_version,
        )
        .expect("apply contract");

    let document_type = contract.document_type_for_name("post").expect("post");
    let transform = document_type
        .indexes()
        .get("trendingTtl")
        .expect("trendingTtl index")
        .time_range
        .clone()
        .expect("transform");
    assert_eq!(transform.ttl_seconds, Some(4 * HOUR_SECONDS));

    let mut level_path = contract_document_type_path_vec(contract.id_ref().as_bytes(), "post");
    level_path.push(transform.storage_key("$createdAt").into_bytes());

    let bucket_exists = |start_ms: u64| -> bool {
        let path_refs: Vec<&[u8]> = level_path
            .iter()
            .map(|segment| segment.as_slice())
            .collect();
        let key = DocumentPropertyType::encode_date_timestamp(start_ms);
        let mut ops: Vec<LowLevelDriveOperation> = vec![];
        drive
            .grove_has_raw(
                SubtreePath::from(path_refs.as_slice()),
                key.as_slice(),
                DirectQueryType::StatefulDirectQuery,
                None,
                &mut ops,
                &platform_version.drive,
            )
            .expect("existence check")
    };

    let insert_at = |created_at: u64, tag: &str| -> Document {
        let owner_bytes = fixture_bytes(5, created_at, tag);
        let document = Document::V0(DocumentV0 {
            id: Identifier::from(fixture_bytes(6, created_at, tag)),
            owner_id: Identifier::from(owner_bytes),
            properties: BTreeMap::from([("hashtag".to_string(), Value::Text(tag.to_string()))]),
            created_at: Some(created_at),
            revision: Some(1),
            ..Default::default()
        });
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((
                            &document,
                            StorageFlags::optional_default_as_cow(),
                        )),
                        owner_id: Some(owner_bytes),
                    },
                    contract: &contract,
                    document_type,
                },
                false,
                BlockInfo {
                    time_ms: created_at,
                    ..Default::default()
                },
                true,
                None,
                platform_version,
                None,
            )
            .expect("add document");
        document
    };

    let h = HOUR_MS;
    // Anchor away from the epoch so horizons never underflow.
    let t0 = 1_000 * h;

    let doc_a = insert_at(t0 + 10 * MINUTE_MS_TTL, "alpha"); // bucket t0
    let mut doc_bravo = insert_at(t0 + 2 * h + 10 * MINUTE_MS_TTL, "bravo"); // bucket t0+2h
    assert!(bucket_exists(t0), "nothing is expired yet");

    // Writing at exactly t0+6h (bucket t0+6h) puts the horizon at
    // exactly t0+2h: bucket t0 (start < horizon) is dropped, bucket
    // t0+2h (start == horizon) survives — expiry is strictly-below.
    insert_at(t0 + 6 * h, "charlie");
    assert!(
        !bucket_exists(t0),
        "the bucket behind the horizon must be dropped by the bucket-creating write"
    );
    assert!(
        bucket_exists(t0 + 2 * h),
        "a bucket starting exactly at the horizon is not expired"
    );
    assert!(bucket_exists(t0 + 6 * h));

    // Deleting a document whose buckets were dropped must succeed: the
    // removal side skips exactly the dropped buckets.
    drive
        .delete_document_for_contract(
            doc_a.id(),
            &contract,
            "post",
            BlockInfo {
                time_ms: t0 + 6 * h + 20 * MINUTE_MS_TTL,
                ..Default::default()
            },
            true,
            None,
            platform_version,
            None,
        )
        .expect("deleting a document whose windows were dropped succeeds");

    // Updating a document whose windows have all expired succeeds and
    // never resurrects a dropped bucket. `bravo`'s bucket (t0+2h) is
    // still standing here; move time far enough that it has been dropped
    // first, then update it.
    insert_at(t0 + 10 * h + 10 * MINUTE_MS_TTL, "delta"); // horizon now t0+6h
    assert!(
        !bucket_exists(t0 + 2 * h),
        "catch-up cleanup drops the next expired bucket"
    );

    // Updating the document whose bucket was dropped exercises the update
    // walker's expired-window paths for real: the new entry keys filter to
    // nothing (its windows are all expired), the old-entry loop skips the
    // dropped bucket, and — the invariant that protects the flat-drop
    // path-reuse contract — the dropped bucket is NOT resurrected.
    doc_bravo.set("hashtag", Value::Text("bravo2".to_string()));
    doc_bravo.set_revision(Some(2));
    drive
        .update_document_for_contract(
            &doc_bravo,
            &contract,
            document_type,
            None,
            BlockInfo {
                time_ms: t0 + 10 * h + 15 * MINUTE_MS_TTL,
                ..Default::default()
            },
            true,
            None,
            None,
            platform_version,
            None,
        )
        .expect("updating a document whose windows were all dropped succeeds");
    assert!(
        !bucket_exists(t0 + 2 * h),
        "the update must not resurrect the dropped bucket"
    );

    let mut doc_b = insert_at(t0 + 10 * h + 20 * MINUTE_MS_TTL, "echo");
    // Update a LIVE document normally (control), then delete it — the
    // full mutable lifecycle stays intact under a TTL'd index.
    doc_b.set("hashtag", Value::Text("echo2".to_string()));
    doc_b.set_revision(Some(2));
    drive
        .update_document_for_contract(
            &doc_b,
            &contract,
            document_type,
            None,
            BlockInfo {
                time_ms: t0 + 10 * h + 30 * MINUTE_MS_TTL,
                ..Default::default()
            },
            true,
            None,
            None,
            platform_version,
            None,
        )
        .expect("updating a live document under a TTL'd index succeeds");

    // The live window's per-window leaderboard serves after all of the
    // above: ranked entries for bucket t0+10h are (delta 1, echo2 1) —
    // and echo (the pre-update suffix) is gone.
    {
        use crate::query::drive_document_ranked_query::index_picker::resolve_ranked_query_for_mode;
        use crate::query::drive_document_ranked_query::PrefixPin;
        use crate::query::{DocumentRankedMode, RankedAxis};
        let mode = DocumentRankedMode {
            axis: RankedAxis::Count,
            descending: true,
            k: 10,
            offset: 0,
            group_by_property: "hashtag".to_string(),
            aggregate_field: String::new(),
            prefix_pins: vec![PrefixPin {
                field: "$createdAt".to_string(),
                values: vec![Value::U64(t0 + 10 * h)],
            }],
        };
        let ranked_query = resolve_ranked_query_for_mode(
            contract.id().to_buffer(),
            document_type,
            "post".to_string(),
            document_type.indexes(),
            &mode,
            &created_at_resolution(document_type),
            platform_version,
        )
        .expect("the TTL'd ranked index covers the pinned request");
        let page = ranked_query
            .execute_top_k_no_proof(&drive, None, platform_version)
            .expect("the live window's leaderboard reads");
        let keys: Vec<&[u8]> = page.entries.iter().map(|e| e.key.as_slice()).collect();
        assert!(keys.contains(&b"delta".as_slice()));
        assert!(keys.contains(&b"echo2".as_slice()));
        assert!(!keys.contains(&b"echo".as_slice()));
    }
}

/// One minute in milliseconds, for the TTL lifecycle test's offsets.
const MINUTE_MS_TTL: u64 = 60_000;

/// The TTL grammar rejections that need contract-level context: the
/// SystemLimits cap, and two indexes sharing a grid with different TTLs
/// (one storage level cannot have two lifecycles). The structural lower
/// bound (`ttl >= range`) is covered at the `Index` parse level.
#[test]
fn ttl_contract_level_rejections() {
    let factory =
        DataContractFactory::new(PlatformVersion::latest().protocol_version).expect("factory");
    let time_range_with_ttl = |ttl: u64| {
        Value::Map(vec![
            (
                Value::Text("on".to_string()),
                Value::Text("$createdAt".to_string()),
            ),
            (Value::Text("range".to_string()), Value::U64(HOUR_SECONDS)),
            (Value::Text("step".to_string()), Value::U64(HOUR_SECONDS)),
            (Value::Text("ttl".to_string()), Value::U64(ttl)),
        ])
    };
    let schema_with_indices = |indices: Value| {
        platform_value!({
            "post": {
                "type": "object",
                "properties": {
                    "hashtag": {"type": "string", "maxLength": 61, "position": 0},
                },
                "required": ["hashtag", "$createdAt"],
                "indices": indices,
                "additionalProperties": false,
            }
        })
    };

    // Over the one-week cap.
    let over_cap = schema_with_indices(Value::Array(vec![Value::Map(vec![
        (
            Value::Text("name".to_string()),
            Value::Text("overCap".to_string()),
        ),
        (
            Value::Text("properties".to_string()),
            Value::Array(vec![
                platform_value!({"$createdAt": "asc"}),
                platform_value!({"hashtag": "asc"}),
            ]),
        ),
        (
            Value::Text("timeRange".to_string()),
            time_range_with_ttl(604_800 + 1),
        ),
        (
            Value::Text("countable".to_string()),
            Value::Text("countable".to_string()),
        ),
    ])]));
    let error = factory
        .create_with_value_config(Identifier::from([204u8; 32]), 0, over_cap, None, None)
        .expect_err("a TTL over the cap must be refused");
    assert!(
        error.to_string().contains("exceeds the maximum"),
        "expected the cap rejection, got: {error}"
    );

    // Same grid, different TTLs.
    let index = |name: &str, ttl: u64| {
        Value::Map(vec![
            (
                Value::Text("name".to_string()),
                Value::Text(name.to_string()),
            ),
            (
                Value::Text("properties".to_string()),
                Value::Array(vec![
                    platform_value!({"$createdAt": "asc"}),
                    platform_value!({"hashtag": "asc"}),
                ]),
            ),
            (
                Value::Text("timeRange".to_string()),
                time_range_with_ttl(ttl),
            ),
            (
                Value::Text("countable".to_string()),
                Value::Text("countable".to_string()),
            ),
        ])
    };
    let conflicting = schema_with_indices(Value::Array(vec![
        index("gridA", 3 * HOUR_SECONDS),
        index("gridB", 4 * HOUR_SECONDS),
    ]));
    let error = factory
        .create_with_value_config(Identifier::from([205u8; 32]), 0, conflicting, None, None)
        .expect_err("one grid cannot carry two lifecycles");
    assert!(
        error.to_string().contains("two lifecycles"),
        "expected the shared-grid TTL conflict rejection, got: {error}"
    );
}

/// Budgeted drainage across writes: a bucket whose drop-operation count
/// exceeds one write's budget stands PARTIALLY drained until later writes
/// finish it — groups leave deepest-first in key order — and document
/// removal keeps working through every intermediate state, at full-path
/// granularity: a doc whose group the drain already took deletes as a
/// clean skip, one whose group still stands deletes normally.
#[test]
fn ttl_partial_drain_resumes_across_writes_and_removals_stay_exact() {
    use crate::drive::document::paths::contract_document_type_path_vec;
    use crate::fees::op::LowLevelDriveOperation;
    use crate::util::grove_operations::DirectQueryType;
    use dpp::data_contract::document_type::DocumentPropertyType;
    use grovedb_path::SubtreePath;

    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));

    // Same shape as the lifecycle test: tumbling 2h windows, TTL 4h,
    // ranked hashtags per window.
    let factory =
        DataContractFactory::new(PlatformVersion::latest().protocol_version).expect("factory");
    let index_map = vec![
        (
            Value::Text("name".to_string()),
            Value::Text("trendingTtl".to_string()),
        ),
        (
            Value::Text("properties".to_string()),
            Value::Array(vec![
                platform_value!({"$createdAt": "asc"}),
                platform_value!({"hashtag": "asc"}),
            ]),
        ),
        (
            Value::Text("timeRange".to_string()),
            Value::Map(vec![
                (
                    Value::Text("on".to_string()),
                    Value::Text("$createdAt".to_string()),
                ),
                (
                    Value::Text("range".to_string()),
                    Value::U64(2 * HOUR_SECONDS),
                ),
                (
                    Value::Text("step".to_string()),
                    Value::U64(2 * HOUR_SECONDS),
                ),
                (Value::Text("ttl".to_string()), Value::U64(4 * HOUR_SECONDS)),
            ]),
        ),
        (
            Value::Text("countable".to_string()),
            Value::Text("countable".to_string()),
        ),
        (Value::Text("rangeCountable".to_string()), Value::Bool(true)),
        (
            Value::Text("rankedCountable".to_string()),
            Value::Bool(true),
        ),
    ];
    let document_schema = platform_value!({
        "type": "object",
        "properties": {
            "hashtag": {"type": "string", "maxLength": 61, "position": 0},
        },
        "required": ["hashtag", "$createdAt"],
        "indices": Value::Array(vec![Value::Map(index_map)]),
        "additionalProperties": false,
    });
    let schemas = platform_value!({ "post": document_schema });
    let contract = factory
        .create_with_value_config(Identifier::from([206u8; 32]), 0, schemas, None, None)
        .expect("contract registers")
        .data_contract_owned();
    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            platform_version,
        )
        .expect("apply contract");

    let document_type = contract.document_type_for_name("post").expect("post");
    let transform = document_type
        .indexes()
        .get("trendingTtl")
        .expect("index")
        .time_range
        .clone()
        .expect("transform");

    let mut level_path = contract_document_type_path_vec(contract.id_ref().as_bytes(), "post");
    level_path.push(transform.storage_key("$createdAt").into_bytes());

    let path_exists = |segments: &[Vec<u8>]| -> bool {
        let (key, parents) = segments.split_last().expect("non-empty path");
        let parent_refs: Vec<&[u8]> = parents.iter().map(|segment| segment.as_slice()).collect();
        let mut ops: Vec<LowLevelDriveOperation> = vec![];
        drive
            .grove_has_raw(
                SubtreePath::from(parent_refs.as_slice()),
                key.as_slice(),
                DirectQueryType::StatefulDirectQuery,
                None,
                &mut ops,
                &platform_version.drive,
            )
            .expect("existence check")
    };

    let insert_at = |created_at: u64, tag: &str| -> Document {
        let owner_bytes = fixture_bytes(7, created_at, tag);
        let document = Document::V0(DocumentV0 {
            id: Identifier::from(fixture_bytes(8, created_at, tag)),
            owner_id: Identifier::from(owner_bytes),
            properties: BTreeMap::from([("hashtag".to_string(), Value::Text(tag.to_string()))]),
            created_at: Some(created_at),
            revision: Some(1),
            ..Default::default()
        });
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((
                            &document,
                            StorageFlags::optional_default_as_cow(),
                        )),
                        owner_id: Some(owner_bytes),
                    },
                    contract: &contract,
                    document_type,
                },
                false,
                BlockInfo {
                    time_ms: created_at,
                    ..Default::default()
                },
                true,
                None,
                platform_version,
                None,
            )
            .expect("add document");
        document
    };

    let h = HOUR_MS;
    let t0 = 2_000 * h;
    let old_bucket_key = DocumentPropertyType::encode_date_timestamp(t0);

    // Fourteen groups in the doomed bucket: full drainage costs
    // 14 × ([0] drop + value-tree delete) + property-name drop + bucket
    // drop = 30 operations — enough that the first write and the two
    // deletes below (every write drains, deletes included) each spend a
    // full 8-op budget without finishing it.
    let docs: Vec<Document> = (1..=14)
        .map(|i| insert_at(t0 + i * MINUTE_MS_TTL, &format!("g{i:02}")))
        .collect();

    // First write past the horizon: budget 8 drains groups g01..g04 (2 ops
    // each) and stops — the bucket stands, partially drained, with
    // g05..g14 and the property-name tree intact.
    insert_at(t0 + 6 * h, "w1");
    let bucket_path = {
        let mut path = level_path.clone();
        path.push(old_bucket_key.clone());
        path
    };
    assert!(
        path_exists(&bucket_path),
        "the bucket stands after the budget ran out"
    );
    let group_path = |tag: &str| -> Vec<Vec<u8>> {
        let mut path = bucket_path.clone();
        path.push(b"hashtag".to_vec());
        path.push(tag.as_bytes().to_vec());
        path
    };
    for gone in ["g01", "g02", "g03", "g04"] {
        assert!(
            !path_exists(&group_path(gone)),
            "group {gone} drains in the first write"
        );
    }
    assert!(
        path_exists(&group_path("g05")),
        "the budget stops before g05"
    );
    assert!(path_exists(&group_path("g14")), "g14 stands untouched");

    // A document whose group still stands deletes normally; one whose
    // group the drain took deletes as a clean skip. Both under the
    // standing, partially drained bucket — and each delete's own drain
    // spends another budget (g06..g09, then g10..g13), so the standing
    // group must go first.
    for (doc, label) in [(&docs[4], "standing group"), (&docs[0], "drained group")] {
        drive
            .delete_document_for_contract(
                doc.id(),
                &contract,
                "post",
                BlockInfo {
                    time_ms: t0 + 6 * h + 10 * MINUTE_MS_TTL,
                    ..Default::default()
                },
                true,
                None,
                platform_version,
                None,
            )
            .unwrap_or_else(|e| panic!("deleting a doc from a {label} must succeed: {e:?}"));
    }

    // The first delete's up-tree pruning takes the emptied g05 chain and
    // its drain g06..g09; the second delete's drain takes g10..g13. The
    // property-name tree still holds g14, so the bucket stands, and only
    // the next write's drain can finish it.
    assert!(
        !path_exists(&group_path("g05")),
        "deleting g05's only document prunes the group"
    );
    assert!(
        !path_exists(&group_path("g13")),
        "the deletes' drains reach g13"
    );
    assert!(path_exists(&group_path("g14")), "g14 still stands");
    assert!(
        path_exists(&bucket_path),
        "the bucket stands until drainage resumes"
    );
    insert_at(t0 + 6 * h + 20 * MINUTE_MS_TTL, "w2");
    assert!(
        !path_exists(&bucket_path),
        "drainage completes across writes"
    );
}

/// Shared builder for the TTL parent-layout matrix: a 2h/2h grid with
/// `ttl: 4h` over `[$createdAt, hashtag]`, with the aggregate keywords
/// supplied per case, plus an integer `amount` property for the sum-
/// bearing layouts.
fn build_ttl_contract_with_index_keys(
    seed: u8,
    extra_index_keys: Vec<(Value, Value)>,
) -> DataContract {
    build_time_range_contract_with_index_keys(seed, Some(4 * HOUR_SECONDS), extra_index_keys)
}

/// Same contract shape with the TTL declaration as the only degree of
/// freedom, so a TTL'd index and its standing twin are byte-for-byte
/// comparable in fee tests.
fn build_time_range_contract_with_index_keys(
    seed: u8,
    ttl_seconds: Option<u64>,
    extra_index_keys: Vec<(Value, Value)>,
) -> DataContract {
    let factory =
        DataContractFactory::new(PlatformVersion::latest().protocol_version).expect("factory");
    let mut index_map = vec![
        (
            Value::Text("name".to_string()),
            Value::Text("trendingTtl".to_string()),
        ),
        (
            Value::Text("properties".to_string()),
            Value::Array(vec![
                platform_value!({"$createdAt": "asc"}),
                platform_value!({"hashtag": "asc"}),
            ]),
        ),
        (
            Value::Text("timeRange".to_string()),
            Value::Map(vec![
                (
                    Value::Text("on".to_string()),
                    Value::Text("$createdAt".to_string()),
                ),
                (
                    Value::Text("range".to_string()),
                    Value::U64(2 * HOUR_SECONDS),
                ),
                (
                    Value::Text("step".to_string()),
                    Value::U64(2 * HOUR_SECONDS),
                ),
            ]),
        ),
    ];
    if let Some(ttl) = ttl_seconds {
        let Some((_, Value::Map(time_range_map))) = index_map.last_mut() else {
            panic!("timeRange map is the last base index key");
        };
        time_range_map.push((Value::Text("ttl".to_string()), Value::U64(ttl)));
    }
    index_map.extend(extra_index_keys);
    let document_schema = platform_value!({
        "type": "object",
        "properties": {
            // 59: the Avg axis's 16-byte sort key tightens the ranked
            // group-key cap below the Count axis's 61.
            "hashtag": {"type": "string", "maxLength": 59, "position": 0},
            "amount": {"type": "integer", "minimum": 0, "maximum": 4294967295u64, "position": 1},
        },
        "required": ["hashtag", "amount", "$createdAt"],
        "indices": Value::Array(vec![Value::Map(index_map)]),
        "additionalProperties": false,
    });
    let schemas = platform_value!({ "post": document_schema });
    factory
        .create_with_value_config(Identifier::from([seed; 32]), 0, schemas, None, None)
        .expect("contract registers")
        .data_contract_owned()
}

/// One full TTL drain cycle against a contract: two groups in a doomed
/// bucket, one write past the horizon, and the bucket must be gone —
/// exercising whichever node-removal arm the index's aggregate keywords
/// select. Returns after asserting absence.
fn run_ttl_drain_cycle(contract: &DataContract) {
    use crate::drive::document::paths::contract_document_type_path_vec;
    use crate::fees::op::LowLevelDriveOperation;
    use crate::util::grove_operations::DirectQueryType;
    use dpp::data_contract::document_type::DocumentPropertyType;
    use grovedb_path::SubtreePath;

    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    drive
        .apply_contract(
            contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            platform_version,
        )
        .expect("apply contract");
    let document_type = contract.document_type_for_name("post").expect("post");
    let transform = document_type
        .indexes()
        .get("trendingTtl")
        .expect("index")
        .time_range
        .clone()
        .expect("transform");

    let insert_at = |created_at: u64, tag: &str| {
        let owner_bytes = fixture_bytes(9, created_at, tag);
        let document = Document::V0(DocumentV0 {
            id: Identifier::from(fixture_bytes(10, created_at, tag)),
            owner_id: Identifier::from(owner_bytes),
            properties: BTreeMap::from([
                ("hashtag".to_string(), Value::Text(tag.to_string())),
                ("amount".to_string(), Value::U64(5)),
            ]),
            created_at: Some(created_at),
            revision: Some(1),
            ..Default::default()
        });
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((
                            &document,
                            StorageFlags::optional_default_as_cow(),
                        )),
                        owner_id: Some(owner_bytes),
                    },
                    contract,
                    document_type,
                },
                false,
                BlockInfo {
                    time_ms: created_at,
                    ..Default::default()
                },
                true,
                None,
                platform_version,
                None,
            )
            .expect("add document");
    };

    let h = HOUR_MS;
    let t0 = 3_000 * h;
    insert_at(t0 + MINUTE_MS_TTL, "aa");
    insert_at(t0 + 2 * MINUTE_MS_TTL, "bb");
    insert_at(t0 + 6 * h, "live");

    let mut level_path = contract_document_type_path_vec(contract.id_ref().as_bytes(), "post");
    level_path.push(transform.storage_key("$createdAt").into_bytes());
    let path_refs: Vec<&[u8]> = level_path
        .iter()
        .map(|segment| segment.as_slice())
        .collect();
    let mut ops: Vec<LowLevelDriveOperation> = vec![];
    let doomed = drive
        .grove_has_raw(
            SubtreePath::from(path_refs.as_slice()),
            DocumentPropertyType::encode_date_timestamp(t0).as_slice(),
            DirectQueryType::StatefulDirectQuery,
            None,
            &mut ops,
            &platform_version.drive,
        )
        .expect("existence check");
    assert!(
        !doomed,
        "the doomed bucket drains within one write's budget for this layout"
    );
}

/// Comment-19 matrix: drainage must be exercised for every node-removal
/// arm — the flat-drop fallback under a plain (non-ranked) property-name
/// tree, and the three dedicated indexed-tree deletes.
#[test]
fn ttl_drainage_covers_every_parent_layout() {
    // Plain parent: countable only — value trees leave via the flat drop.
    run_ttl_drain_cycle(&build_ttl_contract_with_index_keys(
        210,
        vec![(
            Value::Text("countable".to_string()),
            Value::Text("countable".to_string()),
        )],
    ));
    // ProvableCountIndexedTree parent (rankedCountable).
    run_ttl_drain_cycle(&build_ttl_contract_with_index_keys(
        211,
        vec![
            (
                Value::Text("countable".to_string()),
                Value::Text("countable".to_string()),
            ),
            (Value::Text("rangeCountable".to_string()), Value::Bool(true)),
            (
                Value::Text("rankedCountable".to_string()),
                Value::Bool(true),
            ),
        ],
    ));
    // ProvableSumIndexedTree parent (rankedSummable over `amount`).
    run_ttl_drain_cycle(&build_ttl_contract_with_index_keys(
        212,
        vec![
            (
                Value::Text("summable".to_string()),
                Value::Text("amount".to_string()),
            ),
            (Value::Text("rangeSummable".to_string()), Value::Bool(true)),
            (Value::Text("rankedSummable".to_string()), Value::Bool(true)),
        ],
    ));
    // ProvableCountProvableSumIndexedTree parent (rankedAverageable).
    run_ttl_drain_cycle(&build_ttl_contract_with_index_keys(
        213,
        vec![
            (
                Value::Text("averageable".to_string()),
                Value::Text("amount".to_string()),
            ),
            (
                Value::Text("rangeAverageable".to_string()),
                Value::Bool(true),
            ),
            (
                Value::Text("rankedAverageable".to_string()),
                Value::Bool(true),
            ),
        ],
    ));
}

/// Comment-17 regression: a drainage budget that runs out immediately
/// after dropping a group's flat `[0]` tree leaves the group's value tree
/// standing without it. Deleting that group's document must then skip at
/// the `[0]` granularity — every shallower segment of its entry path
/// still exists — and a later drain finishes the bucket.
#[test]
fn ttl_budget_boundary_after_zero_tree_keeps_deletes_exact() {
    use crate::drive::document::paths::contract_document_type_path_vec;
    use crate::fees::op::{LowLevelDriveOperation, TimeRangeTtlDrainRequest};
    use crate::util::grove_operations::DirectQueryType;
    use dpp::data_contract::document_type::DocumentPropertyType;
    use grovedb_path::SubtreePath;

    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = build_ttl_contract_with_index_keys(
        214,
        vec![
            (
                Value::Text("countable".to_string()),
                Value::Text("countable".to_string()),
            ),
            (Value::Text("rangeCountable".to_string()), Value::Bool(true)),
            (
                Value::Text("rankedCountable".to_string()),
                Value::Bool(true),
            ),
        ],
    );
    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            platform_version,
        )
        .expect("apply contract");
    let document_type = contract.document_type_for_name("post").expect("post");
    let transform = document_type
        .indexes()
        .get("trendingTtl")
        .expect("index")
        .time_range
        .clone()
        .expect("transform");
    let storage_key = transform.storage_key("$createdAt");
    let bucket_level = document_type
        .index_structure()
        .sub_levels()
        .get(&storage_key)
        .expect("the grid level exists in the index structure");

    let h = HOUR_MS;
    let t0 = 4_000 * h;
    let owner_bytes = fixture_bytes(11, t0, "solo");
    let document = Document::V0(DocumentV0 {
        id: Identifier::from(fixture_bytes(12, t0, "solo")),
        owner_id: Identifier::from(owner_bytes),
        properties: BTreeMap::from([
            ("hashtag".to_string(), Value::Text("solo".to_string())),
            ("amount".to_string(), Value::U64(5)),
        ]),
        created_at: Some(t0 + MINUTE_MS_TTL),
        revision: Some(1),
        ..Default::default()
    });
    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((
                        &document,
                        StorageFlags::optional_default_as_cow(),
                    )),
                    owner_id: Some(owner_bytes),
                },
                contract: &contract,
                document_type,
            },
            false,
            BlockInfo {
                time_ms: t0 + MINUTE_MS_TTL,
                ..Default::default()
            },
            true,
            None,
            platform_version,
            None,
        )
        .expect("add document");

    let mut level_path = contract_document_type_path_vec(contract.id_ref().as_bytes(), "post");
    level_path.push(storage_key.clone().into_bytes());
    let after_expiry_ms = t0 + 6 * h;

    // Budget 1: exactly the group's `[0]` tree drops; its value tree
    // stands without it.
    let drain_request = |max_operations: u16| TimeRangeTtlDrainRequest {
        transform: transform.clone(),
        bucket_level: bucket_level.clone(),
        level_path: level_path.clone(),
        block_time_ms: after_expiry_ms,
        max_operations,
    };
    drive
        .drain_expired_time_range_buckets(&drain_request(1), None, &platform_version.drive)
        .expect("a budget of one drops exactly the [0] tree");
    let exists = |segments: &[Vec<u8>]| -> bool {
        let (key, parents) = segments.split_last().expect("non-empty");
        let parent_refs: Vec<&[u8]> = parents.iter().map(|segment| segment.as_slice()).collect();
        let mut ops: Vec<LowLevelDriveOperation> = vec![];
        drive
            .grove_has_raw(
                SubtreePath::from(parent_refs.as_slice()),
                key.as_slice(),
                DirectQueryType::StatefulDirectQuery,
                None,
                &mut ops,
                &platform_version.drive,
            )
            .expect("existence check")
    };
    let bucket_key = DocumentPropertyType::encode_date_timestamp(t0);
    let mut group_path = level_path.clone();
    group_path.push(bucket_key.clone());
    group_path.push(b"hashtag".to_vec());
    group_path.push(b"solo".to_vec());
    let mut zero_path = group_path.clone();
    zero_path.push(vec![0]);
    assert!(exists(&group_path), "the value tree stands");
    assert!(!exists(&zero_path), "its [0] tree is gone");

    // The delete must skip at [0] granularity rather than target the
    // missing subtree.
    drive
        .delete_document_for_contract(
            document.id(),
            &contract,
            "post",
            BlockInfo {
                time_ms: after_expiry_ms,
                ..Default::default()
            },
            true,
            None,
            platform_version,
            None,
        )
        .expect("deleting a doc whose [0] tree drained must succeed");

    // A later drain finishes the bucket.
    drive
        .drain_expired_time_range_buckets(&drain_request(16), None, &platform_version.drive)
        .expect("the rest of the bucket drains");
    let mut bucket_path = level_path.clone();
    bucket_path.push(bucket_key);
    assert!(!exists(&bucket_path), "the bucket is gone");
}

/// Comment-14 (blocking) regression: drainage is unbilled system
/// maintenance, so a write that performs it must never cost more than
/// its estimate — the `estimated >= actual` invariant that validation's
/// balance check depends on. The estimate runs first (it does not read
/// or mutate state), then the same write applies while draining an
/// expired bucket.
#[test]
fn ttl_draining_write_never_exceeds_its_estimate() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = build_ttl_contract_with_index_keys(
        215,
        vec![
            (
                Value::Text("countable".to_string()),
                Value::Text("countable".to_string()),
            ),
            (Value::Text("rangeCountable".to_string()), Value::Bool(true)),
            (
                Value::Text("rankedCountable".to_string()),
                Value::Bool(true),
            ),
        ],
    );
    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            platform_version,
        )
        .expect("apply contract");
    let document_type = contract.document_type_for_name("post").expect("post");

    let h = HOUR_MS;
    let t0 = 5_000 * h;
    let make_doc = |created_at: u64, tag: &str| -> Document {
        Document::V0(DocumentV0 {
            id: Identifier::from(fixture_bytes(14, created_at, tag)),
            owner_id: Identifier::from(fixture_bytes(13, created_at, tag)),
            properties: BTreeMap::from([
                ("hashtag".to_string(), Value::Text(tag.to_string())),
                ("amount".to_string(), Value::U64(5)),
            ]),
            created_at: Some(created_at),
            revision: Some(1),
            ..Default::default()
        })
    };
    let add = |document: &Document, apply: bool| -> dpp::fee::fee_result::FeeResult {
        let owner_bytes = document.owner_id().to_buffer();
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((
                            document,
                            StorageFlags::optional_default_as_cow(),
                        )),
                        owner_id: Some(owner_bytes),
                    },
                    contract: &contract,
                    document_type,
                },
                false,
                BlockInfo {
                    time_ms: document.created_at().expect("created at"),
                    ..Default::default()
                },
                apply,
                None,
                platform_version,
                None,
            )
            .expect("add document")
    };

    // Seed the doomed bucket, then a write far enough ahead that applying
    // it drains that bucket.
    add(&make_doc(t0 + MINUTE_MS_TTL, "old"), true);
    let draining_doc = make_doc(t0 + 6 * h, "fresh");
    let estimated = add(&draining_doc, false);
    let actual = add(&draining_doc, true);
    assert!(
        estimated.storage_fee >= actual.storage_fee,
        "storage: estimated {} must cover actual {}",
        estimated.storage_fee,
        actual.storage_fee
    );
    assert!(
        estimated.processing_fee >= actual.processing_fee,
        "processing: estimated {} must cover actual {} — drainage must not \
         bill the triggering write beyond its estimate",
        estimated.processing_fee,
        actual.processing_fee
    );
}

/// Drainage rides updates too, not only inserts: with no insert ever
/// touching the index again, a lone update past the horizon must drop
/// the expired bucket — including the one the updated document's own
/// entries lived in, whose old-entry removal then skips coherently.
#[test]
fn ttl_update_only_write_drains_expired_buckets() {
    use crate::drive::document::paths::contract_document_type_path_vec;
    use crate::fees::op::LowLevelDriveOperation;
    use crate::util::grove_operations::DirectQueryType;
    use dpp::data_contract::document_type::DocumentPropertyType;
    use grovedb_path::SubtreePath;

    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = build_ttl_contract_with_index_keys(
        216,
        vec![
            (
                Value::Text("countable".to_string()),
                Value::Text("countable".to_string()),
            ),
            (Value::Text("rangeCountable".to_string()), Value::Bool(true)),
            (
                Value::Text("rankedCountable".to_string()),
                Value::Bool(true),
            ),
        ],
    );
    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            platform_version,
        )
        .expect("apply contract");
    let document_type = contract.document_type_for_name("post").expect("post");
    let transform = document_type
        .indexes()
        .get("trendingTtl")
        .expect("index")
        .time_range
        .clone()
        .expect("transform");

    let h = HOUR_MS;
    let t0 = 6_000 * h;
    let owner_bytes = fixture_bytes(15, t0, "only");
    let mut document = Document::V0(DocumentV0 {
        id: Identifier::from(fixture_bytes(16, t0, "only")),
        owner_id: Identifier::from(owner_bytes),
        properties: BTreeMap::from([
            ("hashtag".to_string(), Value::Text("only".to_string())),
            ("amount".to_string(), Value::U64(5)),
        ]),
        created_at: Some(t0 + MINUTE_MS_TTL),
        revision: Some(1),
        ..Default::default()
    });
    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((
                        &document,
                        StorageFlags::optional_default_as_cow(),
                    )),
                    owner_id: Some(owner_bytes),
                },
                contract: &contract,
                document_type,
            },
            false,
            BlockInfo {
                time_ms: t0 + MINUTE_MS_TTL,
                ..Default::default()
            },
            true,
            None,
            platform_version,
            None,
        )
        .expect("add document");

    // The only write after expiry is an UPDATE.
    document.set("hashtag", Value::Text("only2".to_string()));
    document.set_revision(Some(2));
    drive
        .update_document_for_contract(
            &document,
            &contract,
            document_type,
            Some(owner_bytes),
            BlockInfo {
                time_ms: t0 + 6 * h,
                ..Default::default()
            },
            true,
            None,
            None,
            platform_version,
            None,
        )
        .expect("an update past the horizon succeeds and drains");

    let mut level_path = contract_document_type_path_vec(contract.id_ref().as_bytes(), "post");
    level_path.push(transform.storage_key("$createdAt").into_bytes());
    let path_refs: Vec<&[u8]> = level_path
        .iter()
        .map(|segment| segment.as_slice())
        .collect();
    let mut ops: Vec<LowLevelDriveOperation> = vec![];
    let bucket_stands = drive
        .grove_has_raw(
            SubtreePath::from(path_refs.as_slice()),
            DocumentPropertyType::encode_date_timestamp(t0).as_slice(),
            DirectQueryType::StatefulDirectQuery,
            None,
            &mut ops,
            &platform_version.drive,
        )
        .expect("existence check");
    assert!(
        !bucket_stands,
        "an update-only write must drain the expired bucket"
    );
}

/// The ephemeral-bytes fee reclassification, measured against a standing
/// twin: two contracts identical byte-for-byte except that one declares a
/// `ttl`. The TTL'd insert's index bytes must leave the storage fee (only
/// the primary document row still bills there) and land in processing at
/// the ephemeral-bytes rate; deleting the document must refund strictly
/// less, because flagless ephemeral index bytes have nothing to refund.
/// Estimation stays an upper bound in both classes through the split
/// batch.
#[test]
fn ttl_index_bytes_bill_to_processing_without_refunds() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let index_keys = || {
        vec![
            (
                Value::Text("countable".to_string()),
                Value::Text("countable".to_string()),
            ),
            (Value::Text("rangeCountable".to_string()), Value::Bool(true)),
            (
                Value::Text("rankedCountable".to_string()),
                Value::Bool(true),
            ),
        ]
    };
    let ttl_contract =
        build_time_range_contract_with_index_keys(217, Some(4 * HOUR_SECONDS), index_keys());
    let standing_contract = build_time_range_contract_with_index_keys(218, None, index_keys());
    // Same document schema with no indexes at all: its insert pays for the
    // primary document row alone, giving the exact storage fee a TTL'd
    // contract must match if its index bytes truly bill zero storage.
    let index_free_contract = {
        let factory =
            DataContractFactory::new(PlatformVersion::latest().protocol_version).expect("factory");
        let document_schema = platform_value!({
            "type": "object",
            "properties": {
                "hashtag": {"type": "string", "maxLength": 59, "position": 0},
                "amount": {"type": "integer", "minimum": 0, "maximum": 4294967295u64, "position": 1},
            },
            "required": ["hashtag", "amount", "$createdAt"],
            "additionalProperties": false,
        });
        factory
            .create_with_value_config(
                Identifier::from([219u8; 32]),
                0,
                platform_value!({ "post": document_schema }),
                None,
                None,
            )
            .expect("contract registers")
            .data_contract_owned()
    };
    for contract in [&ttl_contract, &standing_contract, &index_free_contract] {
        drive
            .apply_contract(
                contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("apply contract");
    }

    let t0 = 5_000 * HOUR_MS;
    let block_info = BlockInfo {
        time_ms: t0,
        ..Default::default()
    };
    let make_doc = || -> Document {
        Document::V0(DocumentV0 {
            id: Identifier::from(fixture_bytes(17, t0, "twin")),
            owner_id: Identifier::from(fixture_bytes(18, t0, "twin")),
            properties: BTreeMap::from([
                ("hashtag".to_string(), Value::Text("twin".to_string())),
                ("amount".to_string(), Value::U64(5)),
            ]),
            created_at: Some(t0),
            revision: Some(1),
            ..Default::default()
        })
    };
    let add = |contract: &DataContract, apply: bool| -> FeeResult {
        let document = make_doc();
        let owner_bytes = document.owner_id().to_buffer();
        // Owner-carrying flags, so standing index bytes produce visible
        // refunds on delete — the contrast the TTL side must not show.
        let storage_flags = Cow::Owned(StorageFlags::SingleEpochOwned(0, owner_bytes));
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&document, Some(storage_flags))),
                        owner_id: Some(owner_bytes),
                    },
                    contract,
                    document_type: contract.document_type_for_name("post").expect("post"),
                },
                false,
                block_info,
                apply,
                None,
                platform_version,
                None,
            )
            .expect("add document")
    };

    let ttl_estimated = add(&ttl_contract, false);
    let ttl_insert = add(&ttl_contract, true);
    let standing_insert = add(&standing_contract, true);
    let index_free_insert = add(&index_free_contract, true);

    assert!(
        ttl_insert.storage_fee < standing_insert.storage_fee,
        "TTL'd index bytes must leave the storage fee: {} vs standing {}",
        ttl_insert.storage_fee,
        standing_insert.storage_fee
    );
    assert!(
        ttl_insert.storage_fee > 0,
        "the primary document row still bills to storage"
    );
    assert!(
        ttl_insert.processing_fee > standing_insert.processing_fee,
        "the ephemeral-bytes rate must land in processing: {} vs standing {}",
        ttl_insert.processing_fee,
        standing_insert.processing_fee
    );
    assert_eq!(
        ttl_insert.storage_fee, index_free_insert.storage_fee,
        "with a TTL, index writes must contribute exactly zero storage: the \
         storage fee must equal an index-free contract's"
    );
    assert!(
        ttl_estimated.storage_fee >= ttl_insert.storage_fee
            && ttl_estimated.processing_fee >= ttl_insert.processing_fee,
        "estimation must stay an upper bound in both fee classes through the \
         split batch: estimated ({}, {}) vs actual ({}, {})",
        ttl_estimated.storage_fee,
        ttl_estimated.processing_fee,
        ttl_insert.storage_fee,
        ttl_insert.processing_fee
    );

    let doc_id = make_doc().id();
    let delete = |contract: &DataContract| -> FeeResult {
        drive
            .delete_document_for_contract(
                doc_id,
                contract,
                "post",
                block_info,
                true,
                None,
                platform_version,
                None,
            )
            .expect("delete document")
    };
    let ttl_delete = delete(&ttl_contract);
    let standing_delete = delete(&standing_contract);
    let index_free_delete = delete(&index_free_contract);
    let refund_total = |fee_result: &FeeResult| -> u64 {
        fee_result
            .fee_refunds
            .clone()
            .sum_per_epoch()
            .into_values()
            .sum()
    };
    assert!(
        refund_total(&ttl_delete) < refund_total(&standing_delete),
        "flagless ephemeral index bytes must not refund: ttl {} vs standing {}",
        refund_total(&ttl_delete),
        refund_total(&standing_delete)
    );
    assert_eq!(
        refund_total(&ttl_delete),
        refund_total(&index_free_delete),
        "a TTL'd delete refunds exactly the primary document row — the same \
         as a contract with no indexes at all"
    );
}

/// Two TTL'd indexes sharing one grid on `$createdAt` (`byTag` and
/// `byAmount`): they share the level, its buckets and its drain.
fn build_shared_grid_ttl_contract(seed: u8) -> DataContract {
    let factory =
        DataContractFactory::new(PlatformVersion::latest().protocol_version).expect("factory");
    let time_range = || {
        Value::Map(vec![
            (
                Value::Text("on".to_string()),
                Value::Text("$createdAt".to_string()),
            ),
            (
                Value::Text("range".to_string()),
                Value::U64(2 * HOUR_SECONDS),
            ),
            (
                Value::Text("step".to_string()),
                Value::U64(2 * HOUR_SECONDS),
            ),
            (Value::Text("ttl".to_string()), Value::U64(4 * HOUR_SECONDS)),
        ])
    };
    let index = |name: &str, second: &str| {
        Value::Map(vec![
            (
                Value::Text("name".to_string()),
                Value::Text(name.to_string()),
            ),
            (
                Value::Text("properties".to_string()),
                Value::Array(vec![
                    platform_value!({"$createdAt": "asc"}),
                    Value::Map(vec![(
                        Value::Text(second.to_string()),
                        Value::Text("asc".to_string()),
                    )]),
                ]),
            ),
            (Value::Text("timeRange".to_string()), time_range()),
        ])
    };
    let document_schema = platform_value!({
        "type": "object",
        "properties": {
            "hashtag": {"type": "string", "maxLength": 59, "position": 0},
            "amount": {"type": "integer", "minimum": 0, "maximum": 4294967295u64, "position": 1},
        },
        "required": ["hashtag", "amount", "$createdAt"],
        "indices": Value::Array(vec![index("byTag", "hashtag"), index("byAmount", "amount")]),
        "additionalProperties": false,
    });
    let schemas = platform_value!({ "post": document_schema });
    factory
        .create_with_value_config(Identifier::from([seed; 32]), 0, schemas, None, None)
        .expect("contract registers")
        .data_contract_owned()
}

/// Regression: drainage must never run while a transition's own removals
/// are still queued. A documents batch of `[delete D, create E]` (and
/// `[update D, create E]`) where D's window is expired but still standing
/// queues removals under that bucket; had the create's drain executed
/// inline, it would have flat-dropped the very subtrees those removals
/// target and the batch would have failed to apply. The drain now runs
/// after the batch, once for the level both indexes share.
#[test]
fn ttl_removals_queued_before_a_draining_write_in_one_batch_still_apply() {
    use crate::drive::document::paths::contract_document_type_path_vec;
    use crate::fees::op::LowLevelDriveOperation;
    use crate::util::batch::{DocumentOperationType, DriveOperation};
    use crate::util::grove_operations::DirectQueryType;
    use crate::util::object_size_info::{DataContractInfo, DocumentTypeInfo};
    use dpp::data_contract::document_type::DocumentPropertyType;
    use grovedb_path::SubtreePath;

    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = build_shared_grid_ttl_contract(218);
    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            platform_version,
        )
        .expect("apply contract");
    let document_type = contract.document_type_for_name("post").expect("post");
    let transform = document_type
        .indexes()
        .get("byTag")
        .expect("index")
        .time_range
        .clone()
        .expect("transform");

    let h = HOUR_MS;
    let t0 = 8_000 * h;
    let make_doc = |marker: u8, created_at: u64, tag: &str| -> Document {
        Document::V0(DocumentV0 {
            id: Identifier::from(fixture_bytes(marker, created_at, tag)),
            owner_id: Identifier::from(fixture_bytes(marker + 1, created_at, tag)),
            properties: BTreeMap::from([
                ("hashtag".to_string(), Value::Text(tag.to_string())),
                ("amount".to_string(), Value::U64(3)),
            ]),
            created_at: Some(created_at),
            revision: Some(1),
            ..Default::default()
        })
    };
    fn add_op<'a>(contract: &'a DataContract, document: &'a Document) -> DriveOperation<'a> {
        DriveOperation::DocumentOperation(DocumentOperationType::AddDocument {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentRefInfo((document, None)),
                owner_id: Some(document.owner_id().to_buffer()),
            },
            contract_info: DataContractInfo::BorrowedDataContract(contract),
            document_type_info: DocumentTypeInfo::DocumentTypeNameAsStr("post"),
            override_document: false,
        })
    }
    let apply = |operations: Vec<DriveOperation>, time_ms: u64| {
        drive.apply_drive_operations(
            operations,
            true,
            &BlockInfo {
                time_ms,
                ..Default::default()
            },
            None,
            platform_version,
            None,
        )
    };

    // Two documents in the doomed bucket, in different groups on both
    // indexes; no write happens between their insertion and expiry.
    let d1 = make_doc(20, t0 + MINUTE_MS_TTL, "d1");
    let d2 = make_doc(22, t0 + MINUTE_MS_TTL, "d2");
    apply(vec![add_op(&contract, &d1)], t0 + MINUTE_MS_TTL).expect("d1 inserts");
    apply(vec![add_op(&contract, &d2)], t0 + MINUTE_MS_TTL).expect("d2 inserts");

    let mut level_path = contract_document_type_path_vec(contract.id_ref().as_bytes(), "post");
    level_path.push(transform.storage_key("$createdAt").into_bytes());
    let exists = |segments: &[Vec<u8>]| -> bool {
        let (key, parents) = segments.split_last().expect("non-empty");
        let mut ops: Vec<LowLevelDriveOperation> = vec![];
        drive
            .grove_has_raw(
                SubtreePath::from(parents),
                key.as_slice(),
                DirectQueryType::StatefulDirectQuery,
                None,
                &mut ops,
                &platform_version.drive,
            )
            .expect("existence check")
    };
    let mut old_bucket_path = level_path.clone();
    old_bucket_path.push(DocumentPropertyType::encode_date_timestamp(t0));
    assert!(exists(&old_bucket_path), "the bucket stands before expiry");

    // Past the horizon: delete d1 (queued under the expired-but-standing
    // bucket), update d2 (its old entries queued the same way, on both
    // indexes), then create e — whose drain would have run inline.
    let mut d2_updated = d2.clone();
    d2_updated.set("hashtag", Value::Text("d2b".to_string()));
    d2_updated.set_revision(Some(2));
    let e = make_doc(24, t0 + 6 * h, "e");
    apply(
        vec![
            DriveOperation::DocumentOperation(DocumentOperationType::DeleteDocument {
                document_id: d1.id(),
                contract_info: DataContractInfo::BorrowedDataContract(&contract),
                document_type_info: DocumentTypeInfo::DocumentTypeNameAsStr("post"),
            }),
            DriveOperation::DocumentOperation(DocumentOperationType::UpdateDocument {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((&d2_updated, None)),
                    owner_id: Some(d2_updated.owner_id().to_buffer()),
                },
                contract_info: DataContractInfo::BorrowedDataContract(&contract),
                document_type_info: DocumentTypeInfo::DocumentTypeNameAsStr("post"),
            }),
            add_op(&contract, &e),
        ],
        t0 + 6 * h,
    )
    .expect("removals queued before a draining write must still apply");

    assert!(
        !exists(&old_bucket_path),
        "the expired bucket is gone once the batch and its drain applied"
    );
    let mut new_bucket_path = level_path.clone();
    new_bucket_path.push(DocumentPropertyType::encode_date_timestamp(t0 + 6 * h));
    assert!(exists(&new_bucket_path), "the live bucket took the create");
}

/// Several indexes may share one grid-qualified level (same grid, same
/// ttl); the walkers must drain that level exactly ONCE per write, before
/// any batch mutation is queued. The per-index regression: four countable
/// indexes share `$createdAt#7200#7200`, so a full bucket costs 13 drop
/// operations — more than one 8-op budget — and a per-index drain would
/// keep dropping paths (directly) that an earlier index's queued removals
/// target, failing batch apply with `InvalidPath`, while spending up to
/// four budgets. One update past the horizon must succeed against the
/// partially drained bucket, and a second write finishes the job.
#[test]
fn ttl_shared_grid_drains_once_per_write() {
    use crate::drive::document::paths::contract_document_type_path_vec;
    use crate::fees::op::LowLevelDriveOperation;
    use crate::util::grove_operations::DirectQueryType;
    use dpp::data_contract::document_type::DocumentPropertyType;
    use grovedb_path::SubtreePath;

    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));

    let factory =
        DataContractFactory::new(PlatformVersion::latest().protocol_version).expect("factory");
    let shared_grid_index = |name: &str, second_property: &str| -> Value {
        Value::Map(vec![
            (
                Value::Text("name".to_string()),
                Value::Text(name.to_string()),
            ),
            (
                Value::Text("properties".to_string()),
                Value::Array(vec![
                    platform_value!({"$createdAt": "asc"}),
                    platform_value!({second_property: "asc"}),
                ]),
            ),
            (
                Value::Text("timeRange".to_string()),
                Value::Map(vec![
                    (
                        Value::Text("on".to_string()),
                        Value::Text("$createdAt".to_string()),
                    ),
                    (
                        Value::Text("range".to_string()),
                        Value::U64(2 * HOUR_SECONDS),
                    ),
                    (
                        Value::Text("step".to_string()),
                        Value::U64(2 * HOUR_SECONDS),
                    ),
                    (Value::Text("ttl".to_string()), Value::U64(4 * HOUR_SECONDS)),
                ]),
            ),
            (
                Value::Text("countable".to_string()),
                Value::Text("countable".to_string()),
            ),
        ])
    };
    let document_schema = platform_value!({
        "type": "object",
        "properties": {
            "hashtag": {"type": "string", "maxLength": 59, "position": 0},
            "amount": {"type": "integer", "minimum": 0, "maximum": 4294967295u64, "position": 1},
            "alpha": {"type": "string", "maxLength": 59, "position": 2},
            "beta": {"type": "string", "maxLength": 59, "position": 3},
        },
        "required": ["hashtag", "amount", "alpha", "beta", "$createdAt"],
        // Index names order the walker's per-index loop (BTreeMap), while
        // drainage walks property-name trees in KEY order — so `aHashtag`
        // iterates FIRST while its `hashtag` tree drains LAST. Under a
        // per-index drain that is the poison ordering: the first index
        // queues removals against the still-standing hashtag entries, then
        // a later index's drain drops them directly and batch apply fails
        // with InvalidPath.
        "indices": Value::Array(vec![
            shared_grid_index("aHashtag", "hashtag"),
            shared_grid_index("bAmount", "amount"),
            shared_grid_index("cAlpha", "alpha"),
            shared_grid_index("dBeta", "beta"),
        ]),
        "additionalProperties": false,
    });
    let contract = factory
        .create_with_value_config(
            Identifier::from([220u8; 32]),
            0,
            platform_value!({ "post": document_schema }),
            None,
            None,
        )
        .expect("four indexes sharing one grid and ttl validate")
        .data_contract_owned();
    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            platform_version,
        )
        .expect("apply contract");
    let document_type = contract.document_type_for_name("post").expect("post");
    let transform = document_type
        .indexes()
        .get("aHashtag")
        .expect("index")
        .time_range
        .clone()
        .expect("transform");

    let h = HOUR_MS;
    let t0 = 7_000 * h;
    let owner_bytes = fixture_bytes(19, t0, "zeta");
    let mut document = Document::V0(DocumentV0 {
        id: Identifier::from(fixture_bytes(20, t0, "zeta")),
        owner_id: Identifier::from(owner_bytes),
        properties: BTreeMap::from([
            ("hashtag".to_string(), Value::Text("zeta".to_string())),
            ("amount".to_string(), Value::U64(4)),
            ("alpha".to_string(), Value::Text("four".to_string())),
            ("beta".to_string(), Value::Text("nine".to_string())),
        ]),
        created_at: Some(t0 + MINUTE_MS_TTL),
        revision: Some(1),
        ..Default::default()
    });
    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((
                        &document,
                        StorageFlags::optional_default_as_cow(),
                    )),
                    owner_id: Some(owner_bytes),
                },
                contract: &contract,
                document_type,
            },
            false,
            BlockInfo {
                time_ms: t0 + MINUTE_MS_TTL,
                ..Default::default()
            },
            true,
            None,
            platform_version,
            None,
        )
        .expect("add document");
    // A second document in the same bucket, distinct on every indexed
    // property: the update below prunes its own emptied chains on all
    // four indexes, so without a survivor the bucket would empty and go
    // through up-tree pruning rather than through the drain.
    let survivor_owner = fixture_bytes(21, t0, "eta");
    let survivor = Document::V0(DocumentV0 {
        id: Identifier::from(fixture_bytes(22, t0, "eta")),
        owner_id: Identifier::from(survivor_owner),
        properties: BTreeMap::from([
            ("hashtag".to_string(), Value::Text("eta".to_string())),
            ("amount".to_string(), Value::U64(7)),
            ("alpha".to_string(), Value::Text("seven".to_string())),
            ("beta".to_string(), Value::Text("ten".to_string())),
        ]),
        created_at: Some(t0 + MINUTE_MS_TTL),
        revision: Some(1),
        ..Default::default()
    });
    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((
                        &survivor,
                        StorageFlags::optional_default_as_cow(),
                    )),
                    owner_id: Some(survivor_owner),
                },
                contract: &contract,
                document_type,
            },
            false,
            BlockInfo {
                time_ms: t0 + MINUTE_MS_TTL,
                ..Default::default()
            },
            true,
            None,
            platform_version,
            None,
        )
        .expect("add survivor");

    // First write past the horizon: the survivor's entries leave the
    // bucket needing 13 drop operations, the budget allows 8 — a partial
    // drain is guaranteed, and every index's queued removals must stay
    // consistent with it.
    let mut update_at = |time_ms: u64, revision: u64, hashtag: &str| {
        document.set("hashtag", Value::Text(hashtag.to_string()));
        document.set_revision(Some(revision));
        drive
            .update_document_for_contract(
                &document,
                &contract,
                document_type,
                Some(owner_bytes),
                BlockInfo {
                    time_ms,
                    ..Default::default()
                },
                true,
                None,
                None,
                platform_version,
                None,
            )
            .expect("an update against a partially drained shared-grid bucket succeeds");
    };
    let mut level_path = contract_document_type_path_vec(contract.id_ref().as_bytes(), "post");
    level_path.push(transform.storage_key("$createdAt").into_bytes());
    let bucket_stands = || -> bool {
        let path_refs: Vec<&[u8]> = level_path
            .iter()
            .map(|segment| segment.as_slice())
            .collect();
        let mut ops: Vec<LowLevelDriveOperation> = vec![];
        drive
            .grove_has_raw(
                SubtreePath::from(path_refs.as_slice()),
                DocumentPropertyType::encode_date_timestamp(t0).as_slice(),
                DirectQueryType::StatefulDirectQuery,
                None,
                &mut ops,
                &platform_version.drive,
            )
            .expect("existence check")
    };

    update_at(t0 + 6 * h, 2, "zeta2");
    // One 8-op budget cannot finish the 13-op bucket, so it must still
    // stand here — a per-index drain (four budgets in one write) would
    // already have removed it.
    assert!(
        bucket_stands(),
        "one write spends exactly one budget, so the bucket survives the \
         first update"
    );
    update_at(t0 + 6 * h + MINUTE_MS_TTL, 3, "zeta3");
    assert!(
        !bucket_stands(),
        "two writes' budgets (16 ops) must finish the 13-op shared bucket"
    );
}

/// The every-write cleanup rule includes DELETE-only writes: an index
/// receiving nothing but deletions must still advance drainage. Two
/// documents share one expired bucket (6 drop operations — within one
/// 8-op budget); deleting one past the horizon must take the whole
/// bucket, other document's expired entries included.
#[test]
fn ttl_delete_only_write_drains_expired_buckets() {
    use crate::drive::document::paths::contract_document_type_path_vec;
    use crate::fees::op::LowLevelDriveOperation;
    use crate::util::grove_operations::DirectQueryType;
    use dpp::data_contract::document_type::DocumentPropertyType;
    use grovedb_path::SubtreePath;

    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = build_ttl_contract_with_index_keys(
        221,
        vec![(
            Value::Text("countable".to_string()),
            Value::Text("countable".to_string()),
        )],
    );
    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            platform_version,
        )
        .expect("apply contract");
    let document_type = contract.document_type_for_name("post").expect("post");
    let transform = document_type
        .indexes()
        .get("trendingTtl")
        .expect("index")
        .time_range
        .clone()
        .expect("transform");

    let h = HOUR_MS;
    let t0 = 8_000 * h;
    let insert = |tag: &str, seed: u8| -> Identifier {
        let owner_bytes = fixture_bytes(seed, t0, tag);
        let document = Document::V0(DocumentV0 {
            id: Identifier::from(fixture_bytes(seed.wrapping_add(1), t0, tag)),
            owner_id: Identifier::from(owner_bytes),
            properties: BTreeMap::from([
                ("hashtag".to_string(), Value::Text(tag.to_string())),
                ("amount".to_string(), Value::U64(5)),
            ]),
            created_at: Some(t0 + MINUTE_MS_TTL),
            revision: Some(1),
            ..Default::default()
        });
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((
                            &document,
                            StorageFlags::optional_default_as_cow(),
                        )),
                        owner_id: Some(owner_bytes),
                    },
                    contract: &contract,
                    document_type,
                },
                false,
                BlockInfo {
                    time_ms: t0 + MINUTE_MS_TTL,
                    ..Default::default()
                },
                true,
                None,
                platform_version,
                None,
            )
            .expect("add document");
        document.id()
    };
    let doomed_id = insert("doomed", 21);
    insert("survivor", 23);

    // The only write after expiry is a DELETE.
    drive
        .delete_document_for_contract(
            doomed_id,
            &contract,
            "post",
            BlockInfo {
                time_ms: t0 + 6 * h,
                ..Default::default()
            },
            true,
            None,
            platform_version,
            None,
        )
        .expect("a delete past the horizon succeeds and drains");

    let mut level_path = contract_document_type_path_vec(contract.id_ref().as_bytes(), "post");
    level_path.push(transform.storage_key("$createdAt").into_bytes());
    let path_refs: Vec<&[u8]> = level_path
        .iter()
        .map(|segment| segment.as_slice())
        .collect();
    let mut ops: Vec<LowLevelDriveOperation> = vec![];
    let bucket_stands = drive
        .grove_has_raw(
            SubtreePath::from(path_refs.as_slice()),
            DocumentPropertyType::encode_date_timestamp(t0).as_slice(),
            DirectQueryType::StatefulDirectQuery,
            None,
            &mut ops,
            &platform_version.drive,
        )
        .expect("existence check");
    assert!(
        !bucket_stands,
        "a delete-only write must drain the expired bucket, the surviving \
         document's entries included"
    );
}
