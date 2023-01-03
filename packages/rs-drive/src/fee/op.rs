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

use costs::OperationCost;
use enum_map::Enum;
use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::KeyInfoPath;
use grovedb::{batch::GroveDbOp, Element};

use crate::drive::flags::StorageFlags;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::default_costs::{
    STORAGE_DISK_USAGE_CREDIT_PER_BYTE, STORAGE_LOAD_CREDIT_PER_BYTE,
    STORAGE_PROCESSING_CREDIT_PER_BYTE, STORAGE_SEEK_COST,
};
use crate::fee::op::DriveOperation::{
    CalculatedCostOperation, FunctionOperation, GroveOperation, PreCalculatedFeeResult,
};
use crate::fee::result::refunds::FeeRefunds;
use crate::fee::{get_overflow_error, FeeResult};
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

/// Supported Hash Functions
#[derive(Debug, Enum, PartialEq, Eq)]
pub enum HashFunction {
    /// Used for crypto addresses
    Sha256RipeMD160,
    /// Single Sha256
    Sha256,
    /// Double Sha256
    Sha256_2,
    /// Single Blake3
    Blake3,
}

impl HashFunction {
    fn block_size(&self) -> u16 {
        match self {
            HashFunction::Sha256 => 64,
            HashFunction::Sha256_2 => 64,
            HashFunction::Blake3 => 64,
            HashFunction::Sha256RipeMD160 => 64,
        }
    }

    fn rounds(&self) -> u16 {
        match self {
            HashFunction::Sha256 => 1,
            HashFunction::Sha256_2 => 2,
            HashFunction::Blake3 => 1,
            HashFunction::Sha256RipeMD160 => 1,
        }
    }

    //todo: put real costs in
    fn base_cost(&self, _epoch: &Epoch) -> u64 {
        match self {
            HashFunction::Sha256 => 30,
            HashFunction::Sha256_2 => 30,
            HashFunction::Blake3 => 30,
            HashFunction::Sha256RipeMD160 => 30,
        }
    }
}

/// A Hash Function Operation
#[derive(Debug, PartialEq, Eq)]
pub struct FunctionOp {
    pub(crate) hash: HashFunction,
    pub(crate) rounds: u16,
}

impl FunctionOp {
    /// The cost of the function
    fn cost(&self, epoch: &Epoch) -> u64 {
        self.rounds as u64 * self.hash.base_cost(epoch)
    }

    /// Create a new function operation with the following hash knowing the rounds it will take
    /// in advance
    pub fn new_with_round_count(hash: HashFunction, rounds: u16) -> Self {
        FunctionOp { hash, rounds }
    }

    /// Create a new function operation with the following hash knowing the number of bytes
    /// it will hash
    pub fn new_with_byte_count(hash: HashFunction, byte_count: u16) -> Self {
        let blocks = byte_count / hash.block_size() + 1;
        let rounds = blocks + hash.rounds() - 1;
        FunctionOp { hash, rounds }
    }
}

/// Drive operation
#[derive(Debug, Eq, PartialEq)]
pub enum DriveOperation {
    /// Grove operation
    GroveOperation(GroveDbOp),
    /// A drive operation
    FunctionOperation(FunctionOp),
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
                FunctionOperation(op) => Ok(FeeResult {
                    processing_fee: op.cost(epoch),
                    ..Default::default()
                }),
                _ => {
                    let cost = operation.operation_cost()?;
                    let storage_fee = cost.storage_cost(epoch)?;
                    let processing_fee = cost.ephemeral_cost(epoch)?;
                    let (removed_bytes_from_epochs_by_identities, removed_bytes_from_system) =
                        match cost.storage_cost.removed_bytes {
                            NoStorageRemoval => (FeeRefunds::default(), 0),
                            BasicStorageRemoval(amount) => {
                                // this is not always considered an error
                                (FeeRefunds::default(), amount)
                            }
                            SectionedStorageRemoval(mut removal_per_epoch_by_identifier) => {
                                let system_amount = removal_per_epoch_by_identifier
                                    .remove(&Identifier::default())
                                    .map_or(0, |a| a.values().sum());

                                (
                                    FeeRefunds::from_storage_removal(
                                        removal_per_epoch_by_identifier,
                                    )?,
                                    system_amount,
                                )
                            }
                        };
                    Ok(FeeResult {
                        storage_fee,
                        processing_fee,
                        fee_refunds: removed_bytes_from_epochs_by_identities,
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
                "pre calculated fees should not be requested by operation costs",
            ))),
            FunctionOperation(_) => Err(Error::Drive(DriveError::CorruptedCodeExecution(
                "function operations should not be requested by operation costs",
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

    /// Filters the groveDB ops from a list of operations and collects them in a `Vec<GroveDbOp>`.
    pub fn grovedb_operations_consume(insert_operations: Vec<DriveOperation>) -> Vec<GroveDbOp> {
        insert_operations
            .into_iter()
            .filter_map(|op| match op {
                GroveOperation(grovedb_op) => Some(grovedb_op),
                _ => None,
            })
            .collect()
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

        DriveOperation::insert_for_known_path_key_element(path, key, tree)
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

        DriveOperation::insert_for_estimated_path_key_element(path, key, tree)
    }

    /// Sets `GroveOperation` for inserting an element at the given path and key
    pub fn insert_for_known_path_key_element(
        path: Vec<Vec<u8>>,
        key: Vec<u8>,
        element: Element,
    ) -> Self {
        GroveOperation(GroveDbOp::insert_op(path, key, element))
    }

    /// Sets `GroveOperation` for replacement of an element at the given path and key
    pub fn replace_for_known_path_key_element(
        path: Vec<Vec<u8>>,
        key: Vec<u8>,
        element: Element,
    ) -> Self {
        GroveOperation(GroveDbOp::replace_op(path, key, element))
    }

    /// Sets `GroveOperation` for inserting an element at an unknown estimated path and key
    pub fn insert_for_estimated_path_key_element(
        path: KeyInfoPath,
        key: KeyInfo,
        element: Element,
    ) -> Self {
        GroveOperation(GroveDbOp::insert_estimated_op(path, key, element))
    }

    /// Sets `GroveOperation` for replacement of an element at an unknown estimated path and key
    pub fn replace_for_estimated_path_key_element(
        path: KeyInfoPath,
        key: KeyInfo,
        element: Element,
    ) -> Self {
        GroveOperation(GroveDbOp::replace_estimated_op(path, key, element))
    }
}

/// Drive cost trait
pub trait DriveCost {
    /// Ephemeral cost
    fn ephemeral_cost(&self, epoch: &Epoch) -> Result<u64, Error>;
    /// Storage cost
    fn storage_cost(&self, epoch: &Epoch) -> Result<u64, Error>;
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
        // this can't overflow
        let hash_node_cost =
            FunctionOp::new_with_round_count(HashFunction::Blake3, *hash_node_calls).cost(epoch);
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
