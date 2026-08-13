//! v0 of the non-primary-key path-query lowering — the behavior every
//! protocol version up to 13 committed to: at most one non-primary-key
//! `In` clause per query, with the cursor machinery that supports
//! `startAt` / `startAfter` under a single-branch ancestry. Frozen:
//! changes to already-live behavior belong in a new version module.

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
use indexmap::IndexMap;

impl<'a> DriveDocumentQuery<'a> {
    #[cfg(any(feature = "server", feature = "verify"))]
    /// Returns a `Query` that either starts at or after the given document ID if given.
    pub(in crate::query) fn inner_query_from_starts_at_for_id(
        starts_at_document: Option<&StartAtDocument>,
        left_to_right: bool,
    ) -> Query {
        // We only need items after the start at document
        let mut inner_query = Query::new_with_direction(left_to_right);

        if let Some(StartAtDocument {
            document, included, ..
        }) = starts_at_document
        {
            let start_at_key = document.id().to_vec();
            if *included {
                inner_query.insert_range_from(start_at_key..)
            } else {
                inner_query.insert_range_after(start_at_key..)
            }
        } else {
            // No starts at document, take all NULL items
            inner_query.insert_all();
        }
        inner_query
    }

    #[cfg(any(feature = "server", feature = "verify"))]
    /// Returns a `Query` that either starts at or after the given key.
    pub(in crate::query) fn inner_query_starts_from_key(
        start_at_key: Option<Vec<u8>>,
        left_to_right: bool,
        included: bool,
    ) -> Query {
        // We only need items after the start at document
        let mut inner_query = Query::new_with_direction(left_to_right);

        if left_to_right {
            if let Some(start_at_key) = start_at_key {
                if included {
                    inner_query.insert_range_from(start_at_key..);
                } else {
                    inner_query.insert_range_after(start_at_key..);
                }
            } else {
                inner_query.insert_all();
            }
        } else if included {
            if let Some(start_at_key) = start_at_key {
                inner_query.insert_range_to_inclusive(..=start_at_key);
            } else {
                inner_query.insert_key(vec![]);
            }
        } else if let Some(start_at_key) = start_at_key {
            inner_query.insert_range_to(..start_at_key);
        } else {
            //todo: really not sure if this is correct
            // Should investigate more
            inner_query.insert_key(vec![]);
        }

        inner_query
    }

    #[cfg(any(feature = "server", feature = "verify"))]
    /// Returns a `Query` that either starts at or after the given document if given.
    pub(in crate::query) fn inner_query_from_starts_at(
        starts_at_document: Option<&StartAtDocument>,
        indexed_property: &IndexProperty,
        left_to_right: bool,
        platform_version: &PlatformVersion,
    ) -> Result<Query, Error> {
        let mut inner_query = Query::new_with_direction(left_to_right);
        if let Some(StartAtDocument {
            document,
            document_type,
            included,
        }) = starts_at_document
        {
            // We only need items after the start at document
            let start_at_key = document.get_raw_for_document_type(
                indexed_property.name.as_str(),
                *document_type,
                None,
                platform_version,
            )?;
            // We want to get items starting at the start key
            if let Some(start_at_key) = start_at_key {
                if left_to_right {
                    if *included {
                        inner_query.insert_range_from(start_at_key..)
                    } else {
                        inner_query.insert_range_after(start_at_key..)
                    }
                } else if *included {
                    inner_query.insert_range_to_inclusive(..=start_at_key)
                } else {
                    inner_query.insert_range_to(..start_at_key)
                }
            } else if left_to_right {
                inner_query.insert_all();
            } else {
                inner_query.insert_key(vec![]);
            }
        } else {
            // No starts at document, take all NULL items
            inner_query.insert_all();
        }
        Ok(inner_query)
    }

