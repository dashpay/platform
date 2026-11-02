use crate::drive::contract::moderation::types::{
    ContractTeamActionEntry, ContractTeamActionsQuery,
};
use crate::drive::Drive;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::GroveDb;

impl Drive {
    pub(super) fn verify_contract_team_actions_v0(
        proof: &[u8],
        contract_id: Identifier,
        query: &ContractTeamActionsQuery,
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<ContractTeamActionEntry>), Error> {
        let path_query = Self::contract_team_actions_query(contract_id.to_buffer(), query);
        let (root_hash, proved_key_values) = if verify_subset_of_proof {
            GroveDb::verify_subset_query(proof, &path_query, &platform_version.drive.grove_version)?
        } else {
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)?
        };

        let entries = ContractTeamActionEntry::from_path_key_elements(
            proved_key_values
                .into_iter()
                .filter_map(|(path, key, element)| element.map(|element| (path, key, element))),
        )
        .map_err(|description| {
            Error::Proof(ProofError::CorruptedProof(format!(
                "contract team actions proof is malformed: {}",
                description
            )))
        })?;

        Ok((root_hash, entries))
    }
}
