use crate::drive::contract::moderation::types::{
    encode_settled_deletion, estimated_settled_deletion_value_size, ContractDocumentRecords,
};
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::config::moderation::ContractSettledDeletion;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// The record, flagged with `moderator_id`, the approval's signer, and replaced in place by
    /// every later approval (see `add_document_record_operations` for how its flags follow). An
    /// estimate prices every write as a fresh insert of the whole record.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_contract_settled_deletion_operations_v0(
        &self,
        contract_id: Identifier,
        document_type_name: &str,
        document_id: Identifier,
        settled_deletion: &ContractSettledDeletion,
        replaces_existing: bool,
        moderator_id: Identifier,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        self.add_document_record_operations(
            contract_id,
            ContractDocumentRecords::SettledDeletions,
            document_type_name,
            document_id,
            encode_settled_deletion(settled_deletion),
            replaces_existing,
            estimated_settled_deletion_value_size(),
            moderator_id,
            block_info,
            estimated_costs_only_with_layer_info,
            transaction,
            platform_version,
        )
    }
}
