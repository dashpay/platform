mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::util::estimated_layers::EstimatedLayers;
use dpp::version::drive_versions::DriveVersion;

impl Drive {
    /// Adds the estimated layer information for registering a contract group: the root tree,
    /// the `ContractGroups` tree, its `Groups` subtree and the new group's own tree.
    ///
    /// # Parameters
    ///
    /// * `contract_group_id`: The id of the group being registered.
    /// * `estimated_costs_only_with_layer_info`: The estimation map the layers are added to.
    /// * `drive_version`: The drive version.
    ///
    /// # Returns
    ///
    /// * `Ok(())` once the layers are added to the map.
    /// * `Err(Error)` when the method version is unknown.
    pub(crate) fn add_estimation_costs_for_insert_contract_group(
        contract_group_id: [u8; 32],
        estimated_costs_only_with_layer_info: &mut EstimatedLayers,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match drive_version
            .methods
            .contract_group
            .cost_estimation
            .for_insert_contract_group
        {
            0 => {
                Self::add_estimation_costs_for_insert_contract_group_v0(
                    contract_group_id,
                    estimated_costs_only_with_layer_info,
                );
                Ok(())
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_estimation_costs_for_insert_contract_group".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
