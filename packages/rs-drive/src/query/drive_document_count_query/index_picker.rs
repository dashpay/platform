//! Index pickers for the count query.
//!
//! Pure functions on the document type's index map + where clauses;
//! no Drive, no proof. Picks a covering index for a given query
//! shape, returning `None` if no index can serve the query.

use super::super::conditions::WhereClause;
use super::DriveDocumentCountQuery;
use dpp::data_contract::document_type::Index;
use std::collections::{BTreeMap, BTreeSet};

impl DriveDocumentCountQuery<'_> {
    /// Finds a countable index whose properties form a prefix that matches the
    /// indexable (Equal / In) where-clause fields. For a count query:
    /// - All indexable where-clause fields must appear as a prefix of the index properties
    /// - The index must have `countable = true`
    /// - Returns `None` if any where clause uses an operator other than `Equal` / `In`
    /// - Among matching indexes, we prefer the one with the most properties
    ///   matched by where clauses (most specific)
    pub fn find_countable_index_for_where_clauses<'b>(
        indexes: &'b BTreeMap<String, Index>,
        where_clauses: &[WhereClause],
    ) -> Option<&'b Index> {
        if Self::has_unsupported_operator(where_clauses) {
            return None;
        }

        let indexable_fields: BTreeSet<&str> = where_clauses
            .iter()
            .filter(|wc| Self::is_indexable_for_count(wc.operator))
            .map(|wc| wc.field.as_str())
            .collect();

        let mut best_match: Option<(&Index, usize)> = None;

        for index in indexes.values() {
            if !index.countable.is_countable() {
                continue;
            }

            // Check that the indexable where-clause fields form a prefix of
            // the index properties.
            let mut prefix_len = 0;
            for prop in &index.properties {
                if indexable_fields.contains(prop.name.as_str()) {
                    prefix_len += 1;
                } else {
                    break;
                }
            }

            // All indexable where-clause fields must be consumed as a prefix.
            if prefix_len < indexable_fields.len() {
                continue;
            }

            // Prefer the index with the longest matching prefix (most specific).
            match &best_match {
                None => best_match = Some((index, prefix_len)),
                Some((_, best_len)) if prefix_len > *best_len => {
                    best_match = Some((index, prefix_len));
                }
                _ => {}
            }
        }

        best_match.map(|(index, _)| index)
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
    pub fn find_range_countable_index_for_where_clauses<'b>(
        indexes: &'b BTreeMap<String, Index>,
        where_clauses: &[WhereClause],
    ) -> Option<&'b Index> {
        let range_clauses: Vec<&WhereClause> = where_clauses
            .iter()
            .filter(|wc| Self::is_range_operator(wc.operator))
            .collect();
        if range_clauses.len() != 1 {
            return None;
        }
        let range_clause = range_clauses[0];

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
            if !index.range_countable || !index.countable.is_countable() {
                continue;
            }

            // Walk the index properties: prefix matches must come first,
            // followed by the range property as the LAST element.
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
            if range_prop.name == range_clause.field {
                return Some(index);
            }
        }

        None
    }
}
