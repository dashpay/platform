use crate::drive::Drive;
use grovedb::Element::Item;

use crate::error::proof::ProofError;
use crate::error::Error;

use crate::verify::RootHash;

use dpp::data_contract::group::Group;
use dpp::data_contract::GroupContractPosition;
use dpp::identifier::Identifier;
use dpp::prelude::StartAtIncluded;
use dpp::serialization::PlatformDeserializable;
use grovedb::GroveDb;
use platform_version::version::PlatformVersion;

impl Drive {
    pub(super) fn verify_group_infos_in_contract_v0<
        T: FromIterator<(GroupContractPosition, Group)>,
    >(
        proof: &[u8],
        contract_id: Identifier,
        start_group_contract_position: Option<(GroupContractPosition, StartAtIncluded)>,
        limit: Option<u16>,
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, T), Error> {
        let path_query = Self::group_infos_for_contract_id_query(
            contract_id.to_buffer(),
            start_group_contract_position,
            limit,
        );
        let (root_hash, proved_key_values) = if verify_subset_of_proof {
            GroveDb::verify_subset_query(proof, &path_query, &platform_version.drive.grove_version)?
        } else {
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)?
        };
        let values = proved_key_values
            .into_iter()
            .filter_map(|(path, _, element)| {
                let key_bytes = match path.last() {
                    Some(contract_position_bytes) => contract_position_bytes,
                    None => {
                        return Some(Err(Error::Proof(ProofError::CorruptedProof(
                            "path can't be empty in proof".to_string(),
                        ))))
                    }
                };
                let key_bytes: [u8; 2] = match key_bytes.clone().try_into().map_err(|_| {
                    Error::Proof(ProofError::IncorrectValueSize(
                        "group contract position incorrect size",
                    ))
                }) {
                    Ok(bytes) => bytes,

                    Err(e) => return Some(Err(e)),
                };
                let group_contract_position: GroupContractPosition =
                    GroupContractPosition::from_be_bytes(key_bytes);
                match element {
                    Some(Item(value, ..)) => {
                        let group = match Group::deserialize_from_bytes(&value) {
                            Ok(group) => group,

                            Err(e) => return Some(Err(e.into())),
                        };
                        Some(Ok((group_contract_position, group)))
                    }
                    None => None,
                    Some(element) => Some(Err(Error::Proof(ProofError::IncorrectProof(format!(
                        "group should be in an item, however a {} was returned",
                        element.type_str()
                    ))))),
                }
            })
            .collect::<Result<T, Error>>()?;
        Ok((root_hash, values))
    }
}
