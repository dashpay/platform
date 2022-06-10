use crate::drive::defaults::EMPTY_TREE_STORAGE_SIZE;
use enum_map::{enum_map, Enum, EnumMap};
use grovedb::{Element, PathQuery};

pub(crate) const STORAGE_CREDIT_PER_BYTE: u64 = 5000;
pub(crate) const STORAGE_PROCESSING_CREDIT_PER_BYTE: u64 = 10;
pub(crate) const QUERY_CREDIT_PER_BYTE: u64 = 10;

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
    pub fn cost(&self, word_count: u32) {}
}

pub struct QueryOperation {
    pub key_size: u32,
    pub path_size: u32,
    pub value_size: u32,
}

impl QueryOperation {
    pub fn for_key_check_in_path<'a: 'b, 'b, 'c, P>(key_len: usize, path: P) -> Self
    where
        P: IntoIterator<Item = &'c [u8]>,
        <P as IntoIterator>::IntoIter: ExactSizeIterator + DoubleEndedIterator + Clone,
    {
        let path_size: u32 = path
            .into_iter()
            .map(|inner: &[u8]| inner.len() as u32)
            .sum();
        QueryOperation {
            key_size: key_len as u32,
            path_size,
            value_size: 0,
        }
    }

    pub fn for_key_check_with_path_length(key_len: usize, path_len: usize) -> Self {
        QueryOperation {
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
        QueryOperation {
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
        QueryOperation {
            key_size: key_len as u32,
            path_size: path_len as u32,
            value_size: value_len as u32,
        }
    }

    pub fn for_path_query(path_query: &PathQuery, returned_values: &[Vec<u8>]) -> Self {
        QueryOperation {
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
        QueryOperation {
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

    pub fn data_size(&self) -> u32 {
        self.path_size + self.key_size + self.value_size as u32
    }

    pub fn ephemeral_cost(&self) -> u64 {
        self.data_size() as u64 * QUERY_CREDIT_PER_BYTE
    }
}

pub struct InsertOperation {
    pub key_size: u16,
    pub value_size: u32,
}

impl InsertOperation {
    pub fn for_empty_tree(key_size: usize) -> Self {
        InsertOperation {
            key_size: key_size as u16,
            value_size: EMPTY_TREE_STORAGE_SIZE as u32,
        }
    }
    pub fn for_key_value(key_size: usize, element: &Element) -> Self {
        let value_size = element.node_byte_size(key_size);
        InsertOperation {
            key_size: key_size as u16,
            value_size: value_size as u32,
        }
    }

    pub fn for_key_value_size(key_size: usize, value_size: usize) -> Self {
        let node_value_size = Element::calculate_node_byte_size(value_size, key_size);
        InsertOperation {
            key_size: key_size as u16,
            value_size: node_value_size as u32,
        }
    }

    pub fn data_size(&self) -> u32 {
        self.value_size + self.key_size as u32
    }

    pub fn ephemeral_cost(&self) -> u64 {
        self.data_size() as u64 * STORAGE_PROCESSING_CREDIT_PER_BYTE
    }

    pub fn storage_cost(&self) -> i64 {
        self.data_size() as i64 * STORAGE_CREDIT_PER_BYTE as i64
    }
}

pub struct DeleteOperation {
    pub key_size: u16,
    pub value_size: u32,
    pub multiplier: u64,
}

impl DeleteOperation {
    pub fn for_empty_tree(key_size: usize, multiplier: u64) -> Self {
        DeleteOperation {
            key_size: key_size as u16,
            value_size: 0,
            multiplier,
        }
    }
    pub fn for_key_value(key_size: usize, element: &Element, multiplier: u64) -> Self {
        let value_size = match element {
            Element::Item(item, _) => item.len(),
            Element::Reference(path, _) => path.iter().map(|inner| inner.len()).sum(),
            Element::Tree(..) => 32,
        };
        DeleteOperation {
            key_size: key_size as u16,
            value_size: value_size as u32,
            multiplier,
        }
    }

    pub fn data_size(&self) -> u32 {
        self.value_size + self.key_size as u32
    }

    pub fn ephemeral_cost(&self) -> u64 {
        self.data_size() as u64 * STORAGE_PROCESSING_CREDIT_PER_BYTE
    }

    pub fn storage_cost(&self) -> i64 {
        -(self.data_size() as i64 * STORAGE_CREDIT_PER_BYTE as i64)
    }
}
