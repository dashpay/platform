mod fetch_contract_group_info;
mod fetch_contract_group_members;
mod fetch_contract_group_memberships_for_contract;

use crate::drive::Drive;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;

impl Drive {
    /// Checks a contract group members page limit: at least one entry, at most the configured
    /// maximum query limit, so that no page and no proof grows with the size of the group.
    pub(super) fn check_contract_group_members_limit(&self, limit: u16) -> Result<(), Error> {
        if limit == 0 || limit > self.config.max_query_limit {
            return Err(Error::Query(QuerySyntaxError::InvalidLimit(format!(
                "contract group members limit must be between 1 and {}, got {}",
                self.config.max_query_limit, limit
            ))));
        }
        Ok(())
    }
}
