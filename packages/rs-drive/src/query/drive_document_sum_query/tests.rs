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
    find_range_summable_index_for_where_clauses,
    find_range_summable_index_with_counts_for_where_clauses, find_summable_index_for_where_clauses,
    find_summable_index_with_counts_for_where_clauses,
};
use crate::query::{DriveDocumentCountQuery, WhereClause, WhereOperator};
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
        ranked_countable_at: vec![],
        ranked_summable_at: Vec::new(),
        ranked_averageable_at: Vec::new(),
        ranked_summable: false,
        ranked_averageable: false,
        time_range: None,
        integer_range: None,
        terminal: None,
        preallocated: false,
        outlives_delete: false,
        skip_if_absent: false,
        skip_if_absent_properties: Vec::new(),
        summable_off_count_index: None,
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
        ranked_countable_at: vec![],
        ranked_summable_at: Vec::new(),
        ranked_averageable_at: Vec::new(),
        ranked_summable: false,
        ranked_averageable: false,
        time_range: None,
        integer_range: None,
        terminal: None,
        preallocated: false,
        outlives_delete: false,
        skip_if_absent: false,
        skip_if_absent_properties: Vec::new(),
        summable_off_count_index: None,
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

// ── counts-aware pickers (average / count-and-sum) ─────────────────

/// A counter index summing the source index `byPost`.
fn summable_off_count_index(name: &str, props: &[&str]) -> Index {
    let mut index = range_summable_index(name, props, "byPost");
    index.summable = None;
    index.summable_off_count_index = Some("byPost".to_string());
    index
}

#[test]
fn should_keep_refusing_an_average_whose_first_regular_index_lacks_counts() {
    // Before protocol version 14 the average picked the first summable
    // index by name and then required it to be countable; a released
    // verifier rebuilds that choice, so a later countable index must not
    // answer instead.
    let first = summable_index("aByAB", &["a", "b"], Some("amount"));
    let mut second = summable_index("bByBA", &["b", "a"], Some("amount"));
    second.countable = IndexCountability::Countable;
    let indexes = make_index_map(vec![first, second]);
    let where_clauses = vec![wc_equal("a"), wc_equal("b")];
    assert!(find_summable_index_with_counts_for_where_clauses(
        &indexes,
        &where_clauses,
        "amount",
        &[]
    )
    .is_none());
}

#[test]
fn should_pass_over_a_counter_index_without_counts_for_an_average() {
    let first = summable_off_count_index("aByAuthorTag", &["postAuthor", "hashtag"]);
    let mut second = summable_off_count_index("bByTagAuthor", &["hashtag", "postAuthor"]);
    second.countable = IndexCountability::Countable;
    let indexes = make_index_map(vec![first, second]);
    let where_clauses = vec![wc_equal("postAuthor"), wc_equal("hashtag")];
    let found =
        find_summable_index_with_counts_for_where_clauses(&indexes, &where_clauses, "byPost", &[]);
    assert_eq!(found.map(|i| i.name.as_str()), Some("bByTagAuthor"));
}

#[test]
fn should_read_a_sum_chain_pin_for_a_sum_but_not_for_an_average_above_the_count_chain() {
    // Ranked by sum at `a` and by average at `b`: the sum chain starts at
    // `a`, the count chain (which an average needs) at `b`.
    let mut index = summable_off_count_index("byABT", &["a", "b", "t"]);
    index.countable = IndexCountability::Countable;
    index.ranked_summable_at = vec!["a".to_string()];
    index.ranked_averageable_at = vec!["b".to_string()];
    let indexes = make_index_map(vec![index]);
    let pinned_a = vec![wc_equal("a")];
    let pinned_a_b = vec![wc_equal("a"), wc_equal("b")];

    assert_eq!(
        find_summable_index_for_where_clauses(&indexes, &pinned_a, "byPost", &[])
            .map(|i| i.name.as_str()),
        Some("byABT"),
        "a sum reads the `a` value tree"
    );
    assert!(
        find_summable_index_with_counts_for_where_clauses(&indexes, &pinned_a, "byPost", &[])
            .is_none(),
        "the `a` value tree carries no count"
    );
    assert_eq!(
        find_summable_index_with_counts_for_where_clauses(&indexes, &pinned_a_b, "byPost", &[])
            .map(|i| i.name.as_str()),
        Some("byABT"),
        "the `b` value tree carries the count"
    );
}

