use super::*;
use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
use dpp::data_contract::associated_token::token_keeps_history_rules::v0::TokenKeepsHistoryRulesV0;
use dpp::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
use dpp::data_contract::config::v0::DataContractConfigV0;
use dpp::data_contract::group::v0::GroupV0;
use dpp::data_contract::group::Group;
use dpp::data_contract::v1::DataContractV1;
use dpp::data_contract::DataContract;
use dpp::group::action_event::GroupActionEvent;
use dpp::group::group_action::v0::GroupActionV0;
use dpp::group::group_action::GroupAction;
use dpp::group::GroupStateTransitionInfo;
use dpp::identity::accessors::IdentitySettersV0;
use dpp::identity::Identity;
use dpp::state_transition::batch_transition::batched_transition::BatchedTransition;
use dpp::state_transition::batch_transition::token_base_transition::v0::TokenBaseTransitionV0;
use dpp::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use dpp::state_transition::batch_transition::token_burn_transition::v0::TokenBurnTransitionV0;
use dpp::state_transition::batch_transition::token_burn_transition::TokenBurnTransition;
use dpp::state_transition::batch_transition::{BatchTransition, BatchTransitionV1};
use dpp::tokens::token_event::TokenEvent;
use std::collections::BTreeMap;
use std::sync::Arc;

const CONTRACT: [u8; 32] = [3; 32];
const PROPOSER: [u8; 32] = [1; 32];
const COSIGNER: [u8; 32] = [2; 32];
const CONTRACT_OWNER: [u8; 32] = [4; 32];
const CREDITS: u64 = 1_000_000;

struct Fixture {
    drive: Drive,
    contract: DataContract,
    transition: StateTransition,
    token_id: Identifier,
    action_id: Identifier,
    signer: Identifier,
}

