use crate::drive::constants::AVERAGE_BALANCE_SIZE;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::{BatchInsertApplyType, QueryTarget};
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg, TreeType};
use platform_version::version::PlatformVersion;
use std::collections::HashMap;

/// Average size of nonce stored with balance (8 bytes for u64)
const AVERAGE_NONCE_SIZE: u32 = 8;

impl Drive {
    /// Version 0 implementation of adding a balance for an address.
    /// This operation directly adds the balance for the address.
    /// The nonce stays the same. If there is no address the nonce becomes 0.
    ///
    /// # Parameters
    /// * `address`: The platform address
    /// * `amount_to_add`: The balance value to add
    /// * `estimated_costs_only_with_layer_info`: If `Some`, only estimates costs without applying.
    /// * `transaction`: The database transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    /// * `Ok(Vec<LowLevelDriveOperation>)` - The operations to add balance.
    /// * `Err(Error)` if the operation fails.
    pub(super) fn add_balance_to_address_operations_v0(
        &self,
        address: PlatformAddress,
        amount_to_add: Credits,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let path = Drive::clear_addresses_path();
        let key = address.to_bytes();

        let apply_type = if let Some(estimated_costs_only_with_layer_info) =
            estimated_costs_only_with_layer_info
        {
            Self::add_estimation_costs_for_address_balance_update(
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
            // CLEAR_ADDRESS_POOL is a sum tree containing ItemWithSumItem elements
            BatchInsertApplyType::StatelessBatchInsert {
                in_tree_type: TreeType::SumTree,
                target: QueryTarget::QueryTargetValue(AVERAGE_NONCE_SIZE + AVERAGE_BALANCE_SIZE),
            }
        } else {
            BatchInsertApplyType::StatefulBatchInsert
        };

        let mut drive_operations = vec![];
        self.batch_keep_item_insert_sum_item_or_add_to_if_already_exists::<[u8; 4]>(
            &path,
            key.as_slice(),
            amount_to_add,
            0u32.to_be_bytes(),
            apply_type,
            transaction,
            &mut drive_operations,
            &platform_version.drive,
        )?;
        Ok(drive_operations)
    }
}
