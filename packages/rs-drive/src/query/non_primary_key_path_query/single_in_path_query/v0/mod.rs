//! v0 of the at-most-one-`In` lowering (protocol version 14, reached
//! through v1 of the non-primary-key lowering). Differs from the frozen
//! pre-v14 construction in `non_primary_key_path_query::v0` in two
//! ways: cursor pagination over a multi-branch level is sibling-branch
//! correct, and every cursorless left-over level takes its direction
//! from `order_by`. Frozen: changes to already-live behavior belong in
//! a new version module.

use crate::error::drive::DriveError;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use crate::query::conditions::WhereClause;
use crate::query::ordering::OrderClause;
use crate::query::{DriveDocumentQuery, StartAtDocument};
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::data_contract::document_type::IndexProperty;
use dpp::document::document_methods::DocumentMethodsV0;
use dpp::document::Document;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use grovedb::{PathQuery, Query, QueryItem, SizedQuery};
use indexmap::IndexMap;

impl<'a> DriveDocumentQuery<'a> {
    #[cfg(any(feature = "server", feature = "verify"))]
    /// Counterpart of [`Self::recursive_create_query`] for this
    /// lowering: identical cursor threading, but recursion goes through
    /// [`Self::recursive_insert_on_query_ordered_with_cursor`] so every
    /// cursorless left-over level takes its direction from `order_by`.
    pub(in crate::query) fn recursive_create_query_ordered(
        left_over_index_properties: &[&IndexProperty],
        unique: bool,
        starts_at_document: Option<&StartAtDocument>, //for key level, included
        indexed_property: &IndexProperty,
        order_by: &IndexMap<String, OrderClause>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Query>, Error> {
        match left_over_index_properties.split_first() {
            None => Ok(None),
            Some((first, left_over)) => {
                let left_to_right = order_by
                    .get(first.name.as_str())
                    .map(|order_clause| order_clause.ascending)
                    .unwrap_or(first.ascending);

                let mut inner_query = Self::inner_query_from_starts_at(
                    starts_at_document,
                    indexed_property,
                    left_to_right,
                    platform_version,
                )?;
                Self::recursive_insert_on_query_ordered_with_cursor(
                    &mut inner_query,
                    left_over,
                    unique,
                    starts_at_document,
                    left_to_right,
                    order_by,
                    platform_version,
                )?;
                Ok(Some(inner_query))
            }
        }
    }

    #[cfg(any(feature = "server", feature = "verify"))]
    /// Counterpart of [`Self::recursive_insert_on_query`] for this
    /// lowering: identical cursor threading, but a cursorless level
    /// takes its direction from `order_by` (falling back to the index
    /// property's own) where the v0 helper always used the index
    /// property's.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::query) fn recursive_insert_on_query_ordered_with_cursor(
        query: &mut Query,
        left_over_index_properties: &[&IndexProperty],
        unique: bool,
        starts_at_document: Option<&StartAtDocument>, //for key level, included
        default_left_to_right: bool,
        order_by: &IndexMap<String, OrderClause>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match left_over_index_properties.split_first() {
            None => {
                match unique {
                    true => {
                        query.set_subquery_key(vec![0]);

                        // In the case things are NULL we allow to have multiple values
                        let inner_query = Self::inner_query_from_starts_at_for_id(
                            starts_at_document,
                            true, //for ids we always go left to right
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

                        let inner_query = Self::inner_query_from_starts_at_for_id(
                            starts_at_document,
                            default_left_to_right,
                        );

                        query.add_conditional_subquery(
                            QueryItem::Key(b"".to_vec()),
                            Some(vec![vec![0]]),
                            Some(inner_query),
                        );
                    }
                }
                Ok(())
            }
            Some((first, left_over)) => {
                let left_to_right = order_by
                    .get(first.name.as_str())
                    .map(|order_clause| order_clause.ascending)
                    .unwrap_or(first.ascending);

                if let Some(start_at_document_inner) = starts_at_document {
                    let StartAtDocument {
                        document,
                        document_type,
                        included,
                    } = start_at_document_inner;
                    let start_at_key = document
                        .get_raw_for_document_type(
                            first.name.as_str(),
                            *document_type,
                            None,
                            platform_version,
                        )
                        .ok()
                        .flatten();

                    // We should always include if we have left_over
                    let non_conditional_included =
                        !left_over.is_empty() || *included || start_at_key.is_none();

                    let mut non_conditional_query = Self::inner_query_starts_from_key(
                        start_at_key.clone(),
                        left_to_right,
                        non_conditional_included,
                    );

                    Self::recursive_insert_on_query_ordered_with_cursor(
                        &mut non_conditional_query,
                        left_over,
                        unique,
                        None,
                        left_to_right,
                        order_by,
                        platform_version,
                    )?;

                    Self::recursive_conditional_insert_on_query_ordered(
                        &mut non_conditional_query,
                        start_at_key,
                        left_over,
                        unique,
                        start_at_document_inner,
                        left_to_right,
                        order_by,
                        platform_version,
                    )?;

                    query.set_subquery(non_conditional_query);
                } else {
                    let mut inner_query = Query::new_with_direction(left_to_right);
                    inner_query.insert_all();
                    Self::recursive_insert_on_query_ordered_with_cursor(
                        &mut inner_query,
                        left_over,
                        unique,
                        None,
                        left_to_right,
                        order_by,
                        platform_version,
                    )?;
                    query.set_subquery(inner_query);
                }
                query.set_subquery_key(first.name.as_bytes().to_vec());
                Ok(())
            }
        }
    }

    #[cfg(any(feature = "server", feature = "verify"))]
    /// Counterpart of [`Self::recursive_conditional_insert_on_query`]
    /// for this lowering: identical conditional refinement along the
    /// cursor's path, but the cursorless sub-levels it creates go
    /// through [`Self::recursive_insert_on_query_ordered_with_cursor`]
    /// so they take their direction from `order_by`.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::query) fn recursive_conditional_insert_on_query_ordered(
        query: &mut Query,
        conditional_value: Option<Vec<u8>>,
        left_over_index_properties: &[&IndexProperty],
        unique: bool,
        starts_at_document: &StartAtDocument,
        default_left_to_right: bool,
        order_by: &IndexMap<String, OrderClause>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match left_over_index_properties.split_first() {
            None => {
                match unique {
                    true => {
                        // In the case things are NULL we allow to have multiple values
                        let inner_query = Self::inner_query_from_starts_at_for_id(
                            Some(starts_at_document),
                            true, //for ids we always go left to right
                        );
                        query.add_conditional_subquery(
                            QueryItem::Key(b"".to_vec()),
                            Some(vec![vec![0]]),
                            Some(inner_query),
                        );
                    }
                    false => {
                        let inner_query = Self::inner_query_from_starts_at_for_id(
                            Some(starts_at_document),
                            default_left_to_right,
                        );

                        query.add_conditional_subquery(
                            QueryItem::Key(conditional_value.unwrap_or_default()),
                            Some(vec![vec![0]]),
                            Some(inner_query),
                        );
                    }
                }
            }
            Some((first, left_over)) => {
                let left_to_right = order_by
                    .get(first.name.as_str())
                    .map(|order_clause| order_clause.ascending)
                    .unwrap_or(first.ascending);

                let StartAtDocument {
                    document,
                    document_type,
                    ..
                } = starts_at_document;

                let lower_start_at_key = document
                    .get_raw_for_document_type(
                        first.name.as_str(),
                        *document_type,
                        None,
                        platform_version,
                    )
                    .ok()
                    .flatten();

                // We include it if we are not unique,
                // or if we are unique but the value is empty
                let non_conditional_included = !unique || lower_start_at_key.is_none();

                let mut non_conditional_query = Self::inner_query_starts_from_key(
                    lower_start_at_key.clone(),
                    left_to_right,
                    non_conditional_included,
                );

                Self::recursive_insert_on_query_ordered_with_cursor(
                    &mut non_conditional_query,
                    left_over,
                    unique,
                    None,
                    left_to_right,
                    order_by,
                    platform_version,
                )?;

                Self::recursive_conditional_insert_on_query_ordered(
                    &mut non_conditional_query,
                    lower_start_at_key,
                    left_over,
                    unique,
                    starts_at_document,
                    left_to_right,
                    order_by,
                    platform_version,
                )?;

                query.add_conditional_subquery(
                    QueryItem::Key(conditional_value.unwrap_or_default()),
                    Some(vec![first.name.as_bytes().to_vec()]),
                    Some(non_conditional_query),
                );
            }
        }
        Ok(())
    }

