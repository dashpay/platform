use crate::drive::contract::moderation::types::estimated_entry_value_size;
use crate::drive::contract::paths::contract_moderation_list_path;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::BatchDeleteApplyType;
use crate::util::storage_flags::StorageFlags;
use crate::util::type_constants::DEFAULT_HASH_SIZE_U32;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::config::moderation::ContractModerationList;
use dpp::fee::fee_result::FeeResult;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use grovedb::{MaybeTree, TreeType};
use std::collections::HashMap;

impl Drive {
    /// Generation 1 differs from generation 0 in one thing: when the caller supplies no
    /// transaction and the operations are applied, the write and its pricing share one
    /// owned transaction that is committed only after `Drive::calculate_fee` succeeded.
    /// Generation 0 applied the batch (committing it on its own without a caller
    /// transaction) and priced it afterwards, so from protocol version 15, where pricing an
    /// owner-attributed storage removal without the fee history is an error, this wrapper
    /// persisted the write and then failed. It still passes no fee history, so a call that
    /// frees moderator-flagged bytes still fails at protocol version 15; it now fails before
    /// anything is written. Production applies moderation through `apply_drive_operations`,
    /// which forwards the block's history. With a caller transaction nothing is committed
    /// by Drive in either generation.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn remove_contract_warnings_v1(
        &self,
        contract_id: Identifier,
        identity_id: Identifier,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        let owned_transaction =
            (apply && transaction.is_none()).then(|| self.grove.start_transaction());
        let transaction = owned_transaction.as_ref().or(transaction);

        let mut estimated_costs_only_with_layer_info = if apply {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };

        let batch_operations = self.remove_contract_warnings_operations_v1(
            contract_id,
            identity_id,
            block_info,
            &mut estimated_costs_only_with_layer_info,
            transaction,
            platform_version,
        )?;

        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        self.apply_batch_low_level_drive_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            batch_operations,
            &mut drive_operations,
            &platform_version.drive,
        )?;

        // A pricing error drops the owned transaction with everything it wrote.
        let fees = Drive::calculate_fee(
            None,
            Some(drive_operations),
            &block_info.epoch,
            self.config.epochs_per_era,
            platform_version,
            None,
        )?;
        if let Some(owned_transaction) = owned_transaction {
            self.commit_transaction(owned_transaction, &platform_version.drive)?;
        }
        Ok(fees)
    }

    /// Deletes the entry. The storage refund follows the entry's own flags, which name the
    /// moderator that wrote it.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn remove_contract_warnings_operations_v1(
        &self,
        contract_id: Identifier,
        identity_id: Identifier,
        _block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let list = ContractModerationList::Warnings;

        let apply_type = if let Some(estimated_costs_only_with_layer_info) =
            estimated_costs_only_with_layer_info
        {
            Drive::add_estimation_costs_for_contract_moderation_entry(
                contract_id.to_buffer(),
                list,
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
            BatchDeleteApplyType::StatelessBatchDelete {
                in_tree_type: TreeType::NormalTree,
                estimated_key_size: DEFAULT_HASH_SIZE_U32,
                estimated_value_size: estimated_entry_value_size(list)
                    + StorageFlags::approximate_size(true, None),
            }
        } else {
            BatchDeleteApplyType::StatefulBatchDelete {
                is_known_to_be_subtree_with_sum: Some(MaybeTree::NotTree),
            }
        };

        let mut batch_operations: Vec<LowLevelDriveOperation> = vec![];
        self.batch_delete(
            (&contract_moderation_list_path(contract_id.as_slice(), list)).into(),
            identity_id.as_slice(),
            apply_type,
            transaction,
            &mut batch_operations,
            &platform_version.drive,
        )?;

        Ok(batch_operations)
    }
}
