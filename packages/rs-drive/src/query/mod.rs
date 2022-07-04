use std::collections::BTreeMap;
use std::ops::BitXor;

use ciborium::value::Value;
pub use grovedb::{
    Element, Error as GroveError, GroveDb, PathQuery, Query, QueryItem, SizedQuery, TransactionArg,
};
use indexmap::IndexMap;
use sqlparser::ast;
use sqlparser::ast::TableFactor::Table;
use sqlparser::ast::Value::Number;
use sqlparser::ast::{OrderByExpr, Select, Statement};
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;

use conditions::WhereOperator::{Equal, In};
pub use conditions::{WhereClause, WhereOperator};
pub use ordering::OrderClause;

use crate::common::bytes_for_system_value;
use crate::contract::{document::Document, Contract, DocumentType, Index, IndexProperty};
use crate::drive::object_size_info::KeyValueInfo;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::query::QueryError;
use crate::error::structure::StructureError;
use crate::error::Error;
use crate::error::Error::GroveDB;
use crate::fee::calculate_fee;
use crate::fee::op::DriveOperation;

pub mod conditions;
mod defaults;
pub mod ordering;
mod test_index;

#[derive(Clone, Debug, PartialEq, Default)]
pub struct InternalClauses {
    pub primary_key_in_clause: Option<WhereClause>,
    pub primary_key_equal_clause: Option<WhereClause>,
    pub in_clause: Option<WhereClause>,
    pub range_clause: Option<WhereClause>,
    pub equal_clauses: BTreeMap<String, WhereClause>,
}

impl InternalClauses {
    pub fn verify(&self) -> bool {
        // There can only be 1 primary key clause, or many other clauses
        if self
            .primary_key_in_clause
            .is_some()
            .bitxor(self.primary_key_equal_clause.is_some())
        {
            // One is set, all rest must be empty
            !(self.in_clause.is_some()
                || self.range_clause.is_some()
                || !self.equal_clauses.is_empty())
        } else {
            !(self.primary_key_in_clause.is_some() && self.primary_key_equal_clause.is_some())
        }
    }

    pub fn is_for_primary_key(&self) -> bool {
        self.primary_key_in_clause.is_some() || self.primary_key_equal_clause.is_some()
    }

    pub fn is_empty(&self) -> bool {
        self.in_clause.is_none()
            && self.range_clause.is_none()
            && self.equal_clauses.is_empty()
            && self.primary_key_in_clause.is_none()
            && self.primary_key_equal_clause.is_none()
    }

