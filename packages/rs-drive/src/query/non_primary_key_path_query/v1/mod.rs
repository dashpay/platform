//! v1 of the non-primary-key path-query lowering (protocol version 14):
//! multiple `In` clauses on consecutive index properties lower to
//! multi-level key-set path queries. Single-`In` shapes route through
//! the v0 lowering unchanged. Conservative v1 restrictions (cursor
//! rejection, the cross-product cap, index conformity) live here, as
//! does the order-by-aware left-over recursion the multi-`In` path
//! uses in place of the v0 helper.

use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use crate::query::conditions::WhereClause;
use crate::query::ordering::OrderClause;
use crate::query::{defaults, DriveDocumentQuery};
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::data_contract::document_type::{Index, IndexProperty};
use dpp::document::Document;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use grovedb::{PathQuery, Query, QueryItem, SizedQuery};
use indexmap::IndexMap;

impl<'a> DriveDocumentQuery<'a> {
    #[cfg(any(feature = "server", feature = "verify"))]
    /// v1 counterpart of [`Self::recursive_insert_on_query`] for the
    /// multi-`In` lowering: no cursor support (cursors are rejected by the
    /// shape preflight), and each left-over level takes its direction from
    /// `order_by` — falling back to the index property's — instead of
    /// always using the index property's like the v0 helper does.
    pub(in crate::query) fn recursive_insert_on_query_ordered(
        query: &mut Query,
        left_over_index_properties: &[&IndexProperty],
        unique: bool,
        default_left_to_right: bool,
        order_by: &IndexMap<String, OrderClause>,
    ) {
        match left_over_index_properties.split_first() {
            None => match unique {
                true => {
                    query.set_subquery_key(vec![0]);

                    // In the case things are NULL we allow to have multiple values
                    let inner_query = Self::inner_query_from_starts_at_for_id(
                        None, true, //for ids we always go left to right
                    );
                    query.add_conditional_subquery(
                        QueryItem::Key(b"".to_vec()),
                        Some(vec![vec![0]]),
                        Some(inner_query),
                    );
                }
                false => {
                    query.set_subquery_key(vec![0]);
                    // we just get all by document id order ascending
                    let full_query =
                        Self::inner_query_from_starts_at_for_id(None, default_left_to_right);
                    query.set_subquery(full_query);

                    let inner_query =
                        Self::inner_query_from_starts_at_for_id(None, default_left_to_right);
                    query.add_conditional_subquery(
                        QueryItem::Key(b"".to_vec()),
                        Some(vec![vec![0]]),
                        Some(inner_query),
                    );
                }
            },
            Some((first, left_over)) => {
                let left_to_right = order_by
                    .get(first.name.as_str())
                    .map(|order_clause| order_clause.ascending)
                    .unwrap_or(first.ascending);
                let mut inner_query = Query::new_with_direction(left_to_right);
                inner_query.insert_all();
                Self::recursive_insert_on_query_ordered(
                    &mut inner_query,
                    left_over,
                    unique,
                    left_to_right,
                    order_by,
                );
                query.set_subquery(inner_query);
                query.set_subquery_key(first.name.as_bytes().to_vec());
            }
        }
    }

    #[cfg(any(feature = "server", feature = "verify"))]
    /// v1 of the non-primary-key path query lowering (protocol version 14):
    /// accepts multiple `In` clauses. Single-`In` shapes lower exactly as v0.
    pub(in crate::query) fn get_non_primary_key_path_query_v1(
        &self,
        document_type_path: Vec<Vec<u8>>,
        starts_at_document: Option<(Document, bool)>,
        platform_version: &PlatformVersion,
    ) -> Result<PathQuery, Error> {
        if self.internal_clauses.in_clauses.len() > 1 {
            self.get_non_primary_key_multiple_in_path_query(
                document_type_path,
                starts_at_document,
                platform_version,
            )
        } else {
            self.get_non_primary_key_path_query_v0(
                document_type_path,
                starts_at_document,
                platform_version,
            )
        }
    }

