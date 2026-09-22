//! Contract group queries: a group's stored information, one page of its members of one kind,
//! and the groups a contract belongs to.

mod contract_group_info;
mod contract_group_members;
mod contract_groups_for_contract;

use crate::error::query::QueryError;
use dpp::identifier::Identifier;

/// Parses a 32 byte identifier out of a request field, naming the field in the error.
pub(super) fn identifier_from_request(
    bytes: Vec<u8>,
    field: &str,
) -> Result<Identifier, QueryError> {
    Identifier::from_bytes(&bytes).map_err(|_| {
        QueryError::InvalidArgument(format!(
            "{field} must be a valid identifier (32 bytes long)"
        ))
    })
}
