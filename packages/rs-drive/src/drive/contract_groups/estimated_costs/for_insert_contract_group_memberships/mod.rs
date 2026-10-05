mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::contract_group::ContractGroupMembership;
use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerInformation;
use std::collections::HashMap;

impl Drive {
    /// Adds the estimated layer information for the forward and backwards entries of a new
    /// contract's contract group memberships.
    ///
    /// # Parameters
    ///
    /// * `contract_id`: The new contract whose memberships are written.
    /// * `memberships`: The memberships, each naming a group and the part of the contract that
    ///   joins it.
    /// * `estimated_costs_only_with_layer_info`: The estimation map the layers are added to.
    /// * `drive_version`: The drive version.
    ///
    /// # Returns
    ///
    /// * `Ok(())` once the layers are added to the map.
    /// * `Err(Error)` when the method version is unknown.
    pub(crate) fn add_estimation_costs_for_insert_contract_group_memberships(
        contract_id: [u8; 32],
        memberships: &[ContractGroupMembership],
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match drive_version
            .methods
            .contract_group
            .cost_estimation
            .for_insert_contract_group_memberships
        {
            0 => {
                Self::add_estimation_costs_for_insert_contract_group_memberships_v0(
                    contract_id,
                    memberships,
                    estimated_costs_only_with_layer_info,
                );
                Ok(())
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_estimation_costs_for_insert_contract_group_memberships".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
