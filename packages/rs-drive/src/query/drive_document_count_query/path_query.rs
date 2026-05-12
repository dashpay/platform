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
                    let mut keys: Vec<Vec<u8>> = in_values
                        .iter()
                        .map(|v| {
                            self.document_type.serialize_value_for_key(
                                prop.name.as_str(),
                                v,
                                platform_version,
                            )
                        })
                        .collect::<Result<_, _>>()?;
                    // Sort the serialized In keys lex-ascending before
                    // building the outer Query. This is load-bearing
                    // for both correctness and DoS-resistance:
                    // - **Order parity**: grovedb iterates `Key` items
                    //   in insert order. Without sorting, the emitted
                    //   `(in_key, key)` tuples come out in user-input
                    //   order on the prefix dimension, which diverges
                    //   from the documented lex-asc order contract on
                    //   the no-proof distinct path (which sorts post-
                    //   walk) and forces a per-side sort step.
                    // - **`left_to_right`-driven descent**: with sorted
                    //   keys, `left_to_right = false` walks the outer
                    //   In dimension lex-descending — what the caller
                    //   asked for. Without the sort, descending
                    //   `left_to_right` just reverses user-input
                    //   order, which is gibberish.
                    // - **Pushed-limit safety**: callers that push the
                    //   path-query limit (no-proof distinct mode) get
                    //   the bottom-N or top-N entries by lex order,
                    //   which is the documented limit-on-distinct
                    //   semantics. With unsorted keys, the path-query
                    //   limit would give the first-N entries in user-
                    //   input order — useless for distinct pagination.
                    //
                    // Both the prover and the verifier go through this
                    // builder, so the byte-equality contract still
                    // holds — the sort happens identically on both
                    // sides.
                    keys.sort();
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

    /// Build the grovedb `PathQuery` for a point-lookup count proof
    /// against a `countable: true` index. Returns one element per
    /// covered branch — the `CountTree` element at
    /// `[..., last_field, last_value, 0]` whose `count_value` is the
    /// per-branch document count.
    ///
    /// Shared between the server-side prove path
    /// ([`Self::execute_point_lookup_count_with_proof`]) and the
    /// client-side verify path
    /// ([`Self::verify_point_lookup_count_proof`]). Both sides must
    /// produce the *exact same* `PathQuery` for the merk-root
    /// recomputation to match.
    ///
    /// ## Shape support
    ///
    /// The builder requires the where clauses to **fully cover** the
    /// index — every property in `self.index.properties` must have a
    /// matching `Equal` or `In` clause. Partial-coverage shapes
    /// (where some index properties have no matching clause) require
    /// a recursive subquery enumeration that this builder does not
    /// implement (and that the strict picker already rejects upstream).
    ///
    /// **`In` may appear at any position in the index.** Equal
    /// clauses before the In contribute to `base_path`; Equal clauses
    /// after the In feed `set_subquery_path` on the outer Query so the
    /// descent under each matched In value lands at the right
    /// CountTree leaf. At most one `In` clause per query (multiple
    /// would cartesian-fork beyond what a single `set_subquery`
    /// expresses).
    ///
    /// This is **more permissive than the regular document query
    /// path's `Index::matches` rule** (`packages/rs-dpp/src/
    /// data_contract/document_type/index/mod.rs:503`), which restricts
    /// `In` to the last or before-last index property because its
    /// path-construction code positionally zips intermediate index
    /// names with Equal-clause values (see
    /// `DriveDocumentQuery::get_non_primary_key_path_query`). The
    /// count path doesn't have that constraint: it's a pure CountTree
    /// element lookup with no document-key terminator descent, no
    /// `order_by` interpretation, and no `limit/offset` semantics, so
    /// `set_subquery_path` with an arbitrary trailing tail just
    /// works. Both no-proof ([`Self::execute_no_proof`]) and prove
    /// ([`Self::execute_point_lookup_count_with_proof`]) executors
    /// route through this single builder, so they accept the same
    /// query shapes by construction.
    ///
    /// Output shapes:
    /// - **Equal-only, fully covered**: flat path query at
    ///   `[..., last_field, last_value]` with a single `Key([0])`
    ///   item. Returns one element (the CountTree).
    /// - **Equal prefix + `In` (any position) [+ trailing Equals]**:
    ///   compound query with `base_path` ending at the In-bearing
    ///   property's property-name subtree (so any Equal clauses
    ///   *before* the In are baked into `base_path`); outer Query
    ///   has one `Key` per In value (sorted lex-asc for prove/no-
    ///   proof parity and pushed-limit safety — same convention as
    ///   [`Self::distinct_count_path_query`]). `set_subquery_path`
    ///   carries the post-In Equal clauses' `(prop_name,
    ///   serialized_value)` pairs in index order, and the subquery's
    ///   `Key([0])` picks off the CountTree at the resolved leaf
    ///   under each matched In branch. Same `set_subquery_path` +
    ///   `set_subquery` mechanism as [`Self::distinct_count_path_query`]
    ///   uses for compound In-on-prefix range counts.
    ///
    /// ## Errors
    ///
    /// Rejects shapes the builder doesn't support:
    /// - Partial coverage (uncovered index property)
    /// - More than one `In` clause
    /// - Any non-`Equal` / non-`In` operator (defense-in-depth; mode
    ///   detection already filters these out)
    pub fn point_lookup_count_path_query(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<PathQuery, Error> {
        if self.index.properties.is_empty() {
            return Err(Error::Query(
                QuerySyntaxError::InvalidWhereClauseComponents(
                    "point_lookup_count_path_query: index must have at least one property",
                ),
            ));
        }

        let mut base_path: Vec<Vec<u8>> = vec![
            vec![RootTree::DataContractDocuments as u8],
            self.contract_id.to_vec(),
            vec![1u8],
            self.document_type_name.as_bytes().to_vec(),
        ];

        // `in_outer_keys` is populated when we encounter the (single)
        // `In` clause. Equal clauses *before* the In contribute to
        // `base_path`; Equal clauses *after* the In feed
        // `subquery_path_extension`, which becomes the outer Query's
        // `set_subquery_path` — i.e., the descent under each matched
        // In value walks `[trailing_field_1, trailing_value_1, ...,
        // trailing_field_n, trailing_value_n]` before the
        // `Key([0])` subquery picks off the CountTree leaf.
        //
        // No position restriction on the In clause: any index
        // position works because the count path doesn't have the
        // positional path-construction assumption the regular
        // document query path makes (see this method's docstring for
        // the divergence rationale).
        let mut in_outer_keys: Option<Vec<Vec<u8>>> = None;
        let mut subquery_path_extension: Vec<Vec<u8>> = vec![];

        for prop in self.index.properties.iter() {
            let clause = self
                .where_clauses
                .iter()
                .find(|wc| wc.field == prop.name)
                .ok_or_else(|| {
                    Error::Query(QuerySyntaxError::InvalidWhereClauseComponents(
                        "prove count requires the where clauses to fully cover the \
                         countable index; one or more index properties have no \
                         matching `==` or `in` clause — use a more specific index \
                         (define a `countable: true` index whose properties exactly \
                         match the clauses) or use `prove=false`",
                    ))
                })?;

            match clause.operator {
                WhereOperator::Equal => {
                    let serialized = self.document_type.serialize_value_for_key(
                        prop.name.as_str(),
                        &clause.value,
                        platform_version,
                    )?;
                    if in_outer_keys.is_some() {
                        // Trailing Equal after the (already-seen) In:
                        // descend through it as part of the subquery
                        // path. Any number of these may accumulate —
                        // one for each Equal that sits *after* the In
                        // in the index ordering.
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
                                "prove count: at most one `in` clause is supported on \
                                 the covering countable index",
                            ),
                        ));
                    }
                    // Stops `base_path` at the In-bearing property's
                    // property-name subtree; outer Query lives at
                    // that level. Any trailing Equal property then
                    // routes through `subquery_path_extension`.
                    base_path.push(prop.name.as_bytes().to_vec());
                    let in_values = clause.in_values().into_data_with_error()??;
                    let mut keys: Vec<Vec<u8>> = in_values
                        .iter()
                        .map(|v| {
                            self.document_type.serialize_value_for_key(
                                prop.name.as_str(),
                                v,
                                platform_version,
                            )
                        })
                        .collect::<Result<_, _>>()?;
                    // Sort lex-asc for prove/no-proof entry-order
                    // parity and so the pushed-limit (if any) gives
                    // the documented "first N by lex" semantics.
                    // Same convention as `distinct_count_path_query`.
                    keys.sort();
                    in_outer_keys = Some(keys);
                }
                _ => {
                    return Err(Error::Query(
                        QuerySyntaxError::InvalidWhereClauseComponents(
                            "point_lookup_count_path_query: index properties must use \
                             `==` or `in`",
                        ),
                    ));
                }
            }
        }

        // CountTree storage convention: the count lives at the `[0]`
        // child of the value tree. See the book's "Count Trees and
        // Provable Counts" chapter for the layout.
        const COUNT_TREE_KEY: u8 = 0;

        match in_outer_keys {
            None => {
                // Equal-only, fully covered. `base_path` ends at
                // `[..., last_field, last_value]`; query asks for the
                // single key `[0]` (the CountTree element).
                let mut query = Query::new();
                query.insert_key(vec![COUNT_TREE_KEY]);
                Ok(PathQuery::new(
                    base_path,
                    SizedQuery::new(query, None, None),
                ))
            }
            Some(keys) => {
                // Compound shape. `base_path` ends at the In-bearing
                // property's property-name subtree; the outer Query
                // enumerates serialized In values; the subquery
                // descends to the CountTree element under each
                // matched In value.
                //
                // `subquery_path_extension` carries 0..N segments,
                // one `(prop_name, serialized_value)` pair per Equal
                // clause that sits *after* the In in the index
                // ordering:
                // - **In on last property**: `subquery_path_extension`
                //   is empty; subquery's `Key([0])` runs directly
                //   under each In value's value tree.
                // - **In with any number of trailing Equals**:
                //   `set_subquery_path` consumes those segments so
                //   the subquery descends through them before grabbing
                //   the `Key([0])` CountTree at the resolved leaf.
                let mut outer_query = Query::new();
                for key in keys {
                    outer_query.insert_key(key);
                }
                let mut subquery = Query::new();
                subquery.insert_key(vec![COUNT_TREE_KEY]);
                if !subquery_path_extension.is_empty() {
                    outer_query.set_subquery_path(subquery_path_extension);
                }
                outer_query.set_subquery(subquery);

                Ok(PathQuery::new(
                    base_path,
                    SizedQuery::new(outer_query, None, None),
                ))
            }
        }
    }

    /// Build the grovedb `PathQuery` for proving the document type's
    /// primary-key `CountTree` element at `[contract_doc, contract_id,
    /// 1, doctype, 0]`. Used for unfiltered total counts when the
    /// document type has `documents_countable: true` — the
    /// type-level CountTree's `count_value` IS the total document
    /// count, no index walk needed.
    ///
    /// Shared between the server-side prove path
    /// ([`Drive::execute_document_count_point_lookup_proof`]'s
    /// documents_countable fast path) and the client-side verify path
    /// ([`Self::verify_primary_key_count_tree_proof`]). Both sides
    /// produce the exact same `PathQuery` for merk-root recomputation.
    ///
    /// Free function rather than a method on `DriveDocumentCountQuery`
    /// because the documents_countable case isn't tied to any index —
    /// it operates at the doctype level directly.
    pub fn primary_key_count_tree_path_query(
        contract_id: [u8; 32],
        document_type_name: &str,
    ) -> PathQuery {
        let path = vec![
            vec![RootTree::DataContractDocuments as u8],
            contract_id.to_vec(),
            vec![1u8],
            document_type_name.as_bytes().to_vec(),
        ];
        let mut query = Query::new();
        query.insert_key(vec![0]);
        PathQuery::new(path, SizedQuery::new(query, None, None))
    }
}
