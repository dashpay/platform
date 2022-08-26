use crate::drive::batch::GroveDbOpBatch;
use costs::OperationCost;
use enum_map::Enum;
use grovedb::{batch::GroveDbOp, Element, PathQuery};

use crate::drive::flags::StorageFlags;
use crate::error::drive::DriveError;
use crate::error::fee::FeeError;
use crate::error::Error;
use crate::fee::default_costs::{
    HASH_BYTE_COST, HASH_NODE_COST, NON_STORAGE_LOAD_CREDIT_PER_BYTE,
    STORAGE_DISK_USAGE_CREDIT_PER_BYTE, STORAGE_LOAD_CREDIT_PER_BYTE,
    STORAGE_PROCESSING_CREDIT_PER_BYTE, STORAGE_SEEK_COST,
};
use crate::fee::op::DriveOperation::{
    CalculatedCostOperation, ContractFetch, CostCalculationDeleteOperation,
    CostCalculationInsertOperation, CostCalculationQueryOperation, GroveOperation,
};

#[derive(Debug, Enum)]
pub enum BaseOp {
    Stop,
    Add,
    Mul,
    Sub,
    Div,
    Sdiv,
    Mod,
    Smod,
    Addmod,
    Mulmod,
    Signextend,
    Lt,
    Gt,
    Slt,
    Sgt,
    Eq,
    Iszero,
    And,
    Or,
    Xor,
    Not,
    Byte,
}

impl BaseOp {
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

#[derive(Debug, Enum)]
pub enum FunctionOp {
    Exp,
    Sha256,
    Sha256_2,
    Blake3,
}

impl FunctionOp {
    pub fn cost(&self, _word_count: u32) {}
}

#[derive(Debug)]
pub struct SizesOfQueryOperation {
    pub key_size: u32,
    pub path_size: u32,
    pub value_size: u32,
}

trait OperationCostConvert {
    fn cost(&self) -> OperationCost;
}

impl SizesOfQueryOperation {
    pub fn for_key_check_in_path<'a: 'b, 'b, 'c, P>(key_len: usize, path: P) -> Self
    where
        P: IntoIterator<Item = &'c [u8]>,
        <P as IntoIterator>::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
    {
        let path_size: u32 = path
            .into_iter()
            .map(|inner: &[u8]| inner.len() as u32)
            .sum();
        SizesOfQueryOperation {
            key_size: key_len as u32,
            path_size,
            value_size: 0,
        }
    }

    pub fn for_key_check_with_path_length(key_len: usize, path_len: usize) -> Self {
        SizesOfQueryOperation {
            key_size: key_len as u32,
            path_size: path_len as u32,
            value_size: 0,
        }
    }

    pub fn for_value_retrieval_in_path<'a: 'b, 'b, 'c, P>(
        key_len: usize,
        path: P,
        value_len: usize,
    ) -> Self
    where
        P: IntoIterator<Item = &'c [u8]>,
        <P as IntoIterator>::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
    {
        let path_size: u32 = path
            .into_iter()
            .map(|inner: &[u8]| inner.len() as u32)
            .sum();
        SizesOfQueryOperation {
            key_size: key_len as u32,
            path_size,
            value_size: value_len as u32,
        }
    }

    pub fn for_value_retrieval_with_path_length(
        key_len: usize,
        path_len: usize,
        value_len: usize,
    ) -> Self {
        SizesOfQueryOperation {
            key_size: key_len as u32,
            path_size: path_len as u32,
            value_size: value_len as u32,
        }
    }

    pub fn for_path_query(path_query: &PathQuery, returned_values: &[Vec<u8>]) -> Self {
        SizesOfQueryOperation {
            key_size: path_query
                .query
                .query
                .items
                .iter()
                .map(|query_item| query_item.processing_footprint())
                .sum(),
            path_size: path_query.path.len() as u32,
            value_size: returned_values.iter().map(|v| v.len() as u32).sum(),
        }
    }

