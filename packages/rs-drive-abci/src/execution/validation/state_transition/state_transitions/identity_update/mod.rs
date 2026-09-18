pub(crate) mod advanced_structure;
mod basic_structure;
mod nonce;
mod state;

use dpp::address_funds::PlatformAddress;
use dpp::block::block_info::BlockInfo;
use dpp::dashcore::Network;
use dpp::fee::Credits;
use dpp::prelude::AddressNonce;
use dpp::state_transition::identity_update_transition::IdentityUpdateTransition;
use dpp::validation::{ConsensusValidationResult, SimpleConsensusValidationResult};
use dpp::version::PlatformVersion;
use drive::state_transition_action::StateTransitionAction;
use std::collections::BTreeMap;

use drive::grovedb::TransactionArg;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;

use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;

use crate::execution::validation::state_transition::identity_update::basic_structure::v0::IdentityUpdateStateTransitionStructureValidationV0;
use crate::execution::validation::state_transition::identity_update::state::v0::IdentityUpdateStateTransitionStateValidationV0;
use crate::execution::validation::state_transition::identity_update::state::v1::IdentityUpdateStateTransitionStateValidationV1;
use crate::execution::validation::state_transition::processor::basic_structure::StateTransitionBasicStructureValidationV0;
use crate::execution::validation::state_transition::processor::state::StateTransitionStateValidation;
use crate::execution::validation::state_transition::transformer::StateTransitionActionTransformer;
use crate::execution::validation::state_transition::ValidationMode;
use crate::platform_types::platform_state::PlatformStateV0Methods;

