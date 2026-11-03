use crate::drive::contract::paths::{
    contract_document_removals_path, contract_other_path, CONTRACT_DOCUMENT_REMOVALS_KEY,
};
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::object_size_info::DriveKeyInfo;
use crate::util::storage_flags::StorageFlags;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn insert_contract_document_removal_trees_operations_v0(
        &self,
        contract_id: [u8; 32],
        with_root: bool,
        document_type_names: &[&str],
        storage_flags: Option<&StorageFlags>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        _transaction: TransactionArg,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Drive::add_estimation_costs_for_contract_document_removal_trees(
                contract_id,
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }

        // Unconditional, like the list trees: an insertion (re)creates the contract's root
        // subtree in the same batch, and an update only names document types the stored
        // contract does not have, whose trees can not exist.
        if with_root {
            self.batch_insert_empty_tree(
                contract_other_path(&contract_id),
                DriveKeyInfo::KeyRef(&[CONTRACT_DOCUMENT_REMOVALS_KEY]),
                storage_flags,
                batch_operations,
                &platform_version.drive,
            )?;
        }

        for document_type_name in document_type_names {
            self.batch_insert_empty_tree(
                contract_document_removals_path(&contract_id),
                DriveKeyInfo::KeyRef(document_type_name.as_bytes()),
                storage_flags,
                batch_operations,
                &platform_version.drive,
            )?;
        }

        Ok(())
    }
}
