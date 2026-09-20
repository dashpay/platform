use super::*;
use crate::execution::check_tx::CheckTxLevel;
use crate::execution::validation::state_transition::tests::setup_identity_without_adding_it;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::MockCoreRPCLike;
use crate::test::helpers::contract_groups::{join_group, register_group, single_owner_info};
use crate::test::helpers::setup::TempPlatform;
use dpp::consensus::codes::ErrorWithCode;
use dpp::contract_group::{
    ContractGroupInfo, ContractGroupMember, ContractGroupMembership, ContractGroupRegistration,
};
use dpp::identifier::Identifier;
use dpp::identity::contract_bounds::ContractBounds;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{accessors::IdentitySettersV0, IdentityPublicKey};
use dpp::state_transition::batch_transition::methods::v1::DocumentsBatchTransitionMethodsV1;
use dpp::version::PLATFORM_VERSIONS;
use std::collections::BTreeSet;

const KEY_GROUP: [u8; 32] = [0x61; 32];
const OTHER_GROUP: [u8; 32] = [0x62; 32];

fn register_contract_group(
    platform: &TempPlatform<MockCoreRPCLike>,
    contract_group_id: [u8; 32],
    version: &PlatformVersion,
) {
    register_group(
        platform,
        Identifier::from(contract_group_id),
        &single_owner_info(Identifier::from([0x60; 32]), None, None),
        version,
    );
}

fn join_contract_group(
    platform: &TempPlatform<MockCoreRPCLike>,
    contract_id: Identifier,
    contract_group_id: [u8; 32],
    member: ContractGroupMember,
    version: &PlatformVersion,
) {
    join_group(
        platform,
        contract_id,
        &[(Identifier::from(contract_group_id), member)],
        version,
    );
}

