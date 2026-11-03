use crate::drive::contract_groups::types::ContractGroupMembershipsForContract;
use crate::drive::Drive;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::identifier::Identifier;
use grovedb::GroveDb;
use platform_version::version::PlatformVersion;

impl Drive {
    pub(super) fn verify_contract_group_memberships_for_contract_v0(
        proof: &[u8],
        contract_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, ContractGroupMembershipsForContract), Error> {
        let path_query =
            Self::contract_group_memberships_for_contract_query(contract_id.to_buffer());
        let (root_hash, proved_key_values) =
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)?;

        let present = proved_key_values
            .into_iter()
            .filter_map(|(path, key, element)| element.map(|element| (path, key, element)));

        let memberships = ContractGroupMembershipsForContract::from_path_key_elements(present)
            .map_err(|description| {
                Error::Proof(ProofError::CorruptedProof(format!(
                    "contract group memberships proof is malformed: {}",
                    description
                )))
            })?;

        Ok((root_hash, memberships))
    }
}