impl Fixture {
    fn new(
        signer_is_proposer: bool,
        closed: bool,
        cosigner_balance: Option<u64>,
        burn_amount: u64,
        keeps_history: bool,
        platform_version: &PlatformVersion,
    ) -> Self {
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let mut token = TokenConfigurationV0::default_most_restrictive().with_base_supply(0);
        token.keeps_history =
            TokenKeepsHistoryRulesV0::default_for_keeping_all_history(keeps_history).into();
        token
            .manual_burning_rules
            .set_authorized_to_make_change_action_takers(AuthorizedActionTakers::Group(0));
        let contract = DataContract::V1(DataContractV1 {
            id: CONTRACT.into(),
            version: 0,
            owner_id: CONTRACT_OWNER.into(),
            document_types: Default::default(),
            config: DataContractConfigV0::default().into(),
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
                    members: [(PROPOSER.into(), 3), (COSIGNER.into(), 5)].into(),
                    required_power: if signer_is_proposer && closed {
                        3
                    } else if closed {
                        8
                    } else {
                        9
                    },
                }),
            )]),
            tokens: BTreeMap::from([(0, TokenConfiguration::V0(token))]),
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
            .expect("insert grouped token contract");
        let token_id = contract.token_id(0).expect("token at position zero");
        if keeps_history {
            let history_contract =
                load_system_data_contract(SystemDataContract::TokenHistory, platform_version)
                    .expect("load token history contract");
            drive
                .insert_contract(
                    &history_contract,
                    BlockInfo::default(),
                    true,
                    None,
                    platform_version,
                )
                .expect("insert token history contract");
        }
        for id in [PROPOSER, COSIGNER] {
            let mut identity =
                Identity::random_identity(1, Some(14), platform_version).expect("create identity");
            identity.set_id(id.into());
            identity.set_balance(CREDITS);
            drive
                .add_new_identity(
                    identity,
                    false,
                    &BlockInfo::default(),
                    true,
                    None,
                    platform_version,
                )
                .expect("insert signer credit balance");
        }
        for (id, amount) in [(PROPOSER, Some(10)), (COSIGNER, cosigner_balance)] {
            if let Some(amount) = amount {
                drive
                    .token_mint(
                        token_id.to_buffer(),
                        id,
                        amount,
                        true,
                        false,
                        &BlockInfo::default(),
                        true,
                        None,
                        platform_version,
                    )
                    .expect("fund token balance");
            }
        }
        let action_id = TokenBurnTransition::calculate_action_id_with_fields(
            &token_id.to_buffer(),
            &PROPOSER,
            1,
            burn_amount,
        );
        let action = GroupAction::V0(GroupActionV0 {
            contract_id: CONTRACT.into(),
            proposer_id: PROPOSER.into(),
            token_contract_position: 0,
            event: GroupActionEvent::TokenEvent(TokenEvent::Burn(
                burn_amount,
                PROPOSER.into(),
                None,
            )),
        });
        drive
            .add_group_action(
                CONTRACT.into(),
                0,
                Some(action),
                signer_is_proposer && closed,
                action_id,
                PROPOSER.into(),
                3,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("record proposal");
        if !signer_is_proposer {
            drive
                .add_group_action(
                    CONTRACT.into(),
                    0,
                    None,
                    closed,
                    action_id,
                    COSIGNER.into(),
                    5,
                    &BlockInfo::default(),
                    true,
                    None,
                    platform_version,
                )
                .expect("record co-signature");
        }
        if closed {
            drive
                .token_burn(
                    token_id.to_buffer(),
                    PROPOSER,
                    burn_amount,
                    &BlockInfo::default(),
                    true,
                    None,
                    platform_version,
                )
                .expect("burn proposer tokens");
        }
        let signer = if signer_is_proposer {
            PROPOSER
        } else {
            COSIGNER
        }
        .into();
        if closed && keeps_history {
            let operations = drive
                .add_token_transaction_history_operations(
                    token_id,
                    signer,
                    1,
                    TokenEvent::Burn(burn_amount, PROPOSER.into(), None),
                    &BlockInfo::default(),
                    &mut None,
                    None,
                    platform_version,
                )
                .expect("build burn history operations");
            drive
                .apply_batch_low_level_drive_operations(
                    None,
                    None,
                    operations,
                    &mut vec![],
                    &platform_version.drive,
                )
                .expect("record burn history");
        }
        let transition = StateTransition::Batch(BatchTransition::V1(BatchTransitionV1 {
            owner_id: signer,
            transitions: vec![BatchedTransition::Token(TokenTransition::Burn(
                TokenBurnTransition::V0(TokenBurnTransitionV0 {
                    base: TokenBaseTransition::V0(TokenBaseTransitionV0 {
                        identity_contract_nonce: 1,
                        token_contract_position: 0,
                        data_contract_id: CONTRACT.into(),
                        token_id,
                        using_group_info: Some(GroupStateTransitionInfo {
                            group_contract_position: 0,
                            action_id,
                            action_is_proposer: signer_is_proposer,
                        }),
                    }),
                    burn_amount,
                    public_note: None,
                }),
            ))],
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: Default::default(),
        }));
        Self {
            drive,
            contract,
            transition,
            token_id,
            action_id,
            signer,
        }
    }

    fn proof(&self, platform_version: &PlatformVersion) -> Vec<u8> {
        self.drive
            .prove_state_transition(&self.transition, None, platform_version)
            .expect("prove stored burn result")
            .into_data()
            .expect("proof bytes")
    }

    fn verify(
        &self,
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, StateTransitionProofOutcome), Error> {
        Drive::verify_state_transition_was_executed_with_proof(
            &self.transition,
            &BlockInfo::default(),
            proof,
            &|_| Ok(Some(Arc::new(self.contract.clone()))),
            platform_version,
        )
    }
}

