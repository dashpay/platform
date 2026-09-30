use crate::drive::contract::moderation::types::{
    ContractDocumentRemovalEntry, ContractDocumentRemovalsQuery,
};
use crate::drive::Drive;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::data_contract::config::moderation::ContractDocumentRemoval;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;

impl Drive {
    pub(super) fn verify_contract_document_removals_v0(
        proof: &[u8],
        contract_id: Identifier,
        query: &ContractDocumentRemovalsQuery,
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<ContractDocumentRemovalEntry>), Error> {
        Self::verify_document_records::<ContractDocumentRemoval>(
            proof,
            contract_id,
            query,
            verify_subset_of_proof,
            platform_version,
        )
    }
}