/// Sign with the original unbounded key metadata to bypass the SDK preflight; validators must
/// enforce the bounds of the key stored in Drive, reading the group memberships from state.
#[tokio::test]
async fn should_authorize_documents_by_the_contract_group_memberships_of_their_contract() {
    for case in [
        "whole_contract",
        "document_type",
        "joined_after_registration",
        "other_document_type",
        "other_group",
        "no_memberships",
    ] {
        let version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();
        let (mut identity, signer, signing_key) =
            setup_identity_without_adding_it(958, dash_to_credits!(0.1));
        let dashpay = platform
            .drive
            .cache
            .system_data_contracts
            .load_dashpay(version)
            .unwrap();
        register_contract_group(&platform, KEY_GROUP, version);
        register_contract_group(&platform, OTHER_GROUP, version);
        let membership = match case {
            "whole_contract" | "joined_after_registration" => {
                Some((KEY_GROUP, ContractGroupMember::Contract))
            }
            "document_type" => Some((
                KEY_GROUP,
                ContractGroupMember::DocumentType("profile".into()),
            )),
            "other_document_type" => Some((
                KEY_GROUP,
                ContractGroupMember::DocumentType("contactRequest".into()),
            )),
            "other_group" => Some((OTHER_GROUP, ContractGroupMember::Contract)),
            _ => None,
        };
        if case != "joined_after_registration" {
            if let Some((group, member)) = membership.clone() {
                join_contract_group(&platform, dashpay.id(), group, member, version);
            }
        }
        let mut stored_key = signing_key.clone();
        let IdentityPublicKey::V0(ref mut key) = stored_key else {
            panic!("expected a version 0 key")
        };
        key.contract_bounds = Some(ContractBounds::ContractGroup {
            id: Identifier::from(KEY_GROUP),
        });
        identity.add_public_key(stored_key);
        platform
            .drive
            .add_new_identity(
                identity.clone(),
                false,
                &BlockInfo::default(),
                true,
                None,
                version,
            )
            .unwrap();
        if case == "joined_after_registration" {
            // Memberships are append-only: the key reaches a contract that joins later.
            if let Some((group, member)) = membership {
                join_contract_group(&platform, dashpay.id(), group, member, version);
            }
        }
        let state = platform.state.load();
        let profile = dashpay.document_type_for_name("profile").unwrap();
        let mut rng = StdRng::seed_from_u64(433);
        let entropy = Bytes32::random_with_rng(&mut rng);
        let mut document = profile
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity.id(),
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                version,
            )
            .unwrap();
        document
            .set_id_for_creation(profile, &entropy.0, 2, version)
            .expect("expected to set the document id");
        set_valid_profile_payment_addresses(&mut document, profile);
        document.set("avatarUrl", "http://test.com/bob.jpg".into());
        let batch = BatchTransition::new_document_creation_transition_from_document(
            document,
            profile,
            entropy.0,
            &signing_key,
            2,
            0,
            None,
            &signer,
            version,
            None,
        )
        .await
        .unwrap();
        let bytes = batch.serialize_to_bytes().unwrap();

        // CheckTx resolves the memberships through the same transformer and reaches the same
        // verdict, so the mempool neither admits a batch outside the group nor drops one
        // inside it.
        let platform_ref = PlatformRef {
            drive: &platform.drive,
            state: &state,
            config: &platform.config,
            core_rpc: &platform.core_rpc,
        };
        let check_tx_result = platform
            .check_tx(&bytes, CheckTxLevel::FirstTimeCheck, &platform_ref, version)
            .expect("expected to check tx");
        let check_tx_codes: Vec<u32> = check_tx_result
            .errors
            .iter()
            .map(ErrorWithCode::code)
            .collect();
        match case {
            "whole_contract" | "document_type" | "joined_after_registration" => {
                assert!(check_tx_codes.is_empty(), "{case}: {check_tx_codes:?}")
            }
            _ => assert_eq!(check_tx_codes, vec![20014], "{case}"),
        }

        let tx = platform.drive.grove.start_transaction();
        let block_info = BlockInfo {
            time_ms: 100,
            ..Default::default()
        };
        let result = platform
            .platform
            .process_raw_state_transitions(
                &vec![bytes.clone()],
                &state,
                &block_info,
                &tx,
                version,
                false,
                None,
            )
            .unwrap();
        let execution = &result.execution_results()[0];
        match case {
            "whole_contract" | "document_type" | "joined_after_registration" => assert!(
                matches!(
                    execution,
                    StateTransitionExecutionResult::SuccessfulExecution { .. }
                ),
                "{case}: {execution:?}"
            ),
            _ => {
                assert!(
                    matches!(execution, StateTransitionExecutionResult::PaidConsensusError { error, .. } if error.code() == 20014),
                    "{case}: {execution:?}"
                );
                assert_eq!(
                    platform
                        .drive
                        .fetch_identity_contract_nonce(
                            identity.id().to_buffer(),
                            dashpay.id().to_buffer(),
                            true,
                            Some(&tx),
                            version
                        )
                        .unwrap(),
                    Some((1u64 << 40) | 2),
                    "{case}"
                );
                let balance = platform
                    .drive
                    .fetch_identity_balance(identity.id().to_buffer(), Some(&tx), version)
                    .unwrap()
                    .unwrap();
                assert!(
                    balance < identity.balance(),
                    "{case}: a batch outside the group must pay validation fees"
                );
                let replay = platform
                    .platform
                    .process_raw_state_transitions(
                        &vec![bytes],
                        &state,
                        &block_info,
                        &tx,
                        version,
                        false,
                        None,
                    )
                    .unwrap();
                assert!(
                    matches!(
                        &replay.execution_results()[0],
                        StateTransitionExecutionResult::UnpaidConsensusError(_)
                    ),
                    "{case}: replay must not charge twice"
                );
            }
        }
    }
}

#[tokio::test]
async fn should_authorize_token_operations_by_the_contract_group_memberships_of_their_contract() {
    use crate::execution::validation::state_transition::tests::create_token_contract_with_owner_identity;
    use dpp::data_contract::TokenConfiguration;
    for (case, member, allowed) in [
        ("token", Some(ContractGroupMember::Token(0)), true),
        ("whole_contract", Some(ContractGroupMember::Contract), true),
        ("another_token", Some(ContractGroupMember::Token(1)), false),
        (
            "document_type_only",
            Some(ContractGroupMember::DocumentType("note".into())),
            false,
        ),
        ("no_memberships", None, false),
    ] {
        let version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();
        let (owner, _, _) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));
        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            owner.id(),
            None::<fn(&mut TokenConfiguration)>,
            None,
            None,
            None,
            version,
        );
        register_contract_group(&platform, KEY_GROUP, version);
        if let Some(member) = member {
            join_contract_group(&platform, contract.id(), KEY_GROUP, member, version);
        }
        let (mut identity, signer, signing_key) =
            setup_identity_without_adding_it(234, dash_to_credits!(0.1));
        let mut stored_key = signing_key.clone();
        let IdentityPublicKey::V0(ref mut key) = stored_key else {
            panic!("expected a version 0 key")
        };
        key.contract_bounds = Some(ContractBounds::ContractGroup {
            id: Identifier::from(KEY_GROUP),
        });
        identity.add_public_key(stored_key);
        platform
            .drive
            .add_new_identity(
                identity.clone(),
                false,
                &BlockInfo::default(),
                true,
                None,
                version,
            )
            .unwrap();
        add_tokens_to_identity(&platform, token_id.into(), identity.id(), 15);
        let batch = BatchTransition::new_token_transfer_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            5,
            owner.id(),
            None,
            None,
            None,
            &signing_key,
            2,
            0,
            &signer,
            version,
            None,
        )
        .await
        .unwrap();
        let tx = platform.drive.grove.start_transaction();
        let state = platform.state.load();
        let result = platform
            .platform
            .process_raw_state_transitions(
                &vec![batch.serialize_to_bytes().unwrap()],
                &state,
                &BlockInfo::default(),
                &tx,
                version,
                false,
                None,
            )
            .unwrap();
        let execution = &result.execution_results()[0];
        if allowed {
            assert!(
                matches!(
                    execution,
                    StateTransitionExecutionResult::SuccessfulExecution { .. }
                ),
                "{case}: {execution:?}"
            );
        } else {
            assert!(
                matches!(execution, StateTransitionExecutionResult::PaidConsensusError { error, .. } if error.code() == 20014),
                "{case}: {execution:?}"
            );
        }
    }
}