    fn extract_from_clauses(all_where_clauses: Vec<WhereClause>) -> Result<Self, Error> {
        let primary_key_equal_clauses_array = all_where_clauses
            .iter()
            .filter_map(|where_clause| match where_clause.operator {
                Equal => match where_clause.is_identifier() {
                    true => Some(where_clause.clone()),
                    false => None,
                },
                _ => None,
            })
            .collect::<Vec<WhereClause>>();

        let primary_key_in_clauses_array = all_where_clauses
            .iter()
            .filter_map(|where_clause| match where_clause.operator {
                In => match where_clause.is_identifier() {
                    true => Some(where_clause.clone()),
                    false => None,
                },
                _ => None,
            })
            .collect::<Vec<WhereClause>>();

        let (equal_clauses, range_clause, in_clause) =
            WhereClause::group_clauses(&all_where_clauses)?;

        let primary_key_equal_clause = match primary_key_equal_clauses_array.len() {
            0 => Ok(None),
            1 => Ok(Some(
                primary_key_equal_clauses_array
                    .get(0)
                    .expect("there must be a value")
                    .clone(),
            )),
            _ => Err(Error::Query(
                QueryError::DuplicateNonGroupableClauseSameField(
                    "There should only be one equal clause for the primary key",
                ),
            )),
        }?;

        let primary_key_in_clause = match primary_key_in_clauses_array.len() {
            0 => Ok(None),
            1 => Ok(Some(
                primary_key_in_clauses_array
                    .get(0)
                    .expect("there must be a value")
                    .clone(),
            )),
            _ => Err(Error::Query(
                QueryError::DuplicateNonGroupableClauseSameField(
                    "There should only be one in clause for the primary key",
                ),
            )),
        }?;

        let internal_clauses = InternalClauses {
            primary_key_equal_clause,
            primary_key_in_clause,
            in_clause,
            range_clause,
            equal_clauses,
        };

        match internal_clauses.verify() {
            true => Ok(internal_clauses),
            false => Err(Error::Query(QueryError::InvalidWhereClauseComponents(
                "Query has invalid where clauses",
            ))),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct DriveQuery<'a> {
    pub contract: &'a Contract,
    pub document_type: &'a DocumentType,
    pub internal_clauses: InternalClauses,
    pub offset: u16,
    pub limit: u16,
    pub order_by: IndexMap<String, OrderClause>,
    pub start_at: Option<Vec<u8>>,
    pub start_at_included: bool,
    pub block_time: Option<f64>,
}

impl<'a> DriveQuery<'a> {
    pub fn is_for_primary_key(&self) -> bool {
        self.internal_clauses.is_for_primary_key()
            || (self.internal_clauses.is_empty()
                && (self.order_by.is_empty()
                    || (self.order_by.len() == 1
                        && self
                            .order_by
                            .keys()
                            .collect::<Vec<&String>>()
                            .first()
                            .unwrap()
                            .as_str()
                            == "$id")))
    }

    pub fn from_cbor(
        query_cbor: &[u8],
        contract: &'a Contract,
        document_type: &'a DocumentType,
    ) -> Result<Self, Error> {
        let mut query_document: BTreeMap<String, Value> = ciborium::de::from_reader(query_cbor)
            .map_err(|_| Error::Structure(StructureError::InvalidCBOR("unable to decode query")))?;

        let limit: u16 = query_document
            .remove("limit")
            .map_or(Some(defaults::DEFAULT_QUERY_LIMIT), |id_cbor| {
                if let Value::Integer(b) = id_cbor {
                    let reduced = i128::from(b) as u64;
                    if reduced == 0 || reduced > (defaults::DEFAULT_QUERY_LIMIT as u64) {
                        None
                    } else {
                        Some(reduced as u16)
                    }
                } else {
                    None
                }
            })
            .ok_or(Error::Query(QueryError::InvalidLimit(
                "limit should be a integer from 1 to 100",
            )))?;

        let block_time: Option<f64> = query_document.remove("blockTime").and_then(|id_cbor| {
            if let Value::Float(b) = id_cbor {
                Some(b)
            } else if let Value::Integer(b) = id_cbor {
                Some(i128::from(b) as f64)
            } else {
                None
            }
        });

        let all_where_clauses: Vec<WhereClause> =
            query_document
                .remove("where")
                .map_or(Ok(vec![]), |id_cbor| {
                    if let Value::Array(clauses) = id_cbor {
                        clauses
                            .iter()
                            .map(|where_clause| {
                                if let Value::Array(clauses_components) = where_clause {
                                    WhereClause::from_components(clauses_components)
                                } else {
                                    Err(Error::Query(QueryError::InvalidFormatWhereClause(
                                        "where clause must be an array",
                                    )))
                                }
                            })
                            .collect::<Result<Vec<WhereClause>, Error>>()
                    } else {
                        Err(Error::Query(QueryError::InvalidFormatWhereClause(
                            "where clause must be an array",
                        )))
                    }
                })?;

        let internal_clauses = InternalClauses::extract_from_clauses(all_where_clauses)?;

        let start_at_option = query_document.remove("startAt");
        let start_after_option = query_document.remove("startAfter");
        if start_after_option.is_some() && start_at_option.is_some() {
            return Err(Error::Query(QueryError::DuplicateStartConditions(
                "only one of startAt or startAfter should be provided",
            )));
        }

        let mut start_at_included = true;

        let mut start_option: Option<Value> = None;

        if start_after_option.is_some() {
            start_option = start_after_option;
            start_at_included = false;
        } else if start_at_option.is_some() {
            start_option = start_at_option;
            start_at_included = true;
        }

        let start_at: Option<Vec<u8>> = if start_option.is_some() {
            bytes_for_system_value(&start_option.unwrap())?
        } else {
            None
        };

        let order_by: IndexMap<String, OrderClause> = query_document
            .remove("orderBy")
            .map_or(vec![], |id_cbor| {
                if let Value::Array(clauses) = id_cbor {
                    clauses
                        .iter()
                        .filter_map(|order_clause| {
                            if let Value::Array(clauses_components) = order_clause {
                                OrderClause::from_components(clauses_components).ok()
                            } else {
                                None
                            }
                        })
                        .collect()
                } else {
                    vec![]
                }
            })
            .iter()
            .map(|order_clause| Ok((order_clause.field.clone(), order_clause.to_owned())))
            .collect::<Result<IndexMap<String, OrderClause>, Error>>()?;

        if !query_document.is_empty() {
            return Err(Error::Query(QueryError::Unsupported(
                "unsupported syntax in where clause",
            )));
        }

        Ok(DriveQuery {
            contract,
            document_type,
            internal_clauses,
            offset: 0,
            limit,
            order_by,
            start_at,
            start_at_included,
            block_time,
        })
    }

    pub fn from_sql_expr(sql_string: &str, contract: &'a Contract) -> Result<Self, Error> {
        let dialect: GenericDialect = sqlparser::dialect::GenericDialect {};
        let statements: Vec<Statement> = Parser::parse_sql(&dialect, sql_string)
            .map_err(|_| Error::Query(QueryError::InvalidSQL("Issue parsing sql")))?;

        // Should ideally iterate over each statement
        let first_statement = statements
            .get(0)
            .ok_or(Error::Query(QueryError::InvalidSQL("Issue parsing sql")))?;

        let query: &ast::Query = match first_statement {
            ast::Statement::Query(query_struct) => Some(query_struct),
            _ => None,
        }
        .ok_or(Error::Query(QueryError::InvalidSQL("Issue parsing sql")))?;

        let limit: u16 = if let Some(limit_expr) = &query.limit {
            match limit_expr {
                ast::Expr::Value(Number(num_string, _)) => {
                    let cast_num_string: &String = num_string;
                    cast_num_string.parse::<u16>().ok()
                }
                _ => None,
            }
            .ok_or(Error::Query(QueryError::InvalidLimit(
                "Issue parsing sql: invalid limit value",
            )))?
        } else {
            defaults::DEFAULT_QUERY_LIMIT
        };

        let order_by: IndexMap<String, OrderClause> = query
            .order_by
            .iter()
            .map(|order_exp: &OrderByExpr| {
                let ascending = order_exp.asc.is_none() || order_exp.asc.unwrap();
                let field = order_exp.expr.to_string();
                (field.clone(), OrderClause { field, ascending })
            })
            .collect::<IndexMap<String, OrderClause>>();

        // Grab the select section of the query
        let select: &Select = match &query.body {
            ast::SetExpr::Select(select) => Some(select),
            _ => None,
        }
        .ok_or(Error::Query(QueryError::InvalidSQL("Issue parsing sql")))?;

        // Get the document type from the 'from' section
        let document_type_name = match &select
            .from
            .get(0)
            .ok_or(Error::Query(QueryError::InvalidSQL(
                "Invalid query: missing from section",
            )))?
            .relation
        {
            Table {
                name,
                alias: _,
                args: _,
                with_hints: _,
            } => name.0.get(0).as_ref().map(|identifier| &identifier.value),
            _ => None,
        }
        .ok_or(Error::Query(QueryError::InvalidSQL(
            "Issue parsing sql: invalid from value",
        )))?;

        let document_type = contract
            .document_types
            .get(document_type_name)
            .ok_or(Error::Query(QueryError::DocumentTypeNotFound(
                "document type not found in contract",
            )))?;

        // Restrictions
        // only binary where clauses are supported
        // i.e. [<fieldname>, <operator>, <value>]
        // [and] is used to separate where clauses
        // currently where clauses are either binary operations or list descriptions (in clauses)
        // hence once [and] is encountered [left] and [right] must be only one of the above
        // i.e other where clauses
        // e.g. firstname = wisdom and lastname = ogwu
        // if op is not [and] then [left] or [right] must not be a binary operation or list description
        let mut all_where_clauses: Vec<WhereClause> = Vec::new();
        let selection_tree = select.selection.as_ref();

        // Where clauses are optional
        if let Some(selection_tree) = selection_tree {
            WhereClause::build_where_clauses_from_operations(
                selection_tree,
                &mut all_where_clauses,
            )?;
        }

        let internal_clauses = InternalClauses::extract_from_clauses(all_where_clauses)?;

        let start_at_option = None;
        let start_at_included = true;
        let start_at: Option<Vec<u8>> = if start_at_option.is_some() {
            bytes_for_system_value(start_at_option.unwrap())?
        } else {
            None
        };

        Ok(DriveQuery {
            contract,
            document_type,
            internal_clauses,
            offset: 0,
            limit,
            order_by,
            start_at,
            start_at_included,
            block_time: None,
        })
    }

    pub fn construct_path_query_operations(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<PathQuery, Error> {
        // First we should get the overall document_type_path
        let document_type_path = self
            .contract
            .document_type_path(self.document_type.name.as_str())
            .into_iter()
            .map(|a| a.to_vec())
            .collect::<Vec<Vec<u8>>>();

        let starts_at_document: Option<(Document, bool)> = match &self.start_at {
            None => Ok(None),
            Some(starts_at) => {
                // First if we have a startAt or or startsAfter we must get the element
                // from the backing store

                let (start_at_document_path, start_at_document_key) =
                    if self.document_type.documents_keep_history {
                        let document_holding_path =
                            self.contract.documents_with_history_primary_key_path(
                                self.document_type.name.as_str(),
                                starts_at,
                            );
                        (Vec::from(document_holding_path), vec![0])
                    } else {
                        let document_holding_path = self
                            .contract
                            .documents_primary_key_path(self.document_type.name.as_str());
                        (
                            Vec::from(document_holding_path.as_slice()),
                            starts_at.clone(),
                        )
                    };

                let start_at_document = drive
                    .grove_get(
                        start_at_document_path,
                        KeyValueInfo::KeyRefRequest(&start_at_document_key),
                        transaction,
                        drive_operations,
                    )
                    .map_err(|e| match e {
                        Error::GroveDB(GroveError::PathKeyNotFound(_))
                        | Error::GroveDB(GroveError::PathNotFound(_)) => {
                            let error_message = if self.start_at_included {
                                "startAt document not found"
                            } else {
                                "startAfter document not found"
                            };

                            Error::Query(QueryError::StartDocumentNotFound(error_message))
                        }
                        _ => e,
                    })?
                    .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                        "expected a value",
                    )))?;

                if let Element::Item(item, _) = start_at_document {
                    let document = Document::from_cbor(item.as_slice(), None, None)?;
                    Ok(Some((document, self.start_at_included)))
                } else {
                    Err(Error::Drive(DriveError::CorruptedDocumentPath(
                        "Holding paths should only have items",
                    )))
                }
            }
        }?;
        if self.is_for_primary_key() {
            self.get_primary_key_path_query(document_type_path, starts_at_document)
        } else {
            self.get_non_primary_key_path_query(document_type_path, starts_at_document)
        }
    }

