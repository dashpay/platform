use std::collections::HashMap;

use crate::drive::group::paths::{group_active_action_path, ACTION_INFO_KEY};
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::DirectQueryType;
use crate::util::grove_operations::QueryTarget::QueryTargetValue;
use dpp::data_contract::GroupContractPosition;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg, TreeType};

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
    pub(super) fn fetch_action_id_info_keep_serialized_v0(
        &self,
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        action_id: Identifier,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let group_contract_position_bytes = group_contract_position.to_be_bytes().to_vec();
        // Construct the GroveDB path for the action signers
        let path = group_active_action_path(
            contract_id.as_ref(),
            &group_contract_position_bytes,
            action_id.as_ref(),
        );

        let value = self.grove_get_raw_item(
            (&path).into(),
            ACTION_INFO_KEY,
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )?;

        Ok(value)
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
    // TODO: Is not using
    #[allow(dead_code)]
    pub(super) fn fetch_action_id_info_keep_serialized_and_add_operations_v0(
        &self,
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        action_id: Identifier,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let group_contract_position_bytes = group_contract_position.to_be_bytes().to_vec();
        // Construct the GroveDB path for the action signers
        let path = group_active_action_path(
            contract_id.as_ref(),
            &group_contract_position_bytes,
            action_id.as_ref(),
        );

        // no estimated_costs_only_with_layer_info, means we want to apply to state
        let direct_query_type = if estimated_costs_only_with_layer_info.is_none() {
            DirectQueryType::StatefulDirectQuery
        } else {
            DirectQueryType::StatelessDirectQuery {
                in_tree_type: TreeType::NormalTree,
                query_target: QueryTargetValue(8),
            }
        };

        let value = self.grove_get_raw_item(
            (&path).into(),
            ACTION_INFO_KEY,
            direct_query_type,
            transaction,
            drive_operations,
            &platform_version.drive,
        )?;

        Ok(value)
    }
}
