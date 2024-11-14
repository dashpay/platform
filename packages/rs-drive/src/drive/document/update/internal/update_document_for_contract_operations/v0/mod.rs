use crate::drive::constants::CONTRACT_DOCUMENTS_PATH_HEIGHT;
use crate::drive::document::make_document_reference;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::{
    BatchDeleteUpTreeApplyType, BatchInsertApplyType, BatchInsertTreeApplyType, DirectQueryType,
    QueryType,
};
use crate::util::object_size_info::DocumentInfo::DocumentOwnedInfo;
use crate::util::object_size_info::DriveKeyInfo::{Key, KeyRef, KeySize};
use crate::util::object_size_info::PathKeyElementInfo::PathKeyRefElement;
use crate::util::object_size_info::{
    DocumentAndContractInfo, DocumentInfoV0Methods, DriveKeyInfo, PathKeyInfo,
};
use crate::util::storage_flags::StorageFlags;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;

use dpp::document::document_methods::DocumentMethodsV0;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::{Document, DocumentV0Getters};

use crate::drive::document::paths::{
    contract_document_type_path,
    contract_documents_keeping_history_primary_key_path_for_document_id,
    contract_documents_primary_key_path,
};
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::version::PlatformVersion;
use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::key_info::KeyInfo::KnownKey;
use grovedb::batch::KeyInfoPath;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

