//! Unit tests for the sum-query surface.
//!
//! Remaining test plan (full executor coverage waits on grovedb PR 670):
//!
//! - Total fast path: contract with `documents_summable: "amount"`,
//!   insert N documents, assert `Drive::execute_document_sum_request`
//!   returns `Aggregate(sum)` where sum equals the expected total
//!   given the bench's deterministic schedule.
//! - Per-recipient point lookup: `WHERE recipient == X` on the
//!   `byRecipient` index → `Aggregate(per_recipient_sum)`.
//! - Range: `WHERE sentAt > T` on the `bySentAt` index →
//!   `Aggregate(sum_in_range)`.
//! - All three get a prove/verify variant once
//!   `verify_aggregate_sum_query` lands in grovedb.

use super::index_picker::{
    find_range_summable_index_for_where_clauses, find_summable_index_for_where_clauses,
};
use crate::query::{WhereClause, WhereOperator};
use dpp::data_contract::document_type::{Index, IndexCountability, IndexProperty};
use dpp::platform_value::Value;
use std::collections::BTreeMap;

// ── Picker fixture builders ────────────────────────────────────────

fn idx_property(name: &str) -> IndexProperty {
    IndexProperty {
        name: name.to_string(),
        ascending: true,
    }
}

/// Build a non-range summable index with the given property list.
fn summable_index(name: &str, props: &[&str], summable: Option<&str>) -> Index {
    Index {
        name: name.to_string(),
        properties: props.iter().map(|p| idx_property(p)).collect(),
        unique: false,
        null_searchable: true,
        contested_index: None,
        countable: IndexCountability::NotCountable,
        range_countable: false,
        summable: summable.map(String::from),
        range_summable: false,
        ranked_countable: false,
        ranked_summable: false,
        ranked_averageable: false,
        time_range: None,
        terminal: None,
        preallocated: false,
    }
}

/// Build a range-summable index. Terminator is the last entry of
/// `props`; `summable` names the integer property being summed.
fn range_summable_index(name: &str, props: &[&str], summable: &str) -> Index {
    Index {
        name: name.to_string(),
        properties: props.iter().map(|p| idx_property(p)).collect(),
        unique: false,
        null_searchable: true,
        contested_index: None,
        countable: IndexCountability::NotCountable,
        range_countable: false,
        summable: Some(summable.to_string()),
        range_summable: true,
        ranked_countable: false,
        ranked_summable: false,
        ranked_averageable: false,
        time_range: None,
        terminal: None,
        preallocated: false,
    }
}

fn wc_equal(field: &str) -> WhereClause {
    WhereClause {
        field: field.to_string(),
        operator: WhereOperator::Equal,
        value: Value::U64(1),
    }
}

fn wc_in(field: &str) -> WhereClause {
    WhereClause {
        field: field.to_string(),
        operator: WhereOperator::In,
        value: Value::Array(vec![Value::U64(1), Value::U64(2)]),
    }
}

fn wc_gt(field: &str, v: u64) -> WhereClause {
    WhereClause {
        field: field.to_string(),
        operator: WhereOperator::GreaterThan,
        value: Value::U64(v),
    }
}

fn make_index_map(indexes: Vec<Index>) -> BTreeMap<String, Index> {
    indexes.into_iter().map(|i| (i.name.clone(), i)).collect()
}

// ── find_summable_index_for_where_clauses ──────────────────────────

#[test]
fn summable_picker_matches_single_prop_exactly() {
    let indexes = make_index_map(vec![summable_index(
        "byRecipient",
        &["recipient"],
        Some("amount"),
    )]);
    let found =
        find_summable_index_for_where_clauses(&indexes, &[wc_equal("recipient")], "amount", &[]);
    assert_eq!(found.map(|i| i.name.as_str()), Some("byRecipient"));
}

#[test]
fn summable_picker_rejects_partial_coverage() {
    // Two-prop index with only one of the props matched by where clauses.
    let indexes = make_index_map(vec![summable_index("byAB", &["a", "b"], Some("amount"))]);
    assert!(
        find_summable_index_for_where_clauses(&indexes, &[wc_equal("a")], "amount", &[]).is_none(),
        "partial coverage must miss the strict picker"
    );
}

