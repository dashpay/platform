use crate::drive::contract::moderation::types::{
    ContractDocumentRecords, ContractDocumentRemovalsQuery,
};
use crate::drive::Drive;
use crate::error::Error;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    #[inline(always)]
    pub(super) fn prove_contract_document_removals_v0(
        &self,
        contract_id: Identifier,
        query: &ContractDocumentRemovalsQuery,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        self.prove_document_records(
            contract_id,
            ContractDocumentRecords::Removals,
            query,
            transaction,
            platform_version,
        )
    }
}