#[test]
fn should_read_a_counter_index_s_last_property_tree_for_a_sum_pinned_above_it() {
    // Unranked and pinned on every property but the last, a sum reads the
    // last property's tree, and an average does when that tree counts
    let mut counting = summable_off_count_index("byAuthorPost", &["postAuthor", "postId"]);
    counting.countable = IndexCountability::Countable;
    counting.range_countable = true;
    let pinned_author = vec![wc_equal("postAuthor")];
    let picks = |index: &Index| {
        let indexes = make_index_map(vec![index.clone()]);
        (
            find_summable_index_for_where_clauses(&indexes, &pinned_author, "byPost", &[])
                .is_some(),
            find_summable_index_with_counts_for_where_clauses(
                &indexes,
                &pinned_author,
                "byPost",
                &[],
            )
            .is_some(),
        )
    };
    assert_eq!(picks(&counting), (true, true), "a counting tree");

    // Without `rangeCountable` the tree sums but counts no posts
    let mut summing = counting.clone();
    summing.range_countable = false;
    assert_eq!(picks(&summing), (true, false), "a summing tree");

    // Ranked by sum alone at `postAuthor`, whose value trees count no posts:
    // the counting last tree still answers the average
    let mut sum_ranked = counting.clone();
    sum_ranked.ranked_summable_at = vec!["postAuthor".to_string()];
    assert_eq!(picks(&sum_ranked), (true, true), "a sum-ranked prefix");

    // A ranked last property is an indexed tree, which is never read whole
    let mut ranked = counting.clone();
    ranked.ranked_summable = true;
    assert_eq!(picks(&ranked), (false, false), "a ranked tree");

    // A regular summable index keeps the exact-cover rule
    let mut regular = range_summable_index("byAuthorPost", &["postAuthor", "postId"], "byPost");
    regular.countable = IndexCountability::Countable;
    regular.range_countable = true;
    assert_eq!(picks(&regular), (false, false), "a regular index");
}

#[test]
fn should_read_a_sum_from_the_index_a_count_with_the_same_pins_reads() {
    // Ranked by sum at `hashtag` and `postId`, the first index's `hashtag`
    // value trees sum their posts, but its last property's tree is ranked, so
    // a count pinned on `postAuthor` and `hashtag` reads the second index's
    // last property's tree; the sum reads that same tree
    let mut chained =
        summable_off_count_index("aByAuthorTagPost", &["postAuthor", "hashtag", "postId"]);
    chained.ranked_summable_at = vec!["hashtag".to_string()];
    chained.ranked_summable = true;
    let plain = summable_off_count_index("bByTagAuthorPost", &["hashtag", "postAuthor", "postId"]);
    let indexes = make_index_map(vec![chained, plain]);
    let pins = vec![wc_equal("postAuthor"), wc_equal("hashtag")];
    assert_eq!(
        DriveDocumentCountQuery::find_countable_index_for_where_clauses(&indexes, &pins, &[])
            .map(|i| i.name.as_str()),
        Some("bByTagAuthorPost"),
        "the count reads the unranked last tree"
    );
    assert_eq!(
        find_summable_index_for_where_clauses(&indexes, &pins, "byPost", &[])
            .map(|i| i.name.as_str()),
        Some("bByTagAuthorPost"),
        "the sum reads the tree the count reads"
    );
}

#[test]
fn should_keep_refusing_a_range_average_whose_first_regular_index_lacks_range_counts() {
    let first = range_summable_index("aByABT", &["a", "b", "t"], "amount");
    let mut second = range_summable_index("bByBAT", &["b", "a", "t"], "amount");
    second.countable = IndexCountability::Countable;
    second.range_countable = true;
    let indexes = make_index_map(vec![first, second]);
    let where_clauses = vec![wc_equal("a"), wc_equal("b"), wc_gt("t", 0)];
    assert!(find_range_summable_index_with_counts_for_where_clauses(
        &indexes,
        &where_clauses,
        "amount",
        &[]
    )
    .is_none());
}

