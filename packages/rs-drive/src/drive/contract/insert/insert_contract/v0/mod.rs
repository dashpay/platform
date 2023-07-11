use std::collections::{HashMap, HashSet};
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use grovedb::batch::KeyInfoPath;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::DataContract;
use dpp::serialization_traits::PlatformSerializable;
use crate::fee::calculate_fee;
use dpp::fee::fee_result::FeeResult;
use dpp::version::drive_versions::DriveVersion;
use crate::drive::{contract_documents_path, Drive, RootTree};
use crate::drive::contract::paths;
use crate::drive::flags::StorageFlags;
use crate::drive::object_size_info::DriveKeyInfo::{Key, KeyRef};
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;

impl Drive {
    /// Insert a contract.
    pub(super) fn insert_contract_v0(
        &self,
        contract: &DataContract,
        block_info: BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<FeeResult, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];

        let storage_flags = if contract.config.can_be_deleted || !contract.config.readonly {
            Some(StorageFlags::new_single_epoch(
                block_info.epoch.index,
                Some(contract.owner_id.to_buffer()),
            ))
        } else {
            None
        };

        let contract_element = Element::Item(
            contract.serialize()?,
            StorageFlags::map_to_some_element_flags(storage_flags.as_ref()),
        );

        self.insert_contract_element_v0(
            contract_element,
            contract,
            &block_info,
            apply,
            transaction,
            &mut drive_operations,
            drive_version,
        )?;

        calculate_fee(None, Some(drive_operations), &block_info.epoch).map_err(Error::Protocol)
    }

    /// Adds a contract to storage using `add_contract_to_storage`
    /// and inserts the empty trees which will be necessary to later insert documents.
    pub(super) fn insert_contract_element_v0(
        &self,
        contract_element: Element,
        contract: &DataContract,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        let mut estimated_costs_only_with_layer_info = if apply {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };
        let batch_operations = self.insert_contract_operations_v0(
            contract_element,
            contract,
            block_info,
            &mut estimated_costs_only_with_layer_info,
            drive_version,
        )?;
        self.apply_batch_low_level_drive_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            batch_operations,
            drive_operations,
            drive_version,
        )
    }

    /// The operations for adding a contract.
    /// These operations add a contract to storage using `add_contract_to_storage`
    /// and insert the empty trees which will be necessary to later insert documents.
    pub(super) fn insert_contract_add_operations_v0(
        &self,
        contract_element: Element,
        contract: &DataContract,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        let batch_operations = self.insert_contract_operations_v0(
            contract_element,
            contract,
            block_info,
            estimated_costs_only_with_layer_info,
            drive_version,
        )?;
        drive_operations.extend(batch_operations);
        Ok(())
    }

    // TODO(doc): comment duplicates function above. Needs to be written that this function
    //   generates operations, not adds them to a contract
    /// The operations for adding a contract.
    /// These operations add a contract to storage using `add_contract_to_storage`
    /// and insert the empty trees which will be necessary to later insert documents.
    pub(super) fn insert_contract_operations_v0(
        &self,
        contract_element: Element,
        contract: &DataContract,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        drive_version: &DriveVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut batch_operations: Vec<LowLevelDriveOperation> = vec![];

        let storage_flags = StorageFlags::map_some_element_flags_ref(contract_element.get_flags())?;

        self.batch_insert_empty_tree(
            [Into::<&[u8; 1]>::into(RootTree::ContractDocuments).as_slice()],
            KeyRef(contract.id().as_bytes()),
            storage_flags.as_ref(),
            &mut batch_operations,
            drive_version,
        )?;

        self.add_contract_to_storage(
            contract_element,
            contract,
            block_info,
            estimated_costs_only_with_layer_info,
            &mut batch_operations,
            true,
            None, // we are not inserting into history, hence the transaction will not be used, we can pass None
            drive_version,
        )?;

        // the documents
        let contract_root_path = paths::contract_root_path(contract.id().as_bytes());
        let key_info = Key(vec![1]);
        self.batch_insert_empty_tree(
            contract_root_path,
            key_info,
            storage_flags.as_ref(),
            &mut batch_operations,
            drive_version,
        )?;

        // next we should store each document type
        // right now we are referring them by name
        // toDo: change this to be a reference by index
        let contract_documents_path = contract_documents_path(contract.id().as_bytes());

        for (type_key, document_type) in contract.document_types.iter() {
            self.batch_insert_empty_tree(
                contract_documents_path,
                KeyRef(type_key.as_bytes()),
                storage_flags.as_ref(),
                &mut batch_operations,
                drive_version,
            )?;

            let type_path = [
                contract_documents_path[0],
                contract_documents_path[1],
                contract_documents_path[2],
                type_key.as_bytes(),
            ];

            // primary key tree
            let key_info = Key(vec![0]);
            self.batch_insert_empty_tree(
                type_path,
                key_info,
                storage_flags.as_ref(),
                &mut batch_operations,
                drive_version,
            )?;

            let mut index_cache: HashSet<&[u8]> = HashSet::new();
            // for each type we should insert the indices that are top level
            for index in document_type.top_level_indices() {
                // toDo: change this to be a reference by index
                let index_bytes = index.name.as_bytes();
                if !index_cache.contains(index_bytes) {
                    self.batch_insert_empty_tree(
                        type_path,
                        KeyRef(index_bytes),
                        storage_flags.as_ref(),
                        &mut batch_operations,
                        drive_version,
                    )?;
                    index_cache.insert(index_bytes);
                }
            }
        }

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_contract_insertion(
                contract,
                estimated_costs_only_with_layer_info,
                drive_version
            )?;
        }

        Ok(batch_operations)
    }
}