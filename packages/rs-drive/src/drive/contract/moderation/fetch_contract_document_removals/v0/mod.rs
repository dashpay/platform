use crate::drive::contract::moderation::types::{
    ContractDocumentRemovalEntry, ContractDocumentRemovalsQuery,
};
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::data_contract::config::moderation::ContractDocumentRemoval;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    #[inline(always)]
    pub(super) fn fetch_contract_document_removals_v0(
        &self,
        contract_id: Identifier,
        query: &ContractDocumentRemovalsQuery,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<ContractDocumentRemovalEntry>, Error> {
        self.fetch_document_records::<ContractDocumentRemoval>(
            contract_id,
            query,
            transaction,
            platform_version,
        )
    }

    /// One record, read the way the transform of a moderation reads state: the operations of
    /// the read are added to `drive_operations` for billing. Also how Drive reads the owner of
    /// a removed document a derived index property reads through a `moderatedDocument`
    /// reference.
    #[inline(always)]
    pub(crate) fn fetch_contract_document_removal_add_to_operations_v0(
        &self,
        contract_id: Identifier,
        document_type_name: &str,
        document_id: Identifier,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<ContractDocumentRemoval>, Error> {
        self.fetch_document_record_add_to_operations::<ContractDocumentRemoval>(
            contract_id,
            document_type_name,
            document_id,
            transaction,
            drive_operations,
            platform_version,
        )
    }
}
