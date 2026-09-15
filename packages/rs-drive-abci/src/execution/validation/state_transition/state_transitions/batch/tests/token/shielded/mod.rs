use super::*;

/// Token shielded pool transitions: shield, unshield and shielded transfer inside a batch.
///
/// The bundles are real Orchard bundles proven with the shared proving key, so the tests cover
/// the whole path: identity signature and fee, pool bookkeeping, sighash binding, anchor and
/// nullifier checks, and token conservation with the pool as a balance term.
mod token_shielded_pool_tests {
    use super::*;
    use crate::execution::validation::state_transition::state_transitions::test_helpers::{
        create_dummy_serialized_action, get_proving_key, serialize_authorized_bundle_i64,
    };
    use crate::platform_types::state_transitions_processing_result::StateTransitionsProcessingResult;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TempPlatform;
    use dpp::data_contract::associated_token::token_configuration::accessors::v1::TokenConfigurationV1Setters;
    use dpp::data_contract::DataContract;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::identity_nonce::IDENTITY_NONCE_VALUE_FILTER;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::shielded::{
        compute_platform_sighash, token_shielded_transfer_extra_sighash_data_v0,
        token_unshield_extra_sighash_data_v0, OrchardBundleParams,
    };
    use dpp::state_transition::data_contract_create_transition::methods::DataContractCreateTransitionMethodsV0;
    use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
    use dpp::state_transition::StateTransition;
    use dpp::tests::json_document::json_document_to_contract_with_ids;
    use dpp::version::feature_initial_protocol_versions::TOKEN_SHIELDED_POOL_INITIAL_PROTOCOL_VERSION;
    use drive::drive::shielded::paths::token_shielded_pool_anchors_path_vec;
    use drive::grovedb::Element;
    use grovedb_commitment_tree::{
        Anchor, Authorized, Builder, Bundle, BundleType, ClientMemoryCommitmentTree, DashMemo,
        ExtractedNoteCommitment, Flags, FullViewingKey, MerklePath, Note, NoteValue,
        PaymentAddress, Position, RandomSeed, Retention, Rho, Scope, SpendAuthorizingKey,
        SpendingKey,
    };
    use platform_version::version::PlatformVersion;

    /// The basic token fixture mints this much to the contract owner.
    const OWNER_INITIAL_BALANCE: u64 = 100_000;
    const SHIELD_AMOUNT: u64 = 10_000;

    fn enable_shielded_pool(configuration: &mut TokenConfiguration) {
        configuration.set_has_shielded_pool(true);
    }

    fn spend_keys() -> (FullViewingKey, SpendAuthorizingKey, PaymentAddress) {
        let sk = SpendingKey::from_bytes([7u8; 32]).unwrap();
        let fvk = FullViewingKey::from(&sk);
        let address = fvk.address_at(0u32, Scope::External);
        (fvk, SpendAuthorizingKey::from(&sk), address)
    }

    fn bundle_params(bundle: &Bundle<Authorized, i64, DashMemo>) -> (OrchardBundleParams, i64) {
        let (actions, value_balance, anchor, proof, binding_signature) =
            serialize_authorized_bundle_i64(bundle);
        (
            OrchardBundleParams {
                actions,
                anchor,
                proof,
                binding_signature,
            },
            value_balance,
        )
    }

    /// An outputs-only bundle paying `amount` to the test wallet: what a token shield carries.
    /// The bundle has no spends, so no extra sighash data is bound.
    fn build_shield_bundle(amount: u64, seed: u64) -> OrchardBundleParams {
        let mut rng = StdRng::seed_from_u64(seed);
        let (fvk, _, address) = spend_keys();
        let mut builder = Builder::<DashMemo>::new(
            BundleType::Transactional {
                flags: Flags::SPENDS_DISABLED,
                bundle_required: false,
            },
            Anchor::empty_tree(),
        );
        builder
            .add_output(
                Some(fvk.to_ovk(Scope::External)),
                address,
                NoteValue::from_raw(amount),
                [0u8; 36],
            )
            .unwrap();
        let (unauthorized, _) = builder.build::<i64>(&mut rng).unwrap().unwrap();
        let commitment: [u8; 32] = unauthorized.commitment().into();
        let sighash = compute_platform_sighash(&commitment, &[]);
        let proven = unauthorized
            .create_proof(get_proving_key(), &mut rng)
            .unwrap();
        let bundle = proven.apply_signatures(rng, sighash, &[]).unwrap();
        let (params, value_balance) = bundle_params(&bundle);
        assert_eq!(value_balance, -(amount as i64));
        params
    }

