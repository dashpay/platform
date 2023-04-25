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

//! Drive Contracts.
//!
//! This module defines functions pertinent to Contracts stored in Drive.
//!

#[cfg(feature = "full")]
mod estimation_costs;
/// Various paths for contract operations
#[cfg(feature = "full")]
pub(crate) mod paths;
#[cfg(feature = "full")]
pub(crate) mod prove;
#[cfg(feature = "full")]
pub(crate) mod queries;

#[cfg(feature = "full")]
use std::borrow::Cow;
#[cfg(feature = "full")]
use std::collections::{HashMap, HashSet};
#[cfg(feature = "full")]
use std::sync::Arc;

#[cfg(feature = "full")]
use crate::common::encode::encode_u64;
#[cfg(any(feature = "full", feature = "verify"))]
use costs::OperationCost;
#[cfg(feature = "full")]
use costs::{cost_return_on_error_no_add, CostContext, CostResult, CostsExt};

#[cfg(feature = "full")]
use grovedb::batch::key_info::KeyInfo;
#[cfg(feature = "full")]
use grovedb::batch::KeyInfoPath;
#[cfg(feature = "full")]
use grovedb::reference_path::ReferencePathType::SiblingReference;

use dpp::platform_value::{platform_value, Identifier, Value};
use dpp::Convertible;
#[cfg(feature = "full")]
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};

#[cfg(any(feature = "full", feature = "verify"))]
use crate::contract::Contract;
#[cfg(feature = "full")]
use crate::drive::batch::GroveDbOpBatch;
#[cfg(feature = "full")]
use crate::drive::defaults::CONTRACT_MAX_SERIALIZED_SIZE;
#[cfg(feature = "full")]
use dpp::block::block_info::BlockInfo;

#[cfg(any(feature = "full", feature = "verify"))]
use crate::drive::flags::StorageFlags;
#[cfg(feature = "full")]
use crate::drive::object_size_info::DriveKeyInfo::{Key, KeyRef};

#[cfg(feature = "full")]
use crate::drive::contract::paths::contract_root_path;
#[cfg(feature = "full")]
use crate::drive::grove_operations::QueryTarget::QueryTargetValue;
#[cfg(feature = "full")]
use crate::drive::grove_operations::{BatchInsertTreeApplyType, DirectQueryType};
#[cfg(feature = "full")]
use crate::drive::object_size_info::PathKeyElementInfo::{
    PathFixedSizeKeyRefElement, PathKeyElementSize,
};
#[cfg(feature = "full")]
use crate::drive::object_size_info::PathKeyInfo::PathFixedSizeKeyRef;
#[cfg(feature = "full")]
use crate::drive::{contract_documents_path, Drive, RootTree};
#[cfg(feature = "full")]
use crate::error::drive::DriveError;
#[cfg(feature = "full")]
use crate::error::Error;
#[cfg(feature = "full")]
use crate::fee::calculate_fee;
#[cfg(feature = "full")]
use crate::fee::op::LowLevelDriveOperation;
#[cfg(feature = "full")]
use crate::fee::op::LowLevelDriveOperation::{CalculatedCostOperation, PreCalculatedFeeResult};
#[cfg(any(feature = "full", feature = "verify"))]
use crate::fee::result::FeeResult;
use crate::query::QueryResultEncoding;
#[cfg(feature = "full")]
use dpp::block::epoch::Epoch;
use dpp::prelude::DataContract;
use dpp::serialization_traits::{PlatformDeserializable, PlatformSerializable};

#[cfg(feature = "full")]
/// Adds operations to the op batch relevant to initializing the contract's structure.
/// Namely it inserts an empty tree at the contract's root path.
pub fn add_init_contracts_structure_operations(batch: &mut GroveDbOpBatch) {
    batch.add_insert_empty_tree(vec![], vec![RootTree::ContractDocuments as u8]);
}

#[cfg(any(feature = "full", feature = "verify"))]
/// Contract and fetch information
#[derive(Default, PartialEq, Debug, Clone)]
pub struct ContractFetchInfo {
    /// The contract
    pub contract: Contract,
    /// The contract's potential storage flags
    pub storage_flags: Option<StorageFlags>,
    /// These are the operations that are used to fetch a contract
    /// This is only used on epoch change
    pub(crate) cost: OperationCost,
    /// The fee is updated every epoch based on operation costs
    /// Except if protocol version has changed in which case all the cache is cleared
    pub fee: Option<FeeResult>,
}

