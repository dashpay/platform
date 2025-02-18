use crate::drive::Drive;
use bincode::{Decode, Encode};
use dpp::identifier::Identifier;
use grovedb::PathQuery;

/// Token status drive query struct
#[derive(Debug, PartialEq, Clone, Encode, Decode)]
pub struct TokenStatusDriveQuery {
    /// the token id
    pub token_id: Identifier,
}

impl TokenStatusDriveQuery {
    /// Operations to construct a path query.
    pub fn construct_path_query(&self) -> PathQuery {
        Drive::token_status_query(self.token_id.to_buffer())
    }
}