    /// A note the test wallet controls, alone in a client-side commitment tree. The returned
    /// anchor is the tree's root; a spend of the note is only accepted once that anchor is in
    /// the pool's anchors.
    fn spendable_note(value: u64, tag: u8) -> (Note, Anchor, MerklePath) {
        let (_, _, address) = spend_keys();
        let mut rho_bytes = [0u8; 32];
        rho_bytes[0] = tag;
        let rho = Rho::from_bytes(&rho_bytes).unwrap();
        let rseed = RandomSeed::from_bytes([42u8; 32], &rho).unwrap();
        let note = Note::from_parts(address, NoteValue::from_raw(value), rho, rseed).unwrap();
        let cmx = ExtractedNoteCommitment::from(note.commitment());
        let mut tree = ClientMemoryCommitmentTree::new(100);
        tree.append(cmx.to_bytes(), Retention::Marked).unwrap();
        tree.checkpoint(0u32).unwrap();
        let anchor = tree.anchor().unwrap();
        let merkle_path = tree.witness(Position::from(0u64), 0).unwrap().unwrap();
        (note, anchor, merkle_path)
    }

    /// Spends `note` against `anchor`, sending everything but `leaving_pool` back to the test
    /// wallet as a change note, bound to `extra_sighash_data`. `leaving_pool` is the bundle's
    /// value balance: the unshielded amount, or zero for a shielded transfer.
    fn build_spend_bundle(
        note: Note,
        merkle_path: MerklePath,
        anchor: Anchor,
        leaving_pool: u64,
        extra_sighash_data: &[u8],
        seed: u64,
    ) -> (OrchardBundleParams, i64) {
        let mut rng = StdRng::seed_from_u64(seed);
        let (fvk, ask, address) = spend_keys();
        let change = note.value().inner() - leaving_pool;
        let mut builder = Builder::<DashMemo>::new(BundleType::DEFAULT, anchor);
        builder.add_spend(fvk.clone(), note, merkle_path).unwrap();
        builder
            .add_output(
                Some(fvk.to_ovk(Scope::External)),
                address,
                NoteValue::from_raw(change),
                [0u8; 36],
            )
            .unwrap();
        let (unauthorized, _) = builder.build::<i64>(&mut rng).unwrap().unwrap();
        let commitment: [u8; 32] = unauthorized.commitment().into();
        let sighash = compute_platform_sighash(&commitment, extra_sighash_data);
        let proven = unauthorized
            .create_proof(get_proving_key(), &mut rng)
            .unwrap();
        let bundle = proven.apply_signatures(rng, sighash, &[ask]).unwrap();
        bundle_params(&bundle)
    }

    /// Structurally valid but unprovable: for tests that fail before proof verification.
    fn dummy_bundle() -> OrchardBundleParams {
        OrchardBundleParams {
            actions: vec![create_dummy_serialized_action()],
            anchor: [42u8; 32],
            proof: vec![0u8; 100],
            binding_signature: [0u8; 64],
        }
    }

    fn insert_token_pool_anchor(
        platform: &TempPlatform<MockCoreRPCLike>,
        token_id: Identifier,
        anchor: &Anchor,
    ) {
        let platform_version = PlatformVersion::latest();
        let transaction = platform.drive.grove.start_transaction();
        let anchors_path = token_shielded_pool_anchors_path_vec(token_id.to_buffer());
        platform
            .drive
            .grove
            .insert(
                anchors_path.as_slice(),
                &anchor.to_bytes(),
                Element::new_item(1u64.to_be_bytes().to_vec()),
                None,
                Some(&transaction),
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .expect("should insert token pool anchor");
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("should commit");
    }

    fn pool_balance(platform: &TempPlatform<MockCoreRPCLike>, token_id: Identifier) -> u64 {
        platform
            .drive
            .read_token_shielded_pool_total_balance(
                &token_id.to_buffer(),
                None,
                &mut vec![],
                PlatformVersion::latest(),
            )
            .expect("pool balance")
    }

    fn pool_notes_count(platform: &TempPlatform<MockCoreRPCLike>, token_id: Identifier) -> u64 {
        platform
            .drive
            .token_shielded_pool_notes_count(
                &token_id.to_buffer(),
                None,
                &mut vec![],
                PlatformVersion::latest(),
            )
            .expect("pool notes count")
    }

    fn nullifier_is_spent(
        platform: &TempPlatform<MockCoreRPCLike>,
        token_id: Identifier,
        nullifier: &[u8; 32],
    ) -> bool {
        platform
            .drive
            .has_token_pool_nullifier(
                &token_id.to_buffer(),
                nullifier,
                None,
                &mut vec![],
                PlatformVersion::latest(),
            )
            .expect("nullifier lookup")
    }

    fn identity_token_balance(
        platform: &TempPlatform<MockCoreRPCLike>,
        token_id: Identifier,
        identity_id: Identifier,
    ) -> Option<u64> {
        platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                identity_id.to_buffer(),
                None,
                PlatformVersion::latest(),
            )
            .expect("token balance")
    }