impl StateTransitionActionTransformer for IdentityUpdateTransition {
    fn transform_into_action<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        _block_info: &BlockInfo,
        _remaining_address_input_balances: &Option<
            BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        >,
        _validation_mode: ValidationMode,
        _execution_context: &mut StateTransitionExecutionContext,
        _tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version = platform.state.current_platform_version()?;

        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .identity_update_state_transition
            .transform_into_action
        {
            0 => self.transform_into_action_v0(),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "identity update transition: transform_into_action".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

impl StateTransitionBasicStructureValidationV0 for IdentityUpdateTransition {
    fn validate_basic_structure(
        &self,
        _network_type: Network,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .identity_update_state_transition
            .basic_structure
        {
            Some(0) => self.validate_basic_structure_v0(platform_version),
            Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "identity update transition: validate_basic_structure".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Execution(ExecutionError::VersionNotActive {
                method: "identity update transition: validate_basic_structure".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}

impl StateTransitionStateValidation for IdentityUpdateTransition {
    fn validate_state<C: CoreRPCLike>(
        &self,
        _action: Option<StateTransitionAction>,
        platform: &PlatformRef<C>,
        _validation_mode: ValidationMode,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let platform_version = platform.state.current_platform_version()?;
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .identity_update_state_transition
            .state
        {
            0 => self.validate_state_v0(platform, tx, platform_version),
            1 => self.validate_state_v1(
                platform,
                block_info,
                execution_context,
                tx,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "identity update transition: validate_state".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::config::{PlatformConfig, PlatformTestConfig};
    use crate::execution::validation::state_transition::tests::{
        setup_add_key_to_identity, setup_identity_return_master_key,
    };
    use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
    use crate::test::helpers::contract_groups::{register_group, single_owner_info};
    use crate::test::helpers::setup::TestPlatformBuilder;
    use assert_matches::assert_matches;
    use dpp::block::block_info::BlockInfo;
    use dpp::consensus::codes::ErrorWithCode;
    use dpp::consensus::ConsensusError;
    use dpp::dash_to_credits;
    use dpp::dashcore::key::{Keypair, Secp256k1};
    use dpp::dashcore::signer;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::identifier::Identifier;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::contract_bounds::ContractBounds;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::identity::signer::Signer;
    use dpp::identity::KeyType::ECDSA_SECP256K1;
    use dpp::identity::{KeyType, Purpose, SecurityLevel};
    use dpp::serialization::{PlatformSerializable, Signable};
    use dpp::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0;
    use dpp::state_transition::identity_update_transition::IdentityUpdateTransition;
    use dpp::state_transition::proof_result::StateTransitionProofResult;
    use dpp::state_transition::public_key_in_creation::v0::IdentityPublicKeyInCreationV0;
    use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
    use dpp::state_transition::StateTransition;
    use drive::drive::Drive;
    use platform_version::version::PlatformVersion;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[tokio::test]
    async fn test_identity_update_that_disables_an_authentication_key() {
        let platform_config = PlatformConfig {
            testing_configs: PlatformTestConfig {
                disable_instant_lock_signature_verification: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let platform_version = PlatformVersion::latest();

        let mut platform = TestPlatformBuilder::new()
            .with_config(platform_config)
            .build_with_mock_rpc()
            .set_genesis_state();

        let (identity, signer, _, key) =
            setup_identity_return_master_key(&mut platform, 958, dash_to_credits!(0.1));

        let platform_state = platform.state.load();

        let update_transition: IdentityUpdateTransition = IdentityUpdateTransitionV0 {
            identity_id: identity.id(),
            revision: 1,
            nonce: 1,
            add_public_keys: vec![],
            disable_public_keys: vec![1],
            user_fee_increase: 0,
            signature_public_key_id: key.id(),
            signature: Default::default(),
        }
        .into();

        let mut update_transition: StateTransition = update_transition.into();

        let data = update_transition
            .signable_bytes()
            .expect("expected signable bytes");
        update_transition.set_signature(
            signer
                .sign(&key, data.as_slice())
                .await
                .expect("expected to sign"),
        );

        let update_transition_bytes = update_transition
            .serialize_to_bytes()
            .expect("expected to serialize");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![update_transition_bytes.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                true,
                None,
            )
            .expect("expected to process state transition");

        assert_eq!(processing_result.valid_count(), 1);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");

        let issues = platform
            .drive
            .grove
            .visualize_verify_grovedb(None, true, false, &platform_version.drive.grove_version)
            .expect("expected to have no issues");

        assert_eq!(issues.len(), 0);
    }

    #[tokio::test]
    async fn test_identity_update_that_adds_an_authentication_key() {
        let platform_version = PlatformVersion::latest();

        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let (identity, signer, _, key) =
            setup_identity_return_master_key(&mut platform, 958, dash_to_credits!(0.1));

        let platform_state = platform.state.load();

        let secp = Secp256k1::new();

        let mut rng = StdRng::seed_from_u64(292);

        let new_key_pair = Keypair::new(&secp, &mut rng);

        let mut new_key = IdentityPublicKeyInCreationV0 {
            id: 2,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            key_type: ECDSA_SECP256K1,
            read_only: false,
            data: new_key_pair.public_key().serialize().to_vec().into(),
            signature: Default::default(),
            contract_bounds: None,
        };

        let update_transition: IdentityUpdateTransition = IdentityUpdateTransitionV0 {
            identity_id: identity.id(),
            revision: 1,
            nonce: 1,
            add_public_keys: vec![IdentityPublicKeyInCreation::V0(new_key.clone())],
            disable_public_keys: vec![],
            user_fee_increase: 0,
            signature_public_key_id: key.id(),
            signature: Default::default(),
        }
        .into();

        let update_transition: StateTransition = update_transition.into();

        let signable_bytes = update_transition
            .signable_bytes()
            .expect("expected signable bytes");

        let secret = new_key_pair.secret_key();
        let signature =
            signer::sign(&signable_bytes, &secret.secret_bytes()).expect("expected to sign");

        new_key.signature = signature.to_vec().into();

        let update_transition: IdentityUpdateTransition = IdentityUpdateTransitionV0 {
            identity_id: identity.id(),
            revision: 1,
            nonce: 1,
            add_public_keys: vec![IdentityPublicKeyInCreation::V0(new_key)],
            disable_public_keys: vec![],
            user_fee_increase: 0,
            signature_public_key_id: key.id(),
            signature: Default::default(),
        }
        .into();

        let mut update_transition: StateTransition = update_transition.into();

        update_transition.set_signature(
            signer
                .sign(&key, signable_bytes.as_slice())
                .await
                .expect("expected to sign"),
        );

        let update_transition_bytes = update_transition
            .serialize_to_bytes()
            .expect("expected to serialize");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![update_transition_bytes.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                true,
                None,
            )
            .expect("expected to process state transition");

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let proof_result = platform
            .platform
            .drive
            .prove_state_transition(&update_transition, None, platform_version)
            .map_err(|e| e.to_string())
            .expect("expected to create proof");

        if let Some(proof_error) = proof_result.first_error() {
            panic!("proof_result is not valid with error {}", proof_error);
        }

        let proof_data = proof_result
            .into_data()
            .map_err(|e| e.to_string())
            .expect("expected to get proof data");

        let (_, verification_result) = Drive::verify_state_transition_was_executed_with_proof(
            &update_transition,
            &BlockInfo::default(),
            &proof_data,
            &|_id: &Identifier| Ok(None),
            platform_version,
        )
        .map(|(root_hash, outcome)| (root_hash, outcome.into_result()))
        .map_err(|e| e.to_string())
        .expect("expected to verify state transition");

        let StateTransitionProofResult::VerifiedPartialIdentity(_document) = verification_result
        else {
            panic!(
                "verification_result expected partial identity, but got: {:?}",
                verification_result
            );
        };
    }

    #[test]
    fn should_retain_contract_lookup_fees_only_after_activation() {
        use super::*;
        use crate::execution::types::execution_operation::ValidationOperation;
        use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContextMethodsV0;
        use dpp::data_contract::factory::DataContractFactory;
        use dpp::platform_value::platform_value;
        use dpp::version::DefaultForPlatformVersion;

        for protocol in [13, 14] {
            let version = PlatformVersion::get(protocol).unwrap();
            let mut platform = TestPlatformBuilder::new()
                .with_initial_protocol_version(protocol)
                .build_with_mock_rpc()
                .set_genesis_state();
            let (identity, _, _, master) =
                setup_identity_return_master_key(&mut platform, 958, dash_to_credits!(0.1));
            let factory = DataContractFactory::new(protocol).unwrap();
            let contract = factory
                .create_with_value_config(
                    identity.id(),
                    1,
                    platform_value!({
                        "note": { "type": "object", "requiresIdentityDecryptionBoundedKey": 0_u64,
                            "properties": {"text": {"type": "string", "maxLength": 64, "position": 0}}, "additionalProperties": false }
                    }),
                    None,
                    None,
                )
                .unwrap()
                .data_contract_owned();
            platform
                .drive
                .apply_contract(&contract, BlockInfo::default(), true, None, None, version)
                .unwrap();
            let state = platform.state.load();
            let platform_ref = PlatformRef {
                drive: &platform.drive,
                state: &state,
                config: &platform.config,
                core_rpc: &platform.core_rpc,
            };
            for missing in [false, true] {
                let id = if missing {
                    Identifier::from([0x73; 32])
                } else {
                    contract.id()
                };
                platform.drive.cache.data_contracts.clear();
                let expected = platform
                    .drive
                    .get_system_or_user_contract_with_fee(
                        id.to_buffer(),
                        &BlockInfo::default().epoch,
                        None,
                        version,
                    )
                    .unwrap();
                let expected_fee = expected.fee().unwrap().clone();
                assert!(expected_fee.processing_fee > 0);
                platform.drive.cache.data_contracts.clear();
                let update: IdentityUpdateTransition = IdentityUpdateTransitionV0 {
                    identity_id: identity.id(),
                    revision: 1,
                    nonce: 1,
                    add_public_keys: vec![IdentityPublicKeyInCreationV0 {
                        id: 2,
                        purpose: Purpose::DECRYPTION,
                        security_level: SecurityLevel::HIGH,
                        key_type: KeyType::ECDSA_HASH160,
                        data: vec![0x74; 20].into(),
                        read_only: false,
                        signature: Default::default(),
                        contract_bounds: Some(ContractBounds::SingleContractDocumentType {
                            id,
                            document_type_name: "note".into(),
                        }),
                    }
                    .into()],
                    disable_public_keys: vec![],
                    user_fee_increase: 0,
                    signature_public_key_id: master.id(),
                    signature: Default::default(),
                }
                .into();
                let mut context =
                    StateTransitionExecutionContext::default_for_platform_version(version).unwrap();
                let result = update
                    .validate_state(
                        None,
                        &platform_ref,
                        ValidationMode::Validator,
                        &BlockInfo::default(),
                        &mut context,
                        None,
                    )
                    .unwrap();
                assert_eq!(
                    result.is_valid(),
                    !missing,
                    "protocol {protocol}: {:?}",
                    result.errors
                );
                if missing {
                    assert_matches!(
                        result.into_data().unwrap(),
                        StateTransitionAction::BumpIdentityNonceAction(_)
                    );
                }
                if protocol == 13 {
                    assert!(
                        context.operations_slice().is_empty(),
                        "historical v0 discards its local validation costs"
                    );
                } else {
                    assert!(context.operations_slice().iter().any(|operation| matches!(operation,
                        ValidationOperation::PrecalculatedOperation(fee) if fee == &expected_fee
                    )), "contract lookup costs must reach the caller even on paid failure");
                }
            }
        }
    }

    #[tokio::test]
    async fn should_register_bound_authentication_key_and_preserve_proof_metadata() {
        use drive::drive::identity::key::fetch::{
            IdentityKeysRequest, KeyKindRequestType, KeyRequestType,
        };
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();
        let (identity, signer, _, key) =
            setup_identity_return_master_key(&mut platform, 958, dash_to_credits!(0.1));
        let dashpay = platform
            .drive
            .cache
            .system_data_contracts
            .load_dashpay(platform_version)
            .unwrap();
        let bounds = ContractBounds::SingleContractDocumentType {
            id: dashpay.id(),
            document_type_name: "profile".into(),
        };
        let platform_state = platform.state.load();
        let secp = Secp256k1::new();
        let mut rng = StdRng::seed_from_u64(292);
        let new_key_pair = Keypair::new(&secp, &mut rng);
        let mut new_key = IdentityPublicKeyInCreationV0 {
            id: 2,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            key_type: ECDSA_SECP256K1,
            read_only: false,
            data: new_key_pair.public_key().serialize().to_vec().into(),
            signature: Default::default(),
            contract_bounds: Some(bounds.clone()),
        };
        let build = |new_key: IdentityPublicKeyInCreationV0| -> StateTransition {
            IdentityUpdateTransition::from(IdentityUpdateTransitionV0 {
                identity_id: identity.id(),
                revision: 1,
                nonce: 1,
                add_public_keys: vec![IdentityPublicKeyInCreation::V0(new_key)],
                disable_public_keys: vec![],
                user_fee_increase: 0,
                signature_public_key_id: key.id(),
                signature: Default::default(),
            })
            .into()
        };
        let signable_bytes = build(new_key.clone()).signable_bytes().unwrap();
        new_key.signature =
            signer::sign(&signable_bytes, &new_key_pair.secret_key().secret_bytes())
                .unwrap()
                .to_vec()
                .into();
        let mut update_transition = build(new_key);
        update_transition
            .set_signature(signer.sign(&key, signable_bytes.as_slice()).await.unwrap());

        let transaction = platform.drive.grove.start_transaction();
        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![update_transition.serialize_to_bytes().unwrap()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                true,
                None,
            )
            .unwrap();
        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .unwrap();

        // The state transition proof carries the bounds of the registered key.
        let proof_result = platform
            .platform
            .drive
            .prove_state_transition(&update_transition, None, platform_version)
            .map_err(|e| e.to_string())
            .expect("expected to create proof");
        if let Some(proof_error) = proof_result.first_error() {
            panic!("proof_result is not valid with error {}", proof_error);
        }
        let proof_data = proof_result
            .into_data()
            .map_err(|e| e.to_string())
            .expect("expected to get proof data");
        let (_, verification_result) = Drive::verify_state_transition_was_executed_with_proof(
            &update_transition,
            &BlockInfo::default(),
            &proof_data,
            &|_id: &Identifier| Ok(None),
            platform_version,
        )
        .map(|(root_hash, outcome)| (root_hash, outcome.into_result()))
        .map_err(|e| e.to_string())
        .expect("expected to verify state transition");
        let StateTransitionProofResult::VerifiedPartialIdentity(proven) = verification_result
        else {
            panic!("expected a partial identity, got {verification_result:?}");
        };
        assert_eq!(
            proven.loaded_public_keys.get(&2).unwrap().contract_bounds(),
            Some(&bounds)
        );
        // The key is indexed as the current authentication key of the bound document type.
        let indexed = platform
            .drive
            .fetch_identity_keys_as_partial_identity(
                IdentityKeysRequest {
                    identity_id: identity.id().to_buffer(),
                    request_type: KeyRequestType::ContractDocumentTypeBoundKey(
                        dashpay.id().to_buffer(),
                        "profile".into(),
                        Purpose::AUTHENTICATION,
                        KeyKindRequestType::CurrentKeyOfKindRequest,
                    ),
                    limit: None,
                    offset: None,
                },
                None,
                platform_version,
            )
            .unwrap()
            .unwrap();
        assert_eq!(
            indexed
                .loaded_public_keys
                .get(&2)
                .unwrap()
                .contract_bounds(),
            Some(&bounds)
        );

        // Revocation through the master key keeps the bounds on the disabled key.
        let mut revoke: StateTransition =
            IdentityUpdateTransition::from(IdentityUpdateTransitionV0 {
                identity_id: identity.id(),
                revision: 2,
                nonce: 2,
                add_public_keys: vec![],
                disable_public_keys: vec![2],
                user_fee_increase: 0,
                signature_public_key_id: key.id(),
                signature: Default::default(),
            })
            .into();
        revoke.set_signature(
            signer
                .sign(&key, &revoke.signable_bytes().unwrap())
                .await
                .unwrap(),
        );
        let block = BlockInfo {
            time_ms: 50,
            ..Default::default()
        };
        let tx = platform.drive.grove.start_transaction();
        let result = platform
            .platform
            .process_raw_state_transitions(
                &vec![revoke.serialize_to_bytes().unwrap()],
                &platform_state,
                &block,
                &tx,
                platform_version,
                true,
                None,
            )
            .unwrap();
        assert_eq!(result.valid_count(), 1);
        platform
            .drive
            .grove
            .commit_transaction(tx)
            .unwrap()
            .unwrap();
        let fetched = platform
            .drive
            .fetch_identity_keys_as_partial_identity(
                IdentityKeysRequest::new_specific_key_query(&identity.id().to_buffer(), 2),
                None,
                platform_version,
            )
            .unwrap()
            .unwrap();
        let revoked = fetched.loaded_public_keys.get(&2).unwrap();
        assert_eq!(revoked.disabled_at(), Some(50));
        assert_eq!(revoked.contract_bounds(), Some(&bounds));
        assert!(platform
            .drive
            .grove
            .visualize_verify_grovedb(None, true, false, &platform_version.drive.grove_version)
            .unwrap()
            .is_empty());
    }

    /// A key bound to a contract group is registered through the whole pipeline only once the
    /// group exists; it is then indexed under the group, and stays bound after revocation.
    #[tokio::test]
    async fn should_register_and_revoke_an_authentication_key_bound_to_a_contract_group() {
        use drive::drive::identity::key::fetch::IdentityKeysRequest;
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();
        let (identity, signer, _, key) =
            setup_identity_return_master_key(&mut platform, 958, dash_to_credits!(0.1));
        let contract_group_id = Identifier::from([0x61; 32]);
        let bounds = ContractBounds::ContractGroup {
            id: contract_group_id,
        };
        let platform_state = platform.state.load();
        let secp = Secp256k1::new();
        let mut rng = StdRng::seed_from_u64(293);
        let new_key_pair = Keypair::new(&secp, &mut rng);
        let build =
            |revision: u64, nonce: u64, add: Vec<IdentityPublicKeyInCreationV0>, disable| {
                StateTransition::from(IdentityUpdateTransition::from(IdentityUpdateTransitionV0 {
                    identity_id: identity.id(),
                    revision,
                    nonce,
                    add_public_keys: add
                        .into_iter()
                        .map(IdentityPublicKeyInCreation::V0)
                        .collect(),
                    disable_public_keys: disable,
                    user_fee_increase: 0,
                    signature_public_key_id: key.id(),
                    signature: Default::default(),
                }))
            };
        let mut signed_updates = Vec::new();
        for (revision, nonce) in [(1, 1), (1, 2)] {
            let mut new_key = IdentityPublicKeyInCreationV0 {
                id: 2,
                purpose: Purpose::AUTHENTICATION,
                security_level: SecurityLevel::HIGH,
                key_type: ECDSA_SECP256K1,
                read_only: false,
                data: new_key_pair.public_key().serialize().to_vec().into(),
                signature: Default::default(),
                contract_bounds: Some(bounds.clone()),
            };
            let signable_bytes = build(revision, nonce, vec![new_key.clone()], vec![])
                .signable_bytes()
                .unwrap();
            new_key.signature =
                signer::sign(&signable_bytes, &new_key_pair.secret_key().secret_bytes())
                    .unwrap()
                    .to_vec()
                    .into();
            let mut update = build(revision, nonce, vec![new_key], vec![]);
            update.set_signature(signer.sign(&key, signable_bytes.as_slice()).await.unwrap());
            signed_updates.push(update);
        }

        // The group does not exist yet: a paid failure that registers nothing.
        let transaction = platform.drive.grove.start_transaction();
        let result = platform
            .platform
            .process_raw_state_transitions(
                &vec![signed_updates[0].serialize_to_bytes().unwrap()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                true,
                None,
            )
            .unwrap();
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError { error, .. }]
                if error.code() == 41001
        );
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .unwrap();

        register_group(
            &platform,
            contract_group_id,
            &single_owner_info(Identifier::from([0x60; 32]), None, None),
            platform_version,
        );
        let transaction = platform.drive.grove.start_transaction();
        let result = platform
            .platform
            .process_raw_state_transitions(
                &vec![signed_updates[1].serialize_to_bytes().unwrap()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                true,
                None,
            )
            .unwrap();
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .unwrap();

        // The key is indexed as the current authentication key of the group.
        let group_keys_request = || {
            IdentityKeysRequest::new_contract_group_authentication_keys_query(
                identity.id().to_buffer(),
                contract_group_id.to_buffer(),
            )
        };
        let indexed = platform
            .drive
            .fetch_identity_keys_as_partial_identity(group_keys_request(), None, platform_version)
            .unwrap()
            .unwrap();
        assert_eq!(
            indexed
                .loaded_public_keys
                .get(&2)
                .unwrap()
                .contract_bounds(),
            Some(&bounds)
        );

        // Revocation through the master key refreshes the group references and keeps the
        // bounds on the disabled key.
        let mut revoke = build(2, 3, vec![], vec![2]);
        revoke.set_signature(
            signer
                .sign(&key, &revoke.signable_bytes().unwrap())
                .await
                .unwrap(),
        );
        let block = BlockInfo {
            time_ms: 50,
            ..Default::default()
        };
        let transaction = platform.drive.grove.start_transaction();
        let result = platform
            .platform
            .process_raw_state_transitions(
                &vec![revoke.serialize_to_bytes().unwrap()],
                &platform_state,
                &block,
                &transaction,
                platform_version,
                true,
                None,
            )
            .unwrap();
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .unwrap();
        let fetched = platform
            .drive
            .fetch_identity_keys_as_partial_identity(
                IdentityKeysRequest::new_specific_key_query(&identity.id().to_buffer(), 2),
                None,
                platform_version,
            )
            .unwrap()
            .unwrap();
        let revoked = fetched.loaded_public_keys.get(&2).unwrap();
        assert_eq!(revoked.disabled_at(), Some(50));
        assert_eq!(revoked.contract_bounds(), Some(&bounds));
        assert!(platform
            .drive
            .grove
            .visualize_verify_grovedb(None, true, false, &platform_version.drive.grove_version)
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn should_refresh_every_bound_key_reference_after_revocation() {
        use drive::drive::identity::key::fetch::{
            IdentityKeysRequest, KeyKindRequestType, KeyRequestType,
        };
        let version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();
        let (mut identity, mut signer, _, master) =
            setup_identity_return_master_key(&mut platform, 958, dash_to_credits!(0.1));
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
        let contract_bound = ContractBounds::SingleContract { id: dashpay.id() };
        let type_bound = ContractBounds::SingleContractDocumentType {
            id: dpns.id(),
            document_type_name: "preorder".into(),
        };
        let contract_key = setup_add_key_to_identity(
            &mut platform,
            &mut identity,
            &mut signer,
            4,
            2,
            Purpose::AUTHENTICATION,
            SecurityLevel::HIGH,
            KeyType::ECDSA_SECP256K1,
            Some(contract_bound.clone()),
        );
        let type_key = setup_add_key_to_identity(
            &mut platform,
            &mut identity,
            &mut signer,
            5,
            3,
            Purpose::AUTHENTICATION,
            SecurityLevel::HIGH,
            KeyType::ECDSA_SECP256K1,
            Some(type_bound.clone()),
        );
        let mut update: StateTransition =
            IdentityUpdateTransition::from(IdentityUpdateTransitionV0 {
                identity_id: identity.id(),
                revision: 1,
                nonce: 1,
                add_public_keys: vec![],
                disable_public_keys: vec![contract_key.id(), type_key.id()],
                user_fee_increase: 0,
                signature_public_key_id: master.id(),
                signature: Default::default(),
            })
            .into();
        update.set_signature(
            signer
                .sign(&master, &update.signable_bytes().unwrap())
                .await
                .unwrap(),
        );
        let block = BlockInfo {
            time_ms: 1001,
            ..Default::default()
        };
        let transaction = platform.drive.grove.start_transaction();
        let result = platform
            .platform
            .process_raw_state_transitions(
                &vec![update.serialize_to_bytes().unwrap()],
                &platform.state.load(),
                &block,
                &transaction,
                version,
                true,
                None,
            )
            .unwrap();
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .unwrap();

        // Both current-key references must resolve to the updated keys, not their old hashes.
        for (key, bounds, request_type) in [
            (
                &contract_key,
                &contract_bound,
                KeyRequestType::ContractBoundKey(
                    dashpay.id().to_buffer(),
                    Purpose::AUTHENTICATION,
                    KeyKindRequestType::CurrentKeyOfKindRequest,
                ),
            ),
            (
                &type_key,
                &type_bound,
                KeyRequestType::ContractDocumentTypeBoundKey(
                    dpns.id().to_buffer(),
                    "preorder".into(),
                    Purpose::AUTHENTICATION,
                    KeyKindRequestType::CurrentKeyOfKindRequest,
                ),
            ),
        ] {
            let fetched = platform
                .drive
                .fetch_identity_keys_as_partial_identity(
                    IdentityKeysRequest {
                        identity_id: identity.id().to_buffer(),
                        request_type,
                        limit: None,
                        offset: None,
                    },
                    None,
                    version,
                )
                .unwrap()
                .unwrap();
            let refreshed = fetched
                .loaded_public_keys
                .get(&key.id())
                .expect("bound key reference");
            assert_eq!(refreshed.disabled_at(), Some(block.time_ms));
            assert_eq!(refreshed.contract_bounds(), Some(bounds));
        }
        assert!(
            platform
                .drive
                .grove
                .visualize_verify_grovedb(None, true, false, &version.drive.grove_version)
                .unwrap()
                .is_empty(),
            "revocation must leave no stale GroveDB references"
        );
    }

    #[tokio::test]
    async fn should_keep_the_newest_bound_key_current_across_registration_and_revocation() {
        use drive::config::DriveConfig;
        use drive::drive::identity::key::fetch::{
            IdentityKeysRequest, KeyIDVec, KeyKindRequestType, KeyRequestType,
        };
        use std::collections::BTreeMap;
        let version = PlatformVersion::latest();
        // Consistency verification makes GroveDB reject two pending operations on one slot,
        // which is what bound keys covering one contract queue for its current-key alias.
        let mut platform = TestPlatformBuilder::new()
            .with_config(PlatformConfig {
                drive: DriveConfig {
                    batching_consistency_verification: true,
                    ..DriveConfig::default_testnet()
                },
                ..Default::default()
            })
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();
        let (identity, signer, _, master) =
            setup_identity_return_master_key(&mut platform, 958, dash_to_credits!(0.1));
        let dashpay = platform
            .drive
            .cache
            .system_data_contracts
            .load_dashpay(version)
            .unwrap();
        let bounds = ContractBounds::SingleContract { id: dashpay.id() };
        let secp = Secp256k1::new();
        let mut rng = StdRng::seed_from_u64(292);
        let pairs: BTreeMap<u32, Keypair> = [2u32, 3, 4]
            .into_iter()
            .map(|id| (id, Keypair::new(&secp, &mut rng)))
            .collect();
        let bound_key = |id: u32| IdentityPublicKeyInCreationV0 {
            id,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            key_type: ECDSA_SECP256K1,
            read_only: false,
            data: pairs[&id].public_key().serialize().to_vec().into(),
            signature: Default::default(),
            contract_bounds: Some(bounds.clone()),
        };
        let unsigned = |revision: u64,
                        add: Vec<IdentityPublicKeyInCreationV0>,
                        disable: Vec<u32>|
         -> StateTransition {
            IdentityUpdateTransition::from(IdentityUpdateTransitionV0 {
                identity_id: identity.id(),
                revision,
                nonce: revision,
                add_public_keys: add
                    .into_iter()
                    .map(IdentityPublicKeyInCreation::V0)
                    .collect(),
                disable_public_keys: disable,
                user_fee_increase: 0,
                signature_public_key_id: master.id(),
                signature: Default::default(),
            })
            .into()
        };
        let apply = |transition: &StateTransition, time_ms: u64| {
            let transaction = platform.drive.grove.start_transaction();
            let result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition.serialize_to_bytes().unwrap()],
                    &platform.state.load(),
                    &BlockInfo {
                        time_ms,
                        ..Default::default()
                    },
                    &transaction,
                    version,
                    true,
                    None,
                )
                .unwrap();
            assert_matches!(
                result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution { .. }],
                "update at {time_ms} must apply as one consistent batch"
            );
            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .unwrap();
        };
        let key_ids = |kind: KeyKindRequestType| -> Vec<u32> {
            platform
                .drive
                .fetch_identity_keys::<KeyIDVec>(
                    IdentityKeysRequest {
                        identity_id: identity.id().to_buffer(),
                        request_type: KeyRequestType::ContractBoundKey(
                            dashpay.id().to_buffer(),
                            Purpose::AUTHENTICATION,
                            kind,
                        ),
                        limit: Some(16),
                        offset: None,
                    },
                    None,
                    version,
                )
                .unwrap()
        };
        let disabled_at = |key_id: u32| -> Option<u64> {
            platform
                .drive
                .fetch_identity_keys_as_partial_identity(
                    IdentityKeysRequest {
                        identity_id: identity.id().to_buffer(),
                        request_type: KeyRequestType::SpecificKeys(vec![key_id]),
                        limit: Some(1),
                        offset: None,
                    },
                    None,
                    version,
                )
                .unwrap()
                .unwrap()
                .loaded_public_keys[&key_id]
                .disabled_at()
        };
        let assert_slot = |current: u32, all: &[u32]| {
            assert_eq!(
                key_ids(KeyKindRequestType::CurrentKeyOfKindRequest),
                vec![current],
                "current key"
            );
            assert_eq!(
                key_ids(KeyKindRequestType::AllKeysOfKindRequest),
                all,
                "listing must not repeat the current key alias"
            );
        };

        // 1. Two bound keys in one update, listed newest first: the highest key id must be
        //    current regardless of input order.
        let mut adds = vec![bound_key(3), bound_key(2)];
        let signable = unsigned(1, adds.clone(), vec![]).signable_bytes().unwrap();
        for key in &mut adds {
            key.signature = signer::sign(&signable, &pairs[&key.id].secret_key().secret_bytes())
                .unwrap()
                .to_vec()
                .into();
        }
        let mut registration = unsigned(1, adds, vec![]);
        registration.set_signature(signer.sign(&master, &signable).await.unwrap());
        apply(&registration, 1000);
        assert_slot(3, &[2, 3]);

        // 2. Register a replacement and revoke the current key in the same transition: the
        //    replacement must become current, not the revoked key or a batch conflict.
        let mut adds = vec![bound_key(4)];
        let signable = unsigned(2, adds.clone(), vec![3]).signable_bytes().unwrap();
        for key in &mut adds {
            key.signature = signer::sign(&signable, &pairs[&key.id].secret_key().secret_bytes())
                .unwrap()
                .to_vec()
                .into();
        }
        let mut replacement = unsigned(2, adds, vec![3]);
        replacement.set_signature(signer.sign(&master, &signable).await.unwrap());
        apply(&replacement, 2000);
        assert_slot(4, &[2, 3, 4]);
        assert_eq!(disabled_at(3), Some(2000));
        assert_eq!(disabled_at(4), None);

        // 3. Revoking an older key on its own must not repoint the alias at it.
        let mut revocation = unsigned(3, vec![], vec![2]);
        revocation.set_signature(
            signer
                .sign(&master, &revocation.signable_bytes().unwrap())
                .await
                .unwrap(),
        );
        apply(&revocation, 3000);
        assert_slot(4, &[2, 3, 4]);
        assert_eq!(disabled_at(2), Some(3000));
        assert!(
            platform
                .drive
                .grove
                .visualize_verify_grovedb(None, true, false, &version.drive.grove_version)
                .unwrap()
                .is_empty(),
            "registration and revocation must leave no stale GroveDB references"
        );
    }

    #[tokio::test]
    async fn should_estimate_bound_key_revocation_at_least_at_its_execution_cost() {
        let version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();
        let (mut identity, mut signer, _, _) =
            setup_identity_return_master_key(&mut platform, 958, dash_to_credits!(0.1));
        let dashpay = platform
            .drive
            .cache
            .system_data_contracts
            .load_dashpay(version)
            .unwrap();
        let bound = setup_add_key_to_identity(
            &mut platform,
            &mut identity,
            &mut signer,
            4,
            2,
            Purpose::AUTHENTICATION,
            SecurityLevel::HIGH,
            KeyType::ECDSA_SECP256K1,
            Some(ContractBounds::SingleContract { id: dashpay.id() }),
        );
        let plain = setup_add_key_to_identity(
            &mut platform,
            &mut identity,
            &mut signer,
            5,
            3,
            Purpose::AUTHENTICATION,
            SecurityLevel::HIGH,
            KeyType::ECDSA_SECP256K1,
            None,
        );
        let block = BlockInfo {
            time_ms: 1001,
            ..Default::default()
        };
        let estimate = |key_id: u32| {
            platform
                .drive
                .disable_identity_keys(
                    identity.id().to_buffer(),
                    vec![key_id],
                    block.time_ms,
                    &block,
                    false,
                    None,
                    version,
                )
                .unwrap()
        };
        let estimated_bound = estimate(bound.id());
        let estimated_plain = estimate(plain.id());
        // The bound key has contract-info references to refresh; v0 estimated with an unbounded
        // stand-in key and priced none of them.
        assert!(
            estimated_bound.processing_fee > estimated_plain.processing_fee,
            "bound revocation estimate {} must exceed the unbounded one {}",
            estimated_bound.processing_fee,
            estimated_plain.processing_fee
        );
        let actual = platform
            .drive
            .disable_identity_keys(
                identity.id().to_buffer(),
                vec![bound.id()],
                block.time_ms,
                &block,
                true,
                None,
                version,
            )
            .unwrap();
        assert!(
            estimated_bound.processing_fee >= actual.processing_fee,
            "estimate {} must cover execution {}",
            estimated_bound.processing_fee,
            actual.processing_fee
        );
        assert!(estimated_bound.storage_fee >= actual.storage_fee);
    }

    #[tokio::test]
    async fn should_estimate_bound_key_fees_from_the_real_contract_lookup() {
        use dpp::data_contract::factory::DataContractFactory;
        use dpp::identity::IdentityPublicKey;
        use dpp::platform_value::{platform_value, Value};
        let version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();
        let (identity, _, _, _) =
            setup_identity_return_master_key(&mut platform, 958, dash_to_credits!(0.1));
        // Two user contracts, one bound at contract level and one at document-type level.
        // Neither is served from the system-contract cache.
        let factory = DataContractFactory::new(version.protocol_version).unwrap();
        let document_type = |n: usize| {
            (
                Value::Text(format!("t{n:02}")),
                platform_value!({"type": "object",
                    "properties": {"text": {"type": "string", "maxLength": 64, "position": 0},
                        "note": {"type": "string", "maxLength": 128, "position": 1}},
                    "additionalProperties": false}),
            )
        };
        let mut contracts = Vec::new();
        for (nonce, document_types) in [(1u64, 1usize), (2, 16)] {
            let contract = factory
                .create_with_value_config(
                    identity.id(),
                    nonce,
                    Value::Map((0..document_types).map(document_type).collect()),
                    None,
                    None,
                )
                .unwrap()
                .data_contract_owned();
            platform
                .drive
                .apply_contract(&contract, BlockInfo::default(), true, None, None, version)
                .unwrap();
            contracts.push(contract);
        }
        let mut rng = StdRng::seed_from_u64(77);
        let keys: Vec<IdentityPublicKey> = contracts
            .iter()
            .enumerate()
            .map(|(index, contract)| {
                IdentityPublicKey::random_key_with_known_attributes(
                    2 + index as u32,
                    &mut rng,
                    Purpose::AUTHENTICATION,
                    SecurityLevel::HIGH,
                    KeyType::ECDSA_SECP256K1,
                    Some(ContractBounds::SingleContractDocumentType {
                        id: contract.id(),
                        document_type_name: "t00".into(),
                    }),
                    version,
                )
                .unwrap()
                .0
            })
            .collect();
        let block = BlockInfo {
            time_ms: 1001,
            ..Default::default()
        };
        let register = |key: &IdentityPublicKey, apply: bool| {
            platform.drive.cache.data_contracts.clear();
            platform
                .drive
                .add_new_unique_keys_to_identity(
                    identity.id().to_buffer(),
                    vec![key.clone()],
                    &block,
                    apply,
                    None,
                    version,
                )
                .unwrap()
        };
        let revoke = |key: &IdentityPublicKey, apply: bool| {
            platform.drive.cache.data_contracts.clear();
            platform
                .drive
                .disable_identity_keys(
                    identity.id().to_buffer(),
                    vec![key.id()],
                    block.time_ms,
                    &block,
                    apply,
                    None,
                    version,
                )
                .unwrap()
        };

        let estimated_registration: Vec<_> = keys.iter().map(|key| register(key, false)).collect();
        let actual_registration: Vec<_> = keys.iter().map(|key| register(key, true)).collect();

        // The estimated operations must bill the same contract lookups as the applied
        // operations: one real fee per bound group, not a fixed stand-in.
        use dpp::block::epoch::Epoch;
        use drive::fees::op::LowLevelDriveOperation;
        use std::collections::HashMap;
        let lookup_fees = |operations: &[LowLevelDriveOperation]| -> Vec<u64> {
            operations
                .iter()
                .filter_map(|operation| match operation {
                    LowLevelDriveOperation::PreCalculatedFeeResult(fee) => Some(fee.processing_fee),
                    _ => None,
                })
                .collect()
        };
        let key_ids: Vec<_> = keys.iter().map(|key| key.id()).collect();
        let revocation_operations = |estimate: bool| {
            platform.drive.cache.data_contracts.clear();
            let mut layer_info = estimate.then(HashMap::new);
            platform
                .drive
                .disable_identity_keys_operations(
                    identity.id().to_buffer(),
                    key_ids.clone(),
                    block.time_ms,
                    &Epoch::new(0).unwrap(),
                    &mut layer_info,
                    None,
                    version,
                )
                .unwrap()
        };
        let estimated_lookups = lookup_fees(&revocation_operations(true));
        let applied_lookups = lookup_fees(&revocation_operations(false));
        assert!(
            estimated_lookups.len() >= keys.len(),
            "at least one contract lookup fee per bound key: {estimated_lookups:?}"
        );
        assert_eq!(
            estimated_lookups, applied_lookups,
            "the estimate must price the same contract lookups the apply path bills"
        );

        let estimated_revocation: Vec<_> = keys.iter().map(|key| revoke(key, false)).collect();
        let actual_revocation: Vec<_> = keys.iter().map(|key| revoke(key, true)).collect();
        for (estimated, actual) in estimated_registration
            .iter()
            .zip(&actual_registration)
            .chain(estimated_revocation.iter().zip(&actual_revocation))
        {
            assert!(
                estimated.processing_fee >= actual.processing_fee,
                "estimate {} must cover execution {}",
                estimated.processing_fee,
                actual.processing_fee
            );
            assert!(estimated.storage_fee >= actual.storage_fee);
        }
    }

    #[tokio::test]
    async fn test_identity_update_that_disables_an_encryption_key() {
        let platform_config = PlatformConfig {
            testing_configs: PlatformTestConfig {
                disable_instant_lock_signature_verification: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let platform_version = PlatformVersion::latest();

        let mut platform = TestPlatformBuilder::new()
            .with_config(platform_config)
            .build_with_mock_rpc()
            .set_genesis_state();

        let (mut identity, mut signer, _, master_key) =
            setup_identity_return_master_key(&mut platform, 958, dash_to_credits!(0.1));

        let dashpay = platform
            .drive
            .cache
            .system_data_contracts
            .load_dashpay(platform_version)
            .expect("expected the dashpay system contract");

        let key = setup_add_key_to_identity(
            &mut platform,
            &mut identity,
            &mut signer,
            4,
            2,
            Purpose::ENCRYPTION,
            SecurityLevel::MEDIUM,
            KeyType::ECDSA_SECP256K1,
            Some(ContractBounds::SingleContractDocumentType {
                id: dashpay.id(),
                document_type_name: "contactRequest".to_string(),
            }),
        );

        let issues = platform
            .drive
            .grove
            .visualize_verify_grovedb(None, true, false, &platform_version.drive.grove_version)
            .expect("expected to have no issues");

        assert_eq!(
            issues.len(),
            0,
            "issues are {}",
            issues
                .iter()
                .map(|(hash, (a, b, c))| format!("{}: {} {} {}", hash, a, b, c))
                .collect::<Vec<_>>()
                .join(" | ")
        );

        let platform_state = platform.state.load();

        let update_transition: IdentityUpdateTransition = IdentityUpdateTransitionV0 {
            identity_id: identity.id(),
            revision: 1,
            nonce: 1,
            add_public_keys: vec![],
            disable_public_keys: vec![key.id()],
            user_fee_increase: 0,
            signature_public_key_id: master_key.id(),
            signature: Default::default(),
        }
        .into();

        let mut update_transition: StateTransition = update_transition.into();

        let data = update_transition
            .signable_bytes()
            .expect("expected signable bytes");
        update_transition.set_signature(
            signer
                .sign(&master_key, data.as_slice())
                .await
                .expect("expected to sign"),
        );

        let update_transition_bytes = update_transition
            .serialize_to_bytes()
            .expect("expected to serialize");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![update_transition_bytes.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                true,
                None,
            )
            .expect("expected to process state transition");

        assert_eq!(processing_result.valid_count(), 1);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");

        let issues = platform
            .drive
            .grove
            .visualize_verify_grovedb(None, true, false, &platform_version.drive.grove_version)
            .expect("expected to have no issues");

        assert_eq!(
            issues.len(),
            0,
            "issues are {}",
            issues
                .iter()
                .map(|(hash, (a, b, c))| format!("{}: {} {} {}", hash, a, b, c))
                .collect::<Vec<_>>()
                .join(" | ")
        );
    }

    #[tokio::test]
    async fn test_identity_update_adding_owner_key_not_allowed() {
        let platform_config = PlatformConfig {
            testing_configs: PlatformTestConfig {
                disable_instant_lock_signature_verification: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let platform_version = PlatformVersion::latest();

        let mut platform = TestPlatformBuilder::new()
            .with_config(platform_config)
            .build_with_mock_rpc()
            .set_genesis_state();

        let (identity, signer, _, key) =
            setup_identity_return_master_key(&mut platform, 958, dash_to_credits!(0.1));

        let platform_state = platform.state.load();

        let secp = Secp256k1::new();

        let mut rng = StdRng::seed_from_u64(1292);

        let new_key_pair = Keypair::new(&secp, &mut rng);

        let new_key = IdentityPublicKeyInCreationV0 {
            id: 2,
            purpose: Purpose::OWNER,
            security_level: SecurityLevel::HIGH,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: new_key_pair.public_key().serialize().to_vec().into(),
            signature: Default::default(),
            contract_bounds: None,
        };

        let update_transition: IdentityUpdateTransition = IdentityUpdateTransitionV0 {
            identity_id: identity.id(),
            revision: 1,
            nonce: 1,
            add_public_keys: vec![new_key.into()],
            disable_public_keys: vec![],
            user_fee_increase: 0,
            signature_public_key_id: key.id(),
            signature: Default::default(),
        }
        .into();

        let mut update_transition: StateTransition = update_transition.into();

        let data = update_transition
            .signable_bytes()
            .expect("expected signable bytes");
        update_transition.set_signature(
            signer
                .sign(&key, data.as_slice())
                .await
                .expect("expected to sign"),
        );

        let update_transition_bytes = update_transition
            .serialize_to_bytes()
            .expect("expected to serialize");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![update_transition_bytes.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                true,
                None,
            )
            .expect("expected to process state transition");

        // We expect there to be an error because you should not be able to add owner keys
        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::UnpaidConsensusError(
                ConsensusError::BasicError(_)
            )]
        );

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");

        let issues = platform
            .drive
            .grove
            .visualize_verify_grovedb(None, true, false, &platform_version.drive.grove_version)
            .expect("expected to have no issues");

        assert_eq!(issues.len(), 0);
    }

    #[tokio::test]
    async fn test_identity_update_adding_contract_bound_key() {
        use crate::execution::validation::state_transition::tests::{
            register_contract_from_bytes, IdentityTestInfo,
        };

        let platform_config = PlatformConfig {
            testing_configs: PlatformTestConfig {
                disable_instant_lock_signature_verification: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let platform_version = PlatformVersion::latest();

        let mut platform = TestPlatformBuilder::new()
            .with_config(platform_config)
            .build_with_mock_rpc()
            .set_genesis_state();

        // {
        //   "$formatVersion": "1",
        //   "id": "5pkMhyeaFjJfVMkFhLtJdDp2ofx6iqt7i9k6ckkHBwbs",
        //   "config": {
        //     "$formatVersion": "1",
        //     "canBeDeleted": false,
        //     "readonly": false,
        //     "keepsHistory": false,
        //     "documentsKeepHistoryContractDefault": false,
        //     "documentsMutableContractDefault": true,
        //     "documentsCanBeDeletedContractDefault": true,
        //     "requiresIdentityEncryptionBoundedKey": 0,
        //     "requiresIdentityDecryptionBoundedKey": 0,
        //     "sizedIntegerTypes": true
        //   },
        //   "version": 1,
        //   "ownerId": "DicUmimv71VqxNBzZHXb887RgssSEjjx7DyLfxrt8q1X",
        //   "schemaDefs": null,
        //   "documentSchemas": {
        //     "preorder": {
        //       "documentsMutable": false,
        //       "canBeDeleted": true,
        //       "type": "object",
        //       "indices": [
        //         {
        //           "name": "saltedHash",
        //           "properties": [
        //             {
        //               "saltedDomainHash": "asc"
        //             }
        //           ],
        //           "unique": true
        //         }
        //       ],
        //       "properties": {
        //         "saltedDomainHash": {
        //           "type": "array",
        //           "byteArray": true,
        //           "minItems": 32,
        //           "maxItems": 32,
        //           "position": 0,
        //           "description": "Double sha-256 of the concatenation of a 32 byte random salt and a normalized domain name"
        //         }
        //       },
        //       "required": [
        //         "saltedDomainHash"
        //       ],
        //       "additionalProperties": false,
        //       "$comment": "Preorder documents are immutable: modification and deletion are restricted"
        //     }
        //   },
        //   "createdAt": 1749816974718,
        //   "updatedAt": null,
        //   "createdAtBlockHeight": 159130,
        //   "updatedAtBlockHeight": null,
        //   "createdAtEpoch": 7906,
        //   "updatedAtEpoch": null,
        //   "groups": {},
        //   "tokens": {},
        //   "keywords": [],
        //   "description": null
        // }
        let contract_bytes = hex::decode("0147aa11d517710d509edaf84bb54902394dcb8f6cc68775138d1cdd8334600d2e01000000000101010001000101bcf52c1c5d57d2e21530c5d03ef4c6e7b39a91da7c444fdb17e0a7746b6285860001087072656f7264657216081210646f63756d656e74734d757461626c651300120c63616e426544656c65746564130012047479706512066f626a6563741207696e64696365731501160312046e616d65120a73616c74656448617368120a70726f7065727469657315011601121073616c746564446f6d61696e4861736812036173631206756e697175651301120a70726f706572746965731601121073616c746564446f6d61696e486173681606120474797065120561727261791209627974654172726179130112086d696e4974656d73022012086d61784974656d7302201208706f736974696f6e0200120b6465736372697074696f6e1259446f75626c65207368612d323536206f662074686520636f6e636174656e6174696f6e206f66206120333220627974652072616e646f6d2073616c7420616e642061206e6f726d616c697a656420646f6d61696e206e616d65120872657175697265641501121073616c746564446f6d61696e4861736812146164646974696f6e616c50726f706572746965731300120824636f6d6d656e74124a5072656f7264657220646f63756d656e74732061726520696d6d757461626c653a206d6f64696669636174696f6e20616e642064656c6574696f6e20617265207265737472696374656401fd0000019769381d7e0001fc00026d9a0001fb1ee20000000000").expect("expected to decode contract bytes");

        let (identity, signer, critical_key, master_key) =
            setup_identity_return_master_key(&mut platform, 958, dash_to_credits!(5.0));

        let platform_state = platform.state.load();

        // Register the contract
        let data_contract = register_contract_from_bytes(
            &mut platform,
            &platform_state,
            contract_bytes,
            IdentityTestInfo::Given {
                identity: &identity,
                signer: &signer,
                public_key: &critical_key,
                identity_nonce: 1,
            },
            platform_version,
        )
        .await;

        let secp = Secp256k1::new();

        let mut rng = StdRng::seed_from_u64(1292);

        let new_key_pair = Keypair::new(&secp, &mut rng);

        let mut new_key = IdentityPublicKeyInCreationV0 {
            id: 2,
            purpose: Purpose::ENCRYPTION,
            security_level: SecurityLevel::MEDIUM,
            key_type: ECDSA_SECP256K1,
            read_only: false,
            data: new_key_pair.public_key().serialize().to_vec().into(),
            signature: Default::default(),
            contract_bounds: Some(ContractBounds::SingleContract {
                id: data_contract.id(),
            }),
        };

        let update_transition: IdentityUpdateTransition = IdentityUpdateTransitionV0 {
            identity_id: identity.id(),
            revision: 1,
            nonce: 2, // Use nonce 2 since we used 1 for contract creation
            add_public_keys: vec![IdentityPublicKeyInCreation::V0(new_key.clone())],
            disable_public_keys: vec![],
            user_fee_increase: 0,
            signature_public_key_id: master_key.id(),
            signature: Default::default(),
        }
        .into();

        let update_transition: StateTransition = update_transition.into();

        let signable_bytes = update_transition
            .signable_bytes()
            .expect("expected signable bytes");

        // Sign the new key with its own private key
        let secret = new_key_pair.secret_key();
        let signature =
            signer::sign(&signable_bytes, &secret.secret_bytes()).expect("expected to sign");

        new_key.signature = signature.to_vec().into();

        // Create the transition again with the signed key
        let update_transition: IdentityUpdateTransition = IdentityUpdateTransitionV0 {
            identity_id: identity.id(),
            revision: 1,
            nonce: 2,
            add_public_keys: vec![IdentityPublicKeyInCreation::V0(new_key)],
            disable_public_keys: vec![],
            user_fee_increase: 0,
            signature_public_key_id: master_key.id(),
            signature: Default::default(),
        }
        .into();

        let mut update_transition: StateTransition = update_transition.into();

        // Sign the transition with the master key
        update_transition.set_signature(
            signer
                .sign(&master_key, signable_bytes.as_slice())
                .await
                .expect("expected to sign"),
        );

        let update_transition_bytes = update_transition
            .serialize_to_bytes()
            .expect("expected to serialize");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![update_transition_bytes.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                true,
                None,
            )
            .expect("expected to process state transition");

        // We expect success - contract bound keys are allowed
        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");

        // Verify the key was added
        use drive::drive::identity::key::fetch::{IdentityKeysRequest, KeyRequestType};

        let identity_keys_request = IdentityKeysRequest {
            identity_id: identity.id().to_buffer(),
            request_type: KeyRequestType::AllKeys,
            limit: None,
            offset: None,
        };

        let updated_partial_identity = platform
            .drive
            .fetch_identity_keys_as_partial_identity(identity_keys_request, None, platform_version)
            .expect("expected to fetch identity")
            .expect("expected identity to exist");

        assert_eq!(updated_partial_identity.loaded_public_keys.len(), 3); // Original 2 + new contract bound key

        let contract_bound_key = updated_partial_identity
            .loaded_public_keys
            .get(&2)
            .expect("expected to find key with id 2");

        assert_eq!(
            contract_bound_key.contract_bounds(),
            Some(&ContractBounds::SingleContract {
                id: data_contract.id()
            })
        );
    }

    #[tokio::test]
    async fn test_identity_update_adding_contract_bound_key_multiple_reference_to_latest() {
        use crate::execution::validation::state_transition::tests::{
            register_contract_from_bytes, IdentityTestInfo,
        };

        let platform_config = PlatformConfig {
            testing_configs: PlatformTestConfig {
                disable_instant_lock_signature_verification: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let platform_version = PlatformVersion::latest();

        let mut platform = TestPlatformBuilder::new()
            .with_config(platform_config)
            .build_with_mock_rpc()
            .set_genesis_state();

        // {
        //   "$formatVersion": "1",
        //   "id": "5pkMhyeaFjJfVMkFhLtJdDp2ofx6iqt7i9k6ckkHBwbs",
        //   "config": {
        //     "$formatVersion": "1",
        //     "canBeDeleted": false,
        //     "readonly": false,
        //     "keepsHistory": false,
        //     "documentsKeepHistoryContractDefault": false,
        //     "documentsMutableContractDefault": true,
        //     "documentsCanBeDeletedContractDefault": true,
        //     "requiresIdentityEncryptionBoundedKey": 0,
        //     "requiresIdentityDecryptionBoundedKey": 0,
        //     "sizedIntegerTypes": true
        //   },
        //   "version": 1,
        //   "ownerId": "DicUmimv71VqxNBzZHXb887RgssSEjjx7DyLfxrt8q1X",
        //   "schemaDefs": null,
        //   "documentSchemas": {
        //     "preorder": {
        //       "documentsMutable": false,
        //       "canBeDeleted": true,
        //       "type": "object",
        //       "indices": [
        //         {
        //           "name": "saltedHash",
        //           "properties": [
        //             {
        //               "saltedDomainHash": "asc"
        //             }
        //           ],
        //           "unique": true
        //         }
        //       ],
        //       "properties": {
        //         "saltedDomainHash": {
        //           "type": "array",
        //           "byteArray": true,
        //           "minItems": 32,
        //           "maxItems": 32,
        //           "position": 0,
        //           "description": "Double sha-256 of the concatenation of a 32 byte random salt and a normalized domain name"
        //         }
        //       },
        //       "required": [
        //         "saltedDomainHash"
        //       ],
        //       "additionalProperties": false,
        //       "$comment": "Preorder documents are immutable: modification and deletion are restricted"
        //     }
        //   },
        //   "createdAt": 1749816974718,
        //   "updatedAt": null,
        //   "createdAtBlockHeight": 159130,
        //   "updatedAtBlockHeight": null,
        //   "createdAtEpoch": 7906,
        //   "updatedAtEpoch": null,
        //   "groups": {},
        //   "tokens": {},
        //   "keywords": [],
        //   "description": null
        // }
        let contract_bytes = hex::decode("0147aa11d517710d509edaf84bb54902394dcb8f6cc68775138d1cdd8334600d2e01000000000101010001000101bcf52c1c5d57d2e21530c5d03ef4c6e7b39a91da7c444fdb17e0a7746b6285860001087072656f7264657216081210646f63756d656e74734d757461626c651300120c63616e426544656c65746564130012047479706512066f626a6563741207696e64696365731501160312046e616d65120a73616c74656448617368120a70726f7065727469657315011601121073616c746564446f6d61696e4861736812036173631206756e697175651301120a70726f706572746965731601121073616c746564446f6d61696e486173681606120474797065120561727261791209627974654172726179130112086d696e4974656d73022012086d61784974656d7302201208706f736974696f6e0200120b6465736372697074696f6e1259446f75626c65207368612d323536206f662074686520636f6e636174656e6174696f6e206f66206120333220627974652072616e646f6d2073616c7420616e642061206e6f726d616c697a656420646f6d61696e206e616d65120872657175697265641501121073616c746564446f6d61696e4861736812146164646974696f6e616c50726f706572746965731300120824636f6d6d656e74124a5072656f7264657220646f63756d656e74732061726520696d6d757461626c653a206d6f64696669636174696f6e20616e642064656c6574696f6e20617265207265737472696374656401fd0000019769381d7e0001fc00026d9a0001fb1ee20000000000").expect("expected to decode contract bytes");

        let (identity, signer, critical_key, master_key) =
            setup_identity_return_master_key(&mut platform, 958, dash_to_credits!(5.0));

        let platform_state = platform.state.load();

        // Same contract, but opting bound ENCRYPTION and DECRYPTION keys in with
        // `MultipleReferenceToLatest` (2), the mode the DashPay contract and every
        // contract published from the JS SDK use, instead of `Unique` (0).
        let contract_bytes = {
            use dpp::data_contract::config::v0::DataContractConfigSettersV0;
            use dpp::data_contract::storage_requirements::keys_for_document_type::StorageKeyRequirements;
            use dpp::serialization::{
                PlatformDeserializableWithPotentialValidationFromVersionedStructureUntrusted,
                PlatformSerializableWithPlatformVersion,
            };
            let mut contract = dpp::data_contract::DataContract::versioned_deserialize_untrusted(
                &contract_bytes,
                false,
                platform_version,
            )
            .expect("expected to deserialize data contract");
            contract
                .config_mut()
                .set_requires_identity_encryption_bounded_key(Some(
                    StorageKeyRequirements::MultipleReferenceToLatest,
                ));
            contract
                .config_mut()
                .set_requires_identity_decryption_bounded_key(Some(
                    StorageKeyRequirements::MultipleReferenceToLatest,
                ));
            contract
                .serialize_to_bytes_with_platform_version(platform_version)
                .expect("expected to serialize data contract")
        };

        // Register the contract
        let data_contract = register_contract_from_bytes(
            &mut platform,
            &platform_state,
            contract_bytes,
            IdentityTestInfo::Given {
                identity: &identity,
                signer: &signer,
                public_key: &critical_key,
                identity_nonce: 1,
            },
            platform_version,
        )
        .await;

        let secp = Secp256k1::new();

        let mut rng = StdRng::seed_from_u64(1292);

        let new_key_pair = Keypair::new(&secp, &mut rng);

        let mut new_key = IdentityPublicKeyInCreationV0 {
            id: 2,
            purpose: Purpose::ENCRYPTION,
            security_level: SecurityLevel::MEDIUM,
            key_type: ECDSA_SECP256K1,
            read_only: false,
            data: new_key_pair.public_key().serialize().to_vec().into(),
            signature: Default::default(),
            contract_bounds: Some(ContractBounds::SingleContract {
                id: data_contract.id(),
            }),
        };

        let update_transition: IdentityUpdateTransition = IdentityUpdateTransitionV0 {
            identity_id: identity.id(),
            revision: 1,
            nonce: 2, // Use nonce 2 since we used 1 for contract creation
            add_public_keys: vec![IdentityPublicKeyInCreation::V0(new_key.clone())],
            disable_public_keys: vec![],
            user_fee_increase: 0,
            signature_public_key_id: master_key.id(),
            signature: Default::default(),
        }
        .into();

        let update_transition: StateTransition = update_transition.into();

        let signable_bytes = update_transition
            .signable_bytes()
            .expect("expected signable bytes");

        // Sign the new key with its own private key
        let secret = new_key_pair.secret_key();
        let signature =
            signer::sign(&signable_bytes, &secret.secret_bytes()).expect("expected to sign");

        new_key.signature = signature.to_vec().into();

        // Create the transition again with the signed key
        let update_transition: IdentityUpdateTransition = IdentityUpdateTransitionV0 {
            identity_id: identity.id(),
            revision: 1,
            nonce: 2,
            add_public_keys: vec![IdentityPublicKeyInCreation::V0(new_key)],
            disable_public_keys: vec![],
            user_fee_increase: 0,
            signature_public_key_id: master_key.id(),
            signature: Default::default(),
        }
        .into();

        let mut update_transition: StateTransition = update_transition.into();

        // Sign the transition with the master key
        update_transition.set_signature(
            signer
                .sign(&master_key, signable_bytes.as_slice())
                .await
                .expect("expected to sign"),
        );

        let update_transition_bytes = update_transition
            .serialize_to_bytes()
            .expect("expected to serialize");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![update_transition_bytes.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                true,
                None,
            )
            .expect("expected to process state transition");

        // We expect success - contract bound keys are allowed
        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");

        // Verify the key was added
        use drive::drive::identity::key::fetch::{IdentityKeysRequest, KeyRequestType};

        let identity_keys_request = IdentityKeysRequest {
            identity_id: identity.id().to_buffer(),
            request_type: KeyRequestType::AllKeys,
            limit: None,
            offset: None,
        };

        let updated_partial_identity = platform
            .drive
            .fetch_identity_keys_as_partial_identity(identity_keys_request, None, platform_version)
            .expect("expected to fetch identity")
            .expect("expected identity to exist");

        assert_eq!(updated_partial_identity.loaded_public_keys.len(), 3); // Original 2 + new contract bound key

        let contract_bound_key = updated_partial_identity
            .loaded_public_keys
            .get(&2)
            .expect("expected to find key with id 2");

        assert_eq!(
            contract_bound_key.contract_bounds(),
            Some(&ContractBounds::SingleContract {
                id: data_contract.id()
            })
        );
    }

    #[tokio::test]
    async fn test_identity_update_adding_contract_bound_key_on_document_level() {
        use crate::execution::validation::state_transition::tests::{
            register_contract_from_bytes, IdentityTestInfo,
        };

        let platform_config = PlatformConfig {
            testing_configs: PlatformTestConfig {
                disable_instant_lock_signature_verification: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let platform_version = PlatformVersion::latest();

        let mut platform = TestPlatformBuilder::new()
            .with_config(platform_config)
            .build_with_mock_rpc()
            .set_genesis_state();

        // {
        //   "$formatVersion": "1",
        //   "id": "8m7H1EScryPTeJzck2qbSckTbEjCg2vu7PRth2LCwsHo",
        //   "config": {
        //     "$formatVersion": "1",
        //     "canBeDeleted": false,
        //     "readonly": false,
        //     "keepsHistory": false,
        //     "documentsKeepHistoryContractDefault": false,
        //     "documentsMutableContractDefault": true,
        //     "documentsCanBeDeletedContractDefault": true,
        //     "requiresIdentityEncryptionBoundedKey": null,
        //     "requiresIdentityDecryptionBoundedKey": null,
        //     "sizedIntegerTypes": true
        //   },
        //   "version": 1,
        //   "ownerId": "Eh5SUY1wHovQn5xoW9gEpA3ABfHSmq2bg9pcSiybkBWW",
        //   "schemaDefs": null,
        //   "documentSchemas": {
        //     "liquidityPool": {
        //       "type": "object",
        //       "requiresIdentityEncryptionBoundedKey": 0,
        //       "requiresIdentityDecryptionBoundedKey": 0,
        //       "properties": {
        //         "tokenA": {
        //           "position": 0,
        //           "type": "string",
        //           "description": "The symbol of the first token in the pool.",
        //           "maxLength": 10
        //         },
        //         "tokenB": {
        //           "position": 1,
        //           "type": "string",
        //           "description": "The symbol of the second token in the pool.",
        //           "maxLength": 10
        //         },
        //         "reserveA": {
        //           "position": 2,
        //           "type": "number",
        //           "description": "The amount of token A in the pool.",
        //           "minimum": 0
        //         },
        //         "reserveB": {
        //           "position": 3,
        //           "type": "number",
        //           "description": "The amount of token B in the pool.",
        //           "minimum": 0
        //         },
        //         "liquidityTokens": {
        //           "position": 4,
        //           "type": "number",
        //           "description": "The total liquidity tokens issued for this pool.",
        //           "minimum": 0
        //         }
        //       },
        //       "indices": [
        //         {
        //           "name": "tokenPair",
        //           "properties": [
        //             {
        //               "tokenA": "asc"
        //             },
        //             {
        //               "tokenB": "asc"
        //             }
        //           ]
        //         }
        //       ],
        //       "required": [
        //         "tokenA",
        //         "tokenB",
        //         "reserveA",
        //         "reserveB",
        //         "liquidityTokens"
        //       ],
        //       "additionalProperties": false,
        //       "description": "Represents a liquidity pool for token trading."
        //     },
        //     "swapTransaction": {
        //       "type": "object",
        //       "properties": {
        //         "fromToken": {
        //           "position": 0,
        //           "type": "string",
        //           "description": "The symbol of the token being swapped from.",
        //           "maxLength": 10
        //         },
        //         "toToken": {
        //           "position": 1,
        //           "type": "string",
        //           "description": "The symbol of the token being swapped to.",
        //           "maxLength": 10
        //         },
        //         "amountIn": {
        //           "position": 2,
        //           "type": "number",
        //           "description": "The amount of the fromToken being swapped.",
        //           "minimum": 0
        //         },
        //         "amountOut": {
        //           "position": 3,
        //           "type": "number",
        //           "description": "The amount of the toToken received.",
        //           "minimum": 0
        //         },
        //         "timestamp": {
        //           "position": 4,
        //           "type": "integer",
        //           "description": "The timestamp of the transaction.",
        //           "minimum": 0
        //         }
        //       },
        //       "indices": [
        //         {
        //           "name": "fromToken",
        //           "properties": [
        //             {
        //               "fromToken": "asc"
        //             }
        //           ]
        //         },
        //         {
        //           "name": "toToken",
        //           "properties": [
        //             {
        //               "toToken": "asc"
        //             }
        //           ]
        //         },
        //         {
        //           "name": "timestamp",
        //           "properties": [
        //             {
        //               "timestamp": "asc"
        //             }
        //           ]
        //         }
        //       ],
        //       "required": [
        //         "fromToken",
        //         "toToken",
        //         "amountIn",
        //         "amountOut",
        //         "timestamp"
        //       ],
        //       "additionalProperties": false,
        //       "description": "Represents a swap transaction between two tokens."
        //     },
        //     "token": {
        //       "type": "object",
        //       "properties": {
        //         "symbol": {
        //           "position": 0,
        //           "type": "string",
        //           "description": "The symbol of the token, e.g., 'DASH'.",
        //           "maxLength": 10
        //         },
        //         "name": {
        //           "position": 1,
        //           "type": "string",
        //           "description": "The full name of the token.",
        //           "maxLength": 63
        //         },
        //         "decimals": {
        //           "position": 2,
        //           "type": "integer",
        //           "description": "The number of decimal places the token uses.",
        //           "minimum": 0,
        //           "maximum": 18
        //         },
        //         "totalSupply": {
        //           "position": 3,
        //           "type": "number",
        //           "description": "The total supply of the token.",
        //           "minimum": 0
        //         }
        //       },
        //       "indices": [
        //         {
        //           "name": "symbol",
        //           "properties": [
        //             {
        //               "symbol": "asc"
        //             }
        //           ]
        //         }
        //       ],
        //       "required": [
        //         "symbol",
        //         "name",
        //         "decimals",
        //         "totalSupply"
        //       ],
        //       "additionalProperties": false,
        //       "description": "Represents a token available for trading on the platform."
        //     },
        //     "userProfile": {
        //       "type": "object",
        //       "properties": {
        //         "username": {
        //           "position": 0,
        //           "type": "string",
        //           "description": "The unique username of the user.",
        //           "maxLength": 63
        //         },
        //         "walletAddress": {
        //           "position": 1,
        //           "type": "string",
        //           "description": "The Dash wallet address of the user.",
        //           "maxLength": 63
        //         },
        //         "createdAt": {
        //           "position": 2,
        //           "type": "integer",
        //           "description": "The timestamp when the user profile was created.",
        //           "minimum": 0
        //         }
        //       },
        //       "indices": [
        //         {
        //           "name": "username",
        //           "properties": [
        //             {
        //               "username": "asc"
        //             }
        //           ]
        //         },
        //         {
        //           "name": "walletAddress",
        //           "properties": [
        //             {
        //               "walletAddress": "asc"
        //             }
        //           ]
        //         }
        //       ],
        //       "required": [
        //         "username",
        //         "walletAddress",
        //         "createdAt"
        //       ],
        //       "additionalProperties": false,
        //       "description": "Represents a user profile within the application."
        //     }
        //   },
        //   "createdAt": 1750216267336,
        //   "updatedAt": null,
        //   "createdAtBlockHeight": 161570,
        //   "updatedAtBlockHeight": null,
        //   "createdAtEpoch": 8017,
        //   "updatedAtEpoch": null,
        //   "groups": {},
        //   "tokens": {},
        //   "keywords": [],
        //   "description": null
        // }
        let contract_bytes = hex::decode("01734e75d6522ecc1161b1594bc8706c2fe32450c26a6680612c51e1f4806d39160100000000010100000101cb6c2e8d89ad3ec7bbfcef8b02eb85b8f27ef7ac847c3d461c898d6929fb6daf00040d6c6971756964697479506f6f6c160812047479706512066f626a656374122472657175697265734964656e74697479456e6372797074696f6e426f756e6465644b65790200122472657175697265734964656e7469747944656372797074696f6e426f756e6465644b65790200120a70726f7065727469657316051206746f6b656e4116041208706f736974696f6e02001204747970651206737472696e67120b6465736372697074696f6e122a5468652073796d626f6c206f662074686520666972737420746f6b656e20696e2074686520706f6f6c2e12096d61784c656e677468020a1206746f6b656e4216041208706f736974696f6e02011204747970651206737472696e67120b6465736372697074696f6e122b5468652073796d626f6c206f6620746865207365636f6e6420746f6b656e20696e2074686520706f6f6c2e12096d61784c656e677468020a1208726573657276654116041208706f736974696f6e020212047479706512066e756d626572120b6465736372697074696f6e122254686520616d6f756e74206f6620746f6b656e204120696e2074686520706f6f6c2e12076d696e696d756d02001208726573657276654216041208706f736974696f6e020312047479706512066e756d626572120b6465736372697074696f6e122254686520616d6f756e74206f6620746f6b656e204220696e2074686520706f6f6c2e12076d696e696d756d0200120f6c6971756964697479546f6b656e7316041208706f736974696f6e020412047479706512066e756d626572120b6465736372697074696f6e123054686520746f74616c206c697175696469747920746f6b656e732069737375656420666f72207468697320706f6f6c2e12076d696e696d756d02001207696e64696365731501160212046e616d651209746f6b656e50616972120a70726f70657274696573150216011206746f6b656e41120361736316011206746f6b656e4212036173631208726571756972656415051206746f6b656e411206746f6b656e421208726573657276654112087265736572766542120f6c6971756964697479546f6b656e7312146164646974696f6e616c50726f706572746965731300120b6465736372697074696f6e122e526570726573656e74732061206c697175696469747920706f6f6c20666f7220746f6b656e2074726164696e672e0f737761705472616e73616374696f6e160612047479706512066f626a656374120a70726f706572746965731605120966726f6d546f6b656e16041208706f736974696f6e02001204747970651206737472696e67120b6465736372697074696f6e122b5468652073796d626f6c206f662074686520746f6b656e206265696e6720737761707065642066726f6d2e12096d61784c656e677468020a1207746f546f6b656e16041208706f736974696f6e02011204747970651206737472696e67120b6465736372697074696f6e12295468652073796d626f6c206f662074686520746f6b656e206265696e67207377617070656420746f2e12096d61784c656e677468020a1208616d6f756e74496e16041208706f736974696f6e020212047479706512066e756d626572120b6465736372697074696f6e122a54686520616d6f756e74206f66207468652066726f6d546f6b656e206265696e6720737761707065642e12076d696e696d756d02001209616d6f756e744f757416041208706f736974696f6e020312047479706512066e756d626572120b6465736372697074696f6e122354686520616d6f756e74206f662074686520746f546f6b656e2072656365697665642e12076d696e696d756d0200120974696d657374616d7016041208706f736974696f6e02041204747970651207696e7465676572120b6465736372697074696f6e12215468652074696d657374616d70206f6620746865207472616e73616374696f6e2e12076d696e696d756d02001207696e64696365731503160212046e616d65120966726f6d546f6b656e120a70726f7065727469657315011601120966726f6d546f6b656e1203617363160212046e616d651207746f546f6b656e120a70726f70657274696573150116011207746f546f6b656e1203617363160212046e616d65120974696d657374616d70120a70726f7065727469657315011601120974696d657374616d701203617363120872657175697265641505120966726f6d546f6b656e1207746f546f6b656e1208616d6f756e74496e1209616d6f756e744f7574120974696d657374616d7012146164646974696f6e616c50726f706572746965731300120b6465736372697074696f6e1231526570726573656e747320612073776170207472616e73616374696f6e206265747765656e2074776f20746f6b656e732e05746f6b656e160612047479706512066f626a656374120a70726f706572746965731604120673796d626f6c16041208706f736974696f6e02001204747970651206737472696e67120b6465736372697074696f6e12265468652073796d626f6c206f662074686520746f6b656e2c20652e672e2c202744415348272e12096d61784c656e677468020a12046e616d6516041208706f736974696f6e02011204747970651206737472696e67120b6465736372697074696f6e121b5468652066756c6c206e616d65206f662074686520746f6b656e2e12096d61784c656e677468023f1208646563696d616c7316051208706f736974696f6e02021204747970651207696e7465676572120b6465736372697074696f6e122c546865206e756d626572206f6620646563696d616c20706c616365732074686520746f6b656e20757365732e12076d696e696d756d020012076d6178696d756d0212120b746f74616c537570706c7916041208706f736974696f6e020312047479706512066e756d626572120b6465736372697074696f6e121e54686520746f74616c20737570706c79206f662074686520746f6b656e2e12076d696e696d756d02001207696e64696365731501160212046e616d65120673796d626f6c120a70726f7065727469657315011601120673796d626f6c1203617363120872657175697265641504120673796d626f6c12046e616d651208646563696d616c73120b746f74616c537570706c7912146164646974696f6e616c50726f706572746965731300120b6465736372697074696f6e1239526570726573656e7473206120746f6b656e20617661696c61626c6520666f722074726164696e67206f6e2074686520706c6174666f726d2e0b7573657250726f66696c65160612047479706512066f626a656374120a70726f7065727469657316031208757365726e616d6516041208706f736974696f6e02001204747970651206737472696e67120b6465736372697074696f6e122054686520756e6971756520757365726e616d65206f662074686520757365722e12096d61784c656e677468023f120d77616c6c65744164647265737316041208706f736974696f6e02011204747970651206737472696e67120b6465736372697074696f6e122454686520446173682077616c6c65742061646472657373206f662074686520757365722e12096d61784c656e677468023f120963726561746564417416041208706f736974696f6e02021204747970651207696e7465676572120b6465736372697074696f6e12305468652074696d657374616d70207768656e2074686520757365722070726f66696c652077617320637265617465642e12076d696e696d756d02001207696e64696365731502160212046e616d651208757365726e616d65120a70726f70657274696573150116011208757365726e616d651203617363160212046e616d65120d77616c6c657441646472657373120a70726f7065727469657315011601120d77616c6c65744164647265737312036173631208726571756972656415031208757365726e616d65120d77616c6c657441646472657373120963726561746564417412146164646974696f6e616c50726f706572746965731300120b6465736372697074696f6e1231526570726573656e7473206120757365722070726f66696c652077697468696e20746865206170706c69636174696f6e2e01fd000001978104d6480001fc000277220001fb1f510000000000").expect("expected to decode contract bytes");

        let (identity, signer, critical_key, master_key) =
            setup_identity_return_master_key(&mut platform, 959, dash_to_credits!(5.0));

        let platform_state = platform.state.load();

        // Register the contract
        let data_contract = register_contract_from_bytes(
            &mut platform,
            &platform_state,
            contract_bytes,
            IdentityTestInfo::Given {
                identity: &identity,
                signer: &signer,
                public_key: &critical_key,
                identity_nonce: 1,
            },
            platform_version,
        )
        .await;

        let secp = Secp256k1::new();

        let mut rng = StdRng::seed_from_u64(1292);

        let new_key_pair = Keypair::new(&secp, &mut rng);

        let mut new_key = IdentityPublicKeyInCreationV0 {
            id: 2,
            purpose: Purpose::ENCRYPTION,
            security_level: SecurityLevel::MEDIUM,
            key_type: ECDSA_SECP256K1,
            read_only: false,
            data: new_key_pair.public_key().serialize().to_vec().into(),
            signature: Default::default(),
            contract_bounds: Some(ContractBounds::SingleContractDocumentType {
                id: data_contract.id(),
                document_type_name: "liquidityPool".to_string(),
            }),
        };

        let update_transition: IdentityUpdateTransition = IdentityUpdateTransitionV0 {
            identity_id: identity.id(),
            revision: 1,
            nonce: 2, // Use nonce 2 since we used 1 for contract creation
            add_public_keys: vec![IdentityPublicKeyInCreation::V0(new_key.clone())],
            disable_public_keys: vec![],
            user_fee_increase: 0,
            signature_public_key_id: master_key.id(),
            signature: Default::default(),
        }
        .into();

        let update_transition: StateTransition = update_transition.into();

        let signable_bytes = update_transition
            .signable_bytes()
            .expect("expected signable bytes");

        // Sign the new key with its own private key
        let secret = new_key_pair.secret_key();
        let signature =
            signer::sign(&signable_bytes, &secret.secret_bytes()).expect("expected to sign");

        new_key.signature = signature.to_vec().into();

        // Create the transition again with the signed key
        let update_transition: IdentityUpdateTransition = IdentityUpdateTransitionV0 {
            identity_id: identity.id(),
            revision: 1,
            nonce: 2,
            add_public_keys: vec![IdentityPublicKeyInCreation::V0(new_key)],
            disable_public_keys: vec![],
            user_fee_increase: 0,
            signature_public_key_id: master_key.id(),
            signature: Default::default(),
        }
        .into();

        let mut update_transition: StateTransition = update_transition.into();

        // Sign the transition with the master key
        update_transition.set_signature(
            signer
                .sign(&master_key, signable_bytes.as_slice())
                .await
                .expect("expected to sign"),
        );

        let update_transition_bytes = update_transition
            .serialize_to_bytes()
            .expect("expected to serialize");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![update_transition_bytes.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                true,
                None,
            )
            .expect("expected to process state transition");

        // We expect success - contract bound keys are allowed
        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");

        // Verify the key was added
        use drive::drive::identity::key::fetch::{IdentityKeysRequest, KeyRequestType};

        let identity_keys_request = IdentityKeysRequest {
            identity_id: identity.id().to_buffer(),
            request_type: KeyRequestType::AllKeys,
            limit: None,
            offset: None,
        };

        let updated_partial_identity = platform
            .drive
            .fetch_identity_keys_as_partial_identity(identity_keys_request, None, platform_version)
            .expect("expected to fetch identity")
            .expect("expected identity to exist");

        assert_eq!(updated_partial_identity.loaded_public_keys.len(), 3); // Original 2 + new contract bound key

        let contract_bound_key = updated_partial_identity
            .loaded_public_keys
            .get(&2)
            .expect("expected to find key with id 2");

        assert_eq!(
            contract_bound_key.contract_bounds(),
            Some(&ContractBounds::SingleContractDocumentType {
                id: data_contract.id(),
                document_type_name: "liquidityPool".to_string(),
            })
        );
    }

    #[tokio::test]
    async fn test_identity_update_empty_transition_rejected() {
        let platform_version = PlatformVersion::latest();
        let platform_config = PlatformConfig {
            testing_configs: PlatformTestConfig {
                disable_instant_lock_signature_verification: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut platform = TestPlatformBuilder::new()
            .with_config(platform_config)
            .build_with_mock_rpc()
            .set_genesis_state();

        let (identity, signer, _, key) =
            setup_identity_return_master_key(&mut platform, 958, dash_to_credits!(0.1));

        let platform_state = platform.state.load();

        // Create transition with neither keys to add nor keys to disable
        let update_transition: IdentityUpdateTransition = IdentityUpdateTransitionV0 {
            identity_id: identity.id(),
            revision: 1,
            nonce: 1,
            add_public_keys: vec![],
            disable_public_keys: vec![],
            user_fee_increase: 0,
            signature_public_key_id: key.id(),
            signature: Default::default(),
        }
        .into();

        let mut update_transition: StateTransition = update_transition.into();
        let data = update_transition
            .signable_bytes()
            .expect("expected signable bytes");
        update_transition.set_signature(
            signer
                .sign(&key, data.as_slice())
                .await
                .expect("expected to sign"),
        );

        let transition_bytes = update_transition
            .serialize_to_bytes()
            .expect("expected to serialize");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![transition_bytes],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                true,
                None,
            )
            .expect("expected to process state transition");

        // Empty update should be rejected with BasicError
        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::UnpaidConsensusError(
                ConsensusError::BasicError(_)
            )]
        );
    }

    #[tokio::test]
    async fn test_identity_update_too_many_keys_to_disable() {
        let platform_version = PlatformVersion::latest();
        let platform_config = PlatformConfig {
            testing_configs: PlatformTestConfig {
                disable_instant_lock_signature_verification: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut platform = TestPlatformBuilder::new()
            .with_config(platform_config)
            .build_with_mock_rpc()
            .set_genesis_state();

        let (identity, signer, _, key) =
            setup_identity_return_master_key(&mut platform, 958, dash_to_credits!(0.1));

        let platform_state = platform.state.load();

        // Try to disable more than MAX_KEYS_TO_DISABLE (10) keys
        let disable_keys: Vec<u32> = (1..=11).collect();

        let update_transition: IdentityUpdateTransition = IdentityUpdateTransitionV0 {
            identity_id: identity.id(),
            revision: 1,
            nonce: 1,
            add_public_keys: vec![],
            disable_public_keys: disable_keys,
            user_fee_increase: 0,
            signature_public_key_id: key.id(),
            signature: Default::default(),
        }
        .into();

        let mut update_transition: StateTransition = update_transition.into();
        let data = update_transition
            .signable_bytes()
            .expect("expected signable bytes");
        update_transition.set_signature(
            signer
                .sign(&key, data.as_slice())
                .await
                .expect("expected to sign"),
        );

        let transition_bytes = update_transition
            .serialize_to_bytes()
            .expect("expected to serialize");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![transition_bytes],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                true,
                None,
            )
            .expect("expected to process state transition");

        // Should be rejected for exceeding max keys to disable
        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::UnpaidConsensusError(
                ConsensusError::StateError(_)
            )]
        );
    }

    #[tokio::test]
    async fn test_identity_update_duplicate_key_ids_to_disable() {
        let platform_version = PlatformVersion::latest();
        let platform_config = PlatformConfig {
            testing_configs: PlatformTestConfig {
                disable_instant_lock_signature_verification: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut platform = TestPlatformBuilder::new()
            .with_config(platform_config)
            .build_with_mock_rpc()
            .set_genesis_state();

        let (identity, signer, _, key) =
            setup_identity_return_master_key(&mut platform, 958, dash_to_credits!(0.1));

        let platform_state = platform.state.load();

        // Duplicate key ID 1 in the disable list
        let update_transition: IdentityUpdateTransition = IdentityUpdateTransitionV0 {
            identity_id: identity.id(),
            revision: 1,
            nonce: 1,
            add_public_keys: vec![],
            disable_public_keys: vec![1, 1],
            user_fee_increase: 0,
            signature_public_key_id: key.id(),
            signature: Default::default(),
        }
        .into();

        let mut update_transition: StateTransition = update_transition.into();
        let data = update_transition
            .signable_bytes()
            .expect("expected signable bytes");
        update_transition.set_signature(
            signer
                .sign(&key, data.as_slice())
                .await
                .expect("expected to sign"),
        );

        let transition_bytes = update_transition
            .serialize_to_bytes()
            .expect("expected to serialize");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![transition_bytes],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                true,
                None,
            )
            .expect("expected to process state transition");

        // Duplicate key IDs should be rejected
        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::UnpaidConsensusError(
                ConsensusError::BasicError(_)
            )]
        );
    }

    #[tokio::test]
    async fn test_identity_update_disabling_key_id_also_being_added() {
        let platform_version = PlatformVersion::latest();
        let platform_config = PlatformConfig {
            testing_configs: PlatformTestConfig {
                disable_instant_lock_signature_verification: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut platform = TestPlatformBuilder::new()
            .with_config(platform_config)
            .build_with_mock_rpc()
            .set_genesis_state();

        let (identity, signer, _, key) =
            setup_identity_return_master_key(&mut platform, 958, dash_to_credits!(0.1));

        let platform_state = platform.state.load();

        let secp = Secp256k1::new();
        let mut rng = StdRng::seed_from_u64(292);
        let new_key_pair = Keypair::new(&secp, &mut rng);

        // Add a key with id 2 and also disable key id 2 in the same transition
        let new_key = IdentityPublicKeyInCreationV0 {
            id: 2,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            key_type: ECDSA_SECP256K1,
            read_only: false,
            data: new_key_pair.public_key().serialize().to_vec().into(),
            signature: Default::default(),
            contract_bounds: None,
        };

        let update_transition: IdentityUpdateTransition = IdentityUpdateTransitionV0 {
            identity_id: identity.id(),
            revision: 1,
            nonce: 1,
            add_public_keys: vec![IdentityPublicKeyInCreation::V0(new_key)],
            disable_public_keys: vec![2], // same id as the key being added
            user_fee_increase: 0,
            signature_public_key_id: key.id(),
            signature: Default::default(),
        }
        .into();

        let mut update_transition: StateTransition = update_transition.into();
        let data = update_transition
            .signable_bytes()
            .expect("expected signable bytes");
        update_transition.set_signature(
            signer
                .sign(&key, data.as_slice())
                .await
                .expect("expected to sign"),
        );

        let transition_bytes = update_transition
            .serialize_to_bytes()
            .expect("expected to serialize");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![transition_bytes],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                true,
                None,
            )
            .expect("expected to process state transition");

        // Should reject because disabling a key id that's also being added
        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::UnpaidConsensusError(
                ConsensusError::BasicError(_)
            )]
        );
    }

    #[tokio::test]
    async fn test_identity_update_wrong_revision() {
        let platform_version = PlatformVersion::latest();
        let platform_config = PlatformConfig {
            testing_configs: PlatformTestConfig {
                disable_instant_lock_signature_verification: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut platform = TestPlatformBuilder::new()
            .with_config(platform_config)
            .build_with_mock_rpc()
            .set_genesis_state();

        let (identity, signer, _, key) =
            setup_identity_return_master_key(&mut platform, 958, dash_to_credits!(0.1));

        let platform_state = platform.state.load();

        // Use wrong revision (5 instead of 1)
        let update_transition: IdentityUpdateTransition = IdentityUpdateTransitionV0 {
            identity_id: identity.id(),
            revision: 5, // wrong -- should be 1 (current is 0)
            nonce: 1,
            add_public_keys: vec![],
            disable_public_keys: vec![1],
            user_fee_increase: 0,
            signature_public_key_id: key.id(),
            signature: Default::default(),
        }
        .into();

        let mut update_transition: StateTransition = update_transition.into();
        let data = update_transition
            .signable_bytes()
            .expect("expected signable bytes");
        update_transition.set_signature(
            signer
                .sign(&key, data.as_slice())
                .await
                .expect("expected to sign"),
        );

        let transition_bytes = update_transition
            .serialize_to_bytes()
            .expect("expected to serialize");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![transition_bytes],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                true,
                None,
            )
            .expect("expected to process state transition");

        // Wrong revision should be a paid consensus error (state error)
        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError { .. }]
        );
    }

    #[tokio::test]
    async fn test_identity_update_disabling_nonexistent_key() {
        let platform_version = PlatformVersion::latest();
        let platform_config = PlatformConfig {
            testing_configs: PlatformTestConfig {
                disable_instant_lock_signature_verification: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut platform = TestPlatformBuilder::new()
            .with_config(platform_config)
            .build_with_mock_rpc()
            .set_genesis_state();

        let (identity, signer, _, key) =
            setup_identity_return_master_key(&mut platform, 958, dash_to_credits!(0.1));

        let platform_state = platform.state.load();

        // Try to disable key id 99 which doesn't exist
        let update_transition: IdentityUpdateTransition = IdentityUpdateTransitionV0 {
            identity_id: identity.id(),
            revision: 1,
            nonce: 1,
            add_public_keys: vec![],
            disable_public_keys: vec![99],
            user_fee_increase: 0,
            signature_public_key_id: key.id(),
            signature: Default::default(),
        }
        .into();

        let mut update_transition: StateTransition = update_transition.into();
        let data = update_transition
            .signable_bytes()
            .expect("expected signable bytes");
        update_transition.set_signature(
            signer
                .sign(&key, data.as_slice())
                .await
                .expect("expected to sign"),
        );

        let transition_bytes = update_transition
            .serialize_to_bytes()
            .expect("expected to serialize");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![transition_bytes],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                true,
                None,
            )
            .expect("expected to process state transition");

        // Disabling nonexistent key should result in a paid consensus error
        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError { .. }]
        );
    }

    #[tokio::test]
    async fn test_identity_update_adding_key_with_existing_id() {
        let platform_version = PlatformVersion::latest();
        let platform_config = PlatformConfig {
            testing_configs: PlatformTestConfig {
                disable_instant_lock_signature_verification: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut platform = TestPlatformBuilder::new()
            .with_config(platform_config)
            .build_with_mock_rpc()
            .set_genesis_state();

        let (identity, signer, _, key) =
            setup_identity_return_master_key(&mut platform, 958, dash_to_credits!(0.1));

        let platform_state = platform.state.load();

        let secp = Secp256k1::new();
        let mut rng = StdRng::seed_from_u64(292);
        let new_key_pair = Keypair::new(&secp, &mut rng);

        let signable_transition: IdentityUpdateTransition = IdentityUpdateTransitionV0 {
            identity_id: identity.id(),
            revision: 1,
            nonce: 1,
            add_public_keys: vec![IdentityPublicKeyInCreation::V0(
                IdentityPublicKeyInCreationV0 {
                    id: 1, // key id 1 already exists on the identity
                    purpose: Purpose::AUTHENTICATION,
                    security_level: SecurityLevel::HIGH,
                    key_type: ECDSA_SECP256K1,
                    read_only: false,
                    data: new_key_pair.public_key().serialize().to_vec().into(),
                    signature: Default::default(),
                    contract_bounds: None,
                },
            )],
            disable_public_keys: vec![],
            user_fee_increase: 0,
            signature_public_key_id: key.id(),
            signature: Default::default(),
        }
        .into();

        let signable_st: StateTransition = signable_transition.into();
        let signable_bytes = signable_st
            .signable_bytes()
            .expect("expected signable bytes");

        // Sign the new key
        let secret = new_key_pair.secret_key();
        let key_sig =
            signer::sign(&signable_bytes, &secret.secret_bytes()).expect("expected to sign");

        let mut new_key = IdentityPublicKeyInCreationV0 {
            id: 1, // existing key ID
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            key_type: ECDSA_SECP256K1,
            read_only: false,
            data: new_key_pair.public_key().serialize().to_vec().into(),
            signature: Default::default(),
            contract_bounds: None,
        };
        new_key.signature = key_sig.to_vec().into();

        let update_transition: IdentityUpdateTransition = IdentityUpdateTransitionV0 {
            identity_id: identity.id(),
            revision: 1,
            nonce: 1,
            add_public_keys: vec![IdentityPublicKeyInCreation::V0(new_key)],
            disable_public_keys: vec![],
            user_fee_increase: 0,
            signature_public_key_id: key.id(),
            signature: Default::default(),
        }
        .into();

        let mut update_transition: StateTransition = update_transition.into();
        update_transition.set_signature(
            signer
                .sign(&key, signable_bytes.as_slice())
                .await
                .expect("expected to sign"),
        );

        let transition_bytes = update_transition
            .serialize_to_bytes()
            .expect("expected to serialize");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![transition_bytes],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                true,
                None,
            )
            .expect("expected to process state transition");

        // Adding key with an existing ID should result in a paid consensus error
        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError { .. }]
        );
    }
    mod key_limits {
        use super::*;
        use dpp::consensus::codes::ErrorWithCode;
        use dpp::identity::identity_public_key::accessors::v1::IdentityPublicKeyGettersV1;
        use dpp::identity::{IdentityPublicKey, TimestampMillis};
        use dpp::state_transition::public_key_in_creation::v1::IdentityPublicKeyInCreationV1;
        use drive::drive::identity::key::fetch::{
            IdentityKeysRequest, KeyIDIdentityPublicKeyPairBTreeMap,
        };

        const IDENTITY_PUBLIC_KEY_LIMITS_NOT_ALLOWED: u32 = 10536;
        const INVALID_IDENTITY_PUBLIC_KEY_BUDGET: u32 = 10537;
        const IDENTITY_PUBLIC_KEY_ALREADY_EXPIRED: u32 = 40219;

        const BLOCK_TIME_MS: TimestampMillis = 1_000_000;
        const NEW_KEY_ID: u32 = 2;

        struct AddedKey {
            purpose: Purpose,
            security_level: SecurityLevel,
            total_budget: Option<u64>,
            expires_at: Option<TimestampMillis>,
        }

        /// Processes an identity update, signed by the master key, that adds one version 1 key
        /// with the given limits, and gives back the platform to look at the outcome.
        async fn add_key(
            added_key: AddedKey,
        ) -> (
            crate::test::helpers::setup::TempPlatform<crate::rpc::core::MockCoreRPCLike>,
            Identifier,
            StateTransitionExecutionResult,
        ) {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();
            let (identity, signer, _, master_key) =
                setup_identity_return_master_key(&mut platform, 958, dash_to_credits!(0.1));
            let platform_state = platform.state.load();

            let secp = Secp256k1::new();
            let mut rng = StdRng::seed_from_u64(292);
            let new_key_pair = Keypair::new(&secp, &mut rng);
            let mut new_key = IdentityPublicKeyInCreationV1 {
                id: NEW_KEY_ID,
                purpose: added_key.purpose,
                security_level: added_key.security_level,
                key_type: ECDSA_SECP256K1,
                read_only: false,
                data: new_key_pair.public_key().serialize().to_vec().into(),
                contract_bounds: None,
                total_budget: added_key.total_budget,
                expires_at: added_key.expires_at,
                signature: Default::default(),
            };

            let transition_adding = |new_key: IdentityPublicKeyInCreationV1| -> StateTransition {
                let update_transition: IdentityUpdateTransition = IdentityUpdateTransitionV0 {
                    identity_id: identity.id(),
                    revision: 1,
                    nonce: 1,
                    add_public_keys: vec![new_key.into()],
                    disable_public_keys: vec![],
                    user_fee_increase: 0,
                    signature_public_key_id: master_key.id(),
                    signature: Default::default(),
                }
                .into();
                update_transition.into()
            };

            // The limits are part of the signable bytes: the new key and the master key both
            // sign over them.
            let signable_bytes = transition_adding(new_key.clone())
                .signable_bytes()
                .expect("expected signable bytes");
            new_key.signature =
                signer::sign(&signable_bytes, &new_key_pair.secret_key().secret_bytes())
                    .expect("expected to sign")
                    .to_vec()
                    .into();

            let mut update_transition = transition_adding(new_key);
            update_transition.set_signature(
                signer
                    .sign(&master_key, signable_bytes.as_slice())
                    .await
                    .expect("expected to sign"),
            );

            let transaction = platform.drive.grove.start_transaction();
            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![update_transition
                        .serialize_to_bytes()
                        .expect("expected to serialize")],
                    &platform_state,
                    &BlockInfo {
                        time_ms: BLOCK_TIME_MS,
                        ..Default::default()
                    },
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");
            let execution = processing_result.execution_results()[0].clone();
            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit");
            drop(platform_state);
            (platform, identity.id(), execution)
        }

        fn stored_key(
            platform: &crate::test::helpers::setup::TempPlatform<crate::rpc::core::MockCoreRPCLike>,
            identity_id: Identifier,
        ) -> Option<IdentityPublicKey> {
            platform
                .drive
                .fetch_identity_keys::<KeyIDIdentityPublicKeyPairBTreeMap>(
                    IdentityKeysRequest::new_specific_key_query(identity_id.as_bytes(), NEW_KEY_ID),
                    None,
                    PlatformVersion::latest(),
                )
                .expect("expected to fetch keys")
                .remove(&NEW_KEY_ID)
        }

        #[tokio::test]
        async fn should_register_a_key_with_a_budget_and_an_expiry() {
            let (platform, identity_id, execution) = add_key(AddedKey {
                purpose: Purpose::AUTHENTICATION,
                security_level: SecurityLevel::HIGH,
                total_budget: Some(5_000_000),
                expires_at: Some(BLOCK_TIME_MS + 1),
            })
            .await;
            assert_matches!(
                execution,
                StateTransitionExecutionResult::SuccessfulExecution { .. }
            );

            let key = stored_key(&platform, identity_id).expect("expected the new key");
            assert_eq!(key.total_budget(), Some(5_000_000));
            assert_eq!(key.expires_at(), Some(BLOCK_TIME_MS + 1));
            assert_eq!(
                platform
                    .drive
                    .fetch_identity_key_remaining_budget(
                        identity_id.to_buffer(),
                        NEW_KEY_ID,
                        None,
                        PlatformVersion::latest()
                    )
                    .expect("expected to fetch the remaining budget"),
                Some(5_000_000),
                "a new key has its whole budget left"
            );
        }

        #[tokio::test]
        async fn should_reject_paid_a_key_that_is_already_expired() {
            // Seconds instead of milliseconds is the usual way to get here.
            let (platform, identity_id, execution) = add_key(AddedKey {
                purpose: Purpose::AUTHENTICATION,
                security_level: SecurityLevel::HIGH,
                total_budget: None,
                expires_at: Some(BLOCK_TIME_MS),
            })
            .await;
            assert!(
                matches!(&execution, StateTransitionExecutionResult::PaidConsensusError { error, .. } if error.code() == IDENTITY_PUBLIC_KEY_ALREADY_EXPIRED),
                "{execution:?}"
            );
            assert_eq!(stored_key(&platform, identity_id), None);
        }

        #[tokio::test]
        async fn should_reject_limits_on_a_key_that_may_not_carry_them() {
            for (purpose, security_level) in [
                (Purpose::TRANSFER, SecurityLevel::CRITICAL),
                (Purpose::AUTHENTICATION, SecurityLevel::MASTER),
            ] {
                let (platform, identity_id, execution) = add_key(AddedKey {
                    purpose,
                    security_level,
                    total_budget: Some(5_000_000),
                    expires_at: None,
                })
                .await;
                assert!(
                    matches!(&execution, StateTransitionExecutionResult::UnpaidConsensusError(error) if error.code() == IDENTITY_PUBLIC_KEY_LIMITS_NOT_ALLOWED),
                    "{purpose:?} {security_level:?}: {execution:?}"
                );
                assert_eq!(stored_key(&platform, identity_id), None);
            }
        }

        #[tokio::test]
        async fn should_reject_a_budget_of_zero() {
            let (_, _, execution) = add_key(AddedKey {
                purpose: Purpose::AUTHENTICATION,
                security_level: SecurityLevel::HIGH,
                total_budget: Some(0),
                expires_at: None,
            })
            .await;
            assert!(
                matches!(&execution, StateTransitionExecutionResult::UnpaidConsensusError(error) if error.code() == INVALID_IDENTITY_PUBLIC_KEY_BUDGET),
                "{execution:?}"
            );
        }
    }
}