    pub fn for_empty_path_query(path_query: &PathQuery) -> Self {
        SizesOfQueryOperation {
            key_size: path_query
                .query
                .query
                .items
                .iter()
                .map(|query_item| query_item.processing_footprint())
                .sum(),
            path_size: path_query.path.len() as u32,
            value_size: 0,
        }
    }
}

#[derive(Debug)]
pub struct SizesOfInsertOperation {
    pub path_size: u32,
    pub key_size: u16,
    pub value_size: u32,
}

#[derive(Debug)]
pub struct SizesOfDeleteOperation {
    pub path_size: u32,
    pub key_size: u16,
    pub value_size: u32,
    pub multiplier: u8,
}

impl SizesOfDeleteOperation {
    pub fn for_empty_tree(path_size: u32, key_size: u16, multiplier: u8) -> Self {
        SizesOfDeleteOperation {
            path_size,
            key_size,
            value_size: 0,
            multiplier,
        }
    }
    pub fn for_key_value(path_size: u32, key_size: u16, element: &Element, multiplier: u8) -> Self {
        let value_size = match element {
            Element::Item(item, _) => item.len(),
            Element::Reference(path, _, _) => path.encoding_length(),
            Element::Tree(..) => 32,
        } as u32;
        SizesOfDeleteOperation::for_key_value_size(path_size, key_size, value_size, multiplier)
    }

    pub fn for_key_value_size(
        path_size: u32,
        key_size: u16,
        value_size: u32,
        multiplier: u8,
    ) -> Self {
        SizesOfDeleteOperation {
            path_size,
            key_size,
            value_size,
            multiplier,
        }
    }
}

impl OperationCostConvert for SizesOfInsertOperation {
    fn cost(&self) -> OperationCost {
        OperationCost {
            seek_count: 0,
            storage_written_bytes: 0,
            storage_loaded_bytes: 0,
            storage_freed_bytes: 0,
            hash_byte_calls: 0,
            hash_node_calls: 0,
        }
    }
}

impl OperationCostConvert for SizesOfQueryOperation {
    fn cost(&self) -> OperationCost {
        OperationCost {
            seek_count: 0,
            storage_written_bytes: 0,
            storage_loaded_bytes: 0,
            storage_freed_bytes: 0,
            hash_byte_calls: 0,
            hash_node_calls: 0,
        }
    }
}

impl OperationCostConvert for SizesOfDeleteOperation {
    fn cost(&self) -> OperationCost {
        OperationCost {
            seek_count: 0,
            storage_written_bytes: 0,
            storage_loaded_bytes: 0,
            storage_freed_bytes: 0,
            hash_byte_calls: 0,
            hash_node_calls: 0,
        }
    }
}

#[derive(Debug)]
pub enum DriveOperation {
    GroveOperation(GroveDbOp),
    CalculatedCostOperation(OperationCost),
    ContractFetch,
    CostCalculationInsertOperation(SizesOfInsertOperation),
    CostCalculationDeleteOperation(SizesOfDeleteOperation),
    CostCalculationQueryOperation(SizesOfQueryOperation),
}

impl DriveOperation {
    pub fn consume_to_costs(
        drive_operation: Vec<DriveOperation>,
    ) -> Result<Vec<OperationCost>, Error> {
        drive_operation
            .into_iter()
            .map(|operation| operation.operation_cost())
            .collect()
    }

    pub fn operation_cost(self) -> Result<OperationCost, Error> {
        match self {
            GroveOperation(_) => Err(Error::Drive(DriveError::CorruptedCodeExecution(
                "grove operations must be executed, not directly transformed to costs",
            ))),
            CostCalculationInsertOperation(worst_case_insert_operation) => {
                Ok(worst_case_insert_operation.cost())
            }
            CostCalculationQueryOperation(worst_case_query_operation) => {
                Ok(worst_case_query_operation.cost())
            }
            CostCalculationDeleteOperation(worst_case_delete_operation) => {
                Ok(worst_case_delete_operation.cost())
            }
            CalculatedCostOperation(c) => Ok(c),
            ContractFetch => Ok(OperationCost {
                seek_count: 0,
                storage_written_bytes: 0,
                storage_loaded_bytes: 0,
                storage_freed_bytes: 0,
                hash_byte_calls: 0,
                hash_node_calls: 0,
            }),
        }
    }

    pub fn grovedb_operations_batch(insert_operations: &Vec<DriveOperation>) -> GroveDbOpBatch {
        let operations = insert_operations
            .iter()
            .filter_map(|op| match op {
                GroveOperation(grovedb_op) => Some(grovedb_op.clone()),
                _ => None,
            })
            .collect();
        GroveDbOpBatch::from_operations(operations)
    }

    pub fn for_empty_tree(
        path: Vec<Vec<u8>>,
        key: Vec<u8>,
        storage_flags: Option<&StorageFlags>,
    ) -> Self {
        let tree = match storage_flags {
            Some(storage_flags) => Element::empty_tree_with_flags(storage_flags.to_element_flags()),
            None => Element::empty_tree(),
        };

        DriveOperation::for_path_key_element(path, key, tree)
    }