#[test]
fn should_pass_over_a_counter_index_without_range_counts_for_a_range_average() {
    let first = summable_off_count_index("aByAuthorTagPost", &["postAuthor", "hashtag", "postId"]);
    let mut second =
        summable_off_count_index("bByTagAuthorPost", &["hashtag", "postAuthor", "postId"]);
    second.countable = IndexCountability::Countable;
    second.range_countable = true;
    let indexes = make_index_map(vec![first, second]);
    let where_clauses = vec![
        wc_equal("postAuthor"),
        wc_equal("hashtag"),
        wc_gt("postId", 0),
    ];
    let found = find_range_summable_index_with_counts_for_where_clauses(
        &indexes,
        &where_clauses,
        "byPost",
        &[],
    );
    assert_eq!(found.map(|i| i.name.as_str()), Some("bByTagAuthorPost"));
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
        DocumentSumRequest, DocumentSumResponse, DriveDocumentSumQuery, SumEntry, SumMode,
    };
    use crate::query::{WhereClause, WhereOperator};
    use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
    use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
    use crate::util::storage_flags::StorageFlags;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
    use dpp::data_contract::{DataContract, DataContractFactory};
    use dpp::document::{Document, DocumentV0};
    use dpp::identifier::Identifier;
    use dpp::platform_value::{platform_value, Value};
    use dpp::version::PlatformVersion;
    use grovedb::GroveDb;
    use std::borrow::Cow;
    use std::collections::BTreeMap as StdBTreeMap;

    const PROTOCOL_VERSION_V12: u32 = 12;

    /// Build a contract at `protocol_version` with one `widget` doctype
    /// carrying `document_schema`, owned by a fixed identity.
    fn build_widget_contract_with(protocol_version: u32, document_schema: Value) -> DataContract {
        DataContractFactory::new(protocol_version)
            .expect("create factory")
            .create_with_value_config(
                Identifier::from([0xAB; 32]),
                0,
                platform_value!({ "widget": document_schema }),
                None,
                None,
            )
            .expect("create data contract")
            .data_contract_owned()
    }

    /// Build a v12 contract with a `widget` doctype carrying a single
    /// `(color, amount)` `rangeSummable: true` index. The `byColor`
    /// index — `summable: "amount"` + `rangeSummable: true` — is what
    /// the SUM `RangeDistinctProof` arm walks (color = the per-distinct
    /// terminator key, amount = the summed per-doc value).
    fn build_widget_contract() -> DataContract {
        build_widget_contract_with(
            PROTOCOL_VERSION_V12,
            platform_value!({
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
            }),
        )
    }

    /// Insert one widget document at the given `(color, amount)` pair
    /// using the index `(i+1)` as a unique 32-byte id.
    fn insert_widget(drive: &Drive, contract: &DataContract, i: usize, color: &str, amount: u64) {
        insert_widget_with(
            drive,
            contract,
            i,
            StdBTreeMap::from([
                ("color".to_string(), Value::Text(color.to_string())),
                ("amount".to_string(), Value::U64(amount)),
            ]),
            PlatformVersion::latest(),
        );
    }

    /// Insert one widget document with `properties` at `platform_version`,
    /// using the index `(i+1)` as a unique 32-byte id.
    fn insert_widget_with(
        drive: &Drive,
        contract: &DataContract,
        i: usize,
        properties: StdBTreeMap<String, Value>,
        platform_version: &PlatformVersion,
    ) {
        let document_type = contract
            .document_type_for_name("widget")
            .expect("widget type exists");
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
            moderated_at: None,
            moderated_by: None,
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

    /// At the last shipped protocol version, over a regular `[brand, color]`
    /// index, a range sum per `IN` value without a proof still answers a
    /// zero total as one entry, and a sum grouped by the range still leaves
    /// out a group summing to zero (only an index that can hold empty groups,
    /// a preallocated one or one sharing its levels, keeps those).
    #[test]
    fn should_keep_a_zero_in_total_and_drop_zero_groups_at_protocol_version_13() {
        let platform_version = PlatformVersion::get(13).expect("protocol version 13 exists");
        let data_contract = build_widget_contract_with(
            platform_version.protocol_version,
            platform_value!({
                "type": "object",
                "properties": {
                    "brand":  {"type": "string",  "position": 0, "maxLength": 32},
                    "color":  {"type": "string",  "position": 1, "maxLength": 32},
                    "amount": {"type": "integer", "position": 2, "minimum": 0, "maximum": 1000},
                },
                "required": ["brand", "color", "amount"],
                "indices": [{
                    "name": "byBrandColor",
                    "properties": [{"brand": "asc"}, {"color": "asc"}],
                    "summable":      "amount",
                    "rangeSummable": true,
                }],
                "additionalProperties": false,
            }),
        );
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
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
        let document_type = data_contract
            .document_type_for_name("widget")
            .expect("widget");
        for (i, (brand, color, amount)) in [
            ("acme", "blue", 0u64),
            ("acme", "red", 5),
            ("contoso", "red", 7),
        ]
        .into_iter()
        .enumerate()
        {
            insert_widget_with(
                &drive,
                &data_contract,
                i,
                StdBTreeMap::from([
                    ("brand".to_string(), Value::Text(brand.to_string())),
                    ("color".to_string(), Value::Text(color.to_string())),
                    ("amount".to_string(), Value::U64(amount)),
                ]),
                platform_version,
            );
        }

        let drive_config = DriveConfig::default();
        let entries = |where_clauses: Vec<WhereClause>, mode: SumMode| match drive
            .execute_document_sum_request(
                DocumentSumRequest {
                    contract: &data_contract,
                    document_type,
                    sum_property: "amount".to_string(),
                    where_clauses,
                    order_clauses: Vec::new(),
                    mode,
                    limit: None,
                    prove: false,
                    drive_config: &drive_config,
                    resolved_time_ranges: vec![],
                },
                None,
                platform_version,
            )
            .expect("the sum executes at protocol version 13")
        {
            DocumentSumResponse::Entries(entries) => entries,
            other => panic!("expected entries, got {other:?}"),
        };
        let clause = |field: &str, operator: WhereOperator, value: Value| WhereClause {
            field: field.to_string(),
            operator,
            value,
        };

        assert_eq!(
            entries(
                vec![
                    clause(
                        "brand",
                        WhereOperator::In,
                        Value::Array(vec![
                            Value::Text("acme".to_string()),
                            Value::Text("contoso".to_string()),
                        ]),
                    ),
                    clause(
                        "color",
                        WhereOperator::GreaterThan,
                        Value::Text("zzz".to_string()),
                    ),
                ],
                SumMode::GroupByIn,
            ),
            vec![SumEntry {
                in_key: None,
                key: Vec::new(),
                sum: Some(0),
            }],
            "a zero total per IN value is one entry"
        );
        assert_eq!(
            entries(
                vec![
                    clause(
                        "brand",
                        WhereOperator::Equal,
                        Value::Text("acme".to_string())
                    ),
                    clause(
                        "color",
                        WhereOperator::GreaterThan,
                        Value::Text("a".to_string()),
                    ),
                ],
                SumMode::GroupByRange,
            )
            .into_iter()
            .map(|entry| (entry.key, entry.sum))
            .collect::<Vec<_>>(),
            vec![(b"red".to_vec(), Some(5))],
            "acme's blue sums to zero and is left out"
        );
    }

    /// The average and sum point verifiers are edited in place to look
    /// through a wrapped element; at the last shipped protocol version, over a
    /// regular `[brand, color]` index counting and summing, no element they
    /// read is wrapped, so an exact and a per-`IN` average and sum prove and
    /// verify against the live root to the unproven answer.
    #[test]
    fn should_verify_an_average_point_proof_unchanged_at_protocol_version_13() {
        use crate::query::drive_document_average_query::{
            AverageEntry, AverageMode, DocumentAverageRequest, DocumentAverageResponse,
        };
        use crate::query::drive_document_sum_query::index_picker::find_summable_index_with_counts_for_where_clauses;

        let platform_version = PlatformVersion::get(13).expect("protocol version 13 exists");
        let data_contract = build_widget_contract_with(
            platform_version.protocol_version,
            platform_value!({
                "type": "object",
                "properties": {
                    "brand":  {"type": "string",  "position": 0, "maxLength": 32},
                    "color":  {"type": "string",  "position": 1, "maxLength": 32},
                    "amount": {"type": "integer", "position": 2, "minimum": 0, "maximum": 1000},
                },
                "required": ["brand", "color", "amount"],
                "indices": [{
                    "name": "byBrandColor",
                    "properties": [{"brand": "asc"}, {"color": "asc"}],
                    "countable": "countable",
                    "summable":  "amount",
                }],
                "additionalProperties": false,
            }),
        );
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
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
        let document_type = data_contract
            .document_type_for_name("widget")
            .expect("widget");
        for (i, (brand, color, amount)) in [
            ("acme", "red", 5u64),
            ("acme", "red", 7),
            ("acme", "blue", 1),
            ("contoso", "red", 4),
        ]
        .into_iter()
        .enumerate()
        {
            insert_widget_with(
                &drive,
                &data_contract,
                i,
                StdBTreeMap::from([
                    ("brand".to_string(), Value::Text(brand.to_string())),
                    ("color".to_string(), Value::Text(color.to_string())),
                    ("amount".to_string(), Value::U64(amount)),
                ]),
                platform_version,
            );
        }
        let live_root = drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .expect("root hash must be readable");

        let drive_config = DriveConfig::default();
        let average = |where_clauses: &[WhereClause], mode: AverageMode, prove: bool| {
            drive
                .execute_document_average_request(
                    DocumentAverageRequest {
                        contract: &data_contract,
                        document_type,
                        sum_property: "amount".to_string(),
                        where_clauses: where_clauses.to_vec(),
                        resolved_time_ranges: vec![],
                        order_clauses: Vec::new(),
                        mode,
                        limit: None,
                        prove,
                        drive_config: &drive_config,
                    },
                    None,
                    platform_version,
                )
                .expect("the average executes at protocol version 13")
        };
        let text = |value: &str| Value::Text(value.to_string());
        let clause = |field: &str, operator: WhereOperator, value: Value| WhereClause {
            field: field.to_string(),
            operator,
            value,
        };
        let red = clause("color", WhereOperator::Equal, text("red"));
        let exact = vec![
            clause("brand", WhereOperator::Equal, text("acme")),
            red.clone(),
        ];
        let per_brand = vec![
            clause(
                "brand",
                WhereOperator::In,
                Value::Array(vec![text("acme"), text("contoso")]),
            ),
            red,
        ];

        match average(&exact, AverageMode::Aggregate, false) {
            DocumentAverageResponse::Aggregate { count, sum } => {
                assert_eq!((count, sum), (2, 12), "acme's red widgets, unproved")
            }
            other => panic!("expected an aggregate average, got {other:?}"),
        }
        for (where_clauses, mode, expected) in [
            (
                exact,
                AverageMode::Aggregate,
                vec![(Vec::new(), Some(2), Some(12))],
            ),
            (
                per_brand,
                AverageMode::GroupByIn,
                vec![
                    (b"acme".to_vec(), Some(2), Some(12)),
                    (b"contoso".to_vec(), Some(1), Some(4)),
                ],
            ),
        ] {
            let proof = match average(&where_clauses, mode, true) {
                DocumentAverageResponse::Proof(proof) => proof,
                other => panic!("expected a proof, got {other:?}"),
            };
            let index = find_summable_index_with_counts_for_where_clauses(
                document_type.indexes(),
                &where_clauses,
                "amount",
                &[],
            )
            .expect("byBrandColor answers the average");
            let (root_hash, entries) = DriveDocumentSumQuery {
                document_type,
                contract_id: data_contract.id().to_buffer(),
                document_type_name: "widget".to_string(),
                index,
                where_clauses: where_clauses.clone(),
                sum_property: "amount".to_string(),
            }
            .verify_point_lookup_count_and_sum_proof(&proof, platform_version)
            .expect("the average proof verifies at protocol version 13");
            assert_eq!(root_hash, live_root, "{where_clauses:?}");
            assert_eq!(
                entries
                    .into_iter()
                    .map(
                        |AverageEntry {
                             key, count, sum, ..
                         }| (key, count, sum)
                    )
                    .collect::<Vec<_>>(),
                expected,
                "{where_clauses:?}"
            );

            // The point sum verifier, edited in place the same way, reads the
            // same sums
            let sum_mode = match mode {
                AverageMode::GroupByIn => SumMode::GroupByIn,
                _ => SumMode::Aggregate,
            };
            let proof = match drive
                .execute_document_sum_request(
                    DocumentSumRequest {
                        contract: &data_contract,
                        document_type,
                        sum_property: "amount".to_string(),
                        where_clauses: where_clauses.clone(),
                        order_clauses: Vec::new(),
                        mode: sum_mode,
                        limit: None,
                        prove: true,
                        drive_config: &drive_config,
                        resolved_time_ranges: vec![],
                    },
                    None,
                    platform_version,
                )
                .expect("the sum proves at protocol version 13")
            {
                DocumentSumResponse::Proof(proof) => proof,
                other => panic!("expected a proof, got {other:?}"),
            };
            let (root_hash, sums) = DriveDocumentSumQuery {
                document_type,
                contract_id: data_contract.id().to_buffer(),
                document_type_name: "widget".to_string(),
                index,
                where_clauses: where_clauses.clone(),
                sum_property: "amount".to_string(),
            }
            .verify_point_lookup_sum_proof(&proof, platform_version)
            .expect("the sum proof verifies at protocol version 13");
            assert_eq!(root_hash, live_root, "{where_clauses:?}");
            assert_eq!(
                sums.into_iter()
                    .map(|entry| (entry.key, entry.sum))
                    .collect::<Vec<_>>(),
                expected
                    .iter()
                    .map(|(key, _, sum)| (key.clone(), *sum))
                    .collect::<Vec<_>>(),
                "{where_clauses:?}"
            );
        }
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
