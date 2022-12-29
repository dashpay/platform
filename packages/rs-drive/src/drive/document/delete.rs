// MIT LICENSE
//
// Copyright (c) 2021 Dash Core Group
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
//

//! Delete Documents.
//!
//! This module implements functions in Drive for deleting documents.
//!

use grovedb::batch::key_info::KeyInfo::KnownKey;
use grovedb::batch::KeyInfoPath;

use grovedb::EstimatedLayerCount::{ApproximateElements, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerSizes::{AllItems, AllReference, AllSubtrees};
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};

use grovedb::EstimatedSumTrees::NoSumTrees;
use std::collections::HashMap;

use crate::contract::document::Document;
use crate::contract::Contract;
use crate::drive::block_info::BlockInfo;
use crate::drive::defaults::{
    AVERAGE_NUMBER_OF_UPDATES, AVERAGE_UPDATE_BYTE_COUNT_REQUIRED_SIZE,
    CONTRACT_DOCUMENTS_PATH_HEIGHT, DEFAULT_HASH_SIZE_U8,
};
use crate::drive::document::{
    contract_document_type_path_vec, contract_documents_primary_key_path, document_reference_size,
    unique_event_id,
};
use crate::drive::flags::StorageFlags;
use crate::drive::object_size_info::DocumentInfo::{
    DocumentEstimatedAverageSize, DocumentWithoutSerialization,
};
use crate::drive::object_size_info::DriveKeyInfo::KeyRef;

use crate::drive::grove_operations::BatchDeleteApplyType::{
    StatefulBatchDelete, StatelessBatchDelete,
};
use crate::drive::grove_operations::DirectQueryType;
use crate::drive::grove_operations::QueryTarget::QueryTargetValue;
use crate::drive::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo, PathInfo};
use crate::drive::Drive;
use crate::error::document::DocumentError;
use crate::error::drive::DriveError;
use crate::error::fee::FeeError;
use crate::error::Error;
use crate::fee::calculate_fee;
use crate::fee::op::DriveOperation;
use crate::fee::result::FeeResult;
use dpp::data_contract::extra::{DocumentType, DriveContractExt, IndexLevel};

impl Drive {
    /// Deletes a document and returns the associated fee.
    pub fn delete_document_for_contract(
        &self,
        document_id: [u8; 32],
        contract: &Contract,
        document_type_name: &str,
        owner_id: Option<[u8; 32]>,
        block_info: BlockInfo,
        apply: bool,
        transaction: TransactionArg,
    ) -> Result<FeeResult, Error> {
        let mut drive_operations: Vec<DriveOperation> = vec![];
        let estimated_costs_only_with_layer_info = if apply {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };
        self.delete_document_for_contract_apply_and_add_to_operations(
            document_id,
            contract,
            document_type_name,
            owner_id,
            estimated_costs_only_with_layer_info,
            transaction,
            &mut drive_operations,
        )?;
        let fees = calculate_fee(None, Some(drive_operations), &block_info.epoch)?;
        Ok(fees)
    }

    /// Deletes a document and returns the associated fee.
    /// The contract CBOR is given instead of the contract itself.
    pub fn delete_document_for_contract_cbor(
        &self,
        document_id: [u8; 32],
        contract_cbor: &[u8],
        document_type_name: &str,
        owner_id: Option<[u8; 32]>,
        block_info: BlockInfo,
        apply: bool,
        transaction: TransactionArg,
    ) -> Result<FeeResult, Error> {
        let contract = <Contract as DriveContractExt>::from_cbor(contract_cbor, None)?;
        self.delete_document_for_contract(
            document_id,
            &contract,
            document_type_name,
            owner_id,
            block_info,
            apply,
            transaction,
        )
    }

    /// Deletes a document and returns the associated fee.
    /// The contract CBOR is given instead of the contract itself.
    pub fn delete_document_for_contract_id(
        &self,
        document_id: [u8; 32],
        contract_id: [u8; 32],
        document_type_name: &str,
        owner_id: Option<[u8; 32]>,
        block_info: BlockInfo,
        apply: bool,
        transaction: TransactionArg,
    ) -> Result<FeeResult, Error> {
        let mut drive_operations: Vec<DriveOperation> = vec![];
        let estimated_costs_only_with_layer_info = if apply {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };

        let contract_fetch_info = self
            .get_contract_with_fetch_info_and_add_to_operations(
                contract_id,
                Some(&block_info.epoch),
                transaction,
                &mut drive_operations,
            )?
            .ok_or(Error::Document(DocumentError::ContractNotFound()))?;

        let contract = &contract_fetch_info.contract;

        self.delete_document_for_contract_apply_and_add_to_operations(
            document_id,
            contract,
            document_type_name,
            owner_id,
            estimated_costs_only_with_layer_info,
            transaction,
            &mut drive_operations,
        )?;

        let fees = calculate_fee(None, Some(drive_operations), &block_info.epoch)?;

        Ok(fees)
    }

    /// Deletes a document.
    pub fn delete_document_for_contract_apply_and_add_to_operations(
        &self,
        document_id: [u8; 32],
        contract: &Contract,
        document_type_name: &str,
        owner_id: Option<[u8; 32]>,
        mut estimated_costs_only_with_layer_info: Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error> {
        let batch_operations = self.delete_document_for_contract_operations(
            document_id,
            contract,
            document_type_name,
            owner_id,
            None,
            &mut estimated_costs_only_with_layer_info,
            transaction,
        )?;
        self.apply_batch_drive_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            batch_operations,
            drive_operations,
        )
    }

    fn add_estimation_costs_for_remove_document_to_primary_storage(
        primary_key_path: [&[u8]; 5],
        document_type: &DocumentType,
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
    ) {
        // we just have the elements
        let approximate_size = if document_type.documents_mutable {
            //todo: have the contract say how often we expect documents to mutate
            Some((
                AVERAGE_NUMBER_OF_UPDATES as u16,
                AVERAGE_UPDATE_BYTE_COUNT_REQUIRED_SIZE,
            ))
        } else {
            None
        };
        let flags_size = StorageFlags::approximate_size(true, approximate_size);
        estimated_costs_only_with_layer_info.insert(
            KeyInfoPath::from_known_path(primary_key_path),
            EstimatedLayerInformation {
                is_sum_tree: false,
                estimated_layer_count: PotentiallyAtMaxElements,
                estimated_layer_sizes: AllItems(
                    DEFAULT_HASH_SIZE_U8,
                    document_type.estimated_size() as u32,
                    Some(flags_size),
                ),
            },
        );
    }

