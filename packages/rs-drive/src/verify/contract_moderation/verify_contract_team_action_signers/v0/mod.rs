use crate::drive::Drive;
use crate::error::proof::ProofError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::group::group_action_status::GroupActionStatus;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::{Element, GroveDb};

impl Drive {
    pub(super) fn verify_contract_team_action_signers_v0(
        proof: &[u8],
        contract_id: Identifier,
        status: GroupActionStatus,
        action_id: Identifier,
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<Identifier>), Error> {
        let path_query = Self::contract_team_action_signers_query(
            contract_id.to_buffer(),
            status,
            action_id.to_buffer(),
        );
        let (root_hash, proved_key_values) = if verify_subset_of_proof {
            GroveDb::verify_subset_query(proof, &path_query, &platform_version.drive.grove_version)?
        } else {
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)?
        };

        let signers = proved_key_values
            .into_iter()
            .filter_map(|(_path, key, element)| element.map(|element| (key, element)))
            .map(|(key, element)| {
                let malformed = || {
                    Error::Proof(ProofError::CorruptedProof(format!(
                        "contract team action signers proof holds a malformed approval {:?}",
                        key
                    )))
                };
                if !matches!(element, Element::SumItem(..)) {
                    return Err(malformed());
                }
                Identifier::from_bytes(&key).map_err(|_| malformed())
            })
            .collect::<Result<Vec<_>, Error>>()?;

        Ok((root_hash, signers))
    }
}