    #[cfg(any(feature = "server", feature = "verify"))]
    /// Lowers a query with multiple `In` clauses into a path query whose
    /// levels carry one key set per `In` clause, in index property order,
    /// followed by an optional range level and the usual left-over /
    /// terminal levels. Only reachable through the v1 lowering.
    pub(in crate::query) fn get_non_primary_key_multiple_in_path_query(
        &self,
        document_type_path: Vec<Vec<u8>>,
        starts_at_document: Option<(Document, bool)>,
        platform_version: &PlatformVersion,
    ) -> Result<PathQuery, Error> {
        // Conservative v1: the cross-branch cursor machinery is not wired
        // for key-set branching at more than one level, so reject cursors
        // instead of shipping silently wrong pagination.
        if starts_at_document.is_some() || self.start_at.is_some() {
            return Err(Error::Query(QuerySyntaxError::Unsupported(
                "startAt/startAfter is not supported with multiple in clauses".to_string(),
            )));
        }

        let (index, ordered_in_clauses, equality_len) =
            self.find_best_index_for_multiple_in_clauses()?;

        // Bound the branch enumeration: the product of the in list sizes is
        // the number of index subtrees the query opens.
        let mut cross_product: usize = 1;
        for in_clause in &ordered_in_clauses {
            let in_values = in_clause.in_values().into_data_with_error()??;
            cross_product = cross_product.saturating_mul(in_values.len());
        }
        if cross_product > defaults::MAX_IN_CROSS_PRODUCT_SIZE {
            return Err(Error::Query(QuerySyntaxError::InvalidInClause(format!(
                "the product of in clause list sizes must be at most {}, got {}",
                defaults::MAX_IN_CROSS_PRODUCT_SIZE,
                cross_product
            ))));
        }

        let left_over_index_properties = index
            .properties
            .iter()
            .filter(|field| {
                !(self
                    .internal_clauses
                    .equal_clauses
                    .contains_key(field.name.as_str())
                    || ordered_in_clauses
                        .iter()
                        .any(|in_clause| in_clause.field == field.name)
                    || self
                        .internal_clauses
                        .range_clause
                        .as_ref()
                        .is_some_and(|range_clause| range_clause.field == field.name))
            })
            .collect::<Vec<&IndexProperty>>();

        // Every level that fans out (each in clause, and the range clause)
        // needs an explicit ordering, like any range-class clause.
        let direction_for = |field: &str| -> Result<bool, Error> {
            let order_clause: &OrderClause = self.order_by.get(field).ok_or(Error::Query(
                QuerySyntaxError::MissingOrderByForRange(
                    "query must have an orderBy field for each range element",
                ),
            ))?;
            Ok(order_clause.ascending)
        };

        // Build the query bottom-up. The deepest clause level is the range
        // clause when present, otherwise the last in clause; the left-over
        // index properties and the terminal document level hang under it.
        let (mut child_field, mut child_query, deepest_left_to_right) =
            match &self.internal_clauses.range_clause {
                Some(range_clause) => {
                    let left_to_right = direction_for(range_clause.field.as_str())?;
                    let query = range_clause.to_path_query(
                        self.document_type,
                        &None,
                        left_to_right,
                        platform_version,
                    )?;
                    (range_clause.field.clone(), query, left_to_right)
                }
                None => {
                    let deepest_in_clause =
                        *ordered_in_clauses.last().expect("more than one in clause");
                    let left_to_right = direction_for(deepest_in_clause.field.as_str())?;
                    let query = deepest_in_clause.to_path_query(
                        self.document_type,
                        &None,
                        left_to_right,
                        platform_version,
                    )?;
                    (deepest_in_clause.field.clone(), query, left_to_right)
                }
            };
        Self::recursive_insert_on_query_ordered(
            &mut child_query,
            left_over_index_properties.as_slice(),
            index.unique,
            deepest_left_to_right,
            &self.order_by,
        );

        // Wrap the remaining in levels around it, deepest first. When a
        // range clause is present every in clause wraps; otherwise the
        // deepest in clause is already the leaf built above.
        let wrapped_in_clauses = if self.internal_clauses.range_clause.is_some() {
            ordered_in_clauses.as_slice()
        } else {
            &ordered_in_clauses[..ordered_in_clauses.len() - 1]
        };
        for in_clause in wrapped_in_clauses.iter().rev() {
            let left_to_right = direction_for(in_clause.field.as_str())?;
            let mut query = in_clause.to_path_query(
                self.document_type,
                &None,
                left_to_right,
                platform_version,
            )?;
            query.set_subquery_key(child_field.as_bytes().to_vec());
            query.set_subquery(child_query);
            child_field = in_clause.field.clone();
            child_query = query;
        }

        let intermediate_values = index.properties[..equality_len]
            .iter()
            .map(|field| {
                let where_clause = self
                    .internal_clauses
                    .equal_clauses
                    .get(field.name.as_str())
                    .expect("equality prefix was validated during index selection");
                self.document_type.serialize_value_for_key(
                    field.name.as_str(),
                    &where_clause.value,
                    platform_version,
                )
            })
            .collect::<Result<Vec<Vec<u8>>, ProtocolError>>()
            .map_err(Error::from)?;

        let mut path = document_type_path;
        for (intermediate_index, intermediate_value) in index.properties[..equality_len]
            .iter()
            .zip(intermediate_values.iter())
        {
            path.push(intermediate_index.name.as_bytes().to_vec());
            path.push(intermediate_value.as_slice().to_vec());
        }
        path.push(child_field.as_bytes().to_vec());

        Ok(PathQuery::new(
            path,
            SizedQuery::new(child_query, self.limit, self.offset),
        ))
    }

