use crate::drive::contract::moderation::types::ContractDocumentRemovalsQuery;
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
        Self::check_contract_document_removals_query(query, platform_version)?;
        let path_query = Self::contract_document_removals_query(contract_id.to_buffer(), query);
        self.grove_get_proved_path_query(
            &path_query,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )
    }
}
