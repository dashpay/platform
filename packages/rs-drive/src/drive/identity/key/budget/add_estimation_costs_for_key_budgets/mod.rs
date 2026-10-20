mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerInformation;
use std::collections::HashMap;

impl Drive {
    /// Adds the estimated layer information of an identity's key budgets subtree.
    ///
    /// The layers above it (the root, the identities tree and the identity itself) are the ones
    /// every key insertion already registers, so they are left to the caller.
    ///
    /// # Parameters
    /// - `identity_id`: The identity the budgets belong to.
    /// - `estimated_costs_only_with_layer_info`: The map the layer information is added to.
    /// - `drive_version`: The drive version selecting the implementation.
    ///
    /// # Returns
    /// - `Ok(())`, or a version error when the key budgets subtree is not active.
    pub(crate) fn add_estimation_costs_for_key_budgets(
        identity_id: [u8; 32],
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match drive_version
            .methods
            .identity
            .keys
            .budget
            .add_estimation_costs_for_key_budgets
        {
            Some(0) => {
                Self::add_estimation_costs_for_key_budgets_v0(
                    identity_id,
                    estimated_costs_only_with_layer_info,
                );
                Ok(())
            }
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_estimation_costs_for_key_budgets".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "add_estimation_costs_for_key_budgets".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}
