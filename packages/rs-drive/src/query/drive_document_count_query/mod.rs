use std::collections::{BTreeMap, BTreeSet};

#[cfg(feature = "server")]
use crate::drive::Drive;
// `QuerySyntaxError` is reachable under both `server` and `verify`
// because [`DriveDocumentCountQuery::detect_mode`] (pure where-clause
// validation, no Drive) is callable in either context.
#[cfg(any(feature = "server", feature = "verify"))]
use crate::error::query::QuerySyntaxError;
// `Error` is needed by the path-builder helpers shared between the
// server prove path and the SDK proof verifier.
#[cfg(any(feature = "server", feature = "verify"))]
use crate::error::Error;
#[cfg(feature = "server")]
use crate::util::grove_operations::DirectQueryType;
#[cfg(feature = "server")]
use dpp::version::drive_versions::DriveVersion;
#[cfg(feature = "server")]
use grovedb::query_result_type::QueryResultType;
#[cfg(feature = "server")]
use grovedb::{Query, SizedQuery, TransactionArg};
// `PathQuery` + `QueryItem` are needed by `aggregate_count_path_query`,
// which is shared between the server prove path and the SDK proof
// verifier (compiled under `verify`).
#[cfg(any(feature = "server", feature = "verify"))]
use grovedb::{PathQuery, QueryItem};
#[cfg(feature = "server")]
use grovedb_path::SubtreePath;

// `RootTree` is the index path's first byte. Available under both
// gates so the verifier can reconstruct the same path the prover built.
#[cfg(any(feature = "server", feature = "verify"))]
use crate::drive::RootTree;
#[cfg(any(feature = "server", feature = "verify"))]
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
#[cfg(any(feature = "server", feature = "verify"))]
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
#[cfg(feature = "server")]
use dpp::data_contract::document_type::IndexProperty;
use dpp::data_contract::document_type::{DocumentTypeRef, Index};
#[cfg(any(feature = "server", feature = "verify"))]
use dpp::version::PlatformVersion;

use super::conditions::{WhereClause, WhereOperator};

#[cfg(feature = "server")]
#[cfg(test)]
mod tests;

/// A query to count documents using CountTree elements in the index path.
///
/// This struct encapsulates all the information needed to perform a count
/// query on a document type's countable index, including optional split-by
/// functionality for getting per-value counts.
#[derive(Debug, Clone)]
pub struct DriveDocumentCountQuery<'a> {
    /// The document type to count
    pub document_type: DocumentTypeRef<'a>,
    /// The contract id (32 bytes)
    pub contract_id: [u8; 32],
    /// The document type name
    pub document_type_name: String,
    /// The countable index to use
    pub index: &'a Index,
    /// The equality where clauses that match index prefix properties
    pub where_clauses: Vec<WhereClause>,
    /// Optional property to split counts by. When set, returns per-value
    /// counts for this property instead of a single total count.
    pub split_by_property: Option<String>,
}

/// An entry in a split count result, containing the serialized key
/// and the count of documents matching that key value.
#[derive(Debug, Clone, PartialEq)]
pub struct SplitCountEntry {
    /// The serialized key bytes for this value
    pub key: Vec<u8>,
    /// The count of documents matching this key value
    pub count: u64,
}

/// Classification of a count query's shape, used to dispatch to the
/// right executor. Returned by
/// [`DriveDocumentCountQuery::detect_mode`].
///
/// The discriminator is purely a function of the where-clause operators
/// + request flags (`return_distinct_counts_in_range`, `prove`); it
/// does not depend on the contract's index set. Picking a covering
/// index for the chosen mode is a separate step that requires the
/// document type's `BTreeMap<String, Index>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentCountMode {
    /// No range, no `In` — single summed entry with empty key. Reads
    /// the `CountTree` count directly at the indexed path.
    Total,
    /// Exactly one `In` clause, no range — one entry per (deduped)
    /// `In` value, each computed as the count at that single value.
    /// The `In` doubles as the per-value split signal.
    PerInValue,
    /// Exactly one range clause, no proof — walks the property-name
    /// `ProvableCountTree`'s children inside the range. Returns either
    /// a single summed entry or per-distinct-value entries depending on
    /// `return_distinct_counts_in_range`.
    RangeNoProof,
    /// Exactly one range clause + `prove = true` — produces a grovedb
    /// `AggregateCountOnRange` proof that verifies to a single u64.
    /// `return_distinct_counts_in_range = true` is rejected here
    /// because the merk-level primitive returns one aggregate.
    RangeProof,
    /// No range clause + `prove = true` — falls back to the
    /// materialize-and-count proof path. Capped at `u16::MAX` matching
    /// docs because each verified document is materialized client-side.
    PointLookupProof,
}

impl<'a> DriveDocumentCountQuery<'a> {
    /// Returns `true` if the where-clause operator is one the count fast path
    /// can serve via point-lookups in a CountTree.
    ///
    /// Today that's `Equal` (one path) and `In` (cartesian fork over the listed
    /// values). Range operators (`>`, `<`, `Between*`, `StartsWith`) need a
    /// boundary walk that the current PathQuery infrastructure cannot express;
    /// callers detect those via [`Self::has_unsupported_operator`] and surface
    /// an error instead of silently returning a wrong count.
    fn is_indexable_for_count(op: WhereOperator) -> bool {
        matches!(op, WhereOperator::Equal | WhereOperator::In)
    }

    /// Returns `true` if `op` is a range operator that can be served by a
    /// `range_countable` index walking the property-name `ProvableCountTree`'s
    /// children. The non-prefix portion of a range count query carries
    /// exactly one range operator on the index's last property.
    pub fn is_range_operator(op: WhereOperator) -> bool {
        matches!(
            op,
            WhereOperator::GreaterThan
                | WhereOperator::GreaterThanOrEquals
                | WhereOperator::LessThan
                | WhereOperator::LessThanOrEquals
                | WhereOperator::Between
                | WhereOperator::BetweenExcludeBounds
                | WhereOperator::BetweenExcludeLeft
                | WhereOperator::BetweenExcludeRight
                | WhereOperator::StartsWith
        )
    }

    /// Returns `true` if any where clause uses an operator the count fast path
    /// cannot serve. Callers should treat this as a query-rejection signal.
    pub fn has_unsupported_operator(where_clauses: &[WhereClause]) -> bool {
        where_clauses
            .iter()
            .any(|wc| !Self::is_indexable_for_count(wc.operator))
    }

