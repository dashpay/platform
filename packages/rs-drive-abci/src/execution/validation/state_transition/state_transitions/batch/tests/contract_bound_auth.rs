use super::*;
use crate::execution::validation::state_transition::tests::setup_identity_without_adding_it;
use dpp::consensus::codes::ErrorWithCode;
use dpp::identity::contract_bounds::ContractBounds;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{accessors::IdentitySettersV0, IdentityPublicKey};
use dpp::state_transition::batch_transition::methods::v1::DocumentsBatchTransitionMethodsV1;

/// Sign with the original unbounded key metadata to deliberately bypass SDK preflight;
/// validators must enforce the bounds of the key stored in Drive.
#[tokio::test]
async fn should_enforce_contract_bounds_in_execution_and_preserve_paid_failure_nonces() {
    for case in [
        "allowed",
        "type_match",
        "wrong_contract",
        "wrong_type",
        "disabled",
        "mixed",
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
        let dpns = platform
            .drive
            .cache
            .system_data_contracts
            .load_dpns(version)
            .unwrap();
        let bounds = match case {
            "wrong_contract" => ContractBounds::SingleContract { id: dpns.id() },
            "type_match" => ContractBounds::SingleContractDocumentType {
                id: dashpay.id(),
                document_type_name: "profile".into(),
            },
            "wrong_type" => ContractBounds::SingleContractDocumentType {
                id: dashpay.id(),
                document_type_name: "contactRequest".into(),
            },
            _ => ContractBounds::SingleContract { id: dashpay.id() },
        };
        let mut stored_key = signing_key.clone();
        let IdentityPublicKey::V0(ref mut key) = stored_key else {
            panic!("expected a version 0 key")
        };
        key.contract_bounds = Some(bounds);
        if case == "disabled" {
            key.disabled_at = Some(99);
        }
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
        set_valid_profile_payment_addresses(&mut document, profile);
        document.set("avatarUrl", "http://test.com/bob.jpg".into());
        let mut batch = BatchTransition::new_document_creation_transition_from_document(
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
        if case == "mixed" {
            use dpp::state_transition::batch_transition::batched_transition::{
                document_transition::DocumentTransitionV0Methods, BatchedTransition,
            };
            use dpp::state_transition::batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
            use dpp::state_transition::StateTransition;
            let StateTransition::Batch(BatchTransition::V1(ref mut inner)) = batch else {
                panic!("expected v1 batch")
            };
            let mut second = inner.transitions[0].clone();
            let BatchedTransition::Document(ref mut doc) = second else {
                unreachable!()
            };
            doc.base_mut()
                .set_document_type_name("contactRequest".into());
            doc.base_mut()
                .set_id(dpp::prelude::Identifier::from([8; 32]));
            inner.transitions.push(second);
            batch
                .sign_external(
                    &signing_key,
                    &signer,
                    Some(|_, _| Ok(dpp::identity::SecurityLevel::HIGH)),
                )
                .await
                .unwrap();
        }
        let bytes = batch.serialize_to_bytes().unwrap();
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
            // Protocol 14 still limits batches to one member; mixed batches must
            // fail at basic validation before any bounds checks or fees.
            "mixed" => {
                assert!(
                    matches!(execution, StateTransitionExecutionResult::UnpaidConsensusError(error) if error.code() == 10412),
                    "{execution:?}"
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
                    None
                );
                assert_eq!(
                    platform
                        .drive
                        .fetch_identity_balance(identity.id().to_buffer(), Some(&tx), version)
                        .unwrap(),
                    Some(identity.balance())
                );
            }
            "allowed" | "type_match" => assert!(
                matches!(
                    execution,
                    StateTransitionExecutionResult::SuccessfulExecution { .. }
                ),
                "{case}: {execution:?}"
            ),
            "disabled" => assert!(
                matches!(
                    execution,
                    StateTransitionExecutionResult::UnpaidConsensusError(_)
                ),
                "{execution:?}"
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
                    Some((1u64 << 40) | 2)
                );
                let balance = platform
                    .drive
                    .fetch_identity_balance(identity.id().to_buffer(), Some(&tx), version)
                    .unwrap()
                    .unwrap();
                assert!(
                    balance < identity.balance(),
                    "invalid batch must pay validation fees"
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
                    "replay must not charge twice"
                );
            }
        }
    }
}

#[tokio::test]
async fn should_reject_non_batch_use_of_a_bound_authentication_key() {
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
    let mut stored_key = signing_key.clone();
    let IdentityPublicKey::V0(ref mut key) = stored_key else {
        panic!("expected a version 0 key")
    };
    key.contract_bounds = Some(ContractBounds::SingleContract { id: dashpay.id() });
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

#[tokio::test]
async fn should_bound_token_operations_to_the_bound_contract() {
    use crate::execution::validation::state_transition::tests::create_token_contract_with_owner_identity;
    use dpp::data_contract::TokenConfiguration;
    for bound_to_token_contract in [true, false] {
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
        let dashpay = platform
            .drive
            .cache
            .system_data_contracts
            .load_dashpay(version)
            .unwrap();
        let (mut identity, signer, signing_key) =
            setup_identity_without_adding_it(234, dash_to_credits!(0.1));
        let mut stored_key = signing_key.clone();
        let IdentityPublicKey::V0(ref mut key) = stored_key else {
            panic!("expected a version 0 key")
        };
        key.contract_bounds = Some(ContractBounds::SingleContract {
            id: if bound_to_token_contract {
                contract.id()
            } else {
                dashpay.id()
            },
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
        if bound_to_token_contract {
            assert!(
                matches!(
                    execution,
                    StateTransitionExecutionResult::SuccessfulExecution { .. }
                ),
                "{execution:?}"
            );
        } else {
            assert!(
                matches!(execution, StateTransitionExecutionResult::PaidConsensusError { error, .. } if error.code() == 20014),
                "{execution:?}"
            );
        }
        assert_eq!(
            platform
                .drive
                .fetch_identity_token_balance(
                    token_id.to_buffer(),
                    identity.id().to_buffer(),
                    Some(&tx),
                    version
                )
                .unwrap(),
            Some(if bound_to_token_contract { 10 } else { 15 })
        );
    }
}
