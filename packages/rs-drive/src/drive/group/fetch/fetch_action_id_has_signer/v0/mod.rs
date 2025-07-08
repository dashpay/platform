use crate::drive::group::paths::group_action_signers_path;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::DirectQueryType;
use crate::util::grove_operations::QueryTarget::QueryTargetValue;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::GroupContractPosition;
use dpp::fee::fee_result::FeeResult;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::{TransactionArg, TreeType};

impl Drive {
    /// v0 implementation of fetching the signers' power for a given action ID within a group contract.
    ///
    /// This function retrieves the signers' power associated with a specific action ID in a group contract.
    /// It constructs the appropriate GroveDB path, fetches the relevant data, deserializes it, and
    /// calculates any applicable fees based on the provided epoch.
    ///
    /// # Parameters
    /// * `contract_id` - The identifier of the contract.
    /// * `group_contract_position` - The position of the group contract within the data contract.
    /// * `action_id` - The identifier of the action whose signers' power is to be fetched.
    /// * `apply` - A boolean flag indicating whether to apply certain operations during the fetch.
    /// * `transaction` - The GroveDB transaction argument for executing the fetch operation.
    /// * `platform_version` - The current platform version, used to ensure compatibility.
    ///
    /// # Returns
    /// * `Ok(Some(Arc<DataContractFetchInfo>))` if the signers' power is successfully fetched.
    /// * `Ok(None)` if the signers' power does not exist.
    /// * `Err(Error)` if an error occurs during the fetch operation.
    ///
    /// # Errors
    /// * `Error::Drive(DriveError::CorruptedContractPath)` if the fetched path does not refer to a valid sum item.
    /// * `Error::Drive(DriveError::CorruptedCodeExecution)` if the element type is unexpected.
    /// * `Error::GroveDB` for any underlying GroveDB errors.
    pub(super) fn fetch_action_id_has_signer_v0(
        &self,
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        action_id: Identifier,
        signer_id: Identifier,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        let group_contract_position_bytes = group_contract_position.to_be_bytes().to_vec();
        // Construct the GroveDB path for the action signers
        let path = group_action_signers_path(
            contract_id.as_slice(),
            &group_contract_position_bytes,
            action_id.as_slice(),
        );

        let value = self.grove_has_raw(
            (&path).into(),
            signer_id.as_slice(),
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )?;

        Ok(value)
    }

    /// Fetches the Identity's balance from the backing store
    /// Passing apply as false get the estimated cost instead
    #[allow(clippy::too_many_arguments)]
    pub(super) fn fetch_action_id_has_signer_with_costs_v0(
        &self,
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        action_id: Identifier,
        signer_id: Identifier,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(bool, FeeResult), Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        let value = self.fetch_action_id_has_signer_and_add_operations_v0(
            contract_id,
            group_contract_position,
            action_id,
            signer_id,
            false,
            transaction,
            &mut drive_operations,
            platform_version,
        )?;
        let fees = Drive::calculate_fee(
            None,
            Some(drive_operations),
            &block_info.epoch,
            self.config.epochs_per_era,
            platform_version,
            None,
        )?;
        Ok((value, fees))
    }

    /// v0 implementation of fetching the signers' power for a given action ID within a group contract and adding related operations.
    ///
    /// This function not only fetches the signers' power but also appends necessary low-level drive operations based on the provided epoch.
    /// It ensures that fees are calculated and added to the `drive_operations` vector when applicable.
    ///
    /// # Parameters
    /// * `contract_id` - The identifier of the contract.
    /// * `group_contract_position` - The position of the group contract within the data contract.
    /// * `action_id` - The identifier of the action whose signers' power is to be fetched.
    /// * `epoch` - An optional reference to an `Epoch` object. If provided, fees will be calculated based on the epoch.
    /// * `transaction` - The GroveDB transaction argument for executing the fetch operation.
    /// * `drive_operations` - A mutable reference to a vector where low-level drive operations and fees will be appended.
    /// * `platform_version` - The current platform version, used to ensure compatibility.
    ///
    /// # Returns
    /// * `Ok(Some(Arc<DataContractFetchInfo>))` if the signers' power is successfully fetched.
    /// * `Ok(None)` if the signers' power does not exist.
    /// * `Err(Error)` if an error occurs during the fetch operation.
    ///
    /// # Errors
    /// * `Error::Drive(DriveError::CorruptedContractPath)` if the fetched path does not refer to a valid sum item.
    /// * `Error::Drive(DriveError::CorruptedCodeExecution)` if the element type is unexpected.
    /// * `Error::Drive(DriveError::NotSupportedPrivate)` if stateful batch insertions are attempted.
    /// * `Error::GroveDB` for any underlying GroveDB errors.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn fetch_action_id_has_signer_and_add_operations_v0(
        &self,
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        action_id: Identifier,
        signer_id: Identifier,
        estimate_costs_only: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        let group_contract_position_bytes = group_contract_position.to_be_bytes().to_vec();
        // Construct the GroveDB path for the action signers
        let path = group_action_signers_path(
            contract_id.as_slice(),
            &group_contract_position_bytes,
            action_id.as_slice(),
        );

        // no estimated_costs_only_with_layer_info, means we want to apply to state
        let direct_query_type = if estimate_costs_only {
            DirectQueryType::StatelessDirectQuery {
                in_tree_type: TreeType::SumTree,
                query_target: QueryTargetValue(8),
            }
        } else {
            DirectQueryType::StatefulDirectQuery
        };

        let value = self.grove_has_raw(
            (&path).into(),
            signer_id.as_slice(),
            direct_query_type,
            transaction,
            drive_operations,
            &platform_version.drive,
        )?;

        Ok(value)
    }
}
