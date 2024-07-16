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

/// Batch replace operation
pub mod batch_replace;

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

/// Clear a subtree in grovedb
pub mod grove_clear;

mod grove_get_path_query_serialized_or_sum_results;
/// Proved path query in grovedb with a conditional query
pub mod grove_get_proved_path_query_with_conditional;

use grovedb_costs::CostContext;

use grovedb::EstimatedLayerInformation;

use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::CalculatedCostOperation;

use grovedb::Error as GroveError;

use intmap::IntMap;

/// Pushes an operation's `OperationCost` to `drive_operations` given its `CostContext`
/// and returns the operation's return value.
fn push_drive_operation_result<T>(
    cost_context: CostContext<Result<T, GroveError>>,
    drive_operations: &mut Vec<LowLevelDriveOperation>,
) -> Result<T, Error> {
    let CostContext { value, cost } = cost_context;
    if !cost.is_nothing() {
        drive_operations.push(CalculatedCostOperation(cost));
    }
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
/// Is subtree?
pub type IsSubTree = bool;
/// Is sum subtree?
pub type IsSumSubTree = bool;
/// Is sum tree?
pub type IsSumTree = bool;

/// Batch delete apply type
#[derive(Debug, Copy, Clone)]
pub enum BatchDeleteApplyType {
    /// Stateless batch delete
    StatelessBatchDelete {
        /// Are we deleting in a sum tree
        is_sum_tree: bool,
        /// What is the estimated value size
        estimated_key_size: u32,
        /// What is the estimated value size
        estimated_value_size: u32,
    },
    /// Stateful batch delete
    StatefulBatchDelete {
        /// Are we known to be in a subtree and does this subtree have sums
        is_known_to_be_subtree_with_sum: Option<(IsSubTree, IsSumSubTree)>,
    },
}

#[derive(Clone)]
/// Batch delete up tree apply type
pub enum BatchDeleteUpTreeApplyType {
    /// Stateless batch delete
    StatelessBatchDelete {
        /// The estimated layer info
        estimated_layer_info: IntMap<EstimatedLayerInformation>,
    },
    /// Stateful batch delete
    StatefulBatchDelete {
        /// Are we known to be in a subtree and does this subtree have sums
        is_known_to_be_subtree_with_sum: Option<(IsSubTree, IsSumSubTree)>,
    },
}

/// batch insert tree apply type
#[derive(Clone, Copy)]
/// Batch insert tree apply type
pub enum BatchInsertTreeApplyType {
    /// Stateless batch insert tree
    StatelessBatchInsertTree {
        /// Does this tree use sums?
        in_tree_using_sums: bool,
        /// Are we inserting in a sum tree
        is_sum_tree: bool,
        /// The flags length
        flags_len: FlagsLen,
    },
    /// Stateful batch insert tree
    StatefulBatchInsertTree,
}

/// Represents the types for batch insert operations in a tree structure.
impl BatchInsertTreeApplyType {
    /// Converts the current `BatchInsertTreeApplyType` into a corresponding `DirectQueryType`.
    ///
    /// # Returns
    ///
    /// - A variant of `DirectQueryType::StatelessDirectQuery` if the current type is `BatchInsertTreeApplyType::StatelessBatchInsertTree`.
    /// - `DirectQueryType::StatefulDirectQuery` if the current type is `BatchInsertTreeApplyType::StatefulBatchInsertTree`.
    /// ```
    pub(crate) fn to_direct_query_type(self) -> DirectQueryType {
        match self {
            BatchInsertTreeApplyType::StatelessBatchInsertTree {
                in_tree_using_sums,
                is_sum_tree,
                flags_len,
            } => DirectQueryType::StatelessDirectQuery {
                in_tree_using_sums,
                query_target: QueryTarget::QueryTargetTree(flags_len, is_sum_tree),
            },
            BatchInsertTreeApplyType::StatefulBatchInsertTree => {
                DirectQueryType::StatefulDirectQuery
            }
        }
    }
}

/// Batch insert apply type
pub enum BatchInsertApplyType {
    /// Stateless batch insert
    StatelessBatchInsert {
        /// Does this tree use sums?
        in_tree_using_sums: bool,
        /// the type of Target (Tree or Value)
        target: QueryTarget,
    },
    /// Stateful batch insert
    StatefulBatchInsert,
}

impl BatchInsertApplyType {
    /// Converts the current `BatchInsertApplyType` into a corresponding `DirectQueryType`.
    ///
    /// # Returns
    ///
    /// - A variant of `DirectQueryType::StatelessDirectQuery` if the current type is `BatchInsertApplyType::StatelessBatchInsert`.
    /// - `DirectQueryType::StatefulDirectQuery` if the current type is `BatchInsertApplyType::StatefulBatchInsert`.
    /// ```
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

/// Flags length
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
    /// Length
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
    /// Stateless direct query
    StatelessDirectQuery {
        /// Does this tree use sums?
        in_tree_using_sums: bool,
        /// the type of Target (Tree or Value)
        query_target: QueryTarget,
    },
    /// Stateful direct query
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
    #[allow(dead_code)]
    #[deprecated(note = "This function is marked as unused.")]
    #[allow(deprecated)]
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

/// Query type
#[derive(Clone)]
pub enum QueryType {
    /// Stateless query
    StatelessQuery {
        /// Does this tree use sums?
        in_tree_using_sums: bool,
        /// the type of Target (Tree or Value)
        query_target: QueryTarget,
        /// The estimated sizes of references
        estimated_reference_sizes: Vec<u32>,
    },
    /// Stateful query
    StatefulQuery,
}

impl From<BatchDeleteApplyType> for QueryType {
    fn from(value: BatchDeleteApplyType) -> Self {
        match value {
            BatchDeleteApplyType::StatelessBatchDelete {
                is_sum_tree,
                estimated_value_size,
                ..
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
                ..
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
                ..
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
                ..
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
