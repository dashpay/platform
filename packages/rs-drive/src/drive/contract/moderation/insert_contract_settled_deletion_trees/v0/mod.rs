use crate::drive::contract::moderation::types::ContractDocumentRecords;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::storage_flags::StorageFlags;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// The tree of all the approvals and the tree of each type: only a contract's insertion
    /// creates them, so the tree above the types' is always created with them.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn insert_contract_settled_deletion_trees_operations_v0(
        &self,
        contract_id: [u8; 32],
        document_type_names: &[&str],
        storage_flags: Option<&StorageFlags>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        self.insert_document_record_trees_operations(
            contract_id,
            ContractDocumentRecords::SettledDeletions,
            true,
            document_type_names,
            storage_flags,
            estimated_costs_only_with_layer_info,
            transaction,
            batch_operations,
            platform_version,
        )
    }
}