    pub fn for_path_key_element(path: Vec<Vec<u8>>, key: Vec<u8>, element: Element) -> Self {
        GroveOperation(GroveDbOp::insert(path, key, element))
    }

    pub fn for_insert_path_key_value_size(path_size: u32, key_size: u16, value_size: u32) -> Self {
        CostCalculationInsertOperation(SizesOfInsertOperation {
            path_size,
            key_size,
            value_size,
        })
    }

    pub fn for_delete_path_key_value_size(
        path: Vec<Vec<u8>>,
        key_size: u16,
        value_size: u32,
        multiplier: u8,
    ) -> Self {
        let path_sizes: Vec<u16> = path.into_iter().map(|x| x.len() as u16).collect();
        Self::for_delete_path_key_value_max_sizes(path_sizes, key_size, value_size, multiplier)
    }

    pub fn for_delete_path_key_value_max_sizes(
        path: Vec<u16>,
        key_size: u16,
        value_size: u32,
        multiplier: u8,
    ) -> Self {
        let path_size: u32 = path.into_iter().map(|x| x as u32).sum();
        CostCalculationDeleteOperation(SizesOfDeleteOperation::for_key_value_size(
            path_size, key_size, value_size, multiplier,
        ))
    }

    pub fn for_query_path_key_value_size(path_size: u32, key_size: u32, value_size: u32) -> Self {
        CostCalculationQueryOperation(SizesOfQueryOperation {
            path_size,
            key_size,
            value_size,
        })
    }
}

pub trait DriveCost {
    fn ephemeral_cost(&self) -> Result<u64, Error>;
    fn storage_cost(&self) -> Result<i64, Error>;
}

fn get_overflow_error(str: &'static str) -> Error {
    Error::Fee(FeeError::Overflow(str))
}

impl DriveCost for OperationCost {
    fn ephemeral_cost(&self) -> Result<u64, Error> {
        let OperationCost {
            seek_count,
            storage_written_bytes,
            storage_loaded_bytes,
            storage_freed_bytes: _,
            hash_byte_calls,
            hash_node_calls,
        } = *self;
        let seek_cost = (seek_count as u64)
            .checked_mul(STORAGE_SEEK_COST)
            .ok_or_else(|| get_overflow_error("seek cost overflow"))?;
        let storage_written_bytes_ephemeral_cost = (storage_written_bytes as u64)
            .checked_mul(STORAGE_PROCESSING_CREDIT_PER_BYTE)
            .ok_or_else(|| get_overflow_error("storage written bytes cost overflow"))?;
        let _storage_loaded_bytes_cost = (storage_loaded_bytes as u64)
            .checked_mul(STORAGE_LOAD_CREDIT_PER_BYTE)
            .ok_or_else(|| get_overflow_error("storage loaded cost overflow"))?;
        let storage_loaded_bytes_cost = (storage_loaded_bytes as u64)
            .checked_mul(NON_STORAGE_LOAD_CREDIT_PER_BYTE)
            .ok_or_else(|| get_overflow_error("loaded bytes cost overflow"))?;
        let hash_byte_cost = (hash_byte_calls as u64)
            .checked_mul(HASH_BYTE_COST)
            .ok_or_else(|| get_overflow_error("hash byte cost overflow"))?;
        let hash_node_cost = (hash_node_calls as u64)
            .checked_mul(HASH_NODE_COST)
            .ok_or_else(|| get_overflow_error("hash node cost overflow"))?;
        let cost = seek_cost
            .checked_add(storage_written_bytes_ephemeral_cost)
            .map(|c| c.checked_add(storage_loaded_bytes_cost))
            .flatten()
            .map(|c| c.checked_add(storage_loaded_bytes_cost))
            .flatten()
            .map(|c| c.checked_add(hash_byte_cost))
            .flatten()
            .map(|c| c.checked_add(hash_node_cost))
            .flatten()
            .ok_or_else(|| get_overflow_error("ephemeral cost addition overflow"));
        cost
    }

    fn storage_cost(&self) -> Result<i64, Error> {
        let OperationCost {
            storage_written_bytes,
            ..
        } = *self;
        let storage_written_bytes_disk_cost = (storage_written_bytes as i64)
            .checked_mul(STORAGE_DISK_USAGE_CREDIT_PER_BYTE)
            .ok_or_else(|| get_overflow_error("storage written bytes cost overflow"));
        storage_written_bytes_disk_cost
    }
}
