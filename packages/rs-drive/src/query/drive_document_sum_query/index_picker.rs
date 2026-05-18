//! Sum-index pickers. Parallels count's `index_picker.rs`.
//!
//! Two strict-coverage pickers:
//! - [`find_summable_index_for_where_clauses`]: returns the `summable: "<prop>"`
//!   index whose properties *exactly* match the Equal/In where-clause fields
//!   and whose summed property equals the request's `sum_property`. None on
//!   miss (no partial coverage).
//! - [`find_range_summable_index_for_where_clauses`]: returns the
//!   `rangeSummable: true` index whose Equal/In prefix covers the
//!   non-range clauses AND whose last property is the range
//!   terminator. None on miss.
//!
//! Reject-on-miss is the load-bearing contract: callers landing in
//! "no covering index" return `WhereClauseOnNonIndexedProperty` so the
//! prover and verifier reject the same set of inputs (same as count).

use crate::query::drive_document_sum_query::{is_indexable_for_sum, is_range_operator};
use crate::query::{WhereClause, WhereOperator};
use dpp::data_contract::document_type::Index;
use std::collections::{BTreeMap, BTreeSet};

/// Find a `summable: "<prop>"` index whose properties exactly cover
/// the Equal/In where-clause fields AND whose summed property name
/// equals the request's `sum_property`.
///
/// Mirror of count's `find_countable_index_for_where_clauses` with the
/// additional `summable == Some(sum_property)` predicate on top of the
/// strict-coverage match.
pub fn find_summable_index_for_where_clauses<'b>(
    indexes: &'b BTreeMap<String, Index>,
    where_clauses: &[WhereClause],
    sum_property: &str,
) -> Option<&'b Index> {
    // Defense-in-depth: any non-indexable operator immediately disqualifies
    // — the sum point-lookup path can only serve Equal/In.
    if where_clauses
        .iter()
        .any(|wc| !is_indexable_for_sum(wc.operator))
    {
        return None;
    }

    let indexable_fields: BTreeSet<&str> = where_clauses
        .iter()
        .filter(|wc| matches!(wc.operator, WhereOperator::Equal | WhereOperator::In))
        .map(|wc| wc.field.as_str())
        .collect();

    if indexable_fields.is_empty() {
        return None;
    }

    for index in indexes.values() {
        // Skip if not summable OR if summable property doesn't match.
        match &index.summable {
            Some(prop) if prop == sum_property => {}
            _ => continue,
        }
        if index.properties.len() != indexable_fields.len() {
            continue;
        }
        let all_covered = index
            .properties
            .iter()
            .all(|prop| indexable_fields.contains(prop.name.as_str()));
        if all_covered {
            return Some(index);
        }
    }

    None
}

/// Find a `rangeSummable: true` index whose properties cover the
/// non-range Equal/In clauses as a prefix AND whose last property is
/// the range terminator. The summed property must match
/// `sum_property`.
///
/// Mirror of count's `find_range_countable_index_for_where_clauses`.
pub fn find_range_summable_index_for_where_clauses<'b>(
    indexes: &'b BTreeMap<String, Index>,
    where_clauses: &[WhereClause],
    sum_property: &str,
) -> Option<&'b Index> {
    let range_clauses: Vec<&WhereClause> = where_clauses
        .iter()
        .filter(|wc| is_range_operator(wc.operator))
        .collect();
    let (outer_range_field, terminator_range_clause) = match range_clauses.len() {
        1 => (None, range_clauses[0]),
        2 => {
            // Same-field two-sided ranges are flattened into `between*`
            // and arrive as one clause; reject if same-field anyway.
            if range_clauses[0].field == range_clauses[1].field {
                return None;
            }
            (
                Some((
                    range_clauses[0].field.as_str(),
                    range_clauses[1].field.as_str(),
                )),
                range_clauses[0],
            )
        }
        _ => return None,
    };

    // Reject any operator that's neither indexable (Equal/In) nor a
    // range operator — anything else has no defined sum semantics.
    if where_clauses
        .iter()
        .any(|wc| !is_indexable_for_sum(wc.operator) && !is_range_operator(wc.operator))
    {
        return None;
    }

    let prefix_fields: BTreeSet<&str> = where_clauses
        .iter()
        .filter(|wc| matches!(wc.operator, WhereOperator::Equal | WhereOperator::In))
        .map(|wc| wc.field.as_str())
        .collect();

    for index in indexes.values() {
        if !index.range_summable {
            continue;
        }
        // `range_summable: true` requires `summable: Some(_)` per the DPP
        // schema; verify it matches the caller's sum_property.
        match &index.summable {
            Some(prop) if prop == sum_property => {}
            _ => continue,
        }

        if let Some((field_a, field_b)) = outer_range_field {
            let terminator = index.properties.last()?;
            let first = index.properties.first()?;
            let (outer_field, _terminator_field) = if terminator.name == field_a {
                (field_b, field_a)
            } else if terminator.name == field_b {
                (field_a, field_b)
            } else {
                continue;
            };
            if first.name != outer_field {
                continue;
            }
            let intermediate_props = &index.properties[1..index.properties.len() - 1];
            let mut intermediate_props_ok = true;
            for prop in intermediate_props {
                if !prefix_fields.contains(prop.name.as_str()) {
                    intermediate_props_ok = false;
                    break;
                }
            }
            // Strict-coverage check: every Equal/In prefix field must
            // appear in the index's intermediate properties. Without
            // this `intermediate_props.len() == prefix_fields.len()`
            // guard, a query with extra prefix fields would silently
            // pick an index that *doesn't* cover them, producing an
            // over-broad result.
            if intermediate_props_ok && intermediate_props.len() == prefix_fields.len() {
                return Some(index);
            }
            continue;
        }

        // Single-range case.
        let mut prefix_len = 0usize;
        for prop in &index.properties {
            if prefix_fields.contains(prop.name.as_str()) {
                prefix_len += 1;
            } else {
                break;
            }
        }
        if prefix_len < prefix_fields.len() {
            continue;
        }
        if prefix_len + 1 != index.properties.len() {
            continue;
        }
        let range_prop = &index.properties[prefix_len];
        if range_prop.name == terminator_range_clause.field {
            return Some(index);
        }
    }

    None
}
