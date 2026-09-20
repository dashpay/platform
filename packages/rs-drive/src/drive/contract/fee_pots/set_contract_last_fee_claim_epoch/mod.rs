mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::epoch::EpochIndex;
use dpp::data_contract::document_type::action_fees::ContractFeePot;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerInformation;
use std::collections::HashMap;

impl Drive {
    /// The operations that record the epoch one of a contract's fee pots was claimed in. A pot
    /// is claimed at most once per epoch, and this item is what the next claim is judged
    /// against.
    ///
    /// # Parameters
    ///
    /// * `contract_id`: The contract the pot belongs to.
    /// * `pot`: The pot.
    /// * `epoch_index`: The epoch of the claim.
    /// * `estimated_costs_only_with_layer_info`: The estimation map, when only estimating.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<LowLevelDriveOperation>)` with the write.
    /// * `Err(Error)` when the version is unknown.
    pub fn set_contract_last_fee_claim_epoch_operations(
        &self,
        contract_id: Identifier,
        pot: ContractFeePot,
        epoch_index: EpochIndex,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version
            .drive
            .methods
            .contract
            .fee_pots
            .set_contract_last_fee_claim_epoch
        {
            0 => self.set_contract_last_fee_claim_epoch_operations_v0(
                contract_id,
                pot,
                epoch_index,
                estimated_costs_only_with_layer_info,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "set_contract_last_fee_claim_epoch_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
