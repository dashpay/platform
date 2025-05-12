use crate::drive::Drive;
use grovedb::Element::SumItem;

use crate::error::proof::ProofError;
use crate::error::Error;

use crate::verify::RootHash;

use dpp::data_contract::group::GroupSumPower;
use dpp::data_contract::GroupContractPosition;
use dpp::group::group_action_status::GroupActionStatus;
use dpp::identifier::Identifier;
use grovedb::{Element, GroveDb, TreeFeatureType, VerifyOptions};
use platform_version::version::PlatformVersion;

impl Drive {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn verify_action_signers_total_power_v0(
        proof: &[u8],
        contract_id: Identifier,
        group_contract_position: GroupContractPosition,
        action_status: Option<GroupActionStatus>,
        action_id: Identifier,
        action_signer_id: Identifier,
        verify_subset_of_proof: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, GroupActionStatus, GroupSumPower), Error> {
        let action_status = match action_status {
            Some(action_status) => action_status,
            None => {
                // We don't actually know the action status, we need to look it up from the proof
                let path_query = Drive::group_active_or_closed_action_query(
                    contract_id.to_buffer(),
                    group_contract_position,
                );
                let mut proved_key_values = GroveDb::verify_query_with_options(
                    proof,
                    &path_query,
                    VerifyOptions {
                        absence_proofs_for_non_existing_searched_keys: false,
                        verify_proof_succinctness: false,
                        include_empty_trees_in_result: true,
                    },
                    &platform_version.drive.grove_version,
                )?
                .1;

                if proved_key_values.len() != 2 {
                    return Err(Error::Proof(ProofError::CorruptedProof(format!(
                        "we should always get back group action statuses for open and closed, we got {}",
                        proved_key_values.len()
                    ))));
                }

                let Some(Element::Tree(active_root, _)) = proved_key_values.remove(0).2 else {
                    return Err(Error::Proof(ProofError::CorruptedProof(
                        "group active action should be returned".to_string(),
                    )));
                };
                let Some(Element::Tree(closed_root, _)) = proved_key_values.remove(0).2 else {
                    return Err(Error::Proof(ProofError::CorruptedProof(
                        "group closed action should be returned".to_string(),
                    )));
                };
                if active_root.is_some() && closed_root.is_some() {
                    return Err(Error::Proof(ProofError::CorruptedProof(
                        "group action should be either active or closed, but was both".to_string(),
                    )));
                }
                if active_root.is_none() && closed_root.is_none() {
                    return Err(Error::Proof(ProofError::CorruptedProof(
                        "group action should be either active or closed, but was neither"
                            .to_string(),
                    )));
                }
                if active_root.is_some() {
                    GroupActionStatus::ActionActive
                } else {
                    GroupActionStatus::ActionClosed
                }
            }
        };
        let path_query = Drive::group_active_or_closed_action_single_signer_query(
            contract_id.to_buffer(),
            group_contract_position,
            action_id.to_buffer(),
            action_status,
            action_signer_id.to_buffer(),
        );

        let (root_hash, tree_feature, mut proved_key_values) = if verify_subset_of_proof {
            GroveDb::verify_subset_query_get_parent_tree_info(
                proof,
                &path_query,
                &platform_version.drive.grove_version,
            )?
        } else {
            GroveDb::verify_query_get_parent_tree_info(
                proof,
                &path_query,
                &platform_version.drive.grove_version,
            )?
        };

        if proved_key_values.len() != 1 {
            return Err(Error::Proof(ProofError::CorruptedProof(format!(
                "we should always get back one group power, we got {}",
                proved_key_values.len()
            ))));
        }

        let path_key_optional_element_trio = proved_key_values.remove(0);

        let element = path_key_optional_element_trio.2;
        match element {
            Some(SumItem(..)) => {
                if let TreeFeatureType::SummedMerkNode(aggregate_power) = tree_feature {
                    Ok((root_hash, action_status, aggregate_power as GroupSumPower))
                } else {
                    Err(Error::Proof(ProofError::IncorrectProof(
                        "we expected a summed tree".to_string(),
                    )))
                }
            }
            None => Err(Error::Proof(ProofError::IncorrectProof(
                "we expect to get back the signing power".to_string(),
            ))),
            _ => Err(Error::Proof(ProofError::IncorrectProof(
                "element should be a sum tree representing total signed power".to_string(),
            ))),
        }
    }
}
