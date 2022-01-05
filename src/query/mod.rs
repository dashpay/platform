use std::path::Path;
use grovedb::{Element, Error, GroveDb, PathQuery, Query, QueryItem};
use crate::contract::{Contract, DocumentType, Index};

pub struct DocumentPathQuery<'a> {
    document_type: &'a DocumentType,
    index : &'a Index,
    intermediate_values: Vec<Vec<u8>>,
    path_query : PathQuery<'a>,
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