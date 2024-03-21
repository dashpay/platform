use grovedb::batch::KeyInfoPath;

use grovedb::{EstimatedLayerInformation, TransactionArg};

use dpp::data_contract::document_type::DocumentTypeRef;

use std::collections::HashMap;

use crate::drive::grove_operations::BatchDeleteApplyType::{
    StatefulBatchDelete, StatelessBatchDelete,
};

use crate::drive::Drive;

use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;

use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;

use dpp::version::PlatformVersion;

impl Drive {
    /// Removes the document from primary storage.
    #[inline(always)]
    pub(super) fn remove_document_from_primary_storage_v0(
        &self,
        document_id: [u8; 32],
        document_type: DocumentTypeRef,
        contract_documents_primary_key_path: [&[u8]; 5],
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let apply_type = if estimated_costs_only_with_layer_info.is_some() {
            StatelessBatchDelete {
                is_sum_tree: false,
                estimated_value_size: document_type.estimated_size(platform_version)? as u32,
            }
        } else {
            // we know we are not deleting a subtree
            StatefulBatchDelete {
                is_known_to_be_subtree_with_sum: Some((false, false)),
            }
        };
        self.batch_delete(
            (&contract_documents_primary_key_path).into(),
            document_id.as_slice(),
            apply_type,
            transaction,
            batch_operations,
            &platform_version.drive,
        )?;

        // if we are trying to get estimated costs we should add this level
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_remove_document_to_primary_storage(
                contract_documents_primary_key_path,
                document_type,
                estimated_costs_only_with_layer_info,
                platform_version,
            )?;
        }
        Ok(())
    }
}
