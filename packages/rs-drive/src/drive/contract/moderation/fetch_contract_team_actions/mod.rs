mod v0;

use crate::drive::contract::moderation::types::{
    ContractTeamActionEntry, ContractTeamActionsQuery,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// One page of a contract's team actions, active or closed as the query says, in action id
    /// order. A contract that keeps no team actions reads as none.
    ///
    /// # Parameters
    ///
    /// * `contract_id`: The contract whose seated team votes on the actions.
    /// * `query`: The status, where the page starts and its limit.
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<ContractTeamActionEntry>)` with the actions found, in action id order.
    /// * `Err(Error)` when the method version is unknown, the limit is zero or above the
    ///   maximum, a read fails, or a stored action is malformed.
    pub fn fetch_contract_team_actions(
        &self,
        contract_id: Identifier,
        query: &ContractTeamActionsQuery,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<ContractTeamActionEntry>, Error> {
        match platform_version
            .drive
            .methods
            .contract
            .moderation
            .fetch_contract_team_actions
        {
            0 => self.fetch_contract_team_actions_v0(
                contract_id,
                query,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_contract_team_actions".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
