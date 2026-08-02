use crate::util::batch::GroveDbOpBatch;
use grovedb_costs::storage_cost::removal::Identifier;
use grovedb_costs::storage_cost::removal::StorageRemovedBytes::{
    BasicStorageRemoval, NoStorageRemoval, SectionedStorageRemoval,
};
use std::collections::BTreeMap;

use enum_map::Enum;
use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::KeyInfoPath;
use grovedb::element::reference_path::ReferencePathType;
use grovedb::element::MaxReferenceHop;
use grovedb::{batch::QualifiedGroveDbOp, Element, ElementFlags, TreeType};
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

                                let system_amount = removal_per_epoch_by_identifier
                                    .remove(&Identifier::default())
                                    .map_or(0, |a| a.values().sum());
                                if fee_version.fee_version_number == 1 {
                                    (
                                        FeeRefunds::from_storage_removal(
                                            removal_per_epoch_by_identifier,
                                            epoch.index,
                                            epochs_per_era,
                                            &BTreeMap::default(),
                                        )?,
                                        system_amount,
                                    )
                                } else {
                                    let previous_fee_versions = previous_fee_versions.ok_or(Error::Drive(DriveError::CorruptedCodeExecution("expected previous epoch index fee versions to be able to offer refunds")))?;
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

    /// Sets `GroveOperation` for inserting an empty sum tree at the given path and key
    pub fn for_known_path_key_empty_big_sum_tree(
        path: Vec<Vec<u8>>,
        key: Vec<u8>,
        storage_flags: Option<&StorageFlags>,
    ) -> Self {
        let tree = match storage_flags {
            Some(storage_flags) => {
                Element::new_big_sum_tree_with_flags(None, storage_flags.to_some_element_flags())
            }
            None => Element::empty_big_sum_tree(),
        };

        LowLevelDriveOperation::insert_for_known_path_key_element(path, key, tree)
    }

    /// Sets `GroveOperation` for inserting an empty count tree at the given path and key
    pub fn for_known_path_key_empty_count_tree(
        path: Vec<Vec<u8>>,
        key: Vec<u8>,
        storage_flags: Option<&StorageFlags>,
    ) -> Self {
        let tree = match storage_flags {
            Some(storage_flags) => {
                Element::new_count_tree_with_flags(None, storage_flags.to_some_element_flags())
            }
            None => Element::empty_count_tree(),
        };

        LowLevelDriveOperation::insert_for_known_path_key_element(path, key, tree)
    }

    /// Sets `GroveOperation` for inserting an empty count tree at the given path and key
    pub fn for_known_path_key_empty_count_sum_tree(
        path: Vec<Vec<u8>>,
        key: Vec<u8>,
        storage_flags: Option<&StorageFlags>,
    ) -> Self {
        let tree = match storage_flags {
            Some(storage_flags) => {
                Element::new_count_sum_tree_with_flags(None, storage_flags.to_some_element_flags())
            }
            None => Element::new_count_sum_tree(None),
        };

        LowLevelDriveOperation::insert_for_known_path_key_element(path, key, tree)
    }

    /// Sets `GroveOperation` for inserting an empty `NormalTree` wrapped in
    /// `Element::NonCounted` at the given path and key. The wrapper makes
    /// the inserted subtree contribute 0 to a parent count tree's aggregate
    /// (per grovedb #654). Used by the index-walker for sibling continuations
    /// inside a `range_countable` value tree, so e.g. a compound `byColorShape`
    /// continuation under a `byColor` value tree (which is a `CountTree`)
    /// doesn't pollute the byColor count.
    pub fn for_known_path_key_empty_non_counted_normal_tree(
        path: Vec<Vec<u8>>,
        key: Vec<u8>,
        storage_flags: Option<&StorageFlags>,
    ) -> Self {
        Self::for_known_path_key_empty_non_counted_tree(
            path,
            key,
            TreeType::NormalTree,
            storage_flags,
        )
        .expect("NormalTree NonCounted wrapping never fails")
    }

    /// Sets `GroveOperation` for inserting an empty tree of the given
    /// `tree_type` wrapped in `Element::NonCounted`. The wrapper makes the
    /// inserted subtree contribute 0 to a parent count tree's aggregate
    /// count (per grovedb #654), regardless of the inner tree variant.
    ///
    /// Used by the index walker for sibling continuations inside a
    /// `range_countable` value tree (a `CountTree`). Most continuations are
    /// plain `NormalTree`, but in nested-`range_countable` cases (e.g. an
    /// index `[color]` is range-countable AND a deeper compound index
    /// `[color, size]` is also range-countable), the continuation
    /// property-name tree at `"size"` is itself a `ProvableCountTree` and
    /// must still contribute 0 to the parent `<c1>` `CountTree`.
    ///
    /// Returns an error for tree variants whose `NonCounted` wrapping
    /// hasn't been validated end-to-end yet (currently anything outside
    /// `NormalTree` / `CountTree` / `ProvableCountTree`).
    pub fn for_known_path_key_empty_non_counted_tree(
        path: Vec<Vec<u8>>,
        key: Vec<u8>,
        tree_type: TreeType,
        storage_flags: Option<&StorageFlags>,
    ) -> Result<Self, Error> {
        // Per grovedb PR 670, `Element::new_non_counted` only wraps
        // count-bearing trees — provable-count parents reject the
        // wrapper at the merk-layer insert guard, and sum-bearing
        // trees use dedicated `NotSummed` / `NotCountedOrSummed`
        // wrappers (see [`Self::for_known_path_key_empty_not_summed_tree`]
        // / [`Self::for_known_path_key_empty_not_counted_or_summed_tree`]).
        let element_flags = storage_flags.map(|s| s.to_element_flags());
        let inner = match tree_type {
            TreeType::NormalTree => Element::empty_tree_with_flags(element_flags),
            TreeType::CountTree => Element::empty_count_tree_with_flags(element_flags),
            TreeType::ProvableCountTree => {
                Element::empty_provable_count_tree_with_flags(element_flags)
            }
            _ => {
                return Err(Error::Drive(DriveError::NotSupported(
                    "NonCounted-wrapping is only supported for NormalTree, CountTree, and \
                     ProvableCountTree. For sum-bearing continuations under a sum or \
                     count+sum parent, use `for_known_path_key_empty_not_summed_tree` or \
                     `for_known_path_key_empty_not_counted_or_summed_tree` instead.",
                )));
            }
        };
        // Propagate the grovedb error as a typed Drive error rather
        // than `.expect`-ing. The match above already restricts `inner`
        // to NormalTree / CountTree / ProvableCountTree — all of which
        // `new_non_counted` accepts at the head this PR pins
        // (`packages/rs-drive/Cargo.toml`'s grovedb rev) — so in
        // practice this `?` is a no-op. Keeping it as `?` means a
        // future grovedb bump that tightens `new_non_counted`'s
        // accepted-variant set lands a typed `Error::GroveDB` at the
        // call site instead of a runtime panic. The `?` conversion
        // uses `impl From<grovedb::element::error::ElementError>`
        // defined in `crate::error::mod.rs`.
        let tree = Element::new_non_counted(inner)?;
        Ok(LowLevelDriveOperation::insert_for_known_path_key_element(
            path, key, tree,
        ))
    }

    /// Sets `GroveOperation` for inserting an empty sum-bearing tree
    /// wrapped in `Element::NotSummed` (grovedb PR 670). The wrapper
    /// makes the inserted subtree contribute 0 to a parent sum tree's
    /// running sum while still allowing any count it carries to
    /// propagate normally. Used by the index walker for continuation
    /// property-name trees inside a `summable`-but-not-`countable`
    /// value tree. For continuations under a count+sum parent, use
    /// [`Self::for_known_path_key_empty_not_counted_or_summed_tree`].
    pub fn for_known_path_key_empty_not_summed_tree(
        path: Vec<Vec<u8>>,
        key: Vec<u8>,
        tree_type: TreeType,
        storage_flags: Option<&StorageFlags>,
    ) -> Result<Self, Error> {
        let element_flags = storage_flags.map(|s| s.to_element_flags());
        let inner = match tree_type {
            TreeType::SumTree => Element::empty_sum_tree_with_flags(element_flags),
            TreeType::BigSumTree => Element::empty_big_sum_tree_with_flags(element_flags),
            TreeType::ProvableSumTree => Element::empty_provable_sum_tree_with_flags(element_flags),
            TreeType::CountSumTree => Element::empty_count_sum_tree_with_flags(element_flags),
            TreeType::ProvableCountSumTree => {
                Element::empty_provable_count_sum_tree_with_flags(element_flags)
            }
            TreeType::ProvableCountProvableSumTree => {
                Element::empty_provable_count_provable_sum_tree_with_flags(element_flags)
            }
            _ => {
                return Err(Error::Drive(DriveError::NotSupported(
                    "NotSummed-wrapping is only supported for the six sum-bearing tree \
                     variants (SumTree, BigSumTree, ProvableSumTree, CountSumTree, \
                     ProvableCountSumTree, ProvableCountProvableSumTree).",
                )));
            }
        };
        let tree = Element::new_not_summed(inner).map_err(|_| {
            Error::Drive(DriveError::NotSupported(
                "Element::new_not_summed rejected the inner tree (unreachable given the \
                 match above).",
            ))
        })?;
        Ok(LowLevelDriveOperation::insert_for_known_path_key_element(
            path, key, tree,
        ))
    }

    /// Sets `GroveOperation` for inserting an empty inner tree wrapped
    /// in the wrapper variant appropriate for an `aggregating_parent_tree_type`.
    ///
    /// Dispatcher around the three concrete wrapper helpers
    /// ([`Self::for_known_path_key_empty_non_counted_tree`] /
    /// [`Self::for_known_path_key_empty_not_summed_tree`] /
    /// [`Self::for_known_path_key_empty_not_counted_or_summed_tree`])
    /// keyed on **the parent's** tree type — the wrapper exists to
    /// suppress contribution to the parent's aggregate, so the parent's
    /// kind picks the wrapper:
    /// - Pure count parents (`CountTree` / `ProvableCountTree`) →
    ///   `Element::NonCounted`.
    /// - Pure sum parents (`SumTree` / `BigSumTree` / `ProvableSumTree`)
    ///   → `Element::NotSummed`.
    /// - Combined count+sum parents (`CountSumTree` /
    ///   `ProvableCountSumTree` / `ProvableCountProvableSumTree`) →
    ///   `Element::NotCountedOrSummed`.
    /// - Non-aggregating parents (`NormalTree`, etc.) — no wrapping
    ///   needed; caller should use
    ///   [`crate::fees::op::LowLevelDriveOperationTreeTypeConverter::empty_tree_operation_for_known_path_key`]
    ///   directly. This dispatcher rejects them with `NotSupported`
    ///   so an upstream bug surfaces immediately rather than silently
    ///   emitting an unwrapped child that pollutes a future parent.
    ///
    /// `inner_tree_type` is the tree variant being inserted under the
    /// parent — typically a property-name continuation tree
    /// (`NormalTree` / `CountTree` / `ProvableCountTree` / their
    /// sum-bearing siblings).
    pub fn wrap_in_non_aggregated_for_parent_tree_type(
        path: Vec<Vec<u8>>,
        key: Vec<u8>,
        aggregating_parent_tree_type: TreeType,
        inner_tree_type: TreeType,
        storage_flags: Option<&StorageFlags>,
    ) -> Result<Self, Error> {
        match aggregating_parent_tree_type {
            // Count-only parents — wrap so the inner contributes 0 to
            // the parent's count. The inner can be plain or itself
            // count-bearing; the helper validates accepted variants.
            TreeType::CountTree | TreeType::ProvableCountTree => {
                Self::for_known_path_key_empty_non_counted_tree(
                    path,
                    key,
                    inner_tree_type,
                    storage_flags,
                )
            }
            // Sum-only parents — wrap so the inner contributes 0 to
            // the parent's sum. Inner must be sum-bearing (see
            // `for_known_path_key_empty_not_summed_tree`'s accepted set).
            TreeType::SumTree | TreeType::BigSumTree | TreeType::ProvableSumTree => {
                Self::for_known_path_key_empty_not_summed_tree(
                    path,
                    key,
                    inner_tree_type,
                    storage_flags,
                )
            }
            // Combined count+sum parents — wrap so both axes contribute
            // 0. Inner must be sum-bearing.
            TreeType::CountSumTree
            | TreeType::ProvableCountSumTree
            | TreeType::ProvableCountProvableSumTree => {
                Self::for_known_path_key_empty_not_counted_or_summed_tree(
                    path,
                    key,
                    inner_tree_type,
                    storage_flags,
                )
            }
            _ => Err(Error::Drive(DriveError::NotSupported(
                "wrap_in_non_aggregated_for_parent_tree_type called with a non-aggregating \
                 parent tree type — caller should use the unwrapped \
                 `empty_tree_operation_for_known_path_key` path instead.",
            ))),
        }
    }

    /// Sets `GroveOperation` for inserting an empty sum-bearing tree
    /// wrapped in `Element::NotCountedOrSummed` (grovedb PR 670).
    /// Suppresses BOTH count and sum propagation to the parent — used
    /// for continuation property-name trees under a count+sum
    /// aggregating value tree (CountSumTree / ProvableCountSumTree /
    /// ProvableCountProvableSumTree). Same accepted inner-type set as
    /// [`Self::for_known_path_key_empty_not_summed_tree`].
    pub fn for_known_path_key_empty_not_counted_or_summed_tree(
        path: Vec<Vec<u8>>,
        key: Vec<u8>,
        tree_type: TreeType,
        storage_flags: Option<&StorageFlags>,
    ) -> Result<Self, Error> {
        let element_flags = storage_flags.map(|s| s.to_element_flags());
        let inner = match tree_type {
            TreeType::SumTree => Element::empty_sum_tree_with_flags(element_flags),
            TreeType::BigSumTree => Element::empty_big_sum_tree_with_flags(element_flags),
            TreeType::ProvableSumTree => Element::empty_provable_sum_tree_with_flags(element_flags),
            TreeType::CountSumTree => Element::empty_count_sum_tree_with_flags(element_flags),
            TreeType::ProvableCountSumTree => {
                Element::empty_provable_count_sum_tree_with_flags(element_flags)
            }
            TreeType::ProvableCountProvableSumTree => {
                Element::empty_provable_count_provable_sum_tree_with_flags(element_flags)
            }
            _ => {
                return Err(Error::Drive(DriveError::NotSupported(
                    "NotCountedOrSummed-wrapping is only supported for the six sum-bearing \
                     tree variants — see `for_known_path_key_empty_not_summed_tree`.",
                )));
            }
        };
        let tree = Element::new_not_counted_or_summed(inner).map_err(|_| {
            Error::Drive(DriveError::NotSupported(
                "Element::new_not_counted_or_summed rejected the inner tree (unreachable \
                 given the match above).",
            ))
        })?;
        Ok(LowLevelDriveOperation::insert_for_known_path_key_element(
            path, key, tree,
        ))
    }

    /// Sets `GroveOperation` for inserting an empty continuation tree under an
    /// aggregating parent so it contributes **zero to every axis the parent
    /// aggregates** — the v2 index walkers' replacement for
    /// [`Self::wrap_in_non_aggregated_for_parent_tree_type`].
    ///
    /// The v0 dispatcher above covers only the diagonal of the parent×inner
    /// matrix (count parent + count-ish inner, sum parent + sum-bearing
    /// inner, count+sum parent + sum-bearing inner) and errors on everything
    /// else, which made shared-prefix aggregate contracts (e.g. a summable
    /// `[a]` next to a plain compound `[a, b]`) reject every document
    /// insert. This dispatcher completes the matrix using only combinations
    /// grovedb accepts:
    /// - `CountTree` parent → `Element::NonCounted(inner)` for any inner
    ///   tree variant (a `NonCounted` child contributes 0 to the count; the
    ///   parent has no sum axis).
    /// - `CountSumTree` parent → sum-bearing inner:
    ///   `Element::NotCountedOrSummed(inner)`; non-sum inner:
    ///   `Element::NonCounted(inner)` (count suppressed by the wrapper, sum
    ///   contribution of a non-sum inner is 0 by definition —
    ///   `sum_value_or_default()` returns 0 for it).
    /// - `SumTree` / `BigSumTree` / `ProvableSumTree` parent → sum-bearing
    ///   inner: `Element::NotSummed(inner)`; non-sum inner: **no wrapper at
    ///   all** — a non-sum child already contributes 0 to a sum-only
    ///   parent, and grovedb has no `NotSummed(non-sum)` form.
    /// - Provable count-bearing parents (`ProvableCountTree` /
    ///   `ProvableCountSumTree` / `ProvableCountProvableSumTree`) →
    ///   `NotSupported`. These commit their count into every node hash and
    ///   reject count-suppressed children at grovedb's insert guards
    ///   (`TreeType::accepts_non_counted_children` /
    ///   `accepts_not_counted_or_summed_children`), so callers must demote
    ///   the parent first — see
    ///   `crate::drive::document::index_level_tree_types`.
    /// - Non-aggregating parents → `NotSupported`; use
    ///   [`crate::fees::op::LowLevelDriveOperationTreeTypeConverter::empty_tree_operation_for_known_path_key`]
    ///   directly.
    ///
    /// Only reachable from the v2 index walkers (platform-version gated);
    /// the v0 dispatcher stays byte-identical for the frozen v0/v1 walkers.
    pub fn for_known_path_key_empty_tree_contributing_zero_to_parent(
        path: Vec<Vec<u8>>,
        key: Vec<u8>,
        aggregating_parent_tree_type: TreeType,
        inner_tree_type: TreeType,
        storage_flags: Option<&StorageFlags>,
    ) -> Result<Self, Error> {
        let inner_is_sum_bearing = matches!(
            inner_tree_type,
            TreeType::SumTree
                | TreeType::BigSumTree
                | TreeType::ProvableSumTree
                | TreeType::CountSumTree
                | TreeType::ProvableCountSumTree
                | TreeType::ProvableCountProvableSumTree
        );
        match aggregating_parent_tree_type {
            TreeType::CountTree => Self::for_known_path_key_empty_non_counted_any_tree(
                path,
                key,
                inner_tree_type,
                storage_flags,
            ),
            TreeType::CountSumTree => {
                if inner_is_sum_bearing {
                    Self::for_known_path_key_empty_not_counted_or_summed_tree(
                        path,
                        key,
                        inner_tree_type,
                        storage_flags,
                    )
                } else {
                    Self::for_known_path_key_empty_non_counted_any_tree(
                        path,
                        key,
                        inner_tree_type,
                        storage_flags,
                    )
                }
            }
            TreeType::SumTree | TreeType::BigSumTree | TreeType::ProvableSumTree => {
                if inner_is_sum_bearing {
                    Self::for_known_path_key_empty_not_summed_tree(
                        path,
                        key,
                        inner_tree_type,
                        storage_flags,
                    )
                } else {
                    inner_tree_type.empty_tree_operation_for_known_path_key(
                        path,
                        key,
                        storage_flags,
                    )
                }
            }
            TreeType::ProvableCountTree
            | TreeType::ProvableCountSumTree
            | TreeType::ProvableCountProvableSumTree => {
                Err(Error::Drive(DriveError::NotSupported(
                    "provable count-bearing parents cannot host zero-contributing children — \
                 grovedb commits their count into every node hash and rejects NonCounted / \
                 NotCountedOrSummed children; the index walker must demote such value trees \
                 to CountSumTree before hanging continuations under them (see \
                 index_level_tree_types_with_continuation_demotion).",
                )))
            }
            _ => Err(Error::Drive(DriveError::NotSupported(
                "for_known_path_key_empty_tree_contributing_zero_to_parent called with a \
                 non-aggregating parent tree type — caller should use the unwrapped \
                 `empty_tree_operation_for_known_path_key` path instead.",
            ))),
        }
    }

    /// Sets `GroveOperation` for inserting an empty tree of any of the nine
    /// standard merk tree variants wrapped in `Element::NonCounted`.
    /// Extends [`Self::for_known_path_key_empty_non_counted_tree`]'s
    /// accepted set (`NormalTree` / `CountTree` / `ProvableCountTree`) with
    /// the six sum-bearing variants: `Element::new_non_counted` accepts any
    /// non-wrapper inner, and under the only parents the v2 walkers use it
    /// for (`CountTree`, `CountSumTree` — both without per-node count
    /// commitments) the wrapper suppresses the count contribution while a
    /// sum-bearing inner's sum still propagates on the parent's sum axis if
    /// it has one — which is exactly the v0-diagonal behavior for
    /// count-only parents, and unreachable for `CountSumTree` parents (the
    /// zero-contribution dispatcher routes their sum-bearing inners through
    /// `NotCountedOrSummed` instead).
    ///
    /// Kept separate from the frozen v0 helper so pre-v14 consensus
    /// behavior stays byte-identical.
    pub fn for_known_path_key_empty_non_counted_any_tree(
        path: Vec<Vec<u8>>,
        key: Vec<u8>,
        tree_type: TreeType,
        storage_flags: Option<&StorageFlags>,
    ) -> Result<Self, Error> {
        let element_flags = storage_flags.map(|s| s.to_element_flags());
        let inner = match tree_type {
            TreeType::NormalTree => Element::empty_tree_with_flags(element_flags),
            TreeType::SumTree => Element::empty_sum_tree_with_flags(element_flags),
            TreeType::BigSumTree => Element::empty_big_sum_tree_with_flags(element_flags),
            TreeType::CountTree => Element::empty_count_tree_with_flags(element_flags),
            TreeType::CountSumTree => Element::empty_count_sum_tree_with_flags(element_flags),
            TreeType::ProvableCountTree => {
                Element::empty_provable_count_tree_with_flags(element_flags)
            }
            TreeType::ProvableCountSumTree => {
                Element::empty_provable_count_sum_tree_with_flags(element_flags)
            }
            TreeType::ProvableSumTree => Element::empty_provable_sum_tree_with_flags(element_flags),
            TreeType::ProvableCountProvableSumTree => {
                Element::empty_provable_count_provable_sum_tree_with_flags(element_flags)
            }
            _ => {
                return Err(Error::Drive(DriveError::NotSupported(
                    "NonCounted-wrapping is only supported for the nine standard merk tree \
                     variants; special trees (commitment / MMR / bulk-append / dense) are \
                     never index continuation trees.",
                )));
            }
        };
        let tree = Element::new_non_counted(inner)?;
        Ok(LowLevelDriveOperation::insert_for_known_path_key_element(
            path, key, tree,
        ))
    }

    /// Sets `GroveOperation` for inserting an empty provable count tree at the given path and key
    pub fn for_known_path_key_empty_provable_count_tree(
        path: Vec<Vec<u8>>,
        key: Vec<u8>,
        storage_flags: Option<&StorageFlags>,
    ) -> Self {
        let tree = match storage_flags {
            Some(storage_flags) => Element::new_provable_count_tree_with_flags(
                None,
                storage_flags.to_some_element_flags(),
            ),
            None => Element::empty_provable_count_tree(),
        };

        LowLevelDriveOperation::insert_for_known_path_key_element(path, key, tree)
    }

    /// Sets `GroveOperation` for inserting an empty provable sum tree at
    /// the given path and key. The provable variant commits aggregated
    /// sub-sums to every internal merk node, enabling O(log n)
    /// `AggregateSumOnRange` proofs over range queries on the property
    /// whose values feed the tree.
    ///
    /// Used by the index walker for property-name trees of indexes that
    /// declare `rangeSummable: true` (mirrors the count-side
    /// [`Self::for_known_path_key_empty_provable_count_tree`]).
    pub fn for_known_path_key_empty_provable_sum_tree(
        path: Vec<Vec<u8>>,
        key: Vec<u8>,
        storage_flags: Option<&StorageFlags>,
    ) -> Self {
        let tree = match storage_flags {
            Some(storage_flags) => Element::new_provable_sum_tree_with_flags(
                None,
                storage_flags.to_some_element_flags(),
            ),
            None => Element::empty_provable_sum_tree(),
        };

        LowLevelDriveOperation::insert_for_known_path_key_element(path, key, tree)
    }

    /// Sets `GroveOperation` for inserting an empty provable
    /// count-sum tree at the given path and key. **Pre-PR-670
    /// variant**: per-node counts committed to every internal merk
    /// node, but the sum is only carried at the root (not per-node).
    /// Use this when an index declares `rangeCountable: true` plus
    /// non-range `summable: "<prop>"` — count queries get the
    /// `AggregateCountOnRange` benefit while sum queries return only
    /// the root total.
    pub fn for_known_path_key_empty_provable_count_sum_tree(
        path: Vec<Vec<u8>>,
        key: Vec<u8>,
        storage_flags: Option<&StorageFlags>,
    ) -> Self {
        let tree = match storage_flags {
            Some(storage_flags) => Element::new_provable_count_sum_tree_with_flags(
                None,
                storage_flags.to_some_element_flags(),
            ),
            None => Element::empty_provable_count_sum_tree(),
        };

        LowLevelDriveOperation::insert_for_known_path_key_element(path, key, tree)
    }

    /// Sets `GroveOperation` for inserting an empty
    /// **provable-count-provable-sum** tree (PCPS) at the given path
    /// and key. The grovedb PR 670 newcomer: **both** per-node counts
    /// AND per-node sums committed to every internal merk node, so a
    /// single tree can answer both `AggregateCountOnRange`,
    /// `AggregateSumOnRange`, AND the new
    /// `AggregateCountAndSumOnRange` (combined) range queries.
    ///
    /// Used by the index walker for property-name trees of indexes
    /// that declare BOTH `rangeCountable: true` AND `rangeSummable:
    /// true`, and for primary-key trees that declare both at the
    /// doctype level. The dispatch table in
    /// [`crate::drive::document::primary_key_tree_type`]'s v1 arm
    /// picks `TreeType::ProvableCountProvableSumTree` for these
    /// cases.
    pub fn for_known_path_key_empty_provable_count_provable_sum_tree(
        path: Vec<Vec<u8>>,
        key: Vec<u8>,
        storage_flags: Option<&StorageFlags>,
    ) -> Self {
        let tree = match storage_flags {
            Some(storage_flags) => Element::new_provable_count_provable_sum_tree_with_flags(
                None,
                storage_flags.to_some_element_flags(),
            ),
            None => Element::empty_provable_count_provable_sum_tree(),
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

    /// Sets `GroveOperation` for inserting an empty sum tree at the given path and key
    pub fn for_estimated_path_key_empty_sum_tree(
        path: KeyInfoPath,
        key: KeyInfo,
        storage_flags: Option<&StorageFlags>,
    ) -> Self {
        let tree = match storage_flags {
            Some(storage_flags) => {
                Element::empty_sum_tree_with_flags(storage_flags.to_some_element_flags())
            }
            None => Element::empty_sum_tree(),
        };

        LowLevelDriveOperation::insert_for_estimated_path_key_element(path, key, tree)
    }

    /// Sets `GroveOperation` for inserting an empty count tree at the given (estimated) path and key
    pub fn for_estimated_path_key_empty_count_tree(
        path: KeyInfoPath,
        key: KeyInfo,
        storage_flags: Option<&StorageFlags>,
    ) -> Self {
        let tree = match storage_flags {
            Some(storage_flags) => {
                Element::empty_count_tree_with_flags(storage_flags.to_some_element_flags())
            }
            None => Element::empty_count_tree(),
        };

        LowLevelDriveOperation::insert_for_estimated_path_key_element(path, key, tree)
    }

    /// Sets `GroveOperation` for inserting an empty provable count tree at the given (estimated) path and key
    pub fn for_estimated_path_key_empty_provable_count_tree(
        path: KeyInfoPath,
        key: KeyInfo,
        storage_flags: Option<&StorageFlags>,
    ) -> Self {
        let tree = match storage_flags {
            Some(storage_flags) => {
                Element::empty_provable_count_tree_with_flags(storage_flags.to_some_element_flags())
            }
            None => Element::empty_provable_count_tree(),
        };

        LowLevelDriveOperation::insert_for_estimated_path_key_element(path, key, tree)
    }

    /// Cost-estimation analog of
    /// [`Self::for_known_path_key_empty_provable_sum_tree`]. See its doc.
    pub fn for_estimated_path_key_empty_provable_sum_tree(
        path: KeyInfoPath,
        key: KeyInfo,
        storage_flags: Option<&StorageFlags>,
    ) -> Self {
        let tree = match storage_flags {
            Some(storage_flags) => {
                Element::empty_provable_sum_tree_with_flags(storage_flags.to_some_element_flags())
            }
            None => Element::empty_provable_sum_tree(),
        };

        LowLevelDriveOperation::insert_for_estimated_path_key_element(path, key, tree)
    }

    /// Cost-estimation analog of
    /// [`Self::for_known_path_key_empty_count_sum_tree`]. See its doc.
    pub fn for_estimated_path_key_empty_count_sum_tree(
        path: KeyInfoPath,
        key: KeyInfo,
        storage_flags: Option<&StorageFlags>,
    ) -> Self {
        let tree = match storage_flags {
            Some(storage_flags) => {
                Element::empty_count_sum_tree_with_flags(storage_flags.to_some_element_flags())
            }
            None => Element::empty_count_sum_tree(),
        };

        LowLevelDriveOperation::insert_for_estimated_path_key_element(path, key, tree)
    }

    /// Cost-estimation analog of
    /// [`Self::for_known_path_key_empty_provable_count_sum_tree`]. See its
    /// doc.
    pub fn for_estimated_path_key_empty_provable_count_sum_tree(
        path: KeyInfoPath,
        key: KeyInfo,
        storage_flags: Option<&StorageFlags>,
    ) -> Self {
        let tree = match storage_flags {
            Some(storage_flags) => Element::empty_provable_count_sum_tree_with_flags(
                storage_flags.to_some_element_flags(),
            ),
            None => Element::empty_provable_count_sum_tree(),
        };

        LowLevelDriveOperation::insert_for_estimated_path_key_element(path, key, tree)
    }

    /// Cost-estimation analog of
    /// [`Self::for_known_path_key_empty_provable_count_provable_sum_tree`].
    /// See its doc.
    pub fn for_estimated_path_key_empty_provable_count_provable_sum_tree(
        path: KeyInfoPath,
        key: KeyInfo,
        storage_flags: Option<&StorageFlags>,
    ) -> Self {
        let tree = match storage_flags {
            Some(storage_flags) => Element::empty_provable_count_provable_sum_tree_with_flags(
                storage_flags.to_some_element_flags(),
            ),
            None => Element::empty_provable_count_provable_sum_tree(),
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
            // `non_counted: false` — Drive's index references contribute to
            // count aggregates on `ProvableCountTree` / `CountTree` parents
            // (and to count × sum aggregates on the dual-axis combined
            // trees). The non-counted variant exists in grovedb for
            // siblings-of-summable-only-trees that must not bump count
            // aggregates; Drive never refreshes those.
            false,
            trust_refresh_reference,
        ))
    }

    /// Sets `GroveOperation` for refresh of a
    /// [`grovedb::Element::ReferenceWithSumItem`] at the given path and
    /// key, **overriding** the carried sum with `sum_value`.
    ///
    /// Used by document-update paths on `summable` indexes: when the
    /// summed property's value changes but the index keys do not, the
    /// reference body stays the same but its sum contribution must be
    /// rewritten so ancestor `SumTree` / `ProvableCountSumTree` /
    /// `ProvableCountProvableSumTree` aggregates pick up the delta.
    ///
    /// Mirrors [`Self::refresh_reference_for_known_path_key_reference_info`]
    /// but emits a grovedb `RefreshReference` op in
    /// `SumItemReference*` mode instead of `PlainReference*` mode.
    pub fn refresh_reference_with_sum_item_for_known_path_key_reference_info(
        path: Vec<Vec<u8>>,
        key: Vec<u8>,
        reference_path_type: ReferencePathType,
        max_reference_hop: MaxReferenceHop,
        sum_value: i64,
        flags: Option<ElementFlags>,
        trust_refresh_reference: bool,
    ) -> Self {
        GroveOperation(QualifiedGroveDbOp::refresh_reference_with_sum_item_op(
            path,
            key,
            reference_path_type,
            max_reference_hop,
            sum_value,
            flags,
            // `non_counted: false` — see the count-tree rationale on the
            // plain-reference helper above. Same reasoning applies on the
            // sum side: index references always contribute to ancestor
            // count aggregates.
            false,
            trust_refresh_reference,
        ))
    }
}

/// A trait for getting an empty tree operation based on the tree type
pub trait LowLevelDriveOperationTreeTypeConverter {
    /// Sets `GroveOperation` for inserting an empty tree at the given path and key
    fn empty_tree_operation_for_known_path_key(
        &self,
        path: Vec<Vec<u8>>,
        key: Vec<u8>,
        storage_flags: Option<&StorageFlags>,
    ) -> Result<LowLevelDriveOperation, Error>;
}

impl LowLevelDriveOperationTreeTypeConverter for TreeType {
    /// Sets `GroveOperation` for inserting an empty tree at the given path and key
    fn empty_tree_operation_for_known_path_key(
        &self,
        path: Vec<Vec<u8>>,
        key: Vec<u8>,
        storage_flags: Option<&StorageFlags>,
    ) -> Result<LowLevelDriveOperation, Error> {
        let element_flags = storage_flags.map(|storage_flags| storage_flags.to_element_flags());
        let element = match self {
            TreeType::NormalTree => Element::empty_tree_with_flags(element_flags),
            TreeType::SumTree => Element::empty_sum_tree_with_flags(element_flags),
            TreeType::BigSumTree => Element::empty_big_sum_tree_with_flags(element_flags),
            TreeType::CountTree => Element::empty_count_tree_with_flags(element_flags),
            TreeType::CountSumTree => Element::empty_count_sum_tree_with_flags(element_flags),
            TreeType::ProvableCountTree => {
                Element::empty_provable_count_tree_with_flags(element_flags)
            }
            TreeType::ProvableCountSumTree => {
                Element::empty_provable_count_sum_tree_with_flags(element_flags)
            }
            TreeType::ProvableCountProvableSumTree => {
                Element::empty_provable_count_provable_sum_tree_with_flags(element_flags)
            }
            TreeType::ProvableSumTree => Element::empty_provable_sum_tree_with_flags(element_flags),
            TreeType::CommitmentTree(chunk_power) => {
                Element::empty_commitment_tree_with_flags(*chunk_power, element_flags)?
            }
            TreeType::MmrTree => Element::empty_mmr_tree_with_flags(element_flags),
            TreeType::BulkAppendTree(chunk_power) => {
                Element::empty_bulk_append_tree_with_flags(*chunk_power, element_flags)?
            }
            TreeType::DenseAppendOnlyFixedSizeTree(chunk_power) => {
                Element::empty_dense_tree_with_flags(*chunk_power, element_flags)
            }
        };

        Ok(LowLevelDriveOperation::insert_for_known_path_key_element(
            path, key, element,
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
            sinsemilla_hash_calls,
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
        let storage_loaded_bytes_cost = { *storage_loaded_bytes }
            .checked_mul(fee_version.storage.storage_load_credit_per_byte)
            .ok_or_else(|| get_overflow_error("storage loaded cost overflow"))?;

        // There is one block per hash node call
        let blake3_total = fee_version.hashing.blake3_base + fee_version.hashing.blake3_per_block;
        // this can't overflow
        let hash_node_cost = blake3_total * (*hash_node_calls as u64);
        let sinsemilla_cost = fee_version.hashing.sinsemilla_base * (*sinsemilla_hash_calls as u64);
        seek_cost
            .checked_add(storage_added_bytes_ephemeral_cost)
            .and_then(|c| c.checked_add(storage_replaced_bytes_ephemeral_cost))
            .and_then(|c| c.checked_add(storage_loaded_bytes_cost))
            .and_then(|c| c.checked_add(storage_removed_bytes_ephemeral_cost))
            .and_then(|c| c.checked_add(hash_node_cost))
            .and_then(|c| c.checked_add(sinsemilla_cost))
            .ok_or_else(|| get_overflow_error("ephemeral cost addition overflow"))
    }
}

#[cfg(test)]
#[allow(clippy::identity_op)]
mod tests {
    use super::*;
    use grovedb_costs::storage_cost::removal::StorageRemovedBytes;
    use grovedb_costs::storage_cost::StorageCost;
    use platform_version::version::fee::storage::FeeStorageVersion;
    use platform_version::version::fee::FeeVersion;

    /// Helper to get the canonical fee version used across these tests.
    fn fee_version() -> &'static FeeVersion {
        FeeVersion::first()
    }

    // ---------------------------------------------------------------
    // 1. BaseOp::cost() — spot-check several opcodes
    // ---------------------------------------------------------------

    #[test]
    fn base_op_stop_costs_zero() {
        assert_eq!(BaseOp::Stop.cost(), 0);
    }

    #[test]
    fn base_op_add_costs_12() {
        assert_eq!(BaseOp::Add.cost(), 12);
    }

    #[test]
    fn base_op_mul_costs_20() {
        assert_eq!(BaseOp::Mul.cost(), 20);
    }

    #[test]
    fn base_op_signextend_costs_20() {
        assert_eq!(BaseOp::Signextend.cost(), 20);
    }

    #[test]
    fn base_op_addmod_costs_32() {
        assert_eq!(BaseOp::Addmod.cost(), 32);
    }

    #[test]
    fn base_op_mulmod_costs_32() {
        assert_eq!(BaseOp::Mulmod.cost(), 32);
    }

    #[test]
    fn base_op_byte_costs_12() {
        assert_eq!(BaseOp::Byte.cost(), 12);
    }

    #[test]
    fn base_op_sub_costs_12() {
        assert_eq!(BaseOp::Sub.cost(), 12);
    }

    #[test]
    fn base_op_div_costs_20() {
        assert_eq!(BaseOp::Div.cost(), 20);
    }

    #[test]
    fn base_op_comparison_ops_all_cost_12() {
        for op in [
            BaseOp::Lt,
            BaseOp::Gt,
            BaseOp::Slt,
            BaseOp::Sgt,
            BaseOp::Eq,
            BaseOp::Iszero,
        ] {
            assert_eq!(op.cost(), 12, "comparison op {:?} should cost 12", op);
        }
    }

    #[test]
    fn base_op_bitwise_ops_all_cost_12() {
        for op in [BaseOp::And, BaseOp::Or, BaseOp::Xor, BaseOp::Not] {
            assert_eq!(op.cost(), 12, "bitwise op {:?} should cost 12", op);
        }
    }

    // ---------------------------------------------------------------
    // 2. HashFunction — block_size / rounds / block_cost / base_cost
    // ---------------------------------------------------------------

    #[test]
    fn hash_function_block_size_all_64() {
        // All four hash functions currently have a 64-byte block size.
        assert_eq!(HashFunction::Sha256.block_size(), 64);
        assert_eq!(HashFunction::Sha256_2.block_size(), 64);
        assert_eq!(HashFunction::Blake3.block_size(), 64);
        assert_eq!(HashFunction::Sha256RipeMD160.block_size(), 64);
    }

    #[test]
    fn hash_function_rounds() {
        assert_eq!(HashFunction::Sha256.rounds(), 1);
        assert_eq!(HashFunction::Sha256_2.rounds(), 2);
        assert_eq!(HashFunction::Blake3.rounds(), 1);
        assert_eq!(HashFunction::Sha256RipeMD160.rounds(), 1);
    }

    #[test]
    fn hash_function_block_cost_sha256_variants_use_sha256_per_block() {
        let fv = fee_version();
        let expected = fv.hashing.sha256_per_block;
        assert_eq!(HashFunction::Sha256.block_cost(fv), expected);
        assert_eq!(HashFunction::Sha256_2.block_cost(fv), expected);
        assert_eq!(HashFunction::Sha256RipeMD160.block_cost(fv), expected);
    }

    #[test]
    fn hash_function_block_cost_blake3_uses_blake3_per_block() {
        let fv = fee_version();
        assert_eq!(
            HashFunction::Blake3.block_cost(fv),
            fv.hashing.blake3_per_block
        );
    }

    #[test]
    fn hash_function_base_cost_sha256() {
        let fv = fee_version();
        assert_eq!(
            HashFunction::Sha256.base_cost(fv),
            fv.hashing.single_sha256_base
        );
    }

    #[test]
    fn hash_function_base_cost_sha256_2_uses_single_sha256_base() {
        let fv = fee_version();
        // Sha256_2 intentionally uses single_sha256_base (extra rounds handle the double hash).
        assert_eq!(
            HashFunction::Sha256_2.base_cost(fv),
            fv.hashing.single_sha256_base
        );
    }

    #[test]
    fn hash_function_base_cost_blake3() {
        let fv = fee_version();
        assert_eq!(HashFunction::Blake3.base_cost(fv), fv.hashing.blake3_base);
    }

    #[test]
    fn hash_function_base_cost_sha256_ripe_md160() {
        let fv = fee_version();
        assert_eq!(
            HashFunction::Sha256RipeMD160.base_cost(fv),
            fv.hashing.sha256_ripe_md160_base
        );
    }

    // ---------------------------------------------------------------
    // 3. FunctionOp::new_with_byte_count — verify blocks/rounds calc
    // ---------------------------------------------------------------

    #[test]
    fn function_op_new_with_byte_count_small_sha256() {
        // 32 bytes => blocks = 32/64 + 1 = 1, rounds = 1 + 1 - 1 = 1
        let op = FunctionOp::new_with_byte_count(HashFunction::Sha256, 32);
        assert_eq!(op.rounds, 1);
        assert_eq!(op.hash, HashFunction::Sha256);
    }

    #[test]
    fn function_op_new_with_byte_count_exact_block_boundary_sha256() {
        // 64 bytes => blocks = 64/64 + 1 = 2, rounds = 2 + 1 - 1 = 2
        let op = FunctionOp::new_with_byte_count(HashFunction::Sha256, 64);
        assert_eq!(op.rounds, 2);
    }

    #[test]
    fn function_op_new_with_byte_count_large_sha256() {
        // 200 bytes => blocks = 200/64 + 1 = 3 + 1 = 4, rounds = 4 + 1 - 1 = 4
        let op = FunctionOp::new_with_byte_count(HashFunction::Sha256, 200);
        assert_eq!(op.rounds, 4);
    }

    #[test]
    fn function_op_new_with_byte_count_sha256_2_has_extra_round() {
        // 32 bytes => blocks = 32/64 + 1 = 1, rounds = 1 + 2 - 1 = 2
        let op = FunctionOp::new_with_byte_count(HashFunction::Sha256_2, 32);
        assert_eq!(op.rounds, 2);
    }

    #[test]
    fn function_op_new_with_byte_count_sha256_2_large() {
        // 200 bytes => blocks = 200/64 + 1 = 4, rounds = 4 + 2 - 1 = 5
        let op = FunctionOp::new_with_byte_count(HashFunction::Sha256_2, 200);
        assert_eq!(op.rounds, 5);
    }

    #[test]
    fn function_op_new_with_byte_count_blake3_small() {
        // 10 bytes => blocks = 10/64 + 1 = 1, rounds = 1 + 1 - 1 = 1
        let op = FunctionOp::new_with_byte_count(HashFunction::Blake3, 10);
        assert_eq!(op.rounds, 1);
        assert_eq!(op.hash, HashFunction::Blake3);
    }

    #[test]
    fn function_op_new_with_byte_count_blake3_large() {
        // 500 bytes => blocks = 500/64 + 1 = 7 + 1 = 8, rounds = 8 + 1 - 1 = 8
        let op = FunctionOp::new_with_byte_count(HashFunction::Blake3, 500);
        assert_eq!(op.rounds, 8);
    }

    #[test]
    fn function_op_new_with_byte_count_zero_bytes() {
        // 0 bytes => blocks = 0/64 + 1 = 1, rounds = 1 + 1 - 1 = 1
        let op = FunctionOp::new_with_byte_count(HashFunction::Sha256, 0);
        assert_eq!(op.rounds, 1);
    }

    #[test]
    fn function_op_new_with_byte_count_sha256_ripemd160() {
        // 20 bytes => blocks = 20/64 + 1 = 1, rounds = 1 + 1 - 1 = 1
        let op = FunctionOp::new_with_byte_count(HashFunction::Sha256RipeMD160, 20);
        assert_eq!(op.rounds, 1);
        assert_eq!(op.hash, HashFunction::Sha256RipeMD160);
    }

    // ---------------------------------------------------------------
    // 4. FunctionOp::cost — verify rounds * block_cost + base_cost
    // ---------------------------------------------------------------

    #[test]
    fn function_op_cost_sha256_one_round() {
        let fv = fee_version();
        let op = FunctionOp::new_with_round_count(HashFunction::Sha256, 1);
        // cost = base + rounds * block_cost = 100 + 1 * 5000 = 5100
        let expected = fv.hashing.single_sha256_base + 1 * fv.hashing.sha256_per_block;
        assert_eq!(op.cost(fv), expected);
    }

    #[test]
    fn function_op_cost_sha256_2_two_rounds() {
        let fv = fee_version();
        let op = FunctionOp::new_with_round_count(HashFunction::Sha256_2, 2);
        // cost = base + rounds * block_cost = 100 + 2 * 5000 = 10100
        let expected = fv.hashing.single_sha256_base + 2 * fv.hashing.sha256_per_block;
        assert_eq!(op.cost(fv), expected);
    }

    #[test]
    fn function_op_cost_blake3_one_round() {
        let fv = fee_version();
        let op = FunctionOp::new_with_round_count(HashFunction::Blake3, 1);
        // cost = blake3_base + 1 * blake3_per_block = 100 + 300 = 400
        let expected = fv.hashing.blake3_base + 1 * fv.hashing.blake3_per_block;
        assert_eq!(op.cost(fv), expected);
    }

    #[test]
    fn function_op_cost_zero_rounds() {
        let fv = fee_version();
        let op = FunctionOp::new_with_round_count(HashFunction::Blake3, 0);
        // cost = blake3_base + 0 * blake3_per_block = blake3_base
        assert_eq!(op.cost(fv), fv.hashing.blake3_base);
    }

    #[test]
    fn function_op_cost_from_byte_count_matches_manual_calc() {
        let fv = fee_version();
        // 128 bytes of SHA256: blocks = 128/64 + 1 = 3, rounds = 3 + 1 - 1 = 3
        let op = FunctionOp::new_with_byte_count(HashFunction::Sha256, 128);
        assert_eq!(op.rounds, 3);
        let expected = fv.hashing.single_sha256_base + 3 * fv.hashing.sha256_per_block;
        assert_eq!(op.cost(fv), expected);
    }

    #[test]
    fn function_op_cost_sha256_ripemd160() {
        let fv = fee_version();
        let op = FunctionOp::new_with_round_count(HashFunction::Sha256RipeMD160, 1);
        let expected = fv.hashing.sha256_ripe_md160_base + 1 * fv.hashing.sha256_per_block;
        assert_eq!(op.cost(fv), expected);
    }

    #[test]
    fn function_op_cost_saturating_mul_does_not_panic_on_large_rounds() {
        let fv = fee_version();
        let op = FunctionOp::new_with_round_count(HashFunction::Sha256, u32::MAX);
        // u32::MAX as u64 * sha256_per_block (5000) fits in u64 without overflow,
        // so cost = base + rounds * block_cost, computed via saturating ops.
        let expected_block_cost = (u32::MAX as u64).saturating_mul(fv.hashing.sha256_per_block);
        let expected = fv
            .hashing
            .single_sha256_base
            .saturating_add(expected_block_cost);
        assert_eq!(op.cost(fv), expected);
    }

    #[test]
    fn function_op_cost_saturates_to_max_with_extreme_fee_version() {
        // Construct a fee version where block_cost is large enough that
        // u32::MAX * block_cost overflows u64, triggering saturation.
        let mut fv = fee_version().clone();
        fv.hashing.sha256_per_block = u64::MAX;
        let op = FunctionOp::new_with_round_count(HashFunction::Sha256, 2);
        // 2 * u64::MAX saturates to u64::MAX, then base.saturating_add(u64::MAX) = u64::MAX.
        assert_eq!(op.cost(&fv), u64::MAX);
    }

    // ---------------------------------------------------------------
    // 5. operation_cost() — test all 4 match arms
    // ---------------------------------------------------------------

    #[test]
    fn operation_cost_calculated_cost_operation_returns_cost() {
        let cost = OperationCost {
            seek_count: 3,
            storage_cost: StorageCost {
                added_bytes: 100,
                replaced_bytes: 50,
                removed_bytes: StorageRemovedBytes::NoStorageRemoval,
            },
            storage_loaded_bytes: 200,
            hash_node_calls: 5,
            sinsemilla_hash_calls: 0,
        };
        let op = CalculatedCostOperation(cost.clone());
        let result = op.operation_cost().expect("should return Ok");
        assert_eq!(result, cost);
    }

    #[test]
    fn operation_cost_grove_operation_returns_error() {
        let grove_op = LowLevelDriveOperation::insert_for_known_path_key_element(
            vec![vec![1, 2, 3]],
            vec![4, 5, 6],
            Element::empty_tree(),
        );
        let result = grove_op.operation_cost();
        assert!(result.is_err());
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(
            err_msg.contains("grove operations must be executed"),
            "unexpected error: {}",
            err_msg
        );
    }

    #[test]
    fn operation_cost_pre_calculated_fee_result_returns_error() {
        let fee = FeeResult {
            storage_fee: 100,
            processing_fee: 200,
            ..Default::default()
        };
        let op = PreCalculatedFeeResult(fee);
        let result = op.operation_cost();
        assert!(result.is_err());
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(
            err_msg.contains("pre calculated fees should not be requested"),
            "unexpected error: {}",
            err_msg
        );
    }

    #[test]
    fn operation_cost_function_operation_returns_error() {
        let func_op = FunctionOperation(FunctionOp::new_with_round_count(HashFunction::Blake3, 1));
        let result = func_op.operation_cost();
        assert!(result.is_err());
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(
            err_msg.contains("function operations should not be requested"),
            "unexpected error: {}",
            err_msg
        );
    }

    // ---------------------------------------------------------------
    // 6. combine_cost_operations — filter and sum
    // ---------------------------------------------------------------

    #[test]
    fn combine_cost_operations_sums_calculated_costs_only() {
        let cost1 = OperationCost {
            seek_count: 2,
            storage_cost: StorageCost {
                added_bytes: 10,
                replaced_bytes: 0,
                removed_bytes: StorageRemovedBytes::NoStorageRemoval,
            },
            storage_loaded_bytes: 50,
            hash_node_calls: 1,
            sinsemilla_hash_calls: 0,
        };
        let cost2 = OperationCost {
            seek_count: 3,
            storage_cost: StorageCost {
                added_bytes: 20,
                replaced_bytes: 5,
                removed_bytes: StorageRemovedBytes::NoStorageRemoval,
            },
            storage_loaded_bytes: 100,
            hash_node_calls: 2,
            sinsemilla_hash_calls: 1,
        };

        let operations = vec![
            CalculatedCostOperation(cost1.clone()),
            // This FunctionOperation should be ignored by combine_cost_operations
            FunctionOperation(FunctionOp::new_with_round_count(HashFunction::Sha256, 1)),
            CalculatedCostOperation(cost2.clone()),
            // PreCalculatedFeeResult should also be ignored
            PreCalculatedFeeResult(FeeResult::default()),
        ];

        let combined = LowLevelDriveOperation::combine_cost_operations(&operations);
        assert_eq!(combined.seek_count, 2 + 3);
        assert_eq!(combined.storage_cost.added_bytes, 10 + 20);
        assert_eq!(combined.storage_cost.replaced_bytes, 0 + 5);
        assert_eq!(combined.storage_loaded_bytes, 50 + 100);
        assert_eq!(combined.hash_node_calls, 1 + 2);
        assert_eq!(combined.sinsemilla_hash_calls, 0 + 1);
    }

    #[test]
    fn combine_cost_operations_empty_list_returns_default() {
        let combined = LowLevelDriveOperation::combine_cost_operations(&[]);
        assert_eq!(combined, OperationCost::default());
    }

    #[test]
    fn combine_cost_operations_no_calculated_costs_returns_default() {
        let operations = vec![
            FunctionOperation(FunctionOp::new_with_round_count(HashFunction::Blake3, 2)),
            PreCalculatedFeeResult(FeeResult {
                processing_fee: 999,
                ..Default::default()
            }),
        ];
        let combined = LowLevelDriveOperation::combine_cost_operations(&operations);
        assert_eq!(combined, OperationCost::default());
    }

    // ---------------------------------------------------------------
    // 7. grovedb_operations_batch / _consume / _consume_with_leftovers
    // ---------------------------------------------------------------

    /// Helper: creates a GroveOperation variant (insert_or_replace).
    fn make_grove_op(key_byte: u8) -> LowLevelDriveOperation {
        LowLevelDriveOperation::insert_for_known_path_key_element(
            vec![vec![0]],
            vec![key_byte],
            Element::new_item(vec![key_byte]),
        )
    }

    fn make_mixed_ops() -> Vec<LowLevelDriveOperation> {
        vec![
            make_grove_op(1),
            FunctionOperation(FunctionOp::new_with_round_count(HashFunction::Sha256, 1)),
            make_grove_op(2),
            CalculatedCostOperation(OperationCost::default()),
            make_grove_op(3),
        ]
    }

    #[test]
    fn grovedb_operations_batch_filters_grove_ops_from_ref() {
        let ops = make_mixed_ops();
        let batch = LowLevelDriveOperation::grovedb_operations_batch(&ops);
        assert_eq!(batch.len(), 3);
    }

    #[test]
    fn grovedb_operations_batch_empty_input() {
        let batch = LowLevelDriveOperation::grovedb_operations_batch(&[]);
        assert!(batch.is_empty());
    }

    #[test]
    fn grovedb_operations_batch_no_grove_ops() {
        let ops = vec![
            FunctionOperation(FunctionOp::new_with_round_count(HashFunction::Blake3, 1)),
            CalculatedCostOperation(OperationCost::default()),
        ];
        let batch = LowLevelDriveOperation::grovedb_operations_batch(&ops);
        assert!(batch.is_empty());
    }

    #[test]
    fn grovedb_operations_batch_consume_filters_grove_ops() {
        let ops = make_mixed_ops();
        let batch = LowLevelDriveOperation::grovedb_operations_batch_consume(ops);
        assert_eq!(batch.len(), 3);
    }

    #[test]
    fn grovedb_operations_batch_consume_empty_input() {
        let batch = LowLevelDriveOperation::grovedb_operations_batch_consume(vec![]);
        assert!(batch.is_empty());
    }

    #[test]
    fn grovedb_operations_batch_consume_with_leftovers_partitions_correctly() {
        let ops = make_mixed_ops();
        let (batch, leftovers) =
            LowLevelDriveOperation::grovedb_operations_batch_consume_with_leftovers(ops);
        assert_eq!(batch.len(), 3);
        assert_eq!(leftovers.len(), 2);

        // Verify leftovers contain the non-grove operations.
        for leftover in &leftovers {
            assert!(
                !matches!(leftover, GroveOperation(_)),
                "leftovers should not contain GroveOperation variants"
            );
        }
    }

    #[test]
    fn grovedb_operations_batch_consume_with_leftovers_all_grove() {
        let ops = vec![make_grove_op(10), make_grove_op(20)];
        let (batch, leftovers) =
            LowLevelDriveOperation::grovedb_operations_batch_consume_with_leftovers(ops);
        assert_eq!(batch.len(), 2);
        assert!(leftovers.is_empty());
    }

    #[test]
    fn grovedb_operations_batch_consume_with_leftovers_no_grove() {
        let ops = vec![
            CalculatedCostOperation(OperationCost::default()),
            FunctionOperation(FunctionOp::new_with_round_count(HashFunction::Sha256, 1)),
        ];
        let (batch, leftovers) =
            LowLevelDriveOperation::grovedb_operations_batch_consume_with_leftovers(ops);
        assert!(batch.is_empty());
        assert_eq!(leftovers.len(), 2);
    }

    #[test]
    fn grovedb_operations_batch_consume_with_leftovers_empty() {
        let (batch, leftovers) =
            LowLevelDriveOperation::grovedb_operations_batch_consume_with_leftovers(vec![]);
        assert!(batch.is_empty());
        assert!(leftovers.is_empty());
    }

    // ---------------------------------------------------------------
    // 8. DriveCost::ephemeral_cost — various scenarios
    // ---------------------------------------------------------------

    #[test]
    fn ephemeral_cost_zero_operation() {
        let fv = fee_version();
        let cost = OperationCost::default();
        let result = cost.ephemeral_cost(fv).expect("should not overflow");
        assert_eq!(result, 0);
    }

    #[test]
    fn ephemeral_cost_seek_only() {
        let fv = fee_version();
        let cost = OperationCost {
            seek_count: 5,
            storage_cost: StorageCost::default(),
            storage_loaded_bytes: 0,
            hash_node_calls: 0,
            sinsemilla_hash_calls: 0,
        };
        let result = cost.ephemeral_cost(fv).expect("should not overflow");
        let expected = 5u64 * fv.storage.storage_seek_cost;
        assert_eq!(result, expected);
    }

    #[test]
    fn ephemeral_cost_storage_added_bytes() {
        let fv = fee_version();
        let cost = OperationCost {
            seek_count: 0,
            storage_cost: StorageCost {
                added_bytes: 100,
                replaced_bytes: 0,
                removed_bytes: StorageRemovedBytes::NoStorageRemoval,
            },
            storage_loaded_bytes: 0,
            hash_node_calls: 0,
            sinsemilla_hash_calls: 0,
        };
        let result = cost.ephemeral_cost(fv).expect("should not overflow");
        let expected = 100u64 * fv.storage.storage_processing_credit_per_byte;
        assert_eq!(result, expected);
    }

    #[test]
    fn ephemeral_cost_storage_replaced_bytes() {
        let fv = fee_version();
        let cost = OperationCost {
            seek_count: 0,
            storage_cost: StorageCost {
                added_bytes: 0,
                replaced_bytes: 50,
                removed_bytes: StorageRemovedBytes::NoStorageRemoval,
            },
            storage_loaded_bytes: 0,
            hash_node_calls: 0,
            sinsemilla_hash_calls: 0,
        };
        let result = cost.ephemeral_cost(fv).expect("should not overflow");
        let expected = 50u64 * fv.storage.storage_processing_credit_per_byte;
        assert_eq!(result, expected);
    }

    #[test]
    fn ephemeral_cost_storage_removed_bytes_basic() {
        let fv = fee_version();
        let cost = OperationCost {
            seek_count: 0,
            storage_cost: StorageCost {
                added_bytes: 0,
                replaced_bytes: 0,
                removed_bytes: StorageRemovedBytes::BasicStorageRemoval(75),
            },
            storage_loaded_bytes: 0,
            hash_node_calls: 0,
            sinsemilla_hash_calls: 0,
        };
        let result = cost.ephemeral_cost(fv).expect("should not overflow");
        let expected = 75u64 * fv.storage.storage_processing_credit_per_byte;
        assert_eq!(result, expected);
    }

    #[test]
    fn ephemeral_cost_loaded_bytes() {
        let fv = fee_version();
        let cost = OperationCost {
            seek_count: 0,
            storage_cost: StorageCost::default(),
            storage_loaded_bytes: 300,
            hash_node_calls: 0,
            sinsemilla_hash_calls: 0,
        };
        let result = cost.ephemeral_cost(fv).expect("should not overflow");
        let expected = 300u64 * fv.storage.storage_load_credit_per_byte;
        assert_eq!(result, expected);
    }

    #[test]
    fn ephemeral_cost_hash_node_calls() {
        let fv = fee_version();
        let cost = OperationCost {
            seek_count: 0,
            storage_cost: StorageCost::default(),
            storage_loaded_bytes: 0,
            hash_node_calls: 10,
            sinsemilla_hash_calls: 0,
        };
        let result = cost.ephemeral_cost(fv).expect("should not overflow");
        let blake3_total = fv.hashing.blake3_base + fv.hashing.blake3_per_block;
        let expected = blake3_total * 10;
        assert_eq!(result, expected);
    }

    #[test]
    fn ephemeral_cost_sinsemilla_hash_calls() {
        let fv = fee_version();
        let cost = OperationCost {
            seek_count: 0,
            storage_cost: StorageCost::default(),
            storage_loaded_bytes: 0,
            hash_node_calls: 0,
            sinsemilla_hash_calls: 3,
        };
        let result = cost.ephemeral_cost(fv).expect("should not overflow");
        let expected = fv.hashing.sinsemilla_base * 3;
        assert_eq!(result, expected);
    }

    #[test]
    fn ephemeral_cost_all_components_combined() {
        let fv = fee_version();
        let cost = OperationCost {
            seek_count: 2,
            storage_cost: StorageCost {
                added_bytes: 10,
                replaced_bytes: 20,
                removed_bytes: StorageRemovedBytes::BasicStorageRemoval(30),
            },
            storage_loaded_bytes: 40,
            hash_node_calls: 5,
            sinsemilla_hash_calls: 1,
        };
        let result = cost.ephemeral_cost(fv).expect("should not overflow");

        let seek_cost = 2u64 * fv.storage.storage_seek_cost;
        let processing_per_byte = fv.storage.storage_processing_credit_per_byte;
        let added_cost = 10u64 * processing_per_byte;
        let replaced_cost = 20u64 * processing_per_byte;
        let removed_cost = 30u64 * processing_per_byte;
        let loaded_cost = 40u64 * fv.storage.storage_load_credit_per_byte;
        let blake3_total = fv.hashing.blake3_base + fv.hashing.blake3_per_block;
        let hash_cost = blake3_total * 5;
        let sinsemilla_cost = fv.hashing.sinsemilla_base * 1;

        let expected = seek_cost
            + added_cost
            + replaced_cost
            + loaded_cost
            + removed_cost
            + hash_cost
            + sinsemilla_cost;
        assert_eq!(result, expected);
    }

    #[test]
    fn ephemeral_cost_overflow_seek_cost() {
        let fv = &FeeVersion {
            storage: FeeStorageVersion {
                storage_seek_cost: u64::MAX,
                ..fee_version().storage.clone()
            },
            ..fee_version().clone()
        };
        let cost = OperationCost {
            seek_count: 2, // 2 * u64::MAX overflows
            storage_cost: StorageCost::default(),
            storage_loaded_bytes: 0,
            hash_node_calls: 0,
            sinsemilla_hash_calls: 0,
        };
        let result = cost.ephemeral_cost(fv);
        assert!(result.is_err(), "expected overflow error for seek cost");
    }

    #[test]
    fn ephemeral_cost_overflow_storage_written_bytes() {
        let fv = &FeeVersion {
            storage: FeeStorageVersion {
                storage_processing_credit_per_byte: u64::MAX,
                ..fee_version().storage.clone()
            },
            ..fee_version().clone()
        };
        let cost = OperationCost {
            seek_count: 0,
            storage_cost: StorageCost {
                added_bytes: 2, // 2 * u64::MAX overflows
                replaced_bytes: 0,
                removed_bytes: StorageRemovedBytes::NoStorageRemoval,
            },
            storage_loaded_bytes: 0,
            hash_node_calls: 0,
            sinsemilla_hash_calls: 0,
        };
        let result = cost.ephemeral_cost(fv);
        assert!(
            result.is_err(),
            "expected overflow error for storage written bytes"
        );
    }

    #[test]
    fn ephemeral_cost_overflow_loaded_bytes() {
        let fv = &FeeVersion {
            storage: FeeStorageVersion {
                storage_load_credit_per_byte: u64::MAX,
                ..fee_version().storage.clone()
            },
            ..fee_version().clone()
        };
        let cost = OperationCost {
            seek_count: 0,
            storage_cost: StorageCost::default(),
            storage_loaded_bytes: 2, // 2 * u64::MAX overflows
            hash_node_calls: 0,
            sinsemilla_hash_calls: 0,
        };
        let result = cost.ephemeral_cost(fv);
        assert!(
            result.is_err(),
            "expected overflow error for loaded bytes cost"
        );
    }

    /// Covers the `TreeType::ProvableSumTree` arm of
    /// `LowLevelDriveOperationTreeTypeConverter::empty_tree_operation_for_known_path_key`
    /// added by the grovedb#661 bump. Drive doesn't currently construct
    /// `ProvableSumTree` anywhere else, so without this test the new arm is
    /// uncovered.
    #[test]
    fn empty_tree_operation_for_known_path_key_provable_sum_tree() {
        use grovedb::batch::GroveOp;

        let op = TreeType::ProvableSumTree
            .empty_tree_operation_for_known_path_key(vec![b"root".to_vec()], b"k".to_vec(), None)
            .expect("empty_tree_operation_for_known_path_key");

        match op {
            LowLevelDriveOperation::GroveOperation(grove_op) => match grove_op.op {
                GroveOp::InsertOrReplace { element } => assert!(
                    matches!(element, Element::ProvableSumTree(..)),
                    "expected ProvableSumTree element, got: {:?}",
                    element
                ),
                other => panic!("expected GroveOp::InsertOrReplace, got: {:?}", other),
            },
            other => panic!("expected GroveOperation, got: {:?}", other),
        }
    }

    /// Table-driven pin of the v14 zero-contribution dispatcher: every
    /// accepted parent × inner cell must produce exactly the specified
    /// wrapper (or an unwrapped tree), and every rejected parent must
    /// error for every inner. This decides consensus-relevant element
    /// shapes for v14 continuation inserts, so a regression here (or a
    /// demotion-helper change routing a provable parent in) must fail
    /// loudly.
    #[test]
    fn zero_contribution_dispatcher_full_matrix() {
        use grovedb::batch::GroveOp;

        const ALL_INNERS: [TreeType; 9] = [
            TreeType::NormalTree,
            TreeType::SumTree,
            TreeType::BigSumTree,
            TreeType::CountTree,
            TreeType::CountSumTree,
            TreeType::ProvableCountTree,
            TreeType::ProvableCountSumTree,
            TreeType::ProvableSumTree,
            TreeType::ProvableCountProvableSumTree,
        ];

        fn is_sum_bearing(tree_type: TreeType) -> bool {
            matches!(
                tree_type,
                TreeType::SumTree
                    | TreeType::BigSumTree
                    | TreeType::CountSumTree
                    | TreeType::ProvableCountSumTree
                    | TreeType::ProvableSumTree
                    | TreeType::ProvableCountProvableSumTree
            )
        }

        fn element_tree_type(element: &Element) -> TreeType {
            match element {
                Element::Tree(..) => TreeType::NormalTree,
                Element::SumTree(..) => TreeType::SumTree,
                Element::BigSumTree(..) => TreeType::BigSumTree,
                Element::CountTree(..) => TreeType::CountTree,
                Element::CountSumTree(..) => TreeType::CountSumTree,
                Element::ProvableCountTree(..) => TreeType::ProvableCountTree,
                Element::ProvableCountSumTree(..) => TreeType::ProvableCountSumTree,
                Element::ProvableSumTree(..) => TreeType::ProvableSumTree,
                Element::ProvableCountProvableSumTree(..) => TreeType::ProvableCountProvableSumTree,
                other => panic!("unexpected inner element: {other:?}"),
            }
        }

        #[derive(Debug, PartialEq)]
        enum Expected {
            NonCounted,
            NotSummed,
            NotCountedOrSummed,
            Unwrapped,
        }

        let dispatch = |parent: TreeType, inner: TreeType| {
            LowLevelDriveOperation::for_known_path_key_empty_tree_contributing_zero_to_parent(
                vec![b"root".to_vec()],
                b"key".to_vec(),
                parent,
                inner,
                None,
            )
        };

        let assert_cell = |parent: TreeType, inner: TreeType, expected: Expected| {
            let op = dispatch(parent, inner).unwrap_or_else(|error| {
                panic!("parent {parent:?} inner {inner:?} must be accepted: {error}")
            });
            let element = match op {
                LowLevelDriveOperation::GroveOperation(grove_op) => match grove_op.op {
                    GroveOp::InsertOrReplace { element } => element,
                    other => panic!("expected InsertOrReplace, got {other:?}"),
                },
                other => panic!("expected GroveOperation, got {other:?}"),
            };
            let (wrapper, produced_inner) = match &element {
                Element::NonCounted(inner_element) => {
                    (Expected::NonCounted, inner_element.as_ref())
                }
                Element::NotSummed(inner_element) => (Expected::NotSummed, inner_element.as_ref()),
                Element::NotCountedOrSummed(inner_element) => {
                    (Expected::NotCountedOrSummed, inner_element.as_ref())
                }
                plain => (Expected::Unwrapped, plain),
            };
            assert_eq!(
                wrapper, expected,
                "parent {parent:?} inner {inner:?}: wrong wrapper"
            );
            assert_eq!(
                element_tree_type(produced_inner),
                inner,
                "parent {parent:?} inner {inner:?}: wrong inner tree type"
            );
        };

        // Count-only parents wrap every inner NonCounted.
        for inner in ALL_INNERS {
            assert_cell(TreeType::CountTree, inner, Expected::NonCounted);
        }
        // Count-sum parents: sum-bearing inners get NotCountedOrSummed,
        // non-sum inners get NonCounted.
        for inner in ALL_INNERS {
            let expected = if is_sum_bearing(inner) {
                Expected::NotCountedOrSummed
            } else {
                Expected::NonCounted
            };
            assert_cell(TreeType::CountSumTree, inner, expected);
        }
        // Sum-only parents: sum-bearing inners get NotSummed, non-sum
        // inners are inserted unwrapped (they contribute 0 naturally).
        for parent in [
            TreeType::SumTree,
            TreeType::BigSumTree,
            TreeType::ProvableSumTree,
        ] {
            for inner in ALL_INNERS {
                let expected = if is_sum_bearing(inner) {
                    Expected::NotSummed
                } else {
                    Expected::Unwrapped
                };
                assert_cell(parent, inner, expected);
            }
        }
        // Provable count-bearing parents can't host zero-contributing
        // children (the walkers demote them first); non-aggregating
        // parents should use the plain path. Both must error for every
        // inner.
        for parent in [
            TreeType::NormalTree,
            TreeType::ProvableCountTree,
            TreeType::ProvableCountSumTree,
            TreeType::ProvableCountProvableSumTree,
        ] {
            for inner in ALL_INNERS {
                assert!(
                    dispatch(parent, inner).is_err(),
                    "parent {parent:?} inner {inner:?} must be rejected"
                );
            }
        }
    }

    #[test]
    fn ephemeral_cost_overflow_in_addition_chain() {
        // Use values that individually do not overflow but whose sum does.
        let fv = fee_version();
        let cost = OperationCost {
            seek_count: u32::MAX,
            storage_cost: StorageCost {
                added_bytes: u32::MAX,
                replaced_bytes: u32::MAX,
                removed_bytes: StorageRemovedBytes::BasicStorageRemoval(u32::MAX),
            },
            storage_loaded_bytes: u64::MAX,
            hash_node_calls: u32::MAX,
            sinsemilla_hash_calls: u32::MAX,
        };
        let result = cost.ephemeral_cost(fv);
        assert!(
            result.is_err(),
            "expected overflow error when summing large components"
        );
    }
}
