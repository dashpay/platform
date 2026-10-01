mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::epoch::Epoch;
use dpp::data_contract::config::moderation::ContractTeamAction;
use dpp::fee::fee_result::FeeResult;
use dpp::group::group_action_status::GroupActionStatus;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// One of a contract's team actions, wherever it is: active, still gathering approvals, or
    /// closed, having run. `None` when the contract's team never proposed it, and when the
    /// contract keeps no team actions.
    ///
    /// # Parameters
    ///
    /// * `contract_id`: The contract whose seated team votes on the action.
    /// * `action_id`: The action.
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok(Some((status, action)))` for an action the team proposed.
    /// * `Ok(None)` when there is none.
    /// * `Err(Error)` when the method version is unknown, a read fails, or the stored action is
    ///   malformed.
    pub fn fetch_contract_team_action(
        &self,
        contract_id: Identifier,
        action_id: Identifier,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<(GroupActionStatus, ContractTeamAction)>, Error> {
        match platform_version
            .drive
            .methods
            .contract
            .moderation
            .fetch_contract_team_action
        {
            0 => self.fetch_contract_team_action_add_to_operations_v0(
                contract_id,
                action_id,
                transaction,
                &mut vec![],
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_contract_team_action".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// One of a contract's team actions, active or closed, with the fee of the read, so that
    /// consensus validation can bill it. An action the team never proposed reads as `None`, and
    /// so does any action of a contract that keeps none.
    ///
    /// # Parameters
    ///
    /// * `contract_id`: The contract whose seated team votes on the action.
    /// * `action_id`: The action.
    /// * `epoch`: The epoch the read is priced in.
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok((FeeResult, Option<(status, action)>))`: the fee of the read and the action,
    ///   `None` when the team never proposed it.
    /// * `Err(Error)` when the method version is unknown, the read fails, the action is
    ///   malformed, or the fee cannot be calculated.
    pub fn fetch_contract_team_action_with_fee(
        &self,
        contract_id: Identifier,
        action_id: Identifier,
        epoch: &Epoch,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(FeeResult, Option<(GroupActionStatus, ContractTeamAction)>), Error> {
        match platform_version
            .drive
            .methods
            .contract
            .moderation
            .fetch_contract_team_action
        {
            0 => {
                let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
                let action = self.fetch_contract_team_action_add_to_operations_v0(
                    contract_id,
                    action_id,
                    transaction,
                    &mut drive_operations,
                    platform_version,
                )?;
                let fee = Drive::calculate_fee(
                    None,
                    Some(drive_operations),
                    epoch,
                    self.config.epochs_per_era,
                    platform_version,
                    None,
                )?;
                Ok((fee, action))
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_contract_team_action_with_fee".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
