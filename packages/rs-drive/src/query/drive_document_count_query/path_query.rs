//! Path-query builders for the count query.
//!
//! These are the **load-bearing prover/verifier-agreement boundary**:
//! the bytes these builders produce must match byte-for-byte between
//! the prover and the verifier, or the merk-root recomputation
//! fails. Touching anything here without updating both the
//! server-side prove executor AND the SDK's verifier path-query
//! reconstruction simultaneously is a bug waiting to happen.
//!
//! All three builders are gated `#[cfg(any(feature = "server",
//! feature = "verify"))]` so the verifier crate (which only enables
//! `verify`) can reach them via `DriveDocumentCountQuery::*` method
//! syntax.

#![cfg(any(feature = "server", feature = "verify"))]

use super::super::conditions::{WhereClause, WhereOperator};
use super::DriveDocumentCountQuery;
use crate::drive::RootTree;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::version::PlatformVersion;
use grovedb::{PathQuery, Query, QueryItem, SizedQuery};

impl DriveDocumentCountQuery<'_> {
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
    /// - `startsWith "p"` → `Range(serialize("p")..serialize("p") with
    ///   last byte +1)` — same byte-incremented half-open encoding the
    ///   normal docs path uses (see `conditions.rs:1129`'s `StartsWith`
    ///   arm). `value_shape_ok` constrains the prefix to `Value::Text`,
    ///   and valid UTF-8 never contains `0xFF`, so the `+1` doesn't
    ///   overflow for valid string keys; the unlikely 0xFF-tail case is
    ///   caught via `checked_add` and rejected with a clear error.
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
                let left_key = serialize(&clause.value)?;
                let mut right_key = left_key.clone();
                // Byte-increment the last byte to form the half-open
                // upper bound `[prefix, prefix+1)`. Mirrors the
                // normal-docs encoding in `conditions.rs:1129`'s
                // `StartsWith` arm; we use `checked_add` so the
                // pathological `0xFF`-tail input fails loudly instead
                // of wrapping silently (UTF-8 never contains 0xFF so
                // valid string keys never hit this).
                let last = right_key.last_mut().ok_or_else(|| {
                    Error::Query(QuerySyntaxError::InvalidStartsWithClause(
                        "startsWith prefix must have at least one byte",
                    ))
                })?;
                *last = last.checked_add(1).ok_or_else(|| {
                    Error::Query(QuerySyntaxError::InvalidStartsWithClause(
                        "startsWith prefix ends in 0xFF; cannot form half-open upper bound",
                    ))
                })?;
                QueryItem::Range(left_key..right_key)
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
    /// `PathQuery` for verification to recompute the same merk root.
    ///
    /// Aggregate-count specifically restricts prefix props to `Equal`:
    /// grovedb's `AggregateCountOnRange` primitive wraps a *single*
    /// inner range and emits one aggregate `u64` — there's no way for
    /// it to cartesian-fork over multiple In values at the merk
    /// layer. For per-distinct-value counts with In on prefix, use
    /// [`Self::distinct_count_path_query`] instead.
    ///
    /// Errors:
    /// - No range where-clause / multiple range where-clauses →
    ///   `InvalidWhereClauseComponents`
    /// - `In` on a prefix property → `InvalidWhereClauseComponents`
    ///   (aggregate primitive can't fork)
    /// - Missing prefix clause → `InvalidWhereClauseComponents`
    pub fn aggregate_count_path_query(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<PathQuery, Error> {
        let range_clause = self
            .where_clauses
            .iter()
            .find(|wc| Self::is_range_operator(wc.operator))
            .ok_or(Error::Query(
                QuerySyntaxError::InvalidWhereClauseComponents(
                    "aggregate_count_path_query requires a range where-clause",
                ),
            ))?;
        let query_item = self.range_clause_to_query_item(range_clause, platform_version)?;

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
                .ok_or(Error::Query(
                    QuerySyntaxError::InvalidWhereClauseComponents(
                        "aggregate-count proof: missing where clause for an index prefix property",
                    ),
                ))?;
            if clause.operator != WhereOperator::Equal {
                return Err(Error::Query(
                    QuerySyntaxError::InvalidWhereClauseComponents(
                        "aggregate-count proof: prefix properties must use `==` (no `in`); \
                         use `return_distinct_counts_in_range = true` for compound In-on-prefix \
                         queries",
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
            .ok_or(Error::Query(
                QuerySyntaxError::InvalidWhereClauseComponents(
                    "range_countable index must have at least one property",
                ),
            ))?
            .name;
        path.push(range_prop_name.as_bytes().to_vec());

        Ok(PathQuery::new_aggregate_count_on_range(path, query_item))
    }

    /// Build the grovedb `PathQuery` for a *regular* range query
    /// against this count query's `range_countable` index — the
    /// distinct-counts variant. Used by:
    /// - the server's prove-distinct executor
    ///   ([`Self::execute_distinct_count_with_proof`])
    /// - the server's no-proof range executor
    ///   ([`Self::execute_range_count_no_proof`])
    /// - the SDK's per-key-count verifier
    ///   ([`drive_proof_verifier::verify_distinct_count_proof`])
    ///
    /// **In-on-prefix support via grovedb subqueries.** Where
    /// [`Self::aggregate_count_path_query`] rejects In on prefix
    /// (the aggregate merk primitive can't cartesian-fork), this
    /// builder uses grovedb's native subquery primitive:
    ///
    /// - **Flat shape** (no In on prefix, only Equal): path includes
    ///   the range terminator; outer Query has the range item.
    /// - **Compound shape** (one In on prefix): path stops at the
    ///   In-bearing prop's property-name subtree; outer Query has
    ///   one `Key(value)` item per In value; `set_subquery_path`
    ///   carries any post-In Equal-clause `(name, value)` pairs plus
    ///   the terminator name; `set_subquery` is the range item.
    ///
    /// Both shapes return `(path, branched-or-flat Query)` and feed
    /// the same `grove_get_raw_path_query` / `get_proved_path_query`
    /// pipelines downstream. The compound shape replaces the
    /// pre-existing cartesian-fork loop in
    /// `execute_range_count_no_proof`.
    ///
    /// `limit` IS load-bearing for prove-path verification: the
    /// prover bounds the proof at `limit` matched keys, and the
    /// verifier must build the exact same `PathQuery` (including
    /// this cap) for the merk-root recomputation to match. The
    /// dispatcher pre-validates `limit ≤ max_query_limit` on the
    /// prove path, so unbounded queries can't reach this builder
    /// with `Some(...)` greater than the cap. The no-proof path
    /// passes `None` (full walk) so cross-In-fork merging sees
    /// every emitted element before the result-set-level limit is
    /// applied in post-processing.
    ///
    /// `left_to_right` controls grovedb's iteration direction:
    /// `true` (the default, used for ascending `order_by_ascending`)
    /// walks the range from low key to high key; `false` reverses.
    /// On the prove path this is load-bearing: the path query's
    /// `Query.left_to_right` is part of the serialized PathQuery
    /// bytes, so the prover and verifier must agree on the value or
    /// the merk-root recomputation fails. For compound queries the
    /// flag is applied to BOTH the outer In-keys Query and the
    /// inner range subquery, so descending iteration walks
    /// `(in_key_desc, key_desc)` tuples (matching what
    /// `RangeCountOptions::order_by_ascending = false` callers
    /// expect).
    ///
    /// Errors:
    /// - No range where-clause / multiple range where-clauses
    /// - Multiple In clauses on prefix props
    /// - Non-Equal-non-In operator on a prefix prop
    /// - Missing prefix clause
    pub fn distinct_count_path_query(
        &self,
        limit: Option<u16>,
        left_to_right: bool,
        platform_version: &PlatformVersion,
    ) -> Result<PathQuery, Error> {
        let range_clause = self
            .where_clauses
            .iter()
            .find(|wc| Self::is_range_operator(wc.operator))
            .ok_or(Error::Query(
                QuerySyntaxError::InvalidWhereClauseComponents(
                    "distinct_count_path_query requires a range where-clause",
                ),
            ))?;
        let range_item = self.range_clause_to_query_item(range_clause, platform_version)?;

        let prefix_props = &self.index.properties[..self.index.properties.len() - 1];
        let terminator_name = &self
            .index
            .properties
            .last()
            .ok_or(Error::Query(
                QuerySyntaxError::InvalidWhereClauseComponents(
                    "range_countable index must have at least one property",
                ),
            ))?
            .name;

        let mut base_path: Vec<Vec<u8>> = vec![
            vec![RootTree::DataContractDocuments as u8],
            self.contract_id.to_vec(),
            vec![1u8],
            self.document_type_name.as_bytes().to_vec(),
        ];

        // `Some(keys)` once an In clause has been encountered on a
        // prefix property. From that point on, subsequent Equal
        // clauses go into `subquery_path_extension` rather than
        // `base_path`. Only one In allowed (multiple Ins would
        // multiply the fork count beyond what a single Query can
        // express via `set_subquery_path`).
        let mut in_outer_keys: Option<Vec<Vec<u8>>> = None;
        let mut subquery_path_extension: Vec<Vec<u8>> = vec![];

        for prop in prefix_props {
            let clause = self
                .where_clauses
                .iter()
                .find(|wc| wc.field == prop.name)
                .ok_or(Error::Query(
                    QuerySyntaxError::InvalidWhereClauseComponents(
                        "distinct_count_path_query: missing where clause for an index \
                         prefix property",
                    ),
                ))?;

            match clause.operator {
                WhereOperator::Equal => {
                    let serialized = self.document_type.serialize_value_for_key(
                        prop.name.as_str(),
                        &clause.value,
                        platform_version,
                    )?;
                    if in_outer_keys.is_some() {
                        subquery_path_extension.push(prop.name.as_bytes().to_vec());
                        subquery_path_extension.push(serialized);
                    } else {
                        base_path.push(prop.name.as_bytes().to_vec());
                        base_path.push(serialized);
                    }
                }
                WhereOperator::In => {
                    if in_outer_keys.is_some() {
                        return Err(Error::Query(
                            QuerySyntaxError::InvalidWhereClauseComponents(
                                "distinct_count_path_query: at most one `In` clause is supported \
                                 on prefix properties",
                            ),
                        ));
                    }
                    // Path stops at the In-bearing prop's property-
                    // name subtree; outer Query lives at that level.
                    base_path.push(prop.name.as_bytes().to_vec());
                    let in_values = clause.in_values().into_data_with_error()??;
                    let keys: Vec<Vec<u8>> = in_values
                        .iter()
                        .map(|v| {
                            self.document_type.serialize_value_for_key(
                                prop.name.as_str(),
                                v,
                                platform_version,
                            )
                        })
                        .collect::<Result<_, _>>()?;
                    in_outer_keys = Some(keys);
                }
                _ => {
                    return Err(Error::Query(
                        QuerySyntaxError::InvalidWhereClauseComponents(
                            "distinct_count_path_query: prefix properties must use `==` or `in`",
                        ),
                    ));
                }
            }
        }

        match in_outer_keys {
            None => {
                // Flat shape — path includes terminator, single
                // range-only Query.
                base_path.push(terminator_name.as_bytes().to_vec());
                let mut query = Query::new_with_direction(left_to_right);
                query.insert_item(range_item);
                Ok(PathQuery::new(
                    base_path,
                    SizedQuery::new(query, limit, None),
                ))
            }
            Some(keys) => {
                // Compound shape — outer Query has one Key per In
                // value at the In-bearing prop's property-name
                // subtree. `subquery_path` carries any post-In Equal
                // pairs + terminator. Subquery is the range item.
                //
                // `left_to_right` applies to BOTH the outer Query
                // and the subquery so descending iteration walks
                // `(in_key_desc, key_desc)` tuples — otherwise we'd
                // get e.g. In keys ascending but per-fork terminator
                // values descending, which is a weird order no
                // user would expect.
                let mut outer_query = Query::new_with_direction(left_to_right);
                for key in keys {
                    outer_query.insert_key(key);
                }
                subquery_path_extension.push(terminator_name.as_bytes().to_vec());

                let mut subquery = Query::new_with_direction(left_to_right);
                subquery.insert_item(range_item);

                outer_query.set_subquery_path(subquery_path_extension);
                outer_query.set_subquery(subquery);

                Ok(PathQuery::new(
                    base_path,
                    SizedQuery::new(outer_query, limit, None),
                ))
            }
        }
    }
}
