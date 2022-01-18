mod defaults;

use crate::contract::{Contract, DocumentType, Index};
use crate::query::WhereOperator::{
    Between, BetweenExcludeBounds, BetweenExcludeLeft, BetweenExcludeRight, Equal, GreaterThan,
    GreaterThanOrEquals, In, LessThan, LessThanOrEquals, StartsWith,
};
use ciborium::value::{Value as CborValue, Value};
use grovedb::{Error, GroveDb, PathQuery, Query, SizedQuery};
use indexmap::IndexMap;
use std::collections::HashMap;
use storage::rocksdb_storage::OptimisticTransactionDBTransaction;

pub struct DocumentPathQuery<'a> {
    document_type: &'a DocumentType,
    index: &'a Index,
    intermediate_values: Vec<Vec<u8>>,
    path: Vec<Vec<u8>>,
    query: SizedQuery,
    subquery_key: Option<Vec<u8>>,
    subquery: Option<Query>,
}

#[derive(Copy, Clone)]
pub enum WhereOperator {
    Equal,
    GreaterThan,
    GreaterThanOrEquals,
    LessThan,
    LessThanOrEquals,
    Between,
    BetweenExcludeBounds,
    BetweenExcludeLeft,
    BetweenExcludeRight,
    In,
    StartsWith,
}

fn operator_from_string(string: &str) -> Option<WhereOperator> {
    match string {
        "=" => Some(Equal),
        ">" => Some(GreaterThan),
        ">=" => Some(GreaterThanOrEquals),
        "<" => Some(LessThan),
        "<=" => Some(LessThanOrEquals),
        "Between" => Some(Between),
        "BetweenExcludeBounds" => Some(BetweenExcludeBounds),
        "BetweenExcludeLeft" => Some(BetweenExcludeLeft),
        "BetweenExcludeRight" => Some(BetweenExcludeRight),
        "In" => Some(In),
        "StartsWith" => Some(StartsWith),
        &_ => None,
    }
}

#[derive(Clone)]
pub struct WhereClause {
    field: String,
    operator: WhereOperator,
    value: Value,
}