    #[cfg(any(feature = "server", feature = "verify"))]
    /// v0 lowering for queries with at most one `In` clause. Differs
    /// from the pre-v14 construction
    /// ([`Self::get_non_primary_key_path_query_v0`]) in two ways: cursor
    /// pagination over a multi-branch level (an `In` or range last
    /// clause with left-over index properties) trims the branches
    /// ordered before the cursor's branch, gives the branches ordered
    /// after it an unfiltered default subquery, and refines only the
    /// cursor's own branch with a conditional subquery — where the
    /// pre-v14 construction baked the cursor's per-level start keys into
    /// the default subquery applied to every branch, silently dropping
    /// later branches' values below the cursor and including earlier
    /// branches' values above it. And every cursorless left-over level
    /// takes its direction from `order_by` instead of always the index
    /// property's own.
    pub(in crate::query) fn get_non_primary_key_single_in_path_query_v0(
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
                // out (an `In` or range last clause), left-over properties
                // hang under each branch, and a cursor document is present.
                let sibling_aware_cursor_lowering = last_clause_is_range
                    && subquery_clause.is_none()
                    && !left_over_index_properties.is_empty()
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

                let query_starts_at_document = if left_over_index_properties.is_empty() {
                    if last_clause_is_on_transformed_source {
                        &None
                    } else {
                        &starts_at_document
                    }
                } else if sibling_aware_cursor_lowering {
                    // The cursor's branch always stays included at this level,
                    // trimming only the branches ordered before it; whether
                    // the cursor document itself is included is decided by the
                    // conditional subquery below.
                    &starts_at_document_with_branch_included
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
                            &starts_at_document,
                            order_clause.ascending,
                            platform_version,
                        )?;
                        Self::recursive_insert_on_query_ordered_with_cursor(
                            &mut subquery,
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
                        let subindex = subquery_where_clause.field.as_bytes().to_vec();
                        query.set_subquery_key(subindex);
                        query.set_subquery(subquery);
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
