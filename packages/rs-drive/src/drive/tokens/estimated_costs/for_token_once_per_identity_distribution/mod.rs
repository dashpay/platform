mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerInformation;
use std::collections::HashMap;

impl Drive {
    /// Adds the estimated layer information for the once-per-identity distribution trees: the
    /// token distributions root, the once-per-identity root, and, when `token_id` is given, the
    /// token's claims subtree.
    pub(crate) fn add_estimation_costs_for_token_once_per_identity_distribution(
        token_id: Option<[u8; 32]>,
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match drive_version
            .methods
            .identity
            .cost_estimation
            .for_token_once_per_identity_distribution
        {
            0 => {
                Self::add_estimation_costs_for_token_once_per_identity_distribution_v0(
                    token_id,
                    estimated_costs_only_with_layer_info,
                );
                Ok(())
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_estimation_costs_for_token_once_per_identity_distribution".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
