use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::data_contract::GroupContractPosition;
use dpp::identifier::Identifier;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

mod v0;

impl Drive {
    /// Proves the `Group` for the given contract and group contract position.
    ///
    /// This function queries the GroveDB to prove the `Group` associated with a specific contract
    /// and group contract position. The method selects the appropriate version of
    /// `prove_group_info` based on the `platform_version` provided.
    ///
    /// # Parameters
    /// - `contract_id`: The identifier of the contract that the action belongs to.
    /// - `group_contract_position`: The position of the group contract in the data structure.
    /// - `transaction`: The transaction argument used for the query.
    /// - `platform_version`: The version of the platform that determines the correct method version.
    ///
    /// # Returns
    /// - `Ok(Group)`: The `Group` for the specified action ID and contract position.
    /// - `Err(Error)`: If an error occurs, a generic error is returned.
    ///
    /// # Errors
    /// - `DriveError::UnknownVersionMismatch`: If the `platform_version` does not match any known versions.
    pub fn prove_group_info(
        &self,
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version.drive.methods.group.prove.prove_group_info {
            0 => self.prove_group_info_v0(
                contract_id,
                group_contract_position,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_group_info".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Proves the `Group` and adds corresponding operations to the drive for the given action ID and group contract position.
    ///
    /// This function is similar to `prove_group_info` but also adds operations to the drive for state changes or queries.
    /// Additionally, it supports cost estimation by interacting with the layer information if provided.
    ///
    /// # Parameters
    /// - `contract_id`: The identifier of the contract that the action belongs to.
    /// - `group_contract_position`: The position of the group contract in the data structure.
    /// - `estimated_costs_only_with_layer_info`: A mutable reference to an optional `HashMap` containing
    ///   layer information used for cost estimation.
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
    pub(crate) fn prove_group_info_operations(
        &self,
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version.drive.methods.group.prove.prove_group_info {
            0 => self.prove_group_info_operations_v0(
                contract_id,
                group_contract_position,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_group_info_and_add_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