#[test]
fn summable_picker_rejects_property_mismatch() {
    // Index sums "amount", query asks to sum "fee" — must miss.
    let indexes = make_index_map(vec![summable_index(
        "byRecipient",
        &["recipient"],
        Some("amount"),
    )]);
    assert!(
        find_summable_index_for_where_clauses(&indexes, &[wc_equal("recipient")], "fee", &[])
            .is_none()
    );
}

#[test]
fn summable_picker_rejects_non_summable_index() {
    // No `summable` declaration → never picked, even if properties match.
    let indexes = make_index_map(vec![summable_index("byRecipient", &["recipient"], None)]);
    assert!(find_summable_index_for_where_clauses(
        &indexes,
        &[wc_equal("recipient")],
        "amount",
        &[]
    )
    .is_none());
}

#[test]
fn summable_picker_rejects_range_operator() {
    let indexes = make_index_map(vec![summable_index(
        "bySentAt",
        &["sentAt"],
        Some("amount"),
    )]);
    assert!(
        find_summable_index_for_where_clauses(&indexes, &[wc_gt("sentAt", 0)], "amount", &[])
            .is_none(),
        "any range operator disqualifies the point-lookup picker"
    );
}

#[test]
fn summable_picker_accepts_in_clause() {
    let indexes = make_index_map(vec![summable_index(
        "byRecipient",
        &["recipient"],
        Some("amount"),
    )]);
    let found =
        find_summable_index_for_where_clauses(&indexes, &[wc_in("recipient")], "amount", &[]);
    assert_eq!(found.map(|i| i.name.as_str()), Some("byRecipient"));
}

// ── find_range_summable_index_for_where_clauses ────────────────────

#[test]
fn range_summable_picker_matches_terminator_range() {
    // [sentAt] index with rangeSummable: true; `sentAt > 0` should
    // pick it.
    let indexes = make_index_map(vec![range_summable_index(
        "bySentAt",
        &["sentAt"],
        "amount",
    )]);
    let found =
        find_range_summable_index_for_where_clauses(&indexes, &[wc_gt("sentAt", 0)], "amount", &[]);
    assert_eq!(found.map(|i| i.name.as_str()), Some("bySentAt"));
}

#[test]
fn range_summable_picker_matches_prefix_equal_plus_terminator_range() {
    // [recipient, sentAt] with Equal on prefix + range on terminator
    // is the rangeSummable carrier shape.
    let indexes = make_index_map(vec![range_summable_index(
        "byRecipientTime",
        &["recipient", "sentAt"],
        "amount",
    )]);
    let where_clauses = vec![wc_equal("recipient"), wc_gt("sentAt", 0)];
    let found =
        find_range_summable_index_for_where_clauses(&indexes, &where_clauses, "amount", &[]);
    assert_eq!(found.map(|i| i.name.as_str()), Some("byRecipientTime"));
}

#[test]
fn range_summable_picker_rejects_property_mismatch() {
    // Index sums "amount", query asks to sum "fee".
    let indexes = make_index_map(vec![range_summable_index(
        "bySentAt",
        &["sentAt"],
        "amount",
    )]);
    assert!(find_range_summable_index_for_where_clauses(
        &indexes,
        &[wc_gt("sentAt", 0)],
        "fee",
        &[]
    )
    .is_none());
}

#[test]
fn range_summable_picker_rejects_non_range_summable() {
    // summable but not rangeSummable — the point-lookup picker would
    // accept this; the range picker must not.
    let mut idx = range_summable_index("bySentAt", &["sentAt"], "amount");
    idx.range_summable = false;
    let indexes = make_index_map(vec![idx]);
    assert!(find_range_summable_index_for_where_clauses(
        &indexes,
        &[wc_gt("sentAt", 0)],
        "amount",
        &[]
    )
    .is_none());
}

#[test]
fn range_summable_picker_rejects_range_not_on_terminator() {
    // [recipient, sentAt] index but the range is on `recipient`, which
    // sits at position 0, not the terminator. Must miss.
    let indexes = make_index_map(vec![range_summable_index(
        "byRecipientTime",
        &["recipient", "sentAt"],
        "amount",
    )]);
    let where_clauses = vec![wc_gt("recipient", 0)];
    assert!(
        find_range_summable_index_for_where_clauses(&indexes, &where_clauses, "amount", &[])
            .is_none()
    );
}

