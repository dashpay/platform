mod defaults;

use std::collections::HashMap;
use std::path::Path;
use grovedb::{Element, Error, GroveDb, PathQuery, Query, QueryItem};
use crate::contract::{Contract, DocumentType, Index};
use ciborium::value::{Value as CborValue, Value};
use serde::{Deserialize, Serialize};
use crate::query::WhereOperator::{Equal, GreaterThan, GreaterThanOrEquals, LessThan, LessThanOrEquals, In, StartsWith, Between, BetweenExcludeBounds, BetweenExcludeLeft, BetweenExcludeRight};

pub struct DocumentPathQuery<'a> {
    document_type: &'a DocumentType,
    index : &'a Index,
    intermediate_values: Vec<Vec<u8>>,
    path_query : PathQuery<'a>,
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
        &_ => None
    }
}

#[derive(Copy, Clone)]
pub struct WhereClause<'a> {
    field: &'a str,
    operator: WhereOperator,
    value: &'a Value
}

impl<'a> WhereClause<'a> {
    pub fn from_components(clause_components: &'a Vec<Value>) -> Result<Self, Error> {
        if clause_components.len() != 3 {
            return Err(Error::CorruptedData(String::from("limit should be a integer from 1 to 100")));
        }
        let CborValue::Text(field) = clause_components.get(0)
            .ok_or(Error::CorruptedData(String::from("first field of where component should be a string")))?;
        let CborValue::Text(operator_string) = clause_components.get(1)
            .ok_or(Error::CorruptedData(String::from("second field of where component should exist")))?;
        let operator = operator_from_string(operator_string)
            .ok_or(Error::CorruptedData(String::from("second field of where component should be a known operator")))?;
        let value = clause_components.get(2)
            .ok_or(Error::CorruptedData(String::from("third field of where component should exist")))?;
        Ok(WhereClause {
            field,
            operator,
            value,
        })
    }

    fn group_range_clauses(where_clauses: &'a[WhereClause]) -> Result<Option<Self>, Error> {
        // In order to group range clauses
        let groupable_range_clauses : Vec<&WhereClause> = where_clauses.iter().filter(| where_clause| {
            match where_clause.operator {
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
            }
        }).collect();

        let non_groupable_range_clauses : Vec<&WhereClause> = where_clauses.iter().filter(| where_clause| {
            match where_clause.operator {
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
            }
        }).collect();

        if non_groupable_range_clauses.len() == 0 {
            if groupable_range_clauses.len() == 0 {
                return Ok(None);
            }
            if groupable_range_clauses.len() == 1 {
                return Ok(Some(*groupable_range_clauses.get(0).unwrap().clone()))
            }

            if groupable_range_clauses.len() > 2 {
                return Err(Error::CorruptedData(String::from("there can only be at most 2 range clauses")));
            }

            groupable_range_clauses.get(0).unwrap();
        } else if non_groupable_range_clauses.len() == 1 {
            let where_clause = non_groupable_range_clauses.get(0).unwrap();
            return Ok(Some(WhereClause {
                field: where_clause.field,
                operator: where_clause.operator,
                value: where_clause.value,
            }))
        } else {
            // if non_groupable_range_clauses.len() > 1
            return Err(Error::CorruptedData(String::from("there can not be more than 1 non groupable range clause")));
        }

        Ok(None)
    }

    fn to_path_query(&self) -> Query {
        let mut query = Query::new();
        match self.operator {
            Equal => { query.insert_key(self.value)}
            GreaterThan => { query.insert_range_from()}
            GreaterThanOrEquals => {query.insert_range_from()}
            LessThan => {query.insert_range_to()}
            LessThanOrEquals => {query.insert_range_to()}
            Between => { query.insert_range_inclusive()}
            BetweenExcludeBounds => { query.insert_range()}
            BetweenExcludeLeft => { query.insert_range()}
            BetweenExcludeRight => { query.insert_range()}
            In => {}
            StartsWith => {}
        }
        query
    }
}

pub struct DriveQuery<'a> {
    contract: &'a Contract,
    document_type: &'a DocumentType,
    equal_clauses: HashMap<&'a str, WhereClause<'a>>,
    range_clause: Option<WhereClause<'a>>,
    offset: u16,
    limit: u16,
    order_by: &'a str,
    start_at: Option<Vec<u8>>,
    start_at_included: bool,
}

