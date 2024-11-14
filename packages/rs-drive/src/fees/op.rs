use crate::util::batch::GroveDbOpBatch;
use grovedb_costs::storage_cost::removal::Identifier;
use grovedb_costs::storage_cost::removal::StorageRemovedBytes::{
    BasicStorageRemoval, NoStorageRemoval, SectionedStorageRemoval,
};

use enum_map::Enum;
use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::KeyInfoPath;
use grovedb::element::MaxReferenceHop;
use grovedb::reference_path::ReferencePathType;
use grovedb::{batch::QualifiedGroveDbOp, Element, ElementFlags};
use grovedb_costs::OperationCost;
use itertools::Itertools;

use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::get_overflow_error;
use crate::fees::op::LowLevelDriveOperation::{
    CalculatedCostOperation, FunctionOperation, GroveOperation, PreCalculatedFeeResult,
};
use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
use crate::util::storage_flags::StorageFlags;
use dpp::block::epoch::Epoch;
use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use dpp::fee::fee_result::refunds::FeeRefunds;
use dpp::fee::fee_result::FeeResult;
use dpp::fee::Credits;
use platform_version::version::fee::FeeVersion;

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

    fn block_cost(&self, fee_version: &FeeVersion) -> u64 {
        match self {
            HashFunction::Sha256 => fee_version.hashing.sha256_per_block,
            HashFunction::Sha256_2 => fee_version.hashing.sha256_per_block,
            HashFunction::Blake3 => fee_version.hashing.blake3_per_block,
            HashFunction::Sha256RipeMD160 => fee_version.hashing.sha256_per_block,
        }
    }

    fn base_cost(&self, fee_version: &FeeVersion) -> u64 {
        match self {
            HashFunction::Sha256 => fee_version.hashing.single_sha256_base,
            // It's normal that the base cost for a sha256 will have a single sha256 base
            // But it has an extra block
            HashFunction::Sha256_2 => fee_version.hashing.single_sha256_base,
            HashFunction::Blake3 => fee_version.hashing.blake3_base,
            HashFunction::Sha256RipeMD160 => fee_version.hashing.sha256_ripe_md160_base,
        }
    }
}

/// A Hash Function Operation
#[derive(Debug, PartialEq, Eq)]
pub struct FunctionOp {
    /// hash
    pub(crate) hash: HashFunction,
    /// rounds
    pub(crate) rounds: u32,
}

impl FunctionOp {
    /// The cost of the function
    fn cost(&self, fee_version: &FeeVersion) -> Credits {
        let block_cost = (self.rounds as u64).saturating_mul(self.hash.block_cost(fee_version));
        self.hash.base_cost(fee_version).saturating_add(block_cost)
    }

    /// Create a new function operation with the following hash knowing the rounds it will take
    /// in advance
    pub fn new_with_round_count(hash: HashFunction, rounds: u32) -> Self {
        FunctionOp { hash, rounds }
    }

    /// Create a new function operation with the following hash knowing the number of bytes
    /// it will hash
    pub fn new_with_byte_count(hash: HashFunction, byte_count: u16) -> Self {
        let blocks = byte_count / hash.block_size() + 1;
        let rounds = blocks + hash.rounds() - 1;
        FunctionOp {
            hash,
            rounds: rounds as u32,
        }
    }
}

/// Drive operation
#[derive(Debug, Eq, PartialEq)]
pub enum LowLevelDriveOperation {
    /// Grove operation
    GroveOperation(QualifiedGroveDbOp),
    /// A drive operation
    FunctionOperation(FunctionOp),
    /// Calculated cost operation
    CalculatedCostOperation(OperationCost),
    /// Pre Calculated Fee Result
    PreCalculatedFeeResult(FeeResult),
}