    pub fn get_primary_key_path_query(
        &self,
        document_type_path: Vec<Vec<u8>>,
        starts_at_document: Option<(Document, bool)>,
    ) -> Result<PathQuery, Error> {
        let mut path = document_type_path;

        // Add primary key ($id) subtree
        path.push(vec![0]);

        if let Some(primary_key_equal_clause) = &self.internal_clauses.primary_key_equal_clause {
            let mut query = Query::new();
            let key = self
                .document_type
                .serialize_value_for_key("$id", &primary_key_equal_clause.value)?;
            query.insert_key(key);

            if self.document_type.documents_keep_history {
                // if the documents keep history then we should insert a subquery
                if let Some(block_time) = self.block_time {
                    let encoded_block_time = crate::contract::types::encode_float(block_time)?;
                    let mut sub_query = Query::new_with_direction(false);
                    sub_query.insert_range_to_inclusive(..=encoded_block_time);
                    query.set_subquery(sub_query);
                } else {
                    query.set_subquery_key(vec![0]);
                }
            }

            Ok(PathQuery::new(path, SizedQuery::new(query, Some(1), None)))
        } else {
            // This is for a range
            let left_to_right = if self.order_by.keys().len() == 1 {
                if self.order_by.keys().next().unwrap() != "$id" {
                    return Err(Error::Query(QueryError::InvalidOrderByProperties(
                        "order by should include $id only",
                    )));
                }

                let order_clause = self.order_by.get("$id").unwrap();

                order_clause.ascending
            } else {
                true
            };

            let mut query = Query::new_with_direction(left_to_right);
            // If there is a start_at_document, we need to get the value that it has for the
            // current field.
            let starts_at_key_option = match starts_at_document {
                None => None,
                Some((document, included)) => {
                    // if the key doesn't exist then we should ignore the starts at key
                    document
                        .get_raw_for_document_type("$id", self.document_type, None)?
                        .map(|raw_value_option| (raw_value_option, included))
                }
            };

            if let Some(primary_key_in_clause) = &self.internal_clauses.primary_key_in_clause {
                let in_values = primary_key_in_clause.in_values()?;

                match starts_at_key_option {
                    None => {
                        for value in in_values.iter() {
                            let key = self.document_type.serialize_value_for_key("$id", value)?;
                            query.insert_key(key)
                        }
                    }
                    Some((starts_at_key, included)) => {
                        for value in in_values.iter() {
                            let key = self.document_type.serialize_value_for_key("$id", value)?;

                            if (left_to_right && starts_at_key < key)
                                || (!left_to_right && starts_at_key > key)
                                || (included && starts_at_key == key)
                            {
                                query.insert_key(key);
                            }
                        }
                    }
                }

                if self.document_type.documents_keep_history {
                    // if the documents keep history then we should insert a subquery
                    if let Some(_block_time) = self.block_time {
                        //todo
                        return Err(Error::Query(QueryError::Unsupported("Not yet implemented")));
                        // in order to be able to do this we would need limited subqueries
                        // as we only want the first element before the block_time

                        // let encoded_block_time = encode_float(block_time)?;
                        // let mut sub_query = Query::new_with_direction(false);
                        // sub_query.insert_range_to_inclusive(..=encoded_block_time);
                        // query.set_subquery(sub_query);
                    } else {
                        query.set_subquery_key(vec![0]);
                    }
                }

                Ok(PathQuery::new(
                    path,
                    SizedQuery::new(query, Some(self.limit), Some(self.offset)),
                ))
            } else {
                // this is a range on all elements
                match starts_at_key_option {
                    None => {
                        query.insert_all();
                    }
                    Some((starts_at_key, included)) => match left_to_right {
                        true => match included {
                            true => query.insert_range_from(starts_at_key..),
                            false => query.insert_range_after(starts_at_key..),
                        },
                        false => match included {
                            true => query.insert_range_to_inclusive(..=starts_at_key),
                            false => query.insert_range_to(..starts_at_key),
                        },
                    },
                }

                if self.document_type.documents_keep_history {
                    // if the documents keep history then we should insert a subquery
                    if let Some(_block_time) = self.block_time {
                        return Err(Error::Query(QueryError::Unsupported("Not yet implemented")));
                        // in order to be able to do this we would need limited subqueries
                        // as we only want the first element before the block_time

                        // let encoded_block_time = encode_float(block_time)?;
                        // let mut sub_query = Query::new_with_direction(false);
                        // sub_query.insert_range_to_inclusive(..=encoded_block_time);
                        // query.set_subquery(sub_query);
                    } else {
                        query.set_subquery_key(vec![0]);
                    }
                }

                Ok(PathQuery::new(
                    path,
                    SizedQuery::new(query, Some(self.limit), Some(self.offset)),
                ))
            }
        }
    }

