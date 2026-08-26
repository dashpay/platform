//! Index pickers for the count query.
//!
//! Pure functions on the document type's index map + where clauses;
//! no Drive, no proof. Picks a covering index for a given query
//! shape, returning `None` if no index can serve the query.

use super::super::conditions::WhereClause;
use super::DriveDocumentCountQuery;
use crate::query::index_admissible_for_resolved_time_range;
use crate::query::ResolvedTimeRange;
use dpp::data_contract::document_type::Index;
use std::collections::{BTreeMap, BTreeSet};

impl DriveDocumentCountQuery<'_> {
    /// Finds a `countable: true` index whose properties **exactly match** the
    /// indexable (Equal/In) where-clause fields — every index property has a
    /// corresponding clause AND every clause's field appears in the index.
    ///
    /// Exact coverage is the contract for both no-proof and prove count
    /// paths: a countable index counts exactly what it indexes, and queries
    /// against partially-covered indexes are rejected with a clear error
    /// directing the caller at the index-design fix. This avoids the
    /// product-of-uncovered-branching-factors walk that a prefix-match
    /// approach would silently fall through to, and keeps the storage's
    /// "count maintained only at the terminal level" trade-off intact (no
    /// need to maintain counts at intermediate index levels just to serve
    /// partial-coverage queries cheaply).
    ///
    /// Returns `None` if:
    /// - Any where clause uses an operator other than `Equal` / `In`.
    /// - The set of indexable where-clause fields doesn't exactly equal the
    ///   set of properties of any single `countable: true` index.
    ///
    /// For the `documents_countable: true` case (total count with no where
    /// clauses), the dispatcher reads the document-type primary-key tree's
    /// CountTree directly — that path doesn't use this picker because no
    /// index is involved.
    ///
    /// `resolved_time_ranges` names the fields whose equality clause was
    /// produced by `IN_TIME_RANGE` resolution (see
    /// [`crate::query::resolve_time_range_bucket_clause`]); it gates which
    /// indexes are candidates at all — see
    /// [`index_admissible_for_resolved_time_range`].
    pub fn find_countable_index_for_where_clauses<'b>(
        indexes: &'b BTreeMap<String, Index>,
        where_clauses: &[WhereClause],
        resolved_time_ranges: &[ResolvedTimeRange],
    ) -> Option<&'b Index> {
        if Self::has_unsupported_operator(where_clauses) {
            return None;
        }

        let indexable_fields: BTreeSet<&str> = where_clauses
            .iter()
            .filter(|wc| Self::is_indexable_for_count(wc.operator))
            .map(|wc| wc.field.as_str())
            .collect();

        // Need a clause for every property of the index, so empty
        // `indexable_fields` only matches an empty-properties index
        // (which doesn't exist — indexes always have at least one
        // property — so empty where clauses never match here).
        if indexable_fields.is_empty() {
            return None;
        }

        for index in indexes.values() {
            // A time-range index holds one entry per bucket containing the
            // document, keyed by bucket start: counting over it multi-counts
            // every document unless the query pins a single bucket, and only
            // a resolution-produced equality does that. Conversely a raw
            // clause must never bind to bucket keys.
            if !index_admissible_for_resolved_time_range(index, resolved_time_ranges) {
                continue;
            }
            if !index.countable.is_countable() {
                continue;
            }
            if index.properties.len() != indexable_fields.len() {
                continue;
            }
            // Every index property must have a matching where-clause
            // field. Because lengths match, this also implies every
            // where-clause field appears in the index (no orphan
            // clauses).
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

    /// Finds a `range_countable` index that can serve a range-count query.
    ///
    /// Match criteria:
    /// - All `Equal`/`In` where-clause fields form a prefix of the index
    ///   properties.
    /// - There is exactly one range-operator where-clause, on a property
    ///   that is the *last* property of the index (the IndexLevel
    ///   terminator). This is the property whose values get walked.
    /// - The index has `range_countable = true` and `countable.is_countable()`.
    ///
    /// Returns `None` if no such index exists or if there's more than one
    /// range operator in the where clauses (which would require nested range
    /// walks the current model doesn't support). Pure point-lookup queries
    /// (no range operator) should fall back to
    /// [`Self::find_countable_index_for_where_clauses`].
    ///
    /// `resolved_time_ranges` gates the candidate set exactly as in
    /// [`Self::find_countable_index_for_where_clauses`]. A resolved field
    /// never arrives as a range clause — resolution always produces an
    /// equality — so with a non-empty list the only bucketed index this can
    /// return is one whose resolved equality is a prefix property and whose
    /// range terminator is a different property. That is the intended shape:
    /// a range over one property within a single time bucket.
    pub fn find_range_countable_index_for_where_clauses<'b>(
        indexes: &'b BTreeMap<String, Index>,
        where_clauses: &[WhereClause],
        resolved_time_ranges: &[ResolvedTimeRange],
    ) -> Option<&'b Index> {
        let range_clauses: Vec<&WhereClause> = where_clauses
            .iter()
            .filter(|wc| Self::is_range_operator(wc.operator))
            .collect();
        // Accept either:
        // - 1 range clause (Q7 / G4 / G5 / G7 — the range is the
        //   terminator; prefix props use `==` or `In`).
        // - 2 range clauses on distinct fields (G8 — outer range on
        //   an index prefix property, inner range on the terminator;
        //   the carrier-aggregate proof shape introduced by grovedb
        //   PR #664).
        let (outer_range_field, terminator_range_clause) = match range_clauses.len() {
            1 => (None, range_clauses[0]),
            2 => {
                // The two ranges must be on different fields — same-
                // field two-sided ranges are flattened by the parser
                // into `between*` and arrive with one clause.
                if range_clauses[0].field == range_clauses[1].field {
                    return None;
                }
                // One of the two must be on an index's terminator; the
                // other becomes the outer carrier dimension. We pick
                // the terminator below by walking each candidate
                // index's property order — defer choosing here.
                (
                    Some((
                        range_clauses[0].field.as_str(),
                        range_clauses[1].field.as_str(),
                    )),
                    range_clauses[0], // placeholder, refined per-index below
                )
            }
            _ => return None,
        };

        // Reject any operator that's neither indexable (Equal/In) nor a
        // range operator — anything else has no defined count semantics.
        if where_clauses.iter().any(|wc| {
            !Self::is_indexable_for_count(wc.operator) && !Self::is_range_operator(wc.operator)
        }) {
            return None;
        }

        let prefix_fields: BTreeSet<&str> = where_clauses
            .iter()
            .filter(|wc| Self::is_indexable_for_count(wc.operator))
            .map(|wc| wc.field.as_str())
            .collect();

        for index in indexes.values() {
            // Same admissibility rule as the point-lookup picker: bucketed
            // indexes store one entry per containing bucket, so only a query
            // pinned to a single bucket by a resolution-produced equality may
            // walk them, and raw clauses may never bind to bucket keys.
            if !index_admissible_for_resolved_time_range(index, resolved_time_ranges) {
                continue;
            }
            if !index.range_countable || !index.countable.is_countable() {
                continue;
            }

            // For the two-range case, the terminator's field must be
            // one of the two range fields, and the other range field
            // must be the index's first property (the carrier
            // dimension).
            if let Some((field_a, field_b)) = outer_range_field {
                let terminator = index.properties.last()?;
                let first = index.properties.first()?;
                // Determine which range field is the terminator.
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
                // Any Equal/In prefix clauses must sit between the
                // first (outer-range) and last (terminator-range)
                // properties. For the widget contract there are no
                // such middle properties on byBrandColor, but the
                // builder handles the general case.
                let intermediate_props = &index.properties[1..index.properties.len() - 1];
                let mut intermediate_props_ok = true;
                for prop in intermediate_props {
                    if !prefix_fields.contains(prop.name.as_str()) {
                        intermediate_props_ok = false;
                        break;
                    }
                }
                // Strict-coverage check, mirroring sum's picker: every
                // Equal/In prefix field must appear in the index's
                // intermediate properties. Without this
                // `intermediate_props.len() == prefix_fields.len()` guard,
                // a query with extra prefix fields would silently pick an
                // index that *doesn't* cover them — the carrier path-query
                // builder iterates only index properties, so the uncovered
                // clause would simply be dropped and the per-group counts
                // would span all its values (an over-broad result that
                // even verifies, since the verifier rebuilds the same
                // path query from the same picker).
                if intermediate_props_ok && intermediate_props.len() == prefix_fields.len() {
                    return Some(index);
                }
                continue;
            }

            // Single-range case (the original logic): prefix matches
            // must come first, followed by the range property as the
            // LAST element.
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
                // Range property must be the terminator (last property).
                continue;
            }
            let range_prop = &index.properties[prefix_len];
            if range_prop.name == terminator_range_clause.field {
                return Some(index);
            }
        }

        None
    }
}
