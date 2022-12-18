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

//! Grove Operations.
//!
//! Defines and implements in Drive functions pertinent to groveDB operations.
//!

use crate::drive::batch::GroveDbOpBatch;
use costs::storage_cost::removal::StorageRemovedBytes::BasicStorageRemoval;
use costs::storage_cost::transition::OperationStorageTransitionType;
use costs::CostContext;
use grovedb::batch::estimated_costs::EstimatedCostsType::AverageCaseCostsType;
use grovedb::batch::{key_info::KeyInfo, BatchApplyOptions, GroveDbOp, KeyInfoPath, Op};
use grovedb::{Element, EstimatedLayerInformation, GroveDb, PathQuery, TransactionArg};
use std::collections::HashMap;

use crate::drive::flags::StorageFlags;
use crate::drive::object_size_info::DriveKeyInfo::{Key, KeyRef, KeySize};

use crate::drive::object_size_info::PathKeyElementInfo::{
    PathFixedSizeKeyRefElement, PathKeyElement, PathKeyElementSize, PathKeyRefElement,
    PathKeyUnknownElementSize,
};
use crate::drive::object_size_info::PathKeyInfo::{
    PathFixedSizeKey, PathFixedSizeKeyRef, PathKey, PathKeyRef, PathKeySize,
};
use crate::drive::object_size_info::{DriveKeyInfo, PathKeyElementInfo, PathKeyInfo};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::op::DriveOperation;
use crate::fee::op::DriveOperation::CalculatedCostOperation;
use grovedb::operations::delete::DeleteOptions;
use grovedb::operations::insert::InsertOptions;
use grovedb::query_result_type::{QueryResultElements, QueryResultType};
use grovedb::Error as GroveError;
use intmap::IntMap;
use storage::rocksdb_storage::RocksDbStorage;

/// Pushes an operation's `OperationCost` to `drive_operations` given its `CostContext`
/// and returns the operation's return value.
fn push_drive_operation_result<T>(
    cost_context: CostContext<Result<T, GroveError>>,
    drive_operations: &mut Vec<DriveOperation>,
) -> Result<T, Error> {
    let CostContext { value, cost } = cost_context;
    drive_operations.push(CalculatedCostOperation(cost));
    value.map_err(Error::GroveDB)
}

/// Pushes an operation's `OperationCost` to `drive_operations` given its `CostContext`
/// if `drive_operations` is given. Returns the operation's return value.
fn push_drive_operation_result_optional<T>(
    cost_context: CostContext<Result<T, GroveError>>,
    drive_operations: Option<&mut Vec<DriveOperation>>,
) -> Result<T, Error> {
    let CostContext { value, cost } = cost_context;
    if let Some(drive_operations) = drive_operations {
        drive_operations.push(CalculatedCostOperation(cost));
    }
    value.map_err(Error::GroveDB)
}

pub type EstimatedIntermediateFlagSizes = IntMap<u32>;
pub type EstimatedValueSize = u32;
pub type IsSubTree = bool;
pub type IsSumSubTree = bool;
pub type IsSumTree = bool;

pub enum BatchDeleteApplyType {
    StatelessBatchDelete {
        is_sum_tree: bool,
        estimated_value_size: u32,
    },
    StatefulBatchDelete {
        is_known_to_be_subtree_with_sum: Option<(IsSubTree, IsSumSubTree)>,
    },
}

pub enum BatchDeleteUpTreeApplyType {
    StatelessBatchDelete {
        estimated_layer_info: IntMap<EstimatedLayerInformation>,
    },
    StatefulBatchDelete {
        is_known_to_be_subtree_with_sum: Option<(IsSubTree, IsSumSubTree)>,
    },
}

#[derive(Clone, Copy)]
pub enum BatchInsertTreeApplyType {
    StatelessBatchInsert {
        in_tree_using_sums: bool,
        is_sum_tree: bool,
        flags_len: FlagsLen,
    },
    StatefulBatchInsert,
}

