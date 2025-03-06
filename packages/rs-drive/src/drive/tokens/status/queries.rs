use crate::drive::tokens::paths::token_statuses_root_path_vec;
use crate::drive::Drive;
use crate::query::Query;
use grovedb::{PathQuery, SizedQuery};

impl Drive {
    /// The query getting a token statuses
    pub fn token_statuses_query(token_ids: &[[u8; 32]]) -> PathQuery {
        let tokens_root = token_statuses_root_path_vec();

        let mut query = Query::new();

        for token_id in token_ids {
            query.insert_key(token_id.to_vec());
        }

        PathQuery::new(
            tokens_root,
            SizedQuery::new(query, Some(token_ids.len() as u16), None),
        )
    }

    /// The query getting a token statuses
    pub fn token_status_query(token_id: [u8; 32]) -> PathQuery {
        let tokens_root = token_statuses_root_path_vec();

        let mut query = Query::new();

        query.insert_key(token_id.to_vec());

        PathQuery::new(tokens_root, SizedQuery::new(query, Some(1), None))
    }
}
