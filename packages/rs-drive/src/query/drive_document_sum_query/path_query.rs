//! Path-query builders for the sum surface. Single source of truth for
//! the `PathQuery` shape both the prover (in `executors/*`) and the
//! verifier (in tests + bench's `display_proofs`) construct.
//!
//! Parallels [`crate::query::drive_document_count_query::path_query`] —
//! the bench's `display_proofs` function directly calls these as the
//! verifier-side rebuild, so each builder MUST produce the byte-for-byte
//! same `PathQuery` the prover used. Drift breaks every proof
//! verification.
//!
//! Two shapes exist for each builder:
//! - **Instance methods on `impl DriveDocumentSumQuery<'_>`** — called by
//!   the per-mode executors which have already resolved the covering
//!   index via the picker. These use `self.contract_id`,
//!   `self.document_type`, `self.index`, etc.
//! - **Static associated functions** — called by the bench's
//!   `display_proofs` (verifier-side rebuild) and tests. These re-pick
//!   the covering index from the document type's index map so callers
//!   don't have to thread it through.

use crate::drive::RootTree;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use crate::query::drive_document_sum_query::{is_range_operator, DriveDocumentSumQuery};
use crate::query::{WhereClause, WhereOperator};
// `serialize_value_for_key` is a `DocumentTypeV0Methods` method, NOT
// `DocumentTypeBasicMethods` (which is the trait of versionless basic
// helpers). The serializer routes through a versioned dispatcher
// (`serialize_value_for_key_v0` + friends), so it lives on the
// versioned-methods trait.
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::DataContract;
use dpp::version::PlatformVersion;
use grovedb::{PathQuery, Query, QueryItem, SizedQuery};

/// Storage convention: the count/sum tree under a non-rangeSummable
/// value tree lives at child key `[0]` (the ref bucket). Same convention
/// as count's `COUNT_TREE_KEY`.
const SUM_TREE_KEY: u8 = 0;

#[cfg(any(feature = "server", feature = "verify"))]
impl<'a> DriveDocumentSumQuery<'a> {
    /// Build the `PathQuery` for the primary-key SumTree fast path
    /// (used when `documents_summable` is set and the query has no
    /// `where` clauses).
    ///
    /// Mirrors count's `primary_key_count_tree_path_query` signature
    /// — takes the two scalar arguments (`contract_id`,
    /// `document_type_name`) that are the only fields actually used.
    pub fn primary_key_sum_path_query(
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
        query.insert_key(vec![SUM_TREE_KEY]);
        PathQuery::new(path, SizedQuery::new(query, None, None))
    }

    /// Instance-method form of [`Self::point_lookup_sum_path_query_static`]
    /// — uses `self.index` (already resolved by the picker) rather than
    /// re-picking from the document type. Mirrors count's
    /// `point_lookup_count_path_query` shape.
    pub fn point_lookup_sum_path_query(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<PathQuery, Error> {
        if self.index.properties.is_empty() {
            return Err(Error::Query(
                QuerySyntaxError::InvalidWhereClauseComponents(
                    "point_lookup_sum_path_query: index must have at least one property",
                ),
            ));
        }

        let mut base_path: Vec<Vec<u8>> = vec![
            vec![RootTree::DataContractDocuments as u8],
            self.contract_id.to_vec(),
            vec![1u8],
            self.document_type_name.as_bytes().to_vec(),
        ];

        let mut in_outer_keys: Option<Vec<Vec<u8>>> = None;
        let mut subquery_path_extension: Vec<Vec<u8>> = vec![];

        for prop in self.index.properties.iter() {
            let clause = self
                .where_clauses
                .iter()
                .find(|wc| wc.field == prop.name)
                .ok_or_else(|| {
                    Error::Query(QuerySyntaxError::InvalidWhereClauseComponents(
                        "prove sum requires the where clauses to fully cover the \
                         summable index; one or more index properties have no matching \
                         `==` or `in` clause — define a more specific summable index \
                         (with `summable: \"<prop>\"` whose properties exactly equal \
                         the clauses) or use `prove=false`",
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
                                "prove sum: at most one `in` clause is supported on the \
                                 covering summable index",
                            ),
                        ));
                    }
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
                    keys.sort();
                    in_outer_keys = Some(keys);
                }
                _ => {
                    return Err(Error::Query(
                        QuerySyntaxError::InvalidWhereClauseComponents(
                            "point_lookup_sum_path_query: index properties must use \
                             `==` or `in`",
                        ),
                    ));
                }
            }
        }

        // Sum-tree terminator optimization: every summable terminator's
        // value tree is a SumTree (continuations NonCounted-wrapped),
        // so the proof can stop at the value tree without descending
        // to the `[0]` ref bucket. Mirror of count's
        // `count_tree_terminator` gate (uses `is_countable()` on count
        // side; on the sum side, `summable.is_some()` is the right
        // discriminator).
        let sum_tree_terminator = self.index.summable.is_some();