#[cfg(feature = "full")]
impl Drive {
    /// Adds a contract to storage.
    fn add_contract_to_storage(
        &self,
        contract_element: Element,
        contract: &Contract,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        insert_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<(), Error> {
        let contract_root_path = paths::contract_root_path(contract.id.as_bytes());
        if contract.config.keeps_history {
            let element_flags = contract_element.get_flags().clone();
            let storage_flags =
                StorageFlags::map_cow_some_element_flags_ref(contract_element.get_flags())?;

            if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info
            {
                Self::add_estimation_costs_for_levels_up_to_contract_document_type_excluded(
                    contract,
                    estimated_costs_only_with_layer_info,
                );
            }

            self.batch_insert_empty_tree(
                contract_root_path,
                KeyRef(&[0]),
                storage_flags.as_ref().map(|flags| flags.as_ref()),
                insert_operations,
            )?;
            let encoded_time = encode_u64(block_info.time_ms)?;
            let contract_keeping_history_storage_path =
                paths::contract_keeping_history_storage_path(contract.id.as_bytes());
            self.batch_insert(
                PathFixedSizeKeyRefElement((
                    contract_keeping_history_storage_path,
                    encoded_time.as_slice(),
                    contract_element,
                )),
                insert_operations,
            )?;

            let reference_element =
                Element::Reference(SiblingReference(encoded_time), Some(1), element_flags);

            let path_key_element_info = if estimated_costs_only_with_layer_info.is_none() {
                PathFixedSizeKeyRefElement((
                    contract_keeping_history_storage_path,
                    &[0],
                    reference_element,
                ))
            } else {
                PathKeyElementSize((
                    KeyInfoPath::from_known_path(contract_keeping_history_storage_path),
                    KeyInfo::KnownKey(vec![0u8]),
                    reference_element,
                ))
            };
            self.batch_insert(path_key_element_info, insert_operations)?;
        } else {
            // the contract is just stored at key 0
            let path_key_element_info = if estimated_costs_only_with_layer_info.is_none() {
                PathFixedSizeKeyRefElement((contract_root_path, &[0], contract_element))
            } else {
                PathKeyElementSize((
                    KeyInfoPath::from_known_path(contract_root_path),
                    KeyInfo::KnownKey(vec![0u8]),
                    contract_element,
                ))
            };
            self.batch_insert(path_key_element_info, insert_operations)?;
        }
        Ok(())
    }

    /// Insert a contract.
    pub fn insert_contract(
        &self,
        contract: &DataContract,
        block_info: BlockInfo,
        apply: bool,
        transaction: TransactionArg,
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

        self.insert_contract_element(
            contract_element,
            contract,
            &block_info,
            apply,
            transaction,
            &mut drive_operations,
        )?;

        calculate_fee(None, Some(drive_operations), &block_info.epoch)
    }

    /// Adds a contract to storage using `add_contract_to_storage`
    /// and inserts the empty trees which will be necessary to later insert documents.
    pub fn insert_contract_element(
        &self,
        contract_element: Element,
        contract: &Contract,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<(), Error> {
        let mut estimated_costs_only_with_layer_info = if apply {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };
        let batch_operations = self.insert_contract_operations(
            contract_element,
            contract,
            block_info,
            &mut estimated_costs_only_with_layer_info,
        )?;
        self.apply_batch_low_level_drive_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            batch_operations,
            drive_operations,
        )
    }

    /// The operations for adding a contract.
    /// These operations add a contract to storage using `add_contract_to_storage`
    /// and insert the empty trees which will be necessary to later insert documents.
    fn insert_contract_add_operations(
        &self,
        contract_element: Element,
        contract: &Contract,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<(), Error> {
        let batch_operations = self.insert_contract_operations(
            contract_element,
            contract,
            block_info,
            estimated_costs_only_with_layer_info,
        )?;
        drive_operations.extend(batch_operations);
        Ok(())
    }

    /// The operations for adding a contract.
    /// These operations add a contract to storage using `add_contract_to_storage`
    /// and insert the empty trees which will be necessary to later insert documents.
    fn insert_contract_operations(
        &self,
        contract_element: Element,
        contract: &Contract,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut batch_operations: Vec<LowLevelDriveOperation> = vec![];

        let storage_flags = StorageFlags::map_some_element_flags_ref(contract_element.get_flags())?;

        self.batch_insert_empty_tree(
            [Into::<&[u8; 1]>::into(RootTree::ContractDocuments).as_slice()],
            KeyRef(contract.id.as_bytes()),
            storage_flags.as_ref(),
            &mut batch_operations,
        )?;

        self.add_contract_to_storage(
            contract_element,
            contract,
            block_info,
            estimated_costs_only_with_layer_info,
            &mut batch_operations,
        )?;

        // the documents
        let contract_root_path = paths::contract_root_path(contract.id.as_bytes());
        let key_info = Key(vec![1]);
        self.batch_insert_empty_tree(
            contract_root_path,
            key_info,
            storage_flags.as_ref(),
            &mut batch_operations,
        )?;

        // next we should store each document type
        // right now we are referring them by name
        // toDo: change this to be a reference by index
        let contract_documents_path = contract_documents_path(contract.id.as_bytes());

        for (type_key, document_type) in contract.document_types.iter() {
            self.batch_insert_empty_tree(
                contract_documents_path,
                KeyRef(type_key.as_bytes()),
                storage_flags.as_ref(),
                &mut batch_operations,
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
                    )?;
                    index_cache.insert(index_bytes);
                }
            }
        }

        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_contract_insertion(
                contract,
                estimated_costs_only_with_layer_info,
            );
        }