impl BatchInsertTreeApplyType {
    pub(crate) fn to_direct_query_type(&self) -> DirectQueryType {
        match self {
            BatchInsertTreeApplyType::StatelessBatchInsert {
                in_tree_using_sums,
                is_sum_tree,
                flags_len,
            } => DirectQueryType::StatelessDirectQuery {
                in_tree_using_sums: *in_tree_using_sums,
                query_target: QueryTarget::QueryTargetTree(*flags_len, *is_sum_tree),
            },
            BatchInsertTreeApplyType::StatefulBatchInsert => DirectQueryType::StatefulDirectQuery,
        }
    }
}

pub enum BatchInsertApplyType {
    StatelessBatchInsert {
        in_tree_using_sums: bool,
        target: QueryTarget,
    },
    StatefulBatchInsert,
}

impl BatchInsertApplyType {
    pub(crate) fn to_direct_query_type(&self) -> DirectQueryType {
        match self {
            BatchInsertApplyType::StatelessBatchInsert {
                in_tree_using_sums,
                target,
            } => DirectQueryType::StatelessDirectQuery {
                in_tree_using_sums: *in_tree_using_sums,
                query_target: *target,
            },
            BatchInsertApplyType::StatefulBatchInsert => DirectQueryType::StatefulDirectQuery,
        }
    }
}

pub type FlagsLen = u32;

#[derive(Clone, Copy)]
pub enum QueryTarget {
    QueryTargetTree(FlagsLen, IsSumTree),
    QueryTargetValue(u32),
}

impl QueryTarget {
    pub(crate) fn len(&self) -> u32 {
        match self {
            QueryTarget::QueryTargetTree(flags_len, is_sum_tree) => {
                let len = if *is_sum_tree { 11 } else { 3 };
                *flags_len + len
            }
            QueryTarget::QueryTargetValue(len) => *len,
        }
    }
}

#[derive(Clone, Copy)]
pub enum DirectQueryType {
    StatelessDirectQuery {
        in_tree_using_sums: bool,
        query_target: QueryTarget,
    },
    StatefulDirectQuery,
}

impl DirectQueryType {
    pub(crate) fn to_query_type(self) -> QueryType {
        match self {
            DirectQueryType::StatelessDirectQuery {
                in_tree_using_sums,
                query_target,
            } => QueryType::StatelessQuery {
                in_tree_using_sums,
                query_target,
                estimated_reference_sizes: vec![],
            },
            DirectQueryType::StatefulDirectQuery => QueryType::StatefulQuery,
        }
    }

    pub(crate) fn add_reference_sizes(self, reference_sizes: Vec<u32>) -> QueryType {
        match self {
            DirectQueryType::StatelessDirectQuery {
                in_tree_using_sums,
                query_target,
            } => QueryType::StatelessQuery {
                in_tree_using_sums,
                query_target,
                estimated_reference_sizes: reference_sizes,
            },
            DirectQueryType::StatefulDirectQuery => QueryType::StatefulQuery,
        }
    }
}

#[derive(Clone)]
pub enum QueryType {
    StatelessQuery {
        in_tree_using_sums: bool,
        query_target: QueryTarget,
        estimated_reference_sizes: Vec<u32>,
    },
    StatefulQuery,
}

impl Drive {
    /// Pushes the `OperationCost` of inserting an element in groveDB to `drive_operations`.
    pub fn grove_insert<'p, P>(
        &self,
        path: P,
        key: &'p [u8],
        element: Element,
        transaction: TransactionArg,
        options: Option<InsertOptions>,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error>
    where
        P: IntoIterator<Item = &'p [u8]>,
        <P as IntoIterator>::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
    {
        let cost_context = self.grove.insert(path, key, element, options, transaction);
        push_drive_operation_result(cost_context, drive_operations)
    }

