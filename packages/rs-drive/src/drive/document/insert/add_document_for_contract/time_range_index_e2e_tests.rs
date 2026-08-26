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
use dpp::platform_value::{platform_value, Identifier, Value};
use dpp::prelude::DataContract;
use dpp::version::PlatformVersion;
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
/// Both plain indexes start with `hashtag` rather than `$createdAt`:
/// indexes sharing a first property must agree on its `timeRange`
/// transform, so a raw index can never lead with a bucketed field.
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