impl LowLevelDriveOperation {
    /// Returns a list of the costs of the Drive operations.
    /// Should only be used by Calculate fee
    pub fn consume_to_fees_v0(
        drive_operations: Vec<LowLevelDriveOperation>,
        epoch: &Epoch,
        epochs_per_era: u16,
        fee_version: &FeeVersion,
        previous_fee_versions: Option<&CachedEpochIndexFeeVersions>,
    ) -> Result<Vec<FeeResult>, Error> {
        drive_operations
            .into_iter()
            .map(|operation| match operation {
                PreCalculatedFeeResult(f) => Ok(f),
                FunctionOperation(op) => Ok(FeeResult {
                    processing_fee: op.cost(fee_version),
                    ..Default::default()
                }),
                _ => {
                    let cost = operation.operation_cost()?;
                    // There is no need for a checked multiply here because added bytes are u64 and 
                    // storage disk usage credit per byte should never be high enough to cause an overflow
                    let storage_fee = cost.storage_cost.added_bytes as u64 * fee_version.storage.storage_disk_usage_credit_per_byte;
                    let processing_fee = cost.ephemeral_cost(fee_version)?;
                    let (fee_refunds, removed_bytes_from_system) =
                        match cost.storage_cost.removed_bytes {
                            NoStorageRemoval => (FeeRefunds::default(), 0),
                            BasicStorageRemoval(amount) => {
                                // this is not always considered an error
                                (FeeRefunds::default(), amount)
                            }
                            SectionedStorageRemoval(mut removal_per_epoch_by_identifier) => {
                                let previous_fee_versions = previous_fee_versions.ok_or(Error::Drive(DriveError::CorruptedCodeExecution("expected previous epoch index fee versions to be able to offer refunds")))?;
                                let system_amount = removal_per_epoch_by_identifier
                                    .remove(&Identifier::default())
                                    .map_or(0, |a| a.values().sum());

                                (
                                    FeeRefunds::from_storage_removal(
                                        removal_per_epoch_by_identifier,
                                        epoch.index,
                                        epochs_per_era,
                                        previous_fee_versions,
                                    )?,
                                    system_amount,
                                )
                            }
                        };
                    Ok(FeeResult {
                        storage_fee,
                        processing_fee,
                        fee_refunds,
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
    pub fn combine_cost_operations(operations: &[LowLevelDriveOperation]) -> OperationCost {
        let mut cost = OperationCost::default();
        operations.iter().for_each(|op| {
            if let CalculatedCostOperation(operation_cost) = op {
                cost += operation_cost.clone()
            }
        });
        cost
    }

    /// Filters the groveDB ops from a list of operations and puts them in a `GroveDbOpBatch`.
    pub fn grovedb_operations_batch(
        insert_operations: &[LowLevelDriveOperation],
    ) -> GroveDbOpBatch {
        let operations = insert_operations
            .iter()
            .filter_map(|op| match op {
                GroveOperation(grovedb_op) => Some(grovedb_op.clone()),
                _ => None,
            })
            .collect();
        GroveDbOpBatch::from_operations(operations)
    }

    /// Filters the groveDB ops from a list of operations and puts them in a `GroveDbOpBatch`.
    pub fn grovedb_operations_batch_consume(
        insert_operations: Vec<LowLevelDriveOperation>,
    ) -> GroveDbOpBatch {
        let operations = insert_operations
            .into_iter()
            .filter_map(|op| match op {
                GroveOperation(grovedb_op) => Some(grovedb_op),
                _ => None,
            })
            .collect();
        GroveDbOpBatch::from_operations(operations)
    }

    /// Filters the groveDB ops from a list of operations and puts them in a `GroveDbOpBatch`.
    pub fn grovedb_operations_batch_consume_with_leftovers(
        insert_operations: Vec<LowLevelDriveOperation>,
    ) -> (GroveDbOpBatch, Vec<LowLevelDriveOperation>) {
        let (grove_operations, other_operations): (Vec<_>, Vec<_>) =
            insert_operations.into_iter().partition_map(|op| match op {
                GroveOperation(grovedb_op) => itertools::Either::Left(grovedb_op),
                _ => itertools::Either::Right(op),
            });

        (
            GroveDbOpBatch::from_operations(grove_operations),
            other_operations,
        )
    }

    /// Filters the groveDB ops from a list of operations and collects them in a `Vec<QualifiedGroveDbOp>`.
    pub fn grovedb_operations_consume(
        insert_operations: Vec<LowLevelDriveOperation>,
    ) -> Vec<QualifiedGroveDbOp> {
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

        LowLevelDriveOperation::insert_for_known_path_key_element(path, key, tree)
    }

    /// Sets `GroveOperation` for inserting an empty sum tree at the given path and key
    pub fn for_known_path_key_empty_sum_tree(
        path: Vec<Vec<u8>>,
        key: Vec<u8>,
        storage_flags: Option<&StorageFlags>,
    ) -> Self {
        let tree = match storage_flags {
            Some(storage_flags) => {
                Element::empty_sum_tree_with_flags(storage_flags.to_some_element_flags())
            }
            None => Element::empty_sum_tree(),
        };

        LowLevelDriveOperation::insert_for_known_path_key_element(path, key, tree)
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

        LowLevelDriveOperation::insert_for_estimated_path_key_element(path, key, tree)
    }

    /// Sets `GroveOperation` for inserting an element at the given path and key
    pub fn insert_for_known_path_key_element(
        path: Vec<Vec<u8>>,
        key: Vec<u8>,
        element: Element,
    ) -> Self {
        GroveOperation(QualifiedGroveDbOp::insert_or_replace_op(path, key, element))
    }

    /// Sets `GroveOperation` for replacement of an element at the given path and key
    pub fn replace_for_known_path_key_element(
        path: Vec<Vec<u8>>,
        key: Vec<u8>,
        element: Element,
    ) -> Self {
        GroveOperation(QualifiedGroveDbOp::replace_op(path, key, element))
    }

    /// Sets `GroveOperation` for patching of an element at the given path and key
    /// This is different from replacement which does not add or delete bytes
    pub fn patch_for_known_path_key_element(
        path: Vec<Vec<u8>>,
        key: Vec<u8>,
        element: Element,
        change_in_bytes: i32,
    ) -> Self {
        GroveOperation(QualifiedGroveDbOp::patch_op(
            path,
            key,
            element,
            change_in_bytes,
        ))
    }

    /// Sets `GroveOperation` for inserting an element at an unknown estimated path and key
    pub fn insert_for_estimated_path_key_element(
        path: KeyInfoPath,
        key: KeyInfo,
        element: Element,
    ) -> Self {
        GroveOperation(QualifiedGroveDbOp::insert_estimated_op(path, key, element))
    }

    /// Sets `GroveOperation` for replacement of an element at an unknown estimated path and key
    pub fn replace_for_estimated_path_key_element(
        path: KeyInfoPath,
        key: KeyInfo,
        element: Element,
    ) -> Self {
        GroveOperation(QualifiedGroveDbOp::replace_estimated_op(path, key, element))
    }

    /// Sets `GroveOperation` for refresh of a reference at the given path and key
    pub fn refresh_reference_for_known_path_key_reference_info(
        path: Vec<Vec<u8>>,
        key: Vec<u8>,
        reference_path_type: ReferencePathType,
        max_reference_hop: MaxReferenceHop,
        flags: Option<ElementFlags>,
        trust_refresh_reference: bool,
    ) -> Self {
        GroveOperation(QualifiedGroveDbOp::refresh_reference_op(
            path,
            key,
            reference_path_type,
            max_reference_hop,
            flags,
            trust_refresh_reference,
        ))
    }
}

/// Drive cost trait
pub trait DriveCost {
    /// Ephemeral cost
    fn ephemeral_cost(&self, fee_version: &FeeVersion) -> Result<u64, Error>;
}

impl DriveCost for OperationCost {
    /// Return the ephemeral cost from the operation
    fn ephemeral_cost(&self, fee_version: &FeeVersion) -> Result<Credits, Error> {
        let OperationCost {
            seek_count,
            storage_cost,
            storage_loaded_bytes,
            hash_node_calls,
        } = self;
        let epoch_cost_for_processing_credit_per_byte =
            fee_version.storage.storage_processing_credit_per_byte;
        let seek_cost = (*seek_count as u64)
            .checked_mul(fee_version.storage.storage_seek_cost)
            .ok_or_else(|| get_overflow_error("seek cost overflow"))?;
        let storage_added_bytes_ephemeral_cost = (storage_cost.added_bytes as u64)
            .checked_mul(epoch_cost_for_processing_credit_per_byte)
            .ok_or_else(|| get_overflow_error("storage written bytes cost overflow"))?;
        let storage_replaced_bytes_ephemeral_cost = (storage_cost.replaced_bytes as u64)
            .checked_mul(epoch_cost_for_processing_credit_per_byte)
            .ok_or_else(|| get_overflow_error("storage written bytes cost overflow"))?;
        let storage_removed_bytes_ephemeral_cost =
            (storage_cost.removed_bytes.total_removed_bytes() as u64)
                .checked_mul(epoch_cost_for_processing_credit_per_byte)
                .ok_or_else(|| get_overflow_error("storage written bytes cost overflow"))?;
        // not accessible
        let storage_loaded_bytes_cost = (*storage_loaded_bytes as u64)
            .checked_mul(fee_version.storage.storage_load_credit_per_byte)
            .ok_or_else(|| get_overflow_error("storage loaded cost overflow"))?;

        // There is one block per hash node call
        let blake3_total = fee_version.hashing.blake3_base + fee_version.hashing.blake3_per_block;
        // this can't overflow
        let hash_node_cost = blake3_total * (*hash_node_calls as u64);
        seek_cost
            .checked_add(storage_added_bytes_ephemeral_cost)
            .and_then(|c| c.checked_add(storage_replaced_bytes_ephemeral_cost))
            .and_then(|c| c.checked_add(storage_loaded_bytes_cost))
            .and_then(|c| c.checked_add(storage_removed_bytes_ephemeral_cost))
            .and_then(|c| c.checked_add(hash_node_cost))
            .ok_or_else(|| get_overflow_error("ephemeral cost addition overflow"))
    }
}
