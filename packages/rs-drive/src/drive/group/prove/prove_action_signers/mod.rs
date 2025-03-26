use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::data_contract::GroupContractPosition;
use dpp::group::group_action_status::GroupActionStatus;
use dpp::identifier::Identifier;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

mod v0;
impl Drive {
    /// Generates a cryptographic proof of the signers for a specified action in a group within a contract.
    ///
    /// # Arguments
    ///
    /// * `contract_id` - The unique identifier of the contract containing the group.
    /// * `group_contract_position` - The position of the group within the contract.
    /// * `action_status` - The status of the action (e.g., active or closed).
    /// * `action_id` - The unique identifier of the action for which to prove signers.
    /// * `transaction` - The transaction context for this operation.
    /// * `platform_version` - The platform version to use for determining method compatibility.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing:
    /// * `Vec<u8>` - The serialized cryptographic proof of the signers.
    /// * `Error` - An error if the operation fails or if the platform version is unsupported.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    /// * The specified platform version is not supported.
    /// * Internal database or transaction errors occur.
    pub fn prove_action_signers(
        &self,
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        action_status: GroupActionStatus,
        action_id: Identifier,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version
            .drive
            .methods
            .group
            .prove
            .prove_action_signers
        {
            0 => self.prove_action_signers_v0(
                contract_id,
                group_contract_position,
                action_status,
                action_id,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_action_signers".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Generates a cryptographic proof of the signers for a specified action in a group within a contract
    /// and appends the required operations for this proof to the provided list of drive operations.
    ///
    /// # Arguments
    ///
    /// * `contract_id` - The unique identifier of the contract containing the group.
    /// * `group_contract_position` - The position of the group within the contract.
    /// * `action_status` - The status of the action (e.g., active or closed).
    /// * `action_id` - The unique identifier of the action for which to prove signers.
    /// * `transaction` - The transaction context for this operation.
    /// * `drive_operations` - A mutable reference to a list of low-level drive operations to which this operation will be appended.
    /// * `platform_version` - The platform version to use for determining method compatibility.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing:
    /// * `Vec<u8>` - The serialized cryptographic proof of the signers.
    /// * `Error` - An error if the operation fails or if the platform version is unsupported.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if:
    /// * The specified platform version is not supported.
    /// * Internal database or transaction errors occur.
    #[allow(clippy::too_many_arguments)]
    // TODO: Is not using
    #[allow(dead_code)]
    pub(crate) fn prove_action_signers_operations(
        &self,
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        action_status: GroupActionStatus,
        action_id: Identifier,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version
            .drive
            .methods
            .group
            .prove
            .prove_action_signers
        {
            0 => self.prove_action_signers_operations_v0(
                contract_id,
                group_contract_position,
                action_status,
                action_id,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_action_signers_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