#[tokio::test]
async fn should_reject_non_batch_use_of_a_key_bound_to_a_contract_group() {
    use dpp::data_contract::accessors::v0::DataContractV0Setters;
    use dpp::state_transition::data_contract_create_transition::{
        methods::DataContractCreateTransitionMethodsV0, DataContractCreateTransition,
    };
    let version = PlatformVersion::latest();
    let platform = TestPlatformBuilder::new()
        .with_latest_protocol_version()
        .build_with_mock_rpc()
        .set_genesis_state();
    let (mut identity, signer, signing_key) =
        setup_identity_without_adding_it(958, dash_to_credits!(0.1));
    let dashpay = platform
        .drive
        .cache
        .system_data_contracts
        .load_dashpay(version)
        .unwrap();
    register_contract_group(&platform, KEY_GROUP, version);
    let mut stored_key = signing_key.clone();
    let IdentityPublicKey::V0(ref mut key) = stored_key else {
        panic!("expected a version 0 key")
    };
    key.contract_bounds = Some(ContractBounds::ContractGroup {
        id: Identifier::from(KEY_GROUP),
    });
    identity.add_public_key(stored_key);
    platform
        .drive
        .add_new_identity(
            identity.clone(),
            false,
            &BlockInfo::default(),
            true,
            None,
            version,
        )
        .unwrap();
    let mut contract = dashpay.as_ref().clone();
    contract.set_owner_id(identity.id());
    let mut signer_identity = identity.clone();
    signer_identity.add_public_key(signing_key.clone());
    let transition = DataContractCreateTransition::new_from_data_contract(
        contract,
        1,
        &signer_identity.into_partial_identity_info(),
        signing_key.id(),
        &signer,
        version,
        None,
    )
    .await
    .unwrap();
    let tx = platform.drive.grove.start_transaction();
    let state = platform.state.load();
    let result = platform
        .platform
        .process_raw_state_transitions(
            &vec![transition.serialize_to_bytes().unwrap()],
            &state,
            &BlockInfo::default(),
            &tx,
            version,
            false,
            None,
        )
        .unwrap();
    assert!(
        matches!(&result.execution_results()[0], StateTransitionExecutionResult::UnpaidConsensusError(error) if error.code() == 20013),
        "{:?}",
        result.execution_results()
    );
    assert_eq!(
        platform
            .drive
            .fetch_identity_balance(identity.id().to_buffer(), Some(&tx), version)
            .unwrap(),
        Some(identity.balance())
    );
}

