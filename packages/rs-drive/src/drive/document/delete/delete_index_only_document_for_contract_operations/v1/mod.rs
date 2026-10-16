use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::DataContract;
use dpp::document::Document;
use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use dpp::fee::fee_result::FeeResult;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Generation 1 differs from generation 0 in one thing: when the caller supplies no
    /// transaction and the operations are applied, the write and its pricing share one
    /// owned transaction that is committed only after `Drive::calculate_fee` succeeded.
    /// Generation 0 committed its owned transaction and priced afterwards, so from protocol
    /// version 15, where pricing an owner-attributed storage removal without the fee history
    /// is an error, a call passing no history persisted the write and then failed. It now
    /// fails before anything is written. With a caller transaction nothing is committed by
    /// Drive in either generation.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn delete_index_only_document_for_contract_v1(
        &self,
        document: Document,
        contract: &DataContract,
        document_type: DocumentTypeRef,
        block_info: BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
        previous_fee_versions: Option<&CachedEpochIndexFeeVersions>,
    ) -> Result<FeeResult, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        let mut estimated_costs_only_with_layer_info = if apply {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };

        // With no caller transaction, TTL preparation (direct drainage
        // writes) inside the operations builder and the apply below would
        // each commit on their own: a tuple failing the row-commitment gate
        // after preparation would leave drained buckets committed. Span
        // both with one owned transaction.
        let owned_transaction =
            (apply && transaction.is_none()).then(|| self.grove.start_transaction());
        let transaction = owned_transaction.as_ref().or(transaction);
        let batch_operations = self.delete_index_only_document_for_contract_operations(
            document,
            contract,
            document_type,
            None,
            &mut estimated_costs_only_with_layer_info,
            block_info.time_ms,
            transaction,
            platform_version,
        )?;

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
            previous_fee_versions,
        )?;
        if let Some(owned_transaction) = owned_transaction {
            self.commit_transaction(owned_transaction, &platform_version.drive)?;
        }
        Ok(fees)
    }
}
