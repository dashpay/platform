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
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::Identity;
    use dpp::tokens::token_event::TokenEvent;
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

    #[test]
    fn should_prove_and_verify_action_infos_roundtrip() {
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
                    members: [(identity_1_id, 1), (identity_2_id, 1)].into(),
                    required_power: 2,
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

        let action_id_1 = Identifier::random();
        let action_id_2 = Identifier::random();

        let action_1 = GroupAction::V0(GroupActionV0 {
            contract_id,
            proposer_id: identity_1_id,
            token_contract_position: 0,
            event: GroupActionEvent::TokenEvent(TokenEvent::Mint(100, identity_1_id, None)),
        });

        let action_2 = GroupAction::V0(GroupActionV0 {
            contract_id,
            proposer_id: identity_2_id,
            token_contract_position: 0,
            event: GroupActionEvent::TokenEvent(TokenEvent::Burn(50, identity_2_id, None)),
        });

        drive
            .add_group_action(
                contract_id,
                group_contract_position,
                Some(action_1.clone()),
                false,
                action_id_1,
                identity_1_id,
                1,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add action 1");

        drive
            .add_group_action(
                contract_id,
                group_contract_position,
                Some(action_2.clone()),
                false,
                action_id_2,
                identity_2_id,
                1,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add action 2");

        // Prove using the public prove_action_infos method
        let proof = drive
            .prove_action_infos(
                contract_id,
                group_contract_position,
                GroupActionStatus::ActionActive,
                None,
                Some(10),
                None,
                platform_version,
            )
            .expect("should prove action infos");

        // Verify using the v0 verify function
        let (root_hash, proved_actions): (_, BTreeMap<Identifier, GroupAction>) =
            Drive::verify_action_infos_in_contract(
                proof.as_slice(),
                contract_id,
                group_contract_position,
                GroupActionStatus::ActionActive,
                None,
                Some(10),
                false,
                platform_version,
            )
            .expect("should verify proof");

        assert!(!root_hash.is_empty(), "root hash should not be empty");
        assert_eq!(proved_actions.len(), 2, "should have 2 actions");
        assert_eq!(
            proved_actions.get(&action_id_1),
            Some(&action_1),
            "action 1 should match"
        );
        assert_eq!(
            proved_actions.get(&action_id_2),
            Some(&action_2),
            "action 2 should match"
        );
    }

    #[test]
    fn should_prove_and_verify_no_action_infos_when_empty() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let contract_id = Identifier::random();
        let group_contract_position = 0;

        // Prove for non-existent contract
        let proof = drive
            .prove_action_infos(
                contract_id,
                group_contract_position,
                GroupActionStatus::ActionActive,
                None,
                Some(10),
                None,
                platform_version,
            )
            .expect("should prove empty action infos");

        // Verify
        let (root_hash, proved_actions): (_, BTreeMap<Identifier, GroupAction>) =
            Drive::verify_action_infos_in_contract(
                proof.as_slice(),
                contract_id,
                group_contract_position,
                GroupActionStatus::ActionActive,
                None,
                Some(10),
                false,
                platform_version,
            )
            .expect("should verify proof");

        assert!(!root_hash.is_empty(), "root hash should not be empty");
        assert!(proved_actions.is_empty(), "should have no actions");
    }
}
