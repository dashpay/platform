use crate::drive::contract::moderation::types::{
    ContractModerationEntriesQuery, ContractModerationEntry,
};
use crate::drive::Drive;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::GroveDb;

impl Drive {
    pub(super) fn verify_contract_moderation_entries_v0(
        proof: &[u8],
        contract_id: Identifier,
        query: &ContractModerationEntriesQuery,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<ContractModerationEntry>), Error> {
        let path_query = Self::contract_moderation_entries_query(contract_id.to_buffer(), query);
        let (root_hash, proved_key_values) =
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)?;

        let entries = proved_key_values
            .into_iter()
            .filter_map(|(_path, key, element)| element.map(|element| (key, element)))
            .map(|(key, element)| {
                ContractModerationEntry::from_key_element(query.list, &key, &element).map_err(
                    |description| {
                        Error::Proof(ProofError::CorruptedProof(format!(
                            "contract moderation entries proof is malformed: {}",
                            description
                        )))
                    },
                )
            })
            .collect::<Result<Vec<_>, Error>>()?;

        Ok((root_hash, entries))
    }
}
