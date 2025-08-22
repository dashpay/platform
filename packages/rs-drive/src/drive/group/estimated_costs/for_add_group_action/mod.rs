mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::data_contract::GroupContractPosition;
use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerInformation;
use std::collections::HashMap;

impl Drive {
    /// Adds estimation costs for adding a group action based on the specified contract and version.
    ///
    /// This function selects the appropriate version of cost estimation for adding a group action, depending on the
    /// `drive_version` provided. It then delegates the cost estimation to the relevant method for that version.
    ///
    /// Currently, it supports version `0` which uses the `add_estimation_costs_for_add_group_action_v0` method to estimate
    /// the computational costs associated with adding a group action. If an unsupported version is passed in the `drive_version`,
    /// the function will return an error indicating a version mismatch.
    ///
    /// # Parameters
    ///
    /// - `contract_id`: The unique identifier of the contract for which the group action is being added.
    ///   This is used to form paths and calculate relevant costs.
    /// - `group_contract_position`: The position of the group contract within the system, influencing the paths used for cost estimation.
    /// - `action_id`: An optional identifier for the specific action being added. If provided, the estimation for this action and its signers is included.
    /// - `estimated_costs_only_with_layer_info`: A mutable reference to a `HashMap` where the estimated costs for updating
    ///   the group action and its associated data will be stored. The keys in the map represent paths to the trees,
    ///   and the values represent the estimated computational costs for those trees.
    /// - `drive_version`: The version of the drive being used. It determines which method to use for cost estimation.
    ///   This is a versioned API, so different versions may have different cost estimation methods.
    ///
    /// # Return Value
    ///
    /// - Returns `Ok(())` if the version of the `drive_version` matches the supported versions and the estimation is successfully added.
    /// - Returns an error (`Error::Drive(DriveError::UnknownVersionMismatch)`) if the version is not supported.
    ///
    /// # Logic Breakdown
    ///
    /// - **Version Selection**: The function first checks the version provided in `drive_version`. If the version is `0`,
    ///   it uses the `add_estimation_costs_for_add_group_action_v0` method for cost estimation.
    /// - **Version Mismatch**: If the version does not match the supported versions, an error is returned with details
    ///   about the expected versions and the received version.
    ///
    /// # Errors
    ///
    /// - `Error::Drive(DriveError::UnknownVersionMismatch)` will be returned if the `drive_version` does not match the
    ///   known versions supported for this method (currently only version `0`).
    pub(crate) fn add_estimation_costs_for_add_group_action(
        contract_id: [u8; 32],
        group_contract_position: GroupContractPosition,
        action_id: Option<[u8; 32]>,
        also_add_closed_tree: bool,
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match drive_version
            .methods
            .group
            .cost_estimation
            .for_add_group_action
        {
            0 => {
                Self::add_estimation_costs_for_add_group_action_v0(
                    contract_id,
                    group_contract_position,
                    action_id,
                    also_add_closed_tree,
                    estimated_costs_only_with_layer_info,
                );
                Ok(())
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_estimation_costs_for_add_group_action".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
