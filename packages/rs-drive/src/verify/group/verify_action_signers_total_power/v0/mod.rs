use crate::drive::group::paths::{
    group_path_vec, GROUP_ACTIVE_ACTIONS_KEY, GROUP_CLOSED_ACTIONS_KEY,
};
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

fn proof_entry_matches(
    path: &[Vec<u8>],
    key: &[u8],
    parent: &[Vec<u8>],
    expected_key: &[u8],
) -> bool {
    // Legacy proofs append a nonempty terminal tree's key to its path; both shapes name
    // the same location authenticated by the query.
    key == expected_key
        && (path == parent
            || path
                .split_last()
                .is_some_and(|(last, prefix)| prefix == parent && last.as_slice() == expected_key))
}

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
        let mut inferred_root = None;
        let action_status = match action_status {
            Some(action_status) => action_status,
            None => {
                // This client verifier is not called by block validation or execution. Status
                // discovery must refer to the action, since both group branches may hold actions.
                let signer_query = Drive::group_active_and_closed_action_single_signer_query(
                    contract_id.to_buffer(),
                    group_contract_position,
                    action_id.to_buffer(),
                    action_signer_id.to_buffer(),
                );
                let canonical_root = GroveDb::verify_query_with_options(
                    proof,
                    &signer_query,
                    VerifyOptions {
                        absence_proofs_for_non_existing_searched_keys: false,
                        verify_proof_succinctness: !verify_subset_of_proof,
                        include_empty_trees_in_result: false,
                    },
                    &platform_version.drive.grove_version,
                )?
                .0;
                let path_query = Drive::group_active_or_closed_action_query(
                    contract_id.to_buffer(),
                    group_contract_position,
                );
                // Terminal projection authenticates empty branch claims even in legacy proofs,
                // whose subquery verifier can otherwise skip them without binding their bytes.
                let (group_root, proved_key_values) = GroveDb::verify_query_with_options(
                    proof,
                    &path_query,
                    VerifyOptions {
                        absence_proofs_for_non_existing_searched_keys: false,
                        verify_proof_succinctness: false,
                        include_empty_trees_in_result: true,
                    },
                    &platform_version.drive.grove_version,
                )?;

                if group_root != canonical_root {
                    return Err(Error::Proof(ProofError::CorruptedProof(
                        "group branch proof root does not match signer proof root".to_string(),
                    )));
                }

                if proved_key_values.len() != 2 {
                    return Err(Error::Proof(ProofError::CorruptedProof(format!(
                        "we should always get back group action statuses for open and closed, we got {}",
                        proved_key_values.len()
                    ))));
                }

                let group_path = group_path_vec(contract_id.as_bytes(), group_contract_position);
                let branch_keys = [GROUP_ACTIVE_ACTIONS_KEY, GROUP_CLOSED_ACTIONS_KEY];
                let mut group_roots = [None, None];
                for (path, key, element) in proved_key_values {
                    let branch = branch_keys
                        .iter()
                        .position(|expected_key| {
                            proof_entry_matches(&path, &key, &group_path, expected_key.as_slice())
                        })
                        .ok_or_else(|| {
                            Error::Proof(ProofError::CorruptedProof(
                                "unexpected group branch in proof".to_string(),
                            ))
                        })?;
                    let Some(Element::Tree(root, _)) = element else {
                        return Err(Error::Proof(ProofError::CorruptedProof(
                            "group action branch must be a tree".to_string(),
                        )));
                    };
                    if group_roots[branch].replace(root.is_some()).is_some() {
                        return Err(Error::Proof(ProofError::CorruptedProof(
                            "duplicate group branch in proof".to_string(),
                        )));
                    }
                }
                let [Some(active_nonempty), Some(closed_nonempty)] = group_roots else {
                    return Err(Error::Proof(ProofError::CorruptedProof(
                        "proof must contain both group branches".to_string(),
                    )));
                };

                let action_query = Drive::group_active_and_closed_action_query(
                    contract_id.to_buffer(),
                    group_contract_position,
                    action_id.to_buffer(),
                );
                let (action_root, actions) = GroveDb::verify_query_with_options(
                    proof,
                    &action_query,
                    VerifyOptions {
                        absence_proofs_for_non_existing_searched_keys: false,
                        verify_proof_succinctness: false,
                        include_empty_trees_in_result: true,
                    },
                    &platform_version.drive.grove_version,
                )?;
                if action_root != canonical_root {
                    return Err(Error::Proof(ProofError::CorruptedProof(
                        "action proof root does not match signer proof root".to_string(),
                    )));
                }
                let action_parents = branch_keys.map(|key| {
                    let mut path = group_path.clone();
                    path.push(key.to_vec());
                    path
                });
                let group_nonempty = [active_nonempty, closed_nonempty];
                let mut action_present = [false, false];
                for (path, key, element) in actions {
                    let branch = action_parents
                        .iter()
                        .position(|parent| {
                            proof_entry_matches(&path, &key, parent, action_id.as_bytes())
                        })
                        .ok_or_else(|| {
                            Error::Proof(ProofError::CorruptedProof(
                                "unexpected action in proof".to_string(),
                            ))
                        })?;
                    if !matches!(element, Some(Element::Tree(Some(_), _))) {
                        return Err(Error::Proof(ProofError::CorruptedProof(
                            "group action must be a nonempty tree".to_string(),
                        )));
                    }
                    if !group_nonempty[branch] || action_present[branch] {
                        return Err(Error::Proof(ProofError::CorruptedProof(
                            "inconsistent group action branch in proof".to_string(),
                        )));
                    }
                    action_present[branch] = true;
                }
                inferred_root = Some(canonical_root);
                match action_present {
                    [true, false] => GroupActionStatus::ActionActive,
                    [false, true] => GroupActionStatus::ActionClosed,
                    [true, true] => {
                        return Err(Error::Proof(ProofError::CorruptedProof(
                            "group action should be either active or closed, but was both"
                                .to_string(),
                        )))
                    }
                    [false, false] => {
                        return Err(Error::Proof(ProofError::CorruptedProof(
                            "group action should be either active or closed, but was neither"
                                .to_string(),
                        )))
                    }
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

        // An inferred status has already checked strictness on the full two-branch query.
        // The selected signer is a narrower projection of those same authenticated bytes.
        let (root_hash, tree_feature, mut proved_key_values) =
            if verify_subset_of_proof || inferred_root.is_some() {
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

        if inferred_root.is_some_and(|expected| expected != root_hash) {
            return Err(Error::Proof(ProofError::CorruptedProof(
                "signer aggregate proof root does not match action proof root".to_string(),
            )));
        }

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

#[cfg(all(test, feature = "full"))]
mod status_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
    use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
    use dpp::data_contract::config::v0::DataContractConfigV0;
    use dpp::data_contract::config::DataContractConfig;
    use dpp::data_contract::group::v0::GroupV0;
    use dpp::data_contract::group::Group;
    use dpp::data_contract::v1::DataContractV1;
    use dpp::data_contract::DataContract;
    use dpp::group::action_event::GroupActionEvent;
    use dpp::group::group_action::v0::GroupActionV0;
    use dpp::group::group_action::GroupAction;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::Identity;
    use dpp::tokens::token_event::TokenEvent;
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

    #[test]
    fn should_prove_and_verify_action_signers_total_power() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let identity_1 = Identity::random_identity(3, Some(14), platform_version)
            .expect("expected a platform identity");
        let identity_1_id = identity_1.id();

        let identity_2 = Identity::random_identity(3, Some(506), platform_version)
            .expect("expected a platform identity");
        let identity_2_id = identity_2.id();

        // Create a data contract with groups
        let contract = DataContract::V1(DataContractV1 {
            id: Default::default(),
            version: 0,
            owner_id: Default::default(),
            document_types: Default::default(),
            config: DataContractConfig::V0(DataContractConfigV0 {
                can_be_deleted: false,
                readonly: false,
                keeps_history: false,
                documents_keep_history_contract_default: false,
                documents_mutable_contract_default: false,
                documents_can_be_deleted_contract_default: false,
                requires_identity_encryption_bounded_key: None,
                requires_identity_decryption_bounded_key: None,
            }),
            schema_defs: None,
            created_at: None,
            updated_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            created_at_epoch: None,
            updated_at_epoch: None,
            groups: BTreeMap::from([(
                0,
                Group::V0(GroupV0 {
                    members: [(identity_1_id, 3), (identity_2_id, 5)].into(),
                    required_power: 6,
                }),
            )]),
            tokens: BTreeMap::from([(
                0,
                TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive()),
            )]),
            keywords: Vec::new(),
            description: None,
        });

        drive
            .insert_contract(
                &contract,
                BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to insert contract");

        let contract_id = contract.id();
        let group_contract_position = 0;
        let action_id = Identifier::random();

        let action = GroupAction::V0(GroupActionV0 {
            contract_id,
            proposer_id: identity_1_id,
            token_contract_position: 0,
            event: GroupActionEvent::TokenEvent(TokenEvent::Mint(100, identity_1_id, None)),
        });

        // Add action with identity_1 signing (power 3)
        drive
            .add_group_action(
                contract_id,
                group_contract_position,
                Some(action.clone()),
                false,
                action_id,
                identity_1_id,
                3,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add action with identity_1");

        // Add identity_2 signing the same action (power 5)
        drive
            .add_group_action(
                contract_id,
                group_contract_position,
                Some(action.clone()),
                false,
                action_id,
                identity_2_id,
                5,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add action with identity_2");

        // Produce a proof by querying the single signer path
        let path_query = Drive::group_active_or_closed_action_single_signer_query(
            contract_id.to_buffer(),
            group_contract_position,
            action_id.to_buffer(),
            GroupActionStatus::ActionActive,
            identity_1_id.to_buffer(),
        );

        let proof = drive
            .grove_get_proved_path_query(&path_query, None, &mut vec![], &platform_version.drive)
            .expect("should produce proof for single signer");

        // Verify the proof
        let (root_hash, action_status, total_power) = Drive::verify_action_signer_and_total_power(
            proof.as_slice(),
            contract_id,
            group_contract_position,
            Some(GroupActionStatus::ActionActive),
            action_id,
            identity_1_id,
            false,
            platform_version,
        )
        .expect("should verify action signers total power proof");

        assert!(!root_hash.is_empty(), "root hash should not be empty");
        assert_eq!(
            action_status,
            GroupActionStatus::ActionActive,
            "action should be active"
        );
        // Total power = 3 + 5 = 8
        assert_eq!(total_power, 8, "total power should be 8 (3 + 5)");
    }
}
