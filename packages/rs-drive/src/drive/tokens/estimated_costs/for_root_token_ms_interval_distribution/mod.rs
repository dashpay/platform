mod v0;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::prelude::TimestampMillis;
use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerInformation;
use std::collections::HashMap;

impl Drive {
    pub(crate) fn add_estimation_costs_for_root_token_ms_interval_distribution<'a, I>(
        times: I,
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error>
    where
        I: IntoIterator<Item = &'a TimestampMillis>,
    {
        match drive_version
            .methods
            .identity
            .cost_estimation
            .for_root_token_ms_interval_distribution
        {
            0 => {
                Self::add_estimation_costs_for_root_token_ms_interval_distribution_v0(
                    times,
                    estimated_costs_only_with_layer_info,
                );
                Ok(())
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_estimation_costs_for_root_token_ms_interval_distribution".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
