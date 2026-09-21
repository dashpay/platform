mod v0;

use crate::drive::contract::fee_pots::types::ContractFeePotState;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::epoch::Epoch;
use dpp::data_contract::document_type::action_fees::ContractFeePot;
use dpp::fee::fee_result::FeeResult;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Reads one fee pot of a contract: the credits it holds and the epoch it was last claimed
    /// in. A pot that never received credits holds zero, and one that was never claimed has no
    /// last claim epoch.
    ///
    /// # Parameters
    ///
    /// * `contract_id`: The contract the pot belongs to.
    /// * `pot`: The pot.
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok(ContractFeePotState)` with the pot.
    /// * `Err(Error)` when the version is unknown or a read fails.
    pub fn fetch_contract_fee_pot(
        &self,
        contract_id: Identifier,
        pot: ContractFeePot,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ContractFeePotState, Error> {
        match platform_version
            .drive
            .methods
            .contract
            .fee_pots
            .fetch_contract_fee_pot
        {
            0 => self.fetch_contract_fee_pot_add_to_operations_v0(
                contract_id,
                pot,
                transaction,
                &mut vec![],
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_contract_fee_pot".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// [`Drive::fetch_contract_fee_pot`] with the fee of the reads, so that consensus
    /// validation can bill them.
    ///
    /// # Parameters
    ///
    /// * `contract_id`: The contract the pot belongs to.
    /// * `pot`: The pot.
    /// * `epoch`: The epoch the fee is priced for.
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok((FeeResult, ContractFeePotState))` with the fee and the pot.
    /// * `Err(Error)` when the version is unknown or a read fails.
    pub fn fetch_contract_fee_pot_with_fee(
        &self,
        contract_id: Identifier,
        pot: ContractFeePot,
        epoch: &Epoch,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(FeeResult, ContractFeePotState), Error> {
        match platform_version
            .drive
            .methods
            .contract
            .fee_pots
            .fetch_contract_fee_pot
        {
            0 => {
                let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
                let state = self.fetch_contract_fee_pot_add_to_operations_v0(
                    contract_id,
                    pot,
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
                Ok((fee, state))
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_contract_fee_pot_with_fee".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
