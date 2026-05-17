use crate::drive::Drive;
use grovedb::Element::SumItem;

use crate::error::proof::ProofError;
use crate::error::Error;

use crate::verify::RootHash;

use dpp::data_contract::group::GroupMemberPower;
use dpp::data_contract::GroupContractPosition;
use dpp::group::group_action_status::GroupActionStatus;
use dpp::identifier::Identifier;
use grovedb::GroveDb;
use platform_version::version::PlatformVersion;

impl Drive {
    pub(super) fn verify_action_signers_v0<T: FromIterator<(Identifier, GroupMemberPower)>>(
        proof: &[u8],
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        action_status: GroupActionStatus,
        action_id: Identifier,
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, T), Error> {
        let path_query = Drive::group_action_signers_query(
            contract_id.to_buffer(),
            group_contract_position,
            action_status,
            action_id.to_buffer(),
        );

        let (root_hash, proved_key_values) = if verify_subset_of_proof {
            GroveDb::verify_subset_query(proof, &path_query, &platform_version.drive.grove_version)?
        } else {
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)?
        };
        let values = proved_key_values
            .into_iter()
            .filter_map(|(_, key, element)| {
                let id: Identifier = match key.try_into() {
                    Ok(id) => id,
                    Err(_) => {
                        return Some(Err(Error::Proof(ProofError::IncorrectProof(
                            "identifier was not 32 bytes long".to_string(),
                        ))))
                    }
                };
                match element {
                    Some(SumItem(value, ..)) => {
                        let signing_power: GroupMemberPower = match value.try_into() {
                            Ok(signing_power) => signing_power,
                            Err(_) => {
                                return Some(Err(Error::Proof(ProofError::IncorrectProof(
                                    "signed power should be encodable on a u32 integer".to_string(),
                                ))))
                            }
                        };

                        Some(Ok((id, signing_power)))
                    }
                    None => None,
                    _ => Some(Err(Error::Proof(ProofError::IncorrectProof(
                        "element should be a sum item representing member signed power".to_string(),
                    )))),
                }
            })
            .collect::<Result<T, Error>>()?;
        Ok((root_hash, values))
    }
}
