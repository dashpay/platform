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

//! Fee ops
//!

use crate::drive::batch::GroveDbOpBatch;
use costs::storage_cost::removal::Identifier;
use costs::storage_cost::removal::StorageRemovedBytes::{
    BasicStorageRemoval, NoStorageRemoval, SectionedStorageRemoval,
};
use costs::storage_cost::StorageCost;
use costs::OperationCost;
use enum_map::Enum;
use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::KeyInfoPath;
use grovedb::{batch::GroveDbOp, Element, PathQuery};
use std::collections::BTreeMap;

use crate::drive::flags::StorageFlags;
use crate::error::drive::DriveError;
use crate::error::fee::FeeError;
use crate::error::Error;
use crate::fee::default_costs::{
    STORAGE_DISK_USAGE_CREDIT_PER_BYTE, STORAGE_LOAD_CREDIT_PER_BYTE,
    STORAGE_PROCESSING_CREDIT_PER_BYTE, STORAGE_SEEK_COST,
};
use crate::fee::op::DriveOperation::{
    CalculatedCostOperation, GroveOperation, PreCalculatedFeeResult,
};
use crate::fee::removed_bytes_from_epochs_by_identities::RemovedBytesFromEpochsByIdentities;
use crate::fee::FeeResult;
use crate::fee_pools::epochs::Epoch;

/// Base ops
#[derive(Debug, Enum)]
pub enum BaseOp {
    /// Stop
    Stop,
    /// Add
    Add,
    /// Multiply
    Mul,
    /// Subtract
    Sub,
    /// Divide
    Div,
    /// Sdiv
    Sdiv,
    /// Modulo
    Mod,
    /// Smod
    Smod,
    /// Addmod
    Addmod,
    /// Mulmod
    Mulmod,
    /// Signextend
    Signextend,
    /// Less than
    Lt,
    /// Greater than
    Gt,
    /// Slt
    Slt,
    /// Sgt
    Sgt,
    /// Equals
    Eq,
    /// Is zero
    Iszero,
    /// And
    And,
    /// Or
    Or,
    /// Xor
    Xor,
    /// Not
    Not,
    /// Byte
    Byte,
}

impl BaseOp {
    /// Match the op and get the cost
    pub fn cost(&self) -> u64 {
        match self {
            BaseOp::Stop => 0,
            BaseOp::Add => 12,
            BaseOp::Mul => 20,
            BaseOp::Sub => 12,
            BaseOp::Div => 20,
            BaseOp::Sdiv => 20,
            BaseOp::Mod => 20,
            BaseOp::Smod => 20,
            BaseOp::Addmod => 32,
            BaseOp::Mulmod => 32,
            BaseOp::Signextend => 20,
            BaseOp::Lt => 12,
            BaseOp::Gt => 12,
            BaseOp::Slt => 12,
            BaseOp::Sgt => 12,
            BaseOp::Eq => 12,
            BaseOp::Iszero => 12,
            BaseOp::And => 12,
            BaseOp::Or => 12,
            BaseOp::Xor => 12,
            BaseOp::Not => 12,
            BaseOp::Byte => 12,
        }
    }
}

/// Function ops
#[derive(Debug, Enum)]
pub enum FunctionOp {
    /// SHA256
    Sha256,
    /// SHA256_2
    Sha256_2,
    /// BLAKE3
    Blake3,
}

impl FunctionOp {
    /// Cost
    pub fn cost(&self, _epoch: &Epoch) -> u64 {
        match self {
            FunctionOp::Sha256 => 4000,
            FunctionOp::Sha256_2 => 8000,
            FunctionOp::Blake3 => 1000,
        }
    }
}

/// Drive operation
#[derive(Debug)]
pub enum DriveOperation {
    /// Grove operation
    GroveOperation(GroveDbOp),
    /// Calculated cost operation
    CalculatedCostOperation(OperationCost),
    /// Pre Calculated Fee Result
    PreCalculatedFeeResult(FeeResult),
}

