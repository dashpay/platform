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
use costs::CostContext;
use grovedb::batch::{BatchApplyOptions, GroveDbOp, Op};
use grovedb::{Element, PathQuery, TransactionArg};

use crate::drive::flags::StorageFlags;
use crate::drive::object_size_info::KeyInfo::{Key, KeyRef, KeySize};
use crate::drive::object_size_info::KeyValueInfo::{KeyRefRequest, KeyValueMaxSize};
use crate::drive::object_size_info::PathKeyElementInfo::{
    PathFixedSizeKeyElement, PathKeyElement, PathKeyElementSize,
};
use crate::drive::object_size_info::PathKeyInfo::{
    PathFixedSizeKey, PathFixedSizeKeyRef, PathKey, PathKeyRef, PathKeySize,
};
use crate::drive::object_size_info::{KeyInfo, KeyValueInfo, PathKeyElementInfo, PathKeyInfo};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::op::DriveOperation::{CalculatedCostOperation, CostCalculationQueryOperation};
use crate::fee::op::{DriveOperation, SizesOfQueryOperation};
use grovedb::query_result_type::{QueryResultElements, QueryResultType};
use grovedb::Error as GroveError;

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

impl Drive {
    /// Pushes the `OperationCost` of inserting an empty tree in groveDB to `drive_operations`.
    pub(crate) fn grove_insert_empty_tree<'a, 'c, P>(
        &'a self,
        path: P,
        key_info: KeyInfo<'c>,
        storage_flags: &StorageFlags,
        transaction: TransactionArg,
        apply: bool,
        drive_operations: Option<&mut Vec<DriveOperation>>,
    ) -> Result<(), Error>
    where
        P: IntoIterator<Item = &'c [u8]>,
        <P as IntoIterator>::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
    {
        match key_info {
            KeyRef(key) => {
                let (path_items, path): (Vec<Vec<u8>>, Vec<&[u8]>) =
                    path.into_iter().map(|x| (Vec::from(x), x)).unzip();
                if apply {
                    // get cost context for inserting empty tree into grove
                    let CostContext { value, cost } = self
                        .grove
                        .insert(
                            path,
                            key,
                            Element::empty_tree_with_flags(storage_flags.to_element_flags()),
                            transaction,
                        )
                        .map_err(Error::GroveDB);
                    // if drive operations are given, push the operation cost
                    if let Some(drive_operations) = drive_operations {
                        drive_operations.push(CalculatedCostOperation(cost))
                    }
                    // return value
                    value?
                } else if let Some(drive_operations) = drive_operations {
                    drive_operations.push(DriveOperation::for_empty_tree(
                        path_items,
                        key.to_vec(),
                        Some(storage_flags),
                    ));
                }

                Ok(())
            }
            // if drive_operations is given, push `for_insert_path_key_value_size` with the given path size
            KeySize(key_max_length) => {
                if let Some(drive_operations) = drive_operations {
                    let path_size: u32 = path.into_iter().map(|p| p.len() as u32).sum();
                    drive_operations.push(DriveOperation::for_insert_path_key_value_size(
                        path_size,
                        key_max_length as u16,
                        0,
                    ));
                }
                Ok(())
            }
            // return error
            Key(_) => Err(Error::Drive(DriveError::GroveDBInsertion(
                "only a key ref can be inserted into groveDB",
            ))),
        }
    }