    #[cfg(any(feature = "server", feature = "verify"))]
    /// Finds the best index for a query with more than one `In` clause, and
    /// returns it together with the query's `In` clauses ordered by their
    /// position in that index and the length of the equality prefix.
    ///
    /// A candidate index conforms when its properties decompose, left to
    /// right, into: the equality-clause fields (exactly covering positions
    /// `0..E`), then every `In` field on consecutive positions `E..E+K`,
    /// then — when a range clause is present — the range field. The usual
    /// tail rule from [`Index::matches`] still applies with the deepest `In`
    /// field playing the role of the in field, as do the order-by continuity
    /// rules and [`defaults::MAX_INDEX_DIFFERENCE`].
    pub(in crate::query) fn find_best_index_for_multiple_in_clauses(
        &self,
    ) -> Result<(&Index, Vec<&WhereClause>, usize), Error> {
        let equal_clauses = &self.internal_clauses.equal_clauses;
        let in_clauses = &self.internal_clauses.in_clauses;
        let range_field = self
            .internal_clauses
            .range_clause
            .as_ref()
            .map(|range_clause| range_clause.field.as_str());

        let mut fields = equal_clauses
            .keys()
            .map(|s| s.as_str())
            .collect::<Vec<&str>>();
        if let Some(range_field) = range_field {
            fields.push(range_field);
        }
        fields.extend(in_clauses.iter().map(|in_clause| in_clause.field.as_str()));

        let order_by_keys: Vec<&str> = self
            .order_by
            .keys()
            .map(|key: &String| {
                let str = key.as_str();
                if !fields.contains(&str) {
                    fields.push(str);
                }
                str
            })
            .collect();

        let equality_len = equal_clauses.len();
        let mut best: Option<(&Index, Vec<&WhereClause>, u16)> = None;
        for index in self.document_type.indexes().values() {
            let mut positioned: Vec<(usize, &WhereClause)> = Vec::with_capacity(in_clauses.len());
            for in_clause in in_clauses {
                match index
                    .properties
                    .iter()
                    .position(|property| property.name == in_clause.field)
                {
                    Some(position) => positioned.push((position, in_clause)),
                    None => break,
                }
            }
            if positioned.len() != in_clauses.len() {
                continue;
            }
            positioned.sort_by_key(|(position, _)| *position);

            // The equality clauses must exactly cover the index prefix
            if index.properties.len() < equality_len
                || !index.properties[..equality_len]
                    .iter()
                    .all(|property| equal_clauses.contains_key(property.name.as_str()))
            {
                continue;
            }
            // The in clauses must sit on consecutive properties right after it
            let consecutive_after_prefix = positioned
                .iter()
                .enumerate()
                .all(|(i, (position, _))| *position == equality_len + i);
            if !consecutive_after_prefix {
                continue;
            }
            // A range clause must sit immediately after the in block
            if let Some(range_field) = range_field {
                match index.properties.get(equality_len + positioned.len()) {
                    Some(property) if property.name == range_field => {}
                    _ => continue,
                }
            }

            let deepest_in_field = positioned
                .last()
                .expect("more than one in clause")
                .1
                .field
                .as_str();
            let Some(difference) = index.matches(&fields, Some(deepest_in_field), &order_by_keys)
            else {
                continue;
            };
            let ordered = positioned
                .into_iter()
                .map(|(_, in_clause)| in_clause)
                .collect::<Vec<&WhereClause>>();
            if difference == 0 {
                return Ok((index, ordered, equality_len));
            }
            match &best {
                Some((_, _, best_difference)) if *best_difference <= difference => {}
                _ => best = Some((index, ordered, difference)),
            }
        }

        let (index, ordered, difference) = best.ok_or(Error::Query(
            QuerySyntaxError::WhereClauseOnNonIndexedProperty(format!(
                "query with multiple in clauses must be for valid indexes with the in clauses on \
                 consecutive index properties after the equality clauses, valid indexes are: {:?}",
                self.document_type.indexes()
            )),
        ))?;
        if difference > defaults::MAX_INDEX_DIFFERENCE {
            return Err(Error::Query(QuerySyntaxError::QueryTooFarFromIndex(
                "query must better match an existing index",
            )));
        }
        Ok((index, ordered, equality_len))
    }
}