    #[cfg(any(feature = "server", feature = "verify"))]
    pub(in crate::query) fn recursive_create_query(
        left_over_index_properties: &[&IndexProperty],
        unique: bool,
        starts_at_document: Option<&StartAtDocument>, //for key level, included
        indexed_property: &IndexProperty,
        order_by: Option<&IndexMap<String, OrderClause>>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Query>, Error> {
        match left_over_index_properties.split_first() {
            None => Ok(None),
            Some((first, left_over)) => {
                let left_to_right = if let Some(order_by) = order_by {
                    order_by
                        .get(first.name.as_str())
                        .map(|order_clause| order_clause.ascending)
                        .unwrap_or(first.ascending)
                } else {
                    first.ascending
                };

                let mut inner_query = Self::inner_query_from_starts_at(
                    starts_at_document,
                    indexed_property,
                    left_to_right,
                    platform_version,
                )?;
                DriveDocumentQuery::recursive_insert_on_query(
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
    /// Recursively queries as long as there are leftover index properties.
    /// The in_start_at_document_sub_path_needing_conditional is interesting.
    /// It indicates whether the start at document should be applied as a conditional
    /// For example if we have a tree
    /// Root
    /// ├── model
    /// │   ├── sedan
    /// │   │   ├── brand_name
    /// │   │   │   ├── Honda
    /// │   │   │   │   ├── car_type
    /// │   │   │   │   │   ├── Accord
    /// │   │   │   │   │   │   ├── 0
    /// │   │   │   │   │   │   │   ├── a47d2...
    /// │   │   │   │   │   │   │   ├── e19c8...
    /// │   │   │   │   │   │   │   └── f1a7b...
    /// │   │   │   │   │   └── Civic
    /// │   │   │   │   │       ├── 0
    /// │   │   │   │   │       │   ├── b65a7...
    /// │   │   │   │   │       │   └── c43de...
    /// │   │   │   ├── Toyota
    /// │   │   │   │   ├── car_type
    /// │   │   │   │   │   ├── Camry
    /// │   │   │   │   │   │   ├── 0
    /// │   │   │   │   │   │   │   └── 1a9d2...
    /// │   │   │   │   │   └── Corolla
    /// │   │   │   │   │       ├── 0
    /// │   │   │   │   │       │   ├── 3f7b4...
    /// │   │   │   │   │       │   ├── 4e8fa...
    /// │   │   │   │   │       │   └── 9b1c6...
    /// │   ├── suv
    /// │   │   ├── brand_name
    /// │   │   │   ├── Ford*
    /// │   │   │   │   ├── car_type*
    /// │   │   │   │   │   ├── Escape*
    /// │   │   │   │   │   │   ├── 0
    /// │   │   │   │   │   │   │   ├── 102bc...
    /// │   │   │   │   │   │   │   ├── 29f8e... <- Set After this document
    /// │   │   │   │   │   │   │   └── 6b1a3...
    /// │   │   │   │   │   └── Explorer
    /// │   │   │   │   │       ├── 0
    /// │   │   │   │   │       │   ├── b2a9d...
    /// │   │   │   │   │       │   └── f4d5c...
    /// │   │   │   ├── Nissan
    /// │   │   │   │   ├── car_type
    /// │   │   │   │   │   ├── Rogue
    /// │   │   │   │   │   │   ├── 0
    /// │   │   │   │   │   │   │   ├── 5a9c3...
    /// │   │   │   │   │   │   │   └── 7e4b9...
    /// │   │   │   │   │   └── Murano
    /// │   │   │   │   │       ├── 0
    /// │   │   │   │   │       │   ├── 8f6a2...
    /// │   │   │   │   │       │   └── 9c7d4...
    /// │   ├── truck
    /// │   │   ├── brand_name
    /// │   │   │   ├── Ford
    /// │   │   │   │   ├── car_type
    /// │   │   │   │   │   ├── F-150
    /// │   │   │   │   │   │   ├── 0
    /// │   │   │   │   │   │   │   ├── 72a3b...
    /// │   │   │   │   │   │   │   └── 94c8e...
    /// │   │   │   │   │   └── Ranger
    /// │   │   │   │   │       ├── 0
    /// │   │   │   │   │       │   ├── 3f4b1...
    /// │   │   │   │   │       │   ├── 6e7d2...
    /// │   │   │   │   │       │   └── 8a1f5...
    /// │   │   │   ├── Toyota
    /// │   │   │   │   ├── car_type
    /// │   │   │   │   │   ├── Tundra
    /// │   │   │   │   │   │   ├── 0
    /// │   │   │   │   │   │   │   ├── 7c9a4...
    /// │   │   │   │   │   │   │   └── a5d1e...
    /// │   │   │   │   │   └── Tacoma
    /// │   │   │   │   │       ├── 0
    /// │   │   │   │   │       │   ├── 1e7f4...
    /// │   │   │   │   │       │   └── 6b9d3...
    ///
    /// let's say we are asking for suv's after 29f8e
    /// here the * denotes the area needing a conditional
    /// We need a conditional subquery on Ford to say only things after Ford (with Ford included)
    /// We need a conditional subquery on Escape to say only things after Escape (with Escape included)
    pub(in crate::query) fn recursive_insert_on_query(
        query: &mut Query,
        left_over_index_properties: &[&IndexProperty],
        unique: bool,
        starts_at_document: Option<&StartAtDocument>, //for key level, included
        default_left_to_right: bool,
        order_by: Option<&IndexMap<String, OrderClause>>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Query>, Error> {
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
                Ok(None)
            }
            Some((first, left_over)) => {
                let left_to_right = if let Some(order_by) = order_by {
                    order_by
                        .get(first.name.as_str())
                        .map(|order_clause| order_clause.ascending)
                        .unwrap_or(first.ascending)
                } else {
                    first.ascending
                };

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

                    // We place None here on purpose, this has been well-thought-out
                    // and should not change. The reason is that the path of the start
                    // at document is used only on the conditional subquery and not on the
                    // main query
                    // for example in the following
                    // Our query will be with $ownerId == a3f9b81c4d7e6a9f5b1c3e8a2d9c4f7b
                    // With start after 8f2d5
                    // We want to get from 2024-11-17T12:45:00Z
                    // withdrawal
                    // ├── $ownerId
                    // │   ├── a3f9b81c4d7e6a9f5b1c3e8a2d9c4f7b
                    // │   │   ├── $updatedAt
                    // │   │   │   ├── 2024-11-17T12:45:00Z <- conditional subquery here
                    // │   │   │   │   ├── status
                    // │   │   │   │   │   ├── 0
                    // │   │   │   │   │   │   ├── 7a9f1...
                    // │   │   │   │   │   │   └── 4b8c3...
                    // │   │   │   │   │   ├── 1
                    // │   │   │   │   │   │   ├── 8f2d5... <- start after
                    // │   │   │   │   │   │   └── 5c1e4...
                    // │   │   │   │   │   ├── 2
                    // │   │   │   │   │   │   ├── 2e7a9...
                    // │   │   │   │   │   │   └── 1c8b3...
                    // │   │   │   ├── 2024-11-18T11:25:00Z <- we want all statuses here, so normal subquery, with None as start at document
                    // │   │   │   │   ├── status
                    // │   │   │   │   │   ├── 0
                    // │   │   │   │   │   │   └── 1a4f2...
                    // │   │   │   │   │   ├── 2
                    // │   │   │   │   │   │   ├── 3e7a9...
                    // │   │   │   │   │   │   └── 198b4...
                    // │   ├── b6d7e9c4a5f2b3d8e1a7c9f4b1e8a3f
                    // │   │   ├── $updatedAt
                    // │   │   │   ├── 2024-11-17T13:30:00Z
                    // │   │   │   │   ├── status
                    // │   │   │   │   │   ├── 0
                    // │   │   │   │   │   │   ├── 6d7e2...
                    // │   │   │   │   │   │   └── 9c7f5...
                    // │   │   │   │   │   ├── 3
                    // │   │   │   │   │   │   ├── 3a9b7...
                    // │   │   │   │   │   │   └── 8e5c4...
                    // │   │   │   │   │   ├── 4
                    // │   │   │   │   │   │   ├── 1f7a8...
                    // │   │   │   │   │   │   └── 2c9b3...
                    // println!("going to call recursive_insert_on_query on non_conditional_query {} with left_over {:?}", non_conditional_query, left_over);
                    DriveDocumentQuery::recursive_insert_on_query(
                        &mut non_conditional_query,
                        left_over,
                        unique,
                        None,
                        left_to_right,
                        order_by,
                        platform_version,
                    )?;

                    DriveDocumentQuery::recursive_conditional_insert_on_query(
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
                    let mut inner_query = Query::new_with_direction(first.ascending);
                    inner_query.insert_all();
                    DriveDocumentQuery::recursive_insert_on_query(
                        &mut inner_query,
                        left_over,
                        unique,
                        starts_at_document,
                        left_to_right,
                        order_by,
                        platform_version,
                    )?;
                    query.set_subquery(inner_query);
                }
                query.set_subquery_key(first.name.as_bytes().to_vec());
                Ok(None)
            }
        }
    }

    #[cfg(any(feature = "server", feature = "verify"))]
    #[allow(clippy::too_many_arguments)]
    pub(in crate::query) fn recursive_conditional_insert_on_query(
        query: &mut Query,
        conditional_value: Option<Vec<u8>>,
        left_over_index_properties: &[&IndexProperty],
        unique: bool,
        starts_at_document: &StartAtDocument,
        default_left_to_right: bool,
        order_by: Option<&IndexMap<String, OrderClause>>,
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
                let left_to_right = if let Some(order_by) = order_by {
                    order_by
                        .get(first.name.as_str())
                        .map(|order_clause| order_clause.ascending)
                        .unwrap_or(first.ascending)
                } else {
                    first.ascending
                };

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

                DriveDocumentQuery::recursive_insert_on_query(
                    &mut non_conditional_query,
                    left_over,
                    unique,
                    None,
                    left_to_right,
                    order_by,
                    platform_version,
                )?;

                DriveDocumentQuery::recursive_conditional_insert_on_query(
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
    /// v0 of the non-primary-key path query lowering: at most one `In`
    /// clause per query, the behavior every protocol version up to 13
    /// committed to.
    pub(in crate::query) fn get_non_primary_key_path_query_v0(
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
                        // Both an `in` clause and a range clause are present.
                        // The outer path query must operate on the field that
                        // appears *earlier* (closer to the index root) in the
                        // chosen index, and the other clause becomes the leaf
                        // subquery. Without this ordering, a query like
                        // `status > 0 AND transactionIndex in [..]` on an index
                        // `[status, transactionIndex]` builds a path that
                        // terminates at the `status` subtree while the primary
                        // query iterates `transactionIndex` keys, silently
                        // returning []. See issue #2409.
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
                Self::recursive_create_query(
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
                    Some(&self.order_by),
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

                // We should set the starts at document to be included for the query if there are
                // left over index properties.

                let query_starts_at_document = if left_over_index_properties.is_empty() {
                    &starts_at_document
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
                        Self::recursive_insert_on_query(
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
                            Some(&self.order_by),
                            platform_version,
                        )?;
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
                        Self::recursive_insert_on_query(
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
                            Some(&self.order_by),
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

        for (intermediate_index, intermediate_value) in
            intermediate_indexes.iter().zip(intermediate_values.iter())
        {
            path.push(intermediate_index.name.as_bytes().to_vec());
            path.push(intermediate_value.as_slice().to_vec());
        }

        path.push(last_index.name.as_bytes().to_vec());

        Ok(PathQuery::new(
            path,
            SizedQuery::new(final_query, self.limit, self.offset),
        ))
    }
}
