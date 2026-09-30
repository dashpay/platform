use crate::drive::contract::moderation::types::{
    ContractSettledDeletionEntry, ContractSettledDeletionsQuery,
};
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::data_contract::config::moderation::ContractSettledDeletion;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    #[inline(always)]
    pub(super) fn fetch_contract_settled_deletions_v0(
        &self,
        contract_id: Identifier,
        query: &ContractSettledDeletionsQuery,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<ContractSettledDeletionEntry>, Error> {
        self.fetch_document_records::<ContractSettledDeletion>(
            contract_id,
            query,
            transaction,
            platform_version,
        )
    }

    /// One record, read the way the transform of a moderation reads state: the operations of
    /// the read are added to `drive_operations` for billing.
    #[inline(always)]
    pub(super) fn fetch_contract_settled_deletion_add_to_operations_v0(
        &self,
        contract_id: Identifier,
        document_type_name: &str,
        document_id: Identifier,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<ContractSettledDeletion>, Error> {
        self.fetch_document_record_add_to_operations::<ContractSettledDeletion>(
            contract_id,
            document_type_name,
            document_id,
            transaction,
            drive_operations,
            platform_version,
        )
    }
}
