use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
use crate::util::batch::GroveDbOpBatch;

use crate::drive::RootTree;

#[cfg(feature = "data-contract-cbor-conversion")]
use dpp::data_contract::conversion::cbor::DataContractCborConversionMethodsV0;

/// Adds operations to the op batch relevant to initializing the contract's structure.
/// Namely it inserts an empty tree at the contract's root path.
pub fn add_init_contracts_structure_operations(batch: &mut GroveDbOpBatch) {
    batch.add_insert_empty_tree(vec![], vec![RootTree::DataContractDocuments as u8]);
}
