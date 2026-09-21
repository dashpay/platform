use grovedb::batch::KeyInfoPath;

use grovedb::{EstimatedLayerInformation, TransactionArg};

use std::collections::HashMap;

use dpp::block::block_info::BlockInfo;
use dpp::data_contract::DataContract;

use crate::drive::Drive;

use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;

use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::identifier::Identifier;

use dpp::version::PlatformVersion;

impl Drive {
    /// Prepares the operations for deleting a document named by its type
    /// through the entry point that carries only the block time.
    ///
    /// Generation 1 records a lifecycle entry for a keep-history document. That
    /// entry names the deletion time, which this signature carries, and credits
    /// the record's bytes to the deleter, which it does not: an entry written
    /// through here belongs to nobody, as with the fee-applying wrappers, and
    /// refunds nobody when erased.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn delete_document_for_contract_with_named_type_operations_v1(
        &self,
        document_id: Identifier,
        contract: &DataContract,
        document_type_name: &str,
        previous_batch_operations: Option<&mut Vec<LowLevelDriveOperation>>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        block_time_ms: u64,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        self.delete_document_for_contract_with_named_type_operations_with_lifecycle_v1(
            document_id,
            contract,
            document_type_name,
            &BlockInfo::default_with_time(block_time_ms),
            None,
            previous_batch_operations,
            estimated_costs_only_with_layer_info,
            block_time_ms,
            transaction,
            platform_version,
        )
    }

    /// Prepares the operations for deleting a document.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn delete_document_for_contract_with_named_type_operations_with_lifecycle_v1(
        &self,
        document_id: Identifier,
        contract: &DataContract,
        document_type_name: &str,
        block_info: &BlockInfo,
        deleter_id: Option<Identifier>,
        previous_batch_operations: Option<&mut Vec<LowLevelDriveOperation>>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        block_time_ms: u64,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let document_type = contract.document_type_for_name(document_type_name)?;
        self.delete_document_for_contract_operations_with_lifecycle(
            document_id,
            contract,
            document_type,
            block_info,
            deleter_id,
            previous_batch_operations,
            estimated_costs_only_with_layer_info,
            block_time_ms,
            transaction,
            platform_version,
        )
    }
}