impl Drive {
    /// Gathers operations for updating a document.
    pub(in crate::drive::document::update) fn update_document_for_contract_operations_v0(
        &self,
        document_and_contract_info: DocumentAndContractInfo,
        block_info: &BlockInfo,
        previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let drive_version = &platform_version.drive;
        let mut batch_operations: Vec<LowLevelDriveOperation> = vec![];
        if !document_and_contract_info.document_type.requires_revision()
        // if it requires revision then there are reasons for us to be able to update in drive
        {
            return Err(Error::Drive(DriveError::UpdatingReadOnlyImmutableDocument(
                "documents for this contract are not mutable",
            )));
        }

        // If we are going for estimated costs do an add instead as it always worse than an update
        if document_and_contract_info
            .owned_document_info
            .document_info
            .is_document_size()
            || estimated_costs_only_with_layer_info.is_some()
        {
            return self.add_document_for_contract_operations(
                document_and_contract_info,
                true, // we say we should override as this skips an unnecessary check
                block_info,
                previous_batch_operations,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            );
        }

        let contract = document_and_contract_info.contract;
        let document_type = document_and_contract_info.document_type;
        let owner_id = document_and_contract_info.owned_document_info.owner_id;
        let Some((document, storage_flags)) = document_and_contract_info
            .owned_document_info
            .document_info
            .get_borrowed_document_and_storage_flags()
        else {
            return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                "must have document and storage flags",
            )));
        };
        // we need to construct the path for documents on the contract
        // the path is
        //  * Document andDataContract root tree
        //  *DataContract ID recovered from document
        //  * 0 to signify Documents and notDataContract
        let contract_document_type_path =
            contract_document_type_path(contract.id_ref().as_bytes(), document_type.name());

        let contract_documents_primary_key_path =
            contract_documents_primary_key_path(contract.id_ref().as_bytes(), document_type.name());

        let document_reference = make_document_reference(
            document,
            document_and_contract_info.document_type,
            storage_flags,
        );

        // next we need to get the old document from storage
        let old_document_element = if document_type.documents_keep_history() {
            let contract_documents_keeping_history_primary_key_path_for_document_id =
                contract_documents_keeping_history_primary_key_path_for_document_id(
                    contract.id_ref().as_bytes(),
                    document_type.name().as_str(),
                    document.id_ref().as_slice(),
                );
            // When keeping document history the 0 is a reference that points to the current value
            // O is just on one byte, so we have at most one hop of size 1 (1 byte)
            self.grove_get(
                (&contract_documents_keeping_history_primary_key_path_for_document_id).into(),
                &[0],
                QueryType::StatefulQuery,
                transaction,
                &mut batch_operations,
                drive_version,
            )?
        } else {
            self.grove_get_raw(
                (&contract_documents_primary_key_path).into(),
                document.id().as_slice(),
                DirectQueryType::StatefulDirectQuery,
                transaction,
                &mut batch_operations,
                drive_version,
            )?
        };

        // we need to store the document for it's primary key
        // we should be overriding if the document_type does not have history enabled
        self.add_document_to_primary_storage(
            &document_and_contract_info,
            block_info,
            true,
            estimated_costs_only_with_layer_info,
            transaction,
            &mut batch_operations,
            platform_version,
        )?;

        let old_document_info = if let Some(old_document_element) = old_document_element {
            if let Element::Item(old_serialized_document, element_flags) = old_document_element {
                let document = Document::from_bytes(
                    old_serialized_document.as_slice(),
                    document_type,
                    platform_version,
                )?;
                let storage_flags = StorageFlags::map_some_element_flags_ref(&element_flags)?;
                Ok(DocumentOwnedInfo((document, storage_flags.map(Cow::Owned))))
            } else {
                Err(Error::Drive(DriveError::CorruptedDocumentNotItem(
                    "old document is not an item",
                )))
            }?
        } else {
            return Err(Error::Drive(DriveError::UpdatingDocumentThatDoesNotExist(
                "document being updated does not exist",
            )));
        };

        let mut batch_insertion_cache: HashSet<Vec<Vec<u8>>> = HashSet::new();
        // fourth we need to store a reference to the document for each index
        for index in document_type.indexes().values() {
            // at this point the contract path is to the contract documents
            // for each index the top index component will already have been added
            // when the contract itself was created
            let mut index_path: Vec<Vec<u8>> = contract_document_type_path
                .iter()
                .map(|&x| Vec::from(x))
                .collect();
            let top_index_property = index.properties.first().ok_or(Error::Drive(
                DriveError::CorruptedContractIndexes("invalid contract indices".to_string()),
            ))?;
            index_path.push(Vec::from(top_index_property.name.as_bytes()));

            // with the example of the dashpay contract's first index
            // the index path is now something likeDataContracts/ContractID/Documents(1)/$ownerId
            let document_top_field = document
                .get_raw_for_document_type(
                    &top_index_property.name,
                    document_type,
                    owner_id,
                    platform_version,
                )?
                .unwrap_or_default();

            let old_document_top_field = old_document_info
                .get_raw_for_document_type(
                    &top_index_property.name,
                    document_type,
                    None, // We want to use the old owner id
                    None,
                    platform_version,
                )?
                .unwrap_or_default();

            // if we are not applying that means we are trying to get worst case costs
            // which would entail a change on every index
            let mut change_occurred_on_index = match &old_document_top_field {
                DriveKeyInfo::Key(k) => &document_top_field != k,
                DriveKeyInfo::KeyRef(k) => document_top_field.as_slice() != *k,
                DriveKeyInfo::KeySize(_) => {
                    // we should assume true in this worst case cost scenario
                    true
                }
            };

            if change_occurred_on_index {
                // here we are inserting an empty tree that will have a subtree of all other index properties
                let mut qualified_path = index_path.clone();
                qualified_path.push(document_top_field.clone());

                if !batch_insertion_cache.contains(&qualified_path) {
                    let inserted = self.batch_insert_empty_tree_if_not_exists(
                        PathKeyInfo::PathKeyRef::<0>((
                            index_path.clone(),
                            document_top_field.as_slice(),
                        )),
                        false,
                        storage_flags,
                        BatchInsertTreeApplyType::StatefulBatchInsertTree,
                        transaction,
                        previous_batch_operations,
                        &mut batch_operations,
                        drive_version,
                    )?;
                    if inserted {
                        batch_insertion_cache.insert(qualified_path);
                    }
                }
            }

            let mut all_fields_null = document_top_field.is_empty();

            let mut old_index_path: Vec<DriveKeyInfo> = index_path
                .iter()
                .map(|path_item| DriveKeyInfo::Key(path_item.clone()))
                .collect();
            // we push the actual value of the index path
            index_path.push(document_top_field);
            // the index path is now something likeDataContracts/ContractID/Documents(1)/$ownerId/<ownerId>

            old_index_path.push(old_document_top_field);

            for i in 1..index.properties.len() {
                let index_property = index.properties.get(i).ok_or(Error::Drive(
                    DriveError::CorruptedContractIndexes("invalid contract indices".to_string()),
                ))?;

                let document_index_field = document
                    .get_raw_for_document_type(
                        &index_property.name,
                        document_type,
                        owner_id,
                        platform_version,
                    )?
                    .unwrap_or_default();

                let old_document_index_field = old_document_info
                    .get_raw_for_document_type(
                        &index_property.name,
                        document_type,
                        None, // We want to use the old owner_id
                        None,
                        platform_version,
                    )?
                    .unwrap_or_default();

                // if we are not applying that means we are trying to get worst case costs
                // which would entail a change on every index
                change_occurred_on_index |= match &old_document_index_field {
                    DriveKeyInfo::Key(k) => &document_index_field != k,
                    DriveKeyInfo::KeyRef(k) => document_index_field != *k,
                    DriveKeyInfo::KeySize(_) => {
                        // we should assume true in this worst case cost scenario
                        true
                    }
                };

                if change_occurred_on_index {
                    // here we are inserting an empty tree that will have a subtree of all other index properties

                    let mut qualified_path = index_path.clone();
                    qualified_path.push(index_property.name.as_bytes().to_vec());

                    if !batch_insertion_cache.contains(&qualified_path) {
                        let inserted = self.batch_insert_empty_tree_if_not_exists(
                            PathKeyInfo::PathKeyRef::<0>((
                                index_path.clone(),
                                index_property.name.as_bytes(),
                            )),
                            false,
                            storage_flags,
                            BatchInsertTreeApplyType::StatefulBatchInsertTree,
                            transaction,
                            previous_batch_operations,
                            &mut batch_operations,
                            drive_version,
                        )?;
                        if inserted {
                            batch_insertion_cache.insert(qualified_path);
                        }
                    }
                }

                index_path.push(Vec::from(index_property.name.as_bytes()));
                old_index_path.push(DriveKeyInfo::Key(Vec::from(index_property.name.as_bytes())));

                // Iteration 1. the index path is now something likeDataContracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId
                // Iteration 2. the index path is now something likeDataContracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId/<ToUserId>/accountReference

                if change_occurred_on_index {
                    // here we are inserting an empty tree that will have a subtree of all other index properties

                    let mut qualified_path = index_path.clone();
                    qualified_path.push(document_index_field.clone());

                    if !batch_insertion_cache.contains(&qualified_path) {
                        let inserted = self.batch_insert_empty_tree_if_not_exists(
                            PathKeyInfo::PathKeyRef::<0>((
                                index_path.clone(),
                                document_index_field.as_slice(),
                            )),
                            false,
                            storage_flags,
                            BatchInsertTreeApplyType::StatefulBatchInsertTree,
                            transaction,
                            previous_batch_operations,
                            &mut batch_operations,
                            drive_version,
                        )?;
                        if inserted {
                            batch_insertion_cache.insert(qualified_path);
                        }
                    }
                }

                all_fields_null &= document_index_field.is_empty();

                // we push the actual value of the index path, both for the new and the old
                index_path.push(document_index_field);
                old_index_path.push(old_document_index_field);
                // Iteration 1. the index path is now something likeDataContracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId/<ToUserId>/
                // Iteration 2. the index path is now something likeDataContracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId/<ToUserId>/accountReference/<accountReference>
            }

            if change_occurred_on_index {
                // we first need to delete the old values
                // unique indexes will be stored under key "0"
                // non unique indices should have a tree at key "0" that has all elements based off of primary key

                let mut key_info_path = KeyInfoPath::from_vec(
                    old_index_path
                        .into_iter()
                        .map(|key_info| match key_info {
                            Key(key) => KnownKey(key),
                            KeyRef(key_ref) => KnownKey(key_ref.to_vec()),
                            KeySize(key_info) => key_info,
                        })
                        .collect::<Vec<KeyInfo>>(),
                );

                if !index.unique {
                    key_info_path.push(KnownKey(vec![0]));

                    // here we should return an error if the element already exists
                    self.batch_delete_up_tree_while_empty(
                        key_info_path,
                        document.id().as_slice(),
                        Some(CONTRACT_DOCUMENTS_PATH_HEIGHT),
                        BatchDeleteUpTreeApplyType::StatefulBatchDelete {
                            is_known_to_be_subtree_with_sum: Some((false, false)),
                        },
                        transaction,
                        previous_batch_operations,
                        &mut batch_operations,
                        drive_version,
                    )?;
                } else {
                    // here we should return an error if the element already exists
                    self.batch_delete_up_tree_while_empty(
                        key_info_path,
                        &[0],
                        Some(CONTRACT_DOCUMENTS_PATH_HEIGHT),
                        BatchDeleteUpTreeApplyType::StatefulBatchDelete {
                            is_known_to_be_subtree_with_sum: Some((false, false)),
                        },
                        transaction,
                        previous_batch_operations,
                        &mut batch_operations,
                        drive_version,
                    )?;
                }

                // unique indexes will be stored under key "0"
                // non unique indices should have a tree at key "0" that has all elements based off of primary key
                if !index.unique || all_fields_null {
                    // here we are inserting an empty tree that will have a subtree of all other index properties
                    self.batch_insert_empty_tree_if_not_exists(
                        PathKeyInfo::PathKeyRef::<0>((index_path.clone(), &[0])),
                        false,
                        storage_flags,
                        BatchInsertTreeApplyType::StatefulBatchInsertTree,
                        transaction,
                        previous_batch_operations,
                        &mut batch_operations,
                        drive_version,
                    )?;
                    index_path.push(vec![0]);

                    // here we should return an error if the element already exists
                    self.batch_insert(
                        PathKeyRefElement::<0>((
                            index_path,
                            document.id().as_slice(),
                            document_reference.clone(),
                        )),
                        &mut batch_operations,
                        drive_version,
                    )?;
                } else {
                    // in one update you can't insert an element twice, so need to check the cache
                    // here we should return an error if the element already exists
                    let inserted = self.batch_insert_if_not_exists(
                        PathKeyRefElement::<0>((index_path, &[0], document_reference.clone())),
                        BatchInsertApplyType::StatefulBatchInsert,
                        transaction,
                        &mut batch_operations,
                        drive_version,
                    )?;
                    if !inserted {
                        return Err(Error::Drive(DriveError::CorruptedContractIndexes(
                            "index already exists".to_string(),
                        )));
                    }
                }
            } else {
                // no change occurred on index, we need to refresh the references

                // We can only trust the reference content has not changed if there are no storage flags
                let trust_refresh_reference = storage_flags.is_none();

                // unique indexes will be stored under key "0"
                // non unique indices should have a tree at key "0" that has all elements based off of primary key
                if !index.unique || all_fields_null {
                    index_path.push(vec![0]);

                    // here we should return an error if the element already exists
                    self.batch_refresh_reference(
                        index_path,
                        document.id().to_vec(),
                        document_reference.clone(),
                        trust_refresh_reference,
                        &mut batch_operations,
                        drive_version,
                    )?;
                } else {
                    self.batch_refresh_reference(
                        index_path,
                        vec![0],
                        document_reference.clone(),
                        trust_refresh_reference,
                        &mut batch_operations,
                        drive_version,
                    )?;
                }
            }
        }
        Ok(batch_operations)
    }
}