impl<'a> WhereClause {
    pub fn from_components(clause_components: &'a Vec<Value>) -> Result<Self, Error> {
        if clause_components.len() != 3 {
            return Err(Error::CorruptedData(String::from(
                "limit should be a integer from 1 to 100",
            )));
        }

        let field_value = clause_components
            .get(0)
            .expect("check above enforces it exists");
        let field_ref = field_value
            .as_text()
            .ok_or(Error::CorruptedData(String::from(
                "first field of where component should be a string",
            )))?;
        let field = String::from(field_ref);

        let operator_value = clause_components
            .get(1)
            .expect("check above enforces it exists");
        let operator_string =
            operator_value
                .as_text()
                .ok_or(Error::CorruptedData(String::from(
                    "second field of where component should be a string",
                )))?;

        let operator = operator_from_string(operator_string).ok_or(Error::CorruptedData(
            String::from("second field of where component should be a known operator"),
        ))?;

        let value = clause_components
            .get(2)
            .ok_or(Error::CorruptedData(String::from(
                "third field of where component should exist",
            )))?.clone();

        Ok(WhereClause {
            field,
            operator,
            value,
        })
    }

    fn lower_bound_clause(where_clauses: &'a [&WhereClause]) -> Result<Option<&'a Self>, Error> {
        let lower_range_clauses : Vec<&&WhereClause> = where_clauses.iter().filter(|&where_clause| match where_clause.operator {
            GreaterThan => true,
            GreaterThanOrEquals => true,
            _ => false,
        })
        .collect::<Vec<&&WhereClause>>();
        match lower_range_clauses.len() {
            0 => Ok(None),
            1 => Ok(Some(lower_range_clauses.get(0).unwrap())),
            _ => Err(Error::CorruptedData(String::from(
                "there can only at most one range clause with a lower bound",
            )))
        }
    }

    fn upper_bound_clause(where_clauses: &'a [&WhereClause]) -> Result<Option<&'a Self>, Error> {
        let upper_range_clauses : Vec<&&WhereClause> = where_clauses.into_iter().filter(|&where_clause| match where_clause.operator {
            LessThan => true,
            LessThanOrEquals => true,
            _ => false,
        })
            .collect::<Vec<&&WhereClause>>();
        match upper_range_clauses.len() {
            0 => Ok(None),
            1 => Ok(Some(upper_range_clauses.get(0).unwrap())),
            _ => Err(Error::CorruptedData(String::from(
                "there can only at most one range clause with a lower bound",
            )))
        }
    }

    fn group_range_clauses(where_clauses: &'a [WhereClause]) -> Result<Option<Self>, Error> {
        // In order to group range clauses
        let groupable_range_clauses: Vec<&WhereClause> = where_clauses
            .iter()
            .filter(|where_clause| match where_clause.operator {
                Equal => false,
                In => false,
                GreaterThan => true,
                GreaterThanOrEquals => true,
                LessThan => true,
                LessThanOrEquals => true,
                StartsWith => false,
                Between => false,
                BetweenExcludeBounds => false,
                BetweenExcludeRight => false,
                BetweenExcludeLeft => false,
            })
            .collect();

        let non_groupable_range_clauses: Vec<&WhereClause> = where_clauses
            .iter()
            .filter(|where_clause| match where_clause.operator {
                Equal => false,
                In => false,
                GreaterThan => false,
                GreaterThanOrEquals => false,
                LessThan => false,
                LessThanOrEquals => false,
                StartsWith => true,
                Between => true,
                BetweenExcludeBounds => true,
                BetweenExcludeRight => true,
                BetweenExcludeLeft => true,
            })
            .collect();

        if non_groupable_range_clauses.len() == 0 {
            if groupable_range_clauses.len() == 0 {
                return Ok(None);
            } else if groupable_range_clauses.len() == 1 {
                let clause = *groupable_range_clauses.get(0).unwrap();
                return Ok(Some(clause.clone()));
            } else if groupable_range_clauses.len() > 2 {
                return Err(Error::CorruptedData(String::from(
                    "there can only be at most 2 range clauses",
                )));
            }

            if groupable_range_clauses.iter().any(|&z| z.field != groupable_range_clauses.first().unwrap().field) {
                return Err(Error::CorruptedData(String::from(
                    "all ranges must be on same field",
                )));
            }

            // we need to find the bounds of the clauses
            let lower_bounds_clause = WhereClause::lower_bound_clause(groupable_range_clauses.as_slice())?;
            let upper_bounds_clause = WhereClause::upper_bound_clause(groupable_range_clauses.as_slice())?;

            if lower_bounds_clause.is_none() || upper_bounds_clause.is_none() {
                return Err(Error::CorruptedData(String::from(
                    "lower and upper bounds must be passed if providing 2 ranges",
                )));
            }

            let operator = match (lower_bounds_clause.unwrap().operator, upper_bounds_clause.unwrap().operator) {
                (GreaterThanOrEquals, LessThanOrEquals) => Some(Between),
                (GreaterThanOrEquals, LessThan) => Some(BetweenExcludeRight),
                (GreaterThan, LessThanOrEquals) => Some(BetweenExcludeLeft),
                (GreaterThan, LessThan) => Some(BetweenExcludeBounds),
                _ => None
            }.ok_or(Error::CorruptedData(String::from(
                "lower and upper bounds must be passed if providing 2 ranges",
            )))?;

            return Ok(Some(WhereClause {
                field: groupable_range_clauses.first().unwrap().field.clone(),
                operator,
                value: Value::Array(vec![lower_bounds_clause.unwrap().value.clone(), upper_bounds_clause.unwrap().value.clone()])
            }));
        } else if non_groupable_range_clauses.len() == 1 {
            let where_clause = non_groupable_range_clauses.get(0).unwrap();
            return Ok(Some((*where_clause).clone()));
        } else {
            // if non_groupable_range_clauses.len() > 1
            return Err(Error::CorruptedData(String::from(
                "there can not be more than 1 non groupable range clause",
            )));
        }
    }

    fn split_value_for_between(
        &self,
        document_type: &DocumentType,
    ) -> Result<(Vec<u8>, Vec<u8>), Error> {
        let in_values = match &self.value {
            Value::Array(array) => Some(array),
            _ => None,
        }
        .ok_or(Error::CorruptedData(String::from(
            "when using between operator you must provide a tuple array of values",
        )))?;
        if in_values.len() != 2 {
            return Err(Error::CorruptedData(String::from(
                "when using between operator you must provide an array of exactly two values",
            )));
        }
        let left_key =
            document_type.serialize_value_for_key(self.field.as_str(), in_values.get(0).unwrap())?;
        let right_key =
            document_type.serialize_value_for_key(self.field.as_str(), in_values.get(0).unwrap())?;
        Ok((left_key, right_key))
    }

    fn to_path_query(&self, document_type: &DocumentType) -> Result<Query, Error> {
        let mut query = Query::new();
        match self.operator {
            Equal => {
                let key = document_type.serialize_value_for_key(self.field.as_str(), &self.value)?;
                query.insert_key(key);
            }
            GreaterThan => {
                let key = document_type.serialize_value_for_key(self.field.as_str(), &self.value)?;
                query.insert_range_after(key..);
            }
            GreaterThanOrEquals => {
                let key = document_type.serialize_value_for_key(self.field.as_str(), &self.value)?;
                query.insert_range_from(key..);
            }
            LessThan => {
                let key = document_type.serialize_value_for_key(self.field.as_str(), &self.value)?;
                query.insert_range_to(..key);
            }
            LessThanOrEquals => {
                let key = document_type.serialize_value_for_key(self.field.as_str(), &self.value)?;
                query.insert_range_to_inclusive(..=key);
            }
            Between => {
                let (left_key, right_key) = self.split_value_for_between(document_type)?;
                query.insert_range_inclusive(left_key..=right_key)
            }
            BetweenExcludeBounds => {
                let (left_key, right_key) = self.split_value_for_between(document_type)?;
                query.insert_range_after_to(left_key..right_key)
            }
            BetweenExcludeLeft => {
                let (left_key, right_key) = self.split_value_for_between(document_type)?;
                query.insert_range_after_to_inclusive(left_key..=right_key)
            }
            BetweenExcludeRight => {
                let (left_key, right_key) = self.split_value_for_between(document_type)?;
                query.insert_range(left_key..right_key)
            }
            In => {
                let in_values = match &self.value {
                    Value::Array(array) => Some(array),
                    _ => None,
                }
                .ok_or(Error::CorruptedData(String::from(
                    "when using in operator you must provide an array of values",
                )))?;
                for value in in_values.iter() {
                    let key = document_type.serialize_value_for_key(self.field.as_str(), value)?;
                    query.insert_key(key)
                }
            }
            StartsWith => {
                todo!()
            }
        }
        Ok(query)
    }
}

pub struct DriveQuery<'a> {
    contract: &'a Contract,
    document_type: &'a DocumentType,
    equal_clauses: HashMap<String, WhereClause>,
    range_clause: Option<WhereClause>,
    offset: u16,
    limit: u16,
    order_by: IndexMap<String, bool>,
    start_at: Option<Vec<u8>>,
    start_at_included: bool,
}

