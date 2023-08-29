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

/// Grove insert operation
pub mod grove_insert;

/// Grove insert operation into an empty tree
pub mod grove_insert_empty_tree;

/// Grove insert operation into an empty sum tree
pub mod grove_insert_empty_sum_tree;

/// Grove insert operation, but only if it doesn't already exist
pub mod grove_insert_if_not_exists;

/// Grove delete operation
pub mod grove_delete;

/// Fetch raw grove data
pub mod grove_get_raw;

/// Fetch raw grove data if it exists
pub mod grove_get_raw_optional;

/// Fetch u64 value from encoded variable vector in raw grove data
pub mod grove_get_raw_value_u64_from_encoded_var_vec;

/// Grove get operation
pub mod grove_get;

/// Serialized results from grove path query
pub mod grove_get_path_query_serialized_results;

/// Grove path query operation
pub mod grove_get_path_query;

/// Grove path query operation with optional return value
pub mod grove_get_path_query_with_optional;

/// Fetch raw data from grove path query with optional return value
pub mod grove_get_raw_path_query_with_optional;

/// Fetch raw data from grove path query
pub mod grove_get_raw_path_query;

/// Proved path query in grove
pub mod grove_get_proved_path_query;

/// Get total value from sum tree in grove
pub mod grove_get_sum_tree_total_value;

/// Check if raw data exists in grove
pub mod grove_has_raw;

/// Batch insert operation into empty tree
pub mod batch_insert_empty_tree;

/// Batch insert operation into empty tree, but only if it doesn't already exist
pub mod batch_insert_empty_tree_if_not_exists;

/// Batch insert operation into empty tree, but only if it doesn't exist and check existing operations
pub mod batch_insert_empty_tree_if_not_exists_check_existing_operations;

/// Batch insert operation
pub mod batch_insert;

/// Batch insert operation, but only if it doesn't already exist
pub mod batch_insert_if_not_exists;

/// Batch insert operation, but only if the value has changed
pub mod batch_insert_if_changed_value;

/// Batch delete operation
pub mod batch_delete;

/// Batch remove raw data operation
pub mod batch_remove_raw;

/// Batch delete operation up the tree while it's empty
pub mod batch_delete_up_tree_while_empty;

/// Batch refresh reference operation
pub mod batch_refresh_reference;

/// Apply grove operation
pub mod grove_apply_operation;

/// Apply batch grove operation
pub mod grove_apply_batch;

/// Apply batch grove operation with additional costs
pub mod grove_apply_batch_with_add_costs;

/// Apply partial batch grove operation
pub mod grove_apply_partial_batch;

/// Apply partial batch grove operation with additional costs
pub mod grove_apply_partial_batch_with_add_costs;

/// Get cost of grove batch operations
pub mod grove_batch_operations_costs;

use grovedb_costs::CostContext;

use grovedb::EstimatedLayerInformation;

use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use crate::fee::op::LowLevelDriveOperation::CalculatedCostOperation;

use grovedb::Error as GroveError;

use intmap::IntMap;

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
<<<<<<< HEAD
/// Is subtree?
pub type IsSubTree = bool;
/// Is sum subtree?
pub type IsSumSubTree = bool;
/// Is sum tree?
pub type IsSumTree = bool;

