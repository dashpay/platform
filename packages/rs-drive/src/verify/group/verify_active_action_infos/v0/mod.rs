use crate::drive::Drive;
use grovedb::Element::Item;

use crate::error::proof::ProofError;
use crate::error::Error;

use crate::verify::RootHash;

use dpp::data_contract::GroupContractPosition;
use dpp::group::group_action::GroupAction;
use dpp::group::group_action_status::GroupActionStatus;
use dpp::identifier::Identifier;
use dpp::prelude::StartAtIncluded;
use dpp::serialization::PlatformDeserializable;
use grovedb::GroveDb;
use platform_version::version::PlatformVersion;

impl Drive {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn verify_action_infos_in_contract_v0<T: FromIterator<(Identifier, GroupAction)>>(
        proof: &[u8],
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        action_status: GroupActionStatus,
        start_action_id: Option<(Identifier, StartAtIncluded)>,
        limit: Option<u16>,
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, T), Error> {
        let path_query = Drive::group_action_infos_query(
            contract_id.to_buffer(),
            group_contract_position,
            action_status,
            start_action_id.map(|(s, i)| (s.to_buffer(), i)),
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
                let Some(last_path_component) = path.last() else {
                    return Some(Err(Error::Proof(ProofError::IncorrectProof(
                        "last path component is empty".to_string(),
                    ))));
                };
                let action_id = match Identifier::from_bytes(last_path_component) {
                    Ok(action_id) => action_id,
                    Err(e) => return Some(Err(e.into())),
                };

                match element {
                    Some(Item(value, ..)) => {
                        let active_action = match GroupAction::deserialize_from_bytes(&value) {
                            Ok(active_action) => active_action,

                            Err(e) => return Some(Err(e.into())),
                        };
                        Some(Ok((action_id, active_action)))
                    }
                    None => None,
                    Some(element) => Some(Err(Error::Proof(ProofError::IncorrectProof(format!(
                        "group action should be in an item, however a {} was returned",
                        element.type_str()
                    ))))),
                }
            })
            .collect::<Result<T, Error>>()?;
        Ok((root_hash, values))
    }
}
