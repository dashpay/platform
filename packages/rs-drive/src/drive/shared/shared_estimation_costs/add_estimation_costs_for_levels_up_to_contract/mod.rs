mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::util::estimated_layers::EstimatedLayers;
use dpp::version::drive_versions::DriveVersion;

impl Drive {
    /// This function calls the versioned `add_estimation_costs_for_levels_up_to_contract_v0`
    /// function based on the version provided in the `DriveVersion` parameter. It returns an error if the
    /// version doesn't match any existing versioned functions.
    ///
    /// # Parameters
    /// - `estimated_costs_only_with_layer_info`: A mutable reference to a `HashMap` that holds the estimated layer information.
    /// - `drive_version`: A reference to the `DriveVersion` object that specifies the version of the function to call.
    pub(in crate::drive) fn add_estimation_costs_for_levels_up_to_contract(
        estimated_costs_only_with_layer_info: &mut EstimatedLayers,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match drive_version
            .methods
            .estimated_costs
            .add_estimation_costs_for_levels_up_to_contract
        {
            0 => {
                Self::add_estimation_costs_for_levels_up_to_contract_v0(
                    estimated_costs_only_with_layer_info,
                );
                Ok(())
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_estimation_costs_for_levels_up_to_contract".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
