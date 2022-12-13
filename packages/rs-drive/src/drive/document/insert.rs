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

//! Insert Documents.
//!
//! This module implements functions in Drive relevant to inserting documents.
//!

use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::key_info::KeyInfo::KnownKey;
use grovedb::batch::KeyInfoPath;
use grovedb::reference_path::ReferencePathType::SiblingReference;
use grovedb::EstimatedLayerCount::{ApproximateElements, PotentiallyAtMaxElements};
use grovedb::EstimatedLayerSizes::AllSubtrees;
use grovedb::EstimatedSumTrees::NoSumTrees;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;
use std::option::Option::None;

use crate::contract::Contract;
use crate::drive::defaults::{DEFAULT_HASH_SIZE_U8, STORAGE_FLAGS_SIZE};
use crate::drive::document::{
    contract_document_type_path_vec,
    contract_documents_keeping_history_primary_key_path_for_document_id,
    contract_documents_keeping_history_primary_key_path_for_unknown_document_id,
    contract_documents_keeping_history_storage_time_reference_path_size,
    contract_documents_primary_key_path, document_reference_size, make_document_reference,
    unique_event_id,
};
use crate::drive::flags::StorageFlags;
use crate::drive::object_size_info::DocumentInfo::{
    DocumentEstimatedAverageSize, DocumentRefAndSerialization, DocumentRefWithoutSerialization,
    DocumentWithoutSerialization,
};
use crate::drive::object_size_info::DriveKeyInfo::{Key, KeyRef};
use crate::drive::object_size_info::KeyElementInfo::{KeyElement, KeyUnknownElementSize};
use crate::drive::object_size_info::PathKeyElementInfo::{
    PathFixedSizeKeyElement, PathKeyUnknownElementSize,
};
use crate::drive::object_size_info::PathKeyInfo::{PathFixedSizeKeyRef, PathKeySize};
use crate::drive::object_size_info::{DocumentAndContractInfo, PathInfo, PathKeyElementInfo};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::op::DriveOperation;
use crate::fee::{calculate_fee, FeeResult};

use crate::common::encode::encode_unsigned_integer;
use crate::contract::document::Document;
use crate::drive::block_info::BlockInfo;
use crate::drive::grove_operations::DirectQueryType::{StatefulDirectQuery, StatelessDirectQuery};
use crate::drive::grove_operations::QueryTarget::QueryTargetValue;
use crate::drive::grove_operations::{BatchInsertApplyType, BatchInsertTreeApplyType};
use crate::error::document::DocumentError;
use crate::error::fee::FeeError;
use dpp::data_contract::extra::{DriveContractExt, IndexLevel};