        match in_outer_keys {
            None => {
                // Equal-only, fully covered.
                let mut query = Query::new();
                if sum_tree_terminator {
                    // Lift the last serialized value off the path: the
                    // terminator's value tree is a SumTree directly, so
                    // we ask for it as a Key under the property-name
                    // subtree.
                    let last_value = base_path.pop().expect(
                        "Equal-only loop pushes (name, value) per prop; \
                         base_path must hold the terminator's serialized value",
                    );
                    query.insert_key(last_value);
                } else {
                    query.insert_key(vec![SUM_TREE_KEY]);
                }
                Ok(PathQuery::new(
                    base_path,
                    SizedQuery::new(query, None, None),
                ))
            }
            Some(keys) => {
                // Compound shape with In at some position.
                let mut outer_query = Query::new();
                for key in keys {
                    outer_query.insert_key(key);
                }

                if subquery_path_extension.is_empty() {
                    if sum_tree_terminator {
                        // Outer Keys already point at the SumTree value
                        // trees themselves; no subquery needed.
                    } else {
                        let mut subquery = Query::new();
                        subquery.insert_key(vec![SUM_TREE_KEY]);
                        outer_query.set_subquery(subquery);
                    }
                } else {
                    let mut subquery = Query::new();
                    if sum_tree_terminator {
                        let termval = subquery_path_extension.pop().expect(
                            "trailing-Equal loop pushes (name, value) pairs; \
                             non-empty extension's tail must be the terminator's \
                             serialized value",
                        );
                        subquery.insert_key(termval);
                    } else {
                        subquery.insert_key(vec![SUM_TREE_KEY]);
                    }
                    outer_query.set_subquery_path(subquery_path_extension);
                    outer_query.set_subquery(subquery);
                }

                Ok(PathQuery::new(
                    base_path,
                    SizedQuery::new(outer_query, None, None),
                ))
            }
        }
    }

    /// Instance-method form: builds the `AggregateSumOnRange` path
    /// query against `self.index` (resolved upstream by the
    /// `find_range_summable_index_for_where_clauses` picker). The
    /// terminator's range clause is required; prefix properties must
    /// use `==`.
    pub fn aggregate_sum_path_query(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<PathQuery, Error> {
        // Bind the range clause to the index's *terminator* property so a
        // request with multiple range-like clauses (e.g. `prefix > x AND
        // terminator > y`) picks the right one. The previous predicate
        // returned the first range operator and could pick the prefix.
        let terminator_prop_name = &self
            .index
            .properties
            .last()
            .ok_or(Error::Query(
                QuerySyntaxError::InvalidWhereClauseComponents(
                    "range_summable index must have at least one property",
                ),
            ))?
            .name;
        let range_clause = self
            .where_clauses
            .iter()
            .find(|wc| wc.field == *terminator_prop_name && is_range_operator(wc.operator))
            .ok_or(Error::Query(
                QuerySyntaxError::InvalidWhereClauseComponents(
                    "aggregate_sum_path_query requires a range where-clause on the index terminator property",
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
                        "aggregate-sum proof: missing where clause for an index prefix property",
                    ),
                ))?;
            if clause.operator != WhereOperator::Equal {
                return Err(Error::Query(
                    QuerySyntaxError::InvalidWhereClauseComponents(
                        "aggregate-sum proof: prefix properties must use `==` (no `in`); use \
                         `group_by = [in_field, range_field]` (carrier-aggregate variant) for \
                         compound In-on-prefix sum queries",
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
                    "range_summable index must have at least one property",
                ),
            ))?
            .name;
        path.push(range_prop_name.as_bytes().to_vec());

        // grovedb PR 670 surface: `Query::new_aggregate_sum_on_range`.
        let query = Query::new_aggregate_sum_on_range(query_item);
        Ok(PathQuery::new(path, SizedQuery::new(query, None, None)))
    }

    /// Instance-method form: builds the combined PCPS
    /// `AggregateCountAndSumOnRange` path query against `self.index`.
    /// Requires the index to declare BOTH `rangeCountable: true` AND
    /// `rangeSummable: true`.
    pub fn aggregate_count_and_sum_path_query(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<PathQuery, Error> {
        if !self.index.range_countable {
            return Err(Error::Query(QuerySyntaxError::Unsupported(
                "aggregate_count_and_sum_path_query: index must declare BOTH \
                 `rangeCountable: true` AND `rangeSummable: true` to produce a PCPS \
                 (ProvableCountProvableSumTree) property-name tree."
                    .to_string(),
            )));
        }

        // Bind to the terminator property — see the sibling
        // `aggregate_sum_path_query` comment.
        let terminator_prop_name = &self
            .index
            .properties
            .last()
            .ok_or(Error::Query(
                QuerySyntaxError::InvalidWhereClauseComponents(
                    "PCPS index must have at least one property",
                ),
            ))?
            .name;
        let range_clause = self
            .where_clauses
            .iter()
            .find(|wc| wc.field == *terminator_prop_name && is_range_operator(wc.operator))
            .ok_or(Error::Query(
                QuerySyntaxError::InvalidWhereClauseComponents(
                    "aggregate_count_and_sum_path_query requires a range where-clause on the index terminator property",
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
                .ok_or(Error::Query(QuerySyntaxError::InvalidWhereClauseComponents(
                    "aggregate-count-and-sum proof: missing where clause for an index prefix property",
                )))?;
            if clause.operator != WhereOperator::Equal {
                return Err(Error::Query(
                    QuerySyntaxError::InvalidWhereClauseComponents(
                        "aggregate-count-and-sum proof: prefix properties must use `==` (no `in`)",
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
                    "range_countable + range_summable index must have at least one property",
                ),
            ))?
            .name;
        path.push(range_prop_name.as_bytes().to_vec());

        let query = grovedb::Query::new_aggregate_count_and_sum_on_range(query_item);
        Ok(PathQuery::new(
            path,
            grovedb::SizedQuery::new(query, None, None),
        ))
    }

    /// Convert a single range where-clause + value into the grovedb
    /// `QueryItem` used to walk children of the property-name
    /// `ProvableSumTree`. The clause's value is serialized via the
    /// document type's `serialize_value_for_key`, which produces the
    /// canonical bytes used everywhere else in the index path.
    ///
    /// Identical to count's analog — sum-agnostic operator mapping.
    /// See count's `range_clause_to_query_item` for the per-operator
    /// docs.
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
                if right_key.is_empty() {
                    return Err(Error::Query(QuerySyntaxError::InvalidStartsWithClause(
                        "startsWith prefix must have at least one byte",
                    )));
                }
                // Byte-wise carry propagation. Strip trailing 0xFFs (they
                // already cover the entire byte range) and increment the
                // first non-0xFF byte from the right. This correctly
                // handles prefixes like [0x12, 0xFF] → upper bound [0x13].
                // Only fail if every byte is 0xFF (no representable
                // exclusive upper bound).
                let mut i = right_key.len();
                while i > 0 && right_key[i - 1] == 0xFF {
                    i -= 1;
                }
                if i == 0 {
                    return Err(Error::Query(QuerySyntaxError::InvalidStartsWithClause(
                        "startsWith prefix is all 0xFF bytes; cannot form half-open upper bound",
                    )));
                }
                right_key.truncate(i);
                *right_key
                    .last_mut()
                    .expect("non-empty after truncate to non-zero length") += 1;
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

    /// Build the grovedb `PathQuery` for a per-distinct-key range-sum
    /// proof / no-proof walk against this query's `rangeSummable`
    /// index. Sum analog of count's `distinct_count_path_query` — the
    /// path-query shape is structurally identical (range on the
    /// terminator + outer `Key`s per `In` value on a prefix prop, if
    /// any). The only difference is at proof-emission time:
    /// the terminator's value tree is a `SumTree` (vs `CountTree` on
    /// the count side), so grovedb emits `KVSum` ops instead of
    /// `KVCount`. The path-query bytes the prover and verifier
    /// reconstruct are the same on both sides.
    ///
    /// `left_to_right` flips both the outer Query (when there's an
    /// `In` on prefix) and the subquery direction so the iteration
    /// walks `(in_key, terminator_key)` tuples in the requested
    /// order — descending on `left_to_right = false` walks the In
    /// dimension lex-descending too, not just the inner range.
    ///
    /// Errors:
    /// - No range where-clause / multiple range where-clauses
    /// - Multiple In clauses on prefix props
    /// - Non-Equal-non-In operator on a prefix prop
    /// - Missing prefix clause
    pub fn distinct_sum_path_query(
        &self,
        limit: Option<u16>,
        left_to_right: bool,
        platform_version: &PlatformVersion,
    ) -> Result<PathQuery, Error> {
        let range_clause = self
            .where_clauses
            .iter()
            .find(|wc| is_range_operator(wc.operator))
            .ok_or(Error::Query(
                QuerySyntaxError::InvalidWhereClauseComponents(
                    "distinct_sum_path_query requires a range where-clause",
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
                    "range_summable index must have at least one property",
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
                        "distinct_sum_path_query: missing where clause for an index \
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
                                "distinct_sum_path_query: at most one `In` clause is supported \
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
                    // Same sort + parity rationale as count's
                    // `distinct_count_path_query` — see the long
                    // docstring there. Prover and verifier share
                    // this builder so the sort happens identically
                    // on both sides; without it, descending walks
                    // and pushed-limit pagination produce gibberish.
                    keys.sort();
                    in_outer_keys = Some(keys);
                }
                _ => {
                    return Err(Error::Query(
                        QuerySyntaxError::InvalidWhereClauseComponents(
                            "distinct_sum_path_query: prefix properties must use `==` or `in`",
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
                // subtree. `subquery_path` carries any post-In
                // Equal pairs + terminator. Subquery is the range
                // item. `left_to_right` applies to both layers so
                // descending iteration walks `(in_key_desc,
                // key_desc)` tuples consistently.
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

    /// Build the grovedb `PathQuery` for a **carrier**
    /// `AggregateSumOnRange` proof — one outer Key per `In`
    /// value (or one outer QueryItem per outer-range match), each
    /// terminating in an ASOR boundary walk over the per-branch
    /// range subtree. Returns one `(in_key, i64)` pair per resolved
    /// In branch via [`grovedb::GroveDb::query_aggregate_sum_per_key`]
    /// (no-proof) and
    /// [`grovedb::GroveDb::verify_aggregate_sum_query_per_key`]
    /// (verify), once those primitives ship.
    ///
    /// Required where-clause shape (validated upstream by
    /// [`crate::query::drive_document_sum_query::drive_dispatcher::detect_sum_mode`]
    /// routing to [`DocumentSumMode::RangeAggregateCarrierProof`]):
    /// - Exactly one `In` clause on the In-property
    /// - Exactly one range clause on the *terminator* property of
    ///   a `rangeSummable: true` index whose first property is
    ///   the In-property
    /// - Any prefix properties between In and range must use
    ///   `==` (mirror of [`Self::aggregate_sum_path_query`]'s
    ///   non-In prefix rule)
    ///
    /// Path-query structure (mirror of count's analog —
    /// [`crate::query::drive_document_count_query::path_query::DriveDocumentCountQuery::carrier_aggregate_count_path_query`]):
    /// - Outer path stops one level above the In-bearing property
    ///   subtree's children (`@/doc_prefix/0x01/doctype/<In-prop>`).
    /// - Outer Query: `Key(in_value_0)`, `Key(in_value_1)`, … in
    ///   lex-asc serialized order (grovedb's multi-key walker
    ///   invariant — required for prove/verify byte-parity).
    /// - `subquery_path`: the terminator property name (and any
    ///   trailing `==` clause names between In and range, in
    ///   index order).
    /// - `subquery`: `Query::new_aggregate_sum_on_range(range_item)`.
    ///
    /// Both the executor and the verifier consume the `PathQuery`
    /// this builder produces. Grovedb PR #670 (head `e98bab5f`)
    /// landed carrier-`AggregateSumOnRange` support
    /// (`Query::validate_carrier_aggregate_sum_on_range` and
    /// `GroveDb::verify_aggregate_sum_query_per_key`), so the
    /// builder's output flows directly through `prove_query` and the
    /// verifier on both sides.
    ///
    /// Errors:
    /// - No range where-clause / multiple range where-clauses →
    ///   `InvalidWhereClauseComponents`
    /// - No In where-clause → `InvalidWhereClauseComponents`
    /// - In on a non-prefix property → `InvalidWhereClauseComponents`
    /// - Prefix property between In and range uses non-Equal →
    ///   `InvalidWhereClauseComponents`
    pub fn carrier_aggregate_sum_path_query(
        &self,
        limit: Option<u16>,
        left_to_right: bool,
        platform_version: &PlatformVersion,
    ) -> Result<PathQuery, Error> {
        // The terminator property (last in the index) carries the
        // ASOR target range. The "carrier" property — the one whose
        // clause becomes the outer Query items — is either:
        // - An `In` clause (G7 shape: one Key per In value)
        // - A range clause on a prefix prop (G8 shape: one QueryItem
        //   bounding the outer range, with `SizedQuery::limit` capping
        //   how many outer matches the carrier walks)
        //
        // The terminator's clause must be a range and is converted to
        // the inner ASOR `QueryItem`. Any properties between the
        // carrier and the terminator must use `==` and extend the
        // subquery_path.
        let terminator_prop_name = &self
            .index
            .properties
            .last()
            .ok_or(Error::Query(
                QuerySyntaxError::InvalidWhereClauseComponents(
                    "range_summable index must have at least one property",
                ),
            ))?
            .name;
        let terminator_clause = self
            .where_clauses
            .iter()
            .find(|wc| wc.field == *terminator_prop_name && is_range_operator(wc.operator))
            .ok_or(Error::Query(
                QuerySyntaxError::InvalidWhereClauseComponents(
                    "carrier_aggregate_sum_path_query requires a range where-clause on the \
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
        // Mirror of count's analog (drive_document_count_query/
        // path_query.rs's `Carrier` enum).
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
                .ok_or(Error::Query(
                    QuerySyntaxError::InvalidWhereClauseComponents(
                        "carrier-aggregate sum proof: missing where clause for an index prefix \
                     property",
                    ),
                ))?;
            match (&carrier, clause.operator) {
                (Carrier::Pending, WhereOperator::Equal) => {
                    base_path.push(prop.name.as_bytes().to_vec());
                    base_path.push(self.document_type.serialize_value_for_key(
                        prop.name.as_str(),
                        &clause.value,
                        platform_version,
                    )?);
                }
                (Carrier::Pending, WhereOperator::In) => {
                    base_path.push(prop.name.as_bytes().to_vec());
                    carrier = Carrier::In(clause.clone());
                }
                (Carrier::Pending, op) if is_range_operator(op) => {
                    base_path.push(prop.name.as_bytes().to_vec());
                    carrier = Carrier::Range(clause.clone());
                }
                (Carrier::In(_) | Carrier::Range(_), WhereOperator::Equal) => {
                    subquery_path_extension.push(prop.name.as_bytes().to_vec());
                    subquery_path_extension.push(self.document_type.serialize_value_for_key(
                        prop.name.as_str(),
                        &clause.value,
                        platform_version,
                    )?);
                }
                (Carrier::In(_) | Carrier::Range(_), _) => {
                    return Err(Error::Query(
                        QuerySyntaxError::InvalidWhereClauseComponents(
                            "carrier-aggregate sum proof: at most one carrier clause (In or \
                             range) is supported on prefix properties; subsequent prefix \
                             clauses must use `==`",
                        ),
                    ));
                }
                _ => {
                    return Err(Error::Query(
                        QuerySyntaxError::InvalidWhereClauseComponents(
                            "carrier-aggregate sum proof: prefix property operator unsupported",
                        ),
                    ));
                }
            }
        }
        subquery_path_extension.push(terminator_prop_name.as_bytes().to_vec());

        let mut outer_query = Query::new_with_direction(left_to_right);
        match carrier {
            Carrier::Pending => {
                return Err(Error::Query(
                    QuerySyntaxError::InvalidWhereClauseComponents(
                        "carrier-aggregate sum proof: an In or range clause must appear on a \
                         prefix property of the chosen index to act as the carrier dimension",
                    ),
                ));
            }
            Carrier::In(in_clause) => {
                // Build one Key per In value, sorted lex-ascending —
                // grovedb's multi-key walker invariant (same convention
                // as count's carrier and the SDK's verifier-side
                // rebuild).
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
                // carrier walks this range and emits one `(key, i64)`
                // pair per matched outer key.
                let outer_range_item =
                    self.range_clause_to_query_item(&range_clause, platform_version)?;
                outer_query.items.push(outer_range_item);
            }
        }
        outer_query.set_subquery_path(subquery_path_extension);
        outer_query.set_subquery(Query::new_aggregate_sum_on_range(inner_range_item));

        // `SizedQuery::limit` mirrors count's carrier:
        // - For In-outer carriers the |IN| array already bounds the
        //   result, so `limit` is typically `None`.
        // - For Range-outer carriers `limit` caps the outer walk and
        //   is load-bearing for proof bytes — must match prover/
        //   verifier for the merk-root recomputation.
        Ok(PathQuery::new(
            base_path,
            SizedQuery::new(outer_query, limit, None),
        ))
    }

    /// Combined PCPS (`ProvableCountProvableSumTree`) carrier variant:
    /// outer In or outer range, inner range carrying both per-bucket
    /// count AND per-bucket sum via grovedb's
    /// `AggregateCountAndSumOnRange` primitive. The terminator
    /// property's value tree must be PCPS (the index must declare
    /// BOTH `rangeCountable: true` AND `rangeSummable: true`).
    ///
    /// PCPS-only — `ProvableSumTree` / `ProvableCountTree` /
    /// `ProvableCountSumTree` (the per-axis or root-only sum
    /// variants) reject the query item at the prover. Returns one
    /// `(outer_key, u64 count, i64 sum)` triple per resolved In
    /// branch. Verified client-side via
    /// `GroveDb::verify_aggregate_count_and_sum_query_per_key`
    /// (grovedb develop (PR #670 merged; head `e98bab5f` as of this PR)).
    ///
    /// Same outer/subquery topology as
    /// [`Self::carrier_aggregate_sum_path_query`] — the only
    /// difference is the inner aggregation primitive
    /// (`Query::new_aggregate_count_and_sum_on_range` vs.
    /// `Query::new_aggregate_sum_on_range`) and the additional
    /// PCPS gate.
    pub fn carrier_aggregate_count_and_sum_path_query(
        &self,
        limit: Option<u16>,
        left_to_right: bool,
        platform_version: &PlatformVersion,
    ) -> Result<PathQuery, Error> {
        if !self.index.range_countable {
            return Err(Error::Query(QuerySyntaxError::Unsupported(
                "carrier_aggregate_count_and_sum_path_query: index must declare BOTH \
                 `rangeCountable: true` AND `rangeSummable: true` to produce a PCPS \
                 (ProvableCountProvableSumTree) property-name tree."
                    .to_string(),
            )));
        }

        let terminator_prop_name = &self
            .index
            .properties
            .last()
            .ok_or(Error::Query(
                QuerySyntaxError::InvalidWhereClauseComponents(
                    "range_countable + range_summable index must have at least one property",
                ),
            ))?
            .name;
        let terminator_clause = self
            .where_clauses
            .iter()
            .find(|wc| wc.field == *terminator_prop_name && is_range_operator(wc.operator))
            .ok_or(Error::Query(
                QuerySyntaxError::InvalidWhereClauseComponents(
                    "carrier_aggregate_count_and_sum_path_query requires a range where-clause \
                     on the terminator property of the chosen index",
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

        // Same Carrier state-machine as the sum-only variant.
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
                .ok_or(Error::Query(
                    QuerySyntaxError::InvalidWhereClauseComponents(
                        "carrier-aggregate count-and-sum proof: missing where clause for an index \
                     prefix property",
                    ),
                ))?;
            match (&carrier, clause.operator) {
                (Carrier::Pending, WhereOperator::Equal) => {
                    base_path.push(prop.name.as_bytes().to_vec());
                    base_path.push(self.document_type.serialize_value_for_key(
                        prop.name.as_str(),
                        &clause.value,
                        platform_version,
                    )?);
                }
                (Carrier::Pending, WhereOperator::In) => {
                    base_path.push(prop.name.as_bytes().to_vec());
                    carrier = Carrier::In(clause.clone());
                }
                (Carrier::Pending, op) if is_range_operator(op) => {
                    base_path.push(prop.name.as_bytes().to_vec());
                    carrier = Carrier::Range(clause.clone());
                }
                (Carrier::In(_) | Carrier::Range(_), WhereOperator::Equal) => {
                    subquery_path_extension.push(prop.name.as_bytes().to_vec());
                    subquery_path_extension.push(self.document_type.serialize_value_for_key(
                        prop.name.as_str(),
                        &clause.value,
                        platform_version,
                    )?);
                }
                (Carrier::In(_) | Carrier::Range(_), _) => {
                    return Err(Error::Query(
                        QuerySyntaxError::InvalidWhereClauseComponents(
                            "carrier-aggregate count-and-sum proof: at most one carrier clause \
                             (In or range) is supported on prefix properties; subsequent prefix \
                             clauses must use `==`",
                        ),
                    ));
                }
                _ => {
                    return Err(Error::Query(
                        QuerySyntaxError::InvalidWhereClauseComponents(
                            "carrier-aggregate count-and-sum proof: prefix property operator \
                             unsupported",
                        ),
                    ));
                }
            }
        }
        subquery_path_extension.push(terminator_prop_name.as_bytes().to_vec());

        let mut outer_query = Query::new_with_direction(left_to_right);
        match carrier {
            Carrier::Pending => {
                return Err(Error::Query(
                    QuerySyntaxError::InvalidWhereClauseComponents(
                        "carrier-aggregate count-and-sum proof: an In or range clause must \
                         appear on a prefix property of the chosen index to act as the carrier \
                         dimension",
                    ),
                ));
            }
            Carrier::In(in_clause) => {
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
                let outer_range_item =
                    self.range_clause_to_query_item(&range_clause, platform_version)?;
                outer_query.items.push(outer_range_item);
            }
        }
        outer_query.set_subquery_path(subquery_path_extension);
        outer_query.set_subquery(grovedb::Query::new_aggregate_count_and_sum_on_range(
            inner_range_item,
        ));

        Ok(PathQuery::new(
            base_path,
            SizedQuery::new(outer_query, limit, None),
        ))
    }
}

// ─── Static / free-function wrappers for the bench + verifier-side
// rebuild. These re-pick the covering index from the document type
// (vs. the instance methods above which use the already-resolved
// `self.index`). ────────────────────────────────────────────────────

#[cfg(any(feature = "server", feature = "verify"))]
impl<'a> DriveDocumentSumQuery<'a> {
    /// Static wrapper for the bench / verifier-side rebuild. Calls
    /// the instance method via a temporary `DriveDocumentSumQuery`
    /// built from the picked covering index.
    pub fn point_lookup_sum_path_query_static(
        contract: &DataContract,
        document_type: DocumentTypeRef,
        sum_property: &str,
        where_clauses: &[WhereClause],
        resolved_time_range_fields: &[String],
        platform_version: &PlatformVersion,
    ) -> Result<PathQuery, Error> {
        use crate::query::drive_document_sum_query::index_picker::find_summable_index_for_where_clauses;
        use dpp::data_contract::accessors::v0::DataContractV0Getters;
        use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;

        let index = find_summable_index_for_where_clauses(
            document_type.indexes(),
            where_clauses,
            sum_property,
            resolved_time_range_fields,
        )
        .ok_or_else(|| {
            Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(
                "no `summable: \"<prop>\"` index exactly matches the where-clause fields. \
                 Define a more specific summable index (with `summable: \"<prop>\"` whose \
                 properties exactly equal the clauses) or use `prove=false`."
                    .to_string(),
            ))
        })?;
        let q = DriveDocumentSumQuery {
            document_type,
            contract_id: contract.id().to_buffer(),
            document_type_name: document_type.name().clone(),
            index,
            where_clauses: where_clauses.to_vec(),
            sum_property: sum_property.to_string(),
        };
        q.point_lookup_sum_path_query(platform_version)
    }

    /// Static wrapper for the bench / verifier-side rebuild — picks the
    /// covering range-summable index and delegates to the instance
    /// method.
    pub fn aggregate_sum_path_query_static(
        contract: &DataContract,
        document_type: DocumentTypeRef,
        sum_property: &str,
        where_clauses: &[WhereClause],
        resolved_time_range_fields: &[String],
        platform_version: &PlatformVersion,
    ) -> Result<PathQuery, Error> {
        use crate::query::drive_document_sum_query::index_picker::find_range_summable_index_for_where_clauses;
        use dpp::data_contract::accessors::v0::DataContractV0Getters;
        use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;

        let index = find_range_summable_index_for_where_clauses(
            document_type.indexes(),
            where_clauses,
            sum_property,
            resolved_time_range_fields,
        )
        .ok_or_else(|| {
            Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(
                "no `rangeSummable: true` index covers the where-clause shape (Equal/In \
                 prefix exactly + range on the index's last property). Define one or use \
                 `prove=false`."
                    .to_string(),
            ))
        })?;
        let q = DriveDocumentSumQuery {
            document_type,
            contract_id: contract.id().to_buffer(),
            document_type_name: document_type.name().clone(),
            index,
            where_clauses: where_clauses.to_vec(),
            sum_property: sum_property.to_string(),
        };
        q.aggregate_sum_path_query(platform_version)
    }

    /// Static wrapper for the bench / verifier-side rebuild — picks
    /// the covering range-summable index and delegates to the carrier
    /// instance method. Mirror of count's analog
    /// [`crate::query::drive_document_count_query::path_query::DriveDocumentCountQuery::carrier_aggregate_count_path_query`]'s
    /// implicit static surface via the executor.
    /// Used by the SDK verifier-side rebuild via
    /// `GroveDb::verify_aggregate_sum_query_per_key` (grovedb PR #670
    /// head `e98bab5f`).
    #[allow(clippy::too_many_arguments)]
    pub fn carrier_aggregate_sum_path_query_static(
        contract: &DataContract,
        document_type: DocumentTypeRef,
        sum_property: &str,
        where_clauses: &[WhereClause],
        resolved_time_range_fields: &[String],
        limit: Option<u16>,
        left_to_right: bool,
        platform_version: &PlatformVersion,
    ) -> Result<PathQuery, Error> {
        use crate::query::drive_document_sum_query::index_picker::find_range_summable_index_for_where_clauses;
        use dpp::data_contract::accessors::v0::DataContractV0Getters;
        use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;

        let index = find_range_summable_index_for_where_clauses(
            document_type.indexes(),
            where_clauses,
            sum_property,
            resolved_time_range_fields,
        )
        .ok_or_else(|| {
            Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(
                "no `rangeSummable: true` index covers the where-clause shape for the \
                 carrier-aggregate sum carrier (Equal/In prefix + In-or-range carrier + \
                 range on the index's last property). Define one or use `prove=false`."
                    .to_string(),
            ))
        })?;
        let q = DriveDocumentSumQuery {
            document_type,
            contract_id: contract.id().to_buffer(),
            document_type_name: document_type.name().clone(),
            index,
            where_clauses: where_clauses.to_vec(),
            sum_property: sum_property.to_string(),
        };
        q.carrier_aggregate_sum_path_query(limit, left_to_right, platform_version)
    }
}

// ── Carrier-shape unit tests ───────────────────────────────────────
//
// The carrier builder is pure Rust data construction — no grovedb
// interaction — so it can be exercised today regardless of the upstream
// grovedb prover gating. Tests assert the structural invariants the
// prover/verifier will require once the sister PR lands:
// - outer path stops at the In-bearing property-name subtree;
// - outer Query has Key items in lex-asc serialized order;
// - default_subquery_branch.subquery is a single
//   `AggregateSumOnRange(inner)`;
// - subquery_path is the (post-In Equals + terminator name) chain.
//
// These tests pin the carrier path-query shape so a future refactor of
// the builder body can't silently drift from what the verifier will
// rebuild on its side.
#[cfg(test)]
mod carrier_path_query_tests {
    use super::*;
    use crate::query::WhereOperator;
    use assert_matches::assert_matches;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
    use dpp::data_contract::DataContract;
    use dpp::tests::json_document::json_document_to_contract;
    use grovedb::QueryItem;

    fn load_tip_jar_contract(platform_version: &PlatformVersion) -> DataContract {
        // The tip-jar contract has a rangeSummable index on
        // `(recipient, sentAt)` (`byRecipientTime`) with
        // `summable: "amount"` — exactly the shape the carrier targets:
        // outer In on `recipient`, inner range on `sentAt`.
        json_document_to_contract(
            "tests/supporting_files/contract/tip-jar/tip-jar-contract.json",
            false,
            platform_version,
        )
        .expect("tip-jar contract fixture loads")
    }

    /// Helper — given a contract and a `(doctype, index)` name pair,
    /// resolve the [`DocumentTypeRef`] for the doctype.
    ///
    /// The matching `Index` is fetched inside each test via
    /// `doc_type.indexes().get(index_name)` rather than returned
    /// alongside `doc_type` here, because the index reference is
    /// bound to the doc_type's lifetime (not the contract's), so
    /// returning both from a helper would create a self-referential
    /// tuple. The two-step pattern (`pick_doc_type` here, then
    /// `.indexes().get(...)` at the call site) is the same pattern
    /// count's tests use.
    fn pick_doc_type<'a>(
        contract: &'a DataContract,
        doc_type_name: &str,
    ) -> dpp::data_contract::document_type::DocumentTypeRef<'a> {
        contract
            .document_type_for_name(doc_type_name)
            .expect("document type exists in tip-jar fixture")
    }

    /// Two recipient byte-array values for In-on-carrier tests. We
    /// pick values in non-lex order so the builder's sort step is
    /// observable in the resulting Key item order.
    fn recipient_a() -> Vec<u8> {
        // Bytes starting with 0x80 — lex-greater.
        let mut v = vec![0x80u8; 32];
        v[31] = 0x01;
        v
    }
    fn recipient_b() -> Vec<u8> {
        // Bytes starting with 0x10 — lex-less.
        let mut v = vec![0x10u8; 32];
        v[31] = 0x02;
        v
    }

    /// G7 — In on carrier + range on terminator. Asserts outer Query
    /// has one `Key` per In value (lex-sorted), subquery is
    /// `AggregateSumOnRange(inner_range)`, and subquery_path is just
    /// the terminator's property-name segment.
    #[test]
    fn carrier_aggregate_sum_in_on_carrier_range_on_terminator() {
        let platform_version = PlatformVersion::latest();
        let contract = load_tip_jar_contract(platform_version);
        let doc_type = pick_doc_type(&contract, "tip");
        let index = doc_type
            .indexes()
            .get("byRecipientTime")
            .expect("byRecipientTime index exists on tip doc type");

        // `byRecipientTime` is `[recipient, sentAt]` with
        // `summable: "amount"` + `rangeSummable: true`. Provide the
        // In values out of lex order so the builder's lex-sort is
        // observable.
        let in_values = vec![
            dpp::platform_value::Value::Bytes(recipient_a()),
            dpp::platform_value::Value::Bytes(recipient_b()),
        ];
        let where_clauses = vec![
            WhereClause {
                field: "recipient".to_string(),
                operator: WhereOperator::In,
                value: dpp::platform_value::Value::Array(in_values.clone()),
            },
            WhereClause {
                field: "sentAt".to_string(),
                operator: WhereOperator::GreaterThan,
                value: dpp::platform_value::Value::U64(0),
            },
        ];
        let q = DriveDocumentSumQuery {
            document_type: doc_type,
            contract_id: contract.id().to_buffer(),
            document_type_name: doc_type.name().clone(),
            index,
            where_clauses,
            sum_property: "amount".to_string(),
        };

        let pq = q
            .carrier_aggregate_sum_path_query(None, true, platform_version)
            .expect("carrier-aggregate sum path query builds");

        // base_path = [contract-docs-root, contract_id, 0x01,
        // doctype_name, "recipient"]. The outer Keys live under the
        // "recipient" property-name subtree.
        assert!(
            pq.path.len() >= 5,
            "expected base_path to extend through the In-bearing prop's name subtree"
        );
        assert_eq!(
            pq.path.last().expect("base_path non-empty"),
            b"recipient",
            "outer path must stop at the In-bearing prop's property-name subtree"
        );

        // Outer Query: one Key per In value, lex-sorted (the
        // builder's `.sort()` step turns the unsorted user input into
        // the prover/verifier-agreement lex-asc order).
        let outer_items = &pq.query.query.items;
        assert_eq!(outer_items.len(), 2, "one outer Key per In value");
        for item in outer_items {
            assert_matches!(item, QueryItem::Key(_));
        }
        if let (QueryItem::Key(a), QueryItem::Key(b)) = (&outer_items[0], &outer_items[1]) {
            assert!(a < b, "outer Keys must be sorted lex-ascending");
        }

        // Subquery_path = ["sentAt"] (just the terminator's name).
        let sub_path = pq
            .query
            .query
            .default_subquery_branch
            .subquery_path
            .as_ref()
            .expect("subquery_path set");
        assert_eq!(sub_path, &vec![b"sentAt".to_vec()]);

        // Subquery is `AggregateSumOnRange(inner_range)`.
        let subquery = pq
            .query
            .query
            .default_subquery_branch
            .subquery
            .as_ref()
            .expect("subquery set");
        assert_eq!(subquery.items.len(), 1);
        assert_matches!(subquery.items[0], QueryItem::AggregateSumOnRange(_));
    }

    /// G7 — same as above but `limit = Some(N)` flows into
    /// `SizedQuery::limit` so the prover/verifier sides agree on the
    /// outer-walk cap byte-for-byte.
    #[test]
    fn carrier_aggregate_sum_limit_flows_into_sized_query() {
        let platform_version = PlatformVersion::latest();
        let contract = load_tip_jar_contract(platform_version);
        let doc_type = pick_doc_type(&contract, "tip");
        let index = doc_type
            .indexes()
            .get("byRecipientTime")
            .expect("byRecipientTime index exists on tip doc type");

        let where_clauses = vec![
            WhereClause {
                field: "recipient".to_string(),
                operator: WhereOperator::In,
                value: dpp::platform_value::Value::Array(vec![
                    dpp::platform_value::Value::Bytes(recipient_a()),
                    dpp::platform_value::Value::Bytes(recipient_b()),
                ]),
            },
            WhereClause {
                field: "sentAt".to_string(),
                operator: WhereOperator::GreaterThan,
                value: dpp::platform_value::Value::U64(0),
            },
        ];
        let q = DriveDocumentSumQuery {
            document_type: doc_type,
            contract_id: contract.id().to_buffer(),
            document_type_name: doc_type.name().clone(),
            index,
            where_clauses,
            sum_property: "amount".to_string(),
        };

        let pq = q
            .carrier_aggregate_sum_path_query(Some(7), true, platform_version)
            .expect("carrier-aggregate sum path query builds with limit");
        assert_eq!(pq.query.limit, Some(7), "outer SizedQuery::limit threads");
    }

    /// Missing terminator range → `InvalidWhereClauseComponents`.
    #[test]
    fn carrier_aggregate_sum_rejects_missing_terminator_range() {
        let platform_version = PlatformVersion::latest();
        let contract = load_tip_jar_contract(platform_version);
        let doc_type = pick_doc_type(&contract, "tip");
        let index = doc_type
            .indexes()
            .get("byRecipientTime")
            .expect("byRecipientTime index exists on tip doc type");

        let where_clauses = vec![WhereClause {
            field: "recipient".to_string(),
            operator: WhereOperator::In,
            value: dpp::platform_value::Value::Array(vec![dpp::platform_value::Value::Bytes(
                recipient_a(),
            )]),
        }];
        let q = DriveDocumentSumQuery {
            document_type: doc_type,
            contract_id: contract.id().to_buffer(),
            document_type_name: doc_type.name().clone(),
            index,
            where_clauses,
            sum_property: "amount".to_string(),
        };

        let err = q
            .carrier_aggregate_sum_path_query(None, true, platform_version)
            .expect_err("missing range clause must be rejected");
        let msg = format!("{err:?}");
        assert!(
            msg.contains("requires a range where-clause"),
            "unexpected error: {msg}"
        );
    }

    /// Missing carrier (no In or outer range on a prefix prop) →
    /// `InvalidWhereClauseComponents`.
    #[test]
    fn carrier_aggregate_sum_rejects_missing_carrier() {
        let platform_version = PlatformVersion::latest();
        let contract = load_tip_jar_contract(platform_version);
        let doc_type = pick_doc_type(&contract, "tip");
        let index = doc_type
            .indexes()
            .get("byRecipientTime")
            .expect("byRecipientTime index exists on tip doc type");

        // Equal on prefix + range on terminator — *no* carrier. This
        // is the `aggregate_sum_path_query` shape, not the carrier
        // shape; the carrier builder must reject because the carrier
        // state stays `Pending` through the prefix loop.
        let where_clauses = vec![
            WhereClause {
                field: "recipient".to_string(),
                operator: WhereOperator::Equal,
                value: dpp::platform_value::Value::Bytes(recipient_a()),
            },
            WhereClause {
                field: "sentAt".to_string(),
                operator: WhereOperator::GreaterThan,
                value: dpp::platform_value::Value::U64(0),
            },
        ];
        let q = DriveDocumentSumQuery {
            document_type: doc_type,
            contract_id: contract.id().to_buffer(),
            document_type_name: doc_type.name().clone(),
            index,
            where_clauses,
            sum_property: "amount".to_string(),
        };

        let err = q
            .carrier_aggregate_sum_path_query(None, true, platform_version)
            .expect_err("Equal-only prefix must be rejected by carrier builder");
        let msg = format!("{err:?}");
        assert!(msg.contains("carrier dimension"), "unexpected error: {msg}");
    }
}