    /// Classify a count query's mode from its where clauses + request flags.
    ///
    /// This is the protocol-version-agnostic shape detection that decides
    /// which executor (Equal/In point lookup, range walk, range proof,
    /// materialize-and-count proof, etc.) the request maps to. The
    /// returned [`DocumentCountMode`] discriminates among the handler's
    /// dispatch arms; concrete pagination / index-picker inputs still
    /// flow through the call sites separately.
    ///
    /// All validation that depends only on the where clauses + flags
    /// (multiple range clauses, range mixed with `In`, distinct mode on
    /// the prove path, distinct mode without a range clause, etc.) is
    /// done here and surfaces as
    /// [`QuerySyntaxError::InvalidWhereClauseComponents`]. Validation
    /// that depends on the contract's index set (no covering index)
    /// stays at the call site since it requires the
    /// `&BTreeMap<String, Index>`.
    pub fn detect_mode(
        where_clauses: &[WhereClause],
        return_distinct_counts_in_range: bool,
        prove: bool,
    ) -> Result<DocumentCountMode, QuerySyntaxError> {
        // Reject any operator that's neither an indexable point operator
        // (Equal/In) nor a range operator. Defense-in-depth: the request
        // shape forbids these elsewhere, but folding the check in here
        // keeps the mode-detection contract self-contained.
        for wc in where_clauses {
            if !Self::is_indexable_for_count(wc.operator) && !Self::is_range_operator(wc.operator) {
                return Err(QuerySyntaxError::InvalidWhereClauseComponents(
                    "count query supports only `==`, `in`, and range operators",
                ));
            }
        }

        let range_count = where_clauses
            .iter()
            .filter(|wc| Self::is_range_operator(wc.operator))
            .count();
        let in_count = where_clauses
            .iter()
            .filter(|wc| wc.operator == WhereOperator::In)
            .count();

        if range_count > 1 {
            return Err(QuerySyntaxError::InvalidWhereClauseComponents(
                "count query supports at most one range where-clause; combine \
                 two-sided ranges via `between*` instead of separate `>` / `<` clauses",
            ));
        }
        if in_count > 1 {
            return Err(QuerySyntaxError::InvalidWhereClauseComponents(
                "count query supports at most one `in` where-clause; the In carries \
                 the split property and only one split dimension is supported per request",
            ));
        }

        let has_range = range_count == 1;
        let has_in = in_count == 1;

        if has_range && has_in {
            return Err(QuerySyntaxError::InvalidWhereClauseComponents(
                "range count queries cannot also carry an `in` clause; pick either \
                 per-value split (In) or per-distinct-value range \
                 (return_distinct_counts_in_range)",
            ));
        }

        if return_distinct_counts_in_range && !has_range {
            return Err(QuerySyntaxError::InvalidWhereClauseComponents(
                "return_distinct_counts_in_range requires a range where-clause",
            ));
        }
        if return_distinct_counts_in_range && prove {
            return Err(QuerySyntaxError::InvalidWhereClauseComponents(
                "return_distinct_counts_in_range = true is only supported on the \
                 no-prove path; the proof primitive returns a single aggregate",
            ));
        }

        Ok(match (has_range, has_in, prove) {
            (true, false, true) => DocumentCountMode::RangeProof,
            (true, false, false) => DocumentCountMode::RangeNoProof,
            (false, true, _) => DocumentCountMode::PerInValue,
            (false, false, true) => DocumentCountMode::PointLookupProof,
            (false, false, false) => DocumentCountMode::Total,
            // (true, true, _) is rejected by the has_range && has_in
            // check above; (false, _, false) falls through cleanly.
            (true, true, _) => unreachable!("range + In is rejected above"),
        })
    }

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