// ── Dispatcher limit-policy regression tests ───────────────────────
//
// Sum-side analogs of count's
// [`test_range_distinct_proof_uses_compile_time_default_query_limit_not_operator_config`]
// and over-max rejection. The sum dispatcher mirrors count's
// validate-don't-clamp policy on the prove path; these tests pin that
// the dispatcher uses [`crate::config::DEFAULT_QUERY_LIMIT`] (compile-time
// constant) rather than the operator-tunable
// `drive_config.default_query_limit`, AND that an explicit
// `limit > max_query_limit` returns a typed
// `QuerySyntaxError::InvalidLimit` instead of silently clamping.
//
// Without these, a regression where the dispatcher reads from
// `drive_config.default_query_limit` would only surface on operators
// who tuned the runtime value away from the constant — exactly the
// silent verify-failure surface flagged by review.

#[cfg(feature = "server")]
mod limit_policy_regression {
    use crate::config::{DriveConfig, DEFAULT_QUERY_LIMIT};
    use crate::drive::Drive;
    use crate::error::query::QuerySyntaxError;
    use crate::error::Error;
    use crate::query::drive_document_sum_query::{
        DocumentSumRequest, DocumentSumResponse, DriveDocumentSumQuery, SumMode,
    };
    use crate::query::{WhereClause, WhereOperator};
    use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
    use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
    use crate::util::storage_flags::StorageFlags;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
    use dpp::data_contract::DataContractFactory;
    use dpp::document::{Document, DocumentV0};
    use dpp::identifier::Identifier;
    use dpp::platform_value::{platform_value, Value};
    use dpp::version::PlatformVersion;
    use grovedb::GroveDb;
    use std::borrow::Cow;
    use std::collections::BTreeMap as StdBTreeMap;

    const PROTOCOL_VERSION_V12: u32 = 12;

    /// Build a v12 contract with a `widget` doctype carrying a single
    /// `(color, amount)` `rangeSummable: true` index. The `byColor`
    /// index — `summable: "amount"` + `rangeSummable: true` — is what
    /// the SUM `RangeDistinctProof` arm walks (color = the per-distinct
    /// terminator key, amount = the summed per-doc value).
    fn build_widget_contract() -> dpp::data_contract::DataContract {
        let factory = DataContractFactory::new(PROTOCOL_VERSION_V12).expect("create factory");
        let document_schema = platform_value!({
            "type": "object",
            "properties": {
                "color":  {"type": "string",  "position": 0, "maxLength": 32},
                "amount": {"type": "integer", "position": 1, "minimum": 0, "maximum": 1000},
            },
            "required": ["color", "amount"],
            "indices": [{
                "name": "byColor",
                "properties": [{"color": "asc"}],
                "summable":      "amount",
                "rangeSummable": true,
            }],
            "additionalProperties": false,
        });
        let schemas = platform_value!({ "widget": document_schema });
        factory
            .create_with_value_config(
                dpp::tests::utils::generate_random_identifier_struct(),
                0,
                schemas,
                None,
                None,
            )
            .expect("create data contract")
            .data_contract_owned()
    }

