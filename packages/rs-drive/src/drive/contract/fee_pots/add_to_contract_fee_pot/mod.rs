mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::balances::credits::Credits;
use dpp::data_contract::document_type::action_fees::ContractFeePot;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// The operations that add `amount` credits to one of a contract's fee pots. The pot is
    /// created by the first credits it receives.
    ///
    /// The caller removes the same credits from an identity's balance in the same batch, so
    /// the sum of all credits is unchanged.
    ///
    /// # Parameters
    ///
    /// * `contract_id`: The contract the pot belongs to.
    /// * `pot`: The pot.
    /// * `amount`: The credits to add.
    /// * `estimated_costs_only_with_layer_info`: The estimation map, when only estimating.
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<LowLevelDriveOperation>)` with the read and the write.
    /// * `Err(Error)` when the version is unknown, a read fails, or the pot would overflow.
    pub fn add_to_contract_fee_pot_operations(
        &self,
        contract_id: Identifier,
        pot: ContractFeePot,
        amount: Credits,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version
            .drive
            .methods
            .contract
            .fee_pots
            .add_to_contract_fee_pot
        {
            0 => self.add_to_contract_fee_pot_operations_v0(
                contract_id,
                pot,
                amount,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_to_contract_fee_pot_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