impl DriveOperation {
    /// Returns a list of the costs of the Drive operations.
    pub fn consume_to_fees(
        drive_operation: Vec<DriveOperation>,
        epoch: &Epoch,
    ) -> Result<Vec<FeeResult>, Error> {
        drive_operation
            .into_iter()
            .map(|operation| match operation {
                PreCalculatedFeeResult(f) => Ok(f),
                _ => {
                    let cost = operation.operation_cost()?;
                    let storage_fee = cost.storage_cost(epoch)?;
                    let processing_fee = cost.ephemeral_cost(epoch)?;
                    let (removed_bytes_from_identities, removed_bytes_from_system) =
                        match cost.storage_cost.removed_bytes {
                            NoStorageRemoval => (BTreeMap::default(), 0),
                            BasicStorageRemoval(amount) => {
                                // this is not always considered an error
                                (BTreeMap::default(), amount)
                            }
                            SectionedStorageRemoval(mut s) => {
                                let system_amount = s
                                    .remove(&Identifier::default())
                                    .map_or(0, |a| a.values().sum());
                                (s, system_amount)
                            }
                        };
                    Ok(FeeResult {
                        storage_fee,
                        processing_fee,
                        removed_bytes_from_epochs_by_identities: RemovedBytesFromEpochsByIdentities(
                            removed_bytes_from_identities,
                        ),
                        removed_bytes_from_system,
                    })
                }
            })
            .collect()
    }

    /// Returns the cost of this operation
    pub fn operation_cost(self) -> Result<OperationCost, Error> {
        match self {
            GroveOperation(_) => Err(Error::Drive(DriveError::CorruptedCodeExecution(
                "grove operations must be executed, not directly transformed to costs",
            ))),
            CalculatedCostOperation(c) => Ok(c),
            PreCalculatedFeeResult(_) => Err(Error::Drive(DriveError::CorruptedCodeExecution(
                "pre calculated fees should be requested by operation costs",
            ))),
        }
    }

    /// Filters the groveDB ops from a list of operations and puts them in a `GroveDbOpBatch`.
    pub fn combine_cost_operations(operations: &[DriveOperation]) -> OperationCost {
        let mut cost = OperationCost::default();
        operations.iter().for_each(|op| {
            if let CalculatedCostOperation(operation_cost) = op {
                cost += operation_cost.clone()
            }
        });
        cost
    }

    /// Filters the groveDB ops from a list of operations and puts them in a `GroveDbOpBatch`.
    pub fn grovedb_operations_batch(insert_operations: &[DriveOperation]) -> GroveDbOpBatch {
        let operations = insert_operations
            .iter()
            .filter_map(|op| match op {
                GroveOperation(grovedb_op) => Some(grovedb_op.clone()),
                _ => None,
            })
            .collect();
        GroveDbOpBatch::from_operations(operations)
    }

    /// Sets `GroveOperation` for inserting an empty tree at the given path and key
    pub fn for_known_path_key_empty_tree(
        path: Vec<Vec<u8>>,
        key: Vec<u8>,
        storage_flags: Option<&StorageFlags>,
    ) -> Self {
        let tree = match storage_flags {
            Some(storage_flags) => {
                Element::empty_tree_with_flags(storage_flags.to_some_element_flags())
            }
            None => Element::empty_tree(),
        };

        DriveOperation::for_known_path_key_element(path, key, tree)
    }

    /// Sets `GroveOperation` for inserting an empty tree at the given path and key
    pub fn for_estimated_path_key_empty_tree(
        path: KeyInfoPath,
        key: KeyInfo,
        storage_flags: Option<&StorageFlags>,
    ) -> Self {
        let tree = match storage_flags {
            Some(storage_flags) => {
                Element::empty_tree_with_flags(storage_flags.to_some_element_flags())
            }
            None => Element::empty_tree(),
        };

        DriveOperation::for_estimated_path_key_element(path, key, tree)
    }