impl<'a> DriveQuery<'a> {
    pub fn from_cbor(query_cbor: &[u8], contract: &'a Contract, document_type: &'a DocumentType) -> Result<Self, Error> {
        let mut query_document: HashMap<String, CborValue> = ciborium::de::from_reader(query_cbor)
            .map_err(|_| Error::CorruptedData(String::from("unable to decode query")))?;

        let limit: u16 = query_document.get("limit").map_or( Some(defaults::DEFAULT_QUERY_LIMIT), |id_cbor| {
            if let CborValue::Integer(b) = id_cbor {
                Some(i128::from(*b) as u16)
            } else {
                None
            }
        }).ok_or(Error::CorruptedData(String::from("limit should be a integer from 1 to 100")))?;

        let all_where_clauses : Vec<WhereClause> = query_document.get("where").map_or( vec![], |id_cbor| {
            if let CborValue::Array(clauses) = id_cbor {
                clauses.iter().filter_map( | where_clause| {
                    if let CborValue::Array(clauses_components) = where_clause {
                        WhereClause::from_components(clauses_components).ok()
                    } else {
                        None
                    }
                }).collect()
            } else {
                vec![]
            }
        });

        let range_clause = WhereClause::group_range_clauses(&all_where_clauses)?;

        let equal_clauses_array = all_where_clauses.into_iter().filter(| where_clause| {
            match where_clause.operator {
                Equal => true,
                In => true,
                _ => false,
            }
        }).collect::<Vec<&WhereClause>>();

        let equal_clauses= equal_clauses_array.into_iter().map(| where_clause| {
            (where_clause.field, where_clause)
        }).collect();

        let start_at_option = query_document.get("startAt");
        let start_after_option = query_document.get("startAfter");
        if start_after_option.is_some() && start_at_option.is_some() {
            return Err(Error::CorruptedData(String::from("only one of startAt or startAfter should be provided")));
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

        let mut offset: u16 = start_option.map_or( Some(0), |id_cbor| {
            if let CborValue::Integer(b) = id_cbor {
                Some(i128::from(*b) as u16)
            } else {
                None
            }
        }).ok_or(Error::CorruptedData(String::from("limit should be a integer from 1 to 100")))?;

        if !start_at_included {
            offset += 1;
        }

        Ok(DriveQuery {
            contract,
            document_type,
            equal_clauses,
            range_clause,
            offset,
            limit,
            order_by: "",
            start_at: None,
            start_at_included,
        })
    }

    pub fn create_path_query(&self) -> Result<DocumentPathQuery, Error> {
        let equal_fields = self.equal_clauses.iter().map(|clause| {
            clause.field
        }).collect::<Vec<&str>>();
        let range_field = match self.range_clause {
            None => None,
            Some(range_clause) => Some(range_clause.field)
        };
        let (index, difference) = self.document_type.index_for_types(equal_fields.as_slice(), range_field).ok_or(
            Error::InvalidQuery("query must be for valid indexes"),
        )?;
        if difference > defaults::MAX_INDEX_DIFFERENCE {
            Err(Error::InvalidQuery("query must better match an existing index"))
        }
        let ordered_clauses: Vec<&WhereClause> = index.properties.iter().filter_map(| field| {
            match self.equal_clauses.get(&field.name) {
                None => None,
                Some(where_clause) => Some(where_clause)
            }
        }).collect();
        let last_clause = match self.range_clause {
            None => {
                ordered_clauses.last()
            }
            Some(where_clause) => {
                &where_clause
            }
        };
        let intermediate_values = index.properties.iter().filter_map(| field| {
            match self.equal_clauses.get(&field.name) {
                None => None,
                Some(where_clause) => Some(where_clause.value)
            }
        }).collect();
        DocumentPathQuery::new(self.contract, self.document_type.name.as_str(), index, intermediate_values, last_clause.unwrap().to_path_query())
    }
}

impl<'a> DocumentPathQuery<'a> {

    pub fn new(contract: &'a Contract, document_type_name : &'a str, index: &'a Index, intermediate_values: Vec<Vec<u8>>, final_query: Query) -> Result<Self, Error> {
        // first let's get the contract path

        let mut contract_path = contract.document_type_path(document_type_name);

        let document_type = contract.document_types.get(document_type_name).ok_or(
            Error::CorruptedData(String::from("unknown document type name")),
        )?;

        let (last_index, intermediate_indexes) = index.properties.split_last().ok_or(
            Error::CorruptedData(String::from("document query has no index with fields")),
        )?;

        for (intermediate_index, intermediate_value) in intermediate_indexes.iter().zip(intermediate_values.iter()) {
            contract_path.push(intermediate_index.name.as_bytes());
            contract_path.push(intermediate_value.as_slice());
        }

        contract_path.push(last_index.name.as_bytes());

        Ok(DocumentPathQuery {
            document_type,
            index,
            intermediate_values,
            path_query: PathQuery::new(contract_path.as_slice(), final_query)
        })
    }

    fn execute_no_proof(self, mut grove: GroveDb, transaction: Option<&OptimisticTransactionDBTransaction>) -> Result<(Vec<Vec<u8>>, u16), Error> {
        let path_query = self.path_query;
        grove.get_path_query(&path_query, transaction)
    }

    fn execute_with_proof(self, mut grove: GroveDb) -> Result<Vec<u8>, Error> {
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