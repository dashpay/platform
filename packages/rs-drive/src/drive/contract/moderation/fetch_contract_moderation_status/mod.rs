mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::epoch::Epoch;
use dpp::data_contract::config::moderation::{ContractModerationList, ContractModerationStatus};
use dpp::fee::fee_result::FeeResult;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Reads one identity's status on a moderated contract: whether it is on the banlist, and
    /// until when it is on the suspension list. Only the lists in `lists` are read; those are
    /// the lists the contract's config declares, and an undeclared list has no tree.
    ///
    /// # Parameters
    ///
    /// * `contract_id`: The moderated contract.
    /// * `identity_id`: The identity.
    /// * `lists`: The lists the contract keeps.
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok(ContractModerationStatus)` with the status.
    /// * `Err(Error)` when the version is unknown or a read fails.
    pub fn fetch_contract_moderation_status(
        &self,
        contract_id: Identifier,
        identity_id: Identifier,
        lists: &[ContractModerationList],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ContractModerationStatus, Error> {
        match platform_version
            .drive
            .methods
            .contract
            .moderation
            .fetch_contract_moderation_status
        {
            0 => self.fetch_contract_moderation_status_add_to_operations_v0(
                contract_id,
                identity_id,
                lists,
                transaction,
                &mut vec![],
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_contract_moderation_status".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// [`Drive::fetch_contract_moderation_status`] with the fee of the reads, so that consensus
    /// validation can bill them.
    ///
    /// # Parameters
    ///
    /// * `contract_id`: The moderated contract.
    /// * `identity_id`: The identity.
    /// * `lists`: The lists the contract keeps.
    /// * `epoch`: The epoch the fee is priced for.
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok((FeeResult, ContractModerationStatus))` with the fee and the status.
    /// * `Err(Error)` when the version is unknown or a read fails.
    pub fn fetch_contract_moderation_status_with_fee(
        &self,
        contract_id: Identifier,
        identity_id: Identifier,
        lists: &[ContractModerationList],
        epoch: &Epoch,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(FeeResult, ContractModerationStatus), Error> {
        match platform_version
            .drive
            .methods
            .contract
            .moderation
            .fetch_contract_moderation_status
        {
            0 => {
                let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
                let status = self.fetch_contract_moderation_status_add_to_operations_v0(
                    contract_id,
                    identity_id,
                    lists,
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
                Ok((fee, status))
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_contract_moderation_status_with_fee".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
