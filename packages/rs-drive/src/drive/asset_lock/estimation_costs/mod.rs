mod add_estimation_costs_for_adding_asset_lock;

use crate::drive::Drive;

use grovedb::batch::KeyInfoPath;

use grovedb::EstimatedLayerInformation;

use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::drive_versions::DriveVersion;

use std::collections::HashMap;

impl Drive {
    pub(crate) fn add_estimation_costs_for_adding_asset_lock(
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match drive_version
            .methods
            .asset_lock
            .add_estimation_costs_for_adding_asset_lock
        {
            0 => {
                Self::add_estimation_costs_for_adding_asset_lock_v0(
                    estimated_costs_only_with_layer_info,
                );
                Ok(())
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_estimation_costs_for_adding_asset_lock".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