fn assert_group_burn(
    signer_is_proposer: bool,
    closed: bool,
    cosigner_balance: Option<u64>,
    burn_amount: u64,
    platform_version: &PlatformVersion,
) {
    let fixture = Fixture::new(
        signer_is_proposer,
        closed,
        cosigner_balance,
        burn_amount,
        false,
        platform_version,
    );
    let proof = fixture.proof(platform_version);
    let (signer_root, status, power) = Drive::verify_action_signer_and_total_power(
        &proof,
        CONTRACT.into(),
        0,
        None,
        fixture.action_id,
        fixture.signer,
        true,
        platform_version,
    )
    .expect("verify exact group signer and status");
    let (root, outcome) = fixture
        .verify(&proof, platform_version)
        .expect("verify successful group burn");
    assert_eq!(
        root, signer_root,
        "return the authenticated signer/status root"
    );
    assert_eq!(
        root,
        fixture
            .drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .expect("stored root")
    );
    assert!(outcome.is_execution_proved());
    assert_eq!(
        outcome.owner_balance(),
        if platform_version.protocol_version >= 14 {
            Some(CREDITS)
        } else {
            None
        }
    );
    assert_eq!(
        status,
        if closed {
            GroupActionStatus::ActionClosed
        } else {
            GroupActionStatus::ActionActive
        }
    );
    assert_eq!(power, if signer_is_proposer { 3 } else { 8 });
    match outcome.into_result() {
        VerifiedTokenGroupActionWithTokenBalance(actual_power, actual_status, balance) => {
            assert_eq!((actual_power, actual_status), (power, status));
            assert_eq!(
                balance,
                signer_is_proposer.then_some(if closed { 10 - burn_amount } else { 10 })
            );
        }
        other => panic!("expected group balance result, got {other:?}"),
    }
    let supply_proof = fixture
        .drive
        .prove_token_total_supply_and_aggregated_identity_balances(
            fixture.token_id.to_buffer(),
            None,
            platform_version,
        )
        .expect("prove token supply");
    let (supply_root, supply) = Drive::verify_token_total_supply_and_aggregated_identity_balance(
        &supply_proof,
        fixture.token_id.to_buffer(),
        false,
        platform_version,
    )
    .expect("verify token supply");
    assert_eq!(supply_root, root);
    assert_eq!(
        supply.token_supply,
        (10 + cosigner_balance.unwrap_or_default() - if closed { burn_amount } else { 0 }) as i64
    );
    // Independent snapshots confirm the burn debited the proposer, leaving the co-signer alone.
    for (id, expected) in [
        (PROPOSER, Some(if closed { 10 - burn_amount } else { 10 })),
        (COSIGNER, cosigner_balance),
    ] {
        let query = Drive::token_balance_for_identity_id_query(fixture.token_id.to_buffer(), id);
        let proof = fixture
            .drive
            .grove_get_proved_path_query(&query, None, &mut vec![], &platform_version.drive)
            .expect("prove ledger balance");
        assert_eq!(
            Drive::verify_token_balance_for_identity_id(
                &proof,
                fixture.token_id.to_buffer(),
                id,
                false,
                platform_version
            )
            .expect("verify ledger balance"),
            (root, expected)
        );
    }
}

#[test]
fn should_omit_unrelated_balance_when_cosigner_closes_group_burn_v0() {
    assert_group_burn(
        false,
        true,
        Some(50),
        3,
        PlatformVersion::get(13).expect("PV13"),
    );
}

#[test]
fn should_omit_unrelated_balance_when_cosigner_closes_group_burn_v1() {
    assert_group_burn(false, true, Some(50), 3, PlatformVersion::latest());
}

#[test]
fn should_verify_group_burn_when_closing_signer_has_no_token_balance_v0() {
    assert_group_burn(
        false,
        true,
        None,
        3,
        PlatformVersion::get(13).expect("PV13"),
    );
}

#[test]
fn should_verify_group_burn_when_closing_signer_has_no_token_balance_v1() {
    assert_group_burn(false, true, None, 3, PlatformVersion::latest());
}

#[test]
fn should_preserve_proposer_group_burn_balance_including_zero() {
    for version in [
        PlatformVersion::get(13).expect("PV13"),
        PlatformVersion::latest(),
    ] {
        for (closed, amount) in [(false, 3), (true, 3), (true, 10)] {
            assert_group_burn(true, closed, Some(50), amount, version);
        }
    }
}

#[test]
fn should_omit_cosigner_balance_while_group_burn_is_active() {
    for version in [
        PlatformVersion::get(13).expect("PV13"),
        PlatformVersion::latest(),
    ] {
        assert_group_burn(false, false, Some(50), 3, version);
    }
}

#[test]
fn should_reject_group_burn_owner_credit_balance_from_another_root() {
    let version = PlatformVersion::latest();
    let fixture = Fixture::new(false, true, Some(50), 3, false, version);
    let proof = fixture.proof(version);
    let (mut root, _, _) = Drive::verify_action_signer_and_total_power(
        &proof,
        CONTRACT.into(),
        0,
        None,
        fixture.action_id,
        fixture.signer,
        true,
        version,
    )
    .expect("verify signer root");
    Drive::owner_balance_of_verified_transition(&fixture.transition, None, &proof, root, version)
        .expect("matching root control");
    root[0] ^= 1;
    let error = Drive::owner_balance_of_verified_transition(
        &fixture.transition,
        None,
        &proof,
        root,
        version,
    )
    .expect_err("owner credit balance must share the signer/status root");
    assert!(
        matches!(error, Error::Proof(ProofError::CorruptedProof(message))
        if message.contains("different states"))
    );
}