    /// Pushes the `OperationCost` of inserting an empty tree in groveDB where the
    /// path key does not yet exist to `drive_operations`.
    pub(crate) fn grove_insert_empty_tree_if_not_exists<'a, 'c, const N: usize>(
        &'a self,
        path_key_info: PathKeyInfo<'c, N>,
        storage_flags: &StorageFlags,
        transaction: TransactionArg,
        apply: bool,
        drive_operations: Option<&mut Vec<DriveOperation>>,
    ) -> Result<bool, Error> {
        match path_key_info {
            PathKeyRef((path, key)) => {
                let path_clone = path.clone();
                let path_iter: Vec<&[u8]> = path_clone.iter().map(|x| x.as_slice()).collect();
                let inserted = if apply {
                    let cost_context = self.grove.insert_if_not_exists(
                        path_iter.clone(),
                        key,
                        Element::empty_tree_with_flags(storage_flags.to_element_flags()),
                        transaction,
                    );
                    push_drive_operation_result_optional(cost_context, drive_operations)?
                } else {
                    if let Some(drive_operations) = drive_operations {
                        drive_operations.push(DriveOperation::for_empty_tree(
                            path,
                            key.to_vec(),
                            Some(storage_flags),
                        ));
                        drive_operations.push(CostCalculationQueryOperation(
                            SizesOfQueryOperation::for_key_check_in_path(key.len(), path_iter),
                        ));
                    }
                    // worst case is always that it was inserted
                    true
                };
                Ok(inserted)
            }
            PathKeySize((path_length, key_length)) => {
                if let Some(drive_operations) = drive_operations {
                    drive_operations.push(DriveOperation::for_insert_path_key_value_size(
                        path_length as u32,
                        key_length as u16,
                        0,
                    ));
                    drive_operations.push(CostCalculationQueryOperation(
                        SizesOfQueryOperation::for_key_check_with_path_length(
                            key_length,
                            path_length,
                        ),
                    ));
                }
                Ok(true)
            }
            PathKey((path, key)) => {
                let path_clone = path.clone();
                let path_iter: Vec<&[u8]> = path_clone.iter().map(|x| x.as_slice()).collect();
                let inserted = if apply {
                    let cost_context = self.grove.insert_if_not_exists(
                        path_iter.clone(),
                        key.as_slice(),
                        Element::empty_tree_with_flags(storage_flags.to_element_flags()),
                        transaction,
                    );
                    push_drive_operation_result_optional(cost_context, drive_operations)?
                } else {
                    if let Some(drive_operations) = drive_operations {
                        drive_operations.push(DriveOperation::for_empty_tree(
                            path,
                            key.to_vec(),
                            Some(storage_flags),
                        ));
                        drive_operations.push(CostCalculationQueryOperation(
                            SizesOfQueryOperation::for_key_check_in_path(key.len(), path_iter),
                        ));
                    }
                    // wost case scenario is true
                    true
                };
                Ok(inserted)
            }
            PathFixedSizeKey((path, key)) => {
                let inserted = if apply {
                    let cost_context = self.grove.insert_if_not_exists(
                        path.clone(),
                        key.as_slice(),
                        Element::empty_tree_with_flags(storage_flags.to_element_flags()),
                        transaction,
                    );
                    push_drive_operation_result_optional(cost_context, drive_operations)?
                } else {
                    if let Some(drive_operations) = drive_operations {
                        let path_clone = path.clone();
                        let path_items: Vec<Vec<u8>> =
                            path_clone.into_iter().map(Vec::from).collect();
                        drive_operations.push(DriveOperation::for_empty_tree(
                            path_items,
                            key.to_vec(),
                            Some(storage_flags),
                        ));
                        drive_operations.push(CostCalculationQueryOperation(
                            SizesOfQueryOperation::for_key_check_in_path(key.len(), path),
                        ));
                    }
                    // wost case scenario is true
                    true
                };
                Ok(inserted)
            }
            PathFixedSizeKeyRef((path, key)) => {
                let inserted = if apply {
                    let cost_context = self.grove.insert_if_not_exists(
                        path.clone(),
                        key,
                        Element::empty_tree_with_flags(storage_flags.to_element_flags()),
                        transaction,
                    );
                    push_drive_operation_result_optional(cost_context, drive_operations)?
                } else {
                    if let Some(drive_operations) = drive_operations {
                        let path_clone = path.clone();
                        let path_items: Vec<Vec<u8>> =
                            path_clone.into_iter().map(Vec::from).collect();
                        drive_operations.push(DriveOperation::for_empty_tree(
                            path_items,
                            key.to_vec(),
                            Some(storage_flags),
                        ));
                        drive_operations.push(CostCalculationQueryOperation(
                            SizesOfQueryOperation::for_key_check_in_path(key.len(), path),
                        ));
                    }
                    true
                };
                Ok(inserted)
            }
        }
    }