        Ok(batch_operations)
    }

    /// Update a data contract
    pub fn update_contract(
        &self,
        contract: &DataContract,
        block_info: BlockInfo,
        apply: bool,
        transaction: TransactionArg,
    ) -> Result<FeeResult, Error> {
        if !apply {
            return self.insert_contract(contract, block_info, false, transaction);
        }
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];

        let contract_bytes = contract.serialize()?;

        // Since we can update the contract by definition it already has storage flags
        let storage_flags = Some(StorageFlags::new_single_epoch(
            block_info.epoch.index,
            Some(contract.owner_id.to_buffer()),
        ));

        let contract_element = Element::Item(
            contract_bytes,
            StorageFlags::map_to_some_element_flags(storage_flags.as_ref()),
        );

        let original_contract_fetch_info = self
            .get_contract_with_fetch_info_and_add_to_operations(
                contract.id.to_buffer(),
                Some(&block_info.epoch),
                true,
                transaction,
                &mut drive_operations,
            )?
            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                "contract should exist",
            )))?;

        if original_contract_fetch_info.contract.config.readonly {
            return Err(Error::Drive(DriveError::UpdatingReadOnlyImmutableContract(
                "original contract is readonly",
            )));
        }

        self.update_contract_element(
            contract_element,
            &contract,
            &original_contract_fetch_info.contract,
            &block_info,
            transaction,
            &mut drive_operations,
        )?;

        // Update Data Contracts cache with the new contract
        let updated_contract_fetch_info = self
            .fetch_contract_and_add_operations(
                contract.id.to_buffer(),
                Some(&block_info.epoch),
                transaction,
                &mut drive_operations,
            )?
            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                "contract should exist",
            )))?;

        let mut drive_cache = self.cache.write().unwrap();

        drive_cache
            .cached_contracts
            .insert(updated_contract_fetch_info, transaction.is_some());

        calculate_fee(None, Some(drive_operations), &block_info.epoch)
    }

    /// Updates a contract.
    pub fn update_contract_element(
        &self,
        contract_element: Element,
        contract: &Contract,
        original_contract: &Contract,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<(), Error> {
        let mut estimated_costs_only_with_layer_info =
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>;
        let batch_operations = self.update_contract_operations(
            contract_element,
            contract,
            original_contract,
            block_info,
            &mut estimated_costs_only_with_layer_info,
            transaction,
        )?;
        self.apply_batch_low_level_drive_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            batch_operations,
            drive_operations,
        )
    }

    /// Updates a contract.
    fn update_contract_add_operations(
        &self,
        contract_element: Element,
        contract: &Contract,
        original_contract: &Contract,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<(), Error> {
        let batch_operations = self.update_contract_operations(
            contract_element,
            contract,
            original_contract,
            block_info,
            estimated_costs_only_with_layer_info,
            transaction,
        )?;
        drive_operations.extend(batch_operations);
        Ok(())
    }

    /// operations for updating a contract.
    fn update_contract_operations(
        &self,
        contract_element: Element,
        contract: &Contract,
        original_contract: &Contract,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut batch_operations: Vec<LowLevelDriveOperation> = vec![];
        if original_contract.config.readonly {
            return Err(Error::Drive(DriveError::UpdatingReadOnlyImmutableContract(
                "contract is readonly",
            )));
        }

        if contract.config.readonly {
            return Err(Error::Drive(DriveError::ChangingContractToReadOnly(
                "contract can not be changed to readonly",
            )));
        }

        if contract.config.keeps_history ^ original_contract.config.keeps_history {
            return Err(Error::Drive(DriveError::ChangingContractKeepsHistory(
                "contract can not change whether it keeps history",
            )));
        }

        if contract.config.documents_keep_history_contract_default
            ^ original_contract
                .config
                .documents_keep_history_contract_default
        {
            return Err(Error::Drive(
                DriveError::ChangingContractDocumentsKeepsHistoryDefault(
                    "contract can not change the default of whether documents keeps history",
                ),
            ));
        }

        if contract.config.documents_mutable_contract_default
            ^ original_contract.config.documents_mutable_contract_default
        {
            return Err(Error::Drive(
                DriveError::ChangingContractDocumentsMutabilityDefault(
                    "contract can not change the default of whether documents are mutable",
                ),
            ));
        }

        let element_flags = contract_element.get_flags().clone();

        // this will override the previous contract if we do not keep history
        self.add_contract_to_storage(
            contract_element,
            contract,
            block_info,
            estimated_costs_only_with_layer_info,
            &mut batch_operations,
        )?;

        let storage_flags = StorageFlags::map_cow_some_element_flags_ref(&element_flags)?;

        let contract_documents_path = contract_documents_path(contract.id.as_bytes());
        for (type_key, document_type) in contract.document_types.iter() {
            let original_document_type = &original_contract.document_types.get(type_key);
            if let Some(original_document_type) = original_document_type {
                if original_document_type.documents_mutable ^ document_type.documents_mutable {
                    return Err(Error::Drive(DriveError::ChangingDocumentTypeMutability(
                        "contract can not change whether a specific document type is mutable",
                    )));
                }
                if original_document_type.documents_keep_history
                    ^ document_type.documents_keep_history
                {
                    return Err(Error::Drive(DriveError::ChangingDocumentTypeKeepsHistory(
                        "contract can not change whether a specific document type keeps history",
                    )));
                }

                let type_path = [
                    contract_documents_path[0],
                    contract_documents_path[1],
                    contract_documents_path[2],
                    type_key.as_bytes(),
                ];

                let apply_type = if estimated_costs_only_with_layer_info.is_none() {
                    BatchInsertTreeApplyType::StatefulBatchInsertTree
                } else {
                    BatchInsertTreeApplyType::StatelessBatchInsertTree {
                        in_tree_using_sums: false,
                        is_sum_tree: false,
                        flags_len: element_flags
                            .as_ref()
                            .map(|e| e.len() as u32)
                            .unwrap_or_default(),
                    }
                };

                let mut index_cache: HashSet<&[u8]> = HashSet::new();
                // for each type we should insert the indices that are top level
                for index in document_type.top_level_indices() {
                    // toDo: we can save a little by only inserting on new indexes
                    let index_bytes = index.name.as_bytes();
                    if !index_cache.contains(index_bytes) {
                        self.batch_insert_empty_tree_if_not_exists(
                            PathFixedSizeKeyRef((type_path, index.name.as_bytes())),
                            storage_flags.as_ref().map(|flags| flags.as_ref()),
                            apply_type,
                            transaction,
                            &mut None,
                            &mut batch_operations,
                        )?;
                        index_cache.insert(index_bytes);
                    }
                }
            } else {
                // We can just insert this directly because the original document type already exists
                self.batch_insert_empty_tree(
                    contract_documents_path,
                    KeyRef(type_key.as_bytes()),
                    storage_flags.as_ref().map(|flags| flags.as_ref()),
                    &mut batch_operations,
                )?;

                let type_path = [
                    contract_documents_path[0],
                    contract_documents_path[1],
                    contract_documents_path[2],
                    type_key.as_bytes(),
                ];

                // primary key tree
                self.batch_insert_empty_tree(
                    type_path,
                    KeyRef(&[0]),
                    storage_flags.as_ref().map(|flags| flags.as_ref()),
                    &mut batch_operations,
                )?;

                let mut index_cache: HashSet<&[u8]> = HashSet::new();
                // for each type we should insert the indices that are top level
                for index in document_type.top_level_indices() {
                    // toDo: change this to be a reference by index
                    let index_bytes = index.name.as_bytes();
                    if !index_cache.contains(index_bytes) {
                        self.batch_insert_empty_tree(
                            type_path,
                            KeyRef(index.name.as_bytes()),
                            storage_flags.as_ref().map(|flags| flags.as_ref()),
                            &mut batch_operations,
                        )?;
                        index_cache.insert(index_bytes);
                    }
                }
            }
        }
        Ok(batch_operations)
    }

    /// Applies a contract CBOR.
    pub fn apply_contract_cbor(
        &self,
        contract_cbor: Vec<u8>,
        contract_id: Option<[u8; 32]>,
        block_info: BlockInfo,
        apply: bool,
        storage_flags: Option<Cow<StorageFlags>>,
        transaction: TransactionArg,
    ) -> Result<FeeResult, Error> {
        // first we need to deserialize the contract
        let contract = DataContract::from_cbor_with_id(
            &contract_cbor,
            contract_id.map(|identifier| Identifier::from(identifier)),
        )?;

        self.apply_contract(&contract, block_info, apply, storage_flags, transaction)
    }

    /// Returns the contract with fetch info and operations with the given ID.
    pub fn query_contract_as_serialized(
        &self,
        contract_id: [u8; 32],
        encoding: QueryResultEncoding,
        transaction: TransactionArg,
    ) -> Result<Vec<u8>, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = Vec::new();

        let contract_fetch_info = self.get_contract_with_fetch_info_and_add_to_operations(
            contract_id,
            None,
            false, //querying the contract should not lead to it being added to the cache
            transaction,
            &mut drive_operations,
        )?;

        let contract_value = match contract_fetch_info {
            None => Value::Null,
            Some(contract_fetch_info) => {
                let contract = &contract_fetch_info.contract;
                contract.to_object()?
            }
        };

        let value = platform_value!({ "contract": contract_value });

        encoding.encode_value(&value)
    }

    /// Returns the contract with fetch info and operations with the given ID.
    pub fn get_contract_with_fetch_info(
        &self,
        contract_id: [u8; 32],
        epoch: Option<&Epoch>,
        add_to_cache_if_pulled: bool,
        transaction: TransactionArg,
    ) -> Result<(Option<FeeResult>, Option<Arc<ContractFetchInfo>>), Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = Vec::new();

        let contract_fetch_info = self.get_contract_with_fetch_info_and_add_to_operations(
            contract_id,
            epoch,
            add_to_cache_if_pulled,
            transaction,
            &mut drive_operations,
        )?;
        let fee_result = epoch.map_or(Ok(None), |epoch| {
            calculate_fee(None, Some(drive_operations), epoch).map(Some)
        })?;
        Ok((fee_result, contract_fetch_info))
    }

    /// Returns the contract with fetch info and operations with the given ID.
    pub(crate) fn get_contract_with_fetch_info_and_add_to_operations(
        &self,
        contract_id: [u8; 32],
        epoch: Option<&Epoch>,
        add_to_cache_if_pulled: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<Option<Arc<ContractFetchInfo>>, Error> {
        let cache = self.cache.read().unwrap();

        match cache
            .cached_contracts
            .get(contract_id, transaction.is_some())
        {
            None => {
                let maybe_contract_fetch_info = self.fetch_contract_and_add_operations(
                    contract_id,
                    epoch,
                    transaction,
                    drive_operations,
                )?;

                if add_to_cache_if_pulled {
                    // Store a contract in cache if present
                    if let Some(contract_fetch_info) = &maybe_contract_fetch_info {
                        drop(cache);
                        let mut cache = self.cache.write().unwrap();
                        cache
                            .cached_contracts
                            .insert(Arc::clone(contract_fetch_info), transaction.is_some());
                    };
                }
                Ok(maybe_contract_fetch_info)
            }
            Some(contract_fetch_info) => {
                // we only need to pay if epoch is set
                if let Some(epoch) = epoch {
                    let fee = if let Some(known_fee) = &contract_fetch_info.fee {
                        known_fee.clone()
                    } else {
                        // we need to calculate new fee
                        let op = vec![CalculatedCostOperation(contract_fetch_info.cost.clone())];
                        let fee = calculate_fee(None, Some(op), epoch)?;

                        let updated_contract_fetch_info = Arc::new(ContractFetchInfo {
                            contract: contract_fetch_info.contract.clone(),
                            storage_flags: contract_fetch_info.storage_flags.clone(),
                            cost: contract_fetch_info.cost.clone(),
                            fee: Some(fee.clone()),
                        });
                        drop(cache);
                        let mut cache = self.cache.write().unwrap();
                        // we override the cache for the contract as the fee is now calculated
                        cache
                            .cached_contracts
                            .insert(updated_contract_fetch_info, transaction.is_some());

                        fee
                    };
                    drive_operations.push(PreCalculatedFeeResult(fee));
                }
                Ok(Some(contract_fetch_info))
            }
        }
    }

    /// Fetch contract from database and add operations
    fn fetch_contract_and_add_operations(
        &self,
        contract_id: [u8; 32],
        epoch: Option<&Epoch>,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<Option<Arc<ContractFetchInfo>>, Error> {
        let mut cost = OperationCost::default();

        //todo: there is a cost here that isn't returned on error
        // we should investigate if this could be a problem
        let maybe_contract_fetch_info = self
            .fetch_contract(contract_id, epoch, transaction)
            .unwrap_add_cost(&mut cost)?;

        if let Some(contract_fetch_info) = &maybe_contract_fetch_info {
            // we only need to pay if epoch is set
            if epoch.is_some() {
                let fee = contract_fetch_info
                    .fee
                    .as_ref()
                    .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                    "should be impossible to not have fee on something just fetched with an epoch",
                )))?;
                drive_operations.push(PreCalculatedFeeResult(fee.clone()));
            }
        } else if epoch.is_some() {
            drive_operations.push(CalculatedCostOperation(cost));
        }

        Ok(maybe_contract_fetch_info)
    }

    /// Returns the contract fetch info with the given ID if it's in cache.
    pub fn get_cached_contract_with_fetch_info(
        &self,
        contract_id: [u8; 32],
        transaction: TransactionArg,
    ) -> Option<Arc<ContractFetchInfo>> {
        self.cache
            .read()
            .unwrap()
            .cached_contracts
            .get(contract_id, transaction.is_some())
            .map(|fetch_info| Arc::clone(&fetch_info))
    }

    /// Returns the contract with the given ID from storage and also inserts it in cache.
    pub fn fetch_contract(
        &self,
        contract_id: [u8; 32],
        epoch: Option<&Epoch>,
        transaction: TransactionArg,
    ) -> CostResult<Option<Arc<ContractFetchInfo>>, Error> {
        // As we want deterministic costs, we want the cost to always be the same for
        // fetching the contract.
        // We need to pass allow cache to false
        let CostContext { value, cost } = self.grove.get_raw_caching_optional(
            paths::contract_root_path(&contract_id),
            &[0],
            false,
            transaction,
        );

        match value {
            Ok(Element::Item(stored_contract_bytes, element_flag)) => {
                let contract = cost_return_on_error_no_add!(
                    &cost,
                    DataContract::deserialize_no_limit(&stored_contract_bytes)
                        .map_err(Error::Protocol)
                );
                let drive_operation = CalculatedCostOperation(cost.clone());
                let fee = if let Some(epoch) = epoch {
                    Some(cost_return_on_error_no_add!(
                        &cost,
                        calculate_fee(None, Some(vec![drive_operation]), epoch)
                    ))
                } else {
                    None
                };

                let storage_flags = cost_return_on_error_no_add!(
                    &cost,
                    StorageFlags::map_some_element_flags_ref(&element_flag)
                );
                let contract_fetch_info = Arc::new(ContractFetchInfo {
                    contract,
                    storage_flags,
                    cost: cost.clone(),
                    fee,
                });

                Ok(Some(Arc::clone(&contract_fetch_info))).wrap_with_cost(cost)
            }
            Ok(_) => Err(Error::Drive(DriveError::CorruptedContractPath(
                "contract path did not refer to a contract element",
            )))
            .wrap_with_cost(cost),
            Err(
                grovedb::Error::PathKeyNotFound(_)
                | grovedb::Error::PathParentLayerNotFound(_)
                | grovedb::Error::PathNotFound(_),
            ) => Ok(None).wrap_with_cost(cost),
            Err(e) => Err(Error::GroveDB(e)).wrap_with_cost(cost),
        }
    }

    /// Applies a contract and returns the fee for applying.
    /// If the contract already exists, an update is applied, otherwise an insert.
    pub fn apply_contract(
        &self,
        contract: &Contract,
        block_info: BlockInfo,
        apply: bool,
        storage_flags: Option<Cow<StorageFlags>>,
        transaction: TransactionArg,
    ) -> Result<FeeResult, Error> {
        self.apply_contract_with_serialization(
            contract,
            contract.serialize()?,
            block_info,
            apply,
            storage_flags,
            transaction,
        )
    }

    /// Applies a contract and returns the fee for applying.
    /// If the contract already exists, an update is applied, otherwise an insert.
    pub fn apply_contract_with_serialization(
        &self,
        contract: &Contract,
        contract_serialization: Vec<u8>,
        block_info: BlockInfo,
        apply: bool,
        storage_flags: Option<Cow<StorageFlags>>,
        transaction: TransactionArg,
    ) -> Result<FeeResult, Error> {
        let mut cost_operations = vec![];
        let mut estimated_costs_only_with_layer_info = if apply {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };
        let batch_operations = self.apply_contract_with_serialization_operations(
            contract,
            contract_serialization,
            &block_info,
            &mut estimated_costs_only_with_layer_info,
            storage_flags,
            transaction,
        )?;
        let fetch_cost = LowLevelDriveOperation::combine_cost_operations(&batch_operations);
        self.apply_batch_low_level_drive_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            batch_operations,
            &mut cost_operations,
        )?;
        cost_operations.push(CalculatedCostOperation(fetch_cost));
        let fees = calculate_fee(None, Some(cost_operations), &block_info.epoch)?;
        Ok(fees)
    }

    /// Gets the operations for applying a contract
    /// If the contract already exists, we get operations for an update
    /// Otherwise we get operations for an insert
    pub(crate) fn apply_contract_operations(
        &self,
        contract: &Contract,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        storage_flags: Option<Cow<StorageFlags>>,
        transaction: TransactionArg,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let serialized_contract = contract.serialize().map_err(Error::Protocol)?;
        self.apply_contract_with_serialization_operations(
            contract,
            serialized_contract,
            block_info,
            estimated_costs_only_with_layer_info,
            storage_flags,
            transaction,
        )
    }

    /// Gets the operations for applying a contract with it's serialization
    /// If the contract already exists, we get operations for an update
    /// Otherwise we get operations for an insert
    pub(crate) fn apply_contract_with_serialization_operations(
        &self,
        contract: &Contract,
        contract_serialization: Vec<u8>,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        storage_flags: Option<Cow<StorageFlags>>,
        transaction: TransactionArg,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];

        // overlying structure
        let mut already_exists = false;
        let mut original_contract_stored_data = vec![];

        // no estimated_costs_only_with_layer_info, means we want to apply to state
        let direct_query_type = if estimated_costs_only_with_layer_info.is_none() {
            DirectQueryType::StatefulDirectQuery
        } else {
            DirectQueryType::StatelessDirectQuery {
                in_tree_using_sums: false,
                // we can ignore flags as this is just an approximation
                // and it's doubtful that contracts will always be inserted at max size
                query_target: QueryTargetValue(CONTRACT_MAX_SERIALIZED_SIZE as u32),
            }
        };

        // We can do a get direct because there are no references involved
        if let Ok(Some(stored_element)) = self.grove_get_raw(
            contract_root_path(contract.id.as_bytes()),
            &[0],
            direct_query_type,
            transaction,
            &mut drive_operations,
        ) {
            already_exists = true;
            match stored_element {
                Element::Item(stored_contract_bytes, _) => {
                    if contract_serialization != stored_contract_bytes {
                        original_contract_stored_data = stored_contract_bytes;
                    }
                }
                _ => {
                    already_exists = false;
                }
            }
        };

        let contract_element = Element::Item(
            contract_serialization,
            StorageFlags::map_cow_to_some_element_flags(storage_flags),
        );

        if already_exists {
            if !original_contract_stored_data.is_empty() {
                let original_contract = Contract::deserialize(&original_contract_stored_data)?;
                // if the contract is not mutable update_contract will return an error
                self.update_contract_add_operations(
                    contract_element,
                    contract,
                    &original_contract,
                    block_info,
                    estimated_costs_only_with_layer_info,
                    transaction,
                    &mut drive_operations,
                )?;
            }
        } else {
            self.insert_contract_add_operations(
                contract_element,
                contract,
                block_info,
                estimated_costs_only_with_layer_info,
                &mut drive_operations,
            )?;
        }
        Ok(drive_operations)
    }
}

