use crate::drive::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
use crate::drive::batch::GroveDbOpBatch;
use crate::drive::flags::StorageFlags;
use crate::drive::{Drive, RootTree};
use crate::error::Error;
use dpp::block::block_info::BlockInfo;
#[cfg(feature = "data-contract-cbor-conversions")]
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

impl Drive {
    /// Applies a contract CBOR. CBOR was originally used to serialize contracts, so a lot of tests
    /// need this method for deserialization
    pub fn apply_contract_cbor(
        &self,
        contract_cbor: Vec<u8>,
        contract_id: Option<[u8; 32]>,
        block_info: BlockInfo,
        apply: bool,
        storage_flags: Option<Cow<StorageFlags>>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        // first we need to deserialize the contract
        let contract = DataContract::from_cbor_with_id(
            contract_cbor,
            contract_id.map(Identifier::from),
            platform_version,
        )?;

        self.apply_contract(
            &contract,
            block_info,
            apply,
            storage_flags,
            transaction,
            platform_version,
        )
    }
}