    /// Pushes the `OperationCost` of inserting an element in groveDB to `drive_operations`.
    pub(crate) fn grove_insert<'a, 'c, const N: usize>(
        &'a self,
        path_key_element_info: PathKeyElementInfo<'c, N>,
        transaction: TransactionArg,
        apply: bool,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error> {
        match path_key_element_info {
            PathKeyElement((path, key, element)) => {
                if apply {
                    // println!("element {:#?}", element);
                    let path_iter: Vec<&[u8]> = path.iter().map(|x| x.as_slice()).collect();
                    let cost_context = self.grove.insert(path_iter, key, element, transaction);
                    push_drive_operation_result(cost_context, drive_operations)
                } else {
                    let path_size = path.iter().map(|x| x.len() as u32).sum();
                    let key_len = key.len();
                    drive_operations.push(DriveOperation::for_insert_path_key_value_size(
                        path_size,
                        key_len as u16,
                        element.node_byte_size(key_len) as u32,
                    ));
                    Ok(())
                }
            }
            PathKeyElementSize((path_max_length, key_max_length, element_max_size)) => {
                drive_operations.push(DriveOperation::for_insert_path_key_value_size(
                    path_max_length as u32,
                    key_max_length as u16,
                    element_max_size as u32,
                ));
                Ok(())
            }
            PathFixedSizeKeyElement((path, key, element)) => {
                if apply {
                    let cost_context = self.grove.insert(path, key, element, transaction);
                    push_drive_operation_result(cost_context, drive_operations)
                } else {
                    let path_size = path.into_iter().map(|a| a.len() as u32).sum();
                    let key_len = key.len();
                    drive_operations.push(DriveOperation::for_insert_path_key_value_size(
                        path_size,
                        key_len as u16,
                        element.node_byte_size(key_len) as u32,
                    ));
                    Ok(())
                }
            }
        }
    }

    /// Pushes the `OperationCost` of inserting an element in groveDB where the path key does not yet exist
    /// to `drive_operations`.
    pub(crate) fn grove_insert_if_not_exists<'a, 'c, const N: usize>(
        &'a self,
        path_key_element_info: PathKeyElementInfo<'c, N>,
        transaction: TransactionArg,
        apply: bool,
        drive_operations: Option<&mut Vec<DriveOperation>>,
    ) -> Result<bool, Error> {
        match path_key_element_info {
            PathKeyElement((path, key, element)) => {
                let (path_iter, path_lengths): (Vec<&[u8]>, Vec<u32>) =
                    path.iter().map(|x| (x.as_slice(), x.len() as u32)).unzip();
                let element_node_byte_size = if drive_operations.is_some() {
                    element.node_byte_size(key.len()) as u32
                } else {
                    0 //doesn't matter
                };
                let inserted = if apply {
                    let cost_context = self.grove.insert_if_not_exists(
                        path_iter.clone(),
                        key,
                        element,
                        transaction,
                    );
                    push_drive_operation_result_optional(cost_context, drive_operations)?
                } else {
                    if let Some(drive_operations) = drive_operations {
                        let query_operation =
                            SizesOfQueryOperation::for_key_check_in_path(key.len(), path_iter);
                        drive_operations.push(CostCalculationQueryOperation(query_operation));
                        let insert_operation = DriveOperation::for_insert_path_key_value_size(
                            path_lengths.iter().sum(),
                            key.len() as u16,
                            element_node_byte_size,
                        );
                        drive_operations.push(insert_operation);
                    }
                    true
                };
                Ok(inserted)
            }
            PathKeyElementSize((path_size, key_max_length, element_max_size)) => {
                let insert_operation = DriveOperation::for_insert_path_key_value_size(
                    path_size as u32,
                    key_max_length as u16,
                    element_max_size as u32,
                );
                let query_operation = SizesOfQueryOperation::for_key_check_with_path_length(
                    key_max_length,
                    path_size,
                );
                if let Some(drive_operations) = drive_operations {
                    drive_operations.push(CostCalculationQueryOperation(query_operation));
                    drive_operations.push(insert_operation);
                }
                Ok(true)
            }
            PathFixedSizeKeyElement((path, key, element)) => {
                let path_iter = path.into_iter();
                let element_node_byte_size = if drive_operations.is_some() {
                    element.node_byte_size(key.len()) as u32
                } else {
                    0 //doesn't matter
                };
                let inserted = if apply {
                    let cost_context =
                        self.grove
                            .insert_if_not_exists(path_iter, key, element, transaction);
                    push_drive_operation_result_optional(cost_context, drive_operations)?
                } else {
                    if let Some(drive_operations) = drive_operations {
                        let query_operation =
                            SizesOfQueryOperation::for_key_check_in_path(key.len(), path_iter);
                        drive_operations.push(CostCalculationQueryOperation(query_operation));
                        let insert_operation = DriveOperation::for_insert_path_key_value_size(
                            path.iter().map(|a| a.len() as u32).sum(),
                            key.len() as u16,
                            element_node_byte_size,
                        );
                        drive_operations.push(insert_operation);
                    }
                    true
                };
                Ok(inserted)
            }
        }
    }

    /// Pushes the `OperationCost` of deleting an element in groveDB to `drive_operations`.
    pub(crate) fn grove_delete<'p>(
        &self,
        path: Vec<Vec<u8>>,
        key: &'p [u8],
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error> {
        if apply {
            let path_iter: Vec<&[u8]> = path.iter().map(|x| x.as_slice()).collect();
            let cost_context = self.grove.delete(path_iter, key, transaction);
            push_drive_operation_result(cost_context, drive_operations)
        } else {
            // TODO this is wrong
            drive_operations.push(DriveOperation::for_delete_path_key_value_size(
                path,
                key.len() as u16,
                0,
                1,
            ));
            Ok(())
        }
    }

    /// Gets the element at the given path from groveDB.
    /// Pushes the `OperationCost` of getting the element to `drive_operations`.
    pub(crate) fn grove_get<'a, 'c, P>(
        &'a self,
        path: P,
        key_value_info: KeyValueInfo<'c>,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<Option<Element>, Error>
    where
        P: IntoIterator<Item = &'c [u8]>,
        <P as IntoIterator>::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
    {
        let path_iter = path.into_iter();
        match key_value_info {
            KeyRefRequest(key) => {
                let CostContext { value, cost } =
                    self.grove.get(path_iter.clone(), key, transaction);
                drive_operations.push(CalculatedCostOperation(cost));
                Ok(Some(value.map_err(Error::GroveDB)?))
            }
            KeyValueMaxSize((key_size, value_size)) => {
                drive_operations.push(CostCalculationQueryOperation(
                    SizesOfQueryOperation::for_value_retrieval_in_path(
                        key_size, path_iter, value_size,
                    ),
                ));
                Ok(None)
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
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<bool, Error>
    where
        P: IntoIterator<Item = &'p [u8]>,
        <P as IntoIterator>::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
    {
        let CostContext { value, cost } = if apply {
            if self.config.has_raw_enabled {
                self.grove.has_raw(path, key, transaction)
            } else {
                self.grove
                    .get_raw(path, key, transaction)
                    .map(|r| r.map(|_e| true))
            }
        } else {
            self.grove.worst_case_for_has_raw(path, key)
        };
        drive_operations.push(CalculatedCostOperation(cost));
        match value {
            Err(GroveError::PathKeyNotFound(_)) | Err(GroveError::PathNotFound(_)) => Ok(false),
            _ => Ok(value?),
        }
    }

    /// Pushes an "insert empty tree" operation to `drive_operations`.
    pub(crate) fn batch_insert_empty_tree<'a, 'c, P>(
        &'a self,
        path: P,
        key_info: KeyInfo<'c>,
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
                drive_operations.push(DriveOperation::for_empty_tree(
                    path_items,
                    key.to_vec(),
                    storage_flags,
                ));
                Ok(())
            }
            KeySize(key_max_length) => {
                let path_size = path.into_iter().map(|x| x.len() as u32).sum();
                drive_operations.push(DriveOperation::for_insert_path_key_value_size(
                    path_size,
                    key_max_length as u16,
                    0,
                ));
                Ok(())
            }
            Key(_) => Err(Error::Drive(DriveError::GroveDBInsertion(
                "only a key ref can be inserted into groveDB",
            ))),
        }
    }

    /// Pushes an "insert empty tree where path key does not yet exist" operation to `drive_operations`.
    pub(crate) fn batch_insert_empty_tree_if_not_exists<'a, 'c, const N: usize>(
        &'a self,
        path_key_info: PathKeyInfo<'c, N>,
        storage_flags: &StorageFlags,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<bool, Error> {
        match path_key_info {
            PathKeyRef((path, key)) => {
                let path_iter: Vec<&[u8]> = path.iter().map(|x| x.as_slice()).collect();
                let has_raw = self.grove_has_raw(
                    path_iter.clone(),
                    key,
                    apply,
                    transaction,
                    drive_operations,
                )?;
                if !has_raw {
                    drive_operations.push(DriveOperation::for_empty_tree(
                        path,
                        key.to_vec(),
                        Some(storage_flags),
                    ));
                }
                Ok(!has_raw)
            }
            PathKeySize((path_length, key_length)) => {
                drive_operations.push(DriveOperation::for_insert_path_key_value_size(
                    path_length as u32,
                    key_length as u16,
                    0,
                ));

                drive_operations.push(CostCalculationQueryOperation(
                    SizesOfQueryOperation::for_key_check_with_path_length(key_length, path_length),
                ));
                Ok(true)
            }
            PathKey((path, key)) => {
                let path_iter: Vec<&[u8]> = path.iter().map(|x| x.as_slice()).collect();
                let has_raw = self.grove_has_raw(
                    path_iter.clone(),
                    key.as_slice(),
                    apply,
                    transaction,
                    drive_operations,
                )?;
                if !has_raw {
                    drive_operations.push(DriveOperation::for_empty_tree(
                        path,
                        key.to_vec(),
                        Some(storage_flags),
                    ));
                }
                Ok(!has_raw)
            }
            PathFixedSizeKey((path, key)) => {
                let has_raw = self.grove_has_raw(
                    path.clone(),
                    key.as_slice(),
                    apply,
                    transaction,
                    drive_operations,
                )?;
                if !has_raw {
                    let path_items: Vec<Vec<u8>> = path.into_iter().map(Vec::from).collect();
                    drive_operations.push(DriveOperation::for_empty_tree(
                        path_items,
                        key.to_vec(),
                        Some(storage_flags),
                    ));
                }
                Ok(!has_raw)
            }
            PathFixedSizeKeyRef((path, key)) => {
                let has_raw =
                    self.grove_has_raw(path.clone(), key, apply, transaction, drive_operations)?;
                if !has_raw {
                    let path_items: Vec<Vec<u8>> = path.into_iter().map(Vec::from).collect();
                    drive_operations.push(DriveOperation::for_empty_tree(
                        path_items,
                        key.to_vec(),
                        Some(storage_flags),
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
            PathKeyElement((path, key, element)) => {
                drive_operations.push(DriveOperation::for_path_key_element(
                    path,
                    key.to_vec(),
                    element,
                ));
                Ok(())
            }
            PathKeyElementSize((path_max_length, key_max_length, element_max_size)) => {
                drive_operations.push(DriveOperation::for_insert_path_key_value_size(
                    path_max_length as u32,
                    key_max_length as u16,
                    element_max_size as u32,
                ));
                Ok(())
            }
            PathFixedSizeKeyElement((path, key, element)) => {
                let path_items: Vec<Vec<u8>> = path.into_iter().map(Vec::from).collect();
                drive_operations.push(DriveOperation::for_path_key_element(
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
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<bool, Error> {
        match path_key_element_info {
            PathKeyElement((path, key, element)) => {
                let path_iter: Vec<&[u8]> = path.iter().map(|x| x.as_slice()).collect();
                let has_raw = self.grove_has_raw(
                    path_iter.clone(),
                    key,
                    apply,
                    transaction,
                    drive_operations,
                )?;
                if !has_raw {
                    drive_operations.push(DriveOperation::for_path_key_element(
                        path,
                        key.to_vec(),
                        element,
                    ));
                }
                Ok(!has_raw)
            }
            PathFixedSizeKeyElement((path, key, element)) => {
                let has_raw =
                    self.grove_has_raw(path, key, apply, transaction, drive_operations)?;
                if !has_raw {
                    let path_items: Vec<Vec<u8>> = path.into_iter().map(Vec::from).collect();
                    drive_operations.push(DriveOperation::for_path_key_element(
                        path_items,
                        key.to_vec(),
                        element,
                    ));
                }
                Ok(!has_raw)
            }
            PathKeyElementSize((path_size, key_max_length, element_max_size)) => {
                let insert_operation = DriveOperation::for_insert_path_key_value_size(
                    path_size as u32,
                    key_max_length as u16,
                    element_max_size as u32,
                );
                let query_operation = SizesOfQueryOperation::for_key_check_with_path_length(
                    key_max_length,
                    path_size,
                );
                drive_operations.push(insert_operation);
                drive_operations.push(CostCalculationQueryOperation(query_operation));
                Ok(true)
            }
        }
    }

    /// Pushes a "delete element" operation to `drive_operations`.
    pub(crate) fn batch_delete<'a, 'c, P>(
        &'a self,
        path: P,
        key: &'c [u8],
        only_delete_tree_if_empty: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error>
    where
        P: IntoIterator<Item = &'c [u8]>,
        <P as IntoIterator>::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
    {
        let current_batch_operations = DriveOperation::grovedb_operations_batch(drive_operations);
        let cost_context = self.grove.delete_operation_for_delete_internal(
            path,
            key,
            only_delete_tree_if_empty,
            true,
            &current_batch_operations.operations,
            transaction,
        );

        if let Some(delete_operation) = push_drive_operation_result(cost_context, drive_operations)?
        {
            // we also add the actual delete operation
            drive_operations.push(DriveOperation::GroveOperation(delete_operation))
        }
        Ok(())
    }

    /// Pushes a "delete up tree while empty" operation to `drive_operations`.
    pub(crate) fn batch_delete_up_tree_while_empty<'a, 'c, P>(
        &'a self,
        path: P,
        key: &'c [u8],
        stop_path_height: Option<u16>,
        transaction: TransactionArg,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error>
    where
        P: IntoIterator<Item = &'c [u8]>,
        <P as IntoIterator>::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
    {
        let current_batch_operations = DriveOperation::grovedb_operations_batch(drive_operations);
        let cost_context = self.grove.delete_operations_for_delete_up_tree_while_empty(
            path,
            key,
            stop_path_height,
            true,
            current_batch_operations.operations,
            transaction,
        );
        if let Some(delete_operations) =
            push_drive_operation_result(cost_context, drive_operations)?
        {
            delete_operations
                .into_iter()
                .for_each(|op| drive_operations.push(DriveOperation::GroveOperation(op)))
        }
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
        if ops.len() == 0 {
            return Err(Error::Drive(DriveError::BatchIsEmpty()));
        }
        if self.config.batching_enabled {
            // println!("batch {:#?}", ops);
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

            let cost_context = self.grove.apply_batch(
                ops.operations,
                Some(BatchApplyOptions {
                    validate_insertion_does_not_override: validate,
                    disable_operation_consistency_check: true,
                }),
                transaction,
            );
            push_drive_operation_result(cost_context, drive_operations)
        } else {
            //println!("changes {} {:#?}", ops.len(), ops);
            for op in ops.operations.into_iter() {
                //println!("on {:#?}", op);
                match op.op {
                    Op::Insert { element } => self.grove_insert(
                        PathKeyElementInfo::<0>::PathKeyElement((
                            op.path.clone(),
                            op.key.as_slice(),
                            element,
                        )),
                        transaction,
                        true,
                        drive_operations,
                    )?,
                    Op::Delete => self.grove_delete(
                        op.path,
                        op.key.as_slice(),
                        true,
                        transaction,
                        drive_operations,
                    )?,
                    _ => {
                        return Err(Error::Drive(DriveError::UnsupportedPrivate(
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
        validate: bool,
        drive_operations: &mut Vec<DriveOperation>,
    ) -> Result<(), Error> {
        let cost_context = self.grove.worst_case_operations_for_batch(
            ops.operations,
            Some(BatchApplyOptions {
                disable_operation_consistency_check: true,
                validate_insertion_does_not_override: validate,
            }),
        );
        push_drive_operation_result(cost_context, drive_operations)
    }
}
