use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::data_contract::GroupContractPosition;
use dpp::group::group_action_status::GroupActionStatus;
use dpp::identifier::Identifier;
use dpp::prelude::StartAtIncluded;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

mod v0;
impl Drive {
    /// Generates a proof of active action information within a specific group of a contract.
    ///
    /// This method produces a cryptographic proof that validates and retrieves active action information
    /// associated with a specific group in the given contract based on the `action_status`. The proof can be limited to a subset of actions
    /// based on the `start_action_id` and `limit` parameters. It supports multiple versions for backward
    /// compatibility and forwards the request to the appropriate versioned implementation.
    ///
    /// # Arguments
    /// - `contract_id`: The identifier of the contract containing the group.
    /// - `group_contract_position`: The position of the group within the contract whose actions are to be proven.
    /// - `action_status`: The status of the group actions to prove.
    /// - `start_action_id`: An optional starting action ID, combined with a [`StartAtIncluded`] flag to specify whether
    ///                      the start position is inclusive.
    /// - `limit`: An optional limit on the number of actions to include in the proof.
    /// - `transaction`: The transaction context for the operation.
    /// - `platform_version`: A reference to the platform version, used to determine the appropriate versioned implementation.
    ///
    /// # Returns
    /// - `Ok(Vec<u8>)`: On success, returns a serialized proof as a vector of bytes.
    /// - `Err(Error)`: If the operation fails, returns an [`Error`] indicating the cause of failure.
    ///
    /// # Errors
    /// - [`Error::Drive(DriveError::UnknownVersionMismatch)`]: If the method is called with an unsupported platform version.
    /// - Any other errors propagated from the versioned implementation.
    #[allow(clippy::too_many_arguments)]
    pub fn prove_action_infos(
        &self,
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        action_status: GroupActionStatus,
        start_action_id: Option<(Identifier, StartAtIncluded)>,
        limit: Option<u16>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version
            .drive
            .methods
            .group
            .prove
            .prove_action_infos
        {
            0 => self.prove_action_infos_v0(
                contract_id,
                group_contract_position,
                action_status,
                start_action_id,
                limit,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_action_infos".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Generates a proof of active action information within a specific group of a contract and adds operations.
    ///
    /// This method extends the functionality of `prove_action_infos` by additionally allowing
    /// operations to be added to a provided `drive_operations` list. It produces a cryptographic proof
    /// of active action information associated with a specific group in the given contract based on the `action_status`, and supports
    /// different versions for backward compatibility.
    ///
    /// # Arguments
    /// - `contract_id`: The identifier of the contract containing the group.
    /// - `group_contract_position`: The position of the group within the contract whose actions are to be proven.
    /// - `action_status`: The status of the group actions to prove.
    /// - `start_action_id`: An optional starting action ID, combined with a [`StartAtIncluded`] flag to specify whether
    ///                      the start position is inclusive.
    /// - `limit`: An optional limit on the number of actions to include in the proof.
    /// - `transaction`: The transaction context for the operation.
    /// - `drive_operations`: A mutable reference to a vector where additional low-level operations can be appended.
    /// - `platform_version`: A reference to the platform version, used to determine the appropriate versioned implementation.
    ///
    /// # Returns
    /// - `Ok(Vec<u8>)`: On success, returns a serialized proof as a vector of bytes.
    /// - `Err(Error)`: If the operation fails, returns an [`Error`] indicating the cause of failure.
    ///
    /// # Errors
    /// - [`Error::Drive(DriveError::UnknownVersionMismatch)`]: If the method is called with an unsupported platform version.
    /// - Any other errors propagated from the versioned implementation.
    #[allow(clippy::too_many_arguments)]
    // TODO: Is not using
    #[allow(dead_code)]
    pub(crate) fn prove_action_infos_operations(
        &self,
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        action_status: GroupActionStatus,
        start_action_id: Option<(Identifier, StartAtIncluded)>,
        limit: Option<u16>,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version
            .drive
            .methods
            .group
            .prove
            .prove_action_infos
        {
            0 => self.prove_action_infos_operations_v0(
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
                method: "prove_action_infos_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
