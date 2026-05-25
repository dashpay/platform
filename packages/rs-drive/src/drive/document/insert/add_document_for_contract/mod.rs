mod v0;

use crate::drive::Drive;
use crate::util::object_size_info::DocumentAndContractInfo;

use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;

use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Adds a document to a contract.
    ///
    /// # Parameters
    /// * `document_and_contract_info`: Information about the document and contract.
    /// * `override_document`: Whether to override the document.
    /// * `block_info`: The block info.
    /// * `apply`: Whether to apply the operation.
    /// * `transaction`: The transaction argument.
    /// * `drive_version`: The drive version to select the correct function version to run.
    ///
    /// # Returns
    /// * `Ok(FeeResult)` if the operation was successful.
    /// * `Err(DriveError::UnknownVersionMismatch)` if the drive version does not match known versions.
    #[allow(clippy::too_many_arguments)]
    pub fn add_document_for_contract(
        &self,
        document_and_contract_info: DocumentAndContractInfo,
        override_document: bool,
        block_info: BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
        previous_fee_versions: Option<&CachedEpochIndexFeeVersions>,
    ) -> Result<FeeResult, Error> {
        match platform_version
            .drive
            .methods
            .document
            .insert
            .add_document_for_contract
        {
            0 => self.add_document_for_contract_v0(
                document_and_contract_info,
                override_document,
                block_info,
                apply,
                transaction,
                platform_version,
                previous_fee_versions,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_document_for_contract".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod time_range_index_e2e_tests {
    //! End-to-end coverage for time-range index fan-out: a single document is
    //! indexed under every overlapping range bucket its `$createdAt` falls
    //! into, those buckets are queryable by exact bucket start, and deletion
    //! removes every entry.
    //!
    //! Also covers the other half of that contract: a bucket-start equality
    //! only means "bucket" when it came from `IN_TIME_RANGE` resolution, so
    //! index selection is pinned by
    //! [`DriveDocumentQuery::resolved_time_range_fields`] rather than left to
    //! whichever index happens to cover the fields.
    use crate::config::DriveConfig;
    use crate::drive::Drive;
    use crate::query::DriveDocumentQuery;
    use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
    use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
    use crate::util::storage_flags::StorageFlags;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
    use dpp::data_contract::DataContractFactory;
    use dpp::document::{Document, DocumentV0, DocumentV0Getters, DocumentV0Setters};
    use dpp::platform_value::{platform_value, Identifier, Value};
    use dpp::prelude::DataContract;
    use dpp::tests::utils::generate_random_identifier_struct;
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

    /// One hour in each of the two units these tests deal in: `*_SECONDS`
    /// declares a contract's window, `*_MS` is a document timestamp, a bucket
    /// start or an index key. Scaling the wrong one silently shifts the
    /// buckets by a factor of a thousand, so they are kept apart by name.
    const HOUR_SECONDS: u64 = 3_600;
    const HOUR_MS: u64 = 3_600_000;

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
        let owner_id = generate_random_identifier_struct();
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
            vec!["$createdAt".to_string()],
        );
        query
            .execute_raw_results_no_proof(drive, None, None, platform_version)
            .expect("query")
            .0
            .len()
    }

    /// A `$createdAt == created_at` query, optionally ANDed with
    /// `hashtag == <hashtag>`, carrying `resolved_time_range_fields` verbatim
    /// so tests can drive both the resolved and the raw (empty) provenance.
    fn build_created_at_query<'a>(
        contract: &'a DataContract,
        document_type: dpp::data_contract::document_type::DocumentTypeRef<'a>,
        created_at: u64,
        hashtag: Option<&str>,
        resolved_time_range_fields: Vec<String>,
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
        query.resolved_time_range_fields = resolved_time_range_fields;
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

        let owner_bytes = rand::random::<[u8; 32]>();
        let document = Document::V0(DocumentV0 {
            id: Identifier::from(rand::random::<[u8; 32]>()),
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

        let owner_bytes = rand::random::<[u8; 32]>();
        let mut document = Document::V0(DocumentV0 {
            id: Identifier::from(rand::random::<[u8; 32]>()),
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
        let owner_id = generate_random_identifier_struct();
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
        let owner_bytes = rand::random::<[u8; 32]>();
        let document = Document::V0(DocumentV0 {
            id: Identifier::from(rand::random::<[u8; 32]>()),
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
            vec!["$createdAt".to_string()],
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

        let raw =
            build_created_at_query(&contract, document_type, created_at, Some("ibiza"), vec![]);
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

    /// One index can bucket only one field (a transform's source must be its
    /// index's first property), so a query resolving two time ranges has no
    /// servable shape and is refused rather than routed to whichever index
    /// happens to cover the fields.
    #[test]
    fn two_resolved_time_range_fields_are_rejected() {
        let contract = build_competing_index_trending_contract();
        let document_type = contract.document_type_for_name("post").expect("post");
        let query = build_created_at_query(
            &contract,
            document_type,
            6 * HOUR_MS,
            Some("ibiza"),
            vec!["$createdAt".to_string(), "hashtag".to_string()],
        );
        let error = query
            .find_best_index(PlatformVersion::latest())
            .expect_err("two resolved time-range fields cannot be served");
        assert!(
            matches!(
                error,
                crate::error::Error::Query(crate::error::query::QuerySyntaxError::Unsupported(_))
            ),
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

        query.resolved_time_range_fields = vec!["$createdAt".to_string()];
        let error = query
            .find_best_index(PlatformVersion::latest())
            .expect_err("no bucketed index covers the ordering");
        assert!(
            matches!(
                error,
                crate::error::Error::Query(
                    crate::error::query::QuerySyntaxError::WhereClauseOnNonIndexedProperty(_)
                )
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
        let owner_id = generate_random_identifier_struct();
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
        query.resolved_time_range_fields = vec!["$createdAt".to_string()];
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

        let owner_bytes = rand::random::<[u8; 32]>();
        let mut document = Document::V0(DocumentV0 {
            id: Identifier::from(rand::random::<[u8; 32]>()),
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
        let second_owner = rand::random::<[u8; 32]>();
        let second = Document::V0(DocumentV0 {
            id: Identifier::from(rand::random::<[u8; 32]>()),
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
                crate::error::Error::Query(
                    crate::error::query::QuerySyntaxError::WhereClauseOnNonIndexedProperty(_)
                )
            ),
            "expected a no-covering-index rejection, got {error:?}"
        );
    }
}
