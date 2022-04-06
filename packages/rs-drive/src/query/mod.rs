pub mod conditions;
mod defaults;
pub mod ordering;

use crate::contract::{bytes_for_system_value, Contract, Document, DocumentType, IndexProperty};
use ciborium::value::{Value as CborValue, Value};
use conditions::WhereOperator::{Equal, In};
use conditions::{WhereClause, WhereOperator};
use grovedb::{Element, Error, GroveDb, PathQuery, Query, QueryItem, SizedQuery, TransactionArg};
use indexmap::IndexMap;
use ordering::OrderClause;
use sqlparser::ast;
use sqlparser::ast::TableFactor::Table;
use sqlparser::ast::Value::Number;
use sqlparser::ast::{OrderByExpr, Select, Statement};
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;
use std::collections::HashMap;
use std::ops::BitXor;

#[derive(Clone, Debug, PartialEq, Default)]
pub struct InternalClauses {
    primary_key_in_clause: Option<WhereClause>,
    primary_key_equal_clause: Option<WhereClause>,
    in_clause: Option<WhereClause>,
    range_clause: Option<WhereClause>,
    equal_clauses: HashMap<String, WhereClause>,
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
        let query_document: HashMap<String, CborValue> = ciborium::de::from_reader(query_cbor)
            .map_err(|_| Error::InvalidQuery("unable to decode query"))?;

        let limit: u16 = query_document
            .get("limit")
            .map_or(Some(defaults::DEFAULT_QUERY_LIMIT), |id_cbor| {
                if let CborValue::Integer(b) = id_cbor {
                    Some(i128::from(*b) as u16)
                } else {
                    None
                }
            })
            .ok_or(Error::InvalidQuery(
                "limit should be a integer from 1 to 100",
            ))?;

        let block_time: Option<f64> = query_document.get("blockTime").and_then(|id_cbor| {
            if let CborValue::Float(b) = id_cbor {
                Some(*b)
            } else if let CborValue::Integer(b) = id_cbor {
                Some(i128::from(*b) as f64)
            } else {
                None
            }
        });

        let all_where_clauses: Vec<WhereClause> =
            query_document.get("where").map_or(Ok(vec![]), |id_cbor| {
                if let CborValue::Array(clauses) = id_cbor {
                    clauses
                        .iter()
                        .map(|where_clause| {
                            if let CborValue::Array(clauses_components) = where_clause {
                                WhereClause::from_components(clauses_components)
                            } else {
                                Err(Error::InvalidQuery("where clause must be an array"))
                            }
                        })
                        .collect::<Result<Vec<WhereClause>, Error>>()
                } else {
                    Err(Error::InvalidQuery("where clause must be an array"))
                }
            })?;

        let internal_clauses = Self::extract_clauses(all_where_clauses)?;

        let start_at_option = query_document.get("startAt");
        let start_after_option = query_document.get("startAfter");
        if start_after_option.is_some() && start_at_option.is_some() {
            return Err(Error::InvalidQuery(
                "only one of startAt or startAfter should be provided",
            ));
        }

        let mut start_at_included = true;

        let mut start_option: Option<&Value> = None;

        if start_after_option.is_some() {
            start_option = start_after_option;
            start_at_included = false;
        } else if start_at_option.is_some() {
            start_option = start_at_option;
            start_at_included = true;
        }

        let start_at: Option<Vec<u8>> = if start_option.is_some() {
            bytes_for_system_value(start_option.unwrap())?
        } else {
            None
        };