impl Drive {
    /// Adds a document to primary storage.
    /// If a document isn't sent to this function then we are just calling to know the query and
    /// insert operations
    pub(crate) fn add_document_to_primary_storage(
        &self,
        document_and_contract_info: &DocumentAndContractInfo,
        block_info: &BlockInfo,
        insert_without_check: bool,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error> {
        //let mut base_operations : EnumMap<Op, u64> = EnumMap::default();
        let contract = document_and_contract_info.contract;
        let document_type = document_and_contract_info.document_type;
        let primary_key_path = contract_documents_primary_key_path(
            contract.id().as_bytes(),
            document_type.name.as_str(),
        );
        // if we are trying to get estimated costs we should add this level
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_add_document_to_primary_storage(
                document_and_contract_info,
                primary_key_path,
                estimated_costs_only_with_layer_info,
            );
        }

        if document_type.documents_keep_history {
            let (path_key_info, storage_flags) =
                if document_and_contract_info.document_info.is_document_size() {
                    (
                        PathKeySize(
                            KeyInfoPath::from_known_path(primary_key_path),
                            KeyInfo::MaxKeySize {
                                unique_id: document_type.unique_id_for_storage().to_vec(),
                                max_size: DEFAULT_HASH_SIZE_U8,
                            },
                        ),
                        StorageFlags::optional_default_as_ref(),
                    )
                } else {
                    let inserted_storage_flags = if contract.can_be_deleted() {
                        document_and_contract_info
                            .document_info
                            .get_storage_flags_ref()
                    } else {
                        // there are no need for storage flags if the contract can not be deleted
                        // as this tree can never be deleted
                        None
                    };
                    (
                        PathFixedSizeKeyRef((
                            primary_key_path,
                            document_and_contract_info
                                .document_info
                                .get_document_id_as_slice()
                                .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                                    "can not get document id from estimated document",
                                )))?,
                        )),
                        inserted_storage_flags,
                    )
                };
            let apply_type = if estimated_costs_only_with_layer_info.is_none() {
                BatchInsertTreeApplyType::StatefulBatchInsert
            } else {
                BatchInsertTreeApplyType::StatelessBatchInsert {
                    in_tree_using_sums: false,
                    is_sum_tree: false,
                    flags_len: storage_flags
                        .map(|s| s.serialized_size())
                        .unwrap_or_default(),
                }
            };
            // we first insert an empty tree if the document is new
            self.batch_insert_empty_tree_if_not_exists(
                path_key_info,
                storage_flags,
                apply_type,
                transaction,
                drive_operations,
            )?;
            let encoded_time = encode_unsigned_integer(block_info.time_ms)?;
            let path_key_element_info = match &document_and_contract_info.document_info {
                DocumentRefAndSerialization((document, serialized_document, storage_flags)) => {
                    let element = Element::Item(
                        serialized_document.to_vec(),
                        StorageFlags::map_to_some_element_flags(*storage_flags),
                    );
                    let document_id_in_primary_path =
                        contract_documents_keeping_history_primary_key_path_for_document_id(
                            contract.id().as_bytes(),
                            document_type.name.as_str(),
                            document.id.as_slice(),
                        );
                    PathFixedSizeKeyElement((
                        document_id_in_primary_path,
                        encoded_time.as_slice(),
                        element,
                    ))
                }
                DocumentWithoutSerialization((document, storage_flags)) => {
                    let serialized_document =
                        document.serialize(document_and_contract_info.document_type)?;
                    let element = Element::Item(
                        serialized_document,
                        StorageFlags::map_to_some_element_flags(storage_flags.as_ref()),
                    );
                    let document_id_in_primary_path =
                        contract_documents_keeping_history_primary_key_path_for_document_id(
                            contract.id.as_bytes(),
                            document_type.name.as_str(),
                            document.id.as_slice(),
                        );
                    PathFixedSizeKeyElement((
                        document_id_in_primary_path,
                        encoded_time.as_slice(),
                        element,
                    ))
                }
                DocumentRefWithoutSerialization((document, storage_flags)) => {
                    let serialized_document =
                        document.serialize(document_and_contract_info.document_type)?;
                    let element = Element::Item(
                        serialized_document,
                        StorageFlags::map_to_some_element_flags(*storage_flags),
                    );
                    let document_id_in_primary_path =
                        contract_documents_keeping_history_primary_key_path_for_document_id(
                            contract.id.as_bytes(),
                            document_type.name.as_str(),
                            document.id.as_slice(),
                        );
                    PathFixedSizeKeyElement((
                        document_id_in_primary_path,
                        encoded_time.as_slice(),
                        element,
                    ))
                }
                DocumentEstimatedAverageSize(max_size) => {
                    let document_id_in_primary_path =
                        contract_documents_keeping_history_primary_key_path_for_unknown_document_id(
                            contract.id.as_bytes(),
                            document_type,
                        );
                    PathKeyUnknownElementSize((
                        document_id_in_primary_path,
                        KnownKey(encoded_time.clone()),
                        Element::required_item_space(*max_size, STORAGE_FLAGS_SIZE),
                    ))
                }
            };
            self.batch_insert(path_key_element_info, drive_operations)?;

            let path_key_element_info =
                if document_and_contract_info.document_info.is_document_size() {
                    let document_id_in_primary_path =
                        contract_documents_keeping_history_primary_key_path_for_unknown_document_id(
                            contract.id.as_bytes(),
                            document_type,
                        );
                    let reference_max_size =
                        contract_documents_keeping_history_storage_time_reference_path_size(
                            document_type.name.len() as u32,
                        );
                    PathKeyUnknownElementSize((
                        document_id_in_primary_path,
                        KnownKey(vec![0]),
                        Element::required_item_space(reference_max_size, STORAGE_FLAGS_SIZE),
                    ))
                } else {
                    // we should also insert a reference at 0 to the current value
                    // todo: we could construct this only once
                    let document_id_in_primary_path =
                        contract_documents_keeping_history_primary_key_path_for_document_id(
                            contract.id.as_bytes(),
                            document_type.name.as_str(),
                            document_and_contract_info
                                .document_info
                                .get_document_id_as_slice()
                                .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                                    "can not get document id from estimated document",
                                )))?,
                        );
                    PathFixedSizeKeyElement((
                        document_id_in_primary_path,
                        &[0],
                        Element::Reference(
                            SiblingReference(encoded_time),
                            Some(1),
                            StorageFlags::map_to_some_element_flags(storage_flags),
                        ),
                    ))
                };

            self.batch_insert(path_key_element_info, drive_operations)?;
        } else if insert_without_check {
            let path_key_element_info = match &document_and_contract_info.document_info {
                DocumentRefAndSerialization((document, serialized_document, storage_flags)) => {
                    let element = Element::Item(
                        serialized_document.to_vec(),
                        StorageFlags::map_to_some_element_flags(*storage_flags),
                    );
                    PathFixedSizeKeyElement((primary_key_path, document.id.as_slice(), element))
                }
                DocumentRefWithoutSerialization((document, storage_flags)) => {
                    let serialized_document =
                        document.serialize(document_and_contract_info.document_type)?;
                    let element = Element::Item(
                        serialized_document,
                        StorageFlags::map_to_some_element_flags(*storage_flags),
                    );
                    PathFixedSizeKeyElement((primary_key_path, document.id.as_slice(), element))
                }
                DocumentEstimatedAverageSize(average_size) => PathKeyUnknownElementSize((
                    KeyInfoPath::from_known_path(primary_key_path),
                    KeyInfo::MaxKeySize {
                        unique_id: document_type.unique_id_for_storage().to_vec(),
                        max_size: DEFAULT_HASH_SIZE_U8,
                    },
                    Element::required_item_space(*average_size, STORAGE_FLAGS_SIZE),
                )),
                DocumentWithoutSerialization((document, storage_flags)) => {
                    let serialized_document =
                        document.serialize(document_and_contract_info.document_type)?;
                    let element = Element::Item(
                        serialized_document,
                        StorageFlags::map_to_some_element_flags(storage_flags.as_ref()),
                    );
                    PathFixedSizeKeyElement((primary_key_path, document.id.as_slice(), element))
                }
            };
            self.batch_insert(path_key_element_info, drive_operations)?;
        } else {
            let path_key_element_info = match &document_and_contract_info.document_info {
                DocumentRefAndSerialization((document, serialized_document, storage_flags)) => {
                    let element = Element::Item(
                        serialized_document.to_vec(),
                        StorageFlags::map_to_some_element_flags(*storage_flags),
                    );
                    PathFixedSizeKeyElement((primary_key_path, document.id.as_slice(), element))
                }
                DocumentWithoutSerialization((document, storage_flags)) => {
                    let serialized_document =
                        document.serialize(document_and_contract_info.document_type)?;
                    let element = Element::Item(
                        serialized_document,
                        StorageFlags::map_to_some_element_flags(storage_flags.as_ref()),
                    );
                    PathFixedSizeKeyElement((primary_key_path, document.id.as_slice(), element))
                }
                DocumentRefWithoutSerialization((document, storage_flags)) => {
                    let serialized_document =
                        document.serialize(document_and_contract_info.document_type)?;
                    let element = Element::Item(
                        serialized_document,
                        StorageFlags::map_to_some_element_flags(*storage_flags),
                    );
                    PathFixedSizeKeyElement((primary_key_path, document.id.as_slice(), element))
                }
                DocumentEstimatedAverageSize(max_size) => PathKeyUnknownElementSize((
                    KeyInfoPath::from_known_path(primary_key_path),
                    KeyInfo::MaxKeySize {
                        unique_id: document_type.unique_id_for_storage().to_vec(),
                        max_size: DEFAULT_HASH_SIZE_U8,
                    },
                    Element::required_item_space(*max_size, STORAGE_FLAGS_SIZE),
                )),
            };
            let apply_type = if estimated_costs_only_with_layer_info.is_none() {
                BatchInsertApplyType::StatefulBatchInsert
            } else {
                BatchInsertApplyType::StatelessBatchInsert {
                    in_tree_using_sums: false,
                    target: QueryTargetValue(document_type.estimated_size() as u32),
                }
            };
            let inserted = self.batch_insert_if_not_exists(
                path_key_element_info,
                apply_type,
                transaction,
                drive_operations,
            )?;
            if !inserted {
                return Err(Error::Drive(DriveError::CorruptedDocumentAlreadyExists(
                    "item already exists",
                )));
            }
        }
        Ok(())
    }

    /// To do
    pub fn add_document(&self, _serialized_document: &[u8]) -> Result<(), Error> {
        todo!()
    }

    /// Deserializes a document and a contract and adds the document to the contract.
    pub fn add_serialized_document_for_serialized_contract(
        &self,
        serialized_document: &[u8],
        serialized_contract: &[u8],
        document_type_name: &str,
        owner_id: Option<[u8; 32]>,
        override_document: bool,
        block_info: BlockInfo,
        apply: bool,
        storage_flags: Option<&StorageFlags>,
        transaction: TransactionArg,
    ) -> Result<FeeResult, Error> {
        let contract = <Contract as DriveContractExt>::from_cbor(serialized_contract, None)?;

        let document = Document::from_cbor(serialized_document, None, owner_id)?;

        let document_info =
            DocumentRefAndSerialization((&document, serialized_document, storage_flags));

        let document_type = contract.document_type_for_name(document_type_name)?;

        self.add_document_for_contract(
            DocumentAndContractInfo {
                document_info,
                contract: &contract,
                document_type,
                owner_id,
            },
            override_document,
            block_info,
            apply,
            transaction,
        )
    }

    /// Deserializes a document and adds it to a contract.
    pub fn add_serialized_document_for_contract(
        &self,
        serialized_document: &[u8],
        contract: &Contract,
        document_type_name: &str,
        owner_id: Option<[u8; 32]>,
        override_document: bool,
        block_info: BlockInfo,
        apply: bool,
        storage_flags: Option<&StorageFlags>,
        transaction: TransactionArg,
    ) -> Result<FeeResult, Error> {
        let document = Document::from_cbor(serialized_document, None, owner_id)?;

        let document_info =
            DocumentRefAndSerialization((&document, serialized_document, storage_flags));

        let document_type = contract.document_type_for_name(document_type_name)?;

        self.add_document_for_contract(
            DocumentAndContractInfo {
                document_info,
                contract,
                document_type,
                owner_id,
            },
            override_document,
            block_info,
            apply,
            transaction,
        )
    }

    /// Deserializes a document and adds it to a contract by id.
    pub fn add_serialized_document_for_contract_id(
        &self,
        serialized_document: &[u8],
        contract_id: [u8; 32],
        document_type_name: &str,
        owner_id: Option<[u8; 32]>,
        override_document: bool,
        block_info: BlockInfo,
        apply: bool,
        storage_flags: Option<&StorageFlags>,
        transaction: TransactionArg,
    ) -> Result<FeeResult, Error> {
        let mut drive_operations: Vec<DriveOperation> = vec![];

        let contract_fetch_info = self
            .get_contract_with_fetch_info_and_add_to_operations(
                contract_id,
                Some(&block_info.epoch),
                transaction,
                &mut drive_operations,
            )?
            .ok_or(Error::Document(DocumentError::ContractNotFound()))?;

        let contract = &contract_fetch_info.contract;

        let document = Document::from_cbor(serialized_document, None, owner_id)?;

        let document_info =
            DocumentRefAndSerialization((&document, serialized_document, storage_flags));

        let document_type = contract.document_type_for_name(document_type_name)?;

        self.add_document_for_contract_apply_and_add_to_operations(
            DocumentAndContractInfo {
                document_info,
                contract,
                document_type,
                owner_id,
            },
            override_document,
            &block_info,
            apply,
            transaction,
            &mut drive_operations,
        )?;

        let fees = calculate_fee(None, Some(drive_operations), &block_info.epoch)?;

        Ok(fees)
    }

    /// Adds a document to a contract.
    pub fn add_document_for_contract(
        &self,
        document_and_contract_info: DocumentAndContractInfo,
        override_document: bool,
        block_info: BlockInfo,
        apply: bool,
        transaction: TransactionArg,
    ) -> Result<FeeResult, Error> {
        let mut drive_operations: Vec<DriveOperation> = vec![];
        self.add_document_for_contract_apply_and_add_to_operations(
            document_and_contract_info,
            override_document,
            &block_info,
            apply,
            transaction,
            &mut drive_operations,
        )?;
        let fees = calculate_fee(None, Some(drive_operations), &block_info.epoch)?;
        Ok(fees)
    }

    /// Performs the operations to add a document to a contract.
    pub(crate) fn add_document_for_contract_apply_and_add_to_operations(
        &self,
        document_and_contract_info: DocumentAndContractInfo,
        override_document: bool,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error> {
        let mut estimated_costs_only_with_layer_info = if apply {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };
        let batch_operations = self.add_document_for_contract_operations(
            document_and_contract_info,
            override_document,
            block_info,
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

    /// Adds the terminal reference.
    fn add_reference_for_index_level_for_contract_operations(
        &self,
        document_and_contract_info: &DocumentAndContractInfo,
        mut index_path_info: PathInfo<0>,
        unique: bool,
        any_fields_null: bool,
        storage_flags: &Option<&StorageFlags>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        batch_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error> {
        // unique indexes will be stored under key "0"
        // non unique indices should have a tree at key "0" that has all elements based off of primary key
        if !unique || any_fields_null {
            let key_path_info = KeyRef(&[0]);

            let path_key_info = key_path_info.add_path_info(index_path_info.clone());

            let apply_type = if estimated_costs_only_with_layer_info.is_none() {
                BatchInsertTreeApplyType::StatefulBatchInsert
            } else {
                BatchInsertTreeApplyType::StatelessBatchInsert {
                    in_tree_using_sums: false,
                    is_sum_tree: false,
                    flags_len: storage_flags
                        .map(|s| s.serialized_size())
                        .unwrap_or_default(),
                }
            };

            // here we are inserting an empty tree that will have a subtree of all other index properties
            self.batch_insert_empty_tree_if_not_exists(
                path_key_info,
                *storage_flags,
                apply_type,
                transaction,
                batch_operations,
            )?;

            index_path_info.push(Key(vec![0]))?;

            if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info
            {
                // On this level we will have a 0 and all the top index paths
                estimated_costs_only_with_layer_info.insert(
                    index_path_info.clone().convert_to_key_info_path(),
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

            let key_element_info = match &document_and_contract_info.document_info {
                DocumentRefAndSerialization((document, _, storage_flags))
                | DocumentRefWithoutSerialization((document, storage_flags)) => {
                    let document_reference = make_document_reference(
                        document,
                        document_and_contract_info.document_type,
                        *storage_flags,
                    );
                    KeyElement((document.id.as_slice(), document_reference))
                }
                DocumentWithoutSerialization((document, storage_flags)) => {
                    let document_reference = make_document_reference(
                        document,
                        document_and_contract_info.document_type,
                        storage_flags.as_ref(),
                    );
                    KeyElement((document.id.as_slice(), document_reference))
                }
                DocumentEstimatedAverageSize(max_size) => KeyUnknownElementSize((
                    KeyInfo::MaxKeySize {
                        unique_id: document_and_contract_info
                            .document_type
                            .unique_id_for_storage()
                            .to_vec(),
                        max_size: DEFAULT_HASH_SIZE_U8,
                    },
                    Element::required_item_space(*max_size, STORAGE_FLAGS_SIZE),
                )),
            };

            let path_key_element_info = PathKeyElementInfo::from_path_info_and_key_element(
                index_path_info,
                key_element_info,
            )?;

            // here we should return an error if the element already exists
            self.batch_insert(path_key_element_info, batch_operations)?;
        } else {
            let key_element_info = match &document_and_contract_info.document_info {
                DocumentRefAndSerialization((document, _, storage_flags))
                | DocumentRefWithoutSerialization((document, storage_flags)) => {
                    let document_reference = make_document_reference(
                        document,
                        document_and_contract_info.document_type,
                        *storage_flags,
                    );
                    KeyElement((&[0], document_reference))
                }
                DocumentWithoutSerialization((document, storage_flags)) => {
                    let document_reference = make_document_reference(
                        document,
                        document_and_contract_info.document_type,
                        storage_flags.as_ref(),
                    );
                    KeyElement((&[0], document_reference))
                }
                DocumentEstimatedAverageSize(estimated_size) => KeyUnknownElementSize((
                    KeyInfo::MaxKeySize {
                        unique_id: document_and_contract_info
                            .document_type
                            .unique_id_for_storage()
                            .to_vec(),
                        max_size: 1,
                    },
                    Element::required_item_space(*estimated_size, STORAGE_FLAGS_SIZE),
                )),
            };

            let path_key_element_info = PathKeyElementInfo::from_path_info_and_key_element(
                index_path_info,
                key_element_info,
            )?;

            let apply_type = if estimated_costs_only_with_layer_info.is_none() {
                BatchInsertApplyType::StatefulBatchInsert
            } else {
                BatchInsertApplyType::StatelessBatchInsert {
                    in_tree_using_sums: false,
                    target: QueryTargetValue(
                        document_reference_size(document_and_contract_info.document_type)
                            + storage_flags
                                .map(|s| s.serialized_size())
                                .unwrap_or_default(),
                    ),
                }
            };

            // here we should return an error if the element already exists
            let inserted = self.batch_insert_if_not_exists(
                path_key_element_info,
                apply_type,
                transaction,
                batch_operations,
            )?;
            if !inserted {
                return Err(Error::Drive(DriveError::CorruptedContractIndexes(
                    "reference already exists",
                )));
            }
        }
        Ok(())
    }

    /// Adds indices for an index level and recurses.
    fn add_indices_for_index_level_for_contract_operations(
        &self,
        document_and_contract_info: &DocumentAndContractInfo,
        index_path_info: PathInfo<0>,
        index_level: &IndexLevel,
        mut any_fields_null: bool,
        storage_flags: &Option<&StorageFlags>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        event_id: [u8; 32],
        transaction: TransactionArg,
        batch_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error> {
        if let Some(unique) = index_level.has_index_with_uniqueness {
            self.add_reference_for_index_level_for_contract_operations(
                document_and_contract_info,
                index_path_info.clone(),
                unique,
                any_fields_null,
                storage_flags,
                estimated_costs_only_with_layer_info,
                transaction,
                batch_operations,
            )?;
        }

        let document_type = document_and_contract_info.document_type;

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

        let apply_type = if estimated_costs_only_with_layer_info.is_none() {
            BatchInsertTreeApplyType::StatefulBatchInsert
        } else {
            BatchInsertTreeApplyType::StatelessBatchInsert {
                in_tree_using_sums: false,
                is_sum_tree: false,
                flags_len: storage_flags
                    .map(|s| s.serialized_size())
                    .unwrap_or_default(),
            }
        };

        // fourth we need to store a reference to the document for each index
        for (name, sub_level) in &index_level.sub_index_levels {
            let mut sub_level_index_path_info = index_path_info.clone();
            let index_property_key = KeyRef(name.as_bytes());

            let document_index_field = document_and_contract_info
                .document_info
                .get_raw_for_document_type(
                    name,
                    document_type,
                    document_and_contract_info.owner_id,
                    Some((sub_level, event_id)),
                )?
                .unwrap_or_default();

            let path_key_info = index_property_key
                .clone()
                .add_path_info(sub_level_index_path_info.clone());

            // here we are inserting an empty tree that will have a subtree of all other index properties
            self.batch_insert_empty_tree_if_not_exists(
                path_key_info.clone(),
                *storage_flags,
                apply_type,
                transaction,
                batch_operations,
            )?;

            sub_level_index_path_info.push(index_property_key)?;

            if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info
            {
                let document_top_field_estimated_size = document_and_contract_info
                    .document_info
                    .get_estimated_size_for_document_type(name, document_type)?;

                if document_top_field_estimated_size > u8::MAX as u16 {
                    return Err(Error::Fee(FeeError::Overflow(
                        "document top field is too big for being an index on delete",
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

            let path_key_info = document_index_field
                .clone()
                .add_path_info(sub_level_index_path_info.clone());

            // here we are inserting an empty tree that will have a subtree of all other index properties
            self.batch_insert_empty_tree_if_not_exists(
                path_key_info.clone(),
                *storage_flags,
                apply_type,
                transaction,
                batch_operations,
            )?;

            any_fields_null |= document_index_field.is_empty();

            // we push the actual value of the index path
            sub_level_index_path_info.push(document_index_field)?;
            // Iteration 1. the index path is now something like Contracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId/<ToUserId>/
            // Iteration 2. the index path is now something like Contracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId/<ToUserId>/accountReference/<accountReference>
            self.add_indices_for_index_level_for_contract_operations(
                document_and_contract_info,
                sub_level_index_path_info,
                sub_level,
                any_fields_null,
                storage_flags,
                estimated_costs_only_with_layer_info,
                event_id,
                transaction,
                batch_operations,
            )?;
        }
        Ok(())
    }

    /// Adds indices for the top index level and calls for lower levels.
    fn add_indices_for_top_index_level_for_contract_operations(
        &self,
        document_and_contract_info: &DocumentAndContractInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        batch_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error> {
        let index_level = &document_and_contract_info.document_type.index_structure;
        let contract = document_and_contract_info.contract;
        let event_id = unique_event_id();
        let document_type = document_and_contract_info.document_type;
        let storage_flags = if document_type.documents_mutable || contract.can_be_deleted() {
            document_and_contract_info
                .document_info
                .get_storage_flags_ref()
        } else {
            None //there are no need for storage flags if documents are not mutable and contract can not be deleted
        };

        // dbg!(&estimated_costs_only_with_layer_info);

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

        let apply_type = if estimated_costs_only_with_layer_info.is_none() {
            BatchInsertTreeApplyType::StatefulBatchInsert
        } else {
            BatchInsertTreeApplyType::StatelessBatchInsert {
                in_tree_using_sums: false,
                is_sum_tree: false,
                flags_len: storage_flags
                    .map(|s| s.serialized_size())
                    .unwrap_or_default(),
            }
        };

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
                .document_info
                .get_raw_for_document_type(
                    name,
                    document_type,
                    document_and_contract_info.owner_id,
                    Some((sub_level, event_id)),
                )?
                .unwrap_or_default();

            // The zero will not matter here, because the PathKeyInfo is variable
            let path_key_info = document_top_field.clone().add_path::<0>(index_path.clone());
            // here we are inserting an empty tree that will have a subtree of all other index properties
            self.batch_insert_empty_tree_if_not_exists(
                path_key_info.clone(),
                storage_flags,
                apply_type,
                transaction,
                batch_operations,
            )?;

            if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info
            {
                let document_top_field_estimated_size = document_and_contract_info
                    .document_info
                    .get_estimated_size_for_document_type(name, document_type)?;

                if document_top_field_estimated_size > u8::MAX as u16 {
                    return Err(Error::Fee(FeeError::Overflow(
                        "document field is too big for being an index on delete",
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

            let mut index_path_info = if document_and_contract_info.document_info.is_document_size()
            {
                // This is a stateless operation
                PathInfo::PathWithSizes(KeyInfoPath::from_known_owned_path(index_path))
            } else {
                PathInfo::PathIterator::<0>(index_path)
            };

            // we push the actual value of the index path
            index_path_info.push(document_top_field)?;
            // the index path is now something like Contracts/ContractID/Documents(1)/$ownerId/<ownerId>

            self.add_indices_for_index_level_for_contract_operations(
                document_and_contract_info,
                index_path_info,
                sub_level,
                any_fields_null,
                &storage_flags,
                estimated_costs_only_with_layer_info,
                event_id,
                transaction,
                batch_operations,
            )?;
        }
        Ok(())
    }

    /// Gathers the operations to add a document to a contract.
    pub(crate) fn add_document_for_contract_operations(
        &self,
        document_and_contract_info: DocumentAndContractInfo,
        override_document: bool,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
    ) -> Result<Vec<DriveOperation>, Error> {
        let mut batch_operations: Vec<DriveOperation> = vec![];

        let primary_key_path = contract_documents_primary_key_path(
            document_and_contract_info.contract.id.as_bytes(),
            document_and_contract_info.document_type.name.as_str(),
        );

        // Apply means stateful query
        let query_type = if estimated_costs_only_with_layer_info.is_none() {
            StatefulDirectQuery
        } else {
            StatelessDirectQuery {
                in_tree_using_sums: false,
                query_target: QueryTargetValue(
                    document_and_contract_info.document_type.estimated_size() as u32,
                ),
            }
        };

        if override_document
            && !document_and_contract_info.document_info.is_document_size()
            && self.grove_has_raw(
                primary_key_path,
                document_and_contract_info
                    .document_info
                    .id_key_value_info()
                    .as_key_ref_request()?,
                query_type,
                transaction,
                &mut batch_operations,
            )?
        {
            let update_operations = self.update_document_for_contract_operations(
                document_and_contract_info,
                block_info,
                estimated_costs_only_with_layer_info,
                transaction,
            )?;
            batch_operations.extend(update_operations);
            return Ok(batch_operations);
        } else {
            // if we are trying to get estimated costs we need to add the upper levels
            if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info
            {
                Self::add_estimation_costs_for_levels_up_to_contract_document_type_excluded(
                    document_and_contract_info.contract,
                    estimated_costs_only_with_layer_info,
                );
            }
            // if we have override_document set that means we already checked if it exists
            self.add_document_to_primary_storage(
                &document_and_contract_info,
                block_info,
                override_document,
                estimated_costs_only_with_layer_info,
                transaction,
                &mut batch_operations,
            )?;
        }

        self.add_indices_for_top_index_level_for_contract_operations(
            &document_and_contract_info,
            estimated_costs_only_with_layer_info,
            transaction,
            &mut batch_operations,
        )?;
        Ok(batch_operations)
    }
}

#[cfg(test)]
mod tests {
    use std::option::Option::None;

    use super::*;
    use rand::Rng;
    use tempfile::TempDir;

    use crate::common::{json_document_to_cbor, setup_contract};
    use crate::contract::document::Document;
    use crate::drive::document::tests::setup_dashpay;
    use crate::drive::flags::StorageFlags;
    use crate::drive::object_size_info::DocumentAndContractInfo;
    use crate::drive::object_size_info::DocumentInfo::DocumentRefAndSerialization;
    use crate::drive::Drive;
    use crate::fee::default_costs::STORAGE_DISK_USAGE_CREDIT_PER_BYTE;
    use crate::fee::op::DriveOperation;

    #[test]
    fn test_add_dashpay_documents_no_transaction() {
        let (drive, dashpay_cbor) = setup_dashpay("add", true);

        let dashpay_cr_serialized_document = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/contact-request0.json",
            Some(1),
        );

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();
        drive
            .add_serialized_document_for_serialized_contract(
                &dashpay_cr_serialized_document,
                &dashpay_cbor,
                "contactRequest",
                Some(random_owner_id),
                false,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_ref(),
                None,
            )
            .expect("expected to insert a document successfully");

        drive
            .add_serialized_document_for_serialized_contract(
                &dashpay_cr_serialized_document,
                &dashpay_cbor,
                "contactRequest",
                Some(random_owner_id),
                false,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_ref(),
                None,
            )
            .expect_err("expected not to be able to insert same document twice");

        drive
            .add_serialized_document_for_serialized_contract(
                &dashpay_cr_serialized_document,
                &dashpay_cbor,
                "contactRequest",
                Some(random_owner_id),
                true,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_ref(),
                None,
            )
            .expect("expected to override a document successfully");
    }

    #[test]
    fn test_add_dashpay_documents() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        let db_transaction = drive.grove.start_transaction();

        drive
            .create_initial_state_structure(Some(&db_transaction))
            .expect("expected to create root tree successfully");

        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
            None,
            Some(&db_transaction),
        );

        let dashpay_cr_serialized_document = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/contact-request0.json",
            Some(1),
        );

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();
        drive
            .add_serialized_document_for_contract(
                &dashpay_cr_serialized_document,
                &contract,
                "contactRequest",
                Some(random_owner_id),
                false,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_ref(),
                Some(&db_transaction),
            )
            .expect("expected to insert a document successfully");

        drive
            .add_serialized_document_for_contract(
                &dashpay_cr_serialized_document,
                &contract,
                "contactRequest",
                Some(random_owner_id),
                false,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_ref(),
                Some(&db_transaction),
            )
            .expect_err("expected not to be able to insert same document twice");

        drive
            .add_serialized_document_for_contract(
                &dashpay_cr_serialized_document,
                &contract,
                "contactRequest",
                Some(random_owner_id),
                true,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_ref(),
                Some(&db_transaction),
            )
            .expect("expected to override a document successfully");
    }

    #[test]
    fn test_add_dashpay_contact_request_with_fee() {
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

        let dashpay_cr_serialized_document = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/contact-request0.json",
            Some(1),
        );

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();

        let FeeResult {
            storage_fee,
            processing_fee,
            removed_bytes_from_epochs_by_identities: _,
            removed_bytes_from_system: _,
        } = drive
            .add_serialized_document_for_contract(
                &dashpay_cr_serialized_document,
                &contract,
                "contactRequest",
                Some(random_owner_id),
                false,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_ref(),
                Some(&db_transaction),
            )
            .expect("expected to insert a document successfully");

        let added_bytes = storage_fee / STORAGE_DISK_USAGE_CREDIT_PER_BYTE;
        assert_eq!((3247, 2914000), (added_bytes, processing_fee));
    }

    #[test]
    fn test_add_dashpay_profile_with_fee() {
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

        let dashpay_cr_serialized_document = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/profile0.json",
            Some(1),
        );

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();

        let FeeResult {
            storage_fee,
            processing_fee,
            removed_bytes_from_epochs_by_identities: _,
            removed_bytes_from_system: _,
        } = drive
            .add_serialized_document_for_contract(
                &dashpay_cr_serialized_document,
                &contract,
                "profile",
                Some(random_owner_id),
                false,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_ref(),
                Some(&db_transaction),
            )
            .expect("expected to insert a document successfully");

        let added_bytes = storage_fee / STORAGE_DISK_USAGE_CREDIT_PER_BYTE;
        assert_eq!((1428, 1895000), (added_bytes, processing_fee));
    }

    #[test]
    fn test_add_dashpay_profile_average_case_cost_fee() {
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

        let dashpay_cr_serialized_document = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/profile0.json",
            Some(1),
        );

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();

        let FeeResult {
            storage_fee,
            processing_fee,
            removed_bytes_from_epochs_by_identities: _,
            removed_bytes_from_system: _,
        } = drive
            .add_serialized_document_for_contract(
                &dashpay_cr_serialized_document,
                &contract,
                "profile",
                Some(random_owner_id),
                false,
                BlockInfo::default(),
                false,
                StorageFlags::optional_default_as_ref(),
                Some(&db_transaction),
            )
            .expect("expected to insert a document successfully");

        let added_bytes = storage_fee / STORAGE_DISK_USAGE_CREDIT_PER_BYTE;
        assert_eq!(1428, added_bytes);
        assert_eq!(145603600, processing_fee);
    }

    #[ignore]
    #[test]
    fn test_unknown_state_cost_dashpay_fee_for_add_documents() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        let db_transaction = drive.grove.start_transaction();

        drive
            .create_initial_state_structure(Some(&db_transaction))
            .expect("expected to create root tree successfully");

        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
            None,
            Some(&db_transaction),
        );

        let dashpay_cr_serialized_document = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/contact-request0.json",
            Some(1),
        );

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();
        let fees = drive
            .add_serialized_document_for_contract(
                &dashpay_cr_serialized_document,
                &contract,
                "contactRequest",
                Some(random_owner_id),
                false,
                BlockInfo::default(),
                false,
                StorageFlags::optional_default_as_ref(),
                Some(&db_transaction),
            )
            .expect("expected to get back fee for document insertion successfully");

        let actual_fees = drive
            .add_serialized_document_for_contract(
                &dashpay_cr_serialized_document,
                &contract,
                "contactRequest",
                Some(random_owner_id),
                false,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_ref(),
                Some(&db_transaction),
            )
            .expect("expected to insert a document successfully");

        assert_eq!(fees, actual_fees);
    }

    #[test]
    fn test_add_dashpay_fee_for_documents_detail() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        let db_transaction = drive.grove.start_transaction();

        drive
            .create_initial_state_structure(Some(&db_transaction))
            .expect("expected to create root tree successfully");

        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
            None,
            Some(&db_transaction),
        );

        let dashpay_cr_serialized_document = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/contact-request0.json",
            Some(1),
        );

        let owner_id = rand::thread_rng().gen::<[u8; 32]>();
        let document = Document::from_cbor(&dashpay_cr_serialized_document, None, Some(owner_id))
            .expect("expected to deserialize document successfully");

        let storage_flags = Some(StorageFlags::SingleEpoch(0));

        let document_info = DocumentRefAndSerialization((
            &document,
            &dashpay_cr_serialized_document,
            storage_flags.as_ref(),
        ));

        let document_type = contract
            .document_type_for_name("contactRequest")
            .expect("expected to get document type successfully");
        let mut fee_drive_operations: Vec<DriveOperation> = vec![];
        let mut actual_drive_operations: Vec<DriveOperation> = vec![];

        let root_hash = drive
            .grove
            .root_hash(Some(&db_transaction))
            .unwrap()
            .expect("expected a root hash calculation to succeed");

        drive
            .add_document_for_contract_apply_and_add_to_operations(
                DocumentAndContractInfo {
                    document_info: document_info.clone(),
                    contract: &contract,
                    document_type,
                    owner_id: Some(owner_id),
                },
                false,
                &BlockInfo::default(),
                false,
                Some(&db_transaction),
                &mut fee_drive_operations,
            )
            .expect("expected to get back fee for document insertion successfully");

        let root_hash_after_fee = drive
            .grove
            .root_hash(Some(&db_transaction))
            .unwrap()
            .expect("expected a root hash calculation to succeed");

        assert_eq!(root_hash, root_hash_after_fee);

        drive
            .add_document_for_contract_apply_and_add_to_operations(
                DocumentAndContractInfo {
                    document_info,
                    contract: &contract,
                    document_type,
                    owner_id: Some(owner_id),
                },
                false,
                &BlockInfo::default(),
                true,
                Some(&db_transaction),
                &mut actual_drive_operations,
            )
            .expect("expected to get back fee for document insertion successfully");

        assert_eq!(actual_drive_operations.len(), fee_drive_operations.len());
    }

    #[test]
    fn test_add_dpns_document_with_fee() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        let db_transaction = drive.grove.start_transaction();

        drive
            .create_initial_state_structure(Some(&db_transaction))
            .expect("expected to create root tree successfully");

        let contract = setup_contract(
            &drive,
            "tests/supporting_files/contract/dpns/dpns-contract.json",
            None,
            Some(&db_transaction),
        );

        let dpns_domain_serialized_document =
            json_document_to_cbor("tests/supporting_files/contract/dpns/domain0.json", Some(1));

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();

        let document = Document::from_cbor(
            &dpns_domain_serialized_document,
            None,
            Some(random_owner_id),
        )
        .expect("expected to deserialize the document");

        let document_type = contract
            .document_type_for_name("domain")
            .expect("expected to get a document type");

        let storage_flags = Some(StorageFlags::SingleEpoch(0));

        let FeeResult {
            storage_fee,
            processing_fee,
            removed_bytes_from_epochs_by_identities: _,
            removed_bytes_from_system: _,
        } = drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    document_info: DocumentRefAndSerialization((
                        &document,
                        &dpns_domain_serialized_document,
                        storage_flags.as_ref(),
                    )),
                    contract: &contract,
                    document_type,
                    owner_id: None,
                },
                false,
                BlockInfo::default(),
                true,
                Some(&db_transaction),
            )
            .expect("expected to insert a document successfully");

        let added_bytes = storage_fee / STORAGE_DISK_USAGE_CREDIT_PER_BYTE;
        assert_eq!((1986, 2604600), (added_bytes, processing_fee));

        drive
            .grove
            .commit_transaction(db_transaction)
            .unwrap()
            .expect("unable to commit transaction");
    }

    #[test]
    fn test_add_dashpay_many_non_conflicting_documents() {
        let (drive, dashpay_cbor) = setup_dashpay("add_no_conflict", true);

        let dashpay_cr_serialized_document_0 = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/contact-request0.json",
            Some(1),
        );

        let dashpay_cr_serialized_document_1 = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/contact-request1.json",
            Some(1),
        );

        let dashpay_cr_serialized_document_2 = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/contact-request2.json",
            Some(1),
        );

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();
        drive
            .add_serialized_document_for_serialized_contract(
                &dashpay_cr_serialized_document_0,
                &dashpay_cbor,
                "contactRequest",
                Some(random_owner_id),
                false,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_ref(),
                None,
            )
            .expect("expected to insert a document successfully");
        drive
            .add_serialized_document_for_serialized_contract(
                &dashpay_cr_serialized_document_1,
                &dashpay_cbor,
                "contactRequest",
                Some(random_owner_id),
                false,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_ref(),
                None,
            )
            .expect("expected to insert a document successfully");
        drive
            .add_serialized_document_for_serialized_contract(
                &dashpay_cr_serialized_document_2,
                &dashpay_cbor,
                "contactRequest",
                Some(random_owner_id),
                false,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_ref(),
                None,
            )
            .expect("expected to insert a document successfully");
    }

    #[test]
    fn test_add_dashpay_conflicting_unique_index_documents() {
        let (drive, dashpay_cbor) = setup_dashpay("add_conflict", true);

        let dashpay_cr_serialized_document_0 = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/contact-request0.json",
            Some(1),
        );

        let dashpay_cr_serialized_document_0_dup = json_document_to_cbor(
            "tests/supporting_files/contract/dashpay/contact-request0-dup-unique-index.json",
            Some(1),
        );

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();
        drive
            .add_serialized_document_for_serialized_contract(
                &dashpay_cr_serialized_document_0,
                &dashpay_cbor,
                "contactRequest",
                Some(random_owner_id),
                false,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_ref(),
                None,
            )
            .expect("expected to insert a document successfully");
        drive
            .add_serialized_document_for_serialized_contract(
                &dashpay_cr_serialized_document_0_dup,
                &dashpay_cbor,
                "contactRequest",
                Some(random_owner_id),
                false,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_ref(),
                None,
            )
            .expect_err(
                "expected not to be able to insert document with already existing unique index",
            );
    }

    #[test]
    fn test_create_two_documents_with_the_same_index_in_different_transactions() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        let db_transaction = drive.grove.start_transaction();

        drive
            .create_initial_state_structure(Some(&db_transaction))
            .expect("expected to create root tree successfully");

        let contract_cbor = hex::decode("01000000a5632469645820e668c659af66aee1e72c186dde7b5b7e0a1d712a09c40d5721f622bf53c531556724736368656d61783468747470733a2f2f736368656d612e646173682e6f72672f6470702d302d342d302f6d6574612f646174612d636f6e7472616374676f776e6572496458203012c19b98ec0033addb36cd64b7f510670f2a351a4304b5f6994144286efdac6776657273696f6e0169646f63756d656e7473a266646f6d61696ea66474797065666f626a65637467696e646963657383a3646e616d6572706172656e744e616d65416e644c6162656c66756e69717565f56a70726f7065727469657382a1781a6e6f726d616c697a6564506172656e74446f6d61696e4e616d6563617363a16f6e6f726d616c697a65644c6162656c63617363a3646e616d656e646173684964656e74697479496466756e69717565f56a70726f7065727469657381a1781c7265636f7264732e64617368556e697175654964656e74697479496463617363a2646e616d656964617368416c6961736a70726f7065727469657381a1781b7265636f7264732e64617368416c6961734964656e746974794964636173636824636f6d6d656e74790137496e206f7264657220746f207265676973746572206120646f6d61696e20796f75206e65656420746f206372656174652061207072656f726465722e20546865207072656f726465722073746570206973206e656564656420746f2070726576656e74206d616e2d696e2d7468652d6d6964646c652061747461636b732e206e6f726d616c697a65644c6162656c202b20272e27202b206e6f726d616c697a6564506172656e74446f6d61696e206d757374206e6f74206265206c6f6e676572207468616e20323533206368617273206c656e67746820617320646566696e65642062792052464320313033352e20446f6d61696e20646f63756d656e74732061726520696d6d757461626c653a206d6f64696669636174696f6e20616e642064656c6574696f6e20617265207265737472696374656468726571756972656486656c6162656c6f6e6f726d616c697a65644c6162656c781a6e6f726d616c697a6564506172656e74446f6d61696e4e616d656c7072656f7264657253616c74677265636f7264736e737562646f6d61696e52756c65736a70726f70657274696573a6656c6162656ca5647479706566737472696e67677061747465726e782a5e5b612d7a412d5a302d395d5b612d7a412d5a302d392d5d7b302c36317d5b612d7a412d5a302d395d24696d61784c656e677468183f696d696e4c656e677468036b6465736372697074696f6e7819446f6d61696e206c6162656c2e20652e672e2027426f62272e677265636f726473a66474797065666f626a6563746824636f6d6d656e747890436f6e73747261696e742077697468206d617820616e64206d696e2070726f7065727469657320656e737572652074686174206f6e6c79206f6e65206964656e74697479207265636f72642069732075736564202d206569746865722061206064617368556e697175654964656e74697479496460206f722061206064617368416c6961734964656e746974794964606a70726f70657274696573a27364617368416c6961734964656e746974794964a764747970656561727261796824636f6d6d656e7478234d75737420626520657175616c20746f2074686520646f63756d656e74206f776e6572686d61784974656d731820686d696e4974656d73182069627974654172726179f56b6465736372697074696f6e783d4964656e7469747920494420746f206265207573656420746f2063726561746520616c696173206e616d657320666f7220746865204964656e7469747970636f6e74656e744d656469615479706578216170706c69636174696f6e2f782e646173682e6470702e6964656e7469666965727464617368556e697175654964656e746974794964a764747970656561727261796824636f6d6d656e7478234d75737420626520657175616c20746f2074686520646f63756d656e74206f776e6572686d61784974656d731820686d696e4974656d73182069627974654172726179f56b6465736372697074696f6e783e4964656e7469747920494420746f206265207573656420746f2063726561746520746865207072696d617279206e616d6520746865204964656e7469747970636f6e74656e744d656469615479706578216170706c69636174696f6e2f782e646173682e6470702e6964656e7469666965726d6d617850726f70657274696573016d6d696e50726f7065727469657301746164646974696f6e616c50726f70657274696573f46c7072656f7264657253616c74a56474797065656172726179686d61784974656d731820686d696e4974656d73182069627974654172726179f56b6465736372697074696f6e782253616c74207573656420696e20746865207072656f7264657220646f63756d656e746e737562646f6d61696e52756c6573a56474797065666f626a656374687265717569726564816f616c6c6f77537562646f6d61696e736a70726f70657274696573a16f616c6c6f77537562646f6d61696e73a3647479706567626f6f6c65616e6824636f6d6d656e74784f4f6e6c792074686520646f6d61696e206f776e657220697320616c6c6f77656420746f2063726561746520737562646f6d61696e7320666f72206e6f6e20746f702d6c6576656c20646f6d61696e736b6465736372697074696f6e785b54686973206f7074696f6e20646566696e65732077686f2063616e2063726561746520737562646f6d61696e733a2074727565202d20616e796f6e653b2066616c7365202d206f6e6c792074686520646f6d61696e206f776e65726b6465736372697074696f6e7842537562646f6d61696e2072756c657320616c6c6f7720646f6d61696e206f776e65727320746f20646566696e652072756c657320666f7220737562646f6d61696e73746164646974696f6e616c50726f70657274696573f46f6e6f726d616c697a65644c6162656ca5647479706566737472696e67677061747465726e78215e5b612d7a302d395d5b612d7a302d392d5d7b302c36317d5b612d7a302d395d246824636f6d6d656e7478694d75737420626520657175616c20746f20746865206c6162656c20696e206c6f776572636173652e20546869732070726f70657274792077696c6c20626520646570726563617465642064756520746f206361736520696e73656e73697469766520696e6469636573696d61784c656e677468183f6b6465736372697074696f6e7850446f6d61696e206c6162656c20696e206c6f7765726361736520666f7220636173652d696e73656e73697469766520756e697175656e6573732076616c69646174696f6e2e20652e672e2027626f6227781a6e6f726d616c697a6564506172656e74446f6d61696e4e616d65a6647479706566737472696e67677061747465726e78285e247c5e5b5b612d7a302d395d5b612d7a302d392d5c2e5d7b302c3138387d5b612d7a302d395d246824636f6d6d656e74788c4d7573742065697468657220626520657175616c20746f20616e206578697374696e6720646f6d61696e206f7220656d70747920746f20637265617465206120746f70206c6576656c20646f6d61696e2e204f6e6c7920746865206461746120636f6e7472616374206f776e65722063616e2063726561746520746f70206c6576656c20646f6d61696e732e696d61784c656e67746818be696d696e4c656e677468006b6465736372697074696f6e785e412066756c6c20706172656e7420646f6d61696e206e616d6520696e206c6f7765726361736520666f7220636173652d696e73656e73697469766520756e697175656e6573732076616c69646174696f6e2e20652e672e20276461736827746164646974696f6e616c50726f70657274696573f4687072656f72646572a66474797065666f626a65637467696e646963657381a3646e616d656a73616c7465644861736866756e69717565f56a70726f7065727469657381a17073616c746564446f6d61696e48617368636173636824636f6d6d656e74784a5072656f7264657220646f63756d656e74732061726520696d6d757461626c653a206d6f64696669636174696f6e20616e642064656c6574696f6e206172652072657374726963746564687265717569726564817073616c746564446f6d61696e486173686a70726f70657274696573a17073616c746564446f6d61696e48617368a56474797065656172726179686d61784974656d731820686d696e4974656d73182069627974654172726179f56b6465736372697074696f6e7859446f75626c65207368612d323536206f662074686520636f6e636174656e6174696f6e206f66206120333220627974652072616e646f6d2073616c7420616e642061206e6f726d616c697a656420646f6d61696e206e616d65746164646974696f6e616c50726f70657274696573f4").unwrap();

        drive
            .apply_contract_cbor(
                contract_cbor.clone(),
                None,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_ref(),
                Some(&db_transaction),
            )
            .expect("expected to apply contract successfully");

        // Create dash TLD

        let dash_tld_cbor = hex::decode("01000000ac632469645820d7f2c53f46a917ab6e5b39a2d7bc260b649289453744d1e0d4f26a8d8eff37cf65247479706566646f6d61696e656c6162656c6464617368677265636f726473a17364617368416c6961734964656e74697479496458203012c19b98ec0033addb36cd64b7f510670f2a351a4304b5f6994144286efdac68246f776e6572496458203012c19b98ec0033addb36cd64b7f510670f2a351a4304b5f6994144286efdac69247265766973696f6e016a246372656174656441741b0000017f07c861586c7072656f7264657253616c745820e0b508c5a36825a206693a1f414aa13edbecf43c41e3c799ea9e737b4f9aa2266e737562646f6d61696e52756c6573a16f616c6c6f77537562646f6d61696e73f56f2464617461436f6e747261637449645820e668c659af66aee1e72c186dde7b5b7e0a1d712a09c40d5721f622bf53c531556f6e6f726d616c697a65644c6162656c6464617368781a6e6f726d616c697a6564506172656e74446f6d61696e4e616d6560").unwrap();

        drive
            .add_serialized_document_for_serialized_contract(
                dash_tld_cbor.as_slice(),
                contract_cbor.as_slice(),
                "domain",
                None,
                true,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_ref(),
                Some(&db_transaction),
            )
            .expect("should create dash tld");

        drive
            .grove
            .commit_transaction(db_transaction)
            .unwrap()
            .expect("should commit transaction");

        let db_transaction = drive.grove.start_transaction();

        // add random TLD

        let random_tld_cbor = hex::decode("01000000ab632469645820655c9b5606f4ad53daea90de9c540aad656ed5fbe5fb14b40700f6f56dc793ac65247479706566646f6d61696e656c6162656c746433653966343532373963343865306261363561677265636f726473a17364617368416c6961734964656e74697479496458203012c19b98ec0033addb36cd64b7f510670f2a351a4304b5f6994144286efdac68246f776e6572496458203012c19b98ec0033addb36cd64b7f510670f2a351a4304b5f6994144286efdac69247265766973696f6e016c7072656f7264657253616c745820219353a923a29cd02c521b141f326ac0d12c362a84f1979a5de89b8dba12891b6e737562646f6d61696e52756c6573a16f616c6c6f77537562646f6d61696e73f56f2464617461436f6e747261637449645820e668c659af66aee1e72c186dde7b5b7e0a1d712a09c40d5721f622bf53c531556f6e6f726d616c697a65644c6162656c746433653966343532373963343865306261363561781a6e6f726d616c697a6564506172656e74446f6d61696e4e616d6560").unwrap();

        drive
            .add_serialized_document_for_serialized_contract(
                random_tld_cbor.as_slice(),
                contract_cbor.as_slice(),
                "domain",
                None,
                true,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_ref(),
                Some(&db_transaction),
            )
            .expect("should add random tld");
    }
}