    /// Finds a countable index where:
    /// - The indexable (Equal / In) where-clause fields form a prefix of the index properties
    /// - The `split_property` is the next property after the covered prefix
    /// - The index has `countable = true`
    /// - Returns `None` if any where clause uses an operator other than `Equal` / `In`
    pub fn find_countable_index_for_split<'b>(
        indexes: &'b BTreeMap<String, Index>,
        where_clauses: &[WhereClause],
        split_property: &str,
    ) -> Option<&'b Index> {
        if Self::has_unsupported_operator(where_clauses) {
            return None;
        }

        let indexable_fields: BTreeSet<&str> = where_clauses
            .iter()
            .filter(|wc| Self::is_indexable_for_count(wc.operator))
            .map(|wc| wc.field.as_str())
            .collect();

        for index in indexes.values() {
            if !index.countable.is_countable() {
                continue;
            }

            // Check that indexable where-clause fields form a prefix.
            let mut prefix_len = 0;
            for prop in &index.properties {
                if indexable_fields.contains(prop.name.as_str()) {
                    prefix_len += 1;
                } else {
                    break;
                }
            }

            if prefix_len < indexable_fields.len() {
                continue;
            }

            // The split property must be the next property after the prefix.
            if let Some(next_prop) = index.properties.get(prefix_len) {
                if next_prop.name == split_property {
                    return Some(index);
                }
            }
        }

        None
    }

    /// Executes the count query without generating a proof.
    ///
    /// When `split_by_property` is `None`, returns the total count as a single
    /// `SplitCountEntry` with an empty key.
    ///
    /// When `split_by_property` is `Some`, returns per-value counts for the
    /// split property.
    #[cfg(feature = "server")]
    pub fn execute_no_proof(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<SplitCountEntry>, Error> {
        if self.split_by_property.is_some() {
            self.execute_split_count(drive, transaction, platform_version)
        } else {
            let count = self.execute_total_count(drive, transaction, platform_version)?;
            Ok(vec![SplitCountEntry { key: vec![], count }])
        }
    }

    /// Executes the count query and generates a GroveDB proof.
    ///
    /// Returns the raw proof bytes. The caller is responsible for verifying
    /// the proof and extracting the count from the verified result.
    #[cfg(feature = "server")]
    pub fn execute_with_proof(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let drive_version = &platform_version.drive;

        // Build the same path as execute_no_proof
        let mut path = vec![
            vec![RootTree::DataContractDocuments as u8],
            self.contract_id.to_vec(),
            vec![1u8],
            self.document_type_name.as_bytes().to_vec(),
        ];

        // Walk the index properties, pushing property keys and equality values
        for prop in &self.index.properties {
            let matching_clause = self
                .where_clauses
                .iter()
                .find(|wc| wc.field == prop.name && wc.operator == WhereOperator::Equal);

            if let Some(clause) = matching_clause {
                path.push(prop.name.as_bytes().to_vec());
                let serialized_value = self.document_type.serialize_value_for_key(
                    prop.name.as_str(),
                    &clause.value,
                    platform_version,
                )?;
                path.push(serialized_value);
            } else {
                break;
            }
        }

        // Build a path query that covers the count tree and its contents
        let mut query = Query::new();
        query.insert_all();

        let path_query = PathQuery::new(path, SizedQuery::new(query, None, None));

        let proof = drive
            .grove
            .get_proved_path_query(&path_query, None, transaction, &drive_version.grove_version)
            .unwrap()
            .map_err(|e| Error::GroveDB(Box::new(e)))?;

        Ok(proof)
    }

    /// Executes the total count query, returning a single u64 count.
    ///
    /// Walks the index level-by-level, branching on `In` clauses (each value
    /// adds a path) and falling through to [`Self::count_recursive`] for any
    /// trailing index properties that have no matching where clause.
    #[cfg(feature = "server")]
    fn execute_total_count(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<u64, Error> {
        // Build the base path: [DataContractDocuments, contract_id, 1, doc_type_name]
        let base_path = vec![
            vec![RootTree::DataContractDocuments as u8],
            self.contract_id.to_vec(),
            vec![1u8],
            self.document_type_name.as_bytes().to_vec(),
        ];

        self.expand_paths_and_count(drive, base_path, 0, transaction, platform_version)
    }

    /// Recursive helper for [`Self::execute_total_count`].
    ///
    /// Visits the index property at `prop_idx`. If a matching where clause is
    /// found:
    /// - `Equal` → extend the current path with `(prop_name, value)` and recurse.
    /// - `In` → for each value in the clause's array, clone the path, extend
    ///   with that value, recurse, and sum the per-branch counts. This is the
    ///   cartesian fork.
    /// - anything else → unreachable; the index picker rejects the query.
    ///
    /// If no clause matches the current property, hand off to
    /// [`Self::count_recursive`] which sums all sub-counts at the remaining
    /// levels.
    #[cfg(feature = "server")]
    fn expand_paths_and_count(
        &self,
        drive: &Drive,
        current_path: Vec<Vec<u8>>,
        prop_idx: usize,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<u64, Error> {
        let drive_version = &platform_version.drive;

        if prop_idx == self.index.properties.len() {
            // All index properties resolved to a fixed key — O(1) read.
            return Self::fetch_count_at_path(drive, &current_path, transaction, drive_version);
        }

        let prop = &self.index.properties[prop_idx];
        let matching_clause = self.where_clauses.iter().find(|wc| wc.field == prop.name);

        let Some(clause) = matching_clause else {
            // No clause for this property. Walk all values at the remaining
            // levels and sum.
            let remaining = &self.index.properties[prop_idx..];
            return Self::count_recursive(
                drive,
                current_path,
                remaining,
                transaction,
                drive_version,
            );
        };

        match clause.operator {
            WhereOperator::Equal => {
                let mut new_path = current_path;
                new_path.push(prop.name.as_bytes().to_vec());
                new_path.push(self.document_type.serialize_value_for_key(
                    prop.name.as_str(),
                    &clause.value,
                    platform_version,
                )?);
                self.expand_paths_and_count(
                    drive,
                    new_path,
                    prop_idx + 1,
                    transaction,
                    platform_version,
                )
            }
            WhereOperator::In => {
                let values = clause.value.as_array().ok_or_else(|| {
                    Error::Query(QuerySyntaxError::InvalidWhereClauseComponents(
                        "In where-clause value must be an array",
                    ))
                })?;

                // `In` is set-membership: serialize each value to the canonical
                // index key and dedupe before forking. Without this, a query
                // like `age in [30, 30]` would visit and sum the same subtree
                // twice (Codex review finding #3).
                let mut seen_keys: BTreeSet<Vec<u8>> = BTreeSet::new();
                let mut total: u64 = 0;
                for v in values {
                    let serialized = self.document_type.serialize_value_for_key(
                        prop.name.as_str(),
                        v,
                        platform_version,
                    )?;
                    if !seen_keys.insert(serialized.clone()) {
                        continue;
                    }
                    let mut new_path = current_path.clone();
                    new_path.push(prop.name.as_bytes().to_vec());
                    new_path.push(serialized);
                    total = total.saturating_add(self.expand_paths_and_count(
                        drive,
                        new_path,
                        prop_idx + 1,
                        transaction,
                        platform_version,
                    )?);
                }
                Ok(total)
            }
            _ => Err(Error::Query(
                QuerySyntaxError::InvalidWhereClauseComponents(
                    "count fast path supports only Equal and In where-clause operators",
                ),
            )),
        }
    }

    /// Executes a split count query, returning per-value counts for the
    /// split property.
    ///
    /// Walks the index prefix that precedes `split_by_property` level by
    /// level, branching on `In` clauses. For each fully-resolved prefix,
    /// runs the per-split-value sub-query (see [`Self::collect_split_at_prefix`])
    /// and merges the results by split key, summing counts.
    #[cfg(feature = "server")]
    fn execute_split_count(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<SplitCountEntry>, Error> {
        let split_property = self
            .split_by_property
            .as_deref()
            .expect("split_by_property must be Some when calling execute_split_count");

        let split_prop_idx = self
            .index
            .properties
            .iter()
            .position(|p| p.name == split_property)
            .unwrap_or(0);

        let base_path = vec![
            vec![RootTree::DataContractDocuments as u8],
            self.contract_id.to_vec(),
            vec![1u8],
            self.document_type_name.as_bytes().to_vec(),
        ];

        let mut merged: BTreeMap<Vec<u8>, u64> = BTreeMap::new();
        self.expand_split_prefix_paths(
            drive,
            base_path,
            0,
            split_prop_idx,
            split_property,
            transaction,
            platform_version,
            &mut merged,
        )?;

        Ok(merged
            .into_iter()
            .filter(|(_, count)| *count > 0)
            .map(|(key, count)| SplitCountEntry { key, count })
            .collect())
    }

    /// Walks the index up to `split_prop_idx`, branching on `In`. At each
    /// fully-resolved prefix, calls [`Self::collect_split_at_prefix`] to
    /// gather the per-split-value counts, and accumulates them into `merged`.
    #[cfg(feature = "server")]
    #[allow(clippy::too_many_arguments)]
    fn expand_split_prefix_paths(
        &self,
        drive: &Drive,
        current_path: Vec<Vec<u8>>,
        prop_idx: usize,
        split_prop_idx: usize,
        split_property: &str,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
        merged: &mut BTreeMap<Vec<u8>, u64>,
    ) -> Result<(), Error> {
        if prop_idx == split_prop_idx {
            // Reached the split property level under this prefix. Run the
            // per-split-value sub-query and merge entries by key.
            return self.collect_split_at_prefix(
                drive,
                current_path,
                split_prop_idx,
                split_property,
                transaction,
                platform_version,
                merged,
            );
        }

        let prop = &self.index.properties[prop_idx];
        let clause = self
            .where_clauses
            .iter()
            .find(|wc| wc.field == prop.name)
            .ok_or_else(|| {
                // The index picker guarantees every property before the split
                // property has a matching clause; missing one indicates a
                // mis-picked index.
                Error::Query(QuerySyntaxError::InvalidWhereClauseComponents(
                    "split count: missing where clause for an index property preceding the split property",
                ))
            })?;

        match clause.operator {
            WhereOperator::Equal => {
                let mut new_path = current_path;
                new_path.push(prop.name.as_bytes().to_vec());
                new_path.push(self.document_type.serialize_value_for_key(
                    prop.name.as_str(),
                    &clause.value,
                    platform_version,
                )?);
                self.expand_split_prefix_paths(
                    drive,
                    new_path,
                    prop_idx + 1,
                    split_prop_idx,
                    split_property,
                    transaction,
                    platform_version,
                    merged,
                )
            }
            WhereOperator::In => {
                let values = clause.value.as_array().ok_or_else(|| {
                    Error::Query(QuerySyntaxError::InvalidWhereClauseComponents(
                        "In where-clause value must be an array",
                    ))
                })?;

                // Same dedup as in `expand_paths_and_count`: serialize each
                // value to the canonical index key and skip duplicates.
                // Without this, a duplicated `In` value on the prefix would
                // visit the same prefix subtree twice and double its
                // contribution to the merged split counts.
                let mut seen_keys: BTreeSet<Vec<u8>> = BTreeSet::new();
                for v in values {
                    let serialized = self.document_type.serialize_value_for_key(
                        prop.name.as_str(),
                        v,
                        platform_version,
                    )?;
                    if !seen_keys.insert(serialized.clone()) {
                        continue;
                    }
                    let mut new_path = current_path.clone();
                    new_path.push(prop.name.as_bytes().to_vec());
                    new_path.push(serialized);
                    self.expand_split_prefix_paths(
                        drive,
                        new_path,
                        prop_idx + 1,
                        split_prop_idx,
                        split_property,
                        transaction,
                        platform_version,
                        merged,
                    )?;
                }
                Ok(())
            }
            _ => Err(Error::Query(
                QuerySyntaxError::InvalidWhereClauseComponents(
                    "split count fast path supports only Equal and In where-clause operators",
                ),
            )),
        }
    }

    /// Reads all per-value sub-counts for `split_property` under
    /// `prefix_path`, summing per-key counts into `merged`. Mirrors the
    /// original (pre-`In`-support) loop; factored out so the prefix-walk
    /// recursion can call it once per resolved prefix.
    #[cfg(feature = "server")]
    #[allow(clippy::too_many_arguments)]
    fn collect_split_at_prefix(
        &self,
        drive: &Drive,
        prefix_path: Vec<Vec<u8>>,
        split_prop_idx: usize,
        split_property: &str,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
        merged: &mut BTreeMap<Vec<u8>, u64>,
    ) -> Result<(), Error> {
        let drive_version = &platform_version.drive;

        // Push the split-property key onto the prefix to address the per-value
        // subtree level.
        let mut path = prefix_path;
        path.push(split_property.as_bytes().to_vec());

        let mut query = Query::new();
        query.insert_all();
        let path_query = PathQuery::new(path.clone(), SizedQuery::new(query, None, None));

        let mut drive_operations = vec![];
        let result = drive.grove_get_raw_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryKeyElementPairResultType,
            &mut drive_operations,
            drive_version,
        );

        let (elements, _) = match result {
            Ok(result) => result,
            Err(Error::GroveDB(e))
                if matches!(
                    e.as_ref(),
                    grovedb::Error::PathNotFound(_)
                        | grovedb::Error::PathParentLayerNotFound(_)
                        | grovedb::Error::PathKeyNotFound(_)
                ) =>
            {
                // No documents under this prefix; nothing to merge.
                return Ok(());
            }
            Err(e) => return Err(e),
        };

        let key_elements = elements.to_key_elements();
        if key_elements.is_empty() {
            return Ok(());
        }

        let remaining_properties = &self.index.properties[split_prop_idx + 1..];

        for (key, _element) in key_elements {
            let mut value_path = path.clone();
            value_path.push(key.clone());

            let count = if remaining_properties.is_empty() {
                Self::fetch_count_at_path(drive, &value_path, transaction, drive_version)?
            } else {
                Self::count_recursive(
                    drive,
                    value_path,
                    remaining_properties,
                    transaction,
                    drive_version,
                )?
            };

            if count == 0 {
                continue;
            }
            *merged.entry(key).or_insert(0) += count;
        }

        Ok(())
    }

    /// Fetches the CountTree element count at the given path.
    /// The CountTree element is at key [0] under the path.
    #[cfg(feature = "server")]
    fn fetch_count_at_path(
        drive: &Drive,
        path: &[Vec<u8>],
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<u64, Error> {
        let mut drive_operations = vec![];
        let path_refs: Vec<&[u8]> = path.iter().map(|p| p.as_slice()).collect();
        let element = drive.grove_get_raw_optional(
            SubtreePath::from(path_refs.as_slice()),
            &[0],
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut drive_operations,
            drive_version,
        )?;

        Ok(element.map_or(0, |e| e.count_value_or_default()))
    }

    /// Recursively descends through remaining index property levels,
    /// iterating over all values at each level, and sums the CountTree
    /// counts at the terminal level.
    #[cfg(feature = "server")]
    fn count_recursive(
        drive: &Drive,
        current_path: Vec<Vec<u8>>,
        remaining_properties: &[IndexProperty],
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<u64, Error> {
        if remaining_properties.is_empty() {
            return Self::fetch_count_at_path(drive, &current_path, transaction, drive_version);
        }

        let prop = &remaining_properties[0];
        let rest = &remaining_properties[1..];

        // Push the index property key to descend into that level
        let mut property_path = current_path;
        property_path.push(prop.name.as_bytes().to_vec());

        // Query all children (value subtrees) at this property level
        let mut query = Query::new();
        query.insert_all();

        let path_query = PathQuery::new(property_path.clone(), SizedQuery::new(query, None, None));

        let mut drive_operations = vec![];
        let result = drive.grove_get_raw_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryKeyElementPairResultType,
            &mut drive_operations,
            drive_version,
        );

        let (elements, _) = match result {
            Ok(result) => result,
            Err(Error::GroveDB(e))
                if matches!(
                    e.as_ref(),
                    grovedb::Error::PathNotFound(_)
                        | grovedb::Error::PathParentLayerNotFound(_)
                        | grovedb::Error::PathKeyNotFound(_)
                ) =>
            {
                return Ok(0);
            }
            Err(e) => return Err(e),
        };

        let key_elements = elements.to_key_elements();

        if key_elements.is_empty() {
            return Ok(0);
        }

        let mut total_count: u64 = 0;

        for (key, _element) in key_elements {
            let mut value_path = property_path.clone();
            value_path.push(key);

            let sub_count =
                Self::count_recursive(drive, value_path, rest, transaction, drive_version)?;
            total_count = total_count.saturating_add(sub_count);
        }

        Ok(total_count)
    }
}

/// Pagination + ordering knobs for `execute_range_count_no_proof`.
///
/// Mirrors the protobuf request fields on
/// `GetDocumentsCountRequestV0` so the drive-abci handler can pass them
/// through unmodified. `distinct = false` collapses the range walk to a
/// single summed entry; `distinct = true` returns one entry per distinct
/// property value within the range.
#[cfg(feature = "server")]
#[derive(Debug, Clone, Default)]
pub struct RangeCountOptions {
    /// When `true`, return one [`SplitCountEntry`] per distinct property
    /// value within the range. When `false`, return a single entry
    /// (empty `key`) summing all per-value counts.
    pub distinct: bool,
    /// Maximum number of entries to return. Only meaningful when
    /// `distinct = true`. Applied after `start_after_split_key`. `None`
    /// means no limit.
    pub limit: Option<u32>,
    /// Pagination cursor: skip entries up to and including this
    /// serialized key. Only meaningful when `distinct = true`.
    pub start_after_split_key: Option<Vec<u8>>,
    /// Sort order for distinct entries. `true` (default) is ascending by
    /// serialized key bytes. Ignored when `distinct = false`.
    pub order_by_ascending: bool,
}

#[cfg(feature = "server")]
impl<'a> DriveDocumentCountQuery<'a> {
    /// Executes a range-aware count query against a `range_countable`
    /// index. Walks children of the property-name `ProvableCountTree` at
    /// path `[contract_doc, doctype, prefix..., range_prop_name]` whose
    /// keys lie within the range. Each child is a `CountTree` whose
    /// `count_value_or_default()` is the document count at that property
    /// value.
    ///
    /// The caller picks the index via
    /// [`Self::find_range_countable_index_for_where_clauses`]; this
    /// method assumes:
    /// - `self.index.range_countable == true`
    /// - All `Equal` / `In` where clauses cover the index prefix
    /// - Exactly one range-operator where clause hits the index's last
    ///   property
    ///
    /// `In` on the prefix forks the walk into one path per (deduped)
    /// `In` value and merges the results.
    ///
    /// When `options.distinct = false`, returns a single entry with
    /// empty key whose count is the sum of all per-value counts in the
    /// range. When `options.distinct = true`, returns one entry per
    /// distinct property value within the range, after applying
    /// `order_by_ascending`, `start_after_split_key`, and `limit`.
    pub fn execute_range_count_no_proof(
        &self,
        drive: &Drive,
        options: &RangeCountOptions,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<SplitCountEntry>, Error> {
        let drive_version = &platform_version.drive;

        let range_clause = self
            .where_clauses
            .iter()
            .find(|wc| Self::is_range_operator(wc.operator))
            .ok_or_else(|| {
                Error::Query(QuerySyntaxError::InvalidWhereClauseComponents(
                    "execute_range_count_no_proof requires exactly one range where-clause",
                ))
            })?;
        if self
            .where_clauses
            .iter()
            .filter(|wc| Self::is_range_operator(wc.operator))
            .count()
            > 1
        {
            return Err(Error::Query(
                QuerySyntaxError::InvalidWhereClauseComponents(
                    "range count supports only one range where-clause",
                ),
            ));
        }
        let query_item = self.range_clause_to_query_item(range_clause, platform_version)?;

        // Build the prefix path: [contract_doc, doctype, prop_a, val_a,
        // prop_b, val_b, ...]. Equal clauses contribute one path each;
        // In clauses fork into multiple paths.
        let base_path = vec![
            vec![RootTree::DataContractDocuments as u8],
            self.contract_id.to_vec(),
            vec![1u8],
            self.document_type_name.as_bytes().to_vec(),
        ];

        // Prefix props are everything in the index up to (but not
        // including) the range property — by picker invariant the range
        // property is `index.properties.last()`.
        let prefix_props = &self.index.properties[..self.index.properties.len() - 1];
        let range_prop_name = &self
            .index
            .properties
            .last()
            .ok_or_else(|| {
                Error::Query(QuerySyntaxError::InvalidWhereClauseComponents(
                    "range_countable index must have at least one property",
                ))
            })?
            .name;

        let mut prefix_paths: Vec<Vec<Vec<u8>>> = vec![base_path];
        for prop in prefix_props {
            let clause = self.where_clauses.iter().find(|wc| wc.field == prop.name).ok_or_else(|| {
                Error::Query(QuerySyntaxError::InvalidWhereClauseComponents(
                    "range count: missing where clause for an index property preceding the range property",
                ))
            })?;
            let mut next_paths: Vec<Vec<Vec<u8>>> = Vec::new();
            match clause.operator {
                WhereOperator::Equal => {
                    let serialized = self.document_type.serialize_value_for_key(
                        prop.name.as_str(),
                        &clause.value,
                        platform_version,
                    )?;
                    for mut path in prefix_paths.into_iter() {
                        path.push(prop.name.as_bytes().to_vec());
                        path.push(serialized.clone());
                        next_paths.push(path);
                    }
                }
                WhereOperator::In => {
                    let values = clause.value.as_array().ok_or_else(|| {
                        Error::Query(QuerySyntaxError::InvalidWhereClauseComponents(
                            "In where-clause value must be an array",
                        ))
                    })?;
                    let mut seen: BTreeSet<Vec<u8>> = BTreeSet::new();
                    for v in values {
                        let serialized = self.document_type.serialize_value_for_key(
                            prop.name.as_str(),
                            v,
                            platform_version,
                        )?;
                        if !seen.insert(serialized.clone()) {
                            continue;
                        }
                        for path in &prefix_paths {
                            let mut p = path.clone();
                            p.push(prop.name.as_bytes().to_vec());
                            p.push(serialized.clone());
                            next_paths.push(p);
                        }
                    }
                }
                _ => {
                    return Err(Error::Query(
                        QuerySyntaxError::InvalidWhereClauseComponents(
                            "range count: only Equal and In are supported on prefix properties",
                        ),
                    ));
                }
            }
            prefix_paths = next_paths;
        }

        // Per prefix path, walk the range under [..., range_prop_name].
        // Merge per-key entries across In-fork paths so a value that
        // appears under two prefixes contributes the sum of both.
        let mut merged: BTreeMap<Vec<u8>, u64> = BTreeMap::new();
        for prefix in prefix_paths {
            let mut path = prefix;
            path.push(range_prop_name.as_bytes().to_vec());

            let mut query = Query::new();
            query.insert_item(query_item.clone());
            let path_query = PathQuery::new(path.clone(), SizedQuery::new(query, None, None));

            let mut drive_operations = vec![];
            let result = drive.grove_get_raw_path_query(
                &path_query,
                transaction,
                QueryResultType::QueryKeyElementPairResultType,
                &mut drive_operations,
                drive_version,
            );
            let (elements, _) = match result {
                Ok(r) => r,
                Err(Error::GroveDB(e))
                    if matches!(
                        e.as_ref(),
                        grovedb::Error::PathNotFound(_)
                            | grovedb::Error::PathParentLayerNotFound(_)
                            | grovedb::Error::PathKeyNotFound(_)
                    ) =>
                {
                    continue;
                }
                Err(e) => return Err(e),
            };

            for (key, element) in elements.to_key_elements() {
                let count = element.count_value_or_default();
                if count == 0 {
                    continue;
                }
                *merged.entry(key).or_insert(0) += count;
            }
        }

        if !options.distinct {
            // Sum mode: collapse all entries into one with empty key.
            let total: u64 = merged.values().copied().sum();
            return Ok(vec![SplitCountEntry {
                key: Vec::new(),
                count: total,
            }]);
        }

        // Distinct mode: apply order, then cursor, then limit.
        let mut entries: Vec<SplitCountEntry> = merged
            .into_iter()
            .map(|(key, count)| SplitCountEntry { key, count })
            .collect();
        // BTreeMap iteration is already ascending; flip if requested.
        if !options.order_by_ascending {
            entries.reverse();
        }
        if let Some(cursor) = options.start_after_split_key.as_ref() {
            // Drop everything up to AND including the cursor key
            // (matches the protobuf doc: "skip entries up to and
            // including this serialized key").
            let kept: Vec<SplitCountEntry> = entries
                .into_iter()
                .skip_while(|e| {
                    if options.order_by_ascending {
                        e.key.as_slice() <= cursor.as_slice()
                    } else {
                        e.key.as_slice() >= cursor.as_slice()
                    }
                })
                .collect();
            entries = kept;
        }
        if let Some(limit) = options.limit {
            entries.truncate(limit as usize);
        }
        Ok(entries)
    }

    /// Generates a grovedb `AggregateCountOnRange` proof for a
    /// range-count query against a `range_countable` index. The returned
    /// proof bytes can be verified client-side via
    /// `GroveDb::verify_aggregate_count_query`, which yields
    /// `(root_hash, count)` — replacing the materialize-and-count proof
    /// path that capped at `u16::MAX` documents.
    ///
    /// Limitations vs. [`Self::execute_range_count_no_proof`]:
    /// - Returns ONLY the total count (a single number, no
    ///   per-distinct-value entries) — `AggregateCountOnRange` is a
    ///   single-aggregate primitive at the merk layer.
    /// - Requires the prefix to resolve to exactly one path. `In` on
    ///   prefix properties is not supported because grovedb's aggregate
    ///   primitive only lifts a single inner range.
    pub fn execute_aggregate_count_with_proof(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let drive_version = &platform_version.drive;
        let path_query = self.aggregate_count_path_query(platform_version)?;
        let proof = drive
            .grove
            .get_proved_path_query(&path_query, None, transaction, &drive_version.grove_version)
            .unwrap()
            .map_err(|e| Error::GroveDB(Box::new(e)))?;
        Ok(proof)
    }
}

#[cfg(any(feature = "server", feature = "verify"))]
impl<'a> DriveDocumentCountQuery<'a> {
    /// Convert a single range where-clause + value into the grovedb
    /// `QueryItem` used to walk children of the property-name
    /// `ProvableCountTree`. The clause's value is serialized via the
    /// document type's `serialize_value_for_key`, which produces the
    /// canonical bytes used everywhere else in the index path.
    ///
    /// Range mappings:
    /// - `>` → `RangeAfter(value..)` (exclusive lower)
    /// - `>=` → `RangeFrom(value..)` (inclusive lower)
    /// - `<` → `RangeTo(..value)` (exclusive upper)
    /// - `<=` → `RangeToInclusive(..=value)` (inclusive upper)
    /// - `between [a, b]` → `RangeInclusive(a..=b)` (inclusive both)
    /// - `between (a, b)` → `RangeAfterTo(a..b)` (exclusive both — the
    ///   inner range is half-open in grovedb terms; this models
    ///   exclude-bounds)
    /// - `between (a, b]` → `RangeAfterToInclusive(a..=b)`
    /// - `between [a, b)` → `Range(a..b)`
    /// - `startsWith` is rejected here — its grovedb encoding requires
    ///   a byte-incremented upper bound that depends on key encoding,
    ///   which we don't compute generically.
    fn range_clause_to_query_item(
        &self,
        clause: &WhereClause,
        platform_version: &PlatformVersion,
    ) -> Result<QueryItem, Error> {
        let serialize = |v: &dpp::platform_value::Value| -> Result<Vec<u8>, Error> {
            Ok(self.document_type.serialize_value_for_key(
                clause.field.as_str(),
                v,
                platform_version,
            )?)
        };
        let serialize_pair = |op_name: &'static str| -> Result<(Vec<u8>, Vec<u8>), Error> {
            let arr = clause.value.as_array().ok_or_else(|| {
                Error::Query(QuerySyntaxError::InvalidWhereClauseComponents(
                    "range bounds value must be a 2-element array",
                ))
            })?;
            if arr.len() != 2 {
                return Err(Error::Query(
                    QuerySyntaxError::InvalidWhereClauseComponents(
                        "range bounds value must be a 2-element array",
                    ),
                ));
            }
            let a = serialize(&arr[0])?;
            let b = serialize(&arr[1])?;
            if a > b {
                let _ = op_name;
                return Err(Error::Query(
                    QuerySyntaxError::InvalidWhereClauseComponents(
                        "range lower bound must be <= upper bound",
                    ),
                ));
            }
            Ok((a, b))
        };

        Ok(match clause.operator {
            WhereOperator::GreaterThan => {
                let v = serialize(&clause.value)?;
                QueryItem::RangeAfter(v..)
            }
            WhereOperator::GreaterThanOrEquals => {
                let v = serialize(&clause.value)?;
                QueryItem::RangeFrom(v..)
            }
            WhereOperator::LessThan => {
                let v = serialize(&clause.value)?;
                QueryItem::RangeTo(..v)
            }
            WhereOperator::LessThanOrEquals => {
                let v = serialize(&clause.value)?;
                QueryItem::RangeToInclusive(..=v)
            }
            WhereOperator::Between => {
                let (a, b) = serialize_pair("between")?;
                QueryItem::RangeInclusive(a..=b)
            }
            WhereOperator::BetweenExcludeBounds => {
                let (a, b) = serialize_pair("betweenExcludeBounds")?;
                QueryItem::RangeAfterTo(a..b)
            }
            WhereOperator::BetweenExcludeLeft => {
                let (a, b) = serialize_pair("betweenExcludeLeft")?;
                QueryItem::RangeAfterToInclusive(a..=b)
            }
            WhereOperator::BetweenExcludeRight => {
                let (a, b) = serialize_pair("betweenExcludeRight")?;
                QueryItem::Range(a..b)
            }
            WhereOperator::StartsWith => {
                return Err(Error::Query(
                    QuerySyntaxError::InvalidWhereClauseComponents(
                        "startsWith is not yet supported on the range_countable count fast path",
                    ),
                ));
            }
            _ => {
                return Err(Error::Query(
                    QuerySyntaxError::InvalidWhereClauseComponents(
                        "range_clause_to_query_item called on a non-range operator",
                    ),
                ));
            }
        })
    }

    /// Build the grovedb `PathQuery` for an `AggregateCountOnRange`
    /// query against this count query's `range_countable` index.
    ///
    /// Shared between the server-side prove path
    /// ([`Self::execute_aggregate_count_with_proof`]) and the client-
    /// side verify path (the SDK's `FromProof<DocumentCountQuery>` for
    /// `DocumentCount`). Both sides must produce the *exact same*
    /// `PathQuery` for verification to recompute the same merk root —
    /// keeping path construction in one place is load-bearing.
    ///
    /// Inputs come from the struct fields:
    /// - `contract_id`, `document_type_name`, `index` — index path prefix
    /// - `where_clauses` — Equal-only prefix clauses + exactly one
    ///    range clause on the index's last property
    /// - `document_type` — for `serialize_value_for_key` on prefix values
    ///
    /// Errors:
    /// - No range where-clause / multiple range where-clauses →
    ///   `InvalidWhereClauseComponents`
    /// - `In` on a prefix property (would need multiple disjoint proofs)
    ///   → `InvalidWhereClauseComponents`
    /// - Missing prefix clause → `InvalidWhereClauseComponents`
    pub fn aggregate_count_path_query(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<PathQuery, Error> {
        let range_clause = self
            .where_clauses
            .iter()
            .find(|wc| Self::is_range_operator(wc.operator))
            .ok_or_else(|| {
                Error::Query(QuerySyntaxError::InvalidWhereClauseComponents(
                    "aggregate_count_path_query requires a range where-clause",
                ))
            })?;
        let query_item = self.range_clause_to_query_item(range_clause, platform_version)?;

        // Build the path. Prefix props must be Equal-only — In would
        // require multiple separate proofs, which doesn't compose into
        // a single aggregate.
        let mut path = vec![
            vec![RootTree::DataContractDocuments as u8],
            self.contract_id.to_vec(),
            vec![1u8],
            self.document_type_name.as_bytes().to_vec(),
        ];
        let prefix_props = &self.index.properties[..self.index.properties.len() - 1];
        for prop in prefix_props {
            let clause = self
                .where_clauses
                .iter()
                .find(|wc| wc.field == prop.name)
                .ok_or_else(|| {
                    Error::Query(QuerySyntaxError::InvalidWhereClauseComponents(
                        "aggregate-count proof: missing where clause for an index prefix property",
                    ))
                })?;
            if clause.operator != WhereOperator::Equal {
                return Err(Error::Query(
                    QuerySyntaxError::InvalidWhereClauseComponents(
                        "aggregate-count proof: prefix properties must use `==` (no `in`)",
                    ),
                ));
            }
            path.push(prop.name.as_bytes().to_vec());
            path.push(self.document_type.serialize_value_for_key(
                prop.name.as_str(),
                &clause.value,
                platform_version,
            )?);
        }
        let range_prop_name = &self
            .index
            .properties
            .last()
            .ok_or_else(|| {
                Error::Query(QuerySyntaxError::InvalidWhereClauseComponents(
                    "range_countable index must have at least one property",
                ))
            })?
            .name;
        path.push(range_prop_name.as_bytes().to_vec());

        Ok(PathQuery::new_aggregate_count_on_range(path, query_item))
    }
}

#[cfg(feature = "server")]
impl Drive {
    //! Per-mode count-query executors. Each method:
    //!   1. Picks the right covering index for its mode (returns
    //!      `Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty)`
    //!      if no index covers the where clauses).
    //!   2. Builds the appropriate `DriveDocumentCountQuery` /
    //!      `DriveDocumentQuery`.
    //!   3. Runs the right executor (`execute_no_proof`,
    //!      `execute_range_count_no_proof`,
    //!      `execute_aggregate_count_with_proof`, or
    //!      `execute_with_proof`).
    //!   4. Returns either `Vec<SplitCountEntry>` (no-proof modes)
    //!      or `Vec<u8>` proof bytes (proof modes).
    //!
    //! These methods are step 2 of the document_count_query handler
    //! refactor: they collapse what used to be ~30-line per-mode
    //! match arms in the drive-abci handler into single calls.

    /// Total count for the given where clauses against the best
    /// covering countable index. Single summed entry with empty key.
    /// Used by [`DocumentCountMode::Total`] dispatch.
    pub fn execute_document_count_total_no_proof(
        &self,
        contract_id: [u8; 32],
        document_type: DocumentTypeRef,
        document_type_name: String,
        where_clauses: Vec<WhereClause>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<SplitCountEntry>, Error> {
        let index = DriveDocumentCountQuery::find_countable_index_for_where_clauses(
            document_type.indexes(),
            &where_clauses,
        )
        .ok_or_else(|| {
            Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(
                "count query requires a countable index on the document type that \
                     matches the where clause properties"
                    .to_string(),
            ))
        })?;
        let count_query = DriveDocumentCountQuery {
            document_type,
            contract_id,
            document_type_name,
            index,
            where_clauses,
            split_by_property: None,
        };
        count_query.execute_no_proof(self, transaction, platform_version)
    }

    /// Per-`In`-value entries: cartesian-fork the single `In` clause
    /// into one Equal-on-each-value sub-query, run each, emit a
    /// `(serialized_value, count)` entry. Used by
    /// [`DocumentCountMode::PerInValue`] dispatch.
    ///
    /// Caller has already verified via [`DriveDocumentCountQuery::detect_mode`]
    /// that exactly one `In` clause is present in `where_clauses`.
    pub fn execute_document_count_per_in_value_no_proof(
        &self,
        contract_id: [u8; 32],
        document_type: DocumentTypeRef,
        document_type_name: String,
        where_clauses: Vec<WhereClause>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<SplitCountEntry>, Error> {
        let in_clause = where_clauses
            .iter()
            .find(|wc| wc.operator == WhereOperator::In)
            .ok_or_else(|| {
                Error::Query(QuerySyntaxError::InvalidWhereClauseComponents(
                    "execute_document_count_per_in_value_no_proof requires exactly one `in` clause",
                ))
            })?
            .clone();
        let in_values = in_clause.value.as_array().ok_or_else(|| {
            Error::Query(QuerySyntaxError::InvalidWhereClauseComponents(
                "In where-clause value must be an array",
            ))
        })?;

        let other_clauses: Vec<WhereClause> = where_clauses
            .iter()
            .filter(|wc| wc.operator != WhereOperator::In)
            .cloned()
            .collect();

        let mut entries = Vec::with_capacity(in_values.len());
        let mut seen_keys: BTreeSet<Vec<u8>> = BTreeSet::new();
        for value in in_values {
            // Pre-serialize so wire keys round-trip consistently with
            // the no-In total-count path AND so we dedupe when an `In`
            // value list contains duplicates.
            let key_bytes = document_type.serialize_value_for_key(
                in_clause.field.as_str(),
                value,
                platform_version,
            )?;
            if !seen_keys.insert(key_bytes.clone()) {
                continue;
            }

            let mut clauses_for_value = other_clauses.clone();
            clauses_for_value.push(WhereClause {
                field: in_clause.field.clone(),
                operator: WhereOperator::Equal,
                value: value.clone(),
            });

            let index = DriveDocumentCountQuery::find_countable_index_for_where_clauses(
                document_type.indexes(),
                &clauses_for_value,
            )
            .ok_or_else(|| {
                Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(
                    "count query requires a countable index on the document type that \
                     matches the where clause properties"
                        .to_string(),
                ))
            })?;

            let count_query = DriveDocumentCountQuery {
                document_type,
                contract_id,
                document_type_name: document_type_name.clone(),
                index,
                where_clauses: clauses_for_value,
                split_by_property: None,
            };
            let results = count_query.execute_no_proof(self, transaction, platform_version)?;
            let count = results.first().map_or(0, |entry| entry.count);

            entries.push(SplitCountEntry {
                key: key_bytes,
                count,
            });
        }
        Ok(entries)
    }

    /// Range-count walk against a `range_countable` index. Returns a
    /// summed entry or per-distinct-value entries depending on
    /// `options.distinct`. Used by [`DocumentCountMode::RangeNoProof`]
    /// dispatch.
    #[allow(clippy::too_many_arguments)]
    pub fn execute_document_count_range_no_proof(
        &self,
        contract_id: [u8; 32],
        document_type: DocumentTypeRef,
        document_type_name: String,
        where_clauses: Vec<WhereClause>,
        options: RangeCountOptions,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<SplitCountEntry>, Error> {
        let index = DriveDocumentCountQuery::find_range_countable_index_for_where_clauses(
            document_type.indexes(),
            &where_clauses,
        )
        .ok_or_else(|| {
            Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(
                "range count requires a `range_countable: true` index whose last \
                     property matches the range field, with all other clauses covering \
                     its prefix as `==` matches"
                    .to_string(),
            ))
        })?;
        let count_query = DriveDocumentCountQuery {
            document_type,
            contract_id,
            document_type_name,
            index,
            where_clauses,
            split_by_property: None,
        };
        count_query.execute_range_count_no_proof(self, &options, transaction, platform_version)
    }

    /// Range-count proof via grovedb's `AggregateCountOnRange`. Returns
    /// proof bytes that the client verifies via
    /// `GroveDb::verify_aggregate_count_query`. Used by
    /// [`DocumentCountMode::RangeProof`] dispatch.
    pub fn execute_document_count_range_proof(
        &self,
        contract_id: [u8; 32],
        document_type: DocumentTypeRef,
        document_type_name: String,
        where_clauses: Vec<WhereClause>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let index = DriveDocumentCountQuery::find_range_countable_index_for_where_clauses(
            document_type.indexes(),
            &where_clauses,
        )
        .ok_or_else(|| {
            Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(
                "range count requires a `range_countable: true` index whose last \
                     property matches the range field"
                    .to_string(),
            ))
        })?;
        let count_query = DriveDocumentCountQuery {
            document_type,
            contract_id,
            document_type_name,
            index,
            where_clauses,
            split_by_property: None,
        };
        count_query.execute_aggregate_count_with_proof(self, transaction, platform_version)
    }

    /// Materialize-and-count proof fallback for point-lookup count
    /// queries with `prove = true`. Capped at `u16::MAX` matching docs
    /// because each document is materialized client-side. Used by
    /// [`DocumentCountMode::PointLookupProof`] dispatch.
    ///
    /// `where_clause` is the raw decoded `Value` (matching what
    /// `DriveDocumentQuery::from_decomposed_values` expects), not a
    /// `Vec<WhereClause>` — the materialize-path uses the broader
    /// `DriveDocumentQuery` which has its own internal where-clause
    /// model.
    #[allow(clippy::too_many_arguments)]
    pub fn execute_document_count_point_lookup_proof(
        &self,
        where_clause: dpp::platform_value::Value,
        contract: &dpp::data_contract::DataContract,
        document_type: DocumentTypeRef,
        drive_config: &crate::config::DriveConfig,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let mut drive_query = crate::query::DriveDocumentQuery::from_decomposed_values(
            where_clause,
            None,
            Some(drive_config.default_query_limit),
            None,
            true,
            None,
            contract,
            document_type,
            drive_config,
        )?;
        // Defensive cap: the proof verifier deserializes every doc.
        // Until per-CountTree count proofs are wired through, callers
        // that need exact counts on larger result sets must use
        // `prove=false` with a covering countable index.
        drive_query.limit = Some(u16::MAX);
        Ok(drive_query
            .execute_with_proof(self, None, transaction, platform_version)?
            .0)
    }
}

/// All inputs required for the unified document-count entry point
/// [`Drive::execute_document_count_request`]. Built by the gRPC
/// handler from a `GetDocumentsCountRequestV0` after CBOR-decoding +
/// contract lookup; drive owns everything past this point including
/// mode detection, index picking, and per-mode dispatch.
///
/// Both `where_clauses` and `raw_where_value` are present because
/// `DriveDocumentQuery::from_decomposed_values` (used by the
/// materialize-and-count fallback for `prove=true` point lookups)
/// takes a `Value` while every other path takes the parsed
/// `Vec<WhereClause>`. The handler decodes once and passes both.
#[cfg(feature = "server")]
pub struct DocumentCountRequest<'a> {
    /// Live contract (already loaded by the handler).
    pub contract: &'a dpp::data_contract::DataContract,
    /// Resolved document type within `contract`.
    pub document_type: DocumentTypeRef<'a>,
    /// Parsed where clauses for mode detection + executor dispatch.
    pub where_clauses: Vec<WhereClause>,
    /// Raw decoded where `Value` — needed only by the materialize-and-
    /// count fallback (`PointLookupProof`); other modes ignore it.
    pub raw_where_value: dpp::platform_value::Value,
    /// `return_distinct_counts_in_range` flag from the request.
    pub return_distinct_counts_in_range: bool,
    /// `order_by_ascending` from the request (`None` = ascending, the
    /// default for distinct-mode entries).
    pub order_by_ascending: Option<bool>,
    /// Limit cap from the request, **already clamped** by the caller
    /// against its `max_query_limit` policy. Drive applies it as-is to
    /// the distinct-mode entry list.
    pub limit: Option<u32>,
    /// Pagination cursor for distinct-mode entries.
    pub start_after_split_key: Option<Vec<u8>>,
    /// Whether to produce a proof (vs. raw counts).
    pub prove: bool,
    /// Drive-side query config — only consumed by the materialize-and-
    /// count fallback.
    pub drive_config: &'a crate::config::DriveConfig,
}

/// Output shape of [`Drive::execute_document_count_request`]. Either
/// a raw set of `(key, count)` entries (Counts modes) or proof bytes
/// the client must verify (Proof modes). The gRPC handler maps these
/// to the protobuf `oneof result` variants.
#[cfg(feature = "server")]
#[derive(Debug, Clone)]
pub enum DocumentCountResponse {
    /// Per-entry counts. The shape inside depends on the request mode:
    /// - `Total` → exactly one entry, empty `key`, count = total
    /// - `PerInValue` → one entry per deduped `In` value
    /// - `RangeNoProof` → one entry summed (empty key) or one per
    ///   distinct value in the range, depending on
    ///   `return_distinct_counts_in_range`
    Counts(Vec<SplitCountEntry>),
    /// Grovedb proof bytes the client verifies via either
    /// `verify_aggregate_count_query` (for `RangeProof`) or the
    /// `DriveDocumentQuery` proof verifier (for `PointLookupProof`).
    Proof(Vec<u8>),
}

#[cfg(feature = "server")]
impl Drive {
    /// Single entry point for the unified `GetDocumentsCount` request.
    ///
    /// Owns the whole pipeline:
    /// 1. [`DriveDocumentCountQuery::detect_mode`] classifies the
    ///    query shape from the where clauses + flags.
    /// 2. The matching `Drive::execute_document_count_*` per-mode
    ///    method picks an index and runs the executor.
    /// 3. The result is wrapped in [`DocumentCountResponse`] —
    ///    `Counts(...)` for no-proof modes, `Proof(...)` for proof
    ///    modes.
    ///
    /// Errors:
    /// - Mode-detection failures (multiple range clauses, range +
    ///   `In`, distinct on prove path, …) come back as
    ///   `Error::Query(QuerySyntaxError::InvalidWhereClauseComponents)`.
    /// - "No covering index" failures come back as
    ///   `Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty)`.
    /// - All other failures (grovedb, cost calculation, …) surface
    ///   as their native `Error` variants.
    ///
    /// The handler maps both `Error::Query(...)` cases to its own
    /// `QueryError::Query(...)` variant uniformly.
    pub fn execute_document_count_request(
        &self,
        request: DocumentCountRequest,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<DocumentCountResponse, Error> {
        use dpp::data_contract::accessors::v0::DataContractV0Getters;

        let mode = DriveDocumentCountQuery::detect_mode(
            &request.where_clauses,
            request.return_distinct_counts_in_range,
            request.prove,
        )?;

        let contract_id = request.contract.id_ref().to_buffer();
        let document_type_name = request.document_type.name().to_string();

        match mode {
            DocumentCountMode::Total => {
                let entries = self.execute_document_count_total_no_proof(
                    contract_id,
                    request.document_type,
                    document_type_name,
                    request.where_clauses,
                    transaction,
                    platform_version,
                )?;
                // Total mode produces exactly one entry; if the indexed
                // path doesn't exist yet the executor returns an empty
                // vec, which we fold to a (empty-key, 0) entry so the
                // wire shape stays uniform across "no docs" and
                // "matched some".
                let entries = if entries.is_empty() {
                    vec![SplitCountEntry {
                        key: Vec::new(),
                        count: 0,
                    }]
                } else {
                    entries
                        .into_iter()
                        .map(|e| SplitCountEntry {
                            key: Vec::new(),
                            count: e.count,
                        })
                        .collect()
                };
                Ok(DocumentCountResponse::Counts(entries))
            }
            DocumentCountMode::PerInValue => Ok(DocumentCountResponse::Counts(
                self.execute_document_count_per_in_value_no_proof(
                    contract_id,
                    request.document_type,
                    document_type_name,
                    request.where_clauses,
                    transaction,
                    platform_version,
                )?,
            )),
            DocumentCountMode::RangeNoProof => {
                let options = RangeCountOptions {
                    distinct: request.return_distinct_counts_in_range,
                    limit: request.limit,
                    start_after_split_key: request.start_after_split_key,
                    // `None` → ascending (BTreeMap natural order).
                    order_by_ascending: request.order_by_ascending.unwrap_or(true),
                };
                Ok(DocumentCountResponse::Counts(
                    self.execute_document_count_range_no_proof(
                        contract_id,
                        request.document_type,
                        document_type_name,
                        request.where_clauses,
                        options,
                        transaction,
                        platform_version,
                    )?,
                ))
            }
            DocumentCountMode::RangeProof => Ok(DocumentCountResponse::Proof(
                self.execute_document_count_range_proof(
                    contract_id,
                    request.document_type,
                    document_type_name,
                    request.where_clauses,
                    transaction,
                    platform_version,
                )?,
            )),
            DocumentCountMode::PointLookupProof => Ok(DocumentCountResponse::Proof(
                self.execute_document_count_point_lookup_proof(
                    request.raw_where_value,
                    request.contract,
                    request.document_type,
                    request.drive_config,
                    transaction,
                    platform_version,
                )?,
            )),
        }
    }
}