/// Batch delete apply type
pub enum BatchDeleteApplyType {
    /// Stateless batch delete
=======
/// is subtree?
pub type IsSubTree = bool;
/// is sum subtree?
pub type IsSumSubTree = bool;
/// is sum tree?
pub type IsSumTree = bool;

/// batch delete apply type
pub enum BatchDeleteApplyType {
    /// stateless batch delete
>>>>>>> 6ac041d9e (feat: add docs)
    StatelessBatchDelete {
        is_sum_tree: bool,
        estimated_value_size: u32,
    },
<<<<<<< HEAD
    /// Stateful batch delete
=======
    /// stateful batch delete
>>>>>>> 6ac041d9e (feat: add docs)
    StatefulBatchDelete {
        is_known_to_be_subtree_with_sum: Option<(IsSubTree, IsSumSubTree)>,
    },
}

<<<<<<< HEAD
/// Batch delete up tree apply type
pub enum BatchDeleteUpTreeApplyType {
    /// Stateless batch delete
    StatelessBatchDelete {
        estimated_layer_info: IntMap<EstimatedLayerInformation>,
    },
    /// Stateful batch delete
=======
/// batch delete up tree apply type
pub enum BatchDeleteUpTreeApplyType {
    /// stateless batch delete
    StatelessBatchDelete {
        estimated_layer_info: IntMap<EstimatedLayerInformation>,
    },
    /// stateful batch delete
>>>>>>> 6ac041d9e (feat: add docs)
    StatefulBatchDelete {
        is_known_to_be_subtree_with_sum: Option<(IsSubTree, IsSumSubTree)>,
    },
}

/// batch insert tree apply type
#[derive(Clone, Copy)]
/// Batch insert tree apply type
pub enum BatchInsertTreeApplyType {
<<<<<<< HEAD
    /// Stateless batch insert tree
=======
    /// stateless batch insert tree
>>>>>>> 6ac041d9e (feat: add docs)
    StatelessBatchInsertTree {
        in_tree_using_sums: bool,
        is_sum_tree: bool,
        flags_len: FlagsLen,
    },
<<<<<<< HEAD
    /// Stateful batch insert tree
=======
    /// stateful batch insert tree
>>>>>>> 6ac041d9e (feat: add docs)
    StatefulBatchInsertTree,
}

/// Represents the types for batch insert operations in a tree structure.
impl BatchInsertTreeApplyType {
<<<<<<< HEAD
    /// Converts the current `BatchInsertTreeApplyType` into a corresponding `DirectQueryType`.
    ///
    /// # Returns
    /// 
    /// - A variant of `DirectQueryType::StatelessDirectQuery` if the current type is `BatchInsertTreeApplyType::StatelessBatchInsertTree`.
    /// - `DirectQueryType::StatefulDirectQuery` if the current type is `BatchInsertTreeApplyType::StatefulBatchInsertTree`.
    ///
    /// # Example
    ///
    /// ```
    /// let batch_type = BatchInsertTreeApplyType::StatelessBatchInsertTree {
    ///     in_tree_using_sums: true,
    ///     is_sum_tree: false,
    ///     flags_len: 5,
    /// };
    ///
    /// let query_type = batch_type.to_direct_query_type();
    /// ```
=======
    /// to direct query type
>>>>>>> 6ac041d9e (feat: add docs)
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

<<<<<<< HEAD
/// Batch insert apply type
pub enum BatchInsertApplyType {
    /// Stateless batch insert
=======
/// batch insert apply type
pub enum BatchInsertApplyType {
    /// stateless
>>>>>>> 6ac041d9e (feat: add docs)
    StatelessBatchInsert {
        in_tree_using_sums: bool,
        target: QueryTarget,
    },
<<<<<<< HEAD
    /// Stateful batch insert
=======
    /// stateful
>>>>>>> 6ac041d9e (feat: add docs)
    StatefulBatchInsert,
}

impl BatchInsertApplyType {
<<<<<<< HEAD
    /// Converts the current `BatchInsertApplyType` into a corresponding `DirectQueryType`.
    ///
    /// # Returns
    /// 
    /// - A variant of `DirectQueryType::StatelessDirectQuery` if the current type is `BatchInsertApplyType::StatelessBatchInsert`.
    /// - `DirectQueryType::StatefulDirectQuery` if the current type is `BatchInsertApplyType::StatefulBatchInsert`.
    ///
    /// # Example
    ///
    /// ```
    /// let batch_type = BatchInsertApplyType::StatelessBatchInsert {
    ///     in_tree_using_sums: true,
    ///     target: SomeQueryTarget, // Replace with an actual target instance.
    /// };
    ///
    /// let query_type = batch_type.to_direct_query_type();
    /// ```
=======
    /// to direct query type
>>>>>>> 6ac041d9e (feat: add docs)
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

<<<<<<< HEAD
/// Flags length
=======
/// flags length
>>>>>>> 6ac041d9e (feat: add docs)
pub type FlagsLen = u32;

/// query target
#[derive(Clone, Copy)]
/// Query target
pub enum QueryTarget {
    /// tree
    QueryTargetTree(FlagsLen, IsSumTree),
    /// value
    QueryTargetValue(u32),
}

impl QueryTarget {
<<<<<<< HEAD
    /// Length
=======
    /// get query target length
>>>>>>> 6ac041d9e (feat: add docs)
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

/// direct query type
#[derive(Clone, Copy)]
/// Direct query type
pub enum DirectQueryType {
<<<<<<< HEAD
    /// Stateless direct query
=======
    /// stateless
>>>>>>> 6ac041d9e (feat: add docs)
    StatelessDirectQuery {
        in_tree_using_sums: bool,
        query_target: QueryTarget,
    },
<<<<<<< HEAD
    /// Stateful direct query
=======
    /// stateful
>>>>>>> 6ac041d9e (feat: add docs)
    StatefulDirectQuery,
}

impl From<DirectQueryType> for QueryType {
    fn from(value: DirectQueryType) -> Self {
        match value {
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
}

impl DirectQueryType {
<<<<<<< HEAD
    /// Converts the current `DirectQueryType` into a corresponding `QueryType` 
    /// while associating it with the given reference sizes.
    ///
    /// # Parameters
    ///
    /// * `reference_sizes`: A vector of `u32` values representing the reference sizes 
    ///   associated with the query.
    ///
    /// # Returns
    /// 
    /// - A variant of `QueryType::StatelessQuery` with the provided reference sizes if 
    ///   the current type is `DirectQueryType::StatelessDirectQuery`.
    /// - `QueryType::StatefulQuery` if the current type is `DirectQueryType::StatefulDirectQuery`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let direct_query = DirectQueryType::StatelessDirectQuery {
    ///     in_tree_using_sums: true,
    ///     query_target: SomeTarget, // Replace with an actual target instance.
    /// };
    ///
    /// let ref_sizes = vec![100, 200, 300];
    /// let query_type = direct_query.add_reference_sizes(ref_sizes);
    /// ```
=======
    /// add reference sizes to direct query type
>>>>>>> 6ac041d9e (feat: add docs)
    #[allow(dead_code)]
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

<<<<<<< HEAD
/// Query type
#[derive(Clone)]
pub enum QueryType {
    /// Stateless query
=======
/// query type (sam is downgraded from A+ manager to A- for making me do all these docs)
#[derive(Clone)]
pub enum QueryType {
    /// stateless
>>>>>>> 6ac041d9e (feat: add docs)
    StatelessQuery {
        in_tree_using_sums: bool,
        query_target: QueryTarget,
        estimated_reference_sizes: Vec<u32>,
    },
<<<<<<< HEAD
    /// Stateful query
=======
    /// stateful
>>>>>>> 6ac041d9e (feat: add docs)
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