#[cfg(feature = "full")]
#[cfg(test)]
mod tests {
    use crate::contract::CreateRandomDocument;
    use rand::Rng;
    use std::option::Option::None;
    use tempfile::TempDir;

    use super::*;
    use crate::contract::Contract;
    use crate::drive::flags::StorageFlags;
    use crate::drive::object_size_info::{
        DocumentAndContractInfo, DocumentInfo, OwnedDocumentInfo,
    };
    use crate::drive::Drive;
    use dpp::data_contract::extra::common::json_document_to_contract;

    use crate::tests::helpers::setup::setup_drive_with_initial_state_structure;

    fn setup_deep_nested_50_contract() -> (Drive, DataContract) {
        // Todo: make TempDir based on _prefix
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        drive
            .create_initial_state_structure(None)
            .expect("expected to create root tree successfully");

        let contract_path = "tests/supporting_files/contract/deepNested/deep-nested50.json";
        // let's construct the grovedb structure for the dashpay data contract
        let contract =
            json_document_to_contract(contract_path).expect("expected to get a contract");
        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
            )
            .expect("expected to apply contract successfully");

        (drive, contract)
    }

    fn setup_deep_nested_10_contract() -> (Drive, DataContract) {
        // Todo: make TempDir based on _prefix
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        drive
            .create_initial_state_structure(None)
            .expect("expected to create root tree successfully");

        let contract_path = "tests/supporting_files/contract/deepNested/deep-nested10.json";
        // let's construct the grovedb structure for the dashpay data contract
        let contract =
            json_document_to_contract(contract_path).expect("expected to get a contract");
        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
            )
            .expect("expected to apply contract successfully");

        (drive, contract)
    }

    fn setup_reference_contract() -> (Drive, Contract) {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        drive
            .create_initial_state_structure(None)
            .expect("expected to create root tree successfully");

        let contract_path = "tests/supporting_files/contract/references/references.json";

        // let's construct the grovedb structure for the dashpay data contract
        let contract =
            json_document_to_contract(contract_path).expect("expected to get a contract");
        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
            )
            .expect("expected to apply contract successfully");

        (drive, contract)
    }

    #[test]
    fn test_create_and_update_contract() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        drive
            .create_initial_state_structure(None)
            .expect("expected to create root tree successfully");

        let initial_contract_cbor = hex::decode("01a66324696458209c2b800c5ea525d032a9fda4dda22a896f1e763af5f0e15ae7f93882b7439d77652464656673a1686c6173744e616d65a1647479706566737472696e676724736368656d61783468747470733a2f2f736368656d612e646173682e6f72672f6470702d302d342d302f6d6574612f646174612d636f6e7472616374676f776e657249645820636d3188dfffe62efb10e20347ec6c41b3e49fa31cb757ef4bad6cd8f1c7f4b66776657273696f6e0169646f63756d656e7473a76b756e697175654461746573a56474797065666f626a65637467696e646963657382a3646e616d6566696e6465783166756e69717565f56a70726f7065727469657382a16a2463726561746564417463617363a16a2475706461746564417463617363a2646e616d6566696e646578326a70726f7065727469657381a16a2475706461746564417463617363687265717569726564836966697273744e616d656a246372656174656441746a247570646174656441746a70726f70657274696573a2686c6173744e616d65a1647479706566737472696e676966697273744e616d65a1647479706566737472696e67746164646974696f6e616c50726f70657274696573f46c6e696365446f63756d656e74a46474797065666f626a656374687265717569726564816a246372656174656441746a70726f70657274696573a1646e616d65a1647479706566737472696e67746164646974696f6e616c50726f70657274696573f46e6e6f54696d65446f63756d656e74a36474797065666f626a6563746a70726f70657274696573a1646e616d65a1647479706566737472696e67746164646974696f6e616c50726f70657274696573f46e707265747479446f63756d656e74a46474797065666f626a65637468726571756972656482686c6173744e616d656a247570646174656441746a70726f70657274696573a1686c6173744e616d65a1642472656670232f24646566732f6c6173744e616d65746164646974696f6e616c50726f70657274696573f46e7769746842797465417272617973a56474797065666f626a65637467696e646963657381a2646e616d6566696e646578316a70726f7065727469657381a16e6279746541727261794669656c6463617363687265717569726564816e6279746541727261794669656c646a70726f70657274696573a26e6279746541727261794669656c64a36474797065656172726179686d61784974656d731069627974654172726179f56f6964656e7469666965724669656c64a56474797065656172726179686d61784974656d731820686d696e4974656d73182069627974654172726179f570636f6e74656e744d656469615479706578216170706c69636174696f6e2f782e646173682e6470702e6964656e746966696572746164646974696f6e616c50726f70657274696573f46f696e6465786564446f63756d656e74a56474797065666f626a65637467696e646963657386a3646e616d6566696e6465783166756e69717565f56a70726f7065727469657382a168246f776e6572496463617363a16966697273744e616d656464657363a3646e616d6566696e6465783266756e69717565f56a70726f7065727469657382a168246f776e6572496463617363a1686c6173744e616d656464657363a2646e616d6566696e646578336a70726f7065727469657381a1686c6173744e616d6563617363a2646e616d6566696e646578346a70726f7065727469657382a16a2463726561746564417463617363a16a2475706461746564417463617363a2646e616d6566696e646578356a70726f7065727469657381a16a2475706461746564417463617363a2646e616d6566696e646578366a70726f7065727469657381a16a2463726561746564417463617363687265717569726564846966697273744e616d656a246372656174656441746a24757064617465644174686c6173744e616d656a70726f70657274696573a2686c6173744e616d65a2647479706566737472696e67696d61784c656e6774681901006966697273744e616d65a2647479706566737472696e67696d61784c656e677468190100746164646974696f6e616c50726f70657274696573f4781d6f7074696f6e616c556e69717565496e6465786564446f63756d656e74a56474797065666f626a65637467696e646963657383a3646e616d6566696e6465783166756e69717565f56a70726f7065727469657381a16966697273744e616d656464657363a3646e616d6566696e6465783266756e69717565f56a70726f7065727469657383a168246f776e6572496463617363a16966697273744e616d6563617363a1686c6173744e616d6563617363a3646e616d6566696e6465783366756e69717565f56a70726f7065727469657382a167636f756e74727963617363a1646369747963617363687265717569726564826966697273744e616d65686c6173744e616d656a70726f70657274696573a46463697479a2647479706566737472696e67696d61784c656e67746819010067636f756e747279a2647479706566737472696e67696d61784c656e677468190100686c6173744e616d65a2647479706566737472696e67696d61784c656e6774681901006966697273744e616d65a2647479706566737472696e67696d61784c656e677468190100746164646974696f6e616c50726f70657274696573f4").unwrap();

        drive
            .apply_contract_cbor(
                initial_contract_cbor,
                None,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
            )
            .expect("expected to apply contract successfully");

        let updated_contract_cbor = hex::decode("01a66324696458209c2b800c5ea525d032a9fda4dda22a896f1e763af5f0e15ae7f93882b7439d77652464656673a1686c6173744e616d65a1647479706566737472696e676724736368656d61783468747470733a2f2f736368656d612e646173682e6f72672f6470702d302d342d302f6d6574612f646174612d636f6e7472616374676f776e657249645820636d3188dfffe62efb10e20347ec6c41b3e49fa31cb757ef4bad6cd8f1c7f4b66776657273696f6e0269646f63756d656e7473a86b756e697175654461746573a56474797065666f626a65637467696e646963657382a3646e616d6566696e6465783166756e69717565f56a70726f7065727469657382a16a2463726561746564417463617363a16a2475706461746564417463617363a2646e616d6566696e646578326a70726f7065727469657381a16a2475706461746564417463617363687265717569726564836966697273744e616d656a246372656174656441746a247570646174656441746a70726f70657274696573a2686c6173744e616d65a1647479706566737472696e676966697273744e616d65a1647479706566737472696e67746164646974696f6e616c50726f70657274696573f46c6e696365446f63756d656e74a46474797065666f626a656374687265717569726564816a246372656174656441746a70726f70657274696573a1646e616d65a1647479706566737472696e67746164646974696f6e616c50726f70657274696573f46e6e6f54696d65446f63756d656e74a36474797065666f626a6563746a70726f70657274696573a1646e616d65a1647479706566737472696e67746164646974696f6e616c50726f70657274696573f46e707265747479446f63756d656e74a46474797065666f626a65637468726571756972656482686c6173744e616d656a247570646174656441746a70726f70657274696573a1686c6173744e616d65a1642472656670232f24646566732f6c6173744e616d65746164646974696f6e616c50726f70657274696573f46e7769746842797465417272617973a56474797065666f626a65637467696e646963657381a2646e616d6566696e646578316a70726f7065727469657381a16e6279746541727261794669656c6463617363687265717569726564816e6279746541727261794669656c646a70726f70657274696573a26e6279746541727261794669656c64a36474797065656172726179686d61784974656d731069627974654172726179f56f6964656e7469666965724669656c64a56474797065656172726179686d61784974656d731820686d696e4974656d73182069627974654172726179f570636f6e74656e744d656469615479706578216170706c69636174696f6e2f782e646173682e6470702e6964656e746966696572746164646974696f6e616c50726f70657274696573f46f696e6465786564446f63756d656e74a56474797065666f626a65637467696e646963657386a3646e616d6566696e6465783166756e69717565f56a70726f7065727469657382a168246f776e6572496463617363a16966697273744e616d656464657363a3646e616d6566696e6465783266756e69717565f56a70726f7065727469657382a168246f776e6572496463617363a1686c6173744e616d656464657363a2646e616d6566696e646578336a70726f7065727469657381a1686c6173744e616d6563617363a2646e616d6566696e646578346a70726f7065727469657382a16a2463726561746564417463617363a16a2475706461746564417463617363a2646e616d6566696e646578356a70726f7065727469657381a16a2475706461746564417463617363a2646e616d6566696e646578366a70726f7065727469657381a16a2463726561746564417463617363687265717569726564846966697273744e616d656a246372656174656441746a24757064617465644174686c6173744e616d656a70726f70657274696573a2686c6173744e616d65a2647479706566737472696e67696d61784c656e6774681901006966697273744e616d65a2647479706566737472696e67696d61784c656e677468190100746164646974696f6e616c50726f70657274696573f4716d79417765736f6d65446f63756d656e74a56474797065666f626a65637467696e646963657382a3646e616d656966697273744e616d6566756e69717565f56a70726f7065727469657381a16966697273744e616d6563617363a3646e616d657166697273744e616d654c6173744e616d6566756e69717565f56a70726f7065727469657382a16966697273744e616d6563617363a1686c6173744e616d6563617363687265717569726564846966697273744e616d656a246372656174656441746a24757064617465644174686c6173744e616d656a70726f70657274696573a2686c6173744e616d65a2647479706566737472696e67696d61784c656e6774681901006966697273744e616d65a2647479706566737472696e67696d61784c656e677468190100746164646974696f6e616c50726f70657274696573f4781d6f7074696f6e616c556e69717565496e6465786564446f63756d656e74a56474797065666f626a65637467696e646963657383a3646e616d6566696e6465783166756e69717565f56a70726f7065727469657381a16966697273744e616d656464657363a3646e616d6566696e6465783266756e69717565f56a70726f7065727469657383a168246f776e6572496463617363a16966697273744e616d6563617363a1686c6173744e616d6563617363a3646e616d6566696e6465783366756e69717565f56a70726f7065727469657382a167636f756e74727963617363a1646369747963617363687265717569726564826966697273744e616d65686c6173744e616d656a70726f70657274696573a46463697479a2647479706566737472696e67696d61784c656e67746819010067636f756e747279a2647479706566737472696e67696d61784c656e677468190100686c6173744e616d65a2647479706566737472696e67696d61784c656e6774681901006966697273744e616d65a2647479706566737472696e67696d61784c656e677468190100746164646974696f6e616c50726f70657274696573f4").unwrap();

        drive
            .apply_contract_cbor(
                updated_contract_cbor,
                None,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
            )
            .expect("should update initial contract");
    }

    #[test]
    fn test_create_deep_nested_contract_50() {
        let (drive, contract) = setup_deep_nested_50_contract();

        let document_type = contract
            .document_type_for_name("nest")
            .expect("expected to get document type");

        let document = document_type.random_document(Some(5));

        let nested_value = document.properties.get("abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0.abc0");

        assert!(nested_value.is_some());

        let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpoch(0)));

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentInfo::DocumentRefInfo((&document, storage_flags)),
                        owner_id: Some(random_owner_id),
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
    }

    #[test]
    fn test_create_reference_contract() {
        let (drive, contract) = setup_reference_contract();

        let document_type = contract
            .document_type_for_name("note")
            .expect("expected to get document type");

        let document = document_type.random_document(Some(5));

        let ref_value = document.properties.get("abc17");

        assert!(ref_value.is_some());

        let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpoch(0)));

        let random_owner_id = rand::thread_rng().gen::<[u8; 32]>();
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentInfo::DocumentRefInfo((&document, storage_flags)),
                        owner_id: Some(random_owner_id),
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
    }

    #[test]
    fn test_create_reference_contract_without_apply() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        drive
            .create_initial_state_structure(None)
            .expect("expected to create root tree successfully");

        let contract_path = "tests/supporting_files/contract/references/references.json";

        // let's construct the grovedb structure for the dashpay data contract
        let contract =
            json_document_to_contract(contract_path).expect("expected to get cbor document");
        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                false,
                StorageFlags::optional_default_as_cow(),
                None,
            )
            .expect("expected to apply contract successfully");
    }

    #[test]
    fn test_create_reference_contract_with_history_without_apply() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        drive
            .create_initial_state_structure(None)
            .expect("expected to create root tree successfully");

        let contract_path =
            "tests/supporting_files/contract/references/references_with_contract_history.json";

        // let's construct the grovedb structure for the dashpay data contract
        let contract = json_document_to_contract(contract_path).expect("expected to get contract");
        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                false,
                StorageFlags::optional_default_as_cow(),
                None,
            )
            .expect("expected to apply contract successfully");
    }

    #[test]
    fn test_update_reference_contract_without_apply() {
        let tmp_dir = TempDir::new().unwrap();
        let drive: Drive = Drive::open(tmp_dir, None).expect("expected to open Drive successfully");

        drive
            .create_initial_state_structure(None)
            .expect("expected to create root tree successfully");

        let contract_path = "tests/supporting_files/contract/references/references.json";

        // let's construct the grovedb structure for the dashpay data contract
        let contract =
            json_document_to_contract(contract_path).expect("expected to get cbor document");

        // Create a contract first
        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
            )
            .expect("expected to apply contract successfully");

        // Update existing contract
        drive
            .update_contract(&contract, BlockInfo::default(), false, None)
            .expect("expected to apply contract successfully");
    }

    mod get_contract_with_fetch_info {
        use super::*;
        use dpp::prelude::Identifier;
        use dpp::Convertible;

        #[test]
        fn should_get_contract_from_global_and_block_cache() {
            let (drive, mut contract) = setup_reference_contract();

            let transaction = drive.grove.start_transaction();

            contract.increment_version();

            drive
                .update_contract(&contract, BlockInfo::default(), true, Some(&transaction))
                .expect("should update contract");

            let fetch_info_from_database = drive
                .get_contract_with_fetch_info(contract.id.to_buffer(), None, true, None)
                .expect("should get contract")
                .1
                .expect("should be present");

            assert_eq!(fetch_info_from_database.contract.version, 1);

            let fetch_info_from_cache = drive
                .get_contract_with_fetch_info(
                    contract.id.to_buffer(),
                    None,
                    true,
                    Some(&transaction),
                )
                .expect("should get contract")
                .1
                .expect("should be present");

            assert_eq!(fetch_info_from_cache.contract.version, 2);
        }

        #[test]
        fn should_return_none_if_contract_not_exist() {
            let drive = setup_drive_with_initial_state_structure();

            let result = drive
                .get_contract_with_fetch_info([0; 32], None, true, None)
                .expect("should get contract");

            assert!(result.0.is_none());
            assert!(result.1.is_none());
        }

        #[test]
        fn should_return_fees_for_non_existing_contract_if_epoch_is_passed() {
            let drive = setup_drive_with_initial_state_structure();

            let result = drive
                .get_contract_with_fetch_info([0; 32], Some(&Epoch::new(0).unwrap()), true, None)
                .expect("should get contract");

            assert_eq!(
                result.0,
                Some(FeeResult {
                    processing_fee: 4060,
                    ..Default::default()
                })
            );

            assert!(result.1.is_none());
        }

        #[test]
        fn should_always_have_then_same_cost() {
            // Merk trees have own cache and depends on does contract node cached or not
            // we get could get different costs. To avoid of it we fetch contracts without tree caching

            let (drive, mut ref_contract) = setup_reference_contract();

            /*
             * Firstly, we create multiple contracts during block processing (in transaction)
             */

            let ref_contract_id_buffer = Identifier::from([0; 32]).to_buffer();

            let transaction = drive.grove.start_transaction();

            // Create more contracts to trigger re-balancing
            for i in 0..150u8 {
                ref_contract.id = Identifier::from([i; 32]);

                drive
                    .apply_contract(
                        &ref_contract,
                        BlockInfo::default(),
                        true,
                        StorageFlags::optional_default_as_cow(),
                        Some(&transaction),
                    )
                    .expect("expected to apply contract successfully");
            }

            // Create a deep placed contract
            let contract_path = "tests/supporting_files/contract/deepNested/deep-nested10.json";
            let deep_contract =
                json_document_to_contract(contract_path).expect("expected to get cbor document");
            drive
                .apply_contract(
                    &deep_contract,
                    BlockInfo::default(),
                    true,
                    StorageFlags::optional_default_as_cow(),
                    Some(&transaction),
                )
                .expect("expected to apply contract successfully");

            let mut ref_contract_fetch_info_transactional = drive
                .get_contract_with_fetch_info(
                    ref_contract_id_buffer,
                    Some(&Epoch::new(0).unwrap()),
                    true,
                    Some(&transaction),
                )
                .expect("got contract")
                .1
                .expect("got contract fetch info");

            let mut deep_contract_fetch_info_transactional = drive
                .get_contract_with_fetch_info(
                    deep_contract.id.to_buffer(),
                    Some(&Epoch::new(0).unwrap()),
                    true,
                    Some(&transaction),
                )
                .expect("got contract")
                .1
                .expect("got contract fetch info");

            /*
             * Then we commit the block
             */

            // Commit transaction and merge block (transactional) cache to global cache
            transaction.commit().expect("expected to commit");

            let mut drive_cache = drive.cache.write().unwrap();
            drive_cache.cached_contracts.merge_block_cache();
            drop(drive_cache);

            /*
             * Contracts fetched with user query and during block execution must have equal costs
             */

            let deep_contract_fetch_info = drive
                .get_contract_with_fetch_info(deep_contract.id.to_buffer(), None, true, None)
                .expect("got contract")
                .1
                .expect("got contract fetch info");

            let ref_contract_fetch_info = drive
                .get_contract_with_fetch_info(ref_contract_id_buffer, None, true, None)
                .expect("got contract")
                .1
                .expect("got contract fetch info");

            assert_eq!(
                deep_contract_fetch_info_transactional,
                deep_contract_fetch_info
            );

            assert_eq!(
                ref_contract_fetch_info_transactional,
                ref_contract_fetch_info
            );

            /*
             * User restarts the node
             */

            // Drop cache so contract will be fetched once again
            drive.drop_cache();

            /*
             * Other nodes weren't restarted so contracts queried by user after restart
             * must have the same costs as transactional contracts and contracts before
             * restart
             */

            let deep_contract_fetch_info_without_cache = drive
                .get_contract_with_fetch_info(deep_contract.id.to_buffer(), None, true, None)
                .expect("got contract")
                .1
                .expect("got contract fetch info");

            let ref_contract_fetch_info_without_cache = drive
                .get_contract_with_fetch_info(ref_contract_id_buffer, None, true, None)
                .expect("got contract")
                .1
                .expect("got contract fetch info");

            // Remove fees to match with fetch with epoch provided
            let mut deep_contract_fetch_info_transactional_without_arc =
                Arc::make_mut(&mut deep_contract_fetch_info_transactional);

            deep_contract_fetch_info_transactional_without_arc.fee = None;

            let mut ref_contract_fetch_info_transactional_without_arc =
                Arc::make_mut(&mut ref_contract_fetch_info_transactional);

            ref_contract_fetch_info_transactional_without_arc.fee = None;

            assert_eq!(
                deep_contract_fetch_info_transactional,
                deep_contract_fetch_info_without_cache
            );
            assert_eq!(
                ref_contract_fetch_info_transactional,
                ref_contract_fetch_info_without_cache
            );

            /*
             * Let's imagine that many blocks were executed and the node is restarted again
             */
            drive.drop_cache();

            /*
             * Drive executes a new block
             */

            let transaction = drive.grove.start_transaction();

            // Create more contracts to trigger re-balancing
            for i in 150..200u8 {
                ref_contract.id = Identifier::from([i; 32]);

                drive
                    .apply_contract(
                        &ref_contract,
                        BlockInfo::default(),
                        true,
                        StorageFlags::optional_default_as_cow(),
                        Some(&transaction),
                    )
                    .expect("expected to apply contract successfully");
            }

            /*
             * Other nodes weren't restarted so contracts fetched during block execution
             * should have the same cost as previously fetched contracts
             */

            let mut deep_contract_fetch_info_transactional2 = drive
                .get_contract_with_fetch_info(
                    deep_contract.id.to_buffer(),
                    Some(&Epoch::new(0).unwrap()),
                    true,
                    Some(&transaction),
                )
                .expect("got contract")
                .1
                .expect("got contract fetch info");

            let mut ref_contract_fetch_info_transactional2 = drive
                .get_contract_with_fetch_info(
                    ref_contract_id_buffer,
                    Some(&Epoch::new(0).unwrap()),
                    true,
                    Some(&transaction),
                )
                .expect("got contract")
                .1
                .expect("got contract fetch info");

            // Remove fees to match with fetch with epoch provided
            let mut deep_contract_fetch_info_transactional_without_arc =
                Arc::make_mut(&mut deep_contract_fetch_info_transactional2);

            deep_contract_fetch_info_transactional_without_arc.fee = None;

            let mut ref_contract_fetch_info_transactional_without_arc =
                Arc::make_mut(&mut ref_contract_fetch_info_transactional2);

            ref_contract_fetch_info_transactional_without_arc.fee = None;

            assert_eq!(
                ref_contract_fetch_info_transactional,
                ref_contract_fetch_info_transactional2,
            );

            assert_eq!(
                deep_contract_fetch_info_transactional,
                deep_contract_fetch_info_transactional2
            );
        }
    }
}
