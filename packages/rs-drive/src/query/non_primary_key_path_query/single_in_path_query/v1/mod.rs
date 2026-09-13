//! v1 of the single-IN lowering: an IN/range pair applies inner cursor
//! bounds only on the cursor's outer branch. Sibling branches keep the
//! original inner predicate, including its direction and bounds.

use crate::error::drive::DriveError;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use crate::query::conditions::WhereClause;
use crate::query::ordering::OrderClause;
use crate::query::{DriveDocumentQuery, StartAtDocument};
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::data_contract::document_type::IndexProperty;
use dpp::document::document_methods::DocumentMethodsV0;
use dpp::document::{Document, DocumentV0Getters};
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use grovedb::{PathQuery, Query, QueryItem, SizedQuery};

impl<'a> DriveDocumentQuery<'a> {
    #[cfg(any(feature = "server", feature = "verify"))]
    /// Lowers at-most-one-IN queries with branch-local compound cursors.
    pub(in crate::query) fn get_non_primary_key_single_in_path_query_v1(
        &self,
        document_type_path: Vec<Vec<u8>>,
        starts_at_document: Option<(Document, bool)>,
        platform_version: &PlatformVersion,
    ) -> Result<PathQuery, Error> {
        if self.internal_clauses.in_clauses.len() > 1 {
            return Err(Error::Query(QuerySyntaxError::MultipleInClauses(
                "There should only be one in clause",
            )));
        }
        let index = self.find_best_index(platform_version)?;
        let ordered_clauses: Vec<&WhereClause> = index
            .properties
            .iter()
            .filter_map(|field| self.internal_clauses.equal_clauses.get(field.name.as_str()))
            .collect();
        let (last_clause, last_clause_is_range, subquery_clause) =
            match self.internal_clauses.in_clauses.first() {
                None => match &self.internal_clauses.range_clause {
                    None => (ordered_clauses.last().copied(), false, None),
                    Some(where_clause) => (Some(where_clause), true, None),
                },
                Some(in_clause) => match &self.internal_clauses.range_clause {
                    None => (Some(in_clause), true, None),
                    Some(range_clause) => {
                        // Same clause ordering rule as the pre-v14
                        // construction — the outer path query must operate on
                        // the field that appears earlier in the chosen index.
                        // See issue #2409.
                        let position_of = |field: &str| -> Option<usize> {
                            index
                                .properties
                                .iter()
                                .position(|p| p.name.as_str() == field)
                        };
                        let in_pos = position_of(in_clause.field.as_str());
                        let range_pos = position_of(range_clause.field.as_str());
                        match (in_pos, range_pos) {
                            (Some(i), Some(r)) if i > r => {
                                (Some(range_clause), true, Some(in_clause))
                            }
                            _ => (Some(in_clause), true, Some(range_clause)),
                        }
                    }
                },
            };

        // We need to get the terminal indexes unused by clauses.
        let left_over_index_properties = index
            .properties
            .iter()
            .filter(|field| {
                !(self
                    .internal_clauses
                    .equal_clauses
                    .contains_key(field.name.as_str())
                    || (last_clause.is_some() && last_clause.unwrap().field == field.name)
                    || (subquery_clause.is_some() && subquery_clause.unwrap().field == field.name))
            })
            .collect::<Vec<&IndexProperty>>();

        let intermediate_values = index
            .properties
            .iter()
            .filter_map(|field| {
                match self.internal_clauses.equal_clauses.get(field.name.as_str()) {
                    None => None,
                    Some(where_clause) => {
                        if !last_clause_is_range
                            && last_clause.is_some()
                            && last_clause.unwrap().field == field.name
                        {
                            //there is no need to give an intermediate value as the last clause is an equality
                            None
                        } else {
                            Some(self.document_type.serialize_value_for_key(
                                field.name.as_str(),
                                &where_clause.value,
                                platform_version,
                            ))
                        }
                    }
                }
            })
            .collect::<Result<Vec<Vec<u8>>, ProtocolError>>()
            .map_err(Error::from)?;

        // Defensive re-check of the protocol-version-14 matcher's
        // contiguity guarantee (`Index::matches_contiguous`): the
        // positional split below pairs `intermediate_values` with the
        // index's leading properties in order, so a gap in the equality
        // cover — or a range/`in` clause not sitting right after it —
        // would misalign every level below the hole.
        if !query_covers_index_prefix_contiguously(
            &index.properties,
            &|field| self.internal_clauses.equal_clauses.contains_key(field),
            last_clause,
            subquery_clause,
            intermediate_values.len(),
        ) {
            return Err(Error::Query(
                QuerySyntaxError::WhereClauseOnNonIndexedProperty(format!(
                    "query fields do not contiguously cover the index's properties from the \
                     first: equality clauses must cover the index prefix and any range or in \
                     clause must immediately follow it; index: {:?}",
                    index
                )),
            ));
        }

        let final_query = match last_clause {
            None => {
                // There is no last_clause which means we are using an index most likely because of an order_by, however we have no
                // clauses, in this case we should use the first value of the index.
                let first_index = index.properties.first().ok_or(Error::Drive(
                    DriveError::CorruptedContractIndexes("index must have properties".to_string()),
                ))?; // Index must have properties
                Self::recursive_create_query_ordered(
                    left_over_index_properties.as_slice(),
                    index.unique,
                    starts_at_document
                        .map(|(document, included)| StartAtDocument {
                            document,
                            document_type: self.document_type,
                            included,
                        })
                        .as_ref(),
                    first_index,
                    &self.order_by,
                    platform_version,
                )?
                .expect("Index must have left over properties if no last clause")
            }
            Some(where_clause) => {
                let left_to_right = if where_clause.operator.is_range() {
                    let order_clause: &OrderClause = self
                        .order_by
                        .get(where_clause.field.as_str())
                        .ok_or(Error::Query(QuerySyntaxError::MissingOrderByForRange(
                            "query must have an orderBy field for each range element",
                        )))?;

                    order_clause.ascending
                } else {
                    true
                };

                // Cursor pagination over a multi-branch level: the level fans
                // out (an `In` or range last clause), another clause or
                // left-over properties hang under each branch, and a cursor
                // document is present.
                let sibling_aware_cursor_lowering = last_clause_is_range
                    && (subquery_clause.is_some() || !left_over_index_properties.is_empty())
                    && starts_at_document.is_some();

                let starts_at_document_with_branch_included = if sibling_aware_cursor_lowering {
                    starts_at_document
                        .as_ref()
                        .map(|(document, _)| (document.clone(), true))
                } else {
                    None
                };

                // We should set the starts at document to be included for the query if there are
                // left over index properties.

                // A time-range index's transformed first level stores bucket
                // *starts*, so the cursor document's raw timestamp is not
                // comparable to this level's keys: an included cursor created
                // mid-bucket orders after the bucket-start key and would
                // suppress it, validly proving an empty page. The resolved
                // equality already pins this level to one key; the terminal
                // document-id query attached below applies the cursor.
                let last_clause_is_on_transformed_source = index
                    .time_range
                    .as_ref()
                    .is_some_and(|transform| transform.source == where_clause.field);

                let query_starts_at_document = if sibling_aware_cursor_lowering {
                    // Keep the cursor's outer branch for both startAt and
                    // startAfter: its remaining inner rows still belong to
                    // the page. Only its conditional subquery uses the cursor.
                    &starts_at_document_with_branch_included
                } else if left_over_index_properties.is_empty() {
                    if last_clause_is_on_transformed_source {
                        &None
                    } else {
                        &starts_at_document
                    }
                } else {
                    &None
                };

                let mut query = where_clause.to_path_query(
                    self.document_type,
                    query_starts_at_document,
                    left_to_right,
                    platform_version,
                )?;

                match subquery_clause {
                    None => {
                        if sibling_aware_cursor_lowering {
                            let (document, included) = starts_at_document
                                .as_ref()
                                .expect("starts_at_document was checked above");

                            let (first, deeper_left_over) = left_over_index_properties
                                .split_first()
                                .expect("left_over_index_properties was checked above");
                            let first_left_to_right = self
                                .order_by
                                .get(first.name.as_str())
                                .map(|order_clause| order_clause.ascending)
                                .unwrap_or(first.ascending);

                            // Branches ordered after the cursor's take everything.
                            let mut default_subquery =
                                Query::new_with_direction(first_left_to_right);
                            default_subquery.insert_all();
                            Self::recursive_insert_on_query_ordered_with_cursor(
                                &mut default_subquery,
                                deeper_left_over,
                                index.unique,
                                None,
                                first_left_to_right,
                                &self.order_by,
                                platform_version,
                            )?;
                            query.set_subquery(default_subquery);
                            query.set_subquery_key(first.name.as_bytes().to_vec());

                            // The cursor's branch continues from the cursor.
                            let start_at_key = document
                                .get_raw_for_document_type(
                                    where_clause.field.as_str(),
                                    self.document_type,
                                    None,
                                    platform_version,
                                )
                                .ok()
                                .flatten();
                            Self::recursive_conditional_insert_on_query_ordered(
                                &mut query,
                                start_at_key,
                                left_over_index_properties.as_slice(),
                                index.unique,
                                &StartAtDocument {
                                    document: document.clone(),
                                    document_type: self.document_type,
                                    included: *included,
                                },
                                left_to_right,
                                &self.order_by,
                                platform_version,
                            )?;
                        } else if last_clause_is_on_transformed_source
                            && left_over_index_properties.is_empty()
                        {
                            // The cursor was deliberately withheld from the
                            // bucket-keyed level above; apply it here by
                            // bucket membership instead. A cursor inside the
                            // selected bucket continues the walk at its
                            // document id (for a unique index the bucket
                            // holds exactly the cursor document, so excluded
                            // means the page is exhausted); a cursor from
                            // outside the bucket cannot order within it and
                            // is ignored.
                            let cursor_in_bucket = match &starts_at_document {
                                None => None,
                                Some((document, included)) => {
                                    let transform = index
                                        .time_range
                                        .as_ref()
                                        .expect("checked by last_clause_is_on_transformed_source");
                                    let bucket_key = self.document_type.serialize_value_for_key(
                                        where_clause.field.as_str(),
                                        &where_clause.value,
                                        platform_version,
                                    )?;
                                    document
                                        .get_raw_for_document_type(
                                            where_clause.field.as_str(),
                                            self.document_type,
                                            None,
                                            platform_version,
                                        )?
                                        .filter(|raw| {
                                            transform.entry_keys_for_raw(raw).contains(&bucket_key)
                                        })
                                        .map(|_| (document, *included))
                                }
                            };
                            match cursor_in_bucket {
                                Some((document, included)) if !index.unique => {
                                    query.set_subquery_key(vec![0]);
                                    query.set_subquery(Self::inner_query_from_starts_at_for_id(
                                        Some(&StartAtDocument {
                                            document: document.clone(),
                                            document_type: self.document_type,
                                            included,
                                        }),
                                        left_to_right,
                                    ));
                                }
                                cursor => {
                                    if matches!(cursor, Some((_, false))) {
                                        // Unique: the excluded cursor is the
                                        // bucket's only document.
                                        query = Query::new_with_direction(left_to_right);
                                    }
                                    Self::recursive_insert_on_query_ordered_with_cursor(
                                        &mut query,
                                        left_over_index_properties.as_slice(),
                                        index.unique,
                                        None,
                                        left_to_right,
                                        &self.order_by,
                                        platform_version,
                                    )?;
                                }
                            }
                        } else {
                            Self::recursive_insert_on_query_ordered_with_cursor(
                                &mut query,
                                left_over_index_properties.as_slice(),
                                index.unique,
                                starts_at_document
                                    .map(|(document, included)| StartAtDocument {
                                        document,
                                        document_type: self.document_type,
                                        included,
                                    })
                                    .as_ref(),
                                left_to_right,
                                &self.order_by,
                                platform_version,
                            )?;
                        }
                    }
                    Some(subquery_where_clause) => {
                        let order_clause: &OrderClause = self
                            .order_by
                            .get(subquery_where_clause.field.as_str())
                            .ok_or(Error::Query(QuerySyntaxError::MissingOrderByForRange(
                                "query must have an orderBy field for each range element",
                            )))?;
                        let mut subquery = subquery_where_clause.to_path_query(
                            self.document_type,
                            &None,
                            order_clause.ascending,
                            platform_version,
                        )?;
                        Self::recursive_insert_on_query_ordered_with_cursor(
                            &mut subquery,
                            left_over_index_properties.as_slice(),
                            index.unique,
                            None,
                            order_clause.ascending,
                            &self.order_by,
                            platform_version,
                        )?;
                        let subindex = subquery_where_clause.field.as_bytes().to_vec();
                        query.set_subquery_key(subindex.clone());
                        query.set_subquery(subquery);

                        if let Some((document, included)) = starts_at_document {
                            // The default subquery above keeps the original
                            // predicate on every later sibling. Intersect it
                            // with the cursor only on the cursor's outer key.
                            // Non-unique or deeper levels must keep the inner
                            // key so their conditional query can paginate
                            // within it, including document-id ties.
                            let inner_included =
                                included || !index.unique || !left_over_index_properties.is_empty();
                            let mut cursor_subquery = subquery_where_clause.to_path_query(
                                self.document_type,
                                &Some((document.clone(), inner_included)),
                                order_clause.ascending,
                                platform_version,
                            )?;
                            Self::recursive_insert_on_query_ordered_with_cursor(
                                &mut cursor_subquery,
                                left_over_index_properties.as_slice(),
                                index.unique,
                                None,
                                order_clause.ascending,
                                &self.order_by,
                                platform_version,
                            )?;
                            let outer_key = document
                                .get_raw_for_document_type(
                                    where_clause.field.as_str(),
                                    self.document_type,
                                    None,
                                    platform_version,
                                )?
                                .unwrap_or_default();
                            let inner_key = document.get_raw_for_document_type(
                                subquery_where_clause.field.as_str(),
                                self.document_type,
                                None,
                                platform_version,
                            )?;
                            if !index.unique && left_over_index_properties.is_empty() {
                                // The terminal id bound follows the range's
                                // direction, including descending duplicate
                                // index values. The legacy id helper always
                                // bounds from below, even for a reverse walk.
                                let id_query = Self::inner_query_starts_from_key(
                                    Some(document.id().to_vec()),
                                    order_clause.ascending,
                                    included,
                                );
                                cursor_subquery.add_conditional_subquery(
                                    QueryItem::Key(inner_key.unwrap_or_default()),
                                    Some(vec![vec![0]]),
                                    Some(id_query),
                                );
                            } else {
                                Self::recursive_conditional_insert_on_query_ordered(
                                    &mut cursor_subquery,
                                    inner_key,
                                    left_over_index_properties.as_slice(),
                                    index.unique,
                                    &StartAtDocument {
                                        document,
                                        document_type: self.document_type,
                                        included,
                                    },
                                    order_clause.ascending,
                                    &self.order_by,
                                    platform_version,
                                )?;
                            }
                            query.add_conditional_subquery(
                                QueryItem::Key(outer_key),
                                Some(vec![subindex]),
                                Some(cursor_subquery),
                            );
                        }
                    }
                };

                query
            }
        };

        let (intermediate_indexes, last_indexes) =
            index.properties.split_at(intermediate_values.len());

        // Now we should construct the path
        let last_index = last_indexes.first().ok_or(Error::Query(
            QuerySyntaxError::QueryOnDocumentTypeWithNoIndexes(
                "document query has no index with fields",
            ),
        ))?;

        let mut path = document_type_path;

        // Path segments are level keys: grid-qualified for a time-range
        // index's first property (`Index::level_key_for_property`), the bare
        // property name everywhere else. The values pushed between them are
        // untouched — a bucket start is encoded exactly like a timestamp.
        for (intermediate_index, intermediate_value) in
            intermediate_indexes.iter().zip(intermediate_values.iter())
        {
            path.push(
                index
                    .level_key_for_property(&intermediate_index.name)
                    .into_bytes(),
            );
            path.push(intermediate_value.as_slice().to_vec());
        }

        path.push(index.level_key_for_property(&last_index.name).into_bytes());

        Ok(PathQuery::new(
            path,
            SizedQuery::new(final_query, self.limit, self.offset),
        ))
    }
}

