use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::data_contract::group::Group;
use dpp::data_contract::GroupContractPosition;
use dpp::identifier::Identifier;
use dpp::prelude::StartAtIncluded;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

mod v0;
impl Drive {
    /// Fetches the `Group` for the given contract.
    ///
    /// This function queries the GroveDB to fetch the potentially many `Group`s associated with a
    /// specific contract. The method selects the appropriate version of
    /// `fetch_group_infos` based on the `platform_version` provided.
    ///
    /// # Parameters
    /// - `contract_id`: The identifier of the contract that the action belongs to.
    /// - `transaction`: The transaction argument used for the query.
    /// - `platform_version`: The version of the platform that determines the correct method version.
    ///
    /// # Returns
    /// - `Ok(Group)`: The `Group` for the specified action ID and contract position.
    /// - `Err(Error)`: If an error occurs, a generic error is returned.
    ///
    /// # Errors
    /// - `DriveError::UnknownVersionMismatch`: If the `platform_version` does not match any known versions.
    pub fn fetch_group_infos(
        &self,
        contract_id: Identifier,
        start_group_contract_position: Option<(GroupContractPosition, StartAtIncluded)>,
        limit: Option<u16>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<GroupContractPosition, Group>, Error> {
        match platform_version.drive.methods.group.fetch.fetch_group_infos {
            0 => self.fetch_group_infos_v0(
                contract_id,
                start_group_contract_position,
                limit,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_group_infos".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Fetches the `Group` and adds corresponding operations to the drive for the given action ID and group contract position.
    ///
    /// This function is similar to `fetch_group_info` but also adds operations to the drive for state changes or queries.
    /// Additionally, it supports cost estimation by interacting with the layer information if provided.
    ///
    /// # Parameters
    /// - `contract_id`: The identifier of the contract that the action belongs to.
    /// - `group_contract_position`: The position of the group contract in the data structure.
    /// - `transaction`: The transaction argument used for the query.
    /// - `drive_operations`: A mutable reference to a vector that stores low-level drive operations.
    /// - `platform_version`: The version of the platform that determines the correct method version.
    ///
    /// # Returns
    /// - `Ok(Group)`: The `Group` for the specified contract ID and contract position, along with any added operations.
    /// - `Err(Error)`: If an error occurs, a generic error is returned.
    ///
    /// # Errors
    /// - `DriveError::UnknownVersionMismatch`: If the `platform_version` does not match any known versions.
    // TODO: Is not using
    #[allow(dead_code)]
    pub(crate) fn fetch_group_infos_operations(
        &self,
        contract_id: Identifier,
        start_group_contract_position: Option<(GroupContractPosition, StartAtIncluded)>,
        limit: Option<u16>,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<GroupContractPosition, Group>, Error> {
        match platform_version.drive.methods.group.fetch.fetch_group_infos {
            0 => self.fetch_group_infos_operations_v0(
                contract_id,
                start_group_contract_position,
                limit,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_group_infos_and_add_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
