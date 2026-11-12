use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::estimated_layers::EstimatedLayers;
use crate::util::object_size_info::DocumentAndContractInfo;
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;
use std::collections::BTreeSet;

impl Drive {
    /// Updates a document.
    pub(in crate::drive::document::update) fn update_document_for_contract_apply_and_add_to_operations_v0(
        &self,
        document_and_contract_info: DocumentAndContractInfo,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: Option<EstimatedLayers>,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        self.update_document_for_contract_apply_and_add_to_operations_v0_body(
            document_and_contract_info,
            block_info,
            estimated_costs_only_with_layer_info,
            None,
            transaction,
            drive_operations,
            platform_version,
        )
    }

    /// Generation 0, applying its batch with `owner_handover` (see
    /// [`GroveDbOpBatch`](crate::util::batch::GroveDbOpBatch)): `None` is generation 0 itself.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::drive::document::update) fn update_document_for_contract_apply_and_add_to_operations_v0_body(
        &self,
        document_and_contract_info: DocumentAndContractInfo,
        block_info: &BlockInfo,
        mut estimated_costs_only_with_layer_info: Option<EstimatedLayers>,
        owner_handover: Option<BTreeSet<[u8; 32]>>,
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
        let batch_operations = self.update_document_for_contract_operations(
            document_and_contract_info,
            block_info,
            &mut None,
            &mut estimated_costs_only_with_layer_info,
            transaction,
            platform_version,
        )?;
        self.apply_batch_low_level_drive_operations_with_owner_handover(
            estimated_costs_only_with_layer_info,
            transaction,
            batch_operations,
            owner_handover,
            drive_operations,
            &platform_version.drive,
        )?;
        if let Some(owned_transaction) = owned_transaction {
            self.commit_transaction(owned_transaction, &platform_version.drive)?;
        }
        Ok(())
    }
}
