use crate::drive::contract::moderation::types::{
    ContractSettledDeletionEntry, ContractSettledDeletionsQuery,
};
use crate::drive::Drive;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::data_contract::config::moderation::ContractSettledDeletion;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;

impl Drive {
    pub(super) fn verify_contract_settled_deletions_v0(
        proof: &[u8],
        contract_id: Identifier,
        query: &ContractSettledDeletionsQuery,
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<ContractSettledDeletionEntry>), Error> {
        Self::verify_document_records::<ContractSettledDeletion>(
            proof,
            contract_id,
            query,
            verify_subset_of_proof,
            platform_version,
        )
    }
}