    pub fn find_best_index(&self) -> Result<&Index, Error> {
        let equal_fields = self
            .internal_clauses
            .equal_clauses
            .keys()
            .map(|s| s.as_str())
            .collect::<Vec<&str>>();
        let in_field = self
            .internal_clauses
            .in_clause
            .as_ref()
            .map(|in_clause| in_clause.field.as_str());
        let range_field = self
            .internal_clauses
            .range_clause
            .as_ref()
            .map(|range_clause| range_clause.field.as_str());
        let mut fields = equal_fields;
        if let Some(range_field) = range_field {
            fields.push(range_field);
        }
        if let Some(in_field) = in_field {
            fields.push(in_field);
            //if there is an in_field, it always takes precedence
        }

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

        let (index, difference) = self
            .document_type
            .index_for_types(fields.as_slice(), in_field, order_by_keys.as_slice())
            .ok_or(Error::Query(QueryError::WhereClauseOnNonIndexedProperty(
                "query must be for valid indexes",
            )))?;
        if difference > defaults::MAX_INDEX_DIFFERENCE {
            return Err(Error::Query(QueryError::QueryTooFarFromIndex(
                "query must better match an existing index",
            )));
        }
        Ok(index)
    }

    fn query_item_for_starts_at_key(starts_at_key: Vec<u8>, left_to_right: bool) -> QueryItem {
        if left_to_right {
            QueryItem::RangeAfter(starts_at_key..)
        } else {
            QueryItem::RangeTo(..starts_at_key)
        }
    }

