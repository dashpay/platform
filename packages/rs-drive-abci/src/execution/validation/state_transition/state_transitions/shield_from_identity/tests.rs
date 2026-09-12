#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use crate::config::{PlatformConfig, PlatformTestConfig};
    use crate::execution::validation::state_transition::state_transitions::shielded_common::compute_platform_sighash;
    use crate::execution::validation::state_transition::state_transitions::test_helpers::{
        create_dummy_serialized_action, get_proving_key, process_transition,
        serialize_authorized_bundle_with_flags, setup_platform,
    };
    use crate::execution::validation::state_transition::state_transitions::tests::process_state_transitions;
    use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
    use assert_matches::assert_matches;
    use dpp::block::block_info::BlockInfo;
    use dpp::consensus::basic::BasicError;
    use dpp::consensus::signature::SignatureError;
    use dpp::consensus::state::state_error::StateError;
    use dpp::consensus::ConsensusError;
    use dpp::dash_to_credits;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::identity::{Identity, IdentityPublicKey, IdentityV0, KeyType, Purpose, SecurityLevel};
    use dpp::platform_value::BinaryData;
    use dpp::prelude::IdentityNonce;
    use dpp::serialization::PlatformSerializable;
    use dpp::shielded::SerializedAction;
    use dpp::state_transition::proof_result::StateTransitionProofResult;
    use dpp::state_transition::shield_from_identity_transition::methods::ShieldFromIdentityTransitionMethodsV0;
    use dpp::state_transition::shield_from_identity_transition::v0::ShieldFromIdentityTransitionV0;
    use dpp::state_transition::shield_from_identity_transition::ShieldFromIdentityTransition;
    use dpp::state_transition::StateTransition;
    use drive::drive::Drive;
    use grovedb_commitment_tree::{
        Anchor, Builder, BundleType, DashMemo, Flags as OrchardFlags, FullViewingKey, NoteValue,
        Scope, SpendingKey,
    };
    use platform_version::version::PlatformVersion;
    use rand::rngs::{OsRng, StdRng};
    use rand::SeedableRng;
    use simple_signer::signer::SimpleSigner;
    use std::collections::BTreeMap;

    // ==========================================
    // Helpers
    // ==========================================

    fn create_identity_with_transfer_key(
        id: [u8; 32],
        balance: u64,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> (Identity, SimpleSigner) {
        let mut signer = SimpleSigner::default();

        let (auth_key, auth_private_key) =
            IdentityPublicKey::random_ecdsa_master_authentication_key_with_rng(
                0,
                rng,
                platform_version,
            )
            .expect("should create auth key");

        let (transfer_key, transfer_private_key) =
            IdentityPublicKey::random_key_with_known_attributes(
                1,
                rng,
                Purpose::TRANSFER,
                SecurityLevel::CRITICAL,
                KeyType::ECDSA_SECP256K1,
                None,
                platform_version,
            )
            .expect("should create transfer key");

        signer.add_identity_public_key(auth_key.clone(), auth_private_key);
        signer.add_identity_public_key(transfer_key.clone(), transfer_private_key);

        let mut public_keys = BTreeMap::new();
        public_keys.insert(auth_key.id(), auth_key);
        public_keys.insert(transfer_key.id(), transfer_key);

        let identity: Identity = IdentityV0 {
            id: id.into(),
            revision: 0,
            balance,
            public_keys,
        }
        .into();

        (identity, signer)
    }

    /// Adds the identity and books its balance into system credits so the fixture
    /// starts balanced (the block-end sum-tree check then asserts conservation).
    fn add_identity_to_drive(platform: &mut TempPlatform<MockCoreRPCLike>, identity: &Identity) {
        let platform_version = PlatformVersion::latest();

        platform
            .drive
            .add_to_system_credits(identity.balance(), None, platform_version)
            .expect("expected to add to system credits");

        platform
            .drive
            .add_new_identity(
                identity.clone(),
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("should add identity");
    }

    struct ProvenBundle {
        actions: Vec<SerializedAction>,
        amount: u64,
        anchor: [u8; 32],
        proof: Vec<u8>,
        binding_signature: [u8; 64],
    }

    /// Builds and proves a real outputs-only Orchard bundle paying `shield_value`
    /// to a fresh recipient. Identical to the `Shield` fixture: no extra sighash data.
    fn build_valid_bundle(shield_value: u64) -> ProvenBundle {
        let mut rng = OsRng;
        let pk = get_proving_key();

        let sk = SpendingKey::from_bytes([0u8; 32]).unwrap();
        let fvk = FullViewingKey::from(&sk);
        let recipient = fvk.address_at(0u32, Scope::External);

        let mut builder = Builder::<DashMemo>::new(
            BundleType::Transactional {
                flags: OrchardFlags::SPENDS_DISABLED,
                bundle_required: false,
            },
            Anchor::empty_tree(),
        );
        builder
            .add_output(
                None,
                recipient,
                NoteValue::from_raw(shield_value),
                [0u8; 36],
            )
            .unwrap();

        let (unauthorized, _) = builder.build::<i64>(&mut rng).unwrap().unwrap();
        let bundle_commitment: [u8; 32] = unauthorized.commitment().into();
        let sighash = compute_platform_sighash(&bundle_commitment, &[]);
        let proven = unauthorized.create_proof(pk, &mut rng).unwrap();
        let bundle = proven.apply_signatures(rng, sighash, &[]).unwrap();

        let (actions, _flags, value_balance, anchor, proof, binding_signature) =
            serialize_authorized_bundle_with_flags(&bundle);
        assert!(value_balance < 0, "value must enter the pool");

        ProvenBundle {
            actions,
            amount: (-value_balance) as u64,
            anchor,
            proof,
            binding_signature,
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn create_signed_transition(
        identity: &Identity,
        signer: &SimpleSigner,
        bundle: ProvenBundle,
        amount: u64,
        nonce: IdentityNonce,
        user_fee_increase: u16,
        platform_version: &PlatformVersion,
    ) -> StateTransition {
        ShieldFromIdentityTransition::try_from_bundle_with_identity_signer(
            identity,
            amount,
            bundle.actions,
            bundle.anchor,
            bundle.proof,
            bundle.binding_signature,
            user_fee_increase,
            signer,
            None,
            nonce,
            platform_version,
        )
        .await
        .expect("should create signed transition")
    }

    /// A structurally valid transition carrying dummy (unprovable) bundle bytes.
    fn dummy_bundle() -> ProvenBundle {
        ProvenBundle {
            actions: vec![create_dummy_serialized_action()],
            amount: 1_000,
            anchor: [42u8; 32],
            proof: vec![7u8; 100],
            binding_signature: [0u8; 64],
        }
    }

    fn identity_balance(platform: &TempPlatform<MockCoreRPCLike>, identity: &Identity) -> u64 {
        platform
            .drive
            .fetch_identity_balance(identity.id().to_buffer(), None, PlatformVersion::latest())
            .expect("should fetch")
            .expect("identity should exist")
    }

    // ==========================================
    // Rejections
    // ==========================================

    #[tokio::test]
    async fn test_zero_amount_is_rejected_by_structure_validation() {
        let platform_version = PlatformVersion::latest();
        let mut platform = setup_platform();
        let mut rng = StdRng::seed_from_u64(1);
        let (identity, signer) = create_identity_with_transfer_key(
            [1u8; 32],
            dash_to_credits!(1.0),
            &mut rng,
            platform_version,
        );
        add_identity_to_drive(&mut platform, &identity);

        let st = create_signed_transition(
            &identity,
            &signer,
            dummy_bundle(),
            0,
            1,
            0,
            platform_version,
        )
        .await;
        let result = process_transition(&platform, st, platform_version);

        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::UnpaidConsensusError(
                ConsensusError::BasicError(BasicError::ShieldedInvalidValueBalanceError(_))
            )]
        );
    }

    #[tokio::test]
    async fn test_unknown_identity_is_rejected() {
        let platform_version = PlatformVersion::latest();
        let platform = setup_platform();
        let mut rng = StdRng::seed_from_u64(2);
        let (identity, signer) = create_identity_with_transfer_key(
            [2u8; 32],
            dash_to_credits!(1.0),
            &mut rng,
            platform_version,
        );

        let st = create_signed_transition(
            &identity,
            &signer,
            dummy_bundle(),
            1_000,
            1,
            0,
            platform_version,
        )
        .await;
        let result = process_transition(&platform, st, platform_version);

        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::UnpaidConsensusError(
                ConsensusError::SignatureError(SignatureError::IdentityNotFoundError(_))
            )]
        );
    }

    #[tokio::test]
    async fn test_insufficient_balance_is_rejected_before_proof_verification() {
        let platform_version = PlatformVersion::latest();
        let mut platform = setup_platform();
        let mut rng = StdRng::seed_from_u64(3);
        let (identity, signer) = create_identity_with_transfer_key(
            [3u8; 32],
            dash_to_credits!(0.001),
            &mut rng,
            platform_version,
        );
        add_identity_to_drive(&mut platform, &identity);

        // amount alone exceeds the balance; the dummy proof never gets verified
        let st = create_signed_transition(
            &identity,
            &signer,
            dummy_bundle(),
            dash_to_credits!(1.0),
            1,
            0,
            platform_version,
        )
        .await;
        let result = process_transition(&platform, st, platform_version);

        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::UnpaidConsensusError(
                ConsensusError::StateError(StateError::IdentityInsufficientBalanceError(_))
            )]
        );
    }

    #[tokio::test]
    async fn test_invalid_identity_signature_is_rejected() {
        let platform_version = PlatformVersion::latest();
        let mut platform = setup_platform();
        let mut rng = StdRng::seed_from_u64(4);
        let (identity, _signer) = create_identity_with_transfer_key(
            [4u8; 32],
            dash_to_credits!(1.0),
            &mut rng,
            platform_version,
        );
        add_identity_to_drive(&mut platform, &identity);

        let transfer_key = identity
            .get_first_public_key_matching(
                Purpose::TRANSFER,
                SecurityLevel::full_range().into(),
                KeyType::all_key_types().into(),
                true,
            )
            .expect("should have transfer key");
        let bundle = dummy_bundle();
        let st: StateTransition = ShieldFromIdentityTransitionV0 {
            identity_id: identity.id(),
            amount: 1_000,
            actions: bundle.actions,
            anchor: bundle.anchor,
            proof: bundle.proof,
            binding_signature: bundle.binding_signature,
            nonce: 1,
            user_fee_increase: 0,
            signature_public_key_id: transfer_key.id(),
            signature: BinaryData::new(vec![0x30; 65]),
        }
        .into();
        let result = process_transition(&platform, st, platform_version);

        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::UnpaidConsensusError(
                ConsensusError::SignatureError(_)
            )]
        );
    }

    #[tokio::test]
    async fn test_invalid_orchard_proof_is_rejected() {
        let platform_version = PlatformVersion::latest();
        let mut platform = setup_platform();
        let mut rng = StdRng::seed_from_u64(5);
        let (identity, signer) = create_identity_with_transfer_key(
            [5u8; 32],
            dash_to_credits!(1.0),
            &mut rng,
            platform_version,
        );
        add_identity_to_drive(&mut platform, &identity);

        let st = create_signed_transition(
            &identity,
            &signer,
            dummy_bundle(),
            1_000,
            1,
            0,
            platform_version,
        )
        .await;
        let result = process_transition(&platform, st, platform_version);

        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::UnpaidConsensusError(
                ConsensusError::StateError(StateError::InvalidShieldedProofError(_))
            )]
        );
    }

    #[tokio::test]
    async fn test_valid_bundle_with_mutated_amount_is_rejected() {
        let platform_version = PlatformVersion::latest();
        let mut platform = setup_platform();
        let mut rng = StdRng::seed_from_u64(6);
        let (identity, signer) = create_identity_with_transfer_key(
            [6u8; 32],
            dash_to_credits!(1.0),
            &mut rng,
            platform_version,
        );
        add_identity_to_drive(&mut platform, &identity);

        let bundle = build_valid_bundle(5_000);
        let mutated_amount = bundle.amount + 1;
        // The identity signature is over the mutated amount, so it verifies; the Orchard
        // binding signature commits to the real value balance and must reject it.
        let st = create_signed_transition(
            &identity,
            &signer,
            bundle,
            mutated_amount,
            1,
            0,
            platform_version,
        )
        .await;
        let result = process_transition(&platform, st, platform_version);

        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::UnpaidConsensusError(
                ConsensusError::StateError(StateError::InvalidShieldedProofError(_))
            )]
        );
    }

    #[tokio::test]
    async fn test_wrong_nonce_is_rejected() {
        let platform_version = PlatformVersion::latest();
        let mut platform = setup_platform();
        let mut rng = StdRng::seed_from_u64(7);
        let (identity, signer) = create_identity_with_transfer_key(
            [7u8; 32],
            dash_to_credits!(1.0),
            &mut rng,
            platform_version,
        );
        add_identity_to_drive(&mut platform, &identity);

        // nonce 0 is never a valid next nonce for a fresh identity
        let st = create_signed_transition(
            &identity,
            &signer,
            dummy_bundle(),
            1_000,
            0,
            0,
            platform_version,
        )
        .await;
        let result = process_transition(&platform, st, platform_version);

        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::UnpaidConsensusError(
                ConsensusError::StateError(StateError::InvalidIdentityNonceError(_))
            )]
        );
    }

    #[tokio::test]
    async fn test_rejected_before_protocol_version_14() {
        let platform_version = PlatformVersion::get(13).expect("protocol version 13");
        let platform_config = PlatformConfig {
            testing_configs: PlatformTestConfig {
                disable_instant_lock_signature_verification: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(platform_config)
            .with_initial_protocol_version(13)
            .build_with_mock_rpc()
            .set_genesis_state();
        let mut rng = StdRng::seed_from_u64(8);
        let (identity, signer) = create_identity_with_transfer_key(
            [8u8; 32],
            dash_to_credits!(1.0),
            &mut rng,
            platform_version,
        );
        add_identity_to_drive(&mut platform, &identity);

        let st = create_signed_transition(
            &identity,
            &signer,
            dummy_bundle(),
            1_000,
            1,
            0,
            platform_version,
        )
        .await;
        let result = process_transition(&platform, st, platform_version);

        // Below the activation version the transition is refused at decode time by the
        // `active_version_range` check (`14..=LATEST`), before any consensus validation;
        // the `is_allowed` gate is the second line of defence for a decodable transition.
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::InternalError(message)]
                if message.contains("ShieldFromIdentity") && message.contains("not active")
        );
    }

    // ==========================================
    // Success, fees, and conservation
    // ==========================================

    #[tokio::test]
    async fn test_valid_shield_from_identity_moves_credits_and_conserves_supply() {
        let platform_version = PlatformVersion::latest();
        let mut platform = setup_platform();
        let mut rng = StdRng::seed_from_u64(9);
        let initial_balance = dash_to_credits!(1.0);
        let (identity, signer) = create_identity_with_transfer_key(
            [9u8; 32],
            initial_balance,
            &mut rng,
            platform_version,
        );
        add_identity_to_drive(&mut platform, &identity);

        let credits_before = platform
            .drive
            .calculate_total_credits_balance(None, &platform_version.drive)
            .expect("should calculate total credits before");
        assert!(
            credits_before.ok().expect("no overflow"),
            "fixture must start balanced"
        );

        let bundle = build_valid_bundle(5_000);
        let shield_amount = bundle.amount;
        let num_actions = bundle.actions.len();
        let st = create_signed_transition(
            &identity,
            &signer,
            bundle,
            shield_amount,
            1,
            0,
            platform_version,
        )
        .await;

        // CheckTx must never mutate committed state for this transition type.
        {
            let guard_bytes = st.serialize_to_bytes().expect("serialize for guard");
            crate::test::helpers::state_mutation_guard::assert_check_tx_valid_at_all_levels(
                &platform,
                &guard_bytes,
                "shield from identity",
            );
        }

        // Full block pipeline: execute, distribute fees, validate sum trees.
        let platform_state = platform.state.load();
        let (fee_results, _) =
            process_state_transitions(&platform, &[st], BlockInfo::default(), &platform_state);

        let booked = &fee_results[0];
        let shielded_verification_fee =
            dpp::shielded::compute_shielded_verification_fee(num_actions, platform_version)
                .expect("compute fee");
        assert!(
            booked.storage_fee > 0,
            "metered note storage must be captured, got {}",
            booked.storage_fee
        );
        assert!(
            booked.processing_fee >= shielded_verification_fee,
            "processing fee ({}) must include the shielded compute fee ({})",
            booked.processing_fee,
            shielded_verification_fee
        );

        let credits_after = platform
            .drive
            .calculate_total_credits_balance(None, &platform_version.drive)
            .expect("should calculate total credits after");
        assert!(
            credits_after.ok().expect("no overflow"),
            "credits must remain balanced after the shield: {credits_after}"
        );
        assert_eq!(
            credits_after.total_credits_in_platform, credits_before.total_credits_in_platform,
            "identity to pool must not mint or burn system credits"
        );
        assert_eq!(
            credits_after.total_in_shielded_balances - credits_before.total_in_shielded_balances,
            shield_amount as i64,
            "shielded pool must gain exactly the shield amount"
        );

        let final_balance = identity_balance(&platform, &identity);
        assert_eq!(
            final_balance,
            initial_balance - shield_amount - booked.total_base_fee(),
            "identity must be debited exactly amount + booked fee"
        );

        let nonce = platform
            .drive
            .fetch_identity_nonce(identity.id().to_buffer(), true, None, platform_version)
            .expect("fetch nonce")
            .expect("nonce must be stored");
        assert_eq!(nonce, 1, "identity nonce must advance");
    }

    #[tokio::test]
    async fn test_user_fee_increase_raises_the_booked_fee() {
        let platform_version = PlatformVersion::latest();
        let mut fees = vec![];
        for (seed, user_fee_increase) in [(10u64, 0u16), (11u64, 100u16)] {
            let mut platform = setup_platform();
            let mut rng = StdRng::seed_from_u64(seed);
            let (identity, signer) = create_identity_with_transfer_key(
                [seed as u8; 32],
                dash_to_credits!(1.0),
                &mut rng,
                platform_version,
            );
            add_identity_to_drive(&mut platform, &identity);
            let bundle = build_valid_bundle(5_000);
            let amount = bundle.amount;
            let st = create_signed_transition(
                &identity,
                &signer,
                bundle,
                amount,
                1,
                user_fee_increase,
                platform_version,
            )
            .await;
            let platform_state = platform.state.load();
            let (fee_results, _) =
                process_state_transitions(&platform, &[st], BlockInfo::default(), &platform_state);
            fees.push(fee_results[0].total_base_fee());
        }
        assert!(
            fees[1] > fees[0],
            "a user fee increase must raise the booked fee ({} vs {})",
            fees[1],
            fees[0]
        );
    }

    #[tokio::test]
    async fn test_prove_and_verify_returns_post_debit_identity_balance() {
        let platform_version = PlatformVersion::latest();
        let mut platform = setup_platform();
        let mut rng = StdRng::seed_from_u64(12);
        let initial_balance = dash_to_credits!(1.0);
        let (identity, signer) = create_identity_with_transfer_key(
            [12u8; 32],
            initial_balance,
            &mut rng,
            platform_version,
        );
        add_identity_to_drive(&mut platform, &identity);

        let bundle = build_valid_bundle(5_000);
        let amount = bundle.amount;
        let st =
            create_signed_transition(&identity, &signer, bundle, amount, 1, 0, platform_version)
                .await;

        let transition_bytes = st.serialize_to_bytes().expect("serialize");
        let platform_state = platform.state.load();
        let transaction = platform.drive.grove.start_transaction();
        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![transition_bytes],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
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
            .expect("commit");

        let proof_bytes = platform
            .drive
            .prove_state_transition(&st, None, platform_version)
            .expect("prove")
            .into_data()
            .expect("proof data");

        let (root_hash, outcome) = Drive::verify_state_transition_was_executed_with_proof(
            &st,
            &BlockInfo::default(),
            &proof_bytes,
            &|_| Ok(None),
            platform_version,
        )
        .expect("verify");
        assert_ne!(root_hash, [0u8; 32]);

        let result = outcome.into_result();
        let StateTransitionProofResult::VerifiedPartialIdentity(partial) = result else {
            panic!("expected VerifiedPartialIdentity, got {result:?}");
        };
        assert_eq!(partial.id, identity.id());
        let proven_balance = partial.balance.expect("balance must be proven");
        assert_eq!(proven_balance, identity_balance(&platform, &identity));
        assert!(proven_balance < initial_balance - amount);
    }
}
