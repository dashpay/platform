use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::data_contract::GroupContractPosition;
use dpp::group::group_action::GroupAction;
use dpp::identifier::Identifier;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

mod v0;

impl Drive {
    /// Fetches the `GroupAction` for the given action ID and group contract position.
    ///
    /// This function queries the GroveDB to fetch the `GroupAction` associated with a specific
    /// group contract position and action ID. The method selects the appropriate version of
    /// `fetch_action_id_info` based on the `platform_version` provided.
    ///
    /// # Parameters
    /// - `contract_id`: The identifier of the contract that the action belongs to.
    /// - `group_contract_position`: The position of the group contract in the data structure.
    /// - `action_id`: The identifier of the action whose `GroupAction` is being fetched.
    /// - `transaction`: The transaction argument used for the query.
    /// - `platform_version`: The version of the platform that determines the correct method version.
    ///
    /// # Returns
    /// - `Ok(GroupAction)`: The `GroupAction` for the specified action ID and contract position.
    /// - `Err(Error)`: If an error occurs, a generic error is returned.
    ///
    /// # Errors
    /// - `DriveError::UnknownVersionMismatch`: If the `platform_version` does not match any known versions.
    pub fn fetch_active_action_info(
        &self,
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        action_id: Identifier,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<GroupAction, Error> {
        match platform_version
            .drive
            .methods
            .group
            .fetch
            .fetch_active_action_info
        {
            0 => self.fetch_active_action_info_v0(
                contract_id,
                group_contract_position,
                action_id,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_active_action_info".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Fetches the `GroupAction` and adds corresponding operations to the drive for the given action ID and group contract position.
    ///
    /// This function is similar to `fetch_action_id_info` but also adds operations to the drive for state changes or queries.
    /// Additionally, it supports cost estimation by interacting with the layer information if provided.
    ///
    /// # Parameters
    /// - `contract_id`: The identifier of the contract that the action belongs to.
    /// - `group_contract_position`: The position of the group contract in the data structure.
    /// - `action_id`: The identifier of the action whose `GroupAction` is being fetched.
    /// - `estimated_costs_only_with_layer_info`: A mutable reference to an optional `HashMap` containing
    ///   layer information used for cost estimation.
    /// - `transaction`: The transaction argument used for the query.
    /// - `drive_operations`: A mutable reference to a vector that stores low-level drive operations.
    /// - `platform_version`: The version of the platform that determines the correct method version.
    ///
    /// # Returns
    /// - `Ok(Option<GroupAction>)`: The `GroupAction` for the specified action ID and contract position, along with any added operations.
    ///   We will always get back a value unless we are approximating without state.
    /// - `Err(Error)`: If an error occurs, a generic error is returned.
    ///
    /// # Errors
    /// - `DriveError::UnknownVersionMismatch`: If the `platform_version` does not match any known versions.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn fetch_active_action_info_and_add_operations(
        &self,
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        action_id: Identifier,
        approximate_without_state_for_costs: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<GroupAction>, Error> {
        match platform_version
            .drive
            .methods
            .group
            .fetch
            .fetch_active_action_info
        {
            0 => self.fetch_active_action_info_and_add_operations_v0(
                contract_id,
                group_contract_position,
                action_id,
                approximate_without_state_for_costs,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_active_action_info_and_add_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
