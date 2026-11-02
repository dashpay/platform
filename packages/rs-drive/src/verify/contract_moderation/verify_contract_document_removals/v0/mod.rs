use crate::drive::contract::moderation::types::{
    ContractDocumentRemovalEntry, ContractDocumentRemovalsQuery,
};
use crate::drive::Drive;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::GroveDb;

impl Drive {
    pub(super) fn verify_contract_document_removals_v0(
        proof: &[u8],
        contract_id: Identifier,
        query: &ContractDocumentRemovalsQuery,
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<ContractDocumentRemovalEntry>), Error> {
        let path_query = Self::contract_document_removals_query(contract_id.to_buffer(), query);
        let (root_hash, proved_key_values) = if verify_subset_of_proof {
            GroveDb::verify_subset_query(proof, &path_query, &platform_version.drive.grove_version)?
        } else {
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)?
        };

        let entries = proved_key_values
            .into_iter()
            .filter_map(|(_path, key, element)| element.map(|element| (key, element)))
            .map(|(key, element)| {
                ContractDocumentRemovalEntry::from_key_element(&key, &element).map_err(
                    |description| {
                        Error::Proof(ProofError::CorruptedProof(format!(
                            "contract document removals proof is malformed: {}",
                            description
                        )))
                    },
                )
            })
            .collect::<Result<Vec<_>, Error>>()?;

        Ok((root_hash, entries))
    }
}