        let order_by: IndexMap<String, OrderClause> = query_document
            .get("orderBy")
            .map_or(vec![], |id_cbor| {
                if let CborValue::Array(clauses) = id_cbor {
                    clauses
                        .iter()
                        .filter_map(|order_clause| {
                            if let CborValue::Array(clauses_components) = order_clause {
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
            .map_err(|_| Error::InvalidQuery("Issue parsing sql"))?;

        // Should ideally iterate over each statement
        let first_statement = statements
            .get(0)
            .ok_or(Error::InvalidQuery("Issue parsing SQL"))?;

        let query: &ast::Query = match first_statement {
            ast::Statement::Query(query_struct) => Some(query_struct),
            _ => None,
        }
        .ok_or(Error::InvalidQuery("Issue parsing sql"))?;

        let limit: u16 = if let Some(limit_expr) = &query.limit {
            match limit_expr {
                ast::Expr::Value(Number(num_string, _)) => {
                    let cast_num_string: &String = num_string;
                    cast_num_string.parse::<u16>().ok()
                }
                _ => None,
            }
            .ok_or(Error::InvalidQuery(
                "Issue parsing sql: invalid limit value",
            ))?
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
        .ok_or(Error::InvalidQuery("Issue parsing sql"))?;

        // Get the document type from the 'from' section
        let document_type_name = match &select
            .from
            .get(0)
            .ok_or(Error::InvalidQuery("Invalid query: missing from section"))?
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
        .ok_or(Error::InvalidQuery("Issue parsing sql: invalid from value"))?;

        let document_type = contract
            .document_types
            .get(document_type_name)
            .ok_or(Error::InvalidQuery("document type not found in contract"))?;

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

        let internal_clauses = Self::extract_clauses(all_where_clauses)?;

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

    fn extract_clauses(all_where_clauses: Vec<WhereClause>) -> Result<InternalClauses, Error> {
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

        let range_clause = WhereClause::group_range_clauses(&all_where_clauses)?;

        let equal_clauses_array =
            all_where_clauses
                .iter()
                .filter_map(|where_clause| match where_clause.operator {
                    Equal => match where_clause.is_identifier() {
                        true => None,
                        false => Some(where_clause.clone()),
                    },
                    _ => None,
                });

        let in_clauses_array = all_where_clauses
            .iter()
            .filter_map(|where_clause| match where_clause.operator {
                WhereOperator::In => match where_clause.is_identifier() {
                    true => None,
                    false => Some(where_clause.clone()),
                },
                _ => None,
            })
            .collect::<Vec<WhereClause>>();

        let primary_key_equal_clause = match primary_key_equal_clauses_array.len() {
            0 => Ok(None),
            1 => Ok(Some(
                primary_key_equal_clauses_array
                    .get(0)
                    .expect("there must be a value")
                    .clone(),
            )),
            _ => Err(Error::InvalidQuery(
                "There should only be one equal clause for the primary key",
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
            _ => Err(Error::InvalidQuery(
                "There should only be one in clause for the primary key",
            )),
        }?;

        let in_clause = match in_clauses_array.len() {
            0 => Ok(None),
            1 => Ok(Some(
                in_clauses_array
                    .get(0)
                    .expect("there must be a value")
                    .clone(),
            )),
            _ => Err(Error::InvalidQuery("There should only be one in clause")),
        }?;

        let equal_clauses: HashMap<String, WhereClause> = equal_clauses_array
            .into_iter()
            .map(|where_clause| (where_clause.field.clone(), where_clause))
            .collect();

        let internal_clauses = InternalClauses {
            primary_key_equal_clause,
            primary_key_in_clause,
            in_clause,
            range_clause,
            equal_clauses,
        };

        match internal_clauses.verify() {
            true => Ok(internal_clauses),
            false => Err(Error::InvalidQuery("Query has invalid where clauses")),
        }
    }

    pub fn construct_path_query(
        &self,
        grove: &GroveDb,
        transaction: TransactionArg,
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

                let start_at_document = grove
                    .get(start_at_document_path, &start_at_document_key, transaction)
                    .map_err(|e| match e {
                        Error::PathKeyNotFound(_) | Error::PathNotFound(_) => {
                            let error_message = if self.start_at_included {
                                "startAt document not found"
                            } else {
                                "startAfter document not found"
                            };

                            Error::InvalidQuery(error_message)
                        }
                        _ => e,
                    })?;

                if let Element::Item(item) = start_at_document {
                    let document = Document::from_cbor(item.as_slice(), None, None)?;
                    Ok(Some((document, self.start_at_included)))
                } else {
                    Err(Error::CorruptedData(String::from(
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
                    return Err(Error::InvalidQuery("order by should include $id only"));
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
                let in_values = match &primary_key_in_clause.value {
                    Value::Array(array) => Ok(array),
                    _ => Err(Error::InvalidQuery(
                        "when using in operator you must provide an array of values",
                    )),
                }?;
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
                        return Err(Error::InternalError("Not yet implemented"));
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
                        return Err(Error::InternalError("Not yet implemented"));
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

    pub fn get_non_primary_key_path_query(
        &self,
        document_type_path: Vec<Vec<u8>>,
        starts_at_document: Option<(Document, bool)>,
    ) -> Result<PathQuery, Error> {
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
            .ok_or(Error::InvalidQuery("query must be for valid indexes"))?;
        if difference > defaults::MAX_INDEX_DIFFERENCE {
            return Err(Error::InvalidQuery(
                "query must better match an existing index",
            ));
        }

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
                Some(where_clause) => match &self.internal_clauses.range_clause {
                    None => (Some(where_clause), true, None),
                    Some(range_clause) => (Some(where_clause), true, Some(range_clause)),
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

        fn recursive_insert(
            query: Option<&mut Query>,
            left_over_index_properties: &[&IndexProperty],
            unique: bool,
        ) -> Option<Query> {
            match left_over_index_properties.split_first() {
                None => {
                    if let Some(query) = query {
                        match unique {
                            true => {
                                query.set_subquery_key(vec![0]);

                                // In the case things are NULL we allow to have multiple values
                                let mut full_query = Query::new();
                                full_query.insert_all();

                                query.add_conditional_subquery(
                                    QueryItem::Key(b"".to_vec()),
                                    Some(vec![0]),
                                    Some(full_query),
                                );
                            }
                            false => {
                                query.set_subquery_key(vec![0]);
                                // we just get all by document id order ascending
                                let mut full_query = Query::new();
                                full_query.insert_all();
                                query.set_subquery(full_query);
                            }
                        }
                    }
                    None
                }
                Some((first, left_over)) => {
                    let mut inner_query = Query::new_with_direction(first.ascending);
                    inner_query.insert_all();
                    recursive_insert(Some(&mut inner_query), left_over, unique);
                    match query {
                        None => Some(inner_query),
                        Some(query) => {
                            query.set_subquery(inner_query);
                            query.set_subquery_key(first.name.as_bytes().to_vec());
                            None
                        }
                    }
                }
            }
        }

        let final_query = match last_clause {
            None => recursive_insert(None, left_over_index_properties.as_slice(), index.unique)
                .expect("Index must have left over properties if no last clause"),
            Some(where_clause) => {
                let left_to_right = if where_clause.operator.is_range() {
                    let order_clause: &OrderClause = self
                        .order_by
                        .get(where_clause.field.as_str())
                        .ok_or(Error::InvalidQuery(
                            "query must have an orderBy field for each range element",
                        ))?;

                    order_clause.ascending
                } else {
                    true
                };

                let mut query = where_clause.to_path_query(
                    self.document_type,
                    &starts_at_document,
                    left_to_right,
                )?;

                match subquery_clause {
                    None => {
                        recursive_insert(
                            Some(&mut query),
                            left_over_index_properties.as_slice(),
                            index.unique,
                        );
                    }
                    Some(where_clause) => {
                        let order_clause: &OrderClause = self
                            .order_by
                            .get(where_clause.field.as_str())
                            .ok_or(Error::InvalidQuery(
                                "query must have an orderBy field for each range element",
                            ))?;
                        let mut subquery = where_clause.to_path_query(
                            self.document_type,
                            &starts_at_document,
                            order_clause.ascending,
                        )?;
                        recursive_insert(
                            Some(&mut subquery),
                            left_over_index_properties.as_slice(),
                            index.unique,
                        );
                        let subindex = where_clause.field.as_bytes().to_vec();
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
        let last_index = last_indexes.first().ok_or(Error::InvalidQuery(
            "document query has no index with fields",
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
        _grove: &GroveDb,
        _transaction: TransactionArg,
    ) -> Result<Vec<u8>, Error> {
        todo!()
    }

    pub fn execute_no_proof(
        &self,
        grove: &GroveDb,
        transaction: TransactionArg,
    ) -> Result<(Vec<Vec<u8>>, u16), Error> {
        let path_query = self.construct_path_query(grove, transaction)?;

        let query_result = grove.get_path_query(&path_query, transaction);
        match query_result {
            Err(Error::PathKeyNotFound(_)) | Err(Error::PathNotFound(_)) => Ok((Vec::new(), 0)),
            _ => query_result,
        }
    }
}
