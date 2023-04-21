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
use costs::{CostContext, OperationCost};
use grovedb::batch::estimated_costs::EstimatedCostsType::AverageCaseCostsType;
use grovedb::batch::{
    key_info::KeyInfo, BatchApplyOptions, GroveDbOp, KeyInfoPath, Op, OpsByLevelPath,
};
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
use crate::fee::op::LowLevelDriveOperation;
use crate::fee::op::LowLevelDriveOperation::{CalculatedCostOperation, GroveOperation};
use grovedb::operations::delete::{DeleteOptions, DeleteUpTreeOptions};
use grovedb::operations::insert::InsertOptions;
use grovedb::query_result_type::{
    PathKeyOptionalElementTrio, QueryResultElements, QueryResultType,
};
use grovedb::Error as GroveError;
use integer_encoding::VarInt;

use intmap::IntMap;
use storage::rocksdb_storage::RocksDbStorage;

/// Pushes an operation's `OperationCost` to `drive_operations` given its `CostContext`
/// and returns the operation's return value.
fn push_drive_operation_result<T>(
    cost_context: CostContext<Result<T, GroveError>>,
    drive_operations: &mut Vec<LowLevelDriveOperation>,
) -> Result<T, Error> {
    let CostContext { value, cost } = cost_context;
    drive_operations.push(CalculatedCostOperation(cost));
    value.map_err(Error::GroveDB)
}

/// Pushes an operation's `OperationCost` to `drive_operations` given its `CostContext`
/// if `drive_operations` is given. Returns the operation's return value.
fn push_drive_operation_result_optional<T>(
    cost_context: CostContext<Result<T, GroveError>>,
    drive_operations: Option<&mut Vec<LowLevelDriveOperation>>,
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
    StatelessBatchInsertTree {
        in_tree_using_sums: bool,
        is_sum_tree: bool,
        flags_len: FlagsLen,
    },
    StatefulBatchInsertTree,
}