    /// Pushes the `OperationCost` of inserting an element in groveDB where the path key does not yet exist
    /// to `drive_operations`.
    pub fn grove_insert_if_not_exists<'p, P>(
        &self,
        path: P,
        key: &'p [u8],
        element: Element,
        transaction: TransactionArg,
        drive_operations: Option<&mut Vec<DriveOperation>>,
    ) -> Result<bool, Error>
    where
        P: IntoIterator<Item = &'p [u8]>,
        <P as IntoIterator>::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
    {
        let cost_context = self
            .grove
            .insert_if_not_exists(path, key, element, transaction);
        push_drive_operation_result_optional(cost_context, drive_operations)
    }

    /// Pushes the `OperationCost` of deleting an element in groveDB to `drive_operations`.
    pub fn grove_delete<'p, P>(
        &self,
        path: P,
        key: &'p [u8],
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error>
    where
        P: IntoIterator<Item = &'p [u8]>,
        <P as IntoIterator>::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
    {
        let options = DeleteOptions {
            allow_deleting_non_empty_trees: false,
            deleting_non_empty_trees_returns_error: true,
            base_root_storage_is_free: true,
        };
        let cost_context = self.grove.delete(path, key, Some(options), transaction);
        push_drive_operation_result(cost_context, drive_operations)
    }

    /// grove_get_direct basically means that there are no reference hops, this only matters
    /// when calculating worst case costs
    pub fn grove_get_direct<'p, P>(
        &self,
        path: P,
        key: &'p [u8],
        direct_query_type: DirectQueryType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<Option<Element>, Error>
    where
        P: IntoIterator<Item = &'p [u8]>,
        <P as IntoIterator>::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
    {
        self.grove_get(
            path,
            key,
            direct_query_type.to_query_type(),
            transaction,
            drive_operations,
        )
    }

    /// Gets the element at the given path from groveDB.
    /// Pushes the `OperationCost` of getting the element to `drive_operations`.
    pub fn grove_get<'p, P>(
        &self,
        path: P,
        key: &'p [u8],
        query_type: QueryType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<Option<Element>, Error>
    where
        P: IntoIterator<Item = &'p [u8]>,
        <P as IntoIterator>::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
    {
        let path_iter = path.into_iter();
        // if let Some((max_value_size, max_reference_sizes)) =
        //     query_stateless_with_max_value_size_and_max_reference_sizes
        // {
        match query_type {
            QueryType::StatelessQuery {
                in_tree_using_sums,
                query_target,
                estimated_reference_sizes,
            } => {
                let key_info_path = KeyInfoPath::from_known_path(path_iter);
                let key_info = KeyInfo::KnownKey(key.to_vec());
                let cost = match query_target {
                    QueryTarget::QueryTargetTree(flags_size, is_sum_tree) => {
                        GroveDb::average_case_for_get_tree(
                            &key_info_path,
                            &key_info,
                            flags_size,
                            is_sum_tree,
                            in_tree_using_sums,
                        )
                    }
                    QueryTarget::QueryTargetValue(estimated_value_size) => {
                        GroveDb::average_case_for_get(
                            &key_info_path,
                            &key_info,
                            in_tree_using_sums,
                            estimated_value_size as u32,
                            estimated_reference_sizes,
                        )
                    }
                };

                drive_operations.push(CalculatedCostOperation(cost));
                Ok(None)
            }
            QueryType::StatefulQuery => {
                let CostContext { value, cost } = self.grove.get(path_iter, key, transaction);
                drive_operations.push(CalculatedCostOperation(cost));
                Ok(Some(value.map_err(Error::GroveDB)?))
            }
        }
    }

    /// Gets the return value and the cost of a groveDB path query.
    /// Pushes the cost to `drive_operations` and returns the return value.
    pub(crate) fn grove_get_path_query(
        &self,
        path_query: &PathQuery,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(Vec<Vec<u8>>, u16), Error> {
        let CostContext { value, cost } = self.grove.query(path_query, transaction);
        drive_operations.push(CalculatedCostOperation(cost));
        value.map_err(Error::GroveDB)
    }

    /// Gets the return value and the cost of a groveDB raw path query.
    /// Pushes the cost to `drive_operations` and returns the return value.
    pub(crate) fn grove_get_raw_path_query(
        &self,
        path_query: &PathQuery,
        transaction: TransactionArg,
        result_type: QueryResultType,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(QueryResultElements, u16), Error> {
        let CostContext { value, cost } =
            self.grove.query_raw(path_query, result_type, transaction);
        drive_operations.push(CalculatedCostOperation(cost));
        value.map_err(Error::GroveDB)
    }

    /// Gets the return value and the cost of a groveDB proved path query.
    /// Pushes the cost to `drive_operations` and returns the return value.
    pub(crate) fn grove_get_proved_path_query(
        &self,
        path_query: &PathQuery,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<Vec<u8>, Error> {
        let CostContext { value, cost } = self.grove.get_proved_path_query(path_query, transaction);
        drive_operations.push(CalculatedCostOperation(cost));
        value.map_err(Error::GroveDB)
    }

    /// Gets the return value and the cost of a groveDB `has_raw` operation.
    /// Pushes the cost to `drive_operations` and returns the return value.
    pub(crate) fn grove_has_raw<'p, P>(
        &self,
        path: P,
        key: &'p [u8],
        query_type: DirectQueryType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<bool, Error>
    where
        P: IntoIterator<Item = &'p [u8]>,
        <P as IntoIterator>::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
    {
        let CostContext { value, cost } = match query_type {
            DirectQueryType::StatelessDirectQuery {
                in_tree_using_sums,
                query_target,
            } => {
                let key_info_path = KeyInfoPath::from_known_path(path);
                let key_info = KeyInfo::KnownKey(key.to_vec());
                let cost = match query_target {
                    QueryTarget::QueryTargetTree(flags_len, is_sum_tree) => {
                        GroveDb::average_case_for_has_raw_tree(
                            &key_info_path,
                            &key_info,
                            flags_len,
                            is_sum_tree,
                            in_tree_using_sums,
                        )
                    }
                    QueryTarget::QueryTargetValue(estimated_value_size) => {
                        GroveDb::average_case_for_has_raw(
                            &key_info_path,
                            &key_info,
                            estimated_value_size as u32,
                            in_tree_using_sums,
                        )
                    }
                };

                CostContext {
                    value: Ok(false),
                    cost,
                }
            }
            DirectQueryType::StatefulDirectQuery => {
                if self.config.has_raw_enabled {
                    self.grove.has_raw(path, key, transaction)
                } else {
                    self.grove.get_raw(path, key, transaction).map(|r| match r {
                        Err(GroveError::PathKeyNotFound(_))
                        | Err(GroveError::PathNotFound(_))
                        | Err(GroveError::PathParentLayerNotFound(_)) => Ok(false),
                        Err(e) => Err(e),
                        Ok(_) => Ok(true),
                    })
                }
            }
        };
        drive_operations.push(CalculatedCostOperation(cost));
        Ok(value?)
    }

    /// Pushes an "insert empty tree" operation to `drive_operations`.
    pub(crate) fn batch_insert_empty_tree<'a, 'c, P>(
        &'a self,
        path: P,
        key_info: DriveKeyInfo<'c>,
        storage_flags: Option<&StorageFlags>,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error>
    where
        P: IntoIterator<Item = &'c [u8]>,
        <P as IntoIterator>::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
    {
        match key_info {
            KeyRef(key) => {
                let path_items: Vec<Vec<u8>> = path.into_iter().map(Vec::from).collect();
                drive_operations.push(DriveOperation::for_known_path_key_empty_tree(
                    path_items,
                    key.to_vec(),
                    storage_flags,
                ));
                Ok(())
            }
            KeySize(key) => {
                drive_operations.push(DriveOperation::for_estimated_path_key_empty_tree(
                    KeyInfoPath::from_known_path(path),
                    key,
                    storage_flags,
                ));
                Ok(())
            }
            Key(key) => {
                let path_items: Vec<Vec<u8>> = path.into_iter().map(Vec::from).collect();
                drive_operations.push(DriveOperation::for_known_path_key_empty_tree(
                    path_items,
                    key,
                    storage_flags,
                ));
                Ok(())
            }
        }
    }

    /// Pushes an "insert empty tree where path key does not yet exist" operation to `drive_operations`.
    pub(crate) fn batch_insert_empty_tree_if_not_exists<'a, 'c, const N: usize>(
        &'a self,
        path_key_info: PathKeyInfo<'c, N>,
        storage_flags: Option<&StorageFlags>,
        apply_type: BatchInsertTreeApplyType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<bool, Error> {
        match path_key_info {
            PathKeyRef((path, key)) => {
                let path_iter: Vec<&[u8]> = path.iter().map(|x| x.as_slice()).collect();
                let has_raw = self.grove_has_raw(
                    path_iter.clone(),
                    key,
                    apply_type.to_direct_query_type(),
                    transaction,
                    drive_operations,
                )?;
                if !has_raw {
                    drive_operations.push(DriveOperation::for_known_path_key_empty_tree(
                        path,
                        key.to_vec(),
                        storage_flags,
                    ));
                }
                Ok(!has_raw)
            }
            PathKeySize(_key_path_info, _key_info) => Err(Error::Drive(
                DriveError::NotSupportedPrivate("document sizes in batch operations not supported"),
            )),
            PathKey((path, key)) => {
                let path_iter: Vec<&[u8]> = path.iter().map(|x| x.as_slice()).collect();
                let has_raw = self.grove_has_raw(
                    path_iter.clone(),
                    key.as_slice(),
                    apply_type.to_direct_query_type(),
                    transaction,
                    drive_operations,
                )?;
                if !has_raw {
                    drive_operations.push(DriveOperation::for_known_path_key_empty_tree(
                        path,
                        key.to_vec(),
                        storage_flags,
                    ));
                }
                Ok(!has_raw)
            }
            PathFixedSizeKey((path, key)) => {
                let has_raw = self.grove_has_raw(
                    path,
                    key.as_slice(),
                    apply_type.to_direct_query_type(),
                    transaction,
                    drive_operations,
                )?;
                if !has_raw {
                    let path_items: Vec<Vec<u8>> = path.into_iter().map(Vec::from).collect();
                    drive_operations.push(DriveOperation::for_known_path_key_empty_tree(
                        path_items,
                        key.to_vec(),
                        storage_flags,
                    ));
                }
                Ok(!has_raw)
            }
            PathFixedSizeKeyRef((path, key)) => {
                let has_raw = self.grove_has_raw(
                    path,
                    key,
                    apply_type.to_direct_query_type(),
                    transaction,
                    drive_operations,
                )?;
                if !has_raw {
                    let path_items: Vec<Vec<u8>> = path.into_iter().map(Vec::from).collect();
                    drive_operations.push(DriveOperation::for_known_path_key_empty_tree(
                        path_items,
                        key.to_vec(),
                        storage_flags,
                    ));
                }
                Ok(!has_raw)
            }
        }
    }

    /// Pushes an "insert element" operation to `drive_operations`.
    pub(crate) fn batch_insert<const N: usize>(
        &self,
        path_key_element_info: PathKeyElementInfo<N>,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error> {
        match path_key_element_info {
            PathKeyRefElement((path, key, element)) => {
                drive_operations.push(DriveOperation::insert_for_known_path_key_element(
                    path,
                    key.to_vec(),
                    element,
                ));
                Ok(())
            }
            PathKeyElement((path, key, element)) => {
                drive_operations.push(DriveOperation::insert_for_known_path_key_element(
                    path, key, element,
                ));
                Ok(())
            }
            PathKeyElementSize((key_info_path, key_info, element)) => {
                drive_operations.push(DriveOperation::insert_for_estimated_path_key_element(
                    key_info_path,
                    key_info,
                    element,
                ));
                Ok(())
            }
            PathKeyUnknownElementSize(_) => Err(Error::Drive(DriveError::NotSupportedPrivate(
                "inserting unsized documents into a batch is not currently supported",
            ))),
            PathFixedSizeKeyRefElement((path, key, element)) => {
                let path_items: Vec<Vec<u8>> = path.into_iter().map(Vec::from).collect();
                drive_operations.push(DriveOperation::insert_for_known_path_key_element(
                    path_items,
                    key.to_vec(),
                    element,
                ));
                Ok(())
            }
        }
    }

    /// Pushes an "insert element if the path key does not yet exist" operation to `drive_operations`.
    /// Returns true if the path key already exists without references.
    pub(crate) fn batch_insert_if_not_exists<'a, 'c, const N: usize>(
        &'a self,
        path_key_element_info: PathKeyElementInfo<'c, N>,
        apply_type: BatchInsertApplyType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<bool, Error> {
        match path_key_element_info {
            PathKeyRefElement((path, key, element)) => {
                let path_iter: Vec<&[u8]> = path.iter().map(|x| x.as_slice()).collect();
                let has_raw = self.grove_has_raw(
                    path_iter.clone(),
                    key,
                    apply_type.to_direct_query_type(),
                    transaction,
                    drive_operations,
                )?;
                if !has_raw {
                    drive_operations.push(DriveOperation::insert_for_known_path_key_element(
                        path,
                        key.to_vec(),
                        element,
                    ));
                }
                Ok(!has_raw)
            }
            PathKeyElement((path, key, element)) => {
                let path_iter: Vec<&[u8]> = path.iter().map(|x| x.as_slice()).collect();
                let has_raw = self.grove_has_raw(
                    path_iter.clone(),
                    key.as_slice(),
                    apply_type.to_direct_query_type(),
                    transaction,
                    drive_operations,
                )?;
                if !has_raw {
                    drive_operations.push(DriveOperation::insert_for_known_path_key_element(
                        path, key, element,
                    ));
                }
                Ok(!has_raw)
            }
            PathFixedSizeKeyRefElement((path, key, element)) => {
                let has_raw = self.grove_has_raw(
                    path,
                    key,
                    apply_type.to_direct_query_type(),
                    transaction,
                    drive_operations,
                )?;
                if !has_raw {
                    let path_items: Vec<Vec<u8>> = path.into_iter().map(Vec::from).collect();
                    drive_operations.push(DriveOperation::insert_for_known_path_key_element(
                        path_items,
                        key.to_vec(),
                        element,
                    ));
                }
                Ok(!has_raw)
            }
            PathKeyElementSize((key_info_path, key_info, element)) => {
                match apply_type {
                    BatchInsertApplyType::StatelessBatchInsert {
                        in_tree_using_sums, ..
                    } => {
                        // we can estimate that the element was the same size
                        drive_operations.push(CalculatedCostOperation(
                            GroveDb::average_case_for_has_raw(
                                &key_info_path,
                                &key_info,
                                element.serialized_size() as u32,
                                in_tree_using_sums,
                            ),
                        ));
                        drive_operations.push(
                            DriveOperation::insert_for_estimated_path_key_element(
                                key_info_path,
                                key_info,
                                element,
                            ),
                        );
                        Ok(true)
                    }
                    BatchInsertApplyType::StatefulBatchInsert => {
                        Err(Error::Drive(DriveError::NotSupportedPrivate(
                            "document sizes for stateful insert in batch operations not supported",
                        )))
                    }
                }
            }
            PathKeyUnknownElementSize(_) => Err(Error::Drive(DriveError::NotSupportedPrivate(
                "document sizes in batch operations not supported",
            ))),
        }
    }

    /// Pushes a "delete element" operation to `drive_operations`.
    pub(crate) fn batch_delete<'a, 'c, P>(
        &'a self,
        path: P,
        key: &'c [u8],
        apply_type: BatchDeleteApplyType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error>
    where
        P: IntoIterator<Item = &'c [u8]>,
        <P as IntoIterator>::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
    {
        let current_batch_operations = DriveOperation::grovedb_operations_batch(drive_operations);
        let options = DeleteOptions {
            allow_deleting_non_empty_trees: false,
            deleting_non_empty_trees_returns_error: true,
            base_root_storage_is_free: true,
        };
        let delete_operation = match apply_type {
            BatchDeleteApplyType::StatelessBatchDelete {
                is_sum_tree,
                estimated_value_size,
            } => GroveDb::worst_case_delete_operation_for_delete_internal::<RocksDbStorage>(
                &KeyInfoPath::from_known_path(path),
                &KeyInfo::KnownKey(key.to_vec()),
                is_sum_tree,
                true,
                true,
                0,
                estimated_value_size,
            )
            .map(|r| r.map(Some)),
            BatchDeleteApplyType::StatefulBatchDelete {
                is_known_to_be_subtree_with_sum,
            } => self.grove.delete_operation_for_delete_internal(
                path,
                key,
                &options,
                true,
                is_known_to_be_subtree_with_sum,
                &current_batch_operations.operations,
                transaction,
            ),
        };

        if let Some(delete_operation) =
            push_drive_operation_result(delete_operation, drive_operations)?
        {
            // we also add the actual delete operation
            drive_operations.push(DriveOperation::GroveOperation(delete_operation))
        }

        Ok(())
    }

    /// Pushes a "delete up tree while empty" operation to `drive_operations`.
    pub(crate) fn batch_delete_up_tree_while_empty<'a, 'c>(
        &'a self,
        path: KeyInfoPath,
        key: &'c [u8],
        stop_path_height: Option<u16>,
        apply_type: BatchDeleteUpTreeApplyType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error> {
        let current_batch_operations = DriveOperation::grovedb_operations_batch(drive_operations);
        let cost_context = match apply_type {
            BatchDeleteUpTreeApplyType::StatelessBatchDelete {
                estimated_layer_info,
            } => GroveDb::average_case_delete_operations_for_delete_up_tree_while_empty::<
                RocksDbStorage,
            >(
                &path,
                &KeyInfo::KnownKey(key.to_vec()),
                stop_path_height,
                true,
                estimated_layer_info,
            ),
            BatchDeleteUpTreeApplyType::StatefulBatchDelete {
                is_known_to_be_subtree_with_sum,
            } => {
                let options = DeleteOptions {
                    allow_deleting_non_empty_trees: false,
                    deleting_non_empty_trees_returns_error: true,
                    base_root_storage_is_free: true,
                };
                self.grove.delete_operations_for_delete_up_tree_while_empty(
                    path.to_path_refs(),
                    key,
                    stop_path_height,
                    &options,
                    true,
                    is_known_to_be_subtree_with_sum,
                    current_batch_operations.operations,
                    transaction,
                )
            }
        };
        let delete_operations = push_drive_operation_result(cost_context, drive_operations)?;
        delete_operations
            .into_iter()
            .for_each(|op| drive_operations.push(DriveOperation::GroveOperation(op)));

        Ok(())
    }

    /// Applies the given groveDB operation
    pub fn grove_apply_operation(
        &self,
        operation: GroveDbOp,
        validate: bool,
        transaction: TransactionArg,
    ) -> Result<(), Error> {
        self.grove_apply_batch_with_add_costs(
            GroveDbOpBatch {
                operations: vec![operation],
            },
            validate,
            transaction,
            &mut vec![],
        )
    }

    /// Applies the given groveDB operations batch.
    pub fn grove_apply_batch(
        &self,
        ops: GroveDbOpBatch,
        validate: bool,
        transaction: TransactionArg,
    ) -> Result<(), Error> {
        self.grove_apply_batch_with_add_costs(ops, validate, transaction, &mut vec![])
    }

    /// Applies the given groveDB operations batch and gets and passes the costs to `push_drive_operation_result`.
    pub(crate) fn grove_apply_batch_with_add_costs(
        &self,
        ops: GroveDbOpBatch,
        validate: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error> {
        if ops.is_empty() {
            return Err(Error::Drive(DriveError::BatchIsEmpty()));
        }
        if self.config.batching_enabled {
            //println!("batch {:#?}", ops);
            if self.config.batching_consistency_verification {
                let consistency_results =
                    GroveDbOp::verify_consistency_of_operations(&ops.operations);
                if !consistency_results.is_empty() {
                    println!("results {:#?}", consistency_results);
                    return Err(Error::Drive(DriveError::GroveDBInsertion(
                        "insertion order error",
                    )));
                }
            }

            let cost_context = self.grove.apply_batch_with_element_flags_update(
                ops.operations,
                Some(BatchApplyOptions {
                    validate_insertion_does_not_override: validate,
                    validate_insertion_does_not_override_tree: validate,
                    allow_deleting_non_empty_trees: false,
                    deleting_non_empty_trees_returns_error: true,
                    disable_operation_consistency_check: false,
                    base_root_storage_is_free: true
                }),
                |cost, old_flags, new_flags| {

                    // if there were no flags before then the new flags are used
                    if old_flags.is_none() {
                        return Ok(false);
                    }
                    // This could be none only because the old element didn't exist
                    // If they were empty we get an error
                    let maybe_old_storage_flags =
                        StorageFlags::from_some_element_flags_ref(&old_flags).map_err(|_| GroveError::JustInTimeElementFlagsClientError("drive did not understand flags of old item being updated"))?;
                    let new_storage_flags = StorageFlags::from_element_flags_ref(new_flags).map_err(|_| GroveError::JustInTimeElementFlagsClientError("drive did not understand updated item flag information"))?.ok_or(GroveError::JustInTimeElementFlagsClientError("removing flags from an item with flags is not allowed"))?;
                    match &cost.transition_type() {
                        OperationStorageTransitionType::OperationUpdateBiggerSize => {
                            let combined_storage_flags =
                                StorageFlags::optional_combine_added_bytes(
                                    maybe_old_storage_flags,
                                    new_storage_flags,
                                    cost.added_bytes,
                                ).map_err(|_| GroveError::JustInTimeElementFlagsClientError("drive could not combine storage flags (new flags were bigger)"))?;
                            let combined_flags = combined_storage_flags.to_element_flags();
                            // it's possible they got bigger in the same epoch
                            if combined_flags == *new_flags {
                                // they are the same there was no update
                                Ok(false)
                            } else {
                                *new_flags = combined_flags;
                                Ok(true)
                            }
                        }
                        OperationStorageTransitionType::OperationUpdateSmallerSize => {
                            let combined_storage_flags =
                                StorageFlags::optional_combine_removed_bytes(
                                    maybe_old_storage_flags,
                                    new_storage_flags,
                                    &cost.removed_bytes,
                                ).map_err(|_| GroveError::JustInTimeElementFlagsClientError("drive could not combine storage flags (new flags were smaller)"))?;
                            let combined_flags = combined_storage_flags.to_element_flags();
                            // it's possible they got bigger in the same epoch
                            if combined_flags == *new_flags {
                                // they are the same there was no update
                                Ok(false)
                            } else {
                                *new_flags = combined_flags;
                                Ok(true)
                            }
                        }
                        _ => Ok(false),
                    }
                },
                |flags, removed_key_bytes, removed_value_bytes| {
                    let maybe_storage_flags = StorageFlags::from_element_flags_ref(flags).map_err(|_| GroveError::SplitRemovalBytesClientError("drive did not understand flags of item being updated"))?;
                    // if there were no flags before then the new flags are used
                    match maybe_storage_flags {
                        None => { Ok((BasicStorageRemoval(removed_key_bytes), BasicStorageRemoval(removed_value_bytes)))}
                        Some(storage_flags) => {
                            storage_flags.split_storage_removed_bytes(removed_key_bytes, removed_value_bytes)
                        }
                    }

                },
                transaction,
            );
            push_drive_operation_result(cost_context, drive_operations)
        } else {
            let options = if validate {
                Some(InsertOptions {
                    validate_insertion_does_not_override: false,
                    validate_insertion_does_not_override_tree: true,
                    base_root_storage_is_free: true,
                })
            } else {
                None
            };
            //println!("changes {} {:#?}", ops.len(), ops);
            for operation in ops.operations.into_iter() {
                //println!("on {:#?}", op);
                let GroveDbOp { path, key, op } = operation;
                match op {
                    Op::Insert { element } => self.grove_insert(
                        path.to_path_refs(),
                        key.as_slice(),
                        element,
                        transaction,
                        options.clone(),
                        drive_operations,
                    )?,
                    Op::Delete | Op::DeleteTree => self.grove_delete(
                        path.to_path_refs(),
                        key.as_slice(),
                        transaction,
                        drive_operations,
                    )?,
                    _ => {
                        return Err(Error::Drive(DriveError::NotSupportedPrivate(
                            "Only Insert and Deletion operations are allowed",
                        )))
                    }
                }
            }
            Ok(())
        }
    }

    /// Gets the costs for the given groveDB op batch and passes them to `push_drive_operation_result`.
    pub(crate) fn grove_batch_operations_costs(
        &self,
        ops: GroveDbOpBatch,
        estimated_layer_info: HashMap<KeyInfoPath, EstimatedLayerInformation>,
        validate: bool,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error> {
        let cost_context = GroveDb::estimated_case_operations_for_batch(
            AverageCaseCostsType(estimated_layer_info),
            ops.operations,
            Some(BatchApplyOptions {
                validate_insertion_does_not_override: validate,
                validate_insertion_does_not_override_tree: validate,
                allow_deleting_non_empty_trees: false,
                deleting_non_empty_trees_returns_error: true,
                disable_operation_consistency_check: false,
                base_root_storage_is_free: true,
            }),
            |_, _, _| Ok(false),
            |_, _, _| Err(GroveError::InternalError("not implemented")),
        );
        push_drive_operation_result(cost_context, drive_operations)
    }
}
