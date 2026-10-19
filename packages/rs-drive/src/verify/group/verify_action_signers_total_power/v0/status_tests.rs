use super::*;
use crate::drive::group::paths::{
    group_active_action_root_path_vec, group_path_vec, GROUP_CLOSED_ACTIONS_KEY,
};
use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use dpp::block::block_info::BlockInfo;
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
use dpp::tokens::token_event::TokenEvent;
use grovedb::operations::proof::{GroveDBProof, ProofBytes};
use grovedb::{MerkProofDecoder, MerkProofNode, MerkProofOp, PathQuery};
use grovedb_merk::proofs::encoding::encode_into;
use std::collections::BTreeMap;

const CONTRACT: [u8; 32] = [3; 32];
const SIGNER: [u8; 32] = [1; 32];
const SECOND_SIGNER: [u8; 32] = [2; 32];
const ACTIVE_ACTION: [u8; 32] = [17; 32];
const CLOSED_ACTION: [u8; 32] = [18; 32];

fn fixture(platform_version: &PlatformVersion, active: bool, closed: bool) -> Drive {
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = DataContract::V1(DataContractV1 {
        id: CONTRACT.into(),
        version: 0,
        owner_id: SIGNER.into(),
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
                members: [(SIGNER.into(), 3), (SECOND_SIGNER.into(), 5)].into(),
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
        .expect("insert grouped contract");

    for action_id in [
        active.then_some(ACTIVE_ACTION),
        closed.then_some(CLOSED_ACTION),
    ]
    .into_iter()
    .flatten()
    {
        let action = GroupAction::V0(GroupActionV0 {
            contract_id: CONTRACT.into(),
            proposer_id: SIGNER.into(),
            token_contract_position: 0,
            event: GroupActionEvent::TokenEvent(TokenEvent::Mint(100, SIGNER.into(), None)),
        });
        drive
            .add_group_action(
                CONTRACT.into(),
                0,
                Some(action),
                false,
                action_id.into(),
                SIGNER.into(),
                3,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("record proposing signer");
        if action_id == CLOSED_ACTION {
            drive
                .add_group_action(
                    CONTRACT.into(),
                    0,
                    None,
                    true,
                    action_id.into(),
                    SECOND_SIGNER.into(),
                    5,
                    &BlockInfo::default(),
                    true,
                    None,
                    platform_version,
                )
                .expect("close action after second signature");
        }
    }
    drive
}

fn versions() -> [&'static PlatformVersion; 3] {
    [
        PlatformVersion::get(11).expect("PV11"),
        PlatformVersion::get(13).expect("PV13"),
        PlatformVersion::latest(),
    ]
}

fn verify(
    proof: &[u8],
    action: [u8; 32],
    status: Option<GroupActionStatus>,
    subset: bool,
    platform_version: &PlatformVersion,
) -> Result<(RootHash, GroupActionStatus, GroupSumPower), Error> {
    Drive::verify_action_signer_and_total_power(
        proof,
        CONTRACT.into(),
        0,
        status,
        action.into(),
        SIGNER.into(),
        subset,
        platform_version,
    )
}

#[test]
fn should_discover_actions_when_the_other_group_branch_is_empty() {
    for platform_version in versions() {
        for (active, closed, action, status, power) in [
            (
                true,
                false,
                ACTIVE_ACTION,
                GroupActionStatus::ActionActive,
                3,
            ),
            (
                false,
                true,
                CLOSED_ACTION,
                GroupActionStatus::ActionClosed,
                8,
            ),
        ] {
            let drive = fixture(platform_version, active, closed);
            let proof = action_proof(&drive, action, platform_version);
            let root = drive
                .grove
                .root_hash(None, &platform_version.drive.grove_version)
                .unwrap()
                .expect("stored root");
            for subset in [false, true] {
                assert_eq!(
                    verify(&proof, action, None, subset, platform_version)
                        .expect("infer action status with an empty opposite branch"),
                    (root, status, power)
                );
            }
        }
    }
}

#[test]
fn should_preserve_explicit_action_status_verification() {
    for platform_version in versions() {
        let drive = fixture(platform_version, true, true);
        let root = drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .expect("stored root");
        for (action, status, power) in [
            (ACTIVE_ACTION, GroupActionStatus::ActionActive, 3),
            (CLOSED_ACTION, GroupActionStatus::ActionClosed, 8),
        ] {
            let query = Drive::group_active_or_closed_action_single_signer_query(
                CONTRACT, 0, action, status, SIGNER,
            );
            let proof = drive
                .grove_get_proved_path_query(&query, None, &mut vec![], &platform_version.drive)
                .expect("prove selected action");
            for subset in [false, true] {
                assert_eq!(
                    verify(&proof, action, Some(status), subset, platform_version)
                        .expect("verify explicitly selected action"),
                    (root, status, power)
                );
            }
        }
    }
}

#[test]
fn should_refuse_absent_actions_and_absent_signers() {
    for platform_version in versions() {
        for (active, closed) in [(false, false), (true, false), (false, true), (true, true)] {
            let drive = fixture(platform_version, active, closed);
            let proof = action_proof(&drive, [19; 32], platform_version);
            for subset in [false, true] {
                assert!(verify(&proof, [19; 32], None, subset, platform_version).is_err());
            }
        }
        let drive = fixture(platform_version, true, false);
        let query = Drive::group_active_and_closed_action_single_signer_query(
            CONTRACT,
            0,
            ACTIVE_ACTION,
            [9; 32],
        );
        let proof = drive
            .grove_get_proved_path_query(&query, None, &mut vec![], &platform_version.drive)
            .expect("prove a signer who has not participated");
        for subset in [false, true] {
            assert!(Drive::verify_action_signer_and_total_power(
                &proof,
                CONTRACT.into(),
                0,
                None,
                ACTIVE_ACTION.into(),
                [9; 32].into(),
                subset,
                platform_version,
            )
            .is_err());
        }
    }
}

#[test]
fn should_refuse_proofs_for_another_group_action_or_signer() {
    for platform_version in versions() {
        let drive = fixture(platform_version, true, true);
        let proof = action_proof(&drive, ACTIVE_ACTION, platform_version);
        verify(&proof, ACTIVE_ACTION, None, true, platform_version).expect("valid control");
        for (contract, group, action, signer) in [
            ([4; 32], 0, ACTIVE_ACTION, SIGNER),
            (CONTRACT, 1, ACTIVE_ACTION, SIGNER),
            (CONTRACT, 0, CLOSED_ACTION, SIGNER),
            (CONTRACT, 0, ACTIVE_ACTION, [9; 32]),
        ] {
            for subset in [false, true] {
                assert!(Drive::verify_action_signer_and_total_power(
                    &proof,
                    contract.into(),
                    group,
                    None,
                    action.into(),
                    signer.into(),
                    subset,
                    platform_version,
                )
                .is_err());
            }
        }
        let mut corrupt = proof;
        corrupt.pop();
        for subset in [false, true] {
            assert!(verify(&corrupt, ACTIVE_ACTION, None, subset, platform_version).is_err());
        }
    }
}

#[test]
fn should_refuse_the_same_action_in_both_group_branches() {
    for platform_version in versions() {
        for signer in [SIGNER, SECOND_SIGNER] {
            let drive = fixture(platform_version, false, true);
            let action = GroupAction::V0(GroupActionV0 {
                contract_id: CONTRACT.into(),
                proposer_id: signer.into(),
                token_contract_position: 0,
                event: GroupActionEvent::TokenEvent(TokenEvent::Mint(100, signer.into(), None)),
            });
            drive
                .add_group_action(
                    CONTRACT.into(),
                    0,
                    Some(action),
                    false,
                    CLOSED_ACTION.into(),
                    signer.into(),
                    3,
                    &BlockInfo::default(),
                    true,
                    None,
                    platform_version,
                )
                .expect("create ambiguous stored action");
            let proof = action_proof(&drive, CLOSED_ACTION, platform_version);
            for subset in [false, true] {
                assert!(verify(&proof, CLOSED_ACTION, None, subset, platform_version).is_err());
            }
        }
    }
}

#[test]
fn should_refuse_a_malformed_stored_action() {
    for platform_version in versions() {
        for element in [Element::empty_tree(), Element::new_item(vec![1])] {
            let drive = fixture(platform_version, false, false);
            let parent = group_active_action_root_path_vec(&CONTRACT, 0);
            drive
                .grove
                .insert(
                    parent.as_slice(),
                    &ACTIVE_ACTION,
                    element,
                    None,
                    None,
                    &platform_version.drive.grove_version,
                )
                .unwrap()
                .expect("insert malformed action in isolated fixture");
            let proof = action_proof(&drive, ACTIVE_ACTION, platform_version);
            for subset in [false, true] {
                assert!(verify(&proof, ACTIVE_ACTION, None, subset, platform_version).is_err());
            }
        }
    }
}

#[test]
fn should_allow_extra_action_proof_layers_only_in_subset_mode() {
    for platform_version in versions() {
        let drive = fixture(platform_version, true, true);
        let active = Drive::group_active_and_closed_action_single_signer_query(
            CONTRACT,
            0,
            ACTIVE_ACTION,
            SIGNER,
        );
        let closed = Drive::group_active_and_closed_action_single_signer_query(
            CONTRACT,
            0,
            CLOSED_ACTION,
            SIGNER,
        );
        let query = PathQuery::merge(
            vec![&active, &closed],
            &platform_version.drive.grove_version,
        )
        .expect("merge both action queries");
        let proof = drive
            .grove_get_proved_path_query(&query, None, &mut vec![], &platform_version.drive)
            .expect("prove both distinct actions");
        assert!(verify(&proof, ACTIVE_ACTION, None, false, platform_version).is_err());
        let root = drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .expect("stored root");
        assert_eq!(
            verify(&proof, ACTIVE_ACTION, None, true, platform_version)
                .expect("a subset caller may ignore the other action's layers"),
            (root, GroupActionStatus::ActionActive, 3)
        );
    }
}

fn forge_empty_element(
    proof: &[u8],
    parent: &[Vec<u8>],
    key: &[u8],
    platform_version: &PlatformVersion,
) -> Vec<u8> {
    fn rewrite(bytes: &mut Vec<u8>, key: &[u8], fake: &[u8]) {
        let mut ops: Vec<_> = MerkProofDecoder::new(bytes)
            .map(|op| op.expect("decode genuine merk proof"))
            .collect();
        let mut changed = false;
        for op in &mut ops {
            let node = match op {
                MerkProofOp::Push(node) | MerkProofOp::PushInverted(node) => node,
                _ => continue,
            };
            match node {
                MerkProofNode::KVValueHash(node_key, value, _) if node_key == key => {
                    *value = fake.to_vec();
                    changed = true;
                }
                MerkProofNode::KVValueHashFeatureType(node_key, value, _, _) if node_key == key => {
                    *value = fake.to_vec();
                    changed = true;
                }
                MerkProofNode::KVValueHashFeatureTypeWithChildHash(
                    node_key,
                    _,
                    hash,
                    feature,
                    _,
                ) if node_key == key => {
                    *node = MerkProofNode::KVValueHashFeatureType(
                        node_key.clone(),
                        fake.to_vec(),
                        *hash,
                        *feature,
                    );
                    changed = true;
                }
                _ => {}
            }
        }
        assert!(
            changed,
            "forgery must rewrite the target's unhashed element bytes"
        );
        bytes.clear();
        encode_into(ops.iter(), bytes);
    }
    let config = bincode::config::standard().with_big_endian();
    let (mut decoded, _): (GroveDBProof, _) =
        bincode::decode_from_slice(proof, config).expect("decode genuine proof envelope");
    let fake = Element::empty_tree()
        .serialize(&platform_version.drive.grove_version)
        .expect("serialize claimed empty tree");
    match &mut decoded {
        GroveDBProof::V0(proof) => {
            let mut layer = &mut proof.root_layer;
            for part in parent {
                layer = layer.lower_layers.get_mut(part).expect("parent layer");
            }
            assert!(layer.lower_layers.remove(key).is_some());
            rewrite(&mut layer.merk_proof, key, &fake);
        }
        GroveDBProof::V1(proof) => {
            let mut layer = &mut proof.root_layer;
            for part in parent {
                layer = layer.lower_layers.get_mut(part).expect("parent layer");
            }
            assert!(layer.lower_layers.remove(key).is_some());
            let ProofBytes::Merk(bytes) = &mut layer.merk_proof else {
                panic!("merk parent");
            };
            rewrite(bytes, key, &fake);
        }
    }
    bincode::encode_to_vec(decoded, config).expect("encode tampered envelope")
}

#[test]
fn should_refuse_forged_empty_group_and_action_branches() {
    for platform_version in versions() {
        let drive = fixture(platform_version, true, true);
        let proof = action_proof(&drive, ACTIVE_ACTION, platform_version);
        verify(&proof, ACTIVE_ACTION, None, true, platform_version).expect("valid control");
        let parent = group_path_vec(&CONTRACT, 0);
        let forged =
            forge_empty_element(&proof, &parent, GROUP_CLOSED_ACTIONS_KEY, platform_version);
        for subset in [false, true] {
            let error = verify(&forged, ACTIVE_ACTION, None, subset, platform_version)
                .expect_err("the omitted group branch must be authenticated");
            assert!(
                error.to_string().contains("empty tree value hash mismatch"),
                "{error}"
            );
        }
        let parent = group_active_action_root_path_vec(&CONTRACT, 0);
        let forged = forge_empty_element(&proof, &parent, &ACTIVE_ACTION, platform_version);
        for subset in [false, true] {
            assert!(verify(&forged, ACTIVE_ACTION, None, subset, platform_version).is_err());
        }

        // Hiding the action in the branch without this signer must not make an ambiguous
        // stored action look unique under the same authenticated root.
        let drive = fixture(platform_version, false, true);
        let action = GroupAction::V0(GroupActionV0 {
            contract_id: CONTRACT.into(),
            proposer_id: SECOND_SIGNER.into(),
            token_contract_position: 0,
            event: GroupActionEvent::TokenEvent(TokenEvent::Mint(100, SECOND_SIGNER.into(), None)),
        });
        drive
            .add_group_action(
                CONTRACT.into(),
                0,
                Some(action),
                false,
                CLOSED_ACTION.into(),
                SECOND_SIGNER.into(),
                5,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("record another branch without the requested signer");
        let proof = action_proof(&drive, CLOSED_ACTION, platform_version);
        let forged = forge_empty_element(&proof, &parent, &CLOSED_ACTION, platform_version);
        for subset in [false, true] {
            let error = verify(&forged, CLOSED_ACTION, None, subset, platform_version)
                .expect_err("cannot hide the duplicate action behind a claimed empty tree");
            assert!(
                error.to_string().contains("empty tree value hash mismatch"),
                "{error}"
            );
        }
    }
}

fn action_proof(drive: &Drive, action: [u8; 32], platform_version: &PlatformVersion) -> Vec<u8> {
    let query =
        Drive::group_active_and_closed_action_single_signer_query(CONTRACT, 0, action, SIGNER);
    drive
        .grove_get_proved_path_query(&query, None, &mut vec![], &platform_version.drive)
        .expect("prove the requested action in both branches")
}

#[test]
fn should_verify_each_action_in_a_group_with_active_and_closed_actions() {
    for platform_version in versions() {
        let drive = fixture(platform_version, true, true);
        let root = drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .expect("stored root");
        for (action, status, power) in [
            (ACTIVE_ACTION, GroupActionStatus::ActionActive, 3),
            (CLOSED_ACTION, GroupActionStatus::ActionClosed, 8),
        ] {
            let proof = action_proof(&drive, action, platform_version);
            for subset in [false, true] {
                let verified = verify(&proof, action, None, subset, platform_version)
                    .unwrap_or_else(|error| {
                        panic!(
                            "PV{} mixed group {status:?}, subset={subset}: {error}",
                            platform_version.protocol_version,
                        )
                    });
                assert_eq!(verified, (root, status, power));
            }
        }
    }
}
