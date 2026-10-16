use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::CalculatedCostOperation;
use crate::util::storage_flags::StorageFlags;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::prelude::DataContract;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::borrow::Cow;
use std::collections::HashMap;

impl Drive {
    /// Generation 1 differs from the previous one in one thing: when the caller supplies no
    /// transaction and the operations are applied, the write and its pricing share one
    /// owned transaction that is committed only after `Drive::calculate_fee` succeeded.
    /// Earlier generations applied the batch (committing it on its own without a caller
    /// transaction) and priced it afterwards, so from protocol version 15, where pricing an
    /// owner-attributed storage removal without the fee history is an error, a call passing
    /// no history persisted the write and then failed. It now fails before anything is
    /// written. With a caller transaction nothing is committed by Drive in either
    /// generation.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn apply_contract_with_serialization_v1(
        &self,
        contract: &DataContract,
        contract_serialization: Vec<u8>,
        block_info: BlockInfo,
        apply: bool,
        storage_flags: Option<Cow<StorageFlags>>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        let owned_transaction =
            (apply && transaction.is_none()).then(|| self.grove.start_transaction());
        let transaction = owned_transaction.as_ref().or(transaction);
        let mut cost_operations = vec![];
        let mut estimated_costs_only_with_layer_info = if apply {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };
        let batch_operations = self.apply_contract_with_serialization_operations(
            contract,
            contract_serialization,
            &block_info,
            &mut estimated_costs_only_with_layer_info,
            storage_flags,
            transaction,
            platform_version,
        )?;
        let fetch_cost = LowLevelDriveOperation::combine_cost_operations(&batch_operations);
        self.apply_batch_low_level_drive_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            batch_operations,
            &mut cost_operations,
            &platform_version.drive,
        )?;
        cost_operations.push(CalculatedCostOperation(fetch_cost));

        // A pricing error drops the owned transaction with everything it wrote.
        let fees = Drive::calculate_fee(
            None,
            Some(cost_operations),
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
}
