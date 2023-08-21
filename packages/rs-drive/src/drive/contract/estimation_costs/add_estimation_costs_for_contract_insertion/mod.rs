mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::data_contract::DataContract;

use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerInformation;
use std::collections::HashMap;

impl Drive {
    /// Adds the estimation costs for a contract insertion
    ///
    /// # Arguments
    ///
    /// * `contract` - A `DataContract` object to be inserted.
    /// * `estimated_costs_only_with_layer_info` - A mutable HashMap reference to be updated with the cost estimations.
    /// * `platform_version` - A `PlatformVersion` object specifying the version of Platform.
    ///
    /// # Errors
    ///
    /// This function will return an error if the version of the Drive is unknown.
    pub fn add_estimation_costs_for_contract_insertion(
        contract: &DataContract,
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .contract
            .costs
            .add_estimation_costs_for_contract_insertion
        {
            0 => {
                Self::add_estimation_costs_for_contract_insertion_v0(
                    contract,
                    estimated_costs_only_with_layer_info,
                    platform_version,
                )?;
                Ok(())
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_estimation_costs_for_contract_insertion".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
