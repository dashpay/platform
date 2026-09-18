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
    pub(super) const OWNER_INITIAL_BALANCE: u64 = 100_000;
    pub(super) const SHIELD_AMOUNT: u64 = 10_000;

    pub(super) fn enable_shielded_pool(configuration: &mut TokenConfiguration) {
        configuration.set_has_shielded_pool(true);
    }

    pub(super) fn spend_keys() -> (FullViewingKey, SpendAuthorizingKey, PaymentAddress) {
        let sk = SpendingKey::from_bytes([7u8; 32]).unwrap();
        let fvk = FullViewingKey::from(&sk);
        let address = fvk.address_at(0u32, Scope::External);
        (fvk, SpendAuthorizingKey::from(&sk), address)
    }

    pub(super) fn bundle_params(
        bundle: &Bundle<Authorized, i64, DashMemo>,
    ) -> (OrchardBundleParams, i64) {
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
    pub(super) fn build_shield_bundle(amount: u64, seed: u64) -> OrchardBundleParams {
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
    pub(super) fn spendable_note(value: u64, tag: u8) -> (Note, Anchor, MerklePath) {
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
    pub(super) fn build_spend_bundle(
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
    pub(super) fn dummy_bundle() -> OrchardBundleParams {
        OrchardBundleParams {
            actions: vec![create_dummy_serialized_action()],
            anchor: [42u8; 32],
            proof: vec![0u8; 100],
            binding_signature: [0u8; 64],
        }
    }

    pub(super) fn insert_token_pool_anchor(
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

    pub(super) fn pool_balance(
        platform: &TempPlatform<MockCoreRPCLike>,
        token_id: Identifier,
    ) -> u64 {
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

    pub(super) fn pool_notes_count(
        platform: &TempPlatform<MockCoreRPCLike>,
        token_id: Identifier,
    ) -> u64 {
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

    pub(super) fn nullifier_is_spent(
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

    pub(super) fn identity_token_balance(
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

    pub(super) fn assert_tokens_conserved(platform: &TempPlatform<MockCoreRPCLike>) {
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
    pub(super) fn process(
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

    pub(super) fn platform_with_latest_version() -> TempPlatform<MockCoreRPCLike> {
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
    async fn test_token_shield_rejected_before_protocol_version_15() {
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
    async fn test_contract_create_with_shielded_pool_token_rejected_before_protocol_version_15() {
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

    #[tokio::test]
    async fn test_contract_create_with_shielded_pool_token_rejects_freeze_rules() {
        let platform_version = PlatformVersion::latest();
        let mut platform = platform_with_latest_version();
        let mut rng = StdRng::seed_from_u64(9011);

        let (identity, signer, key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let mut contract = shielded_token_contract(identity.id(), platform_version);
        contract
            .token_configuration_mut(0)
            .expect("token configuration")
            .set_freeze_rules(ChangeControlRules::V0(ChangeControlRulesV0 {
                authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
                admin_action_takers: AuthorizedActionTakers::NoOne,
                changing_authorized_action_takers_to_no_one_allowed: false,
                changing_admin_action_takers_to_no_one_allowed: false,
                self_changing_admin_action_takers_allowed: false,
            }));

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
            [StateTransitionExecutionResult::UnpaidConsensusError(
                ConsensusError::BasicError(BasicError::TokenShieldedPoolIncompatibleRulesError(
                    error
                ))
            )] if error.token_contract_position() == 0 && error.rule() == "freezeRules"
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

/// Mint, burn, claim and purchase straight into or out of the token shielded pool.
mod token_pool_mint_burn_claim_purchase_tests {
    use super::token_shielded_pool_tests::{
        assert_tokens_conserved, build_shield_bundle, build_spend_bundle, dummy_bundle,
        enable_shielded_pool, identity_token_balance, insert_token_pool_anchor, nullifier_is_spent,
        platform_with_latest_version, pool_balance, pool_notes_count, process, spendable_note,
        OWNER_INITIAL_BALANCE,
    };
    use super::*;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::fast_forward_to_block::fast_forward_to_block;
    use crate::test::helpers::setup::TempPlatform;
    use dpp::block::epoch::Epoch;
    use dpp::data_contract::associated_token::token_configuration::accessors::v1::TokenConfigurationV1Setters;
    use dpp::data_contract::associated_token::token_distribution_key::TokenDistributionType;
    use dpp::data_contract::associated_token::token_pre_programmed_distribution::v0::TokenPreProgrammedDistributionV0;
    use dpp::data_contract::associated_token::token_pre_programmed_distribution::TokenPreProgrammedDistribution;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::shielded::{serialized_actions_digest, token_burn_from_pool_extra_sighash_data_v0};
    use dpp::state_transition::batch_transition::{
        TokenBurnFromPoolTransition, TokenSetPriceForDirectPurchaseTransition,
    };
    use dpp::tokens::token_pricing_schedule::TokenPricingSchedule;
    use platform_version::version::PlatformVersion;

    fn total_supply(platform: &TempPlatform<MockCoreRPCLike>, token_id: Identifier) -> u64 {
        platform
            .drive
            .fetch_token_total_supply(token_id.to_buffer(), None, PlatformVersion::latest())
            .expect("total supply")
            .expect("supply exists")
    }

    #[tokio::test]
    async fn test_token_mint_to_pool_by_owner() {
        let platform_version = PlatformVersion::latest();
        let mut platform = platform_with_latest_version();
        let mut rng = StdRng::seed_from_u64(9101);

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

        let bundle = build_shield_bundle(1_337, 21);
        let mint = BatchTransition::new_token_mint_to_pool_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            1_337,
            bundle.clone(),
            Some("private issuance".to_string()),
            None,
            &key,
            2,
            0,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("token mint to pool transition");

        let result = process(&platform, &mint);
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        assert!(result
            .token_shielded_pools_touched()
            .contains(&token_id.to_buffer()));
        assert_eq!(pool_balance(&platform, token_id), 1_337);
        assert_eq!(
            pool_notes_count(&platform, token_id),
            bundle.actions.len() as u64
        );
        assert_eq!(
            total_supply(&platform, token_id),
            OWNER_INITIAL_BALANCE + 1_337
        );
        assert_eq!(
            identity_token_balance(&platform, token_id, identity.id()),
            Some(OWNER_INITIAL_BALANCE)
        );
        assert_tokens_conserved(&platform);
    }

    #[tokio::test]
    async fn test_token_mint_to_pool_rejected_past_max_supply() {
        let platform_version = PlatformVersion::latest();
        let mut platform = platform_with_latest_version();
        let mut rng = StdRng::seed_from_u64(9102);

        let (identity, signer, key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            identity.id(),
            Some(|configuration: &mut TokenConfiguration| {
                configuration.set_has_shielded_pool(true);
                configuration.set_max_supply(Some(OWNER_INITIAL_BALANCE + 100));
            }),
            None,
            None,
            None,
            platform_version,
        );

        let mint = BatchTransition::new_token_mint_to_pool_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            101,
            dummy_bundle(),
            None,
            None,
            &key,
            2,
            0,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("token mint to pool transition");

        let result = process(&platform, &mint);
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(StateError::TokenMintPastMaxSupplyError(_)),
                ..
            }]
        );
        assert_eq!(pool_balance(&platform, token_id), 0);
        assert_eq!(total_supply(&platform, token_id), OWNER_INITIAL_BALANCE);
    }

    #[tokio::test]
    async fn test_token_mint_to_pool_rejected_for_unauthorized_identity() {
        let platform_version = PlatformVersion::latest();
        let mut platform = platform_with_latest_version();
        let mut rng = StdRng::seed_from_u64(9103);

        let (owner, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (stranger, signer, key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            owner.id(),
            Some(enable_shielded_pool),
            None,
            None,
            None,
            platform_version,
        );

        let mint = BatchTransition::new_token_mint_to_pool_transition(
            token_id,
            stranger.id(),
            contract.id(),
            0,
            10,
            dummy_bundle(),
            None,
            None,
            &key,
            2,
            0,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("token mint to pool transition");

        let result = process(&platform, &mint);
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(StateError::UnauthorizedTokenActionError(_)),
                ..
            }]
        );
        assert_eq!(pool_balance(&platform, token_id), 0);
    }

    #[tokio::test]
    async fn test_token_burn_from_pool_after_shield() {
        let platform_version = PlatformVersion::latest();
        let mut platform = platform_with_latest_version();
        let mut rng = StdRng::seed_from_u64(9104);

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
        let token = token_id.to_buffer();

        // Fund the pool first.
        let shield_bundle = build_shield_bundle(10_000, 22);
        let shield = BatchTransition::new_token_shield_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            10_000,
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

        // Burn 4_000 of a 6_000 note the wallet holds; 2_000 comes back as change.
        let (note, anchor, merkle_path) = spendable_note(6_000, 3);
        insert_token_pool_anchor(&platform, token_id, &anchor);
        let burn_amount = 4_000;
        let extra = token_burn_from_pool_extra_sighash_data_v0(
            &token,
            &identity.id().to_buffer(),
            burn_amount,
        );
        let (burn_bundle, value_balance) =
            build_spend_bundle(note, merkle_path, anchor, burn_amount, &extra, 23);
        assert_eq!(value_balance, burn_amount as i64);

        let burn = BatchTransition::new_token_burn_from_pool_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            burn_amount,
            burn_bundle.clone(),
            None,
            None,
            &key,
            3,
            0,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("token burn from pool transition");

        let result = process(&platform, &burn);
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        assert_eq!(pool_balance(&platform, token_id), 10_000 - burn_amount);
        assert_eq!(
            total_supply(&platform, token_id),
            OWNER_INITIAL_BALANCE - burn_amount
        );
        assert_eq!(
            pool_notes_count(&platform, token_id),
            (shield_bundle.actions.len() + burn_bundle.actions.len()) as u64
        );
        assert_tokens_conserved(&platform);

        // The same notes cannot be burned twice.
        let replay = BatchTransition::new_token_burn_from_pool_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            burn_amount,
            burn_bundle,
            None,
            None,
            &key,
            4,
            0,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("token burn from pool transition");
        let result = process(&platform, &replay);
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(StateError::NullifierAlreadySpentError(_)),
                ..
            }]
        );
    }

    /// A burn from the pool authorized by a group of two. The proposer proves the bundle, bound
    /// to itself; the second signer submits that very bundle and the burn executes on the second
    /// signature. A bundle the second signer built itself is refused: the group action pins the
    /// digest of the proposer's actions.
    #[tokio::test]
    async fn test_token_burn_from_pool_by_group_of_two() {
        let platform_version = PlatformVersion::latest();
        let mut platform = platform_with_latest_version();
        let mut rng = StdRng::seed_from_u64(9105);

        let (proposer, proposer_signer, proposer_key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (confirmer, confirmer_signer, confirmer_key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            proposer.id(),
            Some(|token_configuration: &mut TokenConfiguration| {
                enable_shielded_pool(token_configuration);
                token_configuration.set_manual_burning_rules(ChangeControlRules::V0(
                    ChangeControlRulesV0 {
                        authorized_to_make_change: AuthorizedActionTakers::Group(0),
                        admin_action_takers: AuthorizedActionTakers::NoOne,
                        changing_authorized_action_takers_to_no_one_allowed: false,
                        changing_admin_action_takers_to_no_one_allowed: false,
                        self_changing_admin_action_takers_allowed: false,
                    },
                ));
            }),
            None,
            Some(
                [(
                    0,
                    Group::V0(GroupV0 {
                        members: [(proposer.id(), 1), (confirmer.id(), 1)].into(),
                        required_power: 2,
                    }),
                )]
                .into(),
            ),
            None,
            platform_version,
        );
        let token = token_id.to_buffer();

        // Fund the pool first.
        let shield = BatchTransition::new_token_shield_transition(
            token_id,
            proposer.id(),
            contract.id(),
            0,
            10_000,
            build_shield_bundle(10_000, 24),
            &proposer_key,
            2,
            0,
            &proposer_signer,
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

        // The proposer proves a burn of 4_000 out of a 6_000 note, bound to the proposer.
        let (note, anchor, merkle_path) = spendable_note(6_000, 4);
        insert_token_pool_anchor(&platform, token_id, &anchor);
        let burn_amount = 4_000;
        let extra = token_burn_from_pool_extra_sighash_data_v0(
            &token,
            &proposer.id().to_buffer(),
            burn_amount,
        );
        let (burn_bundle, value_balance) =
            build_spend_bundle(note, merkle_path, anchor, burn_amount, &extra, 25);
        assert_eq!(value_balance, burn_amount as i64);
        let proposer_nonce = 3;

        let proposal = BatchTransition::new_token_burn_from_pool_transition(
            token_id,
            proposer.id(),
            contract.id(),
            0,
            burn_amount,
            burn_bundle.clone(),
            None,
            Some(GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(0)),
            &proposer_key,
            proposer_nonce,
            0,
            &proposer_signer,
            platform_version,
            None,
        )
        .await
        .expect("token burn from pool proposal");
        let result = process(&platform, &proposal);
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        // One signature of two: nothing has left the pool yet.
        assert_eq!(pool_balance(&platform, token_id), 10_000);
        assert_eq!(total_supply(&platform, token_id), OWNER_INITIAL_BALANCE);
        assert!(!nullifier_is_spent(
            &platform,
            token_id,
            &burn_bundle.actions[0].nullifier
        ));

        let action_id = TokenBurnFromPoolTransition::calculate_action_id_with_fields(
            &token,
            proposer.id().as_bytes(),
            proposer_nonce,
            burn_amount,
            &serialized_actions_digest(&burn_bundle.actions),
        );
        let as_other_signer = || {
            Some(
                GroupStateTransitionInfoStatus::GroupStateTransitionInfoOtherSigner(
                    GroupStateTransitionInfo {
                        group_contract_position: 0,
                        action_id,
                        action_is_proposer: false,
                    },
                ),
            )
        };

        // The confirmer cannot substitute a bundle of its own, even for the same amount.
        let (other_note, other_anchor, other_path) = spendable_note(6_000, 5);
        insert_token_pool_anchor(&platform, token_id, &other_anchor);
        let confirmer_extra = token_burn_from_pool_extra_sighash_data_v0(
            &token,
            &confirmer.id().to_buffer(),
            burn_amount,
        );
        let (substituted_bundle, _) = build_spend_bundle(
            other_note,
            other_path,
            other_anchor,
            burn_amount,
            &confirmer_extra,
            26,
        );
        let substituted = BatchTransition::new_token_burn_from_pool_transition(
            token_id,
            confirmer.id(),
            contract.id(),
            0,
            burn_amount,
            substituted_bundle,
            None,
            as_other_signer(),
            &confirmer_key,
            2,
            0,
            &confirmer_signer,
            platform_version,
            None,
        )
        .await
        .expect("token burn from pool confirmation with a substituted bundle");
        let result = process(&platform, &substituted);
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(
                    StateError::ModificationOfGroupActionMainParametersNotPermittedError(_)
                ),
                ..
            }]
        );
        assert_eq!(pool_balance(&platform, token_id), 10_000);

        // The confirmer submits the proposer's bundle unchanged and the burn executes.
        let confirmation = BatchTransition::new_token_burn_from_pool_transition(
            token_id,
            confirmer.id(),
            contract.id(),
            0,
            burn_amount,
            burn_bundle.clone(),
            None,
            as_other_signer(),
            &confirmer_key,
            3,
            0,
            &confirmer_signer,
            platform_version,
            None,
        )
        .await
        .expect("token burn from pool confirmation");
        let result = process(&platform, &confirmation);
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        assert_eq!(pool_balance(&platform, token_id), 10_000 - burn_amount);
        assert_eq!(
            total_supply(&platform, token_id),
            OWNER_INITIAL_BALANCE - burn_amount
        );
        assert!(nullifier_is_spent(
            &platform,
            token_id,
            &burn_bundle.actions[0].nullifier
        ));
        assert_tokens_conserved(&platform);
    }

    #[tokio::test]
    async fn test_token_claim_to_pool_pre_programmed() {
        let platform_version = PlatformVersion::latest();
        let mut platform = platform_with_latest_version();
        let mut rng = StdRng::seed_from_u64(9105);

        let (owner, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (claimant, signer, key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let claimant_id = claimant.id();
        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            owner.id(),
            Some(move |configuration: &mut TokenConfiguration| {
                configuration.set_has_shielded_pool(true);
                configuration
                    .distribution_rules_mut()
                    .set_pre_programmed_distribution(Some(TokenPreProgrammedDistribution::V0(
                        TokenPreProgrammedDistributionV0 {
                            distributions: [(100, [(claimant_id, 445)].into())].into(),
                        },
                    )));
            }),
            None,
            None,
            None,
            platform_version,
        );

        fast_forward_to_block(&platform, 100, 40, 42, 1, false);
        let platform_state = platform.state.load();
        let block_info = BlockInfo {
            time_ms: 200,
            height: 41,
            core_height: 42,
            epoch: Epoch::new(1).unwrap(),
        };

        // The bundle must carry exactly the release (445); a wrong amount is a paid failure.
        let wrong = BatchTransition::new_token_claim_to_pool_transition(
            token_id,
            claimant_id,
            contract.id(),
            0,
            TokenDistributionType::PreProgrammed,
            None,
            build_shield_bundle(444, 24),
            None,
            &key,
            2,
            0,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("token claim to pool transition");
        let transaction = platform.drive.grove.start_transaction();
        let result = platform
            .platform
            .process_raw_state_transitions(
                &[wrong.serialize_to_bytes().expect("serialize")],
                &platform_state,
                &block_info,
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("process state transition");
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(StateError::InvalidShieldedProofError(_)),
                ..
            }]
        );
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("commit");

        let claim_bundle = build_shield_bundle(445, 25);
        let claim = BatchTransition::new_token_claim_to_pool_transition(
            token_id,
            claimant_id,
            contract.id(),
            0,
            TokenDistributionType::PreProgrammed,
            None,
            claim_bundle.clone(),
            None,
            &key,
            3,
            0,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("token claim to pool transition");
        let transaction = platform.drive.grove.start_transaction();
        let result = platform
            .platform
            .process_raw_state_transitions(
                &[claim.serialize_to_bytes().expect("serialize")],
                &platform_state,
                &block_info,
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("process state transition");
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("commit");

        assert_eq!(pool_balance(&platform, token_id), 445);
        assert_eq!(
            identity_token_balance(&platform, token_id, claimant_id),
            None
        );
        assert_eq!(
            total_supply(&platform, token_id),
            OWNER_INITIAL_BALANCE + 445
        );
        assert_eq!(
            pool_notes_count(&platform, token_id),
            claim_bundle.actions.len() as u64
        );
        assert_tokens_conserved(&platform);

        // The release is marked distributed: claiming it again finds nothing to claim.
        let again = BatchTransition::new_token_claim_to_pool_transition(
            token_id,
            claimant_id,
            contract.id(),
            0,
            TokenDistributionType::PreProgrammed,
            None,
            build_shield_bundle(445, 26),
            None,
            &key,
            4,
            0,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("token claim to pool transition");
        let transaction = platform.drive.grove.start_transaction();
        let result = platform
            .platform
            .process_raw_state_transitions(
                &[again.serialize_to_bytes().expect("serialize")],
                &platform_state,
                &block_info,
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("process state transition");
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(StateError::InvalidTokenClaimNoCurrentRewards(_)),
                ..
            }]
        );
    }

    #[tokio::test]
    async fn test_token_direct_purchase_to_pool() {
        let platform_version = PlatformVersion::latest();
        let mut platform = platform_with_latest_version();
        let mut rng = StdRng::seed_from_u64(9106);

        let (seller, seller_signer, seller_key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (buyer, signer, key) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(1.0));
        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            seller.id(),
            Some(|configuration: &mut TokenConfiguration| {
                configuration.set_has_shielded_pool(true);
                configuration
                    .distribution_rules_mut()
                    .set_change_direct_purchase_pricing_rules(ChangeControlRules::V0(
                        ChangeControlRulesV0 {
                            authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
                            admin_action_takers: AuthorizedActionTakers::NoOne,
                            changing_authorized_action_takers_to_no_one_allowed: false,
                            changing_admin_action_takers_to_no_one_allowed: false,
                            self_changing_admin_action_takers_allowed: false,
                        },
                    ));
            }),
            None,
            None,
            None,
            platform_version,
        );

        let set_price = BatchTransition::new_token_change_direct_purchase_price_transition(
            token_id,
            seller.id(),
            contract.id(),
            0,
            Some(TokenPricingSchedule::SinglePrice(dash_to_credits!(0.01))),
            None,
            None,
            &seller_key,
            2,
            0,
            &seller_signer,
            platform_version,
            None,
        )
        .await
        .expect("set price transition");
        let result = process(&platform, &set_price);
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );

        let seller_credits_before = platform
            .drive
            .fetch_identity_balance(seller.id().to_buffer(), None, platform_version)
            .expect("balance")
            .expect("seller exists");

        let purchase_bundle = build_shield_bundle(3, 27);
        let purchase = BatchTransition::new_token_direct_purchase_to_pool_transition(
            token_id,
            buyer.id(),
            contract.id(),
            0,
            3,
            dash_to_credits!(0.03),
            purchase_bundle.clone(),
            &key,
            2,
            0,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("token direct purchase to pool transition");
        let result = process(&platform, &purchase);
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );

        assert_eq!(pool_balance(&platform, token_id), 3);
        assert_eq!(
            identity_token_balance(&platform, token_id, buyer.id()),
            None
        );
        assert_eq!(total_supply(&platform, token_id), OWNER_INITIAL_BALANCE + 3);
        assert_eq!(
            pool_notes_count(&platform, token_id),
            purchase_bundle.actions.len() as u64
        );
        let seller_credits_after = platform
            .drive
            .fetch_identity_balance(seller.id().to_buffer(), None, platform_version)
            .expect("balance")
            .expect("seller exists");
        assert_eq!(
            seller_credits_after,
            seller_credits_before + dash_to_credits!(0.03)
        );
        assert_tokens_conserved(&platform);

        // Underpaying is rejected before any note is created.
        let cheap = BatchTransition::new_token_direct_purchase_to_pool_transition(
            token_id,
            buyer.id(),
            contract.id(),
            0,
            3,
            dash_to_credits!(0.02),
            build_shield_bundle(3, 28),
            &key,
            3,
            0,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("token direct purchase to pool transition");
        let result = process(&platform, &cheap);
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(StateError::TokenDirectPurchaseUserPriceTooLow(
                    _
                )),
                ..
            }]
        );
        assert_eq!(pool_balance(&platform, token_id), 3);
    }
}

/// Documents whose token cost is paid out of the token's shielded pool (`TokenPaymentInfo::V1`).
///
/// The card game contract charges 10 of its token 0 to create a `card`, burned or paid to the
/// contract owner depending on the fixture. The buyer shields the tokens first, then pays the
/// cost with a spend bundle bound to the token, the buyer, the contract and the document id.
mod document_shielded_token_payment_tests {
    use super::token_shielded_pool_tests::{
        assert_tokens_conserved, build_shield_bundle, build_spend_bundle, dummy_bundle,
        identity_token_balance, insert_token_pool_anchor, nullifier_is_spent,
        platform_with_latest_version, pool_balance, process, spendable_note,
    };
    use super::*;
    use crate::execution::validation::state_transition::state_transitions::tests::add_tokens_to_identity;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TempPlatform;
    use dpp::data_contract::accessors::v0::DataContractV0Setters;
    use dpp::data_contract::accessors::v1::DataContractV1Setters;
    use dpp::data_contract::associated_token::token_configuration::accessors::v1::TokenConfigurationV1Setters;
    use dpp::data_contract::document_type::DocumentTypeRef;
    use dpp::data_contract::DataContract;
    use dpp::document::{Document, DocumentV0Getters};
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::{Identity, IdentityPublicKey};
    use dpp::shielded::{document_token_payment_extra_sighash_data_v0, OrchardBundleParams};
    use dpp::tokens::calculate_token_id;
    use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
    use dpp::tokens::token_payment_info::v1::{TokenPaymentInfoV1, TokenShieldedPayment};
    use dpp::tokens::token_payment_info::TokenPaymentInfo;
    use dpp::version::feature_initial_protocol_versions::TOKEN_SHIELDED_POOL_INITIAL_PROTOCOL_VERSION;
    use drive::util::test_helpers::setup_contract;
    use platform_version::version::PlatformVersion;
    use simple_signer::signer::SimpleSigner;

    const CARD_COST: u64 = 10;
    const SHIELDED: u64 = 15;

    /// The card game contract with an in-game currency: creating a `card` costs `CARD_COST` of
    /// token 0, burned (`burn`) or paid to the contract owner. `pool` gives token 0 a shielded pool.
    fn card_game_contract(
        platform: &TempPlatform<MockCoreRPCLike>,
        owner_id: Identifier,
        burn: bool,
        pool: bool,
        platform_version: &PlatformVersion,
    ) -> (DataContract, Identifier) {
        let data_contract_id = DataContract::generate_data_contract_id_v0(owner_id, 1);
        let path = if burn {
            "tests/supporting_files/contract/crypto-card-game/crypto-card-game-in-game-currency-burn-tokens.json"
        } else {
            "tests/supporting_files/contract/crypto-card-game/crypto-card-game-in-game-currency.json"
        };
        let contract = setup_contract(
            &platform.drive,
            path,
            Some(data_contract_id.to_buffer()),
            Some(owner_id.to_buffer()),
            Some(|data_contract: &mut DataContract| {
                data_contract.set_created_at_epoch(Some(0));
                data_contract.set_created_at(Some(0));
                data_contract.set_created_at_block_height(Some(0));
                if pool {
                    let configuration = data_contract
                        .tokens_mut()
                        .and_then(|tokens| tokens.get_mut(&0))
                        .expect("token 0");
                    configuration.set_has_shielded_pool(true);
                }
            }),
            None,
            Some(platform_version),
        );
        (
            contract,
            calculate_token_id(data_contract_id.as_bytes(), 0).into(),
        )
    }

    fn random_card(
        rng: &mut StdRng,
        card_document_type: DocumentTypeRef,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> (Document, Bytes32) {
        let entropy = Bytes32::random_with_rng(rng);
        let mut document = card_document_type
            .random_document_with_identifier_and_entropy(
                rng,
                owner_id,
                entropy,
                DocumentFieldFillType::DoNotFillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");
        document.set("attack", 4.into());
        document.set("defense", 7.into());
        (document, entropy)
    }

    fn shielded_payment(bundle: OrchardBundleParams, amount: u64) -> TokenShieldedPayment {
        TokenShieldedPayment {
            amount,
            actions: bundle.actions,
            anchor: bundle.anchor,
            proof: bundle.proof,
            binding_signature: bundle.binding_signature,
        }
    }

    fn payment_info(shielded_payment: TokenShieldedPayment) -> TokenPaymentInfo {
        TokenPaymentInfo::V1(TokenPaymentInfoV1 {
            payment_token_contract_id: None,
            token_contract_position: 0,
            minimum_token_cost: None,
            maximum_token_cost: Some(CARD_COST),
            gas_fees_paid_by: GasFeesPaidBy::DocumentOwner,
            shielded_payment: Box::new(shielded_payment),
        })
    }

    fn total_supply(platform: &TempPlatform<MockCoreRPCLike>, token_id: Identifier) -> u64 {
        platform
            .drive
            .fetch_token_total_supply(token_id.to_buffer(), None, PlatformVersion::latest())
            .expect("total supply")
            .expect("supply exists")
    }

    /// Funds the buyer with `SHIELDED` tokens and shields all of them (identity contract nonce 1).
    async fn fund_and_shield(
        platform: &mut TempPlatform<MockCoreRPCLike>,
        contract: &DataContract,
        token_id: Identifier,
        buyer: &Identity,
        key: &IdentityPublicKey,
        signer: &SimpleSigner,
        seed: u64,
        platform_version: &PlatformVersion,
    ) {
        add_tokens_to_identity(platform, token_id, buyer.id(), SHIELDED);
        let shield = BatchTransition::new_token_shield_transition(
            token_id,
            buyer.id(),
            contract.id(),
            0,
            SHIELDED,
            build_shield_bundle(SHIELDED, seed),
            key,
            1,
            0,
            signer,
            platform_version,
            None,
        )
        .await
        .expect("token shield transition");
        let result = process(platform, &shield);
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        assert_eq!(pool_balance(platform, token_id), SHIELDED);
    }

    #[tokio::test]
    async fn test_document_creation_paid_from_the_shielded_pool_with_a_burn() {
        let platform_version = PlatformVersion::latest();
        let mut platform = platform_with_latest_version();
        let mut rng = StdRng::seed_from_u64(9301);

        let (contract_owner, _, _) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));
        let (buyer, signer, key) = setup_identity(&mut platform, 234, dash_to_credits!(0.5));
        let (contract, token_id) =
            card_game_contract(&platform, contract_owner.id(), true, true, platform_version);
        fund_and_shield(
            &mut platform,
            &contract,
            token_id,
            &buyer,
            &key,
            &signer,
            41,
            platform_version,
        )
        .await;

        let card_document_type = contract
            .document_type_for_name("card")
            .expect("card document type");
        let (document, entropy) =
            random_card(&mut rng, card_document_type, buyer.id(), platform_version);

        // A 15 note the wallet holds pays the 10 cost; 5 return to the pool as change.
        let (note, anchor, merkle_path) = spendable_note(SHIELDED, 4);
        insert_token_pool_anchor(&platform, token_id, &anchor);
        let extra = document_token_payment_extra_sighash_data_v0(
            &token_id.to_buffer(),
            &buyer.id().to_buffer(),
            &contract.id().to_buffer(),
            &document.id().to_buffer(),
            CARD_COST,
        );
        let (bundle, value_balance) =
            build_spend_bundle(note, merkle_path, anchor, CARD_COST, &extra, 42);
        assert_eq!(value_balance, CARD_COST as i64);
        let payment = shielded_payment(bundle, CARD_COST);

        let transition = BatchTransition::new_document_creation_transition_from_document(
            document,
            card_document_type,
            entropy.0,
            &key,
            2,
            0,
            Some(payment_info(payment.clone())),
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("document create transition");

        let result = process(&platform, &transition);
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );

        // The cost left the pool and the supply; the buyer's balance was never touched.
        assert_eq!(pool_balance(&platform, token_id), SHIELDED - CARD_COST);
        assert_eq!(total_supply(&platform, token_id), SHIELDED - CARD_COST);
        assert_eq!(
            identity_token_balance(&platform, token_id, buyer.id()).unwrap_or_default(),
            0
        );
        assert_eq!(
            identity_token_balance(&platform, token_id, contract_owner.id()),
            None
        );
        assert!(nullifier_is_spent(
            &platform,
            token_id,
            &payment.actions[0].nullifier
        ));
        assert_tokens_conserved(&platform);

        // The spent notes cannot pay for another document.
        let (replay_document, replay_entropy) =
            random_card(&mut rng, card_document_type, buyer.id(), platform_version);
        let replay = BatchTransition::new_document_creation_transition_from_document(
            replay_document,
            card_document_type,
            replay_entropy.0,
            &key,
            3,
            0,
            Some(payment_info(payment)),
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("document create transition");
        let result = process(&platform, &replay);
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(StateError::NullifierAlreadySpentError(_)),
                ..
            }]
        );
        assert_eq!(pool_balance(&platform, token_id), SHIELDED - CARD_COST);
        assert_tokens_conserved(&platform);
    }

    #[tokio::test]
    async fn test_document_creation_paid_from_the_shielded_pool_to_the_contract_owner() {
        let platform_version = PlatformVersion::latest();
        let mut platform = platform_with_latest_version();
        let mut rng = StdRng::seed_from_u64(9302);

        let (contract_owner, _, _) = setup_identity(&mut platform, 959, dash_to_credits!(0.1));
        let (buyer, signer, key) = setup_identity(&mut platform, 235, dash_to_credits!(0.5));
        let (contract, token_id) = card_game_contract(
            &platform,
            contract_owner.id(),
            false,
            true,
            platform_version,
        );
        fund_and_shield(
            &mut platform,
            &contract,
            token_id,
            &buyer,
            &key,
            &signer,
            43,
            platform_version,
        )
        .await;

        let card_document_type = contract
            .document_type_for_name("card")
            .expect("card document type");
        let (document, entropy) =
            random_card(&mut rng, card_document_type, buyer.id(), platform_version);

        let (note, anchor, merkle_path) = spendable_note(SHIELDED, 5);
        insert_token_pool_anchor(&platform, token_id, &anchor);
        let extra = document_token_payment_extra_sighash_data_v0(
            &token_id.to_buffer(),
            &buyer.id().to_buffer(),
            &contract.id().to_buffer(),
            &document.id().to_buffer(),
            CARD_COST,
        );
        let (bundle, _) = build_spend_bundle(note, merkle_path, anchor, CARD_COST, &extra, 44);

        let transition = BatchTransition::new_document_creation_transition_from_document(
            document,
            card_document_type,
            entropy.0,
            &key,
            2,
            0,
            Some(payment_info(shielded_payment(bundle, CARD_COST))),
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("document create transition");

        let result = process(&platform, &transition);
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );

        // The cost moved from the pool into the contract owner's balance; the supply is unchanged.
        assert_eq!(pool_balance(&platform, token_id), SHIELDED - CARD_COST);
        assert_eq!(total_supply(&platform, token_id), SHIELDED);
        assert_eq!(
            identity_token_balance(&platform, token_id, contract_owner.id()),
            Some(CARD_COST)
        );
        assert_tokens_conserved(&platform);
    }

    #[tokio::test]
    async fn test_document_creation_shielded_payment_must_match_the_token_cost() {
        let platform_version = PlatformVersion::latest();
        let mut platform = platform_with_latest_version();
        let mut rng = StdRng::seed_from_u64(9303);

        let (contract_owner, _, _) = setup_identity(&mut platform, 960, dash_to_credits!(0.1));
        let (buyer, signer, key) = setup_identity(&mut platform, 236, dash_to_credits!(0.5));
        let (contract, token_id) =
            card_game_contract(&platform, contract_owner.id(), true, true, platform_version);
        fund_and_shield(
            &mut platform,
            &contract,
            token_id,
            &buyer,
            &key,
            &signer,
            45,
            platform_version,
        )
        .await;

        let card_document_type = contract
            .document_type_for_name("card")
            .expect("card document type");
        let (document, entropy) =
            random_card(&mut rng, card_document_type, buyer.id(), platform_version);

        // A bundle proving 9 where the card costs 10.
        let underpaid = CARD_COST - 1;
        let (note, anchor, merkle_path) = spendable_note(SHIELDED, 6);
        insert_token_pool_anchor(&platform, token_id, &anchor);
        let extra = document_token_payment_extra_sighash_data_v0(
            &token_id.to_buffer(),
            &buyer.id().to_buffer(),
            &contract.id().to_buffer(),
            &document.id().to_buffer(),
            underpaid,
        );
        let (bundle, _) = build_spend_bundle(note, merkle_path, anchor, underpaid, &extra, 46);
        let payment = shielded_payment(bundle, underpaid);

        let transition = BatchTransition::new_document_creation_transition_from_document(
            document,
            card_document_type,
            entropy.0,
            &key,
            2,
            0,
            Some(payment_info(payment.clone())),
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("document create transition");

        let result = process(&platform, &transition);
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(
                    StateError::TokenShieldedPaymentAmountMismatchError(_)
                ),
                ..
            }]
        );
        // Nothing left the pool and the notes are still unspent.
        assert_eq!(pool_balance(&platform, token_id), SHIELDED);
        assert_eq!(total_supply(&platform, token_id), SHIELDED);
        assert!(!nullifier_is_spent(
            &platform,
            token_id,
            &payment.actions[0].nullifier
        ));
        assert_tokens_conserved(&platform);
    }

    #[tokio::test]
    async fn test_document_shielded_payment_rejected_before_protocol_version_15() {
        let platform_version =
            PlatformVersion::get(TOKEN_SHIELDED_POOL_INITIAL_PROTOCOL_VERSION - 1)
                .expect("previous protocol version");
        let mut platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(platform_version.protocol_version)
            .build_with_mock_rpc()
            .set_genesis_state();
        let mut rng = StdRng::seed_from_u64(9304);

        let (contract_owner, _, _) = setup_identity(&mut platform, 961, dash_to_credits!(0.1));
        let (buyer, signer, key) = setup_identity(&mut platform, 237, dash_to_credits!(0.5));
        // No pool: a pooled token cannot exist before the pools root does.
        let (contract, _token_id) = card_game_contract(
            &platform,
            contract_owner.id(),
            true,
            false,
            platform_version,
        );

        let card_document_type = contract
            .document_type_for_name("card")
            .expect("card document type");
        let (document, entropy) =
            random_card(&mut rng, card_document_type, buyer.id(), platform_version);

        let transition = BatchTransition::new_document_creation_transition_from_document(
            document,
            card_document_type,
            entropy.0,
            &key,
            1,
            0,
            Some(payment_info(shielded_payment(dummy_bundle(), CARD_COST))),
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("document create transition");

        let platform_state = platform.state.load();
        let serialized = transition.serialize_to_bytes().expect("serialize");
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
}

/// The identity-less token pool transitions (types 23, 24, 25): a bundle in the token's pool
/// and a fee bundle in the credit pool, no identity anywhere. The test wallet owns notes in
/// both pools; the credit pool is funded directly in state, the token pool through a shield.
mod token_pool_paid_transitions_tests {
    use super::token_shielded_pool_tests::{
        assert_tokens_conserved, build_shield_bundle, dummy_bundle, enable_shielded_pool,
        identity_token_balance, insert_token_pool_anchor, nullifier_is_spent,
        platform_with_latest_version, pool_balance, process, spend_keys, spendable_note,
        OWNER_INITIAL_BALANCE,
    };
    use super::*;
    use crate::execution::validation::state_transition::state_transitions::shielded_common::read_pool_total_balance;
    use crate::execution::validation::state_transition::state_transitions::test_helpers::{
        get_proving_key, insert_anchor_into_state, set_pool_total_balance,
    };
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TempPlatform;
    use dpp::address_funds::OrchardAddress;
    use dpp::data_contract::associated_token::token_configuration::accessors::v1::TokenConfigurationV1Setters;
    use dpp::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
    use dpp::data_contract::change_control_rules::v0::ChangeControlRulesV0;
    use dpp::data_contract::change_control_rules::ChangeControlRules;
    use dpp::data_contract::DataContract;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::{Identity, IdentityPublicKey};
    use dpp::shielded::builder::{
        build_token_purchase_from_shielded_pool_transition,
        build_token_shielded_transfer_with_shielded_fee_transition,
        build_token_unshield_with_shielded_fee_transition, OrchardProver, ShieldedFeePayer,
        SpendableNote, TokenPoolSpender,
    };
    use dpp::state_transition::token_purchase_from_shielded_pool_transition::v0::TokenPurchaseFromShieldedPoolTransitionV0;
    use dpp::state_transition::token_purchase_from_shielded_pool_transition::TokenPurchaseFromShieldedPoolTransition;
    use dpp::state_transition::StateTransition;
    use dpp::tokens::token_pricing_schedule::TokenPricingSchedule;
    use dpp::version::feature_initial_protocol_versions::TOKEN_SHIELDED_POOL_INITIAL_PROTOCOL_VERSION;
    use grovedb_commitment_tree::{Anchor, MerklePath, Note, ProvingKey};
    use platform_version::version::PlatformVersion;
    use simple_signer::signer::SimpleSigner;

    /// A credit pool note the test wallet holds: enough for a purchase at 0.01 DASH a token.
    const CREDIT_NOTE: u64 = 5_000_000_000;
    const SHIELDED: u64 = 15;

    struct Prover;

    impl OrchardProver for Prover {
        fn proving_key(&self) -> &ProvingKey {
            get_proving_key()
        }
    }

    fn wallet_address() -> OrchardAddress {
        let (_, _, address) = spend_keys();
        OrchardAddress::from_raw_bytes(&address.to_raw_address_bytes())
            .expect("valid orchard address bytes")
    }

    fn spendable(note: Note, merkle_path: MerklePath) -> SpendableNote {
        SpendableNote { note, merkle_path }
    }

    /// Puts a `CREDIT_NOTE` note of the test wallet into the credit pool: its anchor is
    /// recorded and the pool total covers it.
    fn fund_credit_pool(
        platform: &TempPlatform<MockCoreRPCLike>,
        tag: u8,
    ) -> (Note, Anchor, MerklePath) {
        let (note, anchor, merkle_path) = spendable_note(CREDIT_NOTE, tag);
        insert_anchor_into_state(platform, &anchor.to_bytes());
        set_pool_total_balance(platform, CREDIT_NOTE);
        (note, anchor, merkle_path)
    }

    fn credit_pool_balance(platform: &TempPlatform<MockCoreRPCLike>) -> u64 {
        read_pool_total_balance(
            &platform.drive,
            None,
            &mut vec![],
            PlatformVersion::latest(),
        )
        .expect("credit pool balance")
    }

    /// Funds the buyer with `SHIELDED` tokens and shields all of them (identity contract nonce
    /// 2, the contract creation used 1), then records an anchor for a `SHIELDED` note of the
    /// test wallet in the token pool.
    async fn fund_token_pool(
        platform: &mut TempPlatform<MockCoreRPCLike>,
        contract: &DataContract,
        token_id: Identifier,
        holder: &Identity,
        key: &IdentityPublicKey,
        signer: &SimpleSigner,
        seed: u64,
        tag: u8,
        platform_version: &PlatformVersion,
    ) -> (Note, Anchor, MerklePath) {
        let shield = BatchTransition::new_token_shield_transition(
            token_id,
            holder.id(),
            contract.id(),
            0,
            SHIELDED,
            build_shield_bundle(SHIELDED, seed),
            key,
            2,
            0,
            signer,
            platform_version,
            None,
        )
        .await
        .expect("token shield transition");
        let result = process(platform, &shield);
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        assert_eq!(pool_balance(platform, token_id), SHIELDED);
        let (note, anchor, merkle_path) = spendable_note(SHIELDED, tag);
        insert_token_pool_anchor(platform, token_id, &anchor);
        (note, anchor, merkle_path)
    }

    #[tokio::test]
    async fn test_token_shielded_transfer_with_shielded_fee() {
        let platform_version = PlatformVersion::latest();
        let mut platform = platform_with_latest_version();
        let mut rng = StdRng::seed_from_u64(9401);

        let (owner, signer, key) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            owner.id(),
            Some(enable_shielded_pool),
            None,
            None,
            None,
            platform_version,
        );
        let (token_note, token_anchor, token_path) = fund_token_pool(
            &mut platform,
            &contract,
            token_id,
            &owner,
            &key,
            &signer,
            51,
            11,
            platform_version,
        )
        .await;
        let (credit_note, credit_anchor, credit_path) = fund_credit_pool(&platform, 12);
        let (fvk, ask, _) = spend_keys();
        let address = wallet_address();

        let (transition, fee) = build_token_shielded_transfer_with_shielded_fee_transition(
            token_id,
            contract.id(),
            0,
            TokenPoolSpender {
                spends: vec![spendable(token_note, token_path)],
                change_address: &address,
                fvk: &fvk,
                ask: &ask,
                anchor: token_anchor,
            },
            &address,
            6,
            [0u8; 36],
            ShieldedFeePayer {
                spends: vec![spendable(credit_note, credit_path)],
                change_address: &address,
                fvk: &fvk,
                ask: &ask,
                anchor: credit_anchor,
            },
            &Prover,
            platform_version,
        )
        .expect("build transition");
        assert!(fee > 0);

        let result = process(&platform, &transition);
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );

        // Nothing left the token pool; the fee left the credit pool; both spends are final.
        assert_eq!(pool_balance(&platform, token_id), SHIELDED);
        assert_eq!(credit_pool_balance(&platform), CREDIT_NOTE - fee);
        assert_eq!(
            identity_token_balance(&platform, token_id, owner.id()),
            Some(OWNER_INITIAL_BALANCE - SHIELDED)
        );
        let StateTransition::TokenShieldedTransferWithShieldedFee(inner) = &transition else {
            unreachable!()
        };
        use dpp::state_transition::token_shielded_transfer_with_shielded_fee_transition::accessors::TokenShieldedTransferWithShieldedFeeTransitionAccessorsV0;
        assert!(nullifier_is_spent(
            &platform,
            token_id,
            &inner.token_actions()[0].nullifier
        ));
        assert_tokens_conserved(&platform);

        // The same notes cannot be spent twice; the rejection is unpaid (no identity).
        let result = process(&platform, &transition);
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::UnpaidConsensusError(
                ConsensusError::StateError(StateError::NullifierAlreadySpentError(_))
            )]
        );
    }

    #[tokio::test]
    async fn test_token_unshield_with_shielded_fee() {
        let platform_version = PlatformVersion::latest();
        let mut platform = platform_with_latest_version();
        let mut rng = StdRng::seed_from_u64(9402);

        let (owner, signer, key) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (recipient, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.1));
        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            owner.id(),
            Some(enable_shielded_pool),
            None,
            None,
            None,
            platform_version,
        );
        let (token_note, token_anchor, token_path) = fund_token_pool(
            &mut platform,
            &contract,
            token_id,
            &owner,
            &key,
            &signer,
            52,
            13,
            platform_version,
        )
        .await;
        let (credit_note, credit_anchor, credit_path) = fund_credit_pool(&platform, 14);
        let (fvk, ask, _) = spend_keys();
        let address = wallet_address();

        let (transition, fee) = build_token_unshield_with_shielded_fee_transition(
            token_id,
            contract.id(),
            0,
            TokenPoolSpender {
                spends: vec![spendable(token_note, token_path)],
                change_address: &address,
                fvk: &fvk,
                ask: &ask,
                anchor: token_anchor,
            },
            recipient.id(),
            10,
            [0u8; 36],
            ShieldedFeePayer {
                spends: vec![spendable(credit_note, credit_path)],
                change_address: &address,
                fvk: &fvk,
                ask: &ask,
                anchor: credit_anchor,
            },
            &Prover,
            platform_version,
        )
        .expect("build transition");

        let result = process(&platform, &transition);
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );

        assert_eq!(pool_balance(&platform, token_id), SHIELDED - 10);
        assert_eq!(
            identity_token_balance(&platform, token_id, recipient.id()),
            Some(10)
        );
        assert_eq!(credit_pool_balance(&platform), CREDIT_NOTE - fee);
        assert_tokens_conserved(&platform);
    }

    #[tokio::test]
    async fn test_token_purchase_from_shielded_pool() {
        let platform_version = PlatformVersion::latest();
        let mut platform = platform_with_latest_version();
        let mut rng = StdRng::seed_from_u64(9403);

        let (seller, seller_signer, seller_key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            seller.id(),
            Some(|configuration: &mut TokenConfiguration| {
                configuration.set_has_shielded_pool(true);
                configuration
                    .distribution_rules_mut()
                    .set_change_direct_purchase_pricing_rules(ChangeControlRules::V0(
                        ChangeControlRulesV0 {
                            authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
                            admin_action_takers: AuthorizedActionTakers::NoOne,
                            changing_authorized_action_takers_to_no_one_allowed: false,
                            changing_admin_action_takers_to_no_one_allowed: false,
                            self_changing_admin_action_takers_allowed: false,
                        },
                    ));
            }),
            None,
            None,
            None,
            platform_version,
        );
        let price_per_token = dash_to_credits!(0.01);
        let set_price = BatchTransition::new_token_change_direct_purchase_price_transition(
            token_id,
            seller.id(),
            contract.id(),
            0,
            Some(TokenPricingSchedule::SinglePrice(price_per_token)),
            None,
            None,
            &seller_key,
            2,
            0,
            &seller_signer,
            platform_version,
            None,
        )
        .await
        .expect("set price transition");
        let result = process(&platform, &set_price);
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        let seller_credits_before = platform
            .drive
            .fetch_identity_balance(seller.id().to_buffer(), None, platform_version)
            .expect("balance")
            .expect("seller exists");

        let (credit_note, credit_anchor, credit_path) = fund_credit_pool(&platform, 15);
        let (fvk, ask, _) = spend_keys();
        let address = wallet_address();
        let token_count = 3;
        let price = price_per_token * token_count;

        let (transition, fee) = build_token_purchase_from_shielded_pool_transition(
            token_id,
            contract.id(),
            0,
            &address,
            &fvk,
            token_count,
            price,
            [0u8; 36],
            ShieldedFeePayer {
                spends: vec![spendable(credit_note, credit_path)],
                change_address: &address,
                fvk: &fvk,
                ask: &ask,
                anchor: credit_anchor,
            },
            &Prover,
            platform_version,
        )
        .expect("build transition");

        let result = process(&platform, &transition);
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );

        // The tokens were minted into the pool, the price reached the seller, the fee and the
        // price left the credit pool.
        assert_eq!(pool_balance(&platform, token_id), token_count);
        let total_supply = platform
            .drive
            .fetch_token_total_supply(token_id.to_buffer(), None, platform_version)
            .expect("total supply")
            .expect("supply exists");
        assert_eq!(total_supply, OWNER_INITIAL_BALANCE + token_count);
        let seller_credits_after = platform
            .drive
            .fetch_identity_balance(seller.id().to_buffer(), None, platform_version)
            .expect("balance")
            .expect("seller exists");
        assert_eq!(seller_credits_after, seller_credits_before + price);
        assert_eq!(credit_pool_balance(&platform), CREDIT_NOTE - price - fee);
        assert_tokens_conserved(&platform);

        // Underpaying is rejected before any note is spent.
        let (credit_note, credit_anchor, credit_path) = fund_credit_pool(&platform, 16);
        let (underpaid, _) = build_token_purchase_from_shielded_pool_transition(
            token_id,
            contract.id(),
            0,
            &address,
            &fvk,
            token_count,
            price - 1,
            [0u8; 36],
            ShieldedFeePayer {
                spends: vec![spendable(credit_note, credit_path)],
                change_address: &address,
                fvk: &fvk,
                ask: &ask,
                anchor: credit_anchor,
            },
            &Prover,
            platform_version,
        )
        .expect("build transition");
        let result = process(&platform, &underpaid);
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::UnpaidConsensusError(
                ConsensusError::StateError(StateError::TokenDirectPurchaseUserPriceTooLow(_))
            )]
        );
        assert_eq!(pool_balance(&platform, token_id), token_count);
    }

    #[tokio::test]
    async fn test_token_pool_paid_transitions_rejected_before_protocol_version_15() {
        let platform_version =
            PlatformVersion::get(TOKEN_SHIELDED_POOL_INITIAL_PROTOCOL_VERSION - 1)
                .expect("previous protocol version");
        let platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(platform_version.protocol_version)
            .build_with_mock_rpc()
            .set_genesis_state();
        let contract_id = Identifier::from([9u8; 32]);
        let token_id: Identifier =
            dpp::tokens::calculate_token_id(contract_id.as_bytes(), 0).into();
        let bundle = dummy_bundle();
        let transition: StateTransition = TokenPurchaseFromShieldedPoolTransition::V0(
            TokenPurchaseFromShieldedPoolTransitionV0 {
                data_contract_id: contract_id,
                token_contract_position: 0,
                token_id,
                token_count: 1,
                total_agreed_price: 1,
                token_actions: bundle.actions.clone(),
                token_anchor: bundle.anchor,
                token_proof: bundle.proof.clone(),
                token_binding_signature: bundle.binding_signature,
                fee_actions: bundle.actions,
                fee_anchor: bundle.anchor,
                fee_proof: bundle.proof,
                fee_binding_signature: bundle.binding_signature,
                credit_amount: 2,
            },
        )
        .into();

        let platform_state = platform.state.load();
        let serialized = transition.serialize_to_bytes().expect("serialize");
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

        // A top-level transition outside its active version range does not even decode, exactly
        // as on software that predates it; the batch token transitions are gated later because
        // the batch itself decodes.
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::InternalError(message)]
                if message.contains("TokenPurchaseFromShieldedPool") && message.contains("not active")
        );
    }
}