impl<'a> DriveQuery<'a> {
    pub fn from_cbor(
        query_cbor: &[u8],
        contract: &'a Contract,
        document_type: &'a DocumentType,
    ) -> Result<Self, Error> {
        let a = hex::encode(query_cbor);
        let query_document: HashMap<String, CborValue> = ciborium::de::from_reader(query_cbor)
            .map_err(|err| Error::CorruptedData(String::from(format!("unable to decode query: {}", err.to_string()))))?;

        let limit: u16 = query_document
            .get("limit")
            .map_or(Some(defaults::DEFAULT_QUERY_LIMIT), |id_cbor| {
                if let CborValue::Integer(b) = id_cbor {
                    Some(i128::from(*b) as u16)
                } else {
                    None
                }
            })
            .ok_or(Error::CorruptedData(String::from(
                "limit should be a integer from 1 to 100",
            )))?;

        let all_where_clauses: Vec<WhereClause> =
            query_document.get("where").map_or(vec![], |id_cbor| {
                if let CborValue::Array(clauses) = id_cbor {
                    clauses
                        .iter()
                        .filter_map(|where_clause| {
                            if let CborValue::Array(clauses_components) = where_clause {
                                WhereClause::from_components(clauses_components).ok()
                            } else {
                                None
                            }
                        })
                        .collect()
                } else {
                    vec![]
                }
            });

        let range_clause = WhereClause::group_range_clauses(&all_where_clauses)?;

        let equal_clauses_array = all_where_clauses.clone()
            .into_iter()
            .filter(|where_clause| match where_clause.operator {
                Equal => true,
                In => true,
                _ => false,
            })
            .collect::<Vec<WhereClause>>();

        let equal_clauses = equal_clauses_array
            .into_iter()
            .map(|where_clause| (where_clause.field.clone(), where_clause))
            .collect();

        let start_at_option = query_document.get("startAt");
        let start_after_option = query_document.get("startAfter");
        if start_after_option.is_some() && start_at_option.is_some() {
            return Err(Error::CorruptedData(String::from(
                "only one of startAt or startAfter should be provided",
            )));
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

        let mut offset: u16 = start_option
            .map_or(Some(0), |id_cbor| {
                if let CborValue::Integer(b) = id_cbor {
                    Some(i128::from(*b) as u16)
                } else {
                    None
                }
            })
            .ok_or(Error::CorruptedData(String::from(
                "limit should be a integer from 1 to 100",
            )))?;

        if !start_at_included {
            offset += 1;
        }

        let order_by = query_document
            .get("orderBy")
            .iter()
            .map(|record| {
                let order_tuple = match record {
                    Value::Array(order_tuple) => Some(order_tuple),
                    _ => None,
                }
                .ok_or(Error::CorruptedData(String::from(
                    "orderBy must always be an array of tuples",
                )))?;
                if order_tuple.len() != 2 {
                    return Err(Error::CorruptedData(String::from(
                        "orderBy must always have a tuple comprising a string and a asc/desc",
                    )));
                }
                let field_value = order_tuple.get(0).unwrap();
                let asc_string_value = order_tuple.get(1).unwrap();
                let asc_string = match asc_string_value {
                    Value::Text(asc_string) => Some(asc_string.as_str()),
                    _ => None,
                }
                .ok_or(Error::CorruptedData(String::from(
                    "orderBy right component must be a string",
                )))?;
                let left_to_right = match asc_string {
                    "asc" => true,
                    "desc" => false,
                    _ => {
                        return Err(Error::CorruptedData(String::from(
                            "orderBy right component must be either a asc or desc string",
                        )));
                    }
                };
                let field = match field_value {
                    Value::Text(field) => Some(field.as_str()),
                    _ => None,
                }
                .ok_or(Error::CorruptedData(String::from(
                    "orderBy left component must be a string",
                )))?;
                Ok((String::from(field), left_to_right))
            })
            .collect::<Result<IndexMap<String, bool>, Error>>()?;

        Ok(DriveQuery {
            contract,
            document_type,
            equal_clauses,
            range_clause,
            offset,
            limit,
            order_by,
            start_at: None,
            start_at_included,
        })
    }

    pub fn execute_no_proof(
        self,
        grove: &mut GroveDb,
        transaction: Option<&OptimisticTransactionDBTransaction>,
    ) -> Result<(Vec<Vec<u8>>, u16), Error> {
        let document_path_query = self.create_path_query()?;
        document_path_query.execute_no_proof(grove, transaction)
    }

    pub fn execute_with_proof(self, grove: &mut GroveDb, transaction: Option<&OptimisticTransactionDBTransaction>) -> Result<Vec<u8>, Error> {
        let document_path_query = self.create_path_query()?;
        document_path_query.execute_with_proof(grove, transaction)
    }

    fn create_path_query(&self) -> Result<DocumentPathQuery, Error> {
        let equal_fields = self.equal_clauses.keys().map(|s| s.as_str()).collect::<Vec<&str>>();
        let range_field = match &self.range_clause {
            None => None,
            Some(range_clause) => Some(range_clause.field.as_str()),
        };
        let (index, difference) = self
            .document_type
            .index_for_types(equal_fields.as_slice(), range_field)
            .ok_or(Error::InvalidQuery("query must be for valid indexes"))?;
        if difference > defaults::MAX_INDEX_DIFFERENCE {
            return Err(Error::InvalidQuery(
                "query must better match an existing index",
            ));
        }
        let ordered_clauses: Vec<&WhereClause> = index
            .properties
            .iter()
            .filter_map(|field| match self.equal_clauses.get(field.name.as_str()) {
                None => None,
                Some(where_clause) => Some(where_clause),
            })
            .collect();
        let (last_clause, last_clause_is_range) = match &self.range_clause {
            None => (ordered_clauses.last().map(|clause| *clause), false),
            Some(where_clause) => (Some(where_clause), true),
        };
        let intermediate_values =
            index
                .properties
                .iter()
                .filter_map(|field| {
                    match self.equal_clauses.get(field.name.as_str()) {
                        None => None,
                        Some(where_clause) => {
                            if self.order_by.len() == 0
                                && !last_clause_is_range
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
        let (final_query, left_to_right) = match last_clause {
            None => {
                let mut query = Query::new();
                query.insert_all();
                (query, true)
            }
            Some(where_clause) => {
                let left_to_right =
                    self.order_by
                        .get(where_clause.field.as_str())
                        .ok_or(Error::InvalidQuery(
                            "query must have an orderBy field for each range element",
                        ))?;
                let query = where_clause.to_path_query(self.document_type)?;
                (query, *left_to_right)
            }
        };
        let final_sized_query = SizedQuery::new(
            final_query,
            Some(self.limit),
            Some(self.offset),
            left_to_right,
        );
        DocumentPathQuery::new(
            self.contract,
            self.document_type.name.as_str(),
            index,
            intermediate_values,
            final_sized_query,
        )
    }
}

impl<'a> DocumentPathQuery<'a> {
    pub fn new(
        contract: &'a Contract,
        document_type_name: &'a str,
        index: &'a Index,
        intermediate_values: Vec<Vec<u8>>,
        final_query: SizedQuery,
    ) -> Result<Self, Error> {
        // first let's get the contract path

        let mut contract_path = contract.document_type_path(document_type_name).into_iter().map(|a| a.to_vec()).collect::<Vec<Vec<u8>>>();

        let document_type =
            contract
                .document_types
                .get(document_type_name)
                .ok_or(Error::CorruptedData(String::from(
                    "unknown document type name",
                )))?;

        let (last_index, intermediate_indexes) =
            index
                .properties
                .split_last()
                .ok_or(Error::CorruptedData(String::from(
                    "document query has no index with fields",
                )))?;

        for (intermediate_index, intermediate_value) in
            intermediate_indexes.iter().zip(intermediate_values.iter())
        {
            contract_path.push(intermediate_index.name.as_bytes().to_vec());
            contract_path.push(intermediate_value.as_slice().to_vec());
        }

        contract_path.push(last_index.name.as_bytes().to_vec());

        let subquery = match index.unique {
            true => None,
            false => {
                let mut query = Query::new();
                query.insert_all();
                Some(query)
            }
        };

        Ok(DocumentPathQuery {
            document_type,
            index,
            intermediate_values,
            path: contract_path,
            query: final_query,
            subquery_key: Some(b"0".to_vec()),
            subquery
        })
    }

    fn execute_no_proof(
        self,
        grove: &mut GroveDb,
        transaction: Option<&OptimisticTransactionDBTransaction>,
    ) -> Result<(Vec<Vec<u8>>, u16), Error> {
        let path = self.path.iter().map(|a| a.as_slice()).collect::<Vec<&[u8]>>();
        let subquery_key = self.subquery_key;
        let path_query = PathQuery::new(path.as_slice(), self.query, subquery_key, self.subquery);
        grove.get_path_query(&path_query, transaction)
    }

    fn execute_with_proof(self, grove: &mut GroveDb, transaction: Option<&OptimisticTransactionDBTransaction>) -> Result<Vec<u8>, Error> {
        todo!()
    }
}

// pub enum JoinType {
//     JoinTypeIntersection,
//     JoinTypeIntersectionExclusion,
//     JoinTypeUnion,
// }
//
// pub struct QueryGroupComponent {
//     paths : Vec<GroveDb::PathQuery>,
//     join : JoinType,
// }
//
// impl QueryGroupComponent {
//     fn execute(&self, mut grove: GroveDb) -> Result<vec<vec<u8>>, Error> {
//         grove.get_query(self.paths)
//     }
// }
//
// pub struct Query {
//     conditions : Vec<QueryGroupComponent>,
// }