/// The transformer reads a contract's group memberships only when the batch is signed by a key
/// bound to a contract group. Any other batch is transformed exactly as before, with no extra
/// read, and a transformer that is not told who signed resolves nothing. Nor does it resolve
/// anything for a batch that already failed: such a result never reaches the bounds check.
#[tokio::test]
async fn should_resolve_contract_group_memberships_only_for_a_group_bound_signing_key() {
    use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
    use crate::execution::validation::state_transition::transformer::{
        StateTransitionActionTransformer, StateTransitionSignerAwareActionTransformer,
    };
    use crate::execution::validation::state_transition::ValidationMode;
    use crate::platform_types::platform::PlatformRef;
    use dpp::version::DefaultForPlatformVersion;
    use drive::state_transition_action::StateTransitionAction;

    for (group_bound, fails_in_state) in [(false, false), (true, false), (true, true)] {
        let version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();
        let (mut identity, signer, signing_key) =
            setup_identity_without_adding_it(958, dash_to_credits!(0.1));
        let dashpay = platform
            .drive
            .cache
            .system_data_contracts
            .load_dashpay(version)
            .unwrap();
        register_contract_group(&platform, KEY_GROUP, version);
        join_contract_group(
            &platform,
            dashpay.id(),
            KEY_GROUP,
            ContractGroupMember::Contract,
            version,
        );
        if group_bound {
            let mut stored_key = signing_key.clone();
            let IdentityPublicKey::V0(ref mut key) = stored_key else {
                panic!("expected a version 0 key")
            };
            key.contract_bounds = Some(ContractBounds::ContractGroup {
                id: Identifier::from(KEY_GROUP),
            });
            identity.add_public_key(stored_key);
        }
        platform
            .drive
            .add_new_identity(
                identity.clone(),
                false,
                &BlockInfo::default(),
                true,
                None,
                version,
            )
            .unwrap();
        let profile = dashpay.document_type_for_name("profile").unwrap();
        let mut rng = StdRng::seed_from_u64(433);
        let entropy = Bytes32::random_with_rng(&mut rng);
        let mut document = profile
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity.id(),
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                version,
            )
            .unwrap();
        document
            .set_id_for_creation(profile, &entropy.0, 2, version)
            .expect("expected to set the document id");
        set_valid_profile_payment_addresses(&mut document, profile);
        document.set("avatarUrl", "http://test.com/bob.jpg".into());
        let batch = if fails_in_state {
            // Replacing a profile that was never created fails in the transformer, which
            // still yields an action (the nonce bump) next to the error.
            BatchTransition::new_document_replacement_transition_from_document(
                document,
                profile,
                &signing_key,
                2,
                0,
                None,
                &signer,
                version,
                None,
            )
            .await
            .unwrap()
        } else {
            BatchTransition::new_document_creation_transition_from_document(
                document,
                profile,
                entropy.0,
                &signing_key,
                2,
                0,
                None,
                &signer,
                version,
                None,
            )
            .await
            .unwrap()
        };

        let state = platform.state.load();
        let platform_ref = PlatformRef {
            drive: &platform.drive,
            state: &state,
            config: &platform.config,
            core_rpc: &platform.core_rpc,
        };
        let signer_identity = identity.clone().into_partial_identity_info();
        let resolved_for = |signer: Option<&dpp::identity::PartialIdentity>| {
            let mut execution_context =
                StateTransitionExecutionContext::default_for_platform_version(version).unwrap();
            let result = match signer {
                Some(signer) => batch.transform_into_action_for_signer(
                    &platform_ref,
                    &BlockInfo::default(),
                    &None,
                    Some(signer),
                    ValidationMode::Validator,
                    &mut execution_context,
                    None,
                ),
                None => batch.transform_into_action(
                    &platform_ref,
                    &BlockInfo::default(),
                    &None,
                    ValidationMode::Validator,
                    &mut execution_context,
                    None,
                ),
            }
            .expect("expected to transform the batch");
            assert_eq!(
                result.errors.is_empty(),
                !fails_in_state,
                "{:?}",
                result.errors
            );
            let Some(StateTransitionAction::BatchAction(action)) = result.data else {
                panic!("expected a batch action");
            };
            action
                .contract_group_memberships(&dashpay.id())
                .map(|resolved| resolved.memberships.clone())
        };

        let with_signer = resolved_for(Some(&signer_identity));
        assert_eq!(
            with_signer.is_some(),
            group_bound && !fails_in_state,
            "group_bound={group_bound} fails_in_state={fails_in_state}"
        );
        if let Some(memberships) = with_signer {
            assert!(memberships.contains(&ContractGroupMembership {
                contract_group_id: Identifier::from(KEY_GROUP),
                member: ContractGroupMember::Contract,
            }));
        }
        assert!(
            resolved_for(None).is_none(),
            "a transformer that is not told who signed resolves nothing"
        );
    }
}

/// Advanced structure v1 judges a group-bound key from the memberships that transform v2
/// resolved into the action. Paired with an older transformer it would find none and fail every
/// such batch with a corrupted code execution error, so no protocol version may select one
/// without the other.
#[test]
fn should_pair_advanced_structure_v1_with_the_transformer_that_resolves_memberships() {
    for version in PLATFORM_VERSIONS {
        let batch = &version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .batch_state_transition;
        assert_eq!(
            batch.advanced_structure >= 1,
            batch.transform_into_action >= 2,
            "protocol version {}: advanced structure {} with transform_into_action {}",
            version.protocol_version,
            batch.advanced_structure,
            batch.transform_into_action
        );
    }
}
