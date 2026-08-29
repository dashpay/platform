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
        // Shared helper for all four `between*` operators. The
        // operator the caller used (`between`, `betweenExcludeBounds`,
        // etc.) is not woven into error messages because
        // `InvalidWhereClauseComponents` takes `&'static str` — a
        // String-typed error variant would let us do that, but the
        // existing static-string contract is fine to live with: the
        // arm name (`WhereOperator::Between` etc.) is visible in
        // backtraces if a malformed payload reaches this far, and
        // mode detection has already filtered out non-range operators.
        let serialize_pair = || -> Result<(Vec<u8>, Vec<u8>), Error> {
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
                let (a, b) = serialize_pair()?;
                QueryItem::RangeInclusive(a..=b)
            }
            WhereOperator::BetweenExcludeBounds => {
                let (a, b) = serialize_pair()?;
                QueryItem::RangeAfterTo(a..b)
            }
            WhereOperator::BetweenExcludeLeft => {
                let (a, b) = serialize_pair()?;
                QueryItem::RangeAfterToInclusive(a..=b)
            }
            WhereOperator::BetweenExcludeRight => {
                let (a, b) = serialize_pair()?;
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
    /// side verify path (the SDK's `FromProof<DocumentQuery>` for
    /// `DocumentCount`, via the shared `verify_aggregate_count`
    /// helper). Both sides must produce the *exact same* `PathQuery`
    /// for verification to recompute the same merk root.
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
                         use a two-field `group_by = [in_field, range_field]` for compound \
                         In-on-prefix queries",
                    ),
                ));
            }
            path.push(self.index.level_key_for_property(&prop.name).into_bytes());
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
        path.push(
            self.index
                .level_key_for_property(range_prop_name)
                .into_bytes(),
        );

        Ok(PathQuery::new_aggregate_count_on_range(path, query_item))
    }

    /// Build the grovedb `PathQuery` for a **carrier**
    /// `AggregateCountOnRange` proof — one outer Key per `In`
    /// value, each terminating in an ACOR boundary walk over the
    /// per-branch range subtree. Returns one `(in_key, u64)` pair
    /// per resolved In branch via
    /// [`grovedb::GroveDb::query_aggregate_count_per_key`] (no-
    /// proof) and
    /// [`grovedb::GroveDb::verify_aggregate_count_query_per_key`]
    /// (verify).
    ///
    /// Required where-clause shape (validated upstream by
    /// [`Self::detect_mode`] routing to
    /// [`DocumentCountMode::RangeAggregateCarrierProof`]):
    /// - Exactly one `In` clause on the In-property
    /// - Exactly one range clause on the *terminator* property of
    ///   a `range_countable: true` index whose first property is
    ///   the In-property
    /// - Any prefix properties between In and range must use
    ///   `==` (mirror of [`Self::aggregate_count_path_query`]'s
    ///   non-In prefix rule)
    ///
    /// Path-query structure:
    /// - Outer path stops one level above the In-bearing property
    ///   subtree's children (`@/doc_prefix/0x01/doctype/<In-prop>`).
    /// - Outer Query: `Key(in_value_0)`, `Key(in_value_1)`, … in
    ///   lex-asc serialized order (grovedb's multi-key walker
    ///   invariant).
    /// - `subquery_path`: the terminator property name (and any
    ///   trailing `==` clause names between In and range, in
    ///   index order).
    /// - `subquery`: `Query::new_aggregate_count_on_range(range_item)`.
    ///
    /// Enabled by [grovedb PR #663](https://github.com/dashpay/grovedb/pull/663).
    /// Before that PR, `AggregateCountOnRange` was required to be
    /// the only item in its query and could not appear under a
    /// `subquery` field — the dispatcher rejected this shape with
    /// "range count queries with an `in` clause are not supported on
    /// the aggregate prove path".
    ///
    /// Errors:
    /// - No range where-clause / multiple range where-clauses →
    ///   `InvalidWhereClauseComponents`
    /// - No In where-clause → `InvalidWhereClauseComponents`
    /// - In on a non-prefix property → `InvalidWhereClauseComponents`
    /// - Prefix property between In and range uses non-Equal →
    ///   `InvalidWhereClauseComponents`
    pub fn carrier_aggregate_count_path_query(
        &self,
        limit: Option<u16>,
        left_to_right: bool,
        platform_version: &PlatformVersion,
    ) -> Result<PathQuery, Error> {
        // The terminator property (last in the index) carries the
        // ACOR target range. The "carrier" property — the one whose
        // clause becomes the outer Query items — is either:
        // - An `In` clause (G7 shape: one Key per In value)
        // - A range clause on a prefix prop (G8 shape: one QueryItem
        //   bounding the outer range, with `SizedQuery::limit` capping
        //   how many outer matches the carrier walks — see
        //   [grovedb PR #664](https://github.com/dashpay/grovedb/pull/664))
        //
        // The terminator's clause must be a range and is converted to
        // the inner ACOR `QueryItem`. Any properties between the
        // carrier and the terminator must use `==` and extend the
        // subquery_path.
        let terminator_prop_name = &self
            .index
            .properties
            .last()
            .ok_or(Error::Query(
                QuerySyntaxError::InvalidWhereClauseComponents(
                    "range_countable index must have at least one property",
                ),
            ))?
            .name;
        let terminator_clause = self
            .where_clauses
            .iter()
            .find(|wc| wc.field == *terminator_prop_name && Self::is_range_operator(wc.operator))
            .ok_or(Error::Query(
                QuerySyntaxError::InvalidWhereClauseComponents(
                    "carrier_aggregate_count_path_query requires a range where-clause on the \
                     terminator property of the chosen index",
                ),
            ))?;
        let inner_range_item =
            self.range_clause_to_query_item(terminator_clause, platform_version)?;

        let mut base_path: Vec<Vec<u8>> = vec![
            vec![RootTree::DataContractDocuments as u8],
            self.contract_id.to_vec(),
            vec![1u8],
            self.document_type_name.as_bytes().to_vec(),
        ];
        let mut subquery_path_extension: Vec<Vec<u8>> = vec![];

        // Carrier clause state: either `None` (not seen yet, still on
        // the `==`-prefix run), `Some(In)` (G7), or `Some(Range)` (G8).
        enum Carrier {
            Pending,
            In(WhereClause),
            Range(WhereClause),
        }
        let mut carrier = Carrier::Pending;
        let prefix_and_carrier_props = &self.index.properties[..self.index.properties.len() - 1];

        for prop in prefix_and_carrier_props {
            let clause = self
                .where_clauses
                .iter()
                .find(|wc| wc.field == prop.name)
                .ok_or(
                Error::Query(QuerySyntaxError::InvalidWhereClauseComponents(
                    "carrier-aggregate proof: missing where clause for an index prefix property",
                )),
            )?;
            match (&carrier, clause.operator) {
                (Carrier::Pending, WhereOperator::Equal) => {
                    base_path.push(self.index.level_key_for_property(&prop.name).into_bytes());
                    base_path.push(self.document_type.serialize_value_for_key(
                        prop.name.as_str(),
                        &clause.value,
                        platform_version,
                    )?);
                }
                (Carrier::Pending, WhereOperator::In) => {
                    base_path.push(self.index.level_key_for_property(&prop.name).into_bytes());
                    carrier = Carrier::In(clause.clone());
                }
                (Carrier::Pending, op) if Self::is_range_operator(op) => {
                    base_path.push(self.index.level_key_for_property(&prop.name).into_bytes());
                    carrier = Carrier::Range(clause.clone());
                }
                (Carrier::In(_) | Carrier::Range(_), WhereOperator::Equal) => {
                    subquery_path_extension
                        .push(self.index.level_key_for_property(&prop.name).into_bytes());
                    subquery_path_extension.push(self.document_type.serialize_value_for_key(
                        prop.name.as_str(),
                        &clause.value,
                        platform_version,
                    )?);
                }
                (Carrier::In(_) | Carrier::Range(_), _) => {
                    return Err(Error::Query(
                        QuerySyntaxError::InvalidWhereClauseComponents(
                            "carrier-aggregate proof: at most one carrier clause (In or range) \
                         is supported on prefix properties; subsequent prefix clauses must \
                         use `==`",
                        ),
                    ));
                }
                _ => {
                    return Err(Error::Query(
                        QuerySyntaxError::InvalidWhereClauseComponents(
                            "carrier-aggregate proof: prefix property operator unsupported",
                        ),
                    ));
                }
            }
        }
        subquery_path_extension.push(
            self.index
                .level_key_for_property(terminator_prop_name)
                .into_bytes(),
        );

        let mut outer_query = Query::new_with_direction(left_to_right);
        match carrier {
            Carrier::Pending => {
                return Err(Error::Query(
                    QuerySyntaxError::InvalidWhereClauseComponents(
                        "carrier-aggregate proof: an In or range clause must appear on a prefix \
                     property of the chosen index to act as the carrier dimension",
                    ),
                ));
            }
            Carrier::In(in_clause) => {
                // Build one Key per In value, sorted lex-ascending
                // (grovedb's multi-key walker invariant per PR #663).
                let in_values = in_clause.in_values().into_data_with_error()??;
                let mut serialized_in_keys: Vec<Vec<u8>> = in_values
                    .iter()
                    .map(|v| {
                        self.document_type.serialize_value_for_key(
                            in_clause.field.as_str(),
                            v,
                            platform_version,
                        )
                    })
                    .collect::<Result<_, _>>()?;
                serialized_in_keys.sort();
                serialized_in_keys.dedup();
                for key in serialized_in_keys {
                    outer_query.insert_key(key);
                }
            }
            Carrier::Range(range_clause) => {
                // Single QueryItem bounding the outer range. The
                // carrier walks this range and emits one `(key, u64)`
                // pair per matched outer key.
                let outer_range_item =
                    self.range_clause_to_query_item(&range_clause, platform_version)?;
                outer_query.items.push(outer_range_item);
            }
        }
        outer_query.set_subquery_path(subquery_path_extension);
        outer_query.set_subquery(Query::new_aggregate_count_on_range(inner_range_item));

        // `SizedQuery::limit` is permitted on carriers as of grovedb
        // PR #664; for In-outer carriers the |IN| array already
        // bounds the result so `limit` is typically `None`, but for
        // Range-outer carriers `limit` caps the outer walk and is
        // load-bearing for proof bytes.
        Ok(PathQuery::new(
            base_path,
            SizedQuery::new(outer_query, limit, None),
        ))
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
                        subquery_path_extension
                            .push(self.index.level_key_for_property(&prop.name).into_bytes());
                        subquery_path_extension.push(serialized);
                    } else {
                        base_path.push(self.index.level_key_for_property(&prop.name).into_bytes());
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
                    base_path.push(self.index.level_key_for_property(&prop.name).into_bytes());
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
    /// covered branch whose `count_value` is the per-branch document
    /// count.
    ///
    /// Shared between the server-side prove path
    /// ([`Self::execute_point_lookup_count_with_proof`]) and the
    /// client-side verify path
    /// ([`Self::verify_point_lookup_count_proof`]). Both sides must
    /// produce the *exact same* `PathQuery` for the merk-root
    /// recomputation to match.
    ///
    /// ## Two terminator shapes depending on `range_countable`
    ///
    /// The proof's terminal element is at one of two layers, picked
    /// from [`Index::range_countable`]:
    ///
    /// - **Normal `countable: true`** (NOT `range_countable`): the
    ///   terminator's value tree is a `NormalTree`, and the doc-count
    ///   `CountTree` sits inside it at the conventional `[0]` child.
    ///   Proof targets `[..., last_field, last_value, 0]`.
    /// - **`range_countable: true`**: the terminator's value tree is
    ///   itself a `CountTree` (continuation property-name subtrees
    ///   sit beneath as `Element::NonCounted` so they don't pollute
    ///   the parent count — see `add_indices_for_index_level_for_contract_operations_v0`).
    ///   The value tree's own `count_value_or_default()` already IS
    ///   the per-branch doc count, so the proof targets the value
    ///   tree directly at `[..., last_field, last_value]` and saves
    ///   one merk-path layer per covered branch.
    ///
    /// Concretely the optimization replaces a trailing `Key([0])`
    /// with `Key(last_value)` against `[..., last_field]` (Equal-
    /// only, no In) — or against the In-bearing prop's property-name
    /// subtree (In on terminator) — or replaces the trailing pair in
    /// `set_subquery_path` (In on prefix + trailing Equals that reach
    /// the terminator). The query shape stays in the same Query/
    /// subquery topology so byte-equality across prover and verifier
    /// is preserved by construction.
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
    /// Output shapes (`countable` / `range_countable` differ only in
    /// whether the trailing `Key([0])` is replaced by `Key(last_value)`):
    /// - **Equal-only, fully covered**:
    ///   - `countable`: path `[..., last_field, last_value]`, single `Key([0])`.
    ///   - `range_countable`: path `[..., last_field]`, single
    ///     `Key(last_value)`.
    /// - **Equal prefix + `In` (any position) [+ trailing Equals]**:
    ///   compound query with `base_path` ending at the In-bearing
    ///   property's property-name subtree (Equal clauses before the
    ///   In are baked into `base_path`); outer Query has one `Key`
    ///   per In value (sorted lex-asc for prove/no-proof parity and
    ///   pushed-limit safety — same convention as
    ///   [`Self::distinct_count_path_query`]).
    ///   - **In on terminator**:
    ///     - `countable`: subquery `Key([0])` under each In value's
    ///       value tree (`set_subquery_path` unset).
    ///     - `range_countable`: outer `Key`s already point at the
    ///       CountTree value trees themselves; no subquery is set.
    ///   - **In on a prefix + trailing Equals reaching the
    ///     terminator**: `set_subquery_path` carries the post-In
    ///     Equal `(name, value)` pairs in index order:
    ///     - `countable`: full pairs, subquery `Key([0])`.
    ///     - `range_countable`: last pair's `value` is hoisted out as
    ///       the subquery's single `Key(value)`; `set_subquery_path`
    ///       ends at the terminator's property-name segment.
    ///
    /// - **Prefix-to-last** (every property except the LAST covered, on
    ///   a `range_countable` index): the selector is the terminal
    ///   property-name tree's own element — a count-bearing tree whose
    ///   aggregate is the whole-prefix total — addressed by its level
    ///   key one layer below the last pin's value tree:
    ///   - Equal-only pins: path `[..., pin_field, pin_value]`, single
    ///     `Key(last_field)`.
    ///   - With an `In` pin: same compound shape as above, with the
    ///     post-In Equal `(name, value)` pairs in `set_subquery_path`
    ///     (all full pairs — nothing is hoisted) and the subquery
    ///     `Key(last_field)`.
    ///
    /// ## Errors
    ///
    /// Rejects shapes the builder doesn't support:
    /// - Partial coverage (an uncovered index property, except the
    ///   trailing free property of the prefix-to-last form on a
    ///   `range_countable` index)
    /// - More than one `In` clause
    /// - Any non-`Equal` / non-`In` operator (defense-in-depth; mode
    ///   detection already filters these out)
    ///
    /// [`Index::range_countable`]: dpp::data_contract::document_type::index::Index::range_countable
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
        // selector subquery (either `Key([0])` for normal countable
        // or a `Key(terminator_value)` lift for range_countable —
        // see the post-loop selector decision below) picks off the
        // count-bearing element.
        //
        // No position restriction on the In clause: any index
        // position works because the count path doesn't have the
        // positional path-construction assumption the regular
        // document query path makes (see this method's docstring for
        // the divergence rationale).
        let mut in_outer_keys: Option<Vec<Vec<u8>>> = None;
        let mut subquery_path_extension: Vec<Vec<u8>> = vec![];
        // Set when the LAST property carries no clause — the
        // prefix-to-last form on a `range_countable` index. The count is
        // then the terminal property-name tree's own element aggregate
        // (the whole-prefix total), addressed by this level key from the
        // last pin's value tree.
        let mut prefix_to_last_key: Option<Vec<u8>> = None;

        for (position, prop) in self.index.properties.iter().enumerate() {
            // The path segment is the level key — grid-qualified for a
            // time-range index's first property — while the clause lookup
            // and value serialization stay on the bare property name.
            let level_key = self.index.level_key(position, &prop.name);
            let Some(clause) = self.where_clauses.iter().find(|wc| wc.field == prop.name) else {
                if position + 1 == self.index.properties.len() && self.index.range_countable {
                    prefix_to_last_key = Some(level_key.into_bytes());
                    break;
                }
                return Err(Error::Query(
                    QuerySyntaxError::InvalidWhereClauseComponents(
                        "prove count requires the where clauses to cover the countable \
                     index; one or more index properties have no matching `==` or \
                     `in` clause — only the LAST property of a `rangeCountable: \
                     true` index may be left free (the prefix-to-last form). Use a \
                     more specific index (define a `countable: true` index whose \
                     properties exactly match the clauses) or use `prove=false`",
                    ),
                ));
            };

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
                        subquery_path_extension.push(level_key.as_bytes().to_vec());
                        subquery_path_extension.push(serialized);
                    } else {
                        base_path.push(level_key.as_bytes().to_vec());
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
                    base_path.push(self.index.level_key_for_property(&prop.name).into_bytes());
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

        // Whether the terminator's value tree is itself a `CountTree`
        // (carries the per-branch doc count directly) vs. a
        // `NormalTree` whose `[0]` child is the `CountTree`. Drives
        // the selector-element decision below.
        //
        // The insertion side
        // (`add_indices_for_index_level_for_contract_operations_v0`)
        // makes the terminator value tree a `CountTree` for **any**
        // countable index — not just `range_countable: true`. Both
        // tiers (`Countable` and `CountableAllowingOffset`) layout
        // the value tree the same way: a `CountTree` whose count
        // equals the `[0]` ref-bucket's doc count (continuations
        // wrapped `NonCounted` so they don't pollute the parent).
        // `range_countable` only additionally upgrades the
        // property-name tree to `ProvableCountTree` for
        // `AggregateCountOnRange` queries — that's orthogonal to the
        // point-lookup proof shape.
        //
        // So gate the optimization on `countable.is_countable()`:
        // every countable index uses the compact shape. The picker
        // upstream already requires the index to be countable to be
        // selected (`find_countable_index_for_where_clauses` / the
        // range_countable picker for range shapes), so reaching this
        // builder with a non-countable index would be a bug — but
        // we keep the gate explicit for clarity.
        //
        // The loop above already enforces full coverage of every
        // index property, so the terminator is always proven; this
        // flag is the only differentiator between the two output
        // shapes.
        let count_tree_terminator = self.index.countable.is_countable();

        // CountTree storage convention for non-countable indexes
        // (defensive — picker upstream filters these out): the count
        // lives at the `[0]` child of the value
        // tree. See the book's "Count Trees and Provable Counts"
        // chapter for the layout.
        const COUNT_TREE_KEY: u8 = 0;

        // Prefix-to-last selector: the terminal property-name tree —
        // count-bearing under `rangeCountable`, its aggregate the sum of
        // every last-property value tree's count, i.e. the whole-prefix
        // total — read as one element by its level key from the last
        // pin's value tree. Structurally the `Key([0])` shape with the
        // terminal level key in place of the bucket key: nothing is
        // hoisted, so the `In` branches keep their full trailing pairs
        // in `set_subquery_path`.
        if let Some(terminal_level_key) = prefix_to_last_key {
            return Ok(match in_outer_keys {
                None => {
                    let mut query = Query::new();
                    query.insert_key(terminal_level_key);
                    PathQuery::new(base_path, SizedQuery::new(query, None, None))
                }
                Some(keys) => {
                    let mut outer_query = Query::new();
                    for key in keys {
                        outer_query.insert_key(key);
                    }
                    let mut subquery = Query::new();
                    subquery.insert_key(terminal_level_key);
                    if !subquery_path_extension.is_empty() {
                        outer_query.set_subquery_path(subquery_path_extension);
                    }
                    outer_query.set_subquery(subquery);
                    PathQuery::new(base_path, SizedQuery::new(outer_query, None, None))
                }
            });
        }

        match in_outer_keys {
            None => {
                // Equal-only, fully covered.
                //
                // - normal countable: `base_path` ends at
                //   `[..., last_field, last_value]`; query asks for
                //   the single key `[0]` (the CountTree under the
                //   value tree).
                // - `range_countable`: peel the trailing `last_value`
                //   off `base_path` and use it as the query's Key.
                //   The resolved element is the value tree itself
                //   (a CountTree), and its `count_value_or_default()`
                //   is the per-branch count — one merk layer shorter
                //   per resolved branch than the `[0]` shape.
                let mut query = Query::new();
                if count_tree_terminator {
                    // The Equal loop always pushes (name, value) per
                    // prop, so `base_path` has at least the trailing
                    // serialized `last_value` to lift. The expect()
                    // here would fire only if the loop above changed
                    // its push contract — a load-bearing invariant
                    // checked by every test in this module that
                    // routes through this builder.
                    let last_value = base_path.pop().expect(
                        "Equal-only loop pushes (name, value) per prop; \
                         base_path must hold the terminator's serialized value",
                    );
                    query.insert_key(last_value);
                } else {
                    query.insert_key(vec![COUNT_TREE_KEY]);
                }
                Ok(PathQuery::new(
                    base_path,
                    SizedQuery::new(query, None, None),
                ))
            }
            Some(keys) => {
                // Compound shape. `base_path` ends at the In-bearing
                // property's property-name subtree; the outer Query
                // enumerates serialized In values; the subquery
                // (when present) descends from each matched In value
                // to the count-bearing element.
                //
                // `subquery_path_extension` carries 0..N segments,
                // one `(prop_name, serialized_value)` pair per Equal
                // clause that sits *after* the In in the index
                // ordering. The exact subquery topology depends on
                // both whether trailing Equals exist AND whether the
                // terminator is range_countable; see the inline
                // branches below.
                let mut outer_query = Query::new();
                for key in keys {
                    outer_query.insert_key(key);
                }

                if subquery_path_extension.is_empty() {
                    // **In on the terminator** (no trailing Equals).
                    if count_tree_terminator {
                        // Outer `Key`s already point at the terminator
                        // value trees, which are themselves CountTrees.
                        // No subquery is needed — grovedb returns one
                        // element per matched outer Key.
                    } else {
                        // Normal countable: descend one more layer
                        // under each matched In value's NormalTree
                        // value tree to grab the `Key([0])` CountTree
                        // child.
                        let mut subquery = Query::new();
                        subquery.insert_key(vec![COUNT_TREE_KEY]);
                        outer_query.set_subquery(subquery);
                    }
                } else {
                    // **In on a prefix + trailing Equals** that
                    // collectively reach the terminator.
                    let mut subquery = Query::new();
                    if count_tree_terminator {
                        // The terminator's serialized value is the
                        // last element pushed into
                        // `subquery_path_extension` (the trailing-
                        // Equal loop pushes `[name, value, ...,
                        // termname, termval]`). Lift `termval` out
                        // as the subquery's Key so the descent stops
                        // at the terminator's property-name subtree
                        // and the subquery resolves the CountTree
                        // value tree directly. `subquery_path_extension`
                        // is left at an odd length on purpose — it
                        // ends with the terminator's `name` segment,
                        // exactly where the subquery's `Key(termval)`
                        // picks up.
                        let termval = subquery_path_extension.pop().expect(
                            "trailing-Equal loop pushes (name, value) pairs; \
                             non-empty extension's tail must be the terminator's \
                             serialized value",
                        );
                        subquery.insert_key(termval);
                    } else {
                        // Normal countable: subquery descends to the
                        // `Key([0])` CountTree at the resolved leaf,
                        // with the full `(name, value)` pairs of the
                        // trailing Equals consumed by
                        // `set_subquery_path`.
                        subquery.insert_key(vec![COUNT_TREE_KEY]);
                    }
                    outer_query.set_subquery_path(subquery_path_extension);
                    outer_query.set_subquery(subquery);
                }

                // `SizedQuery::new(_, None, None)` is intentional —
                // PointLookupProof always returns ALL In branches.
                // The handler rejects `limit` upstream on this path
                // (see [`CountMode::accepts_limit`]'s `GroupByIn`
                // arm) because the In array is already capped at 100
                // by `WhereClause::in_values()`, and a partial-In
                // selection isn't representable in this `SizedQuery`
                // shape without rebuilding the verifier to know
                // which subset got truncated.
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