    /// Insert one widget document at the given `(color, amount)` pair
    /// using the index `(i+1)` as a unique 32-byte id.
    fn insert_widget(
        drive: &Drive,
        contract: &dpp::data_contract::DataContract,
        i: usize,
        color: &str,
        amount: u64,
    ) {
        let platform_version = PlatformVersion::latest();
        let document_type = contract
            .document_type_for_name("widget")
            .expect("widget type exists");
        let mut properties = StdBTreeMap::new();
        properties.insert("color".to_string(), Value::Text(color.to_string()));
        properties.insert("amount".to_string(), Value::U64(amount));
        let document: Document = DocumentV0 {
            contract_version: None,
            id: Identifier::from([(i + 1) as u8; 32]),
            owner_id: Identifier::from([0u8; 32]),
            properties,
            revision: None,
            created_at: None,
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            creator_id: None,
        }
        .into();
        let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpoch(0)));
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&document, storage_flags)),
                        owner_id: None,
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
            .expect("insert widget");
    }

    /// SUM mirror of count's
    /// `test_range_distinct_proof_uses_compile_time_default_query_limit_not_operator_config`.
    ///
    /// Sets `drive_config.default_query_limit = 1` (≠ `DEFAULT_QUERY_LIMIT
    /// = 100`) and submits a SUM `GroupByRange + range + prove` request
    /// with `limit = None`. The dispatcher MUST fall back to the
    /// compile-time `DEFAULT_QUERY_LIMIT`, not the operator-tunable
    /// runtime value, so the proof bytes can be reconstructed and
    /// verified by an SDK that doesn't know the operator's tuned config.
    /// If the dispatcher regressed to using
    /// `drive_config.default_query_limit`, the prover would emit a
    /// 1-key proof and the reconstructed path query (built with
    /// `Some(DEFAULT_QUERY_LIMIT)`) would fail `verify_query` — that
    /// failure is what this test guards against.
    #[test]
    fn range_distinct_sum_proof_uses_compile_time_default_query_limit_not_operator_config() {
        const OPERATOR_TUNED_LIMIT: u16 = 1;
        assert_ne!(
            DEFAULT_QUERY_LIMIT, OPERATOR_TUNED_LIMIT,
            "test invariant: OPERATOR_TUNED_LIMIT must differ from DEFAULT_QUERY_LIMIT"
        );

        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let data_contract = build_widget_contract();

        drive
            .apply_contract(
                &data_contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("apply contract");

        // Distinct keys: 2 red @ 5, 3 green @ 7, 1 blue @ 2. The
        // `color > "blue"` range excludes blue, leaving 2 distinct
        // in-range terminator keys (red, green) — enough to make the
        // limit choice matter (with OPERATOR_TUNED_LIMIT = 1 the proof
        // shapes differ between the two key counts).
        let docs = [
            ("red", 5u64),
            ("red", 5),
            ("green", 7),
            ("green", 7),
            ("green", 7),
            ("blue", 2),
        ];
        for (i, (color, amount)) in docs.iter().enumerate() {
            insert_widget(&drive, &data_contract, i, color, *amount);
        }

        let document_type = data_contract
            .document_type_for_name("widget")
            .expect("widget");

        // Operator-tuned DriveConfig — dispatcher MUST NOT use this
        // on the prove path.
        let drive_config = DriveConfig {
            default_query_limit: OPERATOR_TUNED_LIMIT,
            ..Default::default()
        };

        let color_gt_blue = WhereClause {
            field: "color".to_string(),
            operator: WhereOperator::GreaterThan,
            value: Value::Text("blue".to_string()),
        };
        let request = DocumentSumRequest {
            contract: &data_contract,
            document_type,
            sum_property: "amount".to_string(),
            where_clauses: vec![color_gt_blue.clone()],
            order_clauses: Vec::new(),
            mode: SumMode::GroupByRange,
            limit: None,
            prove: true,
            drive_config: &drive_config,
            resolved_time_ranges: vec![],
        };

        let response = drive
            .execute_document_sum_request(request, None, platform_version)
            .expect("dispatcher should succeed on RangeDistinctProof SUM path");
        let proof_bytes = match response {
            DocumentSumResponse::Proof(p) => p,
            other => panic!("expected Proof response, got {:?}", other),
        };
        assert!(!proof_bytes.is_empty(), "non-empty proof bytes expected");

        // Rebuild the path query the way an SDK verifier does:
        // anchored to DEFAULT_QUERY_LIMIT. If the dispatcher signed
        // with `default_query_limit = OPERATOR_TUNED_LIMIT` instead,
        // the reconstructed `SizedQuery::limit` differs from the
        // prover's and `verify_query` returns Err.
        let index = crate::query::drive_document_sum_query::index_picker::find_range_summable_index_for_where_clauses(
            document_type.indexes(),
            std::slice::from_ref(&color_gt_blue),
            "amount",
            &[],
        )
        .expect("byColor rangeSummable index covers `color > blue`");
        let sum_query = DriveDocumentSumQuery {
            document_type,
            contract_id: data_contract.id().to_buffer(),
            document_type_name: "widget".to_string(),
            index,
            where_clauses: vec![color_gt_blue],
            sum_property: "amount".to_string(),
        };
        let verifier_path_query = sum_query
            .distinct_sum_path_query(Some(DEFAULT_QUERY_LIMIT), true, platform_version)
            .expect("path query builder accepts the same shape the prover used");

        let (_root_hash, _elements) = GroveDb::verify_query(
            &proof_bytes,
            &verifier_path_query,
            &platform_version.drive.grove_version,
        )
        .expect(
            "expected proof to verify against a path query rebuilt with DEFAULT_QUERY_LIMIT; \
             a failure here means the dispatcher signed the SUM proof with the \
             operator-tunable default_query_limit — a consensus-adjacent silent-verify \
             regression",
        );
    }

    /// Pins the over-max rejection on the SUM `RangeDistinctProof`
    /// arm: an explicit `limit > max_query_limit` MUST return
    /// [`QuerySyntaxError::InvalidLimit`] rather than silently
    /// clamping. The previous behavior (pre-fix) was a `.min()` clamp
    /// against `max_query_limit`, which would byte-differ the
    /// reconstructed `SizedQuery::limit` and break SDK verification on
    /// any request with `limit > max`.
    #[test]
    fn range_distinct_sum_proof_rejects_limit_over_max() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let data_contract = build_widget_contract();
        drive
            .apply_contract(
                &data_contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("apply contract");

        // Single distinct in-range doc is enough — the rejection
        // fires at the dispatcher's limit-validation gate before any
        // grovedb walk happens, so the fixture size doesn't matter.
        insert_widget(&drive, &data_contract, 0, "red", 5);

        let document_type = data_contract
            .document_type_for_name("widget")
            .expect("widget");
        let drive_config = DriveConfig::default();
        let over_max = drive_config.max_query_limit as u32 + 1;

        let color_gt_blue = WhereClause {
            field: "color".to_string(),
            operator: WhereOperator::GreaterThan,
            value: Value::Text("blue".to_string()),
        };
        let request = DocumentSumRequest {
            contract: &data_contract,
            document_type,
            sum_property: "amount".to_string(),
            where_clauses: vec![color_gt_blue],
            order_clauses: Vec::new(),
            mode: SumMode::GroupByRange,
            limit: Some(over_max),
            prove: true,
            drive_config: &drive_config,
            resolved_time_ranges: vec![],
        };

        let err = drive
            .execute_document_sum_request(request, None, platform_version)
            .expect_err("limit > max_query_limit must reject, not clamp");

        assert!(
            matches!(err, Error::Query(QuerySyntaxError::InvalidLimit(_))),
            "expected QuerySyntaxError::InvalidLimit, got {err:?}"
        );
        let msg = err.to_string();
        assert!(
            msg.contains("exceeds max_query_limit"),
            "error must name the rejected limit; got: {msg}"
        );
    }

    #[test]
    fn range_distinct_sum_no_proof_applies_default_explicit_and_max_limits() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let data_contract = build_widget_contract();
        drive
            .apply_contract(
                &data_contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("apply contract");

        // Five colors so the range predicate below matches FOUR distinct
        // values — one more than `max_query_limit` — otherwise the clamp
        // case would pass even against an unbounded walk.
        for (i, (color, amount)) in [
            ("blue", 2u64),
            ("green", 3),
            ("red", 5),
            ("white", 11),
            ("yellow", 7),
        ]
        .iter()
        .enumerate()
        {
            insert_widget(&drive, &data_contract, i, color, *amount);
        }

        let document_type = data_contract
            .document_type_for_name("widget")
            .expect("widget");
        let drive_config = DriveConfig {
            default_query_limit: 2,
            max_query_limit: 3,
            ..Default::default()
        };
        let make_request = |limit| DocumentSumRequest {
            contract: &data_contract,
            document_type,
            sum_property: "amount".to_string(),
            where_clauses: vec![WhereClause {
                field: "color".to_string(),
                operator: WhereOperator::GreaterThan,
                value: Value::Text("blue".to_string()),
            }],
            order_clauses: Vec::new(),
            mode: SumMode::GroupByRange,
            limit,
            prove: false,
            drive_config: &drive_config,
            resolved_time_ranges: vec![],
        };

        for (requested, expected) in [(None, 2), (Some(1), 1), (Some(10_000), 3)] {
            let response = drive
                .execute_document_sum_request(make_request(requested), None, platform_version)
                .expect("bounded no-proof distinct SUM should succeed");
            let entries = match response {
                DocumentSumResponse::Entries(entries) => entries,
                other => panic!("expected Entries response, got {other:?}"),
            };
            assert_eq!(
                entries.len(),
                expected,
                "unexpected entry count for requested limit {requested:?}"
            );
        }
    }
}
