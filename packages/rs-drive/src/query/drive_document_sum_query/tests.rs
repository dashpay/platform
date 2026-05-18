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
    let found = find_summable_index_for_where_clauses(&indexes, &[wc_equal("recipient")], "amount");
    assert_eq!(found.map(|i| i.name.as_str()), Some("byRecipient"));
}

#[test]
fn summable_picker_rejects_partial_coverage() {
    // Two-prop index with only one of the props matched by where clauses.
    let indexes = make_index_map(vec![summable_index("byAB", &["a", "b"], Some("amount"))]);
    assert!(
        find_summable_index_for_where_clauses(&indexes, &[wc_equal("a")], "amount").is_none(),
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
        find_summable_index_for_where_clauses(&indexes, &[wc_equal("recipient")], "fee").is_none()
    );
}

#[test]
fn summable_picker_rejects_non_summable_index() {
    // No `summable` declaration → never picked, even if properties match.
    let indexes = make_index_map(vec![summable_index("byRecipient", &["recipient"], None)]);
    assert!(
        find_summable_index_for_where_clauses(&indexes, &[wc_equal("recipient")], "amount")
            .is_none()
    );
}

#[test]
fn summable_picker_rejects_range_operator() {
    let indexes = make_index_map(vec![summable_index(
        "bySentAt",
        &["sentAt"],
        Some("amount"),
    )]);
    assert!(
        find_summable_index_for_where_clauses(&indexes, &[wc_gt("sentAt", 0)], "amount").is_none(),
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
    let found = find_summable_index_for_where_clauses(&indexes, &[wc_in("recipient")], "amount");
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
        find_range_summable_index_for_where_clauses(&indexes, &[wc_gt("sentAt", 0)], "amount");
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
    let found = find_range_summable_index_for_where_clauses(&indexes, &where_clauses, "amount");
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
    assert!(
        find_range_summable_index_for_where_clauses(&indexes, &[wc_gt("sentAt", 0)], "fee")
            .is_none()
    );
}

#[test]
fn range_summable_picker_rejects_non_range_summable() {
    // summable but not rangeSummable — the point-lookup picker would
    // accept this; the range picker must not.
    let mut idx = range_summable_index("bySentAt", &["sentAt"], "amount");
    idx.range_summable = false;
    let indexes = make_index_map(vec![idx]);
    assert!(
        find_range_summable_index_for_where_clauses(&indexes, &[wc_gt("sentAt", 0)], "amount")
            .is_none()
    );
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
        find_range_summable_index_for_where_clauses(&indexes, &where_clauses, "amount").is_none()
    );
}
