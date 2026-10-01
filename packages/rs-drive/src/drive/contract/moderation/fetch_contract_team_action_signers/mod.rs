mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::epoch::Epoch;
use dpp::fee::fee_result::FeeResult;
use dpp::group::group_action_status::GroupActionStatus;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Who approved one of a contract's team actions, active or closed as `status` says, the
    /// proposer among them unless it left the team and its approval was dropped, in identity id
    /// order. None when there is no such action.
    ///
    /// # Parameters
    ///
    /// * `contract_id`: The contract whose seated team votes on the action.
    /// * `status`: Whether the action is active or closed.
    /// * `action_id`: The action.
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<Identifier>)` with the members that approved.
    /// * `Err(Error)` when the method version is unknown, a read fails, or a stored approval is
    ///   malformed.
    pub fn fetch_contract_team_action_signers(
        &self,
        contract_id: Identifier,
        status: GroupActionStatus,
        action_id: Identifier,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<Identifier>, Error> {
        match platform_version
            .drive
            .methods
            .contract
            .moderation
            .fetch_contract_team_action_signers
        {
            0 => {
                if !self.contract_team_action_exists_v0(
                    contract_id,
                    status,
                    action_id,
                    transaction,
                    platform_version,
                )? {
                    return Ok(vec![]);
                }
                self.fetch_contract_team_action_signers_add_to_operations_v0(
                    contract_id,
                    status,
                    action_id,
                    transaction,
                    &mut vec![],
                    platform_version,
                )
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_contract_team_action_signers".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Who approved one of a contract's team actions, with the fee of the read, so that
    /// consensus validation can bill it. The action must exist with that status, as validation
    /// read it: the range read of its approvals is not kept to an existing action here.
    ///
    /// # Parameters
    ///
    /// * `contract_id`: The contract whose seated team votes on the action.
    /// * `status`: Whether the action is active or closed.
    /// * `action_id`: The action.
    /// * `epoch`: The epoch the read is priced in.
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok((FeeResult, Vec<Identifier>))`: the fee of the read and the members that approved.
    /// * `Err(Error)` when the method version is unknown, the read fails, a stored approval is
    ///   malformed, or the fee cannot be calculated.
    pub fn fetch_contract_team_action_signers_with_fee(
        &self,
        contract_id: Identifier,
        status: GroupActionStatus,
        action_id: Identifier,
        epoch: &Epoch,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(FeeResult, Vec<Identifier>), Error> {
        match platform_version
            .drive
            .methods
            .contract
            .moderation
            .fetch_contract_team_action_signers
        {
            0 => {
                let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
                let signers = self.fetch_contract_team_action_signers_add_to_operations_v0(
                    contract_id,
                    status,
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
                Ok((fee, signers))
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_contract_team_action_signers_with_fee".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
