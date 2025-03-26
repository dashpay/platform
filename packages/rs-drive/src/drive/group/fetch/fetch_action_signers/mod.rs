use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::data_contract::group::GroupMemberPower;
use dpp::data_contract::GroupContractPosition;
use dpp::group::group_action_status::GroupActionStatus;
use dpp::identifier::Identifier;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

mod v0;

impl Drive {
    /// Fetches the signers and their respective powers for a specific action in a group.
    ///
    /// This method retrieves the list of signers for an action associated with a given contract,
    /// group, and action status. It selects the appropriate version of the method based on the
    /// provided platform version.
    ///
    /// # Arguments
    ///
    /// * `contract_id` - The identifier of the contract associated with the action.
    /// * `group_contract_position` - The position of the group within the contract.
    /// * `action_status` - The status of the action (e.g., active or closed).
    /// * `action_id` - The identifier of the action for which to fetch signers.
    /// * `transaction` - An optional transaction argument for database operations.
    /// * `platform_version` - The platform version to determine which method version to call.
    ///
    /// # Returns
    ///
    /// A `Result` containing a `BTreeMap` where the keys are the signer identifiers and the values
    /// are their respective powers, or an `Error` if the operation fails.
    ///
    /// # Errors
    ///
    /// This method returns an `Error` if:
    /// * The platform version is unknown.
    /// * An internal issue occurs during the fetching process.
    pub fn fetch_action_signers(
        &self,
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        action_status: GroupActionStatus,
        action_id: Identifier,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<Identifier, GroupMemberPower>, Error> {
        match platform_version
            .drive
            .methods
            .group
            .fetch
            .fetch_action_signers
        {
            0 => self.fetch_action_signers_v0(
                contract_id,
                group_contract_position,
                action_status,
                action_id,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_action_signers".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Fetches the signers and their respective powers for a specific action and adds the
    /// associated operations for database queries.
    ///
    /// This method extends the functionality of `fetch_action_signers` by including additional
    /// database operations that can be executed as part of a larger transaction. The appropriate
    /// version of the method is selected based on the provided platform version.
    ///
    /// # Arguments
    ///
    /// * `contract_id` - The identifier of the contract associated with the action.
    /// * `group_contract_position` - The position of the group within the contract.
    /// * `action_status` - The status of the action (e.g., active or closed).
    /// * `action_id` - The identifier of the action for which to fetch signers.
    /// * `transaction` - An optional transaction argument for database operations.
    /// * `drive_operations` - A mutable vector to which low-level database operations will be added.
    /// * `platform_version` - The platform version to determine which method version to call.
    ///
    /// # Returns
    ///
    /// A `Result` containing a `BTreeMap` where the keys are the signer identifiers and the values
    /// are their respective powers, or an `Error` if the operation fails.
    ///
    /// # Errors
    ///
    /// This method returns an `Error` if:
    /// * The platform version is unknown.
    /// * An internal issue occurs during the fetching process.
    #[allow(clippy::too_many_arguments)]
    // TODO: Is not using
    #[allow(dead_code)]
    pub(crate) fn fetch_action_signers_and_add_operations(
        &self,
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        action_status: GroupActionStatus,
        action_id: Identifier,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<Identifier, GroupMemberPower>, Error> {
        match platform_version
            .drive
            .methods
            .group
            .fetch
            .fetch_action_signers
        {
            0 => self.fetch_action_signers_and_add_operations_v0(
                contract_id,
                group_contract_position,
                action_status,
                action_id,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_action_signers_and_add_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
