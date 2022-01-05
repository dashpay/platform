use std::path::Path;
use grovedb::{Error, GroveDb, PathQuery, Query, QueryItem};
use crate::contract::{Contract, DocumentType, Index};

pub struct DocumentPathQuery<'a> {
    document_type: DocumentType,
    index : Index,
    intermediate_values: Vec<Vec<u8>>,
    path_query : PathQuery<'a>,
}

impl DocumentPathQuery {
    pub fn construct(contract: Contract, document_type_name : &str, index: Index, intermediate_values: Vec<Vec<u8>>, final_query: Query) -> Result<Self, Error> {
        // first let's get the contract path

        let mut contract_path = contract.document_type_path(document_type_name);

        let (last_index, intermediate_indexes) = index.properties.split_last().ok_or(
            Error::CorruptedData(String::from("document query has no index with fields")),
        )?;

        for (intermediate_index, intermediate_value) in intermediate_indexes.iter().zip(intermediate_values.iter()) {
            contract_path.push(intermediate_index.name.as_bytes());
            contract_path.push(intermediate_value.as_slice());
        }
        
        let mut query = Query::new();
        query1.insert_range_inclusive(b"key3".to_vec()..=b"key4".to_vec());
        query2.insert_key(b"key6".to_vec());

        let document_path_query = DocumentPathQuery {
            document_type: DocumentType {},
            index,
            intermediate_values,
            path_query: PathQuery {
                path: &[],
                query: final_query
            }
        };
    }

    fn execute(&self, mut grove: GroveDb) -> Result<vec<vec<u8>>, Error> {
        let (last_index, intermediate_indexes) = self.index.properties.split_last().ok_or(
            Error::CorruptedData(String::from("document query has no index with fields")),
        )?;
        for i in intermediate_indexes {
            grove.get()
        }

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