    /// Removes the document from primary storage.
    fn remove_document_from_primary_storage(
        &self,
        document_id: [u8; 32],
        document_type: &DocumentType,
        contract_documents_primary_key_path: [&[u8]; 5],
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        batch_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error> {
        let apply_type = if estimated_costs_only_with_layer_info.is_some() {
            StatelessBatchDelete {
                is_sum_tree: false,
                estimated_value_size: document_type.estimated_size() as u32,
            }
        } else {
            // we know we are not deleting a subtree
            StatefulBatchDelete {
                is_known_to_be_subtree_with_sum: Some((false, false)),
            }
        };
        self.batch_delete(
            contract_documents_primary_key_path,
            document_id.as_slice(),
            apply_type,
            transaction,
            batch_operations,
        )?;

        // if we are trying to get estimated costs we should add this level
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_remove_document_to_primary_storage(
                contract_documents_primary_key_path,
                document_type,
                estimated_costs_only_with_layer_info,
            );
        }
        Ok(())
    }

    /// Removes the terminal reference.
    fn remove_reference_for_index_level_for_contract_operations(
        &self,
        document_and_contract_info: &DocumentAndContractInfo,
        index_path_info: PathInfo<0>,
        unique: bool,
        any_fields_null: bool,
        storage_flags: &Option<&StorageFlags>,
        previous_batch_operations: &Option<&mut Vec<DriveOperation>>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        event_id: [u8; 32],
        transaction: TransactionArg,
        batch_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error> {
        let mut key_info_path = index_path_info.convert_to_key_info_path();

        let document_type = document_and_contract_info.document_type;

        // unique indexes will be stored under key "0"
        // non unique indices should have a tree at key "0" that has all elements based off of primary key
        if !unique || any_fields_null {
            key_info_path.push(KnownKey(vec![0]));

            if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info
            {
                // On this level we will have a 0 and all the top index paths
                estimated_costs_only_with_layer_info.insert(
                    key_info_path.clone(),
                    EstimatedLayerInformation {
                        is_sum_tree: false,
                        estimated_layer_count: PotentiallyAtMaxElements,
                        estimated_layer_sizes: AllSubtrees(
                            DEFAULT_HASH_SIZE_U8,
                            NoSumTrees,
                            storage_flags.map(|s| s.serialized_size()),
                        ),
                    },
                );
            }

            let delete_apply_type = Self::stateless_delete_of_non_tree_for_costs(
                AllReference(
                    DEFAULT_HASH_SIZE_U8,
                    document_reference_size(document_type),
                    storage_flags.map(|s| s.serialized_size()),
                ),
                &key_info_path,
                // we know we are not deleting a tree
                Some((false, false)),
                estimated_costs_only_with_layer_info,
            )?;

            // here we should return an error if the element already exists
            self.batch_delete_up_tree_while_empty(
                key_info_path,
                document_and_contract_info
                    .owned_document_info
                    .document_info
                    .get_document_id_as_slice()
                    .unwrap_or(event_id.as_slice()),
                Some(CONTRACT_DOCUMENTS_PATH_HEIGHT),
                delete_apply_type,
                transaction,
                previous_batch_operations,
                batch_operations,
            )?;
        } else {
            let delete_apply_type = Self::stateless_delete_of_non_tree_for_costs(
                AllReference(
                    1,
                    document_reference_size(document_type),
                    storage_flags.map(|s| s.serialized_size()),
                ),
                &key_info_path,
                // we know we are not deleting a tree
                Some((false, false)),
                estimated_costs_only_with_layer_info,
            )?;
            // here we should return an error if the element already exists
            self.batch_delete_up_tree_while_empty(
                key_info_path,
                &[0],
                Some(CONTRACT_DOCUMENTS_PATH_HEIGHT),
                delete_apply_type,
                transaction,
                previous_batch_operations,
                batch_operations,
            )?;
        }
        Ok(())
    }

    /// Removes indices for an index level and recurses.
    fn remove_indices_for_index_level_for_contract_operations(
        &self,
        document_and_contract_info: &DocumentAndContractInfo,
        index_path_info: PathInfo<0>,
        index_level: &IndexLevel,
        mut any_fields_null: bool,
        storage_flags: &Option<&StorageFlags>,
        previous_batch_operations: &Option<&mut Vec<DriveOperation>>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        event_id: [u8; 32],
        transaction: TransactionArg,
        batch_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error> {
        let sub_level_index_count = index_level.sub_index_levels.len() as u32;

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            // On this level we will have a 0 and all the top index paths
            estimated_costs_only_with_layer_info.insert(
                index_path_info.clone().convert_to_key_info_path(),
                EstimatedLayerInformation {
                    is_sum_tree: false,
                    estimated_layer_count: ApproximateElements(sub_level_index_count + 1),
                    estimated_layer_sizes: AllSubtrees(
                        DEFAULT_HASH_SIZE_U8,
                        NoSumTrees,
                        storage_flags.map(|s| s.serialized_size()),
                    ),
                },
            );
        }

        if let Some(unique) = index_level.has_index_with_uniqueness {
            self.remove_reference_for_index_level_for_contract_operations(
                document_and_contract_info,
                index_path_info.clone(),
                unique,
                any_fields_null,
                storage_flags,
                previous_batch_operations,
                estimated_costs_only_with_layer_info,
                event_id,
                transaction,
                batch_operations,
            )?;
        }

        let document_type = document_and_contract_info.document_type;

        // fourth we need to store a reference to the document for each index
        for (name, sub_level) in &index_level.sub_index_levels {
            let mut sub_level_index_path_info = index_path_info.clone();
            let index_property_key = KeyRef(name.as_bytes());

            let document_index_field = document_and_contract_info
                .owned_document_info
                .document_info
                .get_raw_for_document_type(
                    name,
                    document_type,
                    document_and_contract_info.owned_document_info.owner_id,
                    Some((sub_level, event_id)),
                )?
                .unwrap_or_default();

            sub_level_index_path_info.push(index_property_key)?;

            if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info
            {
                let document_top_field_estimated_size = document_and_contract_info
                    .owned_document_info
                    .document_info
                    .get_estimated_size_for_document_type(name, document_type)?;

                if document_top_field_estimated_size > u8::MAX as u16 {
                    return Err(Error::Fee(FeeError::Overflow(
                        "document field is too big for being an index",
                    )));
                }

                estimated_costs_only_with_layer_info.insert(
                    sub_level_index_path_info.clone().convert_to_key_info_path(),
                    EstimatedLayerInformation {
                        is_sum_tree: false,
                        estimated_layer_count: PotentiallyAtMaxElements,
                        estimated_layer_sizes: AllSubtrees(
                            document_top_field_estimated_size as u8,
                            NoSumTrees,
                            storage_flags.map(|s| s.serialized_size()),
                        ),
                    },
                );
            }

            // Iteration 1. the index path is now something like Contracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId
            // Iteration 2. the index path is now something like Contracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId/<ToUserId>/accountReference

            any_fields_null |= document_index_field.is_empty();

            // we push the actual value of the index path
            sub_level_index_path_info.push(document_index_field)?;
            // Iteration 1. the index path is now something like Contracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId/<ToUserId>/
            // Iteration 2. the index path is now something like Contracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId/<ToUserId>/accountReference/<accountReference>
            self.remove_indices_for_index_level_for_contract_operations(
                document_and_contract_info,
                sub_level_index_path_info,
                sub_level,
                any_fields_null,
                storage_flags,
                previous_batch_operations,
                estimated_costs_only_with_layer_info,
                event_id,
                transaction,
                batch_operations,
            )?;
        }
        Ok(())
    }

    /// Removes indices for the top index level and calls for lower levels.
    fn remove_indices_for_top_index_level_for_contract_operations(
        &self,
        document_and_contract_info: &DocumentAndContractInfo,
        previous_batch_operations: &Option<&mut Vec<DriveOperation>>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        batch_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error> {
        let document_type = document_and_contract_info.document_type;
        let index_level = &document_type.index_structure;
        let contract = document_and_contract_info.contract;
        let event_id = unique_event_id();
        let storage_flags = if document_type.documents_mutable || contract.can_be_deleted() {
            document_and_contract_info
                .owned_document_info
                .document_info
                .get_storage_flags_ref()
        } else {
            None //there are no need for storage flags if documents are not mutable and contract can not be deleted
        };

        // we need to construct the path for documents on the contract
        // the path is
        //  * Document and Contract root tree
        //  * Contract ID recovered from document
        //  * 0 to signify Documents and not Contract
        let contract_document_type_path = contract_document_type_path_vec(
            document_and_contract_info.contract.id.as_bytes(),
            document_and_contract_info.document_type.name.as_str(),
        );

        let sub_level_index_count = index_level.sub_index_levels.len() as u32;

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            // On this level we will have a 0 and all the top index paths
            estimated_costs_only_with_layer_info.insert(
                KeyInfoPath::from_known_owned_path(contract_document_type_path.clone()),
                EstimatedLayerInformation {
                    is_sum_tree: false,
                    estimated_layer_count: ApproximateElements(sub_level_index_count + 1),
                    estimated_layer_sizes: AllSubtrees(
                        DEFAULT_HASH_SIZE_U8,
                        NoSumTrees,
                        storage_flags.map(|s| s.serialized_size()),
                    ),
                },
            );
        }

        // next we need to store a reference to the document for each index
        for (name, sub_level) in &index_level.sub_index_levels {
            // at this point the contract path is to the contract documents
            // for each index the top index component will already have been added
            // when the contract itself was created
            let mut index_path: Vec<Vec<u8>> = contract_document_type_path.clone();
            index_path.push(Vec::from(name.as_bytes()));

            // with the example of the dashpay contract's first index
            // the index path is now something like Contracts/ContractID/Documents(1)/$ownerId
            let document_top_field = document_and_contract_info
                .owned_document_info
                .document_info
                .get_raw_for_document_type(
                    name,
                    document_type,
                    document_and_contract_info.owned_document_info.owner_id,
                    Some((sub_level, event_id)),
                )?
                .unwrap_or_default();

            if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info
            {
                let document_top_field_estimated_size = document_and_contract_info
                    .owned_document_info
                    .document_info
                    .get_estimated_size_for_document_type(name, document_type)?;

                if document_top_field_estimated_size > u8::MAX as u16 {
                    return Err(Error::Fee(FeeError::Overflow(
                        "document top field is too big for being an index",
                    )));
                }

                // On this level we will have all the user defined values for the paths
                estimated_costs_only_with_layer_info.insert(
                    KeyInfoPath::from_known_owned_path(index_path.clone()),
                    EstimatedLayerInformation {
                        is_sum_tree: false,
                        estimated_layer_count: PotentiallyAtMaxElements,
                        estimated_layer_sizes: AllSubtrees(
                            document_top_field_estimated_size as u8,
                            NoSumTrees,
                            storage_flags.map(|s| s.serialized_size()),
                        ),
                    },
                );
            }

            let any_fields_null = document_top_field.is_empty();

            let mut index_path_info = if document_and_contract_info
                .owned_document_info
                .document_info
                .is_document_size()
            {
                // This is a stateless operation
                PathInfo::PathWithSizes(KeyInfoPath::from_known_owned_path(index_path))
            } else {
                PathInfo::PathIterator::<0>(index_path)
            };

            // we push the actual value of the index path
            index_path_info.push(document_top_field)?;
            // the index path is now something like Contracts/ContractID/Documents(1)/$ownerId/<ownerId>

            self.remove_indices_for_index_level_for_contract_operations(
                document_and_contract_info,
                index_path_info,
                sub_level,
                any_fields_null,
                &storage_flags,
                previous_batch_operations,
                estimated_costs_only_with_layer_info,
                event_id,
                transaction,
                batch_operations,
            )?;
        }
        Ok(())
    }

    /// Prepares the operations for deleting a document.
    pub(crate) fn delete_document_for_contract_operations(
        &self,
        document_id: [u8; 32],
        contract: &Contract,
        document_type_name: &str,
        owner_id: Option<[u8; 32]>,
        previous_batch_operations: Option<&mut Vec<DriveOperation>>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
    ) -> Result<Vec<DriveOperation>, Error> {
        let mut batch_operations: Vec<DriveOperation> = vec![];
        let document_type = contract.document_type_for_name(document_type_name)?;

        if !document_type.documents_mutable {
            return Err(Error::Drive(DriveError::UpdatingReadOnlyImmutableDocument(
                "this document type is not mutable and can not be deleted",
            )));
        }

        if document_type.documents_keep_history {
            return Err(Error::Drive(
                DriveError::InvalidDeletionOfDocumentThatKeepsHistory(
                    "this document type keeps history and therefore can not be deleted",
                ),
            ));
        }

        // first we need to construct the path for documents on the contract
        // the path is
        //  * Document and Contract root tree
        //  * Contract ID recovered from document
        //  * 0 to signify Documents and not Contract
        let contract_documents_primary_key_path =
            contract_documents_primary_key_path(contract.id.as_bytes(), document_type_name);

        let direct_query_type = if let Some(estimated_costs_only_with_layer_info) =
            estimated_costs_only_with_layer_info
        {
            Self::add_estimation_costs_for_levels_up_to_contract_document_type_excluded(
                contract,
                estimated_costs_only_with_layer_info,
            );
            DirectQueryType::StatelessDirectQuery {
                in_tree_using_sums: false,
                query_target: QueryTargetValue(document_type.estimated_size() as u32),
            }
        } else {
            DirectQueryType::StatefulDirectQuery
        };

        // next we need to get the document from storage
        let document_element: Option<Element> = self.grove_get_direct(
            contract_documents_primary_key_path,
            document_id.as_slice(),
            direct_query_type,
            transaction,
            &mut batch_operations,
        )?;

        let document_info =
            if let DirectQueryType::StatelessDirectQuery { query_target, .. } = direct_query_type {
                DocumentEstimatedAverageSize(query_target.len())
            } else if let Some(document_element) = &document_element {
                if let Element::Item(data, element_flags) = document_element {
                    let document = Document::from_cbor(data.as_slice(), None, owner_id)?;
                    let storage_flags = StorageFlags::from_some_element_flags_ref(element_flags)?;
                    DocumentWithoutSerialization((document, storage_flags))
                } else {
                    return Err(Error::Drive(DriveError::CorruptedDocumentNotItem(
                        "document being deleted is not an item",
                    )));
                }
            } else {
                return Err(Error::Drive(DriveError::DeletingDocumentThatDoesNotExist(
                    "document being deleted does not exist",
                )));
            };

        // third we need to delete the document for it's primary key
        self.remove_document_from_primary_storage(
            document_id,
            document_type,
            contract_documents_primary_key_path,
            estimated_costs_only_with_layer_info,
            transaction,
            &mut batch_operations,
        )?;

        let document_and_contract_info = DocumentAndContractInfo {
            owned_document_info: OwnedDocumentInfo {
                document_info,
                owner_id: None,
            },
            contract,
            document_type,
        };

        self.remove_indices_for_top_index_level_for_contract_operations(
            &document_and_contract_info,
            &previous_batch_operations,
            estimated_costs_only_with_layer_info,
            transaction,
            &mut batch_operations,
        )?;
        Ok(batch_operations)
    }
}

#[cfg(test)]
mod tests {
    use rand::Rng;
    use serde_json::json;
    use std::option::Option::None;
    use tempfile::TempDir;