    /// Sets `GroveOperation` for inserting an element at the given path and key
    pub fn for_known_path_key_element(path: Vec<Vec<u8>>, key: Vec<u8>, element: Element) -> Self {
        GroveOperation(GroveDbOp::insert_op(path, key, element))
    }

    /// Sets `GroveOperation` for inserting an element at an unknown estimated path and key
    pub fn for_estimated_path_key_element(
        path: KeyInfoPath,
        key: KeyInfo,
        element: Element,
    ) -> Self {
        GroveOperation(GroveDbOp::insert_estimated_op(path, key, element))
    }
}

/// Drive cost trait
pub trait DriveCost {
    /// Ephemeral cost
    fn ephemeral_cost(&self, epoch: &Epoch) -> Result<u64, Error>;
    /// Storage cost
    fn storage_cost(&self, epoch: &Epoch) -> Result<u64, Error>;
}

fn get_overflow_error(str: &'static str) -> Error {
    Error::Fee(FeeError::Overflow(str))
}

impl DriveCost for OperationCost {
    /// Return the ephemeral cost from the operation
    fn ephemeral_cost(&self, epoch: &Epoch) -> Result<u64, Error> {
        //todo: deal with epochs
        let OperationCost {
            seek_count,
            storage_cost,
            storage_loaded_bytes,
            hash_node_calls,
        } = self;
        let seek_cost = (*seek_count as u64)
            .checked_mul(STORAGE_SEEK_COST)
            .ok_or_else(|| get_overflow_error("seek cost overflow"))?;
        let storage_added_bytes_ephemeral_cost = (storage_cost.added_bytes as u64)
            .checked_mul(STORAGE_PROCESSING_CREDIT_PER_BYTE)
            .ok_or_else(|| get_overflow_error("storage written bytes cost overflow"))?;
        let storage_replaced_bytes_ephemeral_cost = (storage_cost.replaced_bytes as u64)
            .checked_mul(STORAGE_PROCESSING_CREDIT_PER_BYTE)
            .ok_or_else(|| get_overflow_error("storage written bytes cost overflow"))?;
        let storage_removed_bytes_ephemeral_cost =
            (storage_cost.removed_bytes.total_removed_bytes() as u64)
                .checked_mul(STORAGE_PROCESSING_CREDIT_PER_BYTE)
                .ok_or_else(|| get_overflow_error("storage written bytes cost overflow"))?;
        let storage_loaded_bytes_cost = (*storage_loaded_bytes as u64)
            .checked_mul(STORAGE_LOAD_CREDIT_PER_BYTE)
            .ok_or_else(|| get_overflow_error("storage loaded cost overflow"))?;
        let hash_node_cost = (*hash_node_calls as u64)
            .checked_mul(FunctionOp::Blake3.cost(epoch))
            .ok_or_else(|| get_overflow_error("hash node cost overflow"))?;
        seek_cost
            .checked_add(storage_added_bytes_ephemeral_cost)
            .and_then(|c| c.checked_add(storage_replaced_bytes_ephemeral_cost))
            .and_then(|c| c.checked_add(storage_loaded_bytes_cost))
            .and_then(|c| c.checked_add(storage_removed_bytes_ephemeral_cost))
            .and_then(|c| c.checked_add(hash_node_cost))
            .ok_or_else(|| get_overflow_error("ephemeral cost addition overflow"))
    }

    /// Return the storage cost from the operation
    fn storage_cost(&self, _epoch: &Epoch) -> Result<u64, Error> {
        //todo: deal with epochs
        let OperationCost { storage_cost, .. } = self;
        (storage_cost.added_bytes as u64)
            .checked_mul(STORAGE_DISK_USAGE_CREDIT_PER_BYTE)
            .ok_or_else(|| get_overflow_error("storage written bytes cost overflow"))
    }
}