    fn inner_query_from_starts_at_for_id(
        starts_at_document: &Option<(Document, &DocumentType, &IndexProperty, bool)>,
        left_to_right: bool,
    ) -> Query {
        // We only need items after the start at document
        let mut inner_query = Query::new_with_direction(left_to_right);

        if let Some((document, _, _, included)) = starts_at_document {
            let start_at_key = document.id.to_vec();
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

    fn inner_query_starts_from_key(
        start_at_key: Vec<u8>,
        left_to_right: bool,
        included: bool,
    ) -> Query {
        // We only need items after the start at document
        let mut inner_query = Query::new_with_direction(left_to_right);
        if left_to_right {
            if included {
                inner_query.insert_range_from(start_at_key..);
            } else {
                inner_query.insert_range_after(start_at_key..);
            }
        } else {
            if included {
                inner_query.insert_range_to_inclusive(..=start_at_key);
            } else {
                inner_query.insert_range_to(..start_at_key);
            }
        }
        inner_query
    }

    // We are passing in starts_at_document 4 parameters
    // The document
    // The document type (borrowed)
    // The index property (borrowed)
    // if the element itself should be included. ie StartAt vs StartAfter
    fn inner_query_from_starts_at(
        starts_at_document: &Option<(Document, &DocumentType, &IndexProperty, bool)>,
        left_to_right: bool,
    ) -> Result<Query, Error> {
        let mut inner_query = Query::new_with_direction(left_to_right);
        if let Some((document, document_type, indexed_property, included)) = starts_at_document {
            // We only need items after the start at document
            let start_at_key = document.get_raw_for_document_type(
                indexed_property.name.as_str(),
                &document_type,
                None,
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

    fn recursive_insert_on_query(
        query: Option<&mut Query>,
        left_over_index_properties: &[&IndexProperty],
        unique: bool,
        starts_at_document: &Option<(Document, &DocumentType, &IndexProperty, bool)>, //for key level, included
        default_left_to_right: bool,
        order_by: Option<&IndexMap<String, OrderClause>>,
    ) -> Result<Option<Query>, Error> {
        match left_over_index_properties.split_first() {
            None => {
                if let Some(query) = query {
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
                                Some(vec![0]),
                                Some(inner_query),
                            );
                        }
                        false => {
                            query.set_subquery_key(vec![0]);
                            // we just get all by document id order ascending
                            let full_query = Self::inner_query_from_starts_at_for_id(
                                &None,
                                default_left_to_right,
                            );
                            query.set_subquery(full_query);

                            let inner_query = Self::inner_query_from_starts_at_for_id(
                                starts_at_document,
                                default_left_to_right,
                            );

                            query.add_conditional_subquery(
                                QueryItem::Key(b"".to_vec()),
                                Some(vec![0]),
                                Some(inner_query),
                            );
                        }
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

                match query {
                    None => {
                        let mut inner_query =
                            Self::inner_query_from_starts_at(starts_at_document, left_to_right)?;
                        DriveQuery::recursive_insert_on_query(
                            Some(&mut inner_query),
                            left_over,
                            unique,
                            starts_at_document,
                            left_to_right,
                            order_by,
                        )?;
                        Ok(Some(inner_query))
                    }
                    Some(query) => {
                        if let Some((document, document_type, indexed_property, included)) =
                            starts_at_document
                        {
                            let start_at_key = document
                                .get_raw_for_document_type(first.name.as_str(), document_type, None)
                                .ok()
                                .flatten()
                                .unwrap_or_default();

                            // We should always include if we have left_over
                            let non_conditional_included = !left_over.is_empty() | *included;

                            let mut non_conditional_query = Self::inner_query_starts_from_key(
                                start_at_key,
                                left_to_right,
                                non_conditional_included,
                            );

                            DriveQuery::recursive_insert_on_query(
                                Some(&mut non_conditional_query),
                                left_over,
                                unique,
                                starts_at_document,
                                left_to_right,
                                order_by,
                            )?;

                            query.set_subquery(non_conditional_query);
                        } else {
                            let mut inner_query = Query::new_with_direction(first.ascending);
                            inner_query.insert_all();
                            DriveQuery::recursive_insert_on_query(
                                Some(&mut inner_query),
                                left_over,
                                unique,
                                starts_at_document,
                                left_to_right,
                                order_by,
                            )?;
                            query.set_subquery(inner_query);
                        }
                        query.set_subquery_key(first.name.as_bytes().to_vec());
                        Ok(None)
                    }
                }
            }
        }
    }

    pub fn get_non_primary_key_path_query(
        &self,
        document_type_path: Vec<Vec<u8>>,
        starts_at_document: Option<(Document, bool)>,
    ) -> Result<PathQuery, Error> {
        let index = self.find_best_index()?;
        let ordered_clauses: Vec<&WhereClause> = index
            .properties
            .iter()
            .filter_map(|field| self.internal_clauses.equal_clauses.get(field.name.as_str()))
            .collect();
        let (last_clause, last_clause_is_range, subquery_clause) =
            match &self.internal_clauses.in_clause {
                None => match &self.internal_clauses.range_clause {
                    None => (ordered_clauses.last().copied(), false, None),
                    Some(where_clause) => (Some(where_clause), true, None),
                },
                Some(in_clause) => match &self.internal_clauses.range_clause {
                    None => (Some(in_clause), true, None),
                    Some(range_clause) => (Some(in_clause), true, Some(range_clause)),
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
                    .get(field.name.as_str())
                    .is_some()
                    || (last_clause.is_some() && last_clause.unwrap().field == field.name)
                    || (subquery_clause.is_some() && subquery_clause.unwrap().field == field.name))
            })
            .collect::<Vec<&IndexProperty>>();

        let intermediate_values =
            index
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
                                ))
                            }
                        }
                    }
                })
                .collect::<Result<Vec<Vec<u8>>, Error>>()?;

        let final_query = match last_clause {
            None => {
                // There is no last_clause which means we are using an index most likely because of an order_by, however we have no
                // clauses, in this case we should use the first value of the index.
                let first_index = index.properties.first().ok_or(Error::Drive(
                    DriveError::CorruptedContractIndexes("index must have properties"),
                ))?; // Index must have properties
                Self::recursive_insert_on_query(
                    None,
                    left_over_index_properties.as_slice(),
                    index.unique,
                    &starts_at_document.map(|(document, included)| {
                        (document, self.document_type, first_index, included)
                    }),
                    first_index.ascending,
                    None,
                )?
                .expect("Index must have left over properties if no last clause")
            }
            Some(where_clause) => {
                let left_to_right = if where_clause.operator.is_range() {
                    let order_clause: &OrderClause = self
                        .order_by
                        .get(where_clause.field.as_str())
                        .ok_or(Error::Query(QueryError::MissingOrderByForRange(
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
                )?;

                match subquery_clause {
                    None => {
                        // There is a last_clause, but no subquery_clause, we should use the index property of the last clause
                        // We need to get the terminal indexes unused by clauses.
                        let last_index_property = index
                            .properties
                            .iter()
                            .find(|field| where_clause.field == field.name)
                            .ok_or(Error::Drive(DriveError::CorruptedContractIndexes(
                                "index must have last_clause field",
                            )))?;
                        Self::recursive_insert_on_query(
                            Some(&mut query),
                            left_over_index_properties.as_slice(),
                            index.unique,
                            &starts_at_document.map(|(document, included)| {
                                (document, self.document_type, last_index_property, included)
                            }),
                            left_to_right,
                            Some(&self.order_by),
                        )?;
                    }
                    Some(subquery_where_clause) => {
                        let order_clause: &OrderClause = self
                            .order_by
                            .get(subquery_where_clause.field.as_str())
                            .ok_or(Error::Query(QueryError::MissingOrderByForRange(
                                "query must have an orderBy field for each range element",
                            )))?;
                        let mut subquery = subquery_where_clause.to_path_query(
                            self.document_type,
                            &starts_at_document,
                            order_clause.ascending,
                        )?;
                        let last_index_property = index
                            .properties
                            .iter()
                            .find(|field| subquery_where_clause.field == field.name)
                            .ok_or(Error::Drive(DriveError::CorruptedContractIndexes(
                                "index must have subquery_clause field",
                            )))?;
                        Self::recursive_insert_on_query(
                            Some(&mut subquery),
                            left_over_index_properties.as_slice(),
                            index.unique,
                            &starts_at_document.map(|(document, included)| {
                                (document, self.document_type, last_index_property, included)
                            }),
                            left_to_right,
                            Some(&self.order_by),
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
            QueryError::QueryOnDocumentTypeWithNoIndexes("document query has no index with fields"),
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
            SizedQuery::new(final_query, Some(self.limit), Some(self.offset)),
        ))
    }

    pub fn execute_with_proof(
        self,
        drive: &Drive,
        transaction: TransactionArg,
    ) -> Result<(Vec<u8>, u64), Error> {
        let mut drive_operations = vec![];
        let items = self.execute_with_proof_internal(drive, transaction, &mut drive_operations)?;
        let (_, cost) = calculate_fee(None, Some(drive_operations))?;
        Ok((items, cost))
    }

    pub(crate) fn execute_with_proof_internal(
        self,
        drive: &Drive,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<Vec<u8>, Error> {
        let path_query =
            self.construct_path_query_operations(drive, transaction, drive_operations)?;
        drive.grove_get_proved_path_query(&path_query, transaction, drive_operations)
    }

    pub fn execute_with_proof_only_get_elements(
        self,
        drive: &Drive,
        transaction: TransactionArg,
    ) -> Result<([u8; 32], Vec<Vec<u8>>, u64), Error> {
        let mut drive_operations = vec![];
        let (root_hash, items) = self.execute_with_proof_only_get_elements_internal(
            drive,
            transaction,
            &mut drive_operations,
        )?;
        let (_, cost) = calculate_fee(None, Some(drive_operations))?;
        Ok((root_hash, items, cost))
    }

    pub(crate) fn execute_with_proof_only_get_elements_internal(
        self,
        drive: &Drive,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<([u8; 32], Vec<Vec<u8>>), Error> {
        let path_query =
            self.construct_path_query_operations(drive, transaction, drive_operations)?;

        let proof =
            drive.grove_get_proved_path_query(&path_query, transaction, drive_operations)?;
        let (root_hash, mut key_value_elements) =
            GroveDb::verify_query(proof.as_slice(), &path_query).map_err(Error::GroveDB)?;

        let mut values = vec![];
        for (_, value) in key_value_elements.iter_mut() {
            let element = Element::deserialize(&value).unwrap();
            match element {
                Element::Item(val, _) => values.push(val),
                Element::Tree(..) | Element::Reference(..) => {
                    return Err(Error::GroveDB(GroveError::InvalidQuery(
                        "path query should only point to items: got trees",
                    )));
                }
            }
        }

        Ok((root_hash, values))
    }

    pub fn execute_no_proof(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
    ) -> Result<(Vec<Vec<u8>>, u16, u64), Error> {
        let mut drive_operations = vec![];
        let (items, skipped) =
            self.execute_no_proof_internal(drive, transaction, &mut drive_operations)?;
        let (_, cost) = calculate_fee(None, Some(drive_operations))?;
        Ok((items, skipped, cost))
    }

    pub(crate) fn execute_no_proof_internal(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(Vec<Vec<u8>>, u16), Error> {
        let path_query =
            self.construct_path_query_operations(drive, transaction, drive_operations)?;
        let query_result = drive.grove_get_path_query(&path_query, transaction, drive_operations);
        match query_result {
            Err(GroveDB(GroveError::PathKeyNotFound(_)))
            | Err(GroveDB(GroveError::PathNotFound(_))) => Ok((Vec::new(), 0)),
            _ => {
                let (data, skipped) = query_result?;
                {
                    Ok((data, skipped))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use std::option::Option::None;
    use tempfile::TempDir;

    use crate::common;
    use crate::common::json_document_to_cbor;
    use crate::contract::{Contract, DocumentType};
    use crate::drive::flags::StorageFlags;
    use crate::drive::Drive;
    use crate::query::DriveQuery;
    use serde_json::Value::Null;

    fn setup_family_contract() -> (Drive, Contract) {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        drive
            .create_root_tree(None)
            .expect("expected to create root tree successfully");

        let contract_path = "tests/supporting_files/contract/family/family-contract.json";

        // let's construct the grovedb structure for the dashpay data contract
        let contract_cbor = json_document_to_cbor(contract_path, Some(1));
        let contract = Contract::from_cbor(&contract_cbor, None)
            .expect("expected to deserialize the contract");
        let storage_flags = StorageFlags { epoch: 0 };
        drive
            .apply_contract(
                &contract,
                contract_cbor.clone(),
                0f64,
                true,
                storage_flags,
                None,
            )
            .expect("expected to apply contract successfully");

        (drive, contract)
    }

    fn setup_family_birthday_contract() -> (Drive, Contract) {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        drive
            .create_root_tree(None)
            .expect("expected to create root tree successfully");

        let contract_path =
            "tests/supporting_files/contract/family/family-contract-with-birthday.json";

        // let's construct the grovedb structure for the dashpay data contract
        let contract_cbor = json_document_to_cbor(contract_path, Some(1));
        let contract = Contract::from_cbor(&contract_cbor, None)
            .expect("expected to deserialize the contract");
        let storage_flags = StorageFlags { epoch: 0 };
        drive
            .apply_contract(
                &contract,
                contract_cbor.clone(),
                0f64,
                true,
                storage_flags,
                None,
            )
            .expect("expected to apply contract successfully");

        (drive, contract)
    }

    #[test]
    fn test_invalid_query_ranges_different_fields() {
        let query_value = json!({
            "where": [
                ["firstName", "<", "Gilligan"],
                ["lastName", "<", "Michelle"],
            ],
            "limit": 100,
            "orderBy": [
                ["firstName", "asc"],
                ["lastName", "asc"],
            ]
        });
        let contract = Contract::default();
        let document_type = DocumentType::default();

        let where_cbor = common::value_to_cbor(query_value, None);
        DriveQuery::from_cbor(where_cbor.as_slice(), &contract, &document_type)
            .expect_err("all ranges must be on same field");
    }

    #[test]
    fn test_invalid_query_extra_invalid_field() {
        let query_value = json!({
            "where": [
                ["firstName", "<", "Gilligan"],
            ],
            "limit": 100,
            "orderBy": [
                ["firstName", "asc"],
                ["lastName", "asc"],
            ],
            "invalid": 0,
        });
        let contract = Contract::default();
        let document_type = DocumentType::default();

        let where_cbor = common::value_to_cbor(query_value, None);
        DriveQuery::from_cbor(where_cbor.as_slice(), &contract, &document_type).expect_err(
            "fields of queries must of defined supported types (where, limit, orderBy...)",
        );
    }

    #[test]
    fn test_invalid_query_conflicting_clauses() {
        let query_value = json!({
            "where": [
                ["firstName", "<", "Gilligan"],
                ["firstName", ">", "Gilligan"],
            ],
            "limit": 100,
            "orderBy": [
                ["firstName", "asc"],
                ["lastName", "asc"],
            ],
        });

        let contract = Contract::default();
        let document_type = DocumentType::default();

        let where_cbor = common::value_to_cbor(query_value, None);
        DriveQuery::from_cbor(where_cbor.as_slice(), &contract, &document_type)
            .expect_err("the query should not be created");
    }

    #[test]
    fn test_valid_query_groupable_meeting_clauses() {
        let query_value = json!({
            "where": [
                ["firstName", "<=", "Gilligan"],
                ["firstName", ">", "Gilligan"],
            ],
            "limit": 100,
            "orderBy": [
                ["firstName", "asc"],
                ["lastName", "asc"],
            ],
        });

        let contract = Contract::default();
        let document_type = DocumentType::default();

        let where_cbor = common::value_to_cbor(query_value, None);
        DriveQuery::from_cbor(where_cbor.as_slice(), &contract, &document_type)
            .expect("the query should be created");
    }

    #[test]
    fn test_valid_query_query_field_at_max_length() {
        let long_string = "t".repeat(255);
        let query_value = json!({
            "where": [
                ["firstName", "<", long_string],
            ],
            "limit": 100,
            "orderBy": [
                ["firstName", "asc"],
                ["lastName", "asc"],
            ],
        });
        let contract = Contract::default();
        let document_type = DocumentType::default();

        let where_cbor = common::value_to_cbor(query_value, None);
        DriveQuery::from_cbor(where_cbor.as_slice(), &contract, &document_type)
            .expect("query should be fine for a 255 byte long string");
    }

    #[test]
    fn test_invalid_query_field_too_long() {
        let (drive, contract) = setup_family_contract();

        let document_type = contract
            .document_type_for_name("person")
            .expect("expected to get a document type");

        let too_long_string = "t".repeat(256);
        let query_value = json!({
            "where": [
                ["firstName", "<", too_long_string],
            ],
            "limit": 100,
            "orderBy": [
                ["firstName", "asc"],
                ["lastName", "asc"],
            ],
        });

        let where_cbor = common::value_to_cbor(query_value, None);
        let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, &document_type)
            .expect("fields of queries length must be under 256 bytes long");
        query
            .execute_no_proof(&drive, None)
            .expect_err("fields of queries length must be under 256 bytes long");
    }

    // TODO: Eventually we want to error with weird Null values
    // #[test]
    // fn test_invalid_query_scalar_field_with_null_value() {
    //     let (drive, contract) = setup_family_contract();
    //
    //     let document_type = contract
    //         .document_type_for_name("person")
    //         .expect("expected to get a document type");
    //
    //     let query_value = json!({
    //         "where": [
    //             ["age", "<", Null],
    //         ],
    //         "limit": 100,
    //         "orderBy": [
    //             ["age", "asc"],
    //         ],
    //     });
    //
    //     let where_cbor = common::value_to_cbor(query_value, None);
    //     let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, &document_type)
    //         .expect("The query itself should be valid for a null type");
    //     query
    //         .execute_no_proof(&drive, None)
    //         .expect_err("a Null value doesn't make sense for an integer");
    // }

    // TODO: Eventually we want to error with weird Null values
    //
    // #[test]
    // fn test_invalid_query_timestamp_field_with_null_value() {
    //     let (drive, contract) = setup_family_birthday_contract();
    //
    //     let document_type = contract
    //         .document_type_for_name("person")
    //         .expect("expected to get a document type");
    //
    //     let query_value = json!({
    //         "where": [
    //             ["birthday", "<", Null],
    //         ],
    //         "limit": 100,
    //         "orderBy": [
    //             ["birthday", "asc"],
    //         ],
    //     });
    //
    //     let where_cbor = common::value_to_cbor(query_value, None);
    //     let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, &document_type)
    //         .expect("The query itself should be valid for a null type");
    //     query
    //         .execute_no_proof(&drive, None)
    //         .expect_err("the value can not be less than Null");
    // }

    #[test]
    fn test_valid_query_timestamp_field_with_null_value() {
        let (drive, contract) = setup_family_birthday_contract();

        let document_type = contract
            .document_type_for_name("person")
            .expect("expected to get a document type");

        let query_value = json!({
            "where": [
                ["birthday", ">=", Null],
            ],
            "limit": 100,
            "orderBy": [
                ["birthday", "asc"],
            ],
        });

        let where_cbor = common::value_to_cbor(query_value, None);
        let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, &document_type)
            .expect("The query itself should be valid for a null type");
        query
            .execute_no_proof(&drive, None)
            .expect("a Null value doesn't make sense for a float");
    }

    #[test]
    fn test_invalid_query_in_with_empty_array() {
        let (drive, contract) = setup_family_contract();

        let document_type = contract
            .document_type_for_name("person")
            .expect("expected to get a document type");

        let query_value = json!({
            "where": [
                ["firstName", "in", []],
            ],
            "limit": 100,
            "orderBy": [
                ["firstName", "asc"],
            ],
        });

        let where_cbor = common::value_to_cbor(query_value, None);
        let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, &document_type)
            .expect("query should be valid for empty array");

        query
            .execute_no_proof(&drive, None)
            .expect_err("query should not be able to execute for empty array");
    }

    #[test]
    fn test_invalid_query_in_too_many_elements() {
        let (drive, contract) = setup_family_contract();

        let document_type = contract
            .document_type_for_name("person")
            .expect("expected to get a document type");

        let mut array: Vec<String> = Vec::with_capacity(101);
        for _ in 0..array.capacity() {
            array.push(String::from("a"));
        }
        let query_value = json!({
            "where": [
                ["firstName", "in", array],
            ],
            "limit": 100,
            "orderBy": [
                ["firstName", "asc"],
            ],
        });

        let where_cbor = common::value_to_cbor(query_value, None);
        let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, &document_type)
            .expect("query is valid for too many elements");

        query
            .execute_no_proof(&drive, None)
            .expect_err("query should not be able to execute with too many elements");
    }

    #[test]
    fn test_invalid_query_in_unique_elements() {
        let (drive, contract) = setup_family_contract();

        let document_type = contract
            .document_type_for_name("person")
            .expect("expected to get a document type");

        let query_value = json!({
            "where": [
                ["firstName", "in", ["a", "a"]],
            ],
            "limit": 100,
            "orderBy": [
                ["firstName", "asc"],
            ],
        });

        let where_cbor = common::value_to_cbor(query_value, None);

        // The is actually valid, however executing it is not
        // This is in order to optimize query execution

        let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, &document_type)
            .expect("the query should be created");

        query
            .execute_no_proof(&drive, None)
            .expect_err("there should be no duplicates values for In query");
    }

    #[test]
    fn test_invalid_query_starts_with_empty_string() {
        let query_value = json!({
            "where": [
                ["firstName", "startsWith", ""],
            ],
            "limit": 100,
            "orderBy": [
                ["firstName", "asc"],
            ],
        });

        let contract = Contract::default();
        let document_type = DocumentType::default();

        let where_cbor = common::value_to_cbor(query_value, None);
        DriveQuery::from_cbor(where_cbor.as_slice(), &contract, &document_type)
            .expect_err("starts with can not start with an empty string");
    }

    #[test]
    fn test_invalid_query_limit_too_high() {
        let query_value = json!({
            "where": [
                ["firstName", "startsWith", "a"],
            ],
            "limit": 101,
            "orderBy": [
                ["firstName", "asc"],
            ],
        });

        let contract = Contract::default();
        let document_type = DocumentType::default();

        let where_cbor = common::value_to_cbor(query_value, None);
        DriveQuery::from_cbor(where_cbor.as_slice(), &contract, &document_type)
            .expect_err("starts with can not start with an empty string");
    }

    #[test]
    fn test_invalid_query_limit_too_low() {
        let query_value = json!({
            "where": [
                ["firstName", "startsWith", "a"],
            ],
            "limit": -1,
            "orderBy": [
                ["firstName", "asc"],
            ],
        });

        let contract = Contract::default();
        let document_type = DocumentType::default();

        let where_cbor = common::value_to_cbor(query_value, None);
        DriveQuery::from_cbor(where_cbor.as_slice(), &contract, &document_type)
            .expect_err("starts with can not start with an empty string");
    }

    #[test]
    fn test_invalid_query_limit_zero() {
        let query_value = json!({
            "where": [
                ["firstName", "startsWith", "a"],
            ],
            "limit": 0,
            "orderBy": [
                ["firstName", "asc"],
            ],
        });

        let contract = Contract::default();
        let document_type = DocumentType::default();

        let where_cbor = common::value_to_cbor(query_value, None);
        DriveQuery::from_cbor(where_cbor.as_slice(), &contract, &document_type)
            .expect_err("starts with can not start with an empty string");
    }
}