impl BatchInsertTreeApplyType {
    pub(crate) fn to_direct_query_type(&self) -> DirectQueryType {
        match self {
            BatchInsertTreeApplyType::StatelessBatchInsertTree {
                in_tree_using_sums,
                is_sum_tree,
                flags_len,
            } => DirectQueryType::StatelessDirectQuery {
                in_tree_using_sums: *in_tree_using_sums,
                query_target: QueryTarget::QueryTargetTree(*flags_len, *is_sum_tree),
            },
            BatchInsertTreeApplyType::StatefulBatchInsertTree => {
                DirectQueryType::StatefulDirectQuery
            }
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
    pub(crate) fn into_query_type(self) -> QueryType {
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

impl From<BatchDeleteApplyType> for QueryType {
    fn from(value: BatchDeleteApplyType) -> Self {
        match value {
            BatchDeleteApplyType::StatelessBatchDelete {
                is_sum_tree,
                estimated_value_size,
            } => QueryType::StatelessQuery {
                in_tree_using_sums: is_sum_tree,
                query_target: QueryTarget::QueryTargetValue(estimated_value_size),
                estimated_reference_sizes: vec![],
            },
            BatchDeleteApplyType::StatefulBatchDelete { .. } => QueryType::StatefulQuery,
        }
    }
}

impl From<&BatchDeleteApplyType> for QueryType {
    fn from(value: &BatchDeleteApplyType) -> Self {
        match value {
            BatchDeleteApplyType::StatelessBatchDelete {
                is_sum_tree,
                estimated_value_size,
            } => QueryType::StatelessQuery {
                in_tree_using_sums: *is_sum_tree,
                query_target: QueryTarget::QueryTargetValue(*estimated_value_size),
                estimated_reference_sizes: vec![],
            },
            BatchDeleteApplyType::StatefulBatchDelete { .. } => QueryType::StatefulQuery,
        }
    }
}

impl From<BatchDeleteApplyType> for DirectQueryType {
    fn from(value: BatchDeleteApplyType) -> Self {
        match value {
            BatchDeleteApplyType::StatelessBatchDelete {
                is_sum_tree,
                estimated_value_size,
            } => DirectQueryType::StatelessDirectQuery {
                in_tree_using_sums: is_sum_tree,
                query_target: QueryTarget::QueryTargetValue(estimated_value_size),
            },
            BatchDeleteApplyType::StatefulBatchDelete { .. } => {
                DirectQueryType::StatefulDirectQuery
            }
        }
    }
}

impl From<&BatchDeleteApplyType> for DirectQueryType {
    fn from(value: &BatchDeleteApplyType) -> Self {
        match value {
            BatchDeleteApplyType::StatelessBatchDelete {
                is_sum_tree,
                estimated_value_size,
            } => DirectQueryType::StatelessDirectQuery {
                in_tree_using_sums: *is_sum_tree,
                query_target: QueryTarget::QueryTargetValue(*estimated_value_size),
            },
            BatchDeleteApplyType::StatefulBatchDelete { .. } => {
                DirectQueryType::StatefulDirectQuery
            }
        }
    }
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
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<(), Error>
    where
        P: IntoIterator<Item = &'p [u8]>,
        <P as IntoIterator>::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
    {
        let cost_context = self.grove.insert(path, key, element, options, transaction);
        push_drive_operation_result(cost_context, drive_operations)
    }

    /// Pushes the `OperationCost` of inserting an empty tree in groveDB to `drive_operations`.
    pub fn grove_insert_empty_tree<'p, P>(
        &self,
        path: P,
        key: &'p [u8],
        transaction: TransactionArg,
        options: Option<InsertOptions>,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<(), Error>
    where
        P: IntoIterator<Item = &'p [u8]>,
        <P as IntoIterator>::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
    {
        let cost_context =
            self.grove
                .insert(path, key, Element::empty_tree(), options, transaction);
        push_drive_operation_result(cost_context, drive_operations)
    }

    /// Pushes the `OperationCost` of inserting an empty sum tree in groveDB to `drive_operations`.
    pub fn grove_insert_empty_sum_tree<'p, P>(
        &self,
        path: P,
        key: &'p [u8],
        transaction: TransactionArg,
        options: Option<InsertOptions>,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<(), Error>
    where
        P: IntoIterator<Item = &'p [u8]>,
        <P as IntoIterator>::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
    {
        let cost_context =
            self.grove
                .insert(path, key, Element::empty_sum_tree(), options, transaction);
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
        drive_operations: Option<&mut Vec<LowLevelDriveOperation>>,
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
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<(), Error>
    where
        P: IntoIterator<Item = &'p [u8]>,
        <P as IntoIterator>::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
    {
        let options = DeleteOptions {
            allow_deleting_non_empty_trees: false,
            deleting_non_empty_trees_returns_error: true,
            base_root_storage_is_free: true,
            validate_tree_at_path_exists: false,
        };
        let cost_context = self.grove.delete(path, key, Some(options), transaction);
        push_drive_operation_result(cost_context, drive_operations)
    }

    /// grove_get_raw basically means that there are no reference hops, this only matters
    /// when calculating worst case costs
    pub fn grove_get_raw<'p, P>(
        &self,
        path: P,
        key: &'p [u8],
        direct_query_type: DirectQueryType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<Option<Element>, Error>
    where
        P: IntoIterator<Item = &'p [u8]>,
        <P as IntoIterator>::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
    {
        let path_iter = path.into_iter();
        match direct_query_type {
            DirectQueryType::StatelessDirectQuery {
                in_tree_using_sums,
                query_target,
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
                        GroveDb::average_case_for_get_raw(
                            &key_info_path,
                            &key_info,
                            estimated_value_size,
                            in_tree_using_sums,
                        )
                    }
                };

                drive_operations.push(CalculatedCostOperation(cost));
                Ok(None)
            }
            DirectQueryType::StatefulDirectQuery => {
                let CostContext { value, cost } = self.grove.get_raw(path_iter, key, transaction);
                drive_operations.push(CalculatedCostOperation(cost));
                Ok(Some(value.map_err(Error::GroveDB)?))
            }
        }
    }

    /// grove_get_raw basically means that there are no reference hops, this only matters
    /// when calculating worst case costs
    pub fn grove_get_raw_optional<'p, P>(
        &self,
        path: P,
        key: &'p [u8],
        direct_query_type: DirectQueryType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<Option<Element>, Error>
    where
        P: IntoIterator<Item = &'p [u8]>,
        <P as IntoIterator>::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
    {
        let path_iter = path.into_iter();
        match direct_query_type {
            DirectQueryType::StatelessDirectQuery {
                in_tree_using_sums,
                query_target,
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
                        GroveDb::average_case_for_get_raw(
                            &key_info_path,
                            &key_info,
                            estimated_value_size,
                            in_tree_using_sums,
                        )
                    }
                };

                drive_operations.push(CalculatedCostOperation(cost));
                Ok(None)
            }
            DirectQueryType::StatefulDirectQuery => {
                let CostContext { value, cost } =
                    self.grove.get_raw_optional(path_iter, key, transaction);
                drive_operations.push(CalculatedCostOperation(cost));
                Ok(value.map_err(Error::GroveDB)?)
            }
        }
    }

    /// grove_get_direct_u64 is a helper function to get a
    pub fn grove_get_raw_value_u64_from_encoded_var_vec<'p, P>(
        &self,
        path: P,
        key: &'p [u8],
        direct_query_type: DirectQueryType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<Option<u64>, Error>
    where
        P: IntoIterator<Item = &'p [u8]>,
        <P as IntoIterator>::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
    {
        let element = self.grove_get_raw_optional(
            path,
            key,
            direct_query_type,
            transaction,
            drive_operations,
        )?;
        element
            .map(|element| match element {
                Element::Item(value, ..) => u64::decode_var(value.as_slice())
                    .ok_or(Error::Drive(DriveError::CorruptedElementType(
                        "encoded value could not be decoded",
                    )))
                    .map(|(value, _)| value),
                Element::SumItem(value, ..) => Ok(value as u64),
                _ => Err(Error::Drive(DriveError::CorruptedQueryReturnedNonItem(
                    "expected an item",
                ))),
            })
            .transpose()
    }

    /// Gets the element at the given path from groveDB.
    /// Pushes the `OperationCost` of getting the element to `drive_operations`.
    pub fn grove_get<'p, P>(
        &self,
        path: P,
        key: &'p [u8],
        query_type: QueryType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<Option<Element>, Error>
    where
        P: IntoIterator<Item = &'p [u8]>,
        <P as IntoIterator>::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
    {
        let path_iter = path.into_iter();
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
                            estimated_value_size,
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
    pub(crate) fn grove_get_path_query_serialized_results(
        &self,
        path_query: &PathQuery,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<(Vec<Vec<u8>>, u16), Error> {
        let CostContext { value, cost } =
            self.grove
                .query_item_value(path_query, transaction.is_some(), transaction);
        drive_operations.push(CalculatedCostOperation(cost));
        value.map_err(Error::GroveDB)
    }

    /// Gets the return value and the cost of a groveDB path query.
    /// Pushes the cost to `drive_operations` and returns the return value.
    pub(crate) fn grove_get_path_query(
        &self,
        path_query: &PathQuery,
        transaction: TransactionArg,
        result_type: QueryResultType,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<(QueryResultElements, u16), Error> {
        let CostContext { value, cost } =
            self.grove
                .query(path_query, transaction.is_some(), result_type, transaction);
        drive_operations.push(CalculatedCostOperation(cost));
        value.map_err(Error::GroveDB)
    }

    /// Gets the return value and the cost of a groveDB path query.
    /// Pushes the cost to `drive_operations` and returns the return value.
    pub(crate) fn grove_get_path_query_with_optional(
        &self,
        path_query: &PathQuery,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<Vec<PathKeyOptionalElementTrio>, Error> {
        let CostContext { value, cost } =
            self.grove
                .query_keys_optional(path_query, true, transaction);
        drive_operations.push(CalculatedCostOperation(cost));
        value.map_err(Error::GroveDB)
    }

    /// Gets the return value and the cost of a groveDB path query.
    /// Pushes the cost to `drive_operations` and returns the return value.
    pub(crate) fn grove_get_raw_path_query_with_optional(
        &self,
        path_query: &PathQuery,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<Vec<PathKeyOptionalElementTrio>, Error> {
        let CostContext { value, cost } =
            self.grove
                .query_raw_keys_optional(path_query, true, transaction);
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
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<(QueryResultElements, u16), Error> {
        let CostContext { value, cost } =
            self.grove
                .query_raw(path_query, transaction.is_some(), result_type, transaction);
        drive_operations.push(CalculatedCostOperation(cost));
        value.map_err(Error::GroveDB)
    }

    /// Gets the return value and the cost of a groveDB proved path query.
    /// Pushes the cost to `drive_operations` and returns the return value.
    /// Verbose should be generally set to false unless one needs to prove
    /// subsets of a proof.
    pub(crate) fn grove_get_proved_path_query(
        &self,
        path_query: &PathQuery,
        verbose: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<Vec<u8>, Error> {
        let CostContext { value, cost } =
            self.grove
                .get_proved_path_query(path_query, verbose, transaction);
        drive_operations.push(CalculatedCostOperation(cost));
        value.map_err(Error::GroveDB)
    }

    /// Gets the element at the given path from groveDB.
    /// Pushes the `OperationCost` of getting the element to `drive_operations`.
    pub fn grove_get_sum_tree_total_value<'p, P>(
        &self,
        path: P,
        key: &'p [u8],
        query_type: DirectQueryType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<i64, Error>
    where
        P: IntoIterator<Item = &'p [u8]>,
        <P as IntoIterator>::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
    {
        let path_iter = path.into_iter();
        match query_type {
            DirectQueryType::StatelessDirectQuery {
                in_tree_using_sums,
                query_target,
            } => {
                let key_info_path = KeyInfoPath::from_known_path(path_iter);
                let key_info = KeyInfo::KnownKey(key.to_vec());
                let cost = match query_target {
                    QueryTarget::QueryTargetTree(flags_size, is_sum_tree) => {
                        Ok(GroveDb::average_case_for_get_tree(
                            &key_info_path,
                            &key_info,
                            flags_size,
                            is_sum_tree,
                            in_tree_using_sums,
                        ))
                    }
                    _ => Err(Error::Drive(DriveError::CorruptedCodeExecution(
                        "can not query a non tree",
                    ))),
                }?;

                drive_operations.push(CalculatedCostOperation(cost));
                Ok(0)
            }
            DirectQueryType::StatefulDirectQuery => {
                let CostContext { value, cost } = self.grove.get_raw(path_iter, key, transaction);
                drive_operations.push(CalculatedCostOperation(cost));
                let element = value.map_err(Error::GroveDB)?;
                match element {
                    Element::SumTree(_, value, _) => Ok(value),
                    _ => Err(Error::Drive(DriveError::CorruptedBalancePath(
                        "balance path does not refer to a sum tree",
                    ))),
                }
            }
        }
    }

    /// Gets the return value and the cost of a groveDB `has_raw` operation.
    /// Pushes the cost to `drive_operations` and returns the return value.
    pub(crate) fn grove_has_raw<'p, P>(
        &self,
        path: P,
        key: &'p [u8],
        query_type: DirectQueryType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
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
                            estimated_value_size,
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
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<(), Error>
    where
        P: IntoIterator<Item = &'c [u8]>,
        <P as IntoIterator>::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
    {
        match key_info {
            KeyRef(key) => {
                let path_items: Vec<Vec<u8>> = path.into_iter().map(Vec::from).collect();
                drive_operations.push(LowLevelDriveOperation::for_known_path_key_empty_tree(
                    path_items,
                    key.to_vec(),
                    storage_flags,
                ));
                Ok(())
            }
            KeySize(key) => {
                drive_operations.push(LowLevelDriveOperation::for_estimated_path_key_empty_tree(
                    KeyInfoPath::from_known_path(path),
                    key,
                    storage_flags,
                ));
                Ok(())
            }
            Key(key) => {
                let path_items: Vec<Vec<u8>> = path.into_iter().map(Vec::from).collect();
                drive_operations.push(LowLevelDriveOperation::for_known_path_key_empty_tree(
                    path_items,
                    key,
                    storage_flags,
                ));
                Ok(())
            }
        }
    }

    /// Pushes an "insert empty tree where path key does not yet exist" operation to `drive_operations`.
    /// Will also check the current drive operations
    pub(crate) fn batch_insert_empty_tree_if_not_exists<const N: usize>(
        &self,
        path_key_info: PathKeyInfo<N>,
        storage_flags: Option<&StorageFlags>,
        apply_type: BatchInsertTreeApplyType,
        transaction: TransactionArg,
        check_existing_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<bool, Error> {
        //todo: clean up the duplication
        match path_key_info {
            PathKeyRef((path, key)) => {
                let path_iter: Vec<&[u8]> = path.iter().map(|x| x.as_slice()).collect();
                let drive_operation = LowLevelDriveOperation::for_known_path_key_empty_tree(
                    path.clone(),
                    key.to_vec(),
                    storage_flags,
                );
                // we only add the operation if it doesn't already exist in the current batch
                if let Some(existing_operations) = check_existing_operations {
                    let mut i = 0;
                    let mut found = false;
                    while i < existing_operations.len() {
                        // we need to check every drive operation
                        // if it already exists then just ignore things
                        // if we had a delete then we need to remove the delete
                        let previous_drive_operation = &existing_operations[i];
                        if previous_drive_operation == &drive_operation {
                            found = true;
                            break;
                        } else if let GroveOperation(grove_op) = previous_drive_operation {
                            if grove_op.key == key
                                && grove_op.path == path
                                && matches!(grove_op.op, Op::DeleteTree)
                            {
                                found = true;
                                existing_operations.remove(i);
                                break;
                            } else {
                                i += 1;
                            }
                        } else {
                            i += 1;
                        }
                    }
                    if !found {
                        let has_raw = self.grove_has_raw(
                            path_iter.clone(),
                            key,
                            apply_type.to_direct_query_type(),
                            transaction,
                            drive_operations,
                        )?;
                        if !has_raw {
                            drive_operations.push(drive_operation);
                        }
                        Ok(!has_raw)
                    } else {
                        Ok(false)
                    }
                } else {
                    let has_raw = self.grove_has_raw(
                        path_iter.clone(),
                        key,
                        apply_type.to_direct_query_type(),
                        transaction,
                        drive_operations,
                    )?;
                    if !has_raw {
                        drive_operations.push(drive_operation);
                    }
                    Ok(!has_raw)
                }
            }
            PathKeySize(_key_path_info, _key_info) => Err(Error::Drive(
                DriveError::NotSupportedPrivate("document sizes in batch operations not supported"),
            )),
            PathKey((path, key)) => {
                let path_iter: Vec<&[u8]> = path.iter().map(|x| x.as_slice()).collect();
                let drive_operation = LowLevelDriveOperation::for_known_path_key_empty_tree(
                    path.clone(),
                    key.to_vec(),
                    storage_flags,
                );
                // we only add the operation if it doesn't already exist in the current batch
                if let Some(existing_operations) = check_existing_operations {
                    let mut i = 0;
                    let mut found = false;
                    while i < existing_operations.len() {
                        // we need to check every drive operation
                        // if it already exists then just ignore things
                        // if we had a delete then we need to remove the delete
                        let previous_drive_operation = &existing_operations[i];
                        if previous_drive_operation == &drive_operation {
                            found = true;
                            break;
                        } else if let GroveOperation(grove_op) = previous_drive_operation {
                            if grove_op.key == key
                                && grove_op.path == path_iter
                                && matches!(grove_op.op, Op::DeleteTree)
                            {
                                found = true;
                                existing_operations.remove(i);
                                break;
                            } else {
                                i += 1;
                            }
                        } else {
                            i += 1;
                        }
                    }
                    if !found {
                        let has_raw = self.grove_has_raw(
                            path_iter.clone(),
                            key.as_slice(),
                            apply_type.to_direct_query_type(),
                            transaction,
                            drive_operations,
                        )?;
                        if !has_raw {
                            drive_operations.push(drive_operation);
                        }
                        Ok(!has_raw)
                    } else {
                        Ok(false)
                    }
                } else {
                    let has_raw = self.grove_has_raw(
                        path_iter.clone(),
                        key.as_slice(),
                        apply_type.to_direct_query_type(),
                        transaction,
                        drive_operations,
                    )?;
                    if !has_raw {
                        drive_operations.push(drive_operation);
                    }
                    Ok(!has_raw)
                }
            }
            PathFixedSizeKey((path, key)) => {
                let path_items: Vec<Vec<u8>> = path.into_iter().map(Vec::from).collect();
                let drive_operation = LowLevelDriveOperation::for_known_path_key_empty_tree(
                    path_items,
                    key.to_vec(),
                    storage_flags,
                );
                // we only add the operation if it doesn't already exist in the current batch
                if let Some(existing_operations) = check_existing_operations {
                    let mut i = 0;
                    let mut found = false;
                    while i < existing_operations.len() {
                        // we need to check every drive operation
                        // if it already exists then just ignore things
                        // if we had a delete then we need to remove the delete
                        let previous_drive_operation = &existing_operations[i];
                        if previous_drive_operation == &drive_operation {
                            found = true;
                            break;
                        } else if let GroveOperation(grove_op) = previous_drive_operation {
                            if grove_op.key == key
                                && grove_op.path == path
                                && matches!(grove_op.op, Op::DeleteTree)
                            {
                                found = true;
                                existing_operations.remove(i);
                                break;
                            } else {
                                i += 1;
                            }
                        } else {
                            i += 1;
                        }
                    }
                    if !found {
                        let has_raw = self.grove_has_raw(
                            path,
                            key.as_slice(),
                            apply_type.to_direct_query_type(),
                            transaction,
                            drive_operations,
                        )?;
                        if !has_raw {
                            drive_operations.push(drive_operation);
                        }
                        Ok(!has_raw)
                    } else {
                        Ok(false)
                    }
                } else {
                    let has_raw = self.grove_has_raw(
                        path,
                        key.as_slice(),
                        apply_type.to_direct_query_type(),
                        transaction,
                        drive_operations,
                    )?;
                    if !has_raw {
                        drive_operations.push(drive_operation);
                    }
                    Ok(!has_raw)
                }
            }
            PathFixedSizeKeyRef((path, key)) => {
                let path_items: Vec<Vec<u8>> = path.into_iter().map(Vec::from).collect();
                let drive_operation = LowLevelDriveOperation::for_known_path_key_empty_tree(
                    path_items,
                    key.to_vec(),
                    storage_flags,
                );
                // we only add the operation if it doesn't already exist in the current batch
                if let Some(existing_operations) = check_existing_operations {
                    let mut i = 0;
                    let mut found = false;
                    while i < existing_operations.len() {
                        // we need to check every drive operation
                        // if it already exists then just ignore things
                        // if we had a delete then we need to remove the delete
                        let previous_drive_operation = &existing_operations[i];
                        if previous_drive_operation == &drive_operation {
                            found = true;
                            break;
                        } else if let GroveOperation(grove_op) = previous_drive_operation {
                            if grove_op.key == key
                                && grove_op.path == path
                                && matches!(grove_op.op, Op::DeleteTree)
                            {
                                found = true;
                                existing_operations.remove(i);
                                break;
                            } else {
                                i += 1;
                            }
                        } else {
                            i += 1;
                        }
                    }
                    if !found {
                        let has_raw = self.grove_has_raw(
                            path,
                            key,
                            apply_type.to_direct_query_type(),
                            transaction,
                            drive_operations,
                        )?;
                        if !has_raw {
                            drive_operations.push(drive_operation);
                        }
                        Ok(!has_raw)
                    } else {
                        Ok(false)
                    }
                } else {
                    let has_raw = self.grove_has_raw(
                        path,
                        key,
                        apply_type.to_direct_query_type(),
                        transaction,
                        drive_operations,
                    )?;
                    if !has_raw {
                        drive_operations.push(drive_operation);
                    }
                    Ok(!has_raw)
                }
            }
        }
    }

    /// Pushes an "insert empty tree where path key does not yet exist" operation to `drive_operations`.
    /// Will also check the current drive operations
    pub(crate) fn batch_insert_empty_tree_if_not_exists_check_existing_operations<
        const N: usize,
    >(
        &self,
        path_key_info: PathKeyInfo<N>,
        storage_flags: Option<&StorageFlags>,
        apply_type: BatchInsertTreeApplyType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<bool, Error> {
        match path_key_info {
            PathKeyRef((path, key)) => {
                let path_iter: Vec<&[u8]> = path.iter().map(|x| x.as_slice()).collect();
                let drive_operation = LowLevelDriveOperation::for_known_path_key_empty_tree(
                    path.clone(),
                    key.to_vec(),
                    storage_flags,
                );
                // we only add the operation if it doesn't already exist in the current batch
                if !drive_operations.contains(&drive_operation) {
                    let has_raw = self.grove_has_raw(
                        path_iter.clone(),
                        key,
                        apply_type.to_direct_query_type(),
                        transaction,
                        drive_operations,
                    )?;
                    if !has_raw {
                        drive_operations.push(drive_operation);
                    }
                    Ok(!has_raw)
                } else {
                    Ok(false)
                }
            }
            PathKeySize(_key_path_info, _key_info) => Err(Error::Drive(
                DriveError::NotSupportedPrivate("document sizes in batch operations not supported"),
            )),
            PathKey((path, key)) => {
                let drive_operation = LowLevelDriveOperation::for_known_path_key_empty_tree(
                    path.clone(),
                    key.clone(),
                    storage_flags,
                );
                // we only add the operation if it doesn't already exist in the current batch
                if !drive_operations.contains(&drive_operation) {
                    let path_iter: Vec<&[u8]> = path.iter().map(|x| x.as_slice()).collect();
                    let has_raw = self.grove_has_raw(
                        path_iter.clone(),
                        key.as_slice(),
                        apply_type.to_direct_query_type(),
                        transaction,
                        drive_operations,
                    )?;
                    if !has_raw {
                        drive_operations.push(drive_operation);
                    }
                    Ok(!has_raw)
                } else {
                    Ok(false)
                }
            }
            PathFixedSizeKey((path, key)) => {
                let path_items: Vec<Vec<u8>> = path.into_iter().map(Vec::from).collect();
                let drive_operation = LowLevelDriveOperation::for_known_path_key_empty_tree(
                    path_items,
                    key.to_vec(),
                    storage_flags,
                );
                // we only add the operation if it doesn't already exist in the current batch
                if !drive_operations.contains(&drive_operation) {
                    let has_raw = self.grove_has_raw(
                        path,
                        key.as_slice(),
                        apply_type.to_direct_query_type(),
                        transaction,
                        drive_operations,
                    )?;
                    if !has_raw {
                        drive_operations.push(drive_operation);
                    }
                    Ok(!has_raw)
                } else {
                    Ok(false)
                }
            }
            PathFixedSizeKeyRef((path, key)) => {
                let path_items: Vec<Vec<u8>> = path.into_iter().map(Vec::from).collect();
                let drive_operation = LowLevelDriveOperation::for_known_path_key_empty_tree(
                    path_items,
                    key.to_vec(),
                    storage_flags,
                );
                // we only add the operation if it doesn't already exist in the current batch
                if !drive_operations.contains(&drive_operation) {
                    let has_raw = self.grove_has_raw(
                        path,
                        key,
                        apply_type.to_direct_query_type(),
                        transaction,
                        drive_operations,
                    )?;
                    if !has_raw {
                        drive_operations.push(drive_operation);
                    }
                    Ok(!has_raw)
                } else {
                    Ok(false)
                }
            }
        }
    }

    /// Pushes an "insert element" operation to `drive_operations`.
    pub(crate) fn batch_insert<const N: usize>(
        &self,
        path_key_element_info: PathKeyElementInfo<N>,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<(), Error> {
        match path_key_element_info {
            PathKeyRefElement((path, key, element)) => {
                drive_operations.push(LowLevelDriveOperation::insert_for_known_path_key_element(
                    path,
                    key.to_vec(),
                    element,
                ));
                Ok(())
            }
            PathKeyElement((path, key, element)) => {
                drive_operations.push(LowLevelDriveOperation::insert_for_known_path_key_element(
                    path, key, element,
                ));
                Ok(())
            }
            PathKeyElementSize((key_info_path, key_info, element)) => {
                drive_operations.push(
                    LowLevelDriveOperation::insert_for_estimated_path_key_element(
                        key_info_path,
                        key_info,
                        element,
                    ),
                );
                Ok(())
            }
            PathKeyUnknownElementSize(_) => Err(Error::Drive(DriveError::NotSupportedPrivate(
                "inserting unsized documents into a batch is not currently supported",
            ))),
            PathFixedSizeKeyRefElement((path, key, element)) => {
                let path_items: Vec<Vec<u8>> = path.into_iter().map(Vec::from).collect();
                drive_operations.push(LowLevelDriveOperation::insert_for_known_path_key_element(
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
    pub(crate) fn batch_insert_if_not_exists<const N: usize>(
        &self,
        path_key_element_info: PathKeyElementInfo<N>,
        apply_type: BatchInsertApplyType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
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
                    drive_operations.push(
                        LowLevelDriveOperation::insert_for_known_path_key_element(
                            path,
                            key.to_vec(),
                            element,
                        ),
                    );
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
                    drive_operations.push(
                        LowLevelDriveOperation::insert_for_known_path_key_element(
                            path, key, element,
                        ),
                    );
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
                    drive_operations.push(
                        LowLevelDriveOperation::insert_for_known_path_key_element(
                            path_items,
                            key.to_vec(),
                            element,
                        ),
                    );
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
                            LowLevelDriveOperation::insert_for_estimated_path_key_element(
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

    /// Pushes an "insert element if element was changed or is new" operation to `drive_operations`.
    /// Returns true if the path key already exists without references.
    pub(crate) fn batch_insert_if_changed_value<const N: usize>(
        &self,
        path_key_element_info: PathKeyElementInfo<N>,
        apply_type: BatchInsertApplyType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<(bool, Option<Element>), Error> {
        match path_key_element_info {
            PathKeyRefElement((path, key, element)) => {
                let path_iter: Vec<&[u8]> = path.iter().map(|x| x.as_slice()).collect();
                let previous_element = self.grove_get_raw_optional(
                    path_iter.clone(),
                    key,
                    apply_type.to_direct_query_type(),
                    transaction,
                    drive_operations,
                )?;
                let needs_insert = match &previous_element {
                    None => true,
                    Some(previous_element) => previous_element != &element,
                };
                if needs_insert {
                    drive_operations.push(
                        LowLevelDriveOperation::insert_for_known_path_key_element(
                            path,
                            key.to_vec(),
                            element,
                        ),
                    );
                }
                Ok((needs_insert, previous_element))
            }
            PathKeyElement((path, key, element)) => {
                let path_iter: Vec<&[u8]> = path.iter().map(|x| x.as_slice()).collect();
                let previous_element = self.grove_get_raw_optional(
                    path_iter.clone(),
                    key.as_slice(),
                    apply_type.to_direct_query_type(),
                    transaction,
                    drive_operations,
                )?;
                let needs_insert = match &previous_element {
                    None => true,
                    Some(previous_element) => previous_element != &element,
                };
                if needs_insert {
                    drive_operations.push(
                        LowLevelDriveOperation::insert_for_known_path_key_element(
                            path, key, element,
                        ),
                    );
                }
                Ok((needs_insert, previous_element))
            }
            PathFixedSizeKeyRefElement((path, key, element)) => {
                let previous_element = self.grove_get_raw_optional(
                    path,
                    key,
                    apply_type.to_direct_query_type(),
                    transaction,
                    drive_operations,
                )?;
                let needs_insert = match &previous_element {
                    None => true,
                    Some(previous_element) => previous_element != &element,
                };
                if needs_insert {
                    let path_items: Vec<Vec<u8>> = path.into_iter().map(Vec::from).collect();
                    drive_operations.push(
                        LowLevelDriveOperation::insert_for_known_path_key_element(
                            path_items,
                            key.to_vec(),
                            element,
                        ),
                    );
                }
                Ok((needs_insert, previous_element))
            }
            PathKeyElementSize((key_info_path, key_info, element)) => {
                match apply_type {
                    BatchInsertApplyType::StatelessBatchInsert {
                        in_tree_using_sums, ..
                    } => {
                        // we can estimate that the element was the same size
                        drive_operations.push(CalculatedCostOperation(
                            GroveDb::average_case_for_get_raw(
                                &key_info_path,
                                &key_info,
                                element.serialized_size() as u32,
                                in_tree_using_sums,
                            ),
                        ));
                        drive_operations.push(
                            LowLevelDriveOperation::insert_for_estimated_path_key_element(
                                key_info_path,
                                key_info,
                                element,
                            ),
                        );
                        Ok((true, None))
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
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<(), Error>
    where
        P: IntoIterator<Item = &'c [u8]>,
        <P as IntoIterator>::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
    {
        let current_batch_operations =
            LowLevelDriveOperation::grovedb_operations_batch(drive_operations);
        let options = DeleteOptions {
            allow_deleting_non_empty_trees: false,
            deleting_non_empty_trees_returns_error: true,
            base_root_storage_is_free: true,
            validate_tree_at_path_exists: false, //todo: not sure about this one
        };
        let delete_operation = match apply_type {
            BatchDeleteApplyType::StatelessBatchDelete {
                is_sum_tree,
                estimated_value_size,
            } => GroveDb::worst_case_delete_operation_for_delete_internal::<RocksDbStorage>(
                &KeyInfoPath::from_known_path(path),
                &KeyInfo::KnownKey(key.to_vec()),
                is_sum_tree,
                false,
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
                is_known_to_be_subtree_with_sum,
                &current_batch_operations.operations,
                transaction,
            ),
        };

        if let Some(delete_operation) =
            push_drive_operation_result(delete_operation, drive_operations)?
        {
            // we also add the actual delete operation
            drive_operations.push(GroveOperation(delete_operation))
        }

        Ok(())
    }

    /// Pushes a "delete element" operation to `drive_operations` and returns the current element.
    /// If the element didn't exist does nothing.
    /// It is raw, because it does not use references.
    pub(crate) fn batch_remove_raw<'a, 'c, P>(
        &'a self,
        path: P,
        key: &'c [u8],
        apply_type: BatchDeleteApplyType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<Option<Element>, Error>
    where
        P: IntoIterator<Item = &'c [u8]>,
        <P as IntoIterator>::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
    {
        let mut current_batch_operations =
            LowLevelDriveOperation::grovedb_operations_batch(drive_operations);
        let options = DeleteOptions {
            allow_deleting_non_empty_trees: false,
            deleting_non_empty_trees_returns_error: true,
            base_root_storage_is_free: true,
            validate_tree_at_path_exists: false, //todo: not sure about this one
        };

        let path_iter = path.into_iter();

        let needs_removal_from_state =
            match current_batch_operations.remove_if_insert(path_iter.clone(), key) {
                Some(Op::Insert { element })
                | Some(Op::Replace { element })
                | Some(Op::Patch { element, .. }) => return Ok(Some(element)),
                Some(Op::InsertTreeWithRootHash { .. }) => {
                    return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                        "we should not be seeing internal grovedb operations",
                    )));
                }
                Some(Op::Delete { .. })
                | Some(Op::DeleteTree { .. })
                | Some(Op::DeleteSumTree { .. }) => false,
                _ => true,
            };

        let maybe_element = self.grove_get_raw_optional(
            path_iter.clone(),
            key,
            (&apply_type).into(),
            transaction,
            drive_operations,
        )?;
        if maybe_element.is_none()
            && matches!(
                &apply_type,
                &BatchDeleteApplyType::StatefulBatchDelete { .. }
            )
        {
            return Ok(None);
        }
        if needs_removal_from_state {
            let delete_operation = match apply_type {
                BatchDeleteApplyType::StatelessBatchDelete {
                    is_sum_tree,
                    estimated_value_size,
                } => GroveDb::worst_case_delete_operation_for_delete_internal::<RocksDbStorage>(
                    &KeyInfoPath::from_known_path(path_iter.clone()),
                    &KeyInfo::KnownKey(key.to_vec()),
                    is_sum_tree,
                    false,
                    true,
                    0,
                    estimated_value_size,
                )
                .map(|r| r.map(Some)),
                BatchDeleteApplyType::StatefulBatchDelete {
                    is_known_to_be_subtree_with_sum,
                } => self.grove.delete_operation_for_delete_internal(
                    path_iter.clone(),
                    key,
                    &options,
                    is_known_to_be_subtree_with_sum,
                    &current_batch_operations.operations,
                    transaction,
                ),
            };

            if let Some(delete_operation) =
                push_drive_operation_result(delete_operation, drive_operations)?
            {
                // we also add the actual delete operation
                drive_operations.push(GroveOperation(delete_operation))
            }
        }

        Ok(maybe_element)
    }

    /// Pushes a "delete up tree while empty" operation to `drive_operations`.
    pub(crate) fn batch_delete_up_tree_while_empty(
        &self,
        path: KeyInfoPath,
        key: &[u8],
        stop_path_height: Option<u16>,
        apply_type: BatchDeleteUpTreeApplyType,
        transaction: TransactionArg,
        check_existing_operations: &Option<&mut Vec<LowLevelDriveOperation>>,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<(), Error> {
        //these are the operations in the current operations (eg, delete/add)
        let mut current_batch_operations =
            LowLevelDriveOperation::grovedb_operations_batch(drive_operations);

        //These are the operations in the same batch, but in a different operation
        if let Some(existing_operations) = check_existing_operations {
            let mut other_batch_operations =
                LowLevelDriveOperation::grovedb_operations_batch(existing_operations);
            current_batch_operations.append(&mut other_batch_operations);
        }
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
                let options = DeleteUpTreeOptions {
                    allow_deleting_non_empty_trees: false,
                    deleting_non_empty_trees_returns_error: true,
                    base_root_storage_is_free: true,
                    validate_tree_at_path_exists: false,
                    stop_path_height,
                };
                self.grove.delete_operations_for_delete_up_tree_while_empty(
                    path.to_path_refs(),
                    key,
                    &options,
                    is_known_to_be_subtree_with_sum,
                    current_batch_operations.operations,
                    transaction,
                )
            }
        };
        let delete_operations = push_drive_operation_result(cost_context, drive_operations)?;
        delete_operations
            .into_iter()
            .for_each(|op| drive_operations.push(GroveOperation(op)));

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
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<(), Error> {
        if ops.is_empty() {
            return Err(Error::Drive(DriveError::BatchIsEmpty()));
        }
        // println!("batch {:#?}", ops);
        if self.config.batching_consistency_verification {
            let consistency_results = GroveDbOp::verify_consistency_of_operations(&ops.operations);
            if !consistency_results.is_empty() {
                println!("consistency_results {:#?}", consistency_results);
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
                disable_operation_consistency_check: !self.config.batching_consistency_verification,
                base_root_storage_is_free: true,
                batch_pause_height: None,
            }),
            |cost, old_flags, new_flags| {
                // if there were no flags before then the new flags are used
                if old_flags.is_none() {
                    return Ok(false);
                }
                // This could be none only because the old element didn't exist
                // If they were empty we get an error
                let maybe_old_storage_flags = StorageFlags::map_some_element_flags_ref(&old_flags)
                    .map_err(|_| {
                        GroveError::JustInTimeElementFlagsClientError(
                            "drive did not understand flags of old item being updated",
                        )
                    })?;
                let new_storage_flags = StorageFlags::from_element_flags_ref(new_flags)
                    .map_err(|_| {
                        GroveError::JustInTimeElementFlagsClientError(
                            "drive did not understand updated item flag information",
                        )
                    })?
                    .ok_or(GroveError::JustInTimeElementFlagsClientError(
                        "removing flags from an item with flags is not allowed",
                    ))?;
                match &cost.transition_type() {
                    OperationStorageTransitionType::OperationUpdateBiggerSize => {
                        let combined_storage_flags = StorageFlags::optional_combine_added_bytes(
                            maybe_old_storage_flags,
                            new_storage_flags,
                            cost.added_bytes,
                        )
                        .map_err(|_| {
                            GroveError::JustInTimeElementFlagsClientError(
                                "drive could not combine storage flags (new flags were bigger)",
                            )
                        })?;
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
                        let combined_storage_flags = StorageFlags::optional_combine_removed_bytes(
                            maybe_old_storage_flags,
                            new_storage_flags,
                            &cost.removed_bytes,
                        )
                        .map_err(|_| {
                            GroveError::JustInTimeElementFlagsClientError(
                                "drive could not combine storage flags (new flags were smaller)",
                            )
                        })?;
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
                let maybe_storage_flags =
                    StorageFlags::from_element_flags_ref(flags).map_err(|_| {
                        GroveError::SplitRemovalBytesClientError(
                            "drive did not understand flags of item being updated",
                        )
                    })?;
                // if there were no flags before then the new flags are used
                match maybe_storage_flags {
                    None => Ok((
                        BasicStorageRemoval(removed_key_bytes),
                        BasicStorageRemoval(removed_value_bytes),
                    )),
                    Some(storage_flags) => storage_flags
                        .split_storage_removed_bytes(removed_key_bytes, removed_value_bytes),
                }
            },
            transaction,
        );
        push_drive_operation_result(cost_context, drive_operations)
    }

    /// Applies the given groveDB operations batch.
    pub fn grove_apply_partial_batch(
        &self,
        ops: GroveDbOpBatch,
        validate: bool,
        add_on_operations: impl FnMut(
            &OperationCost,
            &Option<OpsByLevelPath>,
        ) -> Result<Vec<GroveDbOp>, GroveError>,
        transaction: TransactionArg,
    ) -> Result<(), Error> {
        self.grove_apply_partial_batch_with_add_costs(
            ops,
            validate,
            transaction,
            add_on_operations,
            &mut vec![],
        )
    }

    /// Applies the given groveDB operations batch and gets and passes the costs to `push_drive_operation_result`.
    pub(crate) fn grove_apply_partial_batch_with_add_costs(
        &self,
        ops: GroveDbOpBatch,
        validate: bool,
        transaction: TransactionArg,
        add_on_operations: impl FnMut(
            &OperationCost,
            &Option<OpsByLevelPath>,
        ) -> Result<Vec<GroveDbOp>, GroveError>,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<(), Error> {
        if ops.is_empty() {
            return Err(Error::Drive(DriveError::BatchIsEmpty()));
        }
        // println!("batch {:#?}", ops);
        if self.config.batching_consistency_verification {
            let consistency_results = GroveDbOp::verify_consistency_of_operations(&ops.operations);
            if !consistency_results.is_empty() {
                println!("consistency_results {:#?}", consistency_results);
                return Err(Error::Drive(DriveError::GroveDBInsertion(
                    "insertion order error",
                )));
            }
        }

        let cost_context = self.grove.apply_partial_batch_with_element_flags_update(
            ops.operations,
            Some(BatchApplyOptions {
                validate_insertion_does_not_override: validate,
                validate_insertion_does_not_override_tree: validate,
                allow_deleting_non_empty_trees: false,
                deleting_non_empty_trees_returns_error: true,
                disable_operation_consistency_check: false,
                base_root_storage_is_free: true,
                batch_pause_height: None,
            }),
            |cost, old_flags, new_flags| {
                // if there were no flags before then the new flags are used
                if old_flags.is_none() {
                    return Ok(false);
                }
                // This could be none only because the old element didn't exist
                // If they were empty we get an error
                let maybe_old_storage_flags = StorageFlags::map_some_element_flags_ref(&old_flags)
                    .map_err(|_| {
                        GroveError::JustInTimeElementFlagsClientError(
                            "drive did not understand flags of old item being updated",
                        )
                    })?;
                let new_storage_flags = StorageFlags::from_element_flags_ref(new_flags)
                    .map_err(|_| {
                        GroveError::JustInTimeElementFlagsClientError(
                            "drive did not understand updated item flag information",
                        )
                    })?
                    .ok_or(GroveError::JustInTimeElementFlagsClientError(
                        "removing flags from an item with flags is not allowed",
                    ))?;
                match &cost.transition_type() {
                    OperationStorageTransitionType::OperationUpdateBiggerSize => {
                        let combined_storage_flags = StorageFlags::optional_combine_added_bytes(
                            maybe_old_storage_flags,
                            new_storage_flags,
                            cost.added_bytes,
                        )
                        .map_err(|_| {
                            GroveError::JustInTimeElementFlagsClientError(
                                "drive could not combine storage flags (new flags were bigger)",
                            )
                        })?;
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
                        let combined_storage_flags = StorageFlags::optional_combine_removed_bytes(
                            maybe_old_storage_flags,
                            new_storage_flags,
                            &cost.removed_bytes,
                        )
                        .map_err(|_| {
                            GroveError::JustInTimeElementFlagsClientError(
                                "drive could not combine storage flags (new flags were smaller)",
                            )
                        })?;
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
                let maybe_storage_flags =
                    StorageFlags::from_element_flags_ref(flags).map_err(|_| {
                        GroveError::SplitRemovalBytesClientError(
                            "drive did not understand flags of item being updated",
                        )
                    })?;
                // if there were no flags before then the new flags are used
                match maybe_storage_flags {
                    None => Ok((
                        BasicStorageRemoval(removed_key_bytes),
                        BasicStorageRemoval(removed_value_bytes),
                    )),
                    Some(storage_flags) => storage_flags
                        .split_storage_removed_bytes(removed_key_bytes, removed_value_bytes),
                }
            },
            add_on_operations,
            transaction,
        );
        push_drive_operation_result(cost_context, drive_operations)
    }

    /// Gets the costs for the given groveDB op batch and passes them to `push_drive_operation_result`.
    pub(crate) fn grove_batch_operations_costs(
        &self,
        ops: GroveDbOpBatch,
        estimated_layer_info: HashMap<KeyInfoPath, EstimatedLayerInformation>,
        validate: bool,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
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
                batch_pause_height: None,
            }),
            |_, _, _| Ok(false),
            |_, _, _| Err(GroveError::InternalError("not implemented")),
        );
        push_drive_operation_result(cost_context, drive_operations)
    }
}