    fn assert_tokens_conserved(platform: &TempPlatform<MockCoreRPCLike>) {
        let total = platform
            .drive
            .calculate_total_tokens_balance(None, PlatformVersion::latest())
            .expect("total tokens balance");
        assert!(
            total.ok().expect("conservation check"),
            "identity balances plus pool balances must equal the total supply: {total:?}"
        );
    }

    /// Processes one transition in its own transaction and commits it.
    fn process(
        platform: &TempPlatform<MockCoreRPCLike>,
        transition: &StateTransition,
    ) -> StateTransitionsProcessingResult {
        let platform_version = PlatformVersion::latest();
        let platform_state = platform.state.load();
        let serialized = transition
            .serialize_to_bytes()
            .expect("serialize transition");
        let transaction = platform.drive.grove.start_transaction();
        let result = platform
            .platform
            .process_raw_state_transitions(
                &[serialized],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("process state transition");
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("commit");
        result
    }

    fn platform_with_latest_version() -> TempPlatform<MockCoreRPCLike> {
        TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state()
    }

    #[tokio::test]
    async fn test_token_shield_unshield_and_shielded_transfer() {
        let platform_version = PlatformVersion::latest();
        let mut platform = platform_with_latest_version();
        let mut rng = StdRng::seed_from_u64(9001);

        let (identity, signer, key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (recipient, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.1));

        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            identity.id(),
            Some(enable_shielded_pool),
            None,
            None,
            None,
            platform_version,
        );
        let token = token_id.to_buffer();

        // Creating the contract created an empty pool.
        assert_eq!(pool_balance(&platform, token_id), 0);
        assert_eq!(pool_notes_count(&platform, token_id), 0);
        assert_tokens_conserved(&platform);

        // Shield: identity balance to pool.
        let shield_bundle = build_shield_bundle(SHIELD_AMOUNT, 11);
        let shield = BatchTransition::new_token_shield_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            SHIELD_AMOUNT,
            shield_bundle.clone(),
            &key,
            2,
            0,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("token shield transition");

        let result = process(&platform, &shield);
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        assert!(
            result.token_shielded_pools_touched().contains(&token),
            "the block end anchor hook must learn about the touched pool"
        );
        assert_eq!(
            identity_token_balance(&platform, token_id, identity.id()),
            Some(OWNER_INITIAL_BALANCE - SHIELD_AMOUNT)
        );
        assert_eq!(pool_balance(&platform, token_id), SHIELD_AMOUNT);
        assert_eq!(
            pool_notes_count(&platform, token_id),
            shield_bundle.actions.len() as u64
        );
        assert_tokens_conserved(&platform);

        // The block end hook records the pool's anchor, and it must be the anchor a client
        // computes from the shield's commitments in action order.
        let transaction = platform.drive.grove.start_transaction();
        platform
            .drive
            .record_token_shielded_pool_anchor_if_changed(token, 1, &transaction, platform_version)
            .expect("record anchor");
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("commit");
        let mut client_tree = ClientMemoryCommitmentTree::new(100);
        for action in &shield_bundle.actions {
            client_tree
                .append(action.cmx, Retention::Marked)
                .expect("append commitment");
        }
        client_tree.checkpoint(1u32).unwrap();
        let client_anchor = client_tree.anchor().unwrap();
        let recorded = platform
            .drive
            .read_latest_recorded_token_shielded_pool_anchor(token, None, platform_version)
            .expect("read recorded anchor");
        assert_eq!(recorded, Some(client_anchor.to_bytes()));

        // Unshield: pool to the recipient's balance.
        let (note, anchor, merkle_path) = spendable_note(6_000, 1);
        insert_token_pool_anchor(&platform, token_id, &anchor);
        let unshield_amount = 4_000;
        let extra = token_unshield_extra_sighash_data_v0(
            &token,
            &identity.id().to_buffer(),
            &recipient.id().to_buffer(),
            unshield_amount,
        );
        let (unshield_bundle, value_balance) =
            build_spend_bundle(note, merkle_path, anchor, unshield_amount, &extra, 12);
        assert_eq!(value_balance, unshield_amount as i64);

        let unshield = BatchTransition::new_token_unshield_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            unshield_amount,
            recipient.id(),
            unshield_bundle.clone(),
            &key,
            3,
            0,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("token unshield transition");

        let result = process(&platform, &unshield);
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        assert_eq!(
            identity_token_balance(&platform, token_id, recipient.id()),
            Some(unshield_amount)
        );
        assert_eq!(
            pool_balance(&platform, token_id),
            SHIELD_AMOUNT - unshield_amount
        );
        assert_eq!(
            pool_notes_count(&platform, token_id),
            (shield_bundle.actions.len() + unshield_bundle.actions.len()) as u64
        );
        for action in &unshield_bundle.actions {
            assert!(nullifier_is_spent(&platform, token_id, &action.nullifier));
        }
        assert_tokens_conserved(&platform);

        // Replaying the spent bundle under a fresh nonce is a paid failure.
        let replay = BatchTransition::new_token_unshield_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            unshield_amount,
            recipient.id(),
            unshield_bundle,
            &key,
            4,
            0,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("token unshield transition");
        let result = process(&platform, &replay);
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(StateError::NullifierAlreadySpentError(_)),
                ..
            }]
        );
        assert_eq!(
            pool_balance(&platform, token_id),
            SHIELD_AMOUNT - unshield_amount
        );

        // Shielded transfer: nothing leaves the pool.
        let (note, anchor, merkle_path) = spendable_note(3_000, 2);
        insert_token_pool_anchor(&platform, token_id, &anchor);
        let extra =
            token_shielded_transfer_extra_sighash_data_v0(&token, &identity.id().to_buffer());
        let (transfer_bundle, value_balance) =
            build_spend_bundle(note, merkle_path, anchor, 0, &extra, 13);
        assert_eq!(value_balance, 0);

        let transfer = BatchTransition::new_token_shielded_transfer_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            transfer_bundle.clone(),
            &key,
            5,
            0,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("token shielded transfer transition");

        let result = process(&platform, &transfer);
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        assert_eq!(
            pool_balance(&platform, token_id),
            SHIELD_AMOUNT - unshield_amount
        );
        assert_eq!(
            pool_notes_count(&platform, token_id),
            (shield_bundle.actions.len() + 2 * transfer_bundle.actions.len()) as u64
        );
        for action in &transfer_bundle.actions {
            assert!(nullifier_is_spent(&platform, token_id, &action.nullifier));
        }
        assert_eq!(
            identity_token_balance(&platform, token_id, identity.id()),
            Some(OWNER_INITIAL_BALANCE - SHIELD_AMOUNT)
        );
        assert_tokens_conserved(&platform);
    }

    #[tokio::test]
    async fn test_token_shield_rejected_when_pool_not_enabled() {
        let platform_version = PlatformVersion::latest();
        let mut platform = platform_with_latest_version();
        let mut rng = StdRng::seed_from_u64(9002);

        let (identity, signer, key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            identity.id(),
            None::<fn(&mut TokenConfiguration)>,
            None,
            None,
            None,
            platform_version,
        );

        let shield = BatchTransition::new_token_shield_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            SHIELD_AMOUNT,
            dummy_bundle(),
            &key,
            2,
            0,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("token shield transition");

        let result = process(&platform, &shield);
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(StateError::TokenShieldedPoolNotEnabledError(_)),
                ..
            }]
        );
        assert_eq!(
            identity_token_balance(&platform, token_id, identity.id()),
            Some(OWNER_INITIAL_BALANCE)
        );
    }

    #[tokio::test]
    async fn test_token_shield_rejected_when_balance_too_low() {
        let platform_version = PlatformVersion::latest();
        let mut platform = platform_with_latest_version();
        let mut rng = StdRng::seed_from_u64(9003);

        let (identity, signer, key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            identity.id(),
            Some(enable_shielded_pool),
            None,
            None,
            None,
            platform_version,
        );

        let shield = BatchTransition::new_token_shield_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            OWNER_INITIAL_BALANCE + 1,
            dummy_bundle(),
            &key,
            2,
            0,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("token shield transition");

        let result = process(&platform, &shield);
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(
                    StateError::IdentityDoesNotHaveEnoughTokenBalanceError(_)
                ),
                ..
            }]
        );
        assert_eq!(pool_balance(&platform, token_id), 0);
    }

    #[tokio::test]
    async fn test_token_shield_rejected_when_token_paused() {
        let platform_version = PlatformVersion::latest();
        let mut platform = platform_with_latest_version();
        let mut rng = StdRng::seed_from_u64(9004);

        let (identity, signer, key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            identity.id(),
            Some(|configuration: &mut TokenConfiguration| {
                configuration.set_has_shielded_pool(true);
                configuration.set_start_as_paused(true);
            }),
            None,
            None,
            None,
            platform_version,
        );

        let shield = BatchTransition::new_token_shield_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            SHIELD_AMOUNT,
            dummy_bundle(),
            &key,
            2,
            0,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("token shield transition");

        let result = process(&platform, &shield);
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(StateError::TokenIsPausedError(_)),
                ..
            }]
        );
        assert_eq!(pool_balance(&platform, token_id), 0);
    }

    #[tokio::test]
    async fn test_token_shield_rejected_when_identity_frozen() {
        let platform_version = PlatformVersion::latest();
        let mut platform = platform_with_latest_version();
        let mut rng = StdRng::seed_from_u64(9005);

        let (identity, signer, key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            identity.id(),
            Some(enable_shielded_pool),
            None,
            None,
            None,
            platform_version,
        );

        platform
            .drive
            .token_freeze(
                token_id,
                identity.id(),
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("freeze identity token account");

        let shield = BatchTransition::new_token_shield_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            SHIELD_AMOUNT,
            dummy_bundle(),
            &key,
            2,
            0,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("token shield transition");

        let result = process(&platform, &shield);
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(StateError::IdentityTokenAccountFrozenError(_)),
                ..
            }]
        );
        assert_eq!(pool_balance(&platform, token_id), 0);
    }

    #[tokio::test]
    async fn test_token_shield_with_invalid_proof_is_a_paid_failure() {
        let platform_version = PlatformVersion::latest();
        let mut platform = platform_with_latest_version();
        let mut rng = StdRng::seed_from_u64(9006);

        let (identity, signer, key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            identity.id(),
            Some(enable_shielded_pool),
            None,
            None,
            None,
            platform_version,
        );

        let credits_before = platform
            .drive
            .fetch_identity_balance(identity.id().to_buffer(), None, platform_version)
            .expect("identity balance")
            .expect("identity exists");

        let shield = BatchTransition::new_token_shield_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            SHIELD_AMOUNT,
            dummy_bundle(),
            &key,
            2,
            0,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("token shield transition");

        let result = process(&platform, &shield);
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(StateError::InvalidShieldedProofError(_)),
                ..
            }]
        );

        // Nothing moved, but the identity paid for the verification attempt and its nonce
        // advanced, so the same transition cannot be resubmitted for free.
        assert_eq!(
            identity_token_balance(&platform, token_id, identity.id()),
            Some(OWNER_INITIAL_BALANCE)
        );
        assert_eq!(pool_balance(&platform, token_id), 0);
        let credits_after = platform
            .drive
            .fetch_identity_balance(identity.id().to_buffer(), None, platform_version)
            .expect("identity balance")
            .expect("identity exists");
        assert!(credits_after < credits_before);
        let nonce = platform
            .drive
            .fetch_identity_contract_nonce(
                identity.id().to_buffer(),
                contract.id().to_buffer(),
                true,
                None,
                platform_version,
            )
            .expect("identity contract nonce")
            .expect("nonce was set");
        // The stored value also carries the recent-nonce bitmap above the counter.
        assert_eq!(nonce & IDENTITY_NONCE_VALUE_FILTER, 2);
    }

    #[tokio::test]
    async fn test_token_unshield_rejected_for_unknown_anchor() {
        let platform_version = PlatformVersion::latest();
        let mut platform = platform_with_latest_version();
        let mut rng = StdRng::seed_from_u64(9007);

        let (identity, signer, key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (recipient, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.1));
        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            identity.id(),
            Some(enable_shielded_pool),
            None,
            None,
            None,
            platform_version,
        );

        let unshield = BatchTransition::new_token_unshield_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            1_000,
            recipient.id(),
            dummy_bundle(),
            &key,
            2,
            0,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("token unshield transition");

        let result = process(&platform, &unshield);
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(StateError::InvalidAnchorError(_)),
                ..
            }]
        );
        assert_eq!(
            identity_token_balance(&platform, token_id, recipient.id()),
            None
        );
    }

    #[tokio::test]
    async fn test_token_shield_rejected_before_protocol_version_14() {
        let platform_version =
            PlatformVersion::get(TOKEN_SHIELDED_POOL_INITIAL_PROTOCOL_VERSION - 1)
                .expect("previous protocol version");
        let mut platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(platform_version.protocol_version)
            .build_with_mock_rpc()
            .set_genesis_state();
        let mut rng = StdRng::seed_from_u64(9008);

        let (identity, signer, key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            identity.id(),
            None::<fn(&mut TokenConfiguration)>,
            None,
            None,
            None,
            platform_version,
        );

        let shield = BatchTransition::new_token_shield_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            SHIELD_AMOUNT,
            dummy_bundle(),
            &key,
            2,
            0,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("token shield transition");

        let platform_state = platform.state.load();
        let serialized = shield.serialize_to_bytes().expect("serialize");
        let transaction = platform.drive.grove.start_transaction();
        let result = platform
            .platform
            .process_raw_state_transitions(
                &[serialized],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("process state transition");

        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::UnpaidConsensusError(
                ConsensusError::BasicError(BasicError::StateTransitionNotActiveError(_))
            )]
        );
    }

    /// A contract carrying a token configuration with a shielded pool.
    fn shielded_token_contract(
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> DataContract {
        let data_contract_id = DataContract::generate_data_contract_id_v0(owner_id, 1);
        let mut contract = json_document_to_contract_with_ids(
            "tests/supporting_files/contract/basic-token/basic-token.json",
            Some(data_contract_id),
            Some(owner_id),
            false,
            platform_version,
        )
        .expect("basic token contract");
        contract
            .token_configuration_mut(0)
            .expect("token configuration")
            .set_has_shielded_pool(true);
        contract
    }

    #[tokio::test]
    async fn test_contract_create_with_shielded_pool_token_creates_the_pool() {
        let platform_version = PlatformVersion::latest();
        let mut platform = platform_with_latest_version();
        let mut rng = StdRng::seed_from_u64(9009);

        let (identity, signer, key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let contract = shielded_token_contract(identity.id(), platform_version);
        let token_id = contract.token_id(0).expect("token id");

        let create = DataContractCreateTransition::new_from_data_contract(
            contract,
            1,
            &identity.clone().into_partial_identity_info(),
            key.id(),
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("contract create transition");

        let result = process(&platform, &create);
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        assert_eq!(pool_balance(&platform, token_id), 0);
        assert_eq!(pool_notes_count(&platform, token_id), 0);
        assert_tokens_conserved(&platform);
    }

    #[tokio::test]
    async fn test_contract_create_with_shielded_pool_token_rejected_before_protocol_version_14() {
        let platform_version =
            PlatformVersion::get(TOKEN_SHIELDED_POOL_INITIAL_PROTOCOL_VERSION - 1)
                .expect("previous protocol version");
        let mut platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(platform_version.protocol_version)
            .build_with_mock_rpc()
            .set_genesis_state();
        let mut rng = StdRng::seed_from_u64(9010);

        let (identity, signer, key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let contract = shielded_token_contract(identity.id(), platform_version);

        let create = DataContractCreateTransition::new_from_data_contract(
            contract,
            1,
            &identity.clone().into_partial_identity_info(),
            key.id(),
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("contract create transition");

        let platform_state = platform.state.load();
        let serialized = create.serialize_to_bytes().expect("serialize");
        let transaction = platform.drive.grove.start_transaction();
        let result = platform
            .platform
            .process_raw_state_transitions(
                &[serialized],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("process state transition");

        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::UnpaidConsensusError(
                ConsensusError::BasicError(BasicError::UnsupportedVersionError(_))
            )]
        );
    }

    /// `hasShieldedPool` is a format-version-1 field, so clients see it in the contract JSON
    /// and it survives the round trip.
    #[test]
    fn test_token_configuration_shielded_pool_flag_round_trips_through_json() {
        let platform_version = PlatformVersion::latest();
        let contract = shielded_token_contract(Identifier::from([9u8; 32]), platform_version);
        let configuration = contract
            .expected_token_configuration(0)
            .expect("token configuration");
        let json = serde_json::to_value(configuration).expect("configuration json");
        assert_eq!(json.get("$formatVersion"), Some(&serde_json::json!("1")));
        assert_eq!(json.get("hasShieldedPool"), Some(&serde_json::json!(true)));
        let decoded: TokenConfiguration =
            serde_json::from_value(json).expect("configuration from json");
        assert_eq!(&decoded, configuration);
    }
}