    use super::*;
    use crate::common::{
        cbor_from_hex, json_document_to_cbor, setup_contract, setup_contract_from_hex,
        value_to_cbor,
    };
    use crate::contract::document::Document;
    use crate::drive::document::tests::setup_dashpay;
    use crate::drive::flags::StorageFlags;
    use crate::drive::object_size_info::DocumentAndContractInfo;
    use crate::drive::object_size_info::DocumentInfo::DocumentRefAndSerialization;
    use crate::drive::Drive;
    use crate::fee::credits::Creditable;
    use crate::fee::default_costs::STORAGE_DISK_USAGE_CREDIT_PER_BYTE;
    use crate::fee_pools::epochs::Epoch;
    use crate::query::DriveQuery;

    #[test]
    fn test_add_and_remove_family_one_document_no_transaction() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        drive
            .create_initial_state_structure(None)
            .expect("expected to create root tree successfully");

        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/family/family-contract-reduced.json",
            None,
            None,
        );

        let person_serialized_document = json_document_to_cbor(
            "tests/supporting_files/contract/family/person0.json",
            Some(1),
        );

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();

        let document =
            Document::from_cbor(&person_serialized_document, None, Some(random_owner_id))
                .expect("expected to deserialize the document");

        let document_type = contract
            .document_type_for_name("person")
            .expect("expected to get a document type");