#[test]
fn should_preserve_group_burn_history_for_proposers_and_cosigners() {
    for version in [
        PlatformVersion::get(13).expect("PV13"),
        PlatformVersion::latest(),
    ] {
        for proposer in [true, false] {
            for closed in [true, false] {
                let fixture = Fixture::new(proposer, closed, None, 3, true, version);
                let proof = fixture.proof(version);
                let (root, outcome) = fixture
                    .verify(&proof, version)
                    .expect("verify history-enabled group burn");
                assert_eq!(
                    root,
                    fixture
                        .drive
                        .grove
                        .root_hash(None, &version.drive.grove_version)
                        .unwrap()
                        .expect("stored root")
                );
                match outcome.into_result() {
                    VerifiedTokenGroupActionWithDocument(power, document) => {
                        assert_eq!(power, if proposer { 3 } else { 8 });
                        assert_eq!(document.is_some(), closed);
                    }
                    other => panic!("expected history result, got {other:?}"),
                }
            }
        }
    }
}

#[test]
fn should_verify_group_burn_with_other_active_actions_in_the_group() {
    for version in [
        PlatformVersion::get(13).expect("PV13"),
        PlatformVersion::latest(),
    ] {
        let fixture = Fixture::new(false, true, Some(50), 3, false, version);
        let action = GroupAction::V0(GroupActionV0 {
            contract_id: CONTRACT.into(),
            proposer_id: PROPOSER.into(),
            token_contract_position: 0,
            event: GroupActionEvent::TokenEvent(TokenEvent::Burn(1, PROPOSER.into(), None)),
        });
        fixture
            .drive
            .add_group_action(
                CONTRACT.into(),
                0,
                Some(action),
                false,
                [18; 32].into(),
                PROPOSER.into(),
                3,
                &BlockInfo::default(),
                true,
                None,
                version,
            )
            .expect("record unrelated active action");
        let proof = fixture.proof(version);
        let (_, outcome) = fixture
            .verify(&proof, version)
            .expect("verify exact closed action in mixed group");
        assert!(matches!(
            outcome.into_result(),
            VerifiedTokenGroupActionWithTokenBalance(8, GroupActionStatus::ActionClosed, None)
        ));
    }
}

#[test]
fn should_preserve_standalone_burn_balances_and_history() {
    for version in [
        PlatformVersion::get(13).expect("PV13"),
        PlatformVersion::latest(),
    ] {
        for history in [false, true] {
            let mut fixture = Fixture::new(true, true, None, 3, history, version);
            let StateTransition::Batch(BatchTransition::V1(batch)) = &mut fixture.transition else {
                panic!("token batch");
            };
            let BatchedTransition::Token(TokenTransition::Burn(TokenBurnTransition::V0(burn))) =
                &mut batch.transitions[0]
            else {
                panic!("burn");
            };
            let TokenBaseTransition::V0(base) = &mut burn.base;
            base.using_group_info = None;
            let proof = fixture.proof(version);
            let (_, outcome) = fixture
                .verify(&proof, version)
                .expect("verify standalone burn");
            match outcome.into_result() {
                VerifiedTokenBalance(owner, 7) if !history => {
                    assert_eq!(owner, Identifier::from(PROPOSER));
                }
                VerifiedTokenActionWithDocument(_) if history => (),
                other => panic!("unexpected standalone burn result {other:?}"),
            }
        }
    }
}

#[test]
fn should_reject_group_burn_proofs_for_another_action_group_or_signer() {
    for version in [
        PlatformVersion::get(13).expect("PV13"),
        PlatformVersion::latest(),
    ] {
        let mut fixture = Fixture::new(false, true, Some(50), 3, false, version);
        let proof = fixture.proof(version);
        fixture
            .verify(&proof, version)
            .expect("exact action control");
        let original = fixture.transition.clone();
        for selector in 0..3 {
            fixture.transition = original.clone();
            let StateTransition::Batch(BatchTransition::V1(batch)) = &mut fixture.transition else {
                panic!("token batch");
            };
            let BatchedTransition::Token(TokenTransition::Burn(TokenBurnTransition::V0(burn))) =
                &mut batch.transitions[0]
            else {
                panic!("burn");
            };
            let TokenBaseTransition::V0(base) = &mut burn.base;
            let info = base.using_group_info.as_mut().expect("group info");
            match selector {
                0 => info.action_id = [19; 32].into(),
                1 => info.group_contract_position = 1,
                _ => batch.owner_id = [9; 32].into(),
            }
            assert!(
                fixture.verify(&proof, version).is_err(),
                "reject mismatched action scope"
            );
        }
    }
}