/// True when the query's clauses line up with the positional path
/// construction: the first `equality_prefix_len` index properties all
/// carry equality clauses (they are the levels `intermediate_values`
/// keys), the terminal clause (equality, range, or `in`) sits on the
/// property right after that prefix, and a subquery clause (the later
/// of an `in`/range pair) on the property after it. Unreachable-false
/// once index selection ran through the protocol-version-14 contiguous
/// matcher; kept as defense in depth for this file's `split_at`.
#[cfg(any(feature = "server", feature = "verify"))]
fn query_covers_index_prefix_contiguously(
    index_properties: &[IndexProperty],
    is_equality_field: &dyn Fn(&str) -> bool,
    last_clause: Option<&WhereClause>,
    subquery_clause: Option<&WhereClause>,
    equality_prefix_len: usize,
) -> bool {
    if index_properties.len() < equality_prefix_len {
        return false;
    }
    let prefix_covered = index_properties[..equality_prefix_len]
        .iter()
        .all(|property| is_equality_field(property.name.as_str()));
    let clause_sits_at = |clause: Option<&WhereClause>, position: usize| match clause {
        Some(clause) => index_properties
            .get(position)
            .is_some_and(|property| property.name == clause.field),
        None => true,
    };
    prefix_covered
        && clause_sits_at(last_clause, equality_prefix_len)
        && clause_sits_at(subquery_clause, equality_prefix_len + 1)
}
