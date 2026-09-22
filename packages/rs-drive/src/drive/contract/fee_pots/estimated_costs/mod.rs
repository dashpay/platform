mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::data_contract::document_type::action_fees::ContractFeePot;
use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerInformation;
use std::collections::HashMap;

impl Drive {
    /// Adds the layer information of a write to one of a contract's fee pots.
    ///
    /// # Parameters
    ///
    /// * `pot`: The pot written to.
    /// * `estimated_costs_only_with_layer_info`: The estimation map.
    /// * `drive_version`: The drive version.
    ///
    /// # Returns
    ///
    /// * `Ok(())` once the layers are described.
    /// * `Err(Error)` when the version is unknown.
    pub(crate) fn add_estimation_costs_for_contract_fee_pot_update(
        pot: ContractFeePot,
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match drive_version
            .methods
            .contract
            .fee_pots
            .add_estimation_costs_for_contract_fee_pot_update
        {
            0 => {
                Self::add_estimation_costs_for_contract_fee_pot_update_v0(
                    pot,
                    estimated_costs_only_with_layer_info,
                );
                Ok(())
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_estimation_costs_for_contract_fee_pot_update".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
