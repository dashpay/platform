use crate::drive::contract_groups::types::{ContractGroupMembersPage, ContractGroupMembersQuery};
use crate::drive::Drive;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::identifier::Identifier;
use grovedb::GroveDb;
use platform_version::version::PlatformVersion;

impl Drive {
    pub(super) fn verify_contract_group_members_v0(
        proof: &[u8],
        contract_group_id: Identifier,
        query: &ContractGroupMembersQuery,
        limit: u16,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, ContractGroupMembersPage), Error> {
        let path_query =
            Self::contract_group_members_query(contract_group_id.to_buffer(), query, limit);
        let (root_hash, proved_key_values) =
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)?;

        let present = proved_key_values
            .into_iter()
            .filter_map(|(path, key, element)| element.map(|element| (path, key, element)));

        let page = ContractGroupMembersPage::from_path_key_elements(query, present).map_err(
            |description| {
                Error::Proof(ProofError::CorruptedProof(format!(
                    "contract group members proof is malformed: {}",
                    description
                )))
            },
        )?;

        Ok((root_hash, page))
    }
}
