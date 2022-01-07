use std::collections::HashMap;
use std::path::Path;
use grovedb::{Element, Error, GroveDb, PathQuery, Query, QueryItem};
use crate::contract::{Contract, DocumentType, Index};
use ciborium::value::{Value as CborValue, Value};
use serde::{Deserialize, Serialize};
use test::RunIgnored::No;
use crate::query::WhereOperator::{Equal, GreaterThan, GreaterThanOrEquals, LessThan, LessThanOrEquals, In, StartsWith};

pub struct DocumentPathQuery<'a> {
    document_type: &'a DocumentType,
    index : &'a Index,
    intermediate_values: Vec<Vec<u8>>,
    path_query : PathQuery<'a>,
}

pub enum WhereOperator {
    Equal,
    GreaterThan,
    GreaterThanOrEquals,
    LessThan,
    LessThanOrEquals,
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
        "In" => Some(In),
        "StartsWith" => Some(StartsWith),
        &_ => None
    }
}

pub struct WhereClause<'a> {
    field: &'a str,
    operator: WhereOperator,
    value: &'a Value
}

impl WhereClause {
    pub fn from_components(clause_components: &Vec<Value>) -> Result<Self, Error> {
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
}

pub struct DriveQuery<'a> {
    contract: &'a Contract,
    document_type: &'a DocumentType,
    where_clauses: Vec<WhereClause<'a>>,
    range_clause: WhereClause<'a>,
    limit: u32,
    order_by: &'a str,
    start_at: Option<vec<u8>>,
    start_at_included: bool,
}

impl DriveQuery {
    pub fn from_cbor(query_cbor: &[u8], contract: &Contract, document_type: &DocumentType) -> Result<Self, Error> {
        let mut document: HashMap<String, CborValue> = ciborium::de::from_reader(query_cbor)
            .map_err(|_| Error::CorruptedData(String::from("unable to decode query")))?;

        let limit: u32 = document.get("limit").map_or( Some(100), |id_cbor| {
            if let CborValue::Integer(b) = id_cbor {
                Some(b as u32)
            } else {
                None
            }
        }).ok_or(Error::CorruptedData(String::from("limit should be a integer from 1 to 100")))?;

        let all_where_clauses : Vec<WhereClause> = document.get("where").map_or( vec![], |id_cbor| {
            if let CborValue::Array(clauses) = id_cbor {
                clauses.iter().map( | where_clause| {
                    if let CborValue::Array(clauses_components) = where_clause {
                        WhereClause::from_components(clauses_components)
                    } else {
                        Err(Error::CorruptedData(String::from("This must be an array of where clauses")))
                    }
                })?.collect()
            } else {
                vec![]
            }
        });

        let equal_clauses = all_where_clauses.iter().filter(| where_clause| {
            match where_clause.operator {
                Equal => true,
                GreaterThan => false,
                GreaterThanOrEquals => false,
                LessThan => false,
                LessThanOrEquals => false,
                In => true,
                StartsWith => false,
            }
        }).collect();

        let range_clauses = all_where_clauses.iter().filter(| where_clause| {
            match where_clause.operator {
                Equal => false,
                GreaterThan => true,
                GreaterThanOrEquals => true,
                LessThan => true,
                LessThanOrEquals => true,
                In => false,
                StartsWith => true,
            }
        }).collect();

        let start_at_option = document.get("startAt");
        let start_after_option = document.get("startAfter");
        if start_after_option.is_some() && start_at_option.is_some() {
            return Err(Error::CorruptedData(String::from("only one of startAt or startAfter should be provided")));
        }

        let mut start_at_included = false;

        let mut start_option: Option<&value> = None;

        if start_after_option.is_some() {
            start_option = start_after_option;
            start_at_included = false;
        } else if start_at_option.is_some() {
            start_option = start_at_option;
            start_at_included = true;
        }

        let start_at: vec<u8> = start_option.map_or( vec![], |id_cbor| {
            if let CborValue::Integer(b) = id_cbor {
                Some(b as u32)
            } else {
                None
            }
        }).ok_or(Error::CorruptedData(String::from("limit should be a integer from 1 to 100")))?;

        Ok(DriveQuery {
            where_clauses,
            range_clause: &WhereClause {},
            limit,
            order_by: "",
            start_at,
            start_at_included
        })
    }
}

impl DocumentPathQuery {

    pub fn construct<'a>(contract: &'a Contract, document_type_name : &str, index: &Index, intermediate_values: Vec<Vec<u8>>, final_query: Query) -> Result<Self, Error> {
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

    pub fn index_and_intermediate_values_for

    fn execute(self, mut grove: GroveDb) -> Result<vec<vec<u8>>, Error> {
        let path_query = self.path_query;
        let proof = grove.get_query(&[path_query])?.iter().map(| element | {
            match element {
                Element::Item(_) => {}
                Element::Reference(a) => {}
                Element::Tree(_) => {
                    // this should never happen
                    None
                }
            }
        })
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