use crate::common::encode::encode_u64;
use crate::drive::document::contract_document_type_path;

use crate::query::Query;
use grovedb::{PathQuery, SizedQuery};

/// Drive query struct
#[derive(Debug, PartialEq, Clone)]
pub struct SingleDocumentDriveQuery {
    ///DataContract
    pub contract_id: [u8; 32],
    /// Document type
    pub document_type_name: String,
    /// Document type keeps history
    pub document_type_keeps_history: bool,
    /// Document
    pub document_id: [u8; 32],
    /// Block time
    pub block_time_ms: Option<u64>,
}

impl SingleDocumentDriveQuery {
    /// Operations to construct a path query.
    pub fn construct_path_query(&self) -> PathQuery {
        // First we should get the overall document_type_path
        let mut path =
            contract_document_type_path(&self.contract_id, self.document_type_name.as_str())
                .into_iter()
                .map(|a| a.to_vec())
                .collect::<Vec<Vec<u8>>>();

        path.push(vec![0]);

        let mut query = Query::new();
        query.insert_key(self.document_id.to_vec());

        if self.document_type_keeps_history {
            // if the documents keep history then we should insert a subquery
            if let Some(block_time) = self.block_time_ms {
                let encoded_block_time = encode_u64(block_time);
                let mut sub_query = Query::new_with_direction(false);
                sub_query.insert_range_to_inclusive(..=encoded_block_time);
                query.set_subquery(sub_query);
            } else {
                query.set_subquery_key(vec![0]);
            }
        }

        PathQuery::new(path, SizedQuery::new(query, Some(1), None))
    }
}

impl From<SingleDocumentDriveQuery> for PathQuery {
    fn from(value: SingleDocumentDriveQuery) -> Self {
        value.construct_path_query()
    }
}