        let storage_flags = Some(StorageFlags::SingleEpoch(0));

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefAndSerialization((
                            &document,
                            &person_serialized_document,
                            storage_flags.as_ref(),
                        )),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                None,
            )
            .expect("expected to insert a document successfully");

        let sql_string =
            "select * from person where firstName = 'Samuel' order by firstName asc limit 100";
        let query = DriveQuery::from_sql_expr(sql_string, &contract).expect("should build query");

        let (results_no_transaction, _, _) = query
            .execute_no_proof(&drive, None, None)
            .expect("expected to execute query");

        assert_eq!(results_no_transaction.len(), 1);

        let (results_on_transaction, _, _) = query
            .execute_no_proof(&drive, None, None)
            .expect("expected to execute query");

        assert_eq!(results_on_transaction.len(), 1);
        let document_id = bs58::decode("AYjYxDqLy2hvGQADqE6FAkBnQEpJSzNd3CRw1tpS6vZ7")
            .into_vec()
            .expect("should decode")
            .as_slice()
            .try_into()
            .expect("this be 32 bytes");

        drive
            .delete_document_for_contract(
                document_id,
                &contract,
                "person",
                Some(random_owner_id),
                BlockInfo::default(),
                true,
                None,
            )
            .expect("expected to be able to delete the document");

        let (results_on_transaction, _, _) = query
            .execute_no_proof(&drive, None, None)
            .expect("expected to execute query");

        assert_eq!(results_on_transaction.len(), 0);
    }

    #[test]
    fn test_add_and_remove_family_one_document() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        let db_transaction = drive.grove.start_transaction();

        drive
            .create_initial_state_structure(Some(&db_transaction))
            .expect("expected to create root tree successfully");

        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/family/family-contract-reduced.json",
            None,
            Some(&db_transaction),
        );

        let person_serialized_document = json_document_to_cbor(
            "tests/supporting_files/contract/family/person0.json",
            Some(1),
        );

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();

        let document =
            Document::from_cbor(&person_serialized_document, None, Some(random_owner_id))
                .expect("expected to deserialize the document");

        let document_type = contract
            .document_type_for_name("person")
            .expect("expected to get a document type");

        let storage_flags = Some(StorageFlags::SingleEpoch(0));

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefAndSerialization((
                            &document,
                            &person_serialized_document,
                            storage_flags.as_ref(),
                        )),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                Some(&db_transaction),
            )
            .expect("expected to insert a document successfully");

        drive
            .grove
            .commit_transaction(db_transaction)
            .unwrap()
            .expect("unable to commit transaction");

        let sql_string =
            "select * from person where firstName = 'Samuel' order by firstName asc limit 100";
        let query = DriveQuery::from_sql_expr(sql_string, &contract).expect("should build query");

        let (results_no_transaction, _, _) = query
            .execute_no_proof(&drive, None, None)
            .expect("expected to execute query");

        assert_eq!(results_no_transaction.len(), 1);

        let db_transaction = drive.grove.start_transaction();

        let (results_on_transaction, _, _) = query
            .execute_no_proof(&drive, None, Some(&db_transaction))
            .expect("expected to execute query");

        assert_eq!(results_on_transaction.len(), 1);
        let document_id = bs58::decode("AYjYxDqLy2hvGQADqE6FAkBnQEpJSzNd3CRw1tpS6vZ7")
            .into_vec()
            .expect("should decode")
            .as_slice()
            .try_into()
            .expect("this be 32 bytes");

        drive
            .delete_document_for_contract(
                document_id,
                &contract,
                "person",
                Some(random_owner_id),
                BlockInfo::default(),
                true,
                Some(&db_transaction),
            )
            .expect("expected to be able to delete the document");

        drive
            .grove
            .commit_transaction(db_transaction)
            .unwrap()
            .expect("unable to commit transaction");

        let db_transaction = drive.grove.start_transaction();

        let (results_on_transaction, _, _) = query
            .execute_no_proof(&drive, None, Some(&db_transaction))
            .expect("expected to execute query");

        assert_eq!(results_on_transaction.len(), 0);
    }

    #[test]
    fn test_add_and_remove_family_documents() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        let db_transaction = drive.grove.start_transaction();

        drive
            .create_initial_state_structure(Some(&db_transaction))
            .expect("expected to create root tree successfully");

        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/family/family-contract-reduced.json",
            None,
            Some(&db_transaction),
        );

        let person_serialized_document = json_document_to_cbor(
            "tests/supporting_files/contract/family/person0.json",
            Some(1),
        );

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();

        let document =
            Document::from_cbor(&person_serialized_document, None, Some(random_owner_id))
                .expect("expected to deserialize the document");

        let document_type = contract
            .document_type_for_name("person")
            .expect("expected to get a document type");

        let storage_flags = Some(StorageFlags::SingleEpoch(0));

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefAndSerialization((
                            &document,
                            &person_serialized_document,
                            storage_flags.as_ref(),
                        )),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                Some(&db_transaction),
            )
            .expect("expected to insert a document successfully");

        let person_serialized_document = json_document_to_cbor(
            "tests/supporting_files/contract/family/person1.json",
            Some(1),
        );

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();

        let document =
            Document::from_cbor(&person_serialized_document, None, Some(random_owner_id))
                .expect("expected to deserialize the document");

        let document_type = contract
            .document_type_for_name("person")
            .expect("expected to get a document type");

        let storage_flags = Some(StorageFlags::SingleEpoch(0));

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefAndSerialization((
                            &document,
                            &person_serialized_document,
                            storage_flags.as_ref(),
                        )),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                Some(&db_transaction),
            )
            .expect("expected to insert a document successfully");

        drive
            .grove
            .commit_transaction(db_transaction)
            .unwrap()
            .expect("unable to commit transaction");

        let sql_string =
            "select * from person where firstName > 'A' order by firstName asc limit 5";
        let query = DriveQuery::from_sql_expr(sql_string, &contract).expect("should build query");

        let (results_no_transaction, _, _) = query
            .execute_no_proof(&drive, None, None)
            .expect("expected to execute query");

        assert_eq!(results_no_transaction.len(), 2);

        let document_id = bs58::decode("8wjx2TC1vj2grssQvdwWnksNLwpi4xKraYy1TbProgd4")
            .into_vec()
            .expect("should decode")
            .as_slice()
            .try_into()
            .expect("this be 32 bytes");

        let db_transaction = drive.grove.start_transaction();

        drive
            .delete_document_for_contract(
                document_id,
                &contract,
                "person",
                Some(random_owner_id),
                BlockInfo::default(),
                true,
                Some(&db_transaction),
            )
            .expect("expected to be able to delete the document");

        drive
            .grove
            .commit_transaction(db_transaction)
            .unwrap()
            .expect("unable to commit transaction");

        let sql_string =
            "select * from person where firstName > 'A' order by firstName asc limit 5";
        let query = DriveQuery::from_sql_expr(sql_string, &contract).expect("should build query");

        let (results_no_transaction, _, _) = query
            .execute_no_proof(&drive, None, None)
            .expect("expected to execute query");

        assert_eq!(results_no_transaction.len(), 1);

        let document_id = bs58::decode("AYjYxDqLy2hvGQADqE6FAkBnQEpJSzNd3CRw1tpS6vZ7")
            .into_vec()
            .expect("should decode")
            .as_slice()
            .try_into()
            .expect("this be 32 bytes");

        let db_transaction = drive.grove.start_transaction();

        drive
            .delete_document_for_contract(
                document_id,
                &contract,
                "person",
                Some(random_owner_id),
                BlockInfo::default(),
                true,
                Some(&db_transaction),
            )
            .expect("expected to be able to delete the document");

        drive
            .grove
            .commit_transaction(db_transaction)
            .unwrap()
            .expect("unable to commit transaction");

        let sql_string =
            "select * from person where firstName > 'A' order by firstName asc limit 5";
        let query = DriveQuery::from_sql_expr(sql_string, &contract).expect("should build query");

        let (results_no_transaction, _, _) = query
            .execute_no_proof(&drive, None, None)
            .expect("expected to execute query");

        assert_eq!(results_no_transaction.len(), 0);
    }

    #[test]
    fn test_add_and_remove_family_documents_with_empty_fields() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        let db_transaction = drive.grove.start_transaction();

        drive
            .create_initial_state_structure(Some(&db_transaction))
            .expect("expected to create root tree successfully");

        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/family/family-contract-reduced.json",
            None,
            Some(&db_transaction),
        );

        let person_serialized_document = json_document_to_cbor(
            "tests/supporting_files/contract/family/person0.json",
            Some(1),
        );

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();

        let document =
            Document::from_cbor(&person_serialized_document, None, Some(random_owner_id))
                .expect("expected to deserialize the document");

        let document_type = contract
            .document_type_for_name("person")
            .expect("expected to get a document type");

        let storage_flags = Some(StorageFlags::SingleEpoch(0));

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefAndSerialization((
                            &document,
                            &person_serialized_document,
                            storage_flags.as_ref(),
                        )),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                Some(&db_transaction),
            )
            .expect("expected to insert a document successfully");

        let person_serialized_document = json_document_to_cbor(
            "tests/supporting_files/contract/family/person2-no-middle-name.json",
            Some(1),
        );

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();

        let document =
            Document::from_cbor(&person_serialized_document, None, Some(random_owner_id))
                .expect("expected to deserialize the document");

        let document_type = contract
            .document_type_for_name("person")
            .expect("expected to get a document type");

        let storage_flags = Some(StorageFlags::SingleEpoch(0));

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefAndSerialization((
                            &document,
                            &person_serialized_document,
                            storage_flags.as_ref(),
                        )),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                Some(&db_transaction),
            )
            .expect("expected to insert a document successfully");

        drive
            .grove
            .commit_transaction(db_transaction)
            .unwrap()
            .expect("unable to commit transaction");

        let sql_string =
            "select * from person where firstName > 'A' order by firstName asc limit 5";
        let query = DriveQuery::from_sql_expr(sql_string, &contract).expect("should build query");

        let (results_no_transaction, _, _) = query
            .execute_no_proof(&drive, None, None)
            .expect("expected to execute query");

        assert_eq!(results_no_transaction.len(), 2);

        let document_id = bs58::decode("BZjYxDqLy2hvGQADqE6FAkBnQEpJSzNd3CRw1tpS6vZ7")
            .into_vec()
            .expect("should decode")
            .as_slice()
            .try_into()
            .expect("this be 32 bytes");

        let db_transaction = drive.grove.start_transaction();

        drive
            .delete_document_for_contract(
                document_id,
                &contract,
                "person",
                Some(random_owner_id),
                BlockInfo::default(),
                true,
                Some(&db_transaction),
            )
            .expect("expected to be able to delete the document");

        drive
            .grove
            .commit_transaction(db_transaction)
            .unwrap()
            .expect("unable to commit transaction");

        // Let's try adding the document back after it was deleted

        let db_transaction = drive.grove.start_transaction();

        let document =
            Document::from_cbor(&person_serialized_document, None, Some(random_owner_id))
                .expect("expected to deserialize the document");

        let storage_flags = Some(StorageFlags::SingleEpoch(0));

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefAndSerialization((
                            &document,
                            &person_serialized_document,
                            storage_flags.as_ref(),
                        )),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                Some(&db_transaction),
            )
            .expect("expected to insert a document successfully");

        drive
            .grove
            .commit_transaction(db_transaction)
            .unwrap()
            .expect("unable to commit transaction");

        // Let's try removing all documents now

        let db_transaction = drive.grove.start_transaction();

        drive
            .delete_document_for_contract(
                document_id,
                &contract,
                "person",
                Some(random_owner_id),
                BlockInfo::default(),
                true,
                Some(&db_transaction),
            )
            .expect("expected to be able to delete the document");

        let document_id = bs58::decode("AYjYxDqLy2hvGQADqE6FAkBnQEpJSzNd3CRw1tpS6vZ7")
            .into_vec()
            .expect("should decode")
            .as_slice()
            .try_into()
            .expect("this be 32 bytes");

        drive
            .delete_document_for_contract(
                document_id,
                &contract,
                "person",
                Some(random_owner_id),
                BlockInfo::default(),
                true,
                Some(&db_transaction),
            )
            .expect("expected to be able to delete the document");

        drive
            .grove
            .commit_transaction(db_transaction)
            .unwrap()
            .expect("unable to commit transaction");

        let sql_string =
            "select * from person where firstName > 'A' order by firstName asc limit 5";
        let query = DriveQuery::from_sql_expr(sql_string, &contract).expect("should build query");

        let (results_no_transaction, _, _) = query
            .execute_no_proof(&drive, None, None)
            .expect("expected to execute query");

        assert_eq!(results_no_transaction.len(), 0);
    }

    #[test]
    fn test_delete_dashpay_documents_no_transaction() {
        let (drive, dashpay_cbor) = setup_dashpay("delete", false);

        let dashpay_profile_serialized_document = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/profile0.json",
            Some(1),
        );

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();
        drive
            .add_serialized_document_for_serialized_contract(
                &dashpay_profile_serialized_document,
                &dashpay_cbor,
                "profile",
                Some(random_owner_id),
                false,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_ref(),
                None,
            )
            .expect("expected to insert a document successfully");

        let document_id = bs58::decode("AM47xnyLfTAC9f61ZQPGfMK5Datk2FeYZwgYvcAnzqFY")
            .into_vec()
            .expect("should decode")
            .as_slice()
            .try_into()
            .expect("this be 32 bytes");

        drive
            .delete_document_for_contract_cbor(
                document_id,
                &dashpay_cbor,
                "profile",
                Some(random_owner_id),
                BlockInfo::default(),
                true,
                None,
            )
            .expect("expected to be able to delete the document");
    }

    #[test]
    fn test_delete_dashpay_documents() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        let db_transaction = drive.grove.start_transaction();

        drive
            .create_initial_state_structure(Some(&db_transaction))
            .expect("expected to create root tree successfully");

        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/dashpay/dashpay-contract.json",
            None,
            Some(&db_transaction),
        );

        let dashpay_profile_serialized_document = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/profile0.json",
            Some(1),
        );

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();
        let storage_flags = StorageFlags::SingleEpochOwned(0, random_owner_id);
        let fee_result = drive
            .add_serialized_document_for_contract(
                &dashpay_profile_serialized_document,
                &contract,
                "profile",
                Some(random_owner_id),
                false,
                BlockInfo::default(),
                true,
                Some(&storage_flags),
                Some(&db_transaction),
            )
            .expect("expected to insert a document successfully");

        let added_bytes = fee_result.storage_fee / STORAGE_DISK_USAGE_CREDIT_PER_BYTE;
        // We added 1682 bytes
        assert_eq!(added_bytes, 1682);

        let document_id = bs58::decode("AM47xnyLfTAC9f61ZQPGfMK5Datk2FeYZwgYvcAnzqFY")
            .into_vec()
            .expect("should decode")
            .as_slice()
            .try_into()
            .expect("this be 32 bytes");

        // Let's delete the document at the third epoch
        let fee_result = drive
            .delete_document_for_contract(
                document_id,
                &contract,
                "profile",
                Some(random_owner_id),
                BlockInfo::default_with_epoch(Epoch::new(3)),
                true,
                Some(&db_transaction),
            )
            .expect("expected to be able to delete the document");

        let removed_credits = fee_result
            .fee_refunds
            .get(&random_owner_id)
            .unwrap()
            .get(&0)
            .unwrap();

        let removed_bytes = removed_credits.to_unsigned() / STORAGE_DISK_USAGE_CREDIT_PER_BYTE;

        assert_eq!(added_bytes, removed_bytes);
    }

    #[test]
    fn test_delete_dashpay_documents_without_apply() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        let db_transaction = drive.grove.start_transaction();

        drive
            .create_initial_state_structure(Some(&db_transaction))
            .expect("expected to create root tree successfully");

        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/dashpay/dashpay-contract.json",
            None,
            Some(&db_transaction),
        );

        let dashpay_profile_serialized_document = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/profile0.json",
            Some(1),
        );

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();
        let storage_flags = StorageFlags::SingleEpochOwned(0, random_owner_id);
        let fee_result = drive
            .add_serialized_document_for_contract(
                &dashpay_profile_serialized_document,
                &contract,
                "profile",
                Some(random_owner_id),
                false,
                BlockInfo::default(),
                true,
                Some(&storage_flags),
                Some(&db_transaction),
            )
            .expect("expected to insert a document successfully");

        let added_bytes = fee_result.storage_fee / STORAGE_DISK_USAGE_CREDIT_PER_BYTE;
        // We added 1682 bytes
        assert_eq!(added_bytes, 1682);

        let document_id = bs58::decode("AM47xnyLfTAC9f61ZQPGfMK5Datk2FeYZwgYvcAnzqFY")
            .into_vec()
            .expect("should decode")
            .as_slice()
            .try_into()
            .expect("this be 32 bytes");

        // Let's delete the document at the third epoch
        let fee_result = drive
            .delete_document_for_contract(
                document_id,
                &contract,
                "profile",
                Some(random_owner_id),
                BlockInfo::default_with_epoch(Epoch::new(3)),
                false,
                Some(&db_transaction),
            )
            .expect("expected to be able to delete the document");

        assert!(fee_result.fee_refunds.0.is_empty());
        assert_eq!(fee_result.storage_fee, 0);
        assert_eq!(fee_result.processing_fee, 148212400);
    }

    #[test]
    fn test_deletion_real_data() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        let db_transaction = drive.grove.start_transaction();

        drive
            .create_initial_state_structure(Some(&db_transaction))
            .expect("expected to create root tree successfully");

        let contract = setup_contract_from_hex(
            &drive,
            "01000000a5632469645820e8f72680f2e3910c95e1497a2b0029d9f7374891ac1f39ab1cfe3ae63336b9a96724736368656d61783468747470733a2f2f736368656d612e646173682e6f72672f6470702d302d342d302f6d6574612f646174612d636f6e7472616374676f776e6572496458209e412570bf3b7ce068b9bce81c569ce701e43edaea80b62a2773be7d21038b266776657273696f6e0169646f63756d656e7473a76b756e697175654461746573a56474797065666f626a65637467696e646963657382a3646e616d6566696e6465783166756e69717565f56a70726f7065727469657382a16a2463726561746564417463617363a16a2475706461746564417463617363a2646e616d6566696e646578326a70726f7065727469657381a16a2475706461746564417463617363687265717569726564836966697273744e616d656a246372656174656441746a247570646174656441746a70726f70657274696573a2686c6173744e616d65a1647479706566737472696e676966697273744e616d65a1647479706566737472696e67746164646974696f6e616c50726f70657274696573f46c6e696365446f63756d656e74a56474797065666f626a65637467696e646963657384a2646e616d6566696e646578316a70726f7065727469657381a1646e616d6563617363a2646e616d6566696e646578336a70726f7065727469657381a1656f7264657263617363a2646e616d6566696e646578346a70726f7065727469657381a1686c6173744e616d6563617363a2646e616d6567696e64657831306a70726f7065727469657381a168246f776e657249646464657363687265717569726564816a246372656174656441746a70726f70657274696573a3646e616d65a1647479706566737472696e67656f72646572a16474797065666e756d626572686c6173744e616d65a1647479706566737472696e67746164646974696f6e616c50726f70657274696573f46e6e6f54696d65446f63756d656e74a36474797065666f626a6563746a70726f70657274696573a1646e616d65a1647479706566737472696e67746164646974696f6e616c50726f70657274696573f46e707265747479446f63756d656e74a46474797065666f626a65637468726571756972656482686c6173744e616d656a247570646174656441746a70726f70657274696573a1686c6173744e616d65a1647479706566737472696e67746164646974696f6e616c50726f70657274696573f46e7769746842797465417272617973a56474797065666f626a65637467696e646963657381a2646e616d6566696e646578316a70726f7065727469657381a16e6279746541727261794669656c6463617363687265717569726564816e6279746541727261794669656c646a70726f70657274696573a26e6279746541727261794669656c64a36474797065656172726179686d61784974656d731069627974654172726179f56f6964656e7469666965724669656c64a56474797065656172726179686d61784974656d731820686d696e4974656d73182069627974654172726179f570636f6e74656e744d656469615479706578216170706c69636174696f6e2f782e646173682e6470702e6964656e746966696572746164646974696f6e616c50726f70657274696573f46f696e6465786564446f63756d656e74a56474797065666f626a65637467696e646963657386a3646e616d6566696e6465783166756e69717565f56a70726f7065727469657382a168246f776e6572496463617363a16966697273744e616d656464657363a3646e616d6566696e6465783266756e69717565f56a70726f7065727469657382a168246f776e6572496463617363a1686c6173744e616d656464657363a2646e616d6566696e646578336a70726f7065727469657381a1686c6173744e616d6563617363a2646e616d6566696e646578346a70726f7065727469657382a16a2463726561746564417463617363a16a2475706461746564417463617363a2646e616d6566696e646578356a70726f7065727469657381a16a2475706461746564417463617363a2646e616d6566696e646578366a70726f7065727469657381a16a2463726561746564417463617363687265717569726564846966697273744e616d656a246372656174656441746a24757064617465644174686c6173744e616d656a70726f70657274696573a2686c6173744e616d65a2647479706566737472696e67696d61784c656e677468183f6966697273744e616d65a2647479706566737472696e67696d61784c656e677468183f746164646974696f6e616c50726f70657274696573f4781d6f7074696f6e616c556e69717565496e6465786564446f63756d656e74a56474797065666f626a65637467696e646963657383a3646e616d6566696e6465783166756e69717565f56a70726f7065727469657381a16966697273744e616d656464657363a3646e616d6566696e6465783266756e69717565f56a70726f7065727469657383a168246f776e6572496463617363a16966697273744e616d6563617363a1686c6173744e616d6563617363a3646e616d6566696e6465783366756e69717565f56a70726f7065727469657382a167636f756e74727963617363a1646369747963617363687265717569726564826966697273744e616d65686c6173744e616d656a70726f70657274696573a46463697479a2647479706566737472696e67696d61784c656e677468183f67636f756e747279a2647479706566737472696e67696d61784c656e677468183f686c6173744e616d65a2647479706566737472696e67696d61784c656e677468183f6966697273744e616d65a2647479706566737472696e67696d61784c656e677468183f746164646974696f6e616c50726f70657274696573f4".to_string(),
            Some(&db_transaction),
        );

        let document_hexes = [
            "01000000a86324696458208fcfbce88a219c6e6f4cca4aa55c1ba08303d62985d94084a28d3c298753b8a6646e616d656543757469656524747970656c6e696365446f63756d656e74656f726465720068246f776e657249645820cac675648b485d2606a53fca9942cb7bfdf34e08cee1ebe6e0e74e8502ac6c8069247265766973696f6e016a246372656174656441741b0000017f9334371f6f2464617461436f6e747261637449645820e8f72680f2e3910c95e1497a2b0029d9f7374891ac1f39ab1cfe3ae63336b9a9",
            "01000000a863246964582067a18898a8bfdd139353359d907d487b45d62ab4694a63ad1fe34a34cd8c42116524747970656c6e696365446f63756d656e74656f726465720168246f776e657249645820cac675648b485d2606a53fca9942cb7bfdf34e08cee1ebe6e0e74e8502ac6c80686c6173744e616d65655368696e7969247265766973696f6e016a247570646174656441741b0000017f9334371f6f2464617461436f6e747261637449645820e8f72680f2e3910c95e1497a2b0029d9f7374891ac1f39ab1cfe3ae63336b9a9",
            "01000000a863246964582091bf487b6041e26d7e22a4a10d544fb733daba7b60ef8ed557bb21fd722bdd036524747970656c6e696365446f63756d656e74656f726465720268246f776e657249645820cac675648b485d2606a53fca9942cb7bfdf34e08cee1ebe6e0e74e8502ac6c80686c6173744e616d656653776565747969247265766973696f6e016a247570646174656441741b0000017f9334371f6f2464617461436f6e747261637449645820e8f72680f2e3910c95e1497a2b0029d9f7374891ac1f39ab1cfe3ae63336b9a9",
            "01000000aa632469645820a2869e44207381542b144f22a65b961e5ddf489d68d7a720144bee223a0555956524747970656c6e696365446f63756d656e74656f726465720368246f776e657249645820cac675648b485d2606a53fca9942cb7bfdf34e08cee1ebe6e0e74e8502ac6c80686c6173744e616d65664269726b696e69247265766973696f6e016966697273744e616d656757696c6c69616d6a246372656174656441741b0000017f933437206a247570646174656441741b0000017f933437206f2464617461436f6e747261637449645820e8f72680f2e3910c95e1497a2b0029d9f7374891ac1f39ab1cfe3ae63336b9a9",
            "01000000aa6324696458208d2a661748268018725cf0dc612c74cf1e8621dc86c5e9cc64d2bbe17a2f855a6524747970656c6e696365446f63756d656e74656f726465720468246f776e657249645820cac675648b485d2606a53fca9942cb7bfdf34e08cee1ebe6e0e74e8502ac6c80686c6173744e616d65674b656e6e65647969247265766973696f6e016966697273744e616d65644c656f6e6a246372656174656441741b0000017f933437206a247570646174656441741b0000017f933437206f2464617461436f6e747261637449645820e8f72680f2e3910c95e1497a2b0029d9f7374891ac1f39ab1cfe3ae63336b9a9"
        ];

        let storage_flags = Some(StorageFlags::SingleEpoch(0));

        let documents: Vec<Document> = document_hexes
            .iter()
            .map(|document_hex| {
                let serialized_document = cbor_from_hex(document_hex.to_string());

                let document = Document::from_cbor(&serialized_document, None, None)
                    .expect("expected to deserialize the document");

                let document_type = contract
                    .document_type_for_name("niceDocument")
                    .expect("expected to get a document type");

                drive
                    .add_document_for_contract(
                        DocumentAndContractInfo {
                            owned_document_info: OwnedDocumentInfo {
                                document_info: DocumentRefAndSerialization((
                                    &document,
                                    &serialized_document,
                                    storage_flags.as_ref(),
                                )),
                                owner_id: None,
                            },
                            contract: &contract,
                            document_type,
                        },
                        false,
                        BlockInfo::default(),
                        true,
                        Some(&db_transaction),
                    )
                    .expect("expected to insert a document successfully");

                document
            })
            .collect();

        let document_id = "AgP2Tx2ayfobSQ6xZCEVLzfmmLD4YR3CNAJcfgZfBcY5";

        let query_json = json!({
            "where": [
                ["$id", "==", String::from(document_id)]
            ],
        });

        let query_cbor = value_to_cbor(query_json, None);

        drive
            .grove
            .commit_transaction(db_transaction)
            .unwrap()
            .expect("unable to commit transaction");

        let (results, _, _) = drive
            .query_documents_from_contract(
                &contract,
                contract.document_types().get("niceDocument").unwrap(),
                query_cbor.as_slice(),
                None,
                None,
            )
            .expect("expected to execute query");

        assert_eq!(results.len(), 1);

        let db_transaction = drive.grove.start_transaction();

        drive
            .delete_document_for_contract(
                documents.get(0).unwrap().id,
                &contract,
                "niceDocument",
                Some(documents.get(0).unwrap().owner_id),
                BlockInfo::default(),
                true,
                Some(&db_transaction),
            )
            .expect("expected to be able to delete the document");

        let query_json = json!({
            "where": [
                ["$id", "==", String::from(document_id)]
            ],
        });

        let query_cbor = value_to_cbor(query_json, None);

        let (results, _, _) = drive
            .query_documents_from_contract(
                &contract,
                contract.document_types().get("niceDocument").unwrap(),
                query_cbor.as_slice(),
                None,
                Some(&db_transaction),
            )
            .expect("expected to execute query");

        assert_eq!(results.len(), 0);
    }
}
