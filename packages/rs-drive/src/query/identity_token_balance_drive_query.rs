use crate::drive::Drive;
use bincode::{Decode, Encode};
use dpp::identifier::Identifier;
use grovedb::PathQuery;

/// Identity token balance drive query struct
#[derive(Debug, PartialEq, Clone, Encode, Decode)]
pub struct IdentityTokenBalanceDriveQuery {
    /// The identity who we are querying for
    pub identity_id: Identifier,
    /// the token id
    pub token_id: Identifier,
}

impl IdentityTokenBalanceDriveQuery {
    /// Operations to construct a path query.
    pub fn construct_path_query(&self) -> PathQuery {
        Drive::token_balance_for_identity_id_query(
            self.token_id.to_buffer(),
            self.identity_id.to_buffer(),
        )
    }
}
