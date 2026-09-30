use crate::drive::contract::moderation::types::{
    ContractTeamActionEntry, ContractTeamActionsQuery,
};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::query_result_type::QueryResultType;
use grovedb::TransactionArg;

impl Drive {
    #[inline(always)]
    pub(super) fn fetch_contract_team_actions_v0(
        &self,
        contract_id: Identifier,
        query: &ContractTeamActionsQuery,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<ContractTeamActionEntry>, Error> {
        Self::check_contract_team_actions_query(query, platform_version)?;
        if !self.contract_keeps_team_actions(contract_id, transaction, platform_version)? {
            return Ok(vec![]);
        }
        let path_query = Self::contract_team_actions_query(contract_id.to_buffer(), query);
        let (results, _) = self.grove_get_raw_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryPathKeyElementTrioResultType,
            &mut vec![],
            &platform_version.drive,
        )?;
        ContractTeamActionEntry::from_path_key_elements(results.to_path_key_elements()).map_err(
            |description| {
                Error::Drive(DriveError::CorruptedDriveState(format!(
                    "contract {} team action is malformed: {}",
                    contract_id, description
                )))
            },
        )
    }
}
