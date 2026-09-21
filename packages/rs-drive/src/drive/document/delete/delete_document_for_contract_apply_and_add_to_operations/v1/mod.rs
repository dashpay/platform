use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::DataContract;

use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Deletes a document and adds the operations through the entry point that
    /// carries only the block time.
    ///
    /// Generation 1 records a lifecycle entry for a keep-history document. That
    /// entry names the deletion time, which this signature carries, and credits
    /// the record's bytes to the deleter, which it does not: an entry written
    /// through here belongs to nobody, as with the fee-applying wrappers, and
    /// refunds nobody when erased.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn delete_document_for_contract_apply_and_add_to_operations_v1(
        &self,
        document_id: Identifier,
        contract: &DataContract,
        document_type_name: &str,
        estimated_costs_only_with_layer_info: Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        block_time_ms: u64,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        self.delete_document_for_contract_apply_and_add_to_operations_with_lifecycle_v1(
            document_id,
            contract,
            document_type_name,
            &BlockInfo::default_with_time(block_time_ms),
            None,
            estimated_costs_only_with_layer_info,
            block_time_ms,
            transaction,
            drive_operations,
            platform_version,
        )
    }

    /// Deletes a document.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn delete_document_for_contract_apply_and_add_to_operations_with_lifecycle_v1(
        &self,
        document_id: Identifier,
        contract: &DataContract,
        document_type_name: &str,
        block_info: &BlockInfo,
        deleter_id: Option<Identifier>,
        mut estimated_costs_only_with_layer_info: Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        block_time_ms: u64,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        // With no caller transaction, TTL preparation (direct drainage
        // writes) and the apply below would each commit on their own; span
        // them with one owned transaction so the write is all-or-nothing.
        let owned_transaction = (estimated_costs_only_with_layer_info.is_none()
            && transaction.is_none())
        .then(|| self.grove.start_transaction());
        let transaction = owned_transaction.as_ref().or(transaction);
        let batch_operations = self
            .delete_document_for_contract_with_named_type_operations_with_lifecycle(
                document_id,
                contract,
                document_type_name,
                block_info,
                deleter_id,
                None,
                &mut estimated_costs_only_with_layer_info,
                block_time_ms,
                transaction,
                platform_version,
            )?;
        self.apply_batch_low_level_drive_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            batch_operations,
            drive_operations,
            &platform_version.drive,
        )?;
        if let Some(owned_transaction) = owned_transaction {
            self.commit_transaction(owned_transaction, &platform_version.drive)?;
        }
        Ok(())
    }
}
