use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::data_contract::GroupContractPosition;
use dpp::group::group_action::GroupAction;
use dpp::group::group_action_status::GroupActionStatus;
use dpp::identifier::Identifier;
use dpp::prelude::StartAtIncluded;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

mod v0;

impl Drive {
    /// Fetches the `GroupAction` for the given action ID and group contract position.
    ///
    /// This function queries the GroveDB to fetch `GroupAction`s associated with a specific
    /// group contract position and `action_status`. The method selects the appropriate version of
    /// `fetch_action_infos` based on the `platform_version` provided.
    ///
    /// # Parameters
    /// - `contract_id`: The identifier of the contract that the action belongs to.
    /// - `group_contract_position`: The position of the group contract in the data structure.
    /// - `action_status`: The status of the group actions to fetch.
    /// - `start_action_id`: An optional starting action ID and inclusion flag.
    /// - `limit`: An optional limit on the number of group actions to fetch.
    /// - `transaction`: The transaction argument used for the query.
    /// - `platform_version`: The version of the platform that determines the correct method version.
    ///
    /// # Returns
    /// - `Ok(GroupAction)`: The `GroupAction` for the specified action ID and contract position.
    /// - `Err(Error)`: If an error occurs, a generic error is returned.
    ///
    /// # Errors
    /// - `DriveError::UnknownVersionMismatch`: If the `platform_version` does not match any known versions.
    #[allow(clippy::too_many_arguments)]
    pub fn fetch_action_infos(
        &self,
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        action_status: GroupActionStatus,
        start_action_id: Option<(Identifier, StartAtIncluded)>,
        limit: Option<u16>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<Identifier, GroupAction>, Error> {
        match platform_version
            .drive
            .methods
            .group
            .fetch
            .fetch_action_infos
        {
            0 => self.fetch_action_infos_v0(
                contract_id,
                group_contract_position,
                action_status,
                start_action_id,
                limit,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_action_infos".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Fetches the `GroupAction` and adds corresponding operations to the drive for the given action ID and group contract position.
    ///
    /// This function is similar to `fetch_action_infos` but also adds operations to the drive for state changes or queries.
    /// It fetches `GroupAction`s based on the specified `action_status`. Additionally, it supports cost estimation by interacting with the layer information if provided.
    ///
    /// # Parameters
    /// - `contract_id`: The identifier of the contract that the action belongs to.
    /// - `group_contract_position`: The position of the group contract in the data structure.
    /// - `action_status`: The status of the group actions to fetch.
    /// - `start_action_id`: An optional starting action ID and inclusion flag.
    /// - `limit`: An optional limit on the number of group actions to fetch.
    /// - `transaction`: The transaction argument used for the query.
    /// - `drive_operations`: A mutable reference to a vector that stores low-level drive operations.
    /// - `platform_version`: The version of the platform that determines the correct method version.
    ///
    /// # Returns
    /// - `Ok(GroupAction)`: The `GroupAction` for the specified action ID and contract position, along with any added operations.
    /// - `Err(Error)`: If an error occurs, a generic error is returned.
    ///
    /// # Errors
    /// - `DriveError::UnknownVersionMismatch`: If the `platform_version` does not match any known versions.
    // TODO: Is not using
    #[allow(dead_code)]
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn fetch_action_infos_and_add_operations(
        &self,
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        action_status: GroupActionStatus,
        start_action_id: Option<(Identifier, StartAtIncluded)>,
        limit: Option<u16>,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<Identifier, GroupAction>, Error> {
        match platform_version
            .drive
            .methods
            .group
            .fetch
            .fetch_action_infos
        {
            0 => self.fetch_action_infos_and_add_operations_v0(
                contract_id,
                group_contract_position,
                action_status,
                start_action_id,
                limit,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_action_infos_and_add_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
