use crate::drive::contract::moderation::types::decode_contract_team_action;
use crate::drive::contract::paths::{contract_team_action_path, CONTRACT_TEAM_ACTION_INFO_KEY};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::DirectQueryType;
use dpp::data_contract::config::moderation::ContractTeamAction;
use dpp::group::group_action_status::GroupActionStatus;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// The action, read under the active actions, then under the closed ones when it is not
    /// there: the operations of the reads are added to `drive_operations` for billing. One point
    /// read per status, of the action's info: a read through an action id a status does not hold
    /// reads as none, so no probe of the id comes first.
    pub(super) fn fetch_contract_team_action_add_to_operations_v0(
        &self,
        contract_id: Identifier,
        action_id: Identifier,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<(GroupActionStatus, ContractTeamAction)>, Error> {
        let contract = contract_id.as_slice();
        let action = action_id.as_slice();
        for status in [
            GroupActionStatus::ActionActive,
            GroupActionStatus::ActionClosed,
        ] {
            let Some(value) = self.grove_get_raw_optional_item(
                (&contract_team_action_path(contract, status, action)).into(),
                CONTRACT_TEAM_ACTION_INFO_KEY,
                DirectQueryType::StatefulDirectQuery,
                transaction,
                drive_operations,
                &platform_version.drive,
            )?
            else {
                continue;
            };
            let action = decode_contract_team_action(&value).map_err(|description| {
                Error::Drive(DriveError::CorruptedDriveState(format!(
                    "contract {} team action {} is malformed: {}",
                    contract_id, action_id, description
                )))
            })?;
            return Ok(Some((status, action)));
        }
        Ok(None)
    }
}
