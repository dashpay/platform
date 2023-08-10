use crate::drive::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
use crate::drive::batch::GroveDbOpBatch;
use crate::drive::flags::StorageFlags;
use crate::drive::{Drive, RootTree};
use crate::error::Error;
use dpp::block::block_info::BlockInfo;
#[cfg(feature = "data-contract-cbor-conversion")]
use dpp::data_contract::conversion::cbor::DataContractCborConversionMethodsV0;
use dpp::data_contract::DataContract;
use dpp::fee::fee_result::FeeResult;
use dpp::platform_value::Identifier;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;
use std::borrow::Cow;

/// Adds operations to the op batch relevant to initializing the contract's structure.
/// Namely it inserts an empty tree at the contract's root path.
pub fn add_init_contracts_structure_operations(batch: &mut GroveDbOpBatch) {
    batch.add_insert_empty_tree(vec![], vec![RootTree::DataContractDocuments as u8]);
}
