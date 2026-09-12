#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use crate::config::{PlatformConfig, PlatformTestConfig};
    use crate::execution::validation::state_transition::state_transitions::shielded_common::compute_platform_sighash;
    use crate::execution::validation::state_transition::state_transitions::test_helpers::{
        create_dummy_serialized_action, get_proving_key, insert_anchor_into_state,
        insert_dummy_encrypted_notes, process_transition, serialize_authorized_bundle_i64,
        set_pool_total_balance, setup_platform,
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
    use dpp::identity::accessors::{IdentityGettersV0, IdentitySettersV0};
    use dpp::identity::Identity;
    use dpp::platform_value::Identifier;
    use dpp::serialization::PlatformSerializable;
    use dpp::shielded::SerializedAction;
    use dpp::state_transition::identity_top_up_from_shielded_pool_transition::v0::IdentityTopUpFromShieldedPoolTransitionV0;
    use dpp::state_transition::identity_top_up_from_shielded_pool_transition::IdentityTopUpFromShieldedPoolTransition;
    use dpp::state_transition::proof_result::StateTransitionProofResult;
    use dpp::state_transition::StateTransition;
    use drive::drive::Drive;
    use grovedb_commitment_tree::{
        Builder, BundleType, ClientMemoryCommitmentTree, DashMemo, ExtractedNoteCommitment,
        FullViewingKey, Note, NoteValue, Position, RandomSeed, Retention, Rho, Scope,
        SpendAuthorizingKey, SpendingKey,
    };
    use platform_version::version::PlatformVersion;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    const NOTE_VALUE: u64 = 500_000_000;
    const CHANGE_VALUE: u64 = 5_000;
    /// value_balance of the fixture bundle: one spent note minus one change output.
    const GROSS_AMOUNT: u64 = NOTE_VALUE - CHANGE_VALUE;

    fn create_transition(
        identity_id: Identifier,
        actions: Vec<SerializedAction>,
        top_up_amount: u64,
        anchor: [u8; 32],
        proof: Vec<u8>,
        binding_signature: [u8; 64],
    ) -> StateTransition {
        IdentityTopUpFromShieldedPoolTransition::V0(IdentityTopUpFromShieldedPoolTransitionV0 {
            identity_id,
            actions,
            top_up_amount,
            anchor,
            proof,
            binding_signature,
        })
        .into()
    }

    fn dummy_transition(identity_id: Identifier, top_up_amount: u64) -> StateTransition {
        create_transition(
            identity_id,
            vec![create_dummy_serialized_action()],
            top_up_amount,
            [42u8; 32],
            vec![7u8; 100],
            [0u8; 64],
        )
    }

    /// Builds and proves a real spend bundle (one 500M note in, one 5_000 change note
    /// out) whose binding signature commits to `identity_id` and `top_up_amount`.
    fn build_valid_bundle(
        identity_id: &Identifier,
        top_up_amount: u64,
    ) -> (Vec<SerializedAction>, i64, [u8; 32], Vec<u8>, [u8; 64]) {
        let mut rng = StdRng::seed_from_u64(0);
        let pk = get_proving_key();

        let sk = SpendingKey::from_bytes([0u8; 32]).unwrap();
        let fvk = FullViewingKey::from(&sk);
        let recipient = fvk.address_at(0u32, Scope::External);
        let ask = SpendAuthorizingKey::from(&sk);

        let rho_bytes: [u8; 32] = {
            let mut b = [0u8; 32];
            b[0] = 1;
            b
        };
        let rho = Rho::from_bytes(&rho_bytes).unwrap();
        let rseed = RandomSeed::from_bytes([42u8; 32], &rho).unwrap();
        let note =
            Note::from_parts(recipient, NoteValue::from_raw(NOTE_VALUE), rho, rseed).unwrap();

        let cmx = ExtractedNoteCommitment::from(note.commitment());
        let mut tree = ClientMemoryCommitmentTree::new(100);
        tree.append(cmx.to_bytes(), Retention::Marked).unwrap();
        tree.checkpoint(0u32).unwrap();
        let anchor = tree.anchor().unwrap();
        let merkle_path = tree.witness(Position::from(0u64), 0).unwrap().unwrap();

        let mut builder = Builder::<DashMemo>::new(BundleType::DEFAULT, anchor);
        builder.add_spend(fvk.clone(), note, merkle_path).unwrap();
        builder
            .add_output(
                None,
                recipient,
                NoteValue::from_raw(CHANGE_VALUE),
                [0u8; 36],
            )
            .unwrap();

        let (unauthorized, _) = builder.build::<i64>(&mut rng).unwrap().unwrap();

        let extra_sighash_data = dpp::shielded::identity_top_up_from_shielded_extra_sighash_data_v0(
            &identity_id.to_buffer(),
            top_up_amount,
        );
        let bundle_commitment: [u8; 32] = unauthorized.commitment().into();
        let sighash = compute_platform_sighash(&bundle_commitment, &extra_sighash_data);

        let proven = unauthorized.create_proof(pk, &mut rng).unwrap();
        let bundle = proven.apply_signatures(rng, sighash, &[ask]).unwrap();

        serialize_authorized_bundle_i64(&bundle)
    }

    /// Adds an identity holding `balance` and books it into system credits so the
    /// fixture starts balanced.
    fn add_identity(platform: &TempPlatform<MockCoreRPCLike>, seed: u64, balance: u64) -> Identity {
        let platform_version = PlatformVersion::latest();
        let mut identity =
            Identity::random_identity(2, Some(seed), platform_version).expect("identity");
        identity.set_balance(balance);
        platform
            .drive
            .add_to_system_credits(balance, None, platform_version)
            .expect("system credits");
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
        identity
    }

    fn identity_balance(platform: &TempPlatform<MockCoreRPCLike>, id: Identifier) -> u64 {
        platform
            .drive
            .fetch_identity_balance(id.to_buffer(), None, PlatformVersion::latest())
            .expect("fetch")
            .expect("identity should exist")
    }

    /// Seeds the pool state the spend needs: enough notes for the minimum-notes floor,
    /// the bundle's anchor, and a pool total balance (`set_pool_total_balance` books it
    /// into system credits, so the block-level conservation check holds).
    fn seed_pool(platform: &TempPlatform<MockCoreRPCLike>, anchor: &[u8; 32], pool_total: u64) {
        insert_dummy_encrypted_notes(platform, 250);
        insert_anchor_into_state(platform, anchor);
        set_pool_total_balance(platform, pool_total);
    }

    fn top_up_fee(num_actions: usize) -> u64 {
        dpp::shielded::compute_shielded_identity_top_up_fee(num_actions, PlatformVersion::latest())
            .expect("fee")
    }

    // ==========================================
    // Rejections
    // ==========================================

    #[test]
    fn test_zero_amount_is_rejected_by_structure_validation() {
        let platform_version = PlatformVersion::latest();
        let platform = setup_platform();
        let result = process_transition(
            &platform,
            dummy_transition(Identifier::from([1u8; 32]), 0),
            platform_version,
        );
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::UnpaidConsensusError(
                ConsensusError::BasicError(BasicError::ShieldedInvalidValueBalanceError(_))
            )]
        );
    }

    #[test]
    fn test_amount_below_the_flat_fee_is_rejected() {
        let platform_version = PlatformVersion::latest();
        let platform = setup_platform();
        let fee = top_up_fee(1);
        let result = process_transition(
            &platform,
            dummy_transition(Identifier::from([1u8; 32]), fee - 1),
            platform_version,
        );
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::UnpaidConsensusError(
                ConsensusError::StateError(StateError::InsufficientShieldedFeeError(_))
            )]
        );
    }

    #[test]
    fn test_invalid_orchard_proof_is_rejected() {
        let platform_version = PlatformVersion::latest();
        let platform = setup_platform();
        let result = process_transition(
            &platform,
            dummy_transition(Identifier::from([1u8; 32]), dash_to_credits!(0.1)),
            platform_version,
        );
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::UnpaidConsensusError(
                ConsensusError::StateError(StateError::InvalidShieldedProofError(_))
            )]
        );
    }

    #[test]
    fn test_unknown_identity_is_rejected_with_a_valid_proof() {
        let platform_version = PlatformVersion::latest();
        let platform = setup_platform();
        let identity_id = Identifier::from([9u8; 32]);
        let (actions, value_balance, anchor, proof, binding_sig) =
            build_valid_bundle(&identity_id, GROSS_AMOUNT);
        assert_eq!(value_balance as u64, GROSS_AMOUNT);
        seed_pool(&platform, &anchor, NOTE_VALUE);

        let st = create_transition(
            identity_id,
            actions,
            GROSS_AMOUNT,
            anchor,
            proof,
            binding_sig,
        );
        let result = process_transition(&platform, st, platform_version);
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::UnpaidConsensusError(
                ConsensusError::SignatureError(SignatureError::IdentityNotFoundError(_))
            )]
        );
    }

    #[test]
    fn test_valid_bundle_with_mutated_amount_is_rejected() {
        let platform_version = PlatformVersion::latest();
        let platform = setup_platform();
        let identity = add_identity(&platform, 1, dash_to_credits!(0.1));
        let (actions, _, anchor, proof, binding_sig) =
            build_valid_bundle(&identity.id(), GROSS_AMOUNT);
        seed_pool(&platform, &anchor, NOTE_VALUE * 2);

        let st = create_transition(
            identity.id(),
            actions,
            GROSS_AMOUNT + 1_000,
            anchor,
            proof,
            binding_sig,
        );
        let result = process_transition(&platform, st, platform_version);
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::UnpaidConsensusError(
                ConsensusError::StateError(StateError::InvalidShieldedProofError(_))
            )]
        );
    }

    #[test]
    fn test_valid_bundle_repointed_at_another_identity_is_rejected() {
        let platform_version = PlatformVersion::latest();
        let platform = setup_platform();
        let signed_for = add_identity(&platform, 2, dash_to_credits!(0.1));
        let other = add_identity(&platform, 3, dash_to_credits!(0.1));
        let (actions, _, anchor, proof, binding_sig) =
            build_valid_bundle(&signed_for.id(), GROSS_AMOUNT);
        seed_pool(&platform, &anchor, NOTE_VALUE);

        // The bundle's sighash commits to `signed_for`; crediting `other` must fail.
        let st = create_transition(
            other.id(),
            actions,
            GROSS_AMOUNT,
            anchor,
            proof,
            binding_sig,
        );
        let result = process_transition(&platform, st, platform_version);
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::UnpaidConsensusError(
                ConsensusError::StateError(StateError::InvalidShieldedProofError(_))
            )]
        );
        assert_eq!(
            identity_balance(&platform, other.id()),
            dash_to_credits!(0.1)
        );
    }

    #[test]
    fn test_rejected_before_protocol_version_14() {
        let platform_version = PlatformVersion::get(13).expect("protocol version 13");
        let platform_config = PlatformConfig {
            testing_configs: PlatformTestConfig {
                disable_instant_lock_signature_verification: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let platform = TestPlatformBuilder::new()
            .with_config(platform_config)
            .with_initial_protocol_version(13)
            .build_with_mock_rpc()
            .set_genesis_state();
        let result = process_transition(
            &platform,
            dummy_transition(Identifier::from([1u8; 32]), dash_to_credits!(0.1)),
            platform_version,
        );
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::InternalError(message)]
                if message.contains("IdentityTopUpFromShieldedPool") && message.contains("not active")
        );
    }

    // ==========================================
    // Success, fees, conservation, proof
    // ==========================================

    #[test]
    fn test_valid_top_up_credits_identity_and_conserves_supply() {
        let platform_version = PlatformVersion::latest();
        let platform = setup_platform();
        let initial_balance = dash_to_credits!(0.1);
        let identity = add_identity(&platform, 4, initial_balance);
        let (actions, value_balance, anchor, proof, binding_sig) =
            build_valid_bundle(&identity.id(), GROSS_AMOUNT);
        assert_eq!(value_balance as u64, GROSS_AMOUNT);
        let num_actions = actions.len();
        seed_pool(&platform, &anchor, NOTE_VALUE);

        let credits_before = platform
            .drive
            .calculate_total_credits_balance(None, &platform_version.drive)
            .expect("total credits before");
        assert!(
            credits_before.ok().expect("no overflow"),
            "fixture must start balanced"
        );

        let st = create_transition(
            identity.id(),
            actions,
            GROSS_AMOUNT,
            anchor,
            proof,
            binding_sig,
        );

        {
            let guard_bytes = st.serialize_to_bytes().expect("serialize for guard");
            crate::test::helpers::state_mutation_guard::assert_check_tx_valid_at_all_levels(
                &platform,
                &guard_bytes,
                "identity top up from shielded pool",
            );
        }

        let platform_state = platform.state.load();
        let (fee_results, _) =
            process_state_transitions(&platform, &[st], BlockInfo::default(), &platform_state);
        let fee = top_up_fee(num_actions);
        assert_eq!(
            fee_results[0].total_base_fee(),
            fee,
            "the flat pool-paid fee must be booked exactly"
        );

        let credits_after = platform
            .drive
            .calculate_total_credits_balance(None, &platform_version.drive)
            .expect("total credits after");
        assert!(
            credits_after.ok().expect("no overflow"),
            "credits must stay balanced: {credits_after}"
        );
        assert_eq!(
            credits_after.total_credits_in_platform, credits_before.total_credits_in_platform,
            "pool to identity must not mint or burn system credits"
        );
        assert_eq!(
            credits_before.total_in_shielded_balances - credits_after.total_in_shielded_balances,
            GROSS_AMOUNT as i64,
            "pool must lose exactly the gross amount"
        );
        assert_eq!(
            identity_balance(&platform, identity.id()),
            initial_balance + GROSS_AMOUNT - fee,
            "identity must gain the gross amount minus the flat fee"
        );
    }

    #[test]
    fn test_prove_and_verify_returns_identity_and_spent_nullifiers() {
        let platform_version = PlatformVersion::latest();
        let platform = setup_platform();
        let identity = add_identity(&platform, 5, dash_to_credits!(0.1));
        let (actions, _, anchor, proof, binding_sig) =
            build_valid_bundle(&identity.id(), GROSS_AMOUNT);
        let expected_nullifiers: Vec<Vec<u8>> =
            actions.iter().map(|a| a.nullifier.to_vec()).collect();
        seed_pool(&platform, &anchor, NOTE_VALUE);

        let st = create_transition(
            identity.id(),
            actions,
            GROSS_AMOUNT,
            anchor,
            proof,
            binding_sig,
        );
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
            .expect("process");
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
        let StateTransitionProofResult::VerifiedIdentityWithShieldedNullifiers(proven, statuses) =
            result
        else {
            panic!("expected VerifiedIdentityWithShieldedNullifiers, got {result:?}");
        };
        assert_eq!(proven.id(), identity.id());
        assert_eq!(proven.balance(), identity_balance(&platform, identity.id()));
        assert_eq!(
            statuses
                .iter()
                .map(|(nf, _)| nf.clone())
                .collect::<Vec<_>>(),
            expected_nullifiers
        );
        assert!(
            statuses.iter().all(|(_, spent)| *spent),
            "all nullifiers must be spent"
        );
    }
}
