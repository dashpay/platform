use crate::drive::Drive;

use crate::error::proof::ProofError;
use crate::error::Error;

use crate::verify::RootHash;

use dpp::data_contract::group::Group;
use dpp::data_contract::GroupContractPosition;
use dpp::identifier::Identifier;
use dpp::serialization::PlatformDeserializable;
use grovedb::GroveDb;
use platform_version::version::PlatformVersion;

impl Drive {
    pub(super) fn verify_group_info_v0(
        proof: &[u8],
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<Group>), Error> {
        let path_query = Self::group_info_for_contract_id_and_group_contract_position_query(
            contract_id.to_buffer(),
            group_contract_position,
        );
        let (root_hash, mut proved_key_values) = if verify_subset_of_proof {
            GroveDb::verify_subset_query_with_absence_proof(
                proof,
                &path_query,
                &platform_version.drive.grove_version,
            )?
        } else {
            GroveDb::verify_query_with_absence_proof(
                proof,
                &path_query,
                &platform_version.drive.grove_version,
            )?
        };

        if proved_key_values.len() != 1 {
            return Err(Error::Proof(ProofError::CorruptedProof(
                "we should always get back one element".to_string(),
            )));
        }

        let element = proved_key_values.remove(0).2;

        let group = element
            .map(|element| element.into_item_bytes().map_err(Error::GroveDB))
            .transpose()?
            .map(|bytes| Group::deserialize_from_bytes(&bytes))
            .transpose()?;

        Ok((root_hash, group))
    }
}
