use super::*;

mod minimum_pool_notes;

/// Token shielded pool transitions: shield, unshield and shielded transfer inside a batch.
///
/// The bundles are real Orchard bundles proven with the shared proving key, so the tests cover
/// the whole path: identity signature and fee, pool bookkeeping, sighash binding, anchor and
/// nullifier checks, and token conservation with the pool as a balance term.
mod token_shielded_pool_tests {
    use super::*;
    use crate::execution::check_tx::CheckTxLevel;
    use crate::execution::validation::state_transition::state_transitions::test_helpers::{
        create_dummy_serialized_action, get_proving_key, serialize_authorized_bundle_i64,
    };
    use crate::platform_types::platform::PlatformRef;
    use crate::platform_types::state_transitions_processing_result::StateTransitionsProcessingResult;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TempPlatform;
    use dpp::data_contract::accessors::v0::DataContractV0Setters;
    use dpp::data_contract::accessors::v1::DataContractV1Setters;
    use dpp::data_contract::associated_token::token_configuration::accessors::v1::TokenConfigurationV1Setters;
    use dpp::data_contract::config::v0::DataContractConfigSettersV0;
    use dpp::data_contract::DataContract;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::identity_nonce::IDENTITY_NONCE_VALUE_FILTER;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::shielded::{
        compute_platform_sighash, token_pool_output_only_extra_sighash_data,
        token_shielded_transfer_extra_sighash_data_v0, token_unshield_extra_sighash_data_v0,
        OrchardBundleParams,
    };
    use dpp::state_transition::batch_transition::batched_transition::token_transition_action_type::TokenTransitionActionType;
    use dpp::state_transition::data_contract_create_transition::methods::DataContractCreateTransitionMethodsV0;
    use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
    use dpp::state_transition::data_contract_update_transition::methods::DataContractUpdateTransitionMethodsV0;
    use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
    use dpp::state_transition::StateTransition;
    use dpp::tests::fixtures::get_data_contract_fixture;
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

    /// Builds the outputs-only bundle of a token pool transition that only creates notes,
    /// paying `amount` to the test wallet. `action_type`, `token_id` and `owner_id` are what the
    /// bundle's sighash commits to, so a bundle built for one pool, one transition kind or one
    /// owner will not verify as another.
    pub(super) fn build_shield_bundle(
        amount: u64,
        seed: u64,
        action_type: TokenTransitionActionType,
        token_id: Identifier,
        owner_id: Identifier,
    ) -> OrchardBundleParams {
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
        let extra_sighash_data = token_pool_output_only_extra_sighash_data(
            action_type,
            &token_id.to_buffer(),
            &owner_id.to_buffer(),
            PlatformVersion::latest(),
        )
        .expect("outputs-only token pool sighash data");
        let sighash = compute_platform_sighash(&commitment, &extra_sighash_data);
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

    /// CheckTx, the mempool admission path: it verifies every token shielded bundle a batch
    /// carries statelessly, under the identity contract nonce limiter, so the sighash it binds
    /// must agree with the one block validation binds.
    pub(super) fn assert_check_tx_accepts(
        platform: &TempPlatform<MockCoreRPCLike>,
        transition: &StateTransition,
    ) {
        let state = platform.state.load();
        let platform_ref = PlatformRef {
            drive: &platform.drive,
            state: &state,
            config: &platform.config,
            core_rpc: &platform.core_rpc,
        };
        let result = platform
            .check_tx(
                &transition
                    .serialize_to_bytes()
                    .expect("serialize transition"),
                CheckTxLevel::FirstTimeCheck,
                &platform_ref,
                PlatformVersion::latest(),
            )
            .expect("check tx");
        assert!(
            result.is_valid(),
            "unexpected CheckTx errors: {:?}",
            result.errors
        );
    }

    /// The counterpart of [`assert_check_tx_accepts`]: CheckTx must refuse the transition, and
    /// the errors it refuses it with are returned.
    ///
    /// One case is an identity that cannot pay. The shielded compute fee is charged when the
    /// action is built, which happens inside CheckTx, so an identity that cannot cover it is
    /// refused at admission rather than after the node has already run the Halo 2 verification
    /// the batch asks for.
    pub(super) fn assert_check_tx_rejects(
        platform: &TempPlatform<MockCoreRPCLike>,
        transition: &StateTransition,
    ) -> Vec<ConsensusError> {
        let state = platform.state.load();
        let platform_ref = PlatformRef {
            drive: &platform.drive,
            state: &state,
            config: &platform.config,
            core_rpc: &platform.core_rpc,
        };
        let result = platform
            .check_tx(
                &transition
                    .serialize_to_bytes()
                    .expect("serialize transition"),
                CheckTxLevel::FirstTimeCheck,
                &platform_ref,
                PlatformVersion::latest(),
            )
            .expect("check tx");
        assert!(
            !result.is_valid(),
            "CheckTx admitted a transition it must refuse"
        );
        result.errors
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
        let shield_bundle = build_shield_bundle(
            SHIELD_AMOUNT,
            11,
            TokenTransitionActionType::Shield,
            token_id,
            identity.id(),
        );
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

        assert_check_tx_accepts(&platform, &unshield);
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

        assert_check_tx_accepts(&platform, &transfer);
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
        // The token owns no pool, so its id must not reach the block-end anchor recorder:
        // recording an anchor for a pool that was never created fails the read, and that error
        // propagates out of `run_block_proposal` and aborts the whole proposal. A paid rejection
        // writes to no pool, so only a successful execution may register one.
        assert!(
            result.token_shielded_pools_touched().is_empty(),
            "a paid rejection must register no pool for anchor recording, got {:?}",
            result.token_shielded_pools_touched()
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

    /// CheckTx must price the Orchard bundle before it agrees to verify it.
    ///
    /// The flat shielded compute fee is charged where the action is built, and CheckTx builds
    /// the action, so an identity whose credits cannot cover that fee is refused at admission.
    /// If the fee were only added during batch state validation — which CheckTx skips — the
    /// mempool would accept the batch, run the Halo 2 verification it asks for, and only the
    /// block would discover the identity could never pay.
    #[tokio::test]
    async fn test_token_shield_rejected_by_check_tx_when_credits_cannot_cover_the_compute_fee() {
        let platform_version = PlatformVersion::latest();
        let mut platform = platform_with_latest_version();
        let mut rng = StdRng::seed_from_u64(9004);

        // The identity has to be funded before the contract exists, so the fee is computed from
        // the action count Orchard pads a single-output bundle to, and the bundle built below
        // is asserted to have exactly that many.
        const PADDED_ACTIONS: usize = 2;
        let compute_fee =
            dpp::shielded::compute_shielded_verification_fee(PADDED_ACTIONS, platform_version)
                .expect("shielded compute fee");

        // Funded well past the preliminary batch minimum — which has no Orchard component — so
        // the rejection has to come from the full fee estimate, and exactly one compute fee
        // short of affording the batch.
        let (identity, signer, key) = setup_identity(&mut platform, rng.gen(), compute_fee);
        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            identity.id(),
            Some(enable_shielded_pool),
            None,
            None,
            None,
            platform_version,
        );

        let bundle = build_shield_bundle(
            SHIELD_AMOUNT,
            13,
            TokenTransitionActionType::Shield,
            token_id,
            identity.id(),
        );
        assert_eq!(
            bundle.actions.len(),
            PADDED_ACTIONS,
            "the funded figure must be this bundle's compute fee"
        );

        let shield = BatchTransition::new_token_shield_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            SHIELD_AMOUNT,
            bundle,
            &key,
            2,
            0,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("token shield transition");

        // The balance CheckTx demands must already contain the compute fee. If the fee were
        // only added later, during batch state validation, the figure here would cover the
        // metered storage alone and an identity funded for it would pass admission.
        let errors = assert_check_tx_rejects(&platform, &shield);
        assert_matches!(
            errors.as_slice(),
            [ConsensusError::StateError(StateError::IdentityInsufficientBalanceError(error))]
                if error.required_balance() > compute_fee
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

    /// A token pool's anchors are namespaced by token id, so an anchor the chain recorded for
    /// one token proves nothing about another token's note tree.
    ///
    /// The existing unknown-anchor test spends against an anchor recorded in no pool at all, so
    /// it would still pass if the lookup read the credit pool's tree or the wrong token's. This
    /// one records the anchor in a real, neighbouring pool and requires the spend to be refused
    /// anyway.
    #[tokio::test]
    async fn test_token_unshield_rejects_an_anchor_recorded_in_another_tokens_pool() {
        let platform_version = PlatformVersion::latest();
        let mut platform = platform_with_latest_version();
        let mut rng = StdRng::seed_from_u64(9101);

        let (identity, signer, key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (recipient, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.1));

        let (contract_a, token_a) = create_token_contract_with_owner_identity(
            &mut platform,
            identity.id(),
            Some(enable_shielded_pool),
            None,
            None,
            None,
            platform_version,
        );
        // A second owner, because a contract id derives from its owner: the same owner would
        // give back the same contract and the same pool, and the test would prove nothing.
        let (other_owner, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.1));
        let (_contract_b, token_b) = create_token_contract_with_owner_identity(
            &mut platform,
            other_owner.id(),
            Some(enable_shielded_pool),
            None,
            None,
            None,
            platform_version,
        );
        assert_ne!(token_a, token_b, "the two tokens must own separate pools");

        // Recorded in token B's pool, and nowhere else.
        let (note, anchor, merkle_path) = spendable_note(6_000, 21);
        insert_token_pool_anchor(&platform, token_b, &anchor);

        let unshield_amount = 1_000;
        let extra = token_unshield_extra_sighash_data_v0(
            &token_a.to_buffer(),
            &identity.id().to_buffer(),
            &recipient.id().to_buffer(),
            unshield_amount,
        );
        let (unshield_bundle, _) =
            build_spend_bundle(note, merkle_path, anchor, unshield_amount, &extra, 22);

        let unshield = BatchTransition::new_token_unshield_transition(
            token_a,
            identity.id(),
            contract_a.id(),
            0,
            unshield_amount,
            recipient.id(),
            unshield_bundle,
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
            identity_token_balance(&platform, token_a, recipient.id()),
            None
        );
    }

    /// An outputs-only bundle carries no anchor, so nothing in the proved bytes themselves says
    /// which pool they were built for: the proof and the binding signature verify against every
    /// token pool, all of which start from the same empty-tree anchor. Only the sighash tells
    /// them apart. Without the token id in it, whoever the bundle names as its owner could submit
    /// it again as a shield of another token, funded by that token, and land a copy of the note
    /// in a pool it was never meant for. Nullifiers are per pool, so both copies stay spendable
    /// and this is not Faerie Gold; the harm is that one `rho` now yields notes in two pools,
    /// which links the recipient's spends across them.
    #[tokio::test]
    async fn test_token_shield_rejects_a_bundle_proved_for_another_tokens_pool() {
        let platform_version = PlatformVersion::latest();
        let mut platform = platform_with_latest_version();
        let mut rng = StdRng::seed_from_u64(9103);

        let (victim, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (_contract_a, token_a) = create_token_contract_with_owner_identity(
            &mut platform,
            victim.id(),
            Some(enable_shielded_pool),
            None,
            None,
            None,
            platform_version,
        );
        // A second owner, because a contract id derives from its owner: the same owner would
        // give back the same contract and the same pool, and the test would prove nothing.
        let (attacker, attacker_signer, attacker_key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (contract_b, token_b) = create_token_contract_with_owner_identity(
            &mut platform,
            attacker.id(),
            Some(enable_shielded_pool),
            None,
            None,
            None,
            platform_version,
        );
        assert_ne!(token_a, token_b, "the two tokens must own separate pools");

        // Proved for the victim's pool, then replayed into the attacker's own, paid for with
        // the attacker's own tokens. The bundle names the attacker as its owner, so the pool is
        // the only thing about it that does not match the replay.
        let stolen_bundle = build_shield_bundle(
            SHIELD_AMOUNT,
            31,
            TokenTransitionActionType::Shield,
            token_a,
            attacker.id(),
        );
        let replay = BatchTransition::new_token_shield_transition(
            token_b,
            attacker.id(),
            contract_b.id(),
            0,
            SHIELD_AMOUNT,
            stolen_bundle,
            &attacker_key,
            2,
            0,
            &attacker_signer,
            platform_version,
            None,
        )
        .await
        .expect("token shield transition");

        let result = process(&platform, &replay);
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(StateError::InvalidShieldedProofError(_)),
                ..
            }]
        );
        assert_eq!(pool_balance(&platform, token_b), 0);
        assert_eq!(pool_notes_count(&platform, token_b), 0);
        assert_tokens_conserved(&platform);
    }

    /// Spent nullifiers are namespaced by token id too: spending a note in one token's pool must
    /// not mark that nullifier spent in another's, or the first token to use a nullifier would
    /// make every other pool's note with the same nullifier unspendable.
    #[tokio::test]
    async fn test_a_nullifier_spent_in_one_token_pool_is_unspent_in_another() {
        let platform_version = PlatformVersion::latest();
        let mut platform = platform_with_latest_version();
        let mut rng = StdRng::seed_from_u64(9102);

        let (identity, signer, key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (recipient, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.1));

        let (contract_a, token_a) = create_token_contract_with_owner_identity(
            &mut platform,
            identity.id(),
            Some(enable_shielded_pool),
            None,
            None,
            None,
            platform_version,
        );
        // A second owner, because a contract id derives from its owner: the same owner would
        // give back the same contract and the same pool, and the test would prove nothing.
        let (other_owner, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.1));
        let (_contract_b, token_b) = create_token_contract_with_owner_identity(
            &mut platform,
            other_owner.id(),
            Some(enable_shielded_pool),
            None,
            None,
            None,
            platform_version,
        );
        assert_ne!(token_a, token_b, "the two tokens must own separate pools");

        let shield = BatchTransition::new_token_shield_transition(
            token_a,
            identity.id(),
            contract_a.id(),
            0,
            SHIELD_AMOUNT,
            build_shield_bundle(
                SHIELD_AMOUNT,
                23,
                TokenTransitionActionType::Shield,
                token_a,
                identity.id(),
            ),
            &key,
            2,
            0,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("token shield transition");
        assert_matches!(
            process(&platform, &shield).execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );

        let (note, anchor, merkle_path) = spendable_note(6_000, 24);
        insert_token_pool_anchor(&platform, token_a, &anchor);
        let unshield_amount = 4_000;
        let extra = token_unshield_extra_sighash_data_v0(
            &token_a.to_buffer(),
            &identity.id().to_buffer(),
            &recipient.id().to_buffer(),
            unshield_amount,
        );
        let (unshield_bundle, _) =
            build_spend_bundle(note, merkle_path, anchor, unshield_amount, &extra, 25);

        let unshield = BatchTransition::new_token_unshield_transition(
            token_a,
            identity.id(),
            contract_a.id(),
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
        assert_matches!(
            process(&platform, &unshield).execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );

        for action in unshield_bundle.actions.iter() {
            assert!(
                nullifier_is_spent(&platform, token_a, &action.nullifier),
                "the spend must be recorded in its own pool"
            );
            assert!(
                !nullifier_is_spent(&platform, token_b, &action.nullifier),
                "a spend in one pool must not mark the nullifier spent in another"
            );
        }
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
    pub(super) fn shielded_token_contract(
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

    #[tokio::test]
    async fn should_gate_new_shielded_tokens_and_validate_their_rules_on_contract_update() {
        for (platform_version, incompatible_rules) in [
            (PlatformVersion::get(13).unwrap(), false),
            (PlatformVersion::latest(), true),
            (PlatformVersion::latest(), false),
        ] {
            let mut platform = TestPlatformBuilder::new()
                .with_initial_protocol_version(platform_version.protocol_version)
                .build_with_mock_rpc()
                .set_genesis_state();
            let (identity, signer, key) =
                setup_identity(&mut platform, 9012, dash_to_credits!(1.0));
            let mut contract =
                get_data_contract_fixture(None, 0, platform_version.protocol_version)
                    .data_contract_owned();
            contract.set_owner_id(identity.id());
            contract.config_mut().set_readonly(false);
            platform
                .drive
                .apply_contract(
                    &contract,
                    BlockInfo::default(),
                    true,
                    StorageFlags::optional_default_as_cow(),
                    None,
                    platform_version,
                )
                .expect("store original contract");

            let token_contract = shielded_token_contract(identity.id(), platform_version);
            let mut configuration = token_contract
                .expected_token_configuration(0)
                .expect("shielded token configuration")
                .clone();
            if incompatible_rules {
                configuration.set_freeze_rules(ChangeControlRules::V0(ChangeControlRulesV0 {
                    authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
                    admin_action_takers: AuthorizedActionTakers::NoOne,
                    changing_authorized_action_takers_to_no_one_allowed: false,
                    changing_admin_action_takers_to_no_one_allowed: false,
                    self_changing_admin_action_takers_allowed: false,
                }));
            }
            contract.add_token(0, configuration);
            contract.increment_version();
            let token_id = contract.token_id(0).expect("token id");
            let update = DataContractUpdateTransition::new_from_data_contract(
                contract,
                &identity.into_partial_identity_info(),
                key.id(),
                1,
                0,
                &signer,
                platform_version,
                None,
            )
            .await
            .expect("contract update transition");
            let state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();
            let result = platform
                .platform
                .process_raw_state_transitions(
                    &[update.serialize_to_bytes().expect("serialize update")],
                    &state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("process contract update");

            if platform_version.protocol_version < 14 {
                assert_matches!(
                    result.execution_results().as_slice(),
                    [StateTransitionExecutionResult::UnpaidConsensusError(
                        ConsensusError::BasicError(BasicError::UnsupportedVersionError(_))
                    )]
                );
            } else if incompatible_rules {
                assert_matches!(result.execution_results().as_slice(),
                    [StateTransitionExecutionResult::UnpaidConsensusError(
                        ConsensusError::BasicError(BasicError::TokenShieldedPoolIncompatibleRulesError(error))
                    )] if error.token_contract_position() == 0 && error.rule() == "freezeRules");
            } else {
                assert_matches!(
                    result.execution_results().as_slice(),
                    [StateTransitionExecutionResult::SuccessfulExecution { .. }]
                );
                assert!(platform
                    .drive
                    .has_token_shielded_pool(
                        token_id.to_buffer(),
                        Some(&transaction),
                        &mut vec![],
                        platform_version,
                    )
                    .expect("new token pool must exist"));
            }
        }
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
        assert_check_tx_accepts, assert_tokens_conserved, build_shield_bundle, build_spend_bundle,
        dummy_bundle, enable_shielded_pool, identity_token_balance, insert_token_pool_anchor,
        nullifier_is_spent, platform_with_latest_version, pool_balance, pool_notes_count, process,
        spendable_note, OWNER_INITIAL_BALANCE,
    };
    use super::*;
    use crate::execution::validation::state_transition::processor::traits::shielded_proof::StateTransitionShieldedProofValidationV0;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::fast_forward_to_block::fast_forward_to_block;
    use crate::test::helpers::setup::TempPlatform;
    use dpp::block::epoch::Epoch;
    use dpp::consensus::basic::BasicError;
    use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
    use dpp::data_contract::associated_token::token_configuration::accessors::v1::TokenConfigurationV1Setters;
    use dpp::data_contract::associated_token::token_distribution_key::TokenDistributionType;
    use dpp::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Setters;
    use dpp::data_contract::associated_token::token_distribution_rules::accessors::v1::TokenDistributionRulesV1Setters;
    use dpp::data_contract::associated_token::token_once_per_identity_distribution::v0::TokenOncePerIdentityDistributionV0;
    use dpp::data_contract::associated_token::token_once_per_identity_distribution::TokenOncePerIdentityDistribution;
    use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction;
    use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::TokenDistributionRecipient;
    use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_type::RewardDistributionType;
    use dpp::data_contract::associated_token::token_perpetual_distribution::v0::TokenPerpetualDistributionV0;
    use dpp::data_contract::associated_token::token_perpetual_distribution::TokenPerpetualDistribution;
    use dpp::data_contract::associated_token::token_pre_programmed_distribution::v0::TokenPreProgrammedDistributionV0;
    use dpp::data_contract::associated_token::token_pre_programmed_distribution::TokenPreProgrammedDistribution;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::shielded::{serialized_actions_digest, token_burn_from_pool_extra_sighash_data_v0};
    use dpp::state_transition::batch_transition::batched_transition::token_transition_action_type::TokenTransitionActionType;
    use dpp::state_transition::batch_transition::{
        TokenBurnFromPoolTransition, TokenSetPriceForDirectPurchaseTransition,
    };
    use dpp::state_transition::StateTransition;
    use dpp::tokens::token_pricing_schedule::TokenPricingSchedule;
    use platform_version::version::PlatformVersion;

    fn total_supply(platform: &TempPlatform<MockCoreRPCLike>, token_id: Identifier) -> u64 {
        platform
            .drive
            .fetch_token_total_supply(token_id.to_buffer(), None, PlatformVersion::latest())
            .expect("total supply")
            .expect("supply exists")
    }

    /// Binding the token id alone would not be enough. A shield and a mint into the same pool
    /// produce outputs-only bundles that agree on everything the proof covers — same flags, same
    /// empty-tree anchor, same `value_balance` — so without a per-kind tag in the sighash the
    /// very same proved bytes would satisfy both. A shield bundle would then be resubmittable as
    /// a mint of the same token, which prints supply against a proof its author never made for
    /// that purpose.
    #[tokio::test]
    async fn test_token_mint_to_pool_rejects_a_bundle_proved_as_a_shield_of_the_same_token() {
        let platform_version = PlatformVersion::latest();
        let mut platform = platform_with_latest_version();
        let mut rng = StdRng::seed_from_u64(9113);

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
        let supply_before = total_supply(&platform, token_id);

        // Same pool, same amount, same flags — only the transition kind differs.
        let shield_bundle = build_shield_bundle(
            1_337,
            21,
            TokenTransitionActionType::Shield,
            token_id,
            identity.id(),
        );
        let mint = BatchTransition::new_token_mint_to_pool_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            1_337,
            shield_bundle,
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
                error: ConsensusError::StateError(StateError::InvalidShieldedProofError(_)),
                ..
            }]
        );
        assert_eq!(pool_balance(&platform, token_id), 0);
        assert_eq!(
            total_supply(&platform, token_id),
            supply_before,
            "a rejected mint must not print supply"
        );
        assert_tokens_conserved(&platform);
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

        let bundle = build_shield_bundle(
            1_337,
            21,
            TokenTransitionActionType::MintToPool,
            token_id,
            identity.id(),
        );
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

        // CheckTx rebuilds the preimage independently of block execution, so a kind that
        // reached for the wrong tag there would drop every honest transition of this kind
        // at admission while block-level tests stayed green.
        assert_check_tx_accepts(&platform, &mint);
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

    /// A mint into the pool sends the tokens wherever the minter's notes say, so it is refused
    /// where the configuration pins the destination of minted tokens.
    #[tokio::test]
    async fn test_token_mint_to_pool_rejected_when_destination_is_fixed() {
        let platform_version = PlatformVersion::latest();
        let mut platform = platform_with_latest_version();
        let mut rng = StdRng::seed_from_u64(9106);

        let (identity, signer, key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            identity.id(),
            Some(|token_configuration: &mut TokenConfiguration| {
                enable_shielded_pool(token_configuration);
                token_configuration
                    .distribution_rules_mut()
                    .set_minting_allow_choosing_destination(false);
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
            1_337,
            build_shield_bundle(
                1_337,
                27,
                TokenTransitionActionType::MintToPool,
                token_id,
                identity.id(),
            ),
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
                error: ConsensusError::BasicError(
                    BasicError::ChoosingTokenMintRecipientNotAllowedError(_)
                ),
                ..
            }]
        );
        assert_eq!(pool_balance(&platform, token_id), 0);
        assert_eq!(total_supply(&platform, token_id), OWNER_INITIAL_BALANCE);
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
        let shield_bundle = build_shield_bundle(
            10_000,
            22,
            TokenTransitionActionType::Shield,
            token_id,
            identity.id(),
        );
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

        assert_check_tx_accepts(&platform, &burn);
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
            build_shield_bundle(
                10_000,
                24,
                TokenTransitionActionType::Shield,
                token_id,
                proposer.id(),
            ),
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
        assert_check_tx_accepts(&platform, &proposal);
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
        assert_check_tx_accepts(&platform, &confirmation);

        // Stateless admission must defer a confirmer's proof, but block execution must
        // still verify it after recovering the proposer. Corrupting only the binding
        // signature preserves the group action's pinned actions digest.
        let mut corrupted_bundle = burn_bundle.clone();
        corrupted_bundle.binding_signature[0] ^= 1;
        let corrupted = BatchTransition::new_token_burn_from_pool_transition(
            token_id,
            confirmer.id(),
            contract.id(),
            0,
            burn_amount,
            corrupted_bundle,
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
        .expect("confirmation with an invalid binding signature");
        assert!(corrupted
            .validate_shielded_proof(platform_version)
            .expect("stateless proof check")
            .is_valid());
        let state = platform.state.load();
        let transaction = platform.drive.grove.start_transaction();
        let rejected = platform
            .platform
            .process_raw_state_transitions(
                &[corrupted
                    .serialize_to_bytes()
                    .expect("serialize corrupted confirmation")],
                &state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("validate corrupted confirmation");
        assert_matches!(
            rejected.execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(StateError::InvalidShieldedProofError(_)),
                ..
            }]
        );
        drop(transaction);
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
        assert_token_claim_to_pool(TokenDistributionType::PreProgrammed).await;
    }

    /// A perpetual claim into the pool must name the cycle-aligned moment it claims up to, so
    /// the amount its bundle proves is fixed before the block lands: without it the claim is a
    /// paid failure, with it the rewards accrued up to that moment enter the pool.
    #[tokio::test]
    async fn test_token_claim_to_pool_perpetual_requires_claim_up_to() {
        let platform_version = PlatformVersion::latest();
        let mut platform = platform_with_latest_version();
        let mut rng = StdRng::seed_from_u64(9107);

        let (owner, signer, key) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            owner.id(),
            Some(|configuration: &mut TokenConfiguration| {
                configuration.set_has_shielded_pool(true);
                configuration
                    .distribution_rules_mut()
                    .set_perpetual_distribution(Some(TokenPerpetualDistribution::V0(
                        TokenPerpetualDistributionV0 {
                            distribution_type: RewardDistributionType::BlockBasedDistribution {
                                interval: 10,
                                function: DistributionFunction::FixedAmount { amount: 50 },
                            },
                            distribution_recipient: TokenDistributionRecipient::ContractOwner,
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
        // Four cycles (heights 10, 20, 30 and 40) of 50 have accrued by height 41.
        let accrued = 200;

        // Without the moment the claim is refused before its proof is verified.
        let unpinned = BatchTransition::new_token_claim_to_pool_transition(
            token_id,
            owner.id(),
            contract.id(),
            0,
            TokenDistributionType::Perpetual,
            None,
            build_shield_bundle(
                accrued,
                27,
                TokenTransitionActionType::ClaimToPool,
                token_id,
                owner.id(),
            ),
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
                &[unpinned.serialize_to_bytes().expect("serialize")],
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
                error: ConsensusError::StateError(StateError::InvalidTokenClaimPropertyMismatch(_)),
                ..
            }]
        );
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("commit");
        assert_eq!(pool_balance(&platform, token_id), 0);

        // Pinned to the current cycle, the bundle proves exactly the accrued rewards.
        let claim_bundle = build_shield_bundle(
            accrued,
            28,
            TokenTransitionActionType::ClaimToPool,
            token_id,
            owner.id(),
        );
        let pinned = BatchTransition::new_token_claim_to_pool_transition(
            token_id,
            owner.id(),
            contract.id(),
            0,
            TokenDistributionType::Perpetual,
            Some(40),
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
                &[pinned.serialize_to_bytes().expect("serialize")],
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
        assert_eq!(pool_balance(&platform, token_id), accrued);
        assert_eq!(
            identity_token_balance(&platform, token_id, owner.id()),
            Some(OWNER_INITIAL_BALANCE)
        );
        assert_eq!(
            total_supply(&platform, token_id),
            OWNER_INITIAL_BALANCE + accrued
        );
        assert_eq!(
            pool_notes_count(&platform, token_id),
            claim_bundle.actions.len() as u64
        );
        assert_tokens_conserved(&platform);
    }

    #[tokio::test]
    async fn should_claim_once_per_identity_into_the_pool_only_once() {
        assert_token_claim_to_pool(TokenDistributionType::OncePerIdentity).await;
    }

    async fn assert_token_claim_to_pool(distribution_type: TokenDistributionType) {
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
                if distribution_type == TokenDistributionType::OncePerIdentity {
                    configuration
                        .distribution_rules_mut()
                        .set_once_per_identity_distribution(Some(
                            TokenOncePerIdentityDistribution::V0(
                                TokenOncePerIdentityDistributionV0 { amount: 445 },
                            ),
                        ));
                } else {
                    configuration
                        .distribution_rules_mut()
                        .set_pre_programmed_distribution(Some(TokenPreProgrammedDistribution::V0(
                            TokenPreProgrammedDistributionV0 {
                                distributions: [(100, [(claimant_id, 445)].into())].into(),
                            },
                        )));
                }
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
            distribution_type,
            None,
            build_shield_bundle(
                444,
                24,
                TokenTransitionActionType::ClaimToPool,
                token_id,
                claimant_id,
            ),
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

        let claim_bundle = build_shield_bundle(
            445,
            25,
            TokenTransitionActionType::ClaimToPool,
            token_id,
            claimant_id,
        );
        let claim = BatchTransition::new_token_claim_to_pool_transition(
            token_id,
            claimant_id,
            contract.id(),
            0,
            distribution_type,
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
            distribution_type,
            None,
            build_shield_bundle(
                445,
                26,
                TokenTransitionActionType::ClaimToPool,
                token_id,
                claimant_id,
            ),
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
        if distribution_type == TokenDistributionType::OncePerIdentity {
            assert_matches!(
                result.execution_results().as_slice(),
                [StateTransitionExecutionResult::PaidConsensusError {
                    error: ConsensusError::StateError(
                        StateError::TokenOncePerIdentityDistributionAlreadyClaimedError(_)
                    ),
                    ..
                }]
            );
        } else {
            assert_matches!(
                result.execution_results().as_slice(),
                [StateTransitionExecutionResult::PaidConsensusError {
                    error: ConsensusError::StateError(
                        StateError::InvalidTokenClaimNoCurrentRewards(_)
                    ),
                    ..
                }]
            );
        }
        assert_eq!(pool_balance(&platform, token_id), 445);
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

        let purchase_bundle = build_shield_bundle(
            3,
            27,
            TokenTransitionActionType::DirectPurchaseToPool,
            token_id,
            buyer.id(),
        );
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
        // CheckTx rebuilds the preimage independently of block execution, so a kind that
        // reached for the wrong tag there would drop every honest transition of this kind
        // at admission while block-level tests stayed green.
        assert_check_tx_accepts(&platform, &purchase);
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
            build_shield_bundle(
                3,
                28,
                TokenTransitionActionType::DirectPurchaseToPool,
                token_id,
                buyer.id(),
            ),
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

        // A buyer whose credits cover the fee but not the price is refused before execution:
        // the agreed price is part of the batch's required balance, so the shortfall is a
        // consensus rejection and never reaches the balance removal.
        let (poor_buyer, poor_signer, poor_key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.02));
        let unaffordable = BatchTransition::new_token_direct_purchase_to_pool_transition(
            token_id,
            poor_buyer.id(),
            contract.id(),
            0,
            3,
            dash_to_credits!(0.03),
            build_shield_bundle(
                3,
                29,
                TokenTransitionActionType::DirectPurchaseToPool,
                token_id,
                poor_buyer.id(),
            ),
            &poor_key,
            1,
            0,
            &poor_signer,
            platform_version,
            None,
        )
        .await
        .expect("token direct purchase to pool transition");
        let result = process(&platform, &unaffordable);
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::UnpaidConsensusError(
                ConsensusError::StateError(StateError::IdentityInsufficientBalanceError(_))
            )]
        );
        assert_eq!(pool_balance(&platform, token_id), 3);
        assert_tokens_conserved(&platform);
    }
}

/// Documents whose token cost is paid out of the token's shielded pool (`TokenPaymentInfo::V1`).
///
/// The card game contract charges 10 of its token 0 to create a `card`, burned or paid to the
/// contract owner depending on the fixture. The buyer shields the tokens first, then pays the
/// cost with a spend bundle bound to the token, the buyer, the contract and the document id.
mod document_shielded_token_payment_tests {
    use super::token_shielded_pool_tests::{
        assert_check_tx_accepts, assert_tokens_conserved, build_shield_bundle, build_spend_bundle,
        dummy_bundle, identity_token_balance, insert_token_pool_anchor, nullifier_is_spent,
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
    use dpp::prelude::IdentityNonce;
    use dpp::shielded::{document_token_payment_extra_sighash_data_v0, OrchardBundleParams};
    use dpp::state_transition::batch_transition::batched_transition::token_transition_action_type::TokenTransitionActionType;
    use dpp::tokens::calculate_token_id;
    use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
    use dpp::tokens::token_payment_info::v1::{TokenPaymentInfoV1, TokenShieldedPayment};
    use dpp::tokens::token_payment_info::TokenPaymentInfo;
    use dpp::version::feature_initial_protocol_versions::TOKEN_SHIELDED_POOL_INITIAL_PROTOCOL_VERSION;
    use drive::util::test_helpers::setup_contract;
    use platform_version::version::PlatformVersion;
    use simple_signer::signer::SimpleSigner;

    pub(super) const CARD_COST: u64 = 10;
    pub(super) const SHIELDED: u64 = 15;

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

    pub(super) fn random_card(
        rng: &mut StdRng,
        card_document_type: DocumentTypeRef,
        owner_id: Identifier,
        identity_contract_nonce: IdentityNonce,
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
        // The proof must bind the final nonce-derived creation ID.
        document
            .set_id_for_creation(
                card_document_type,
                &entropy.0,
                identity_contract_nonce,
                platform_version,
            )
            .expect("creation document id");
        document.set("attack", 4.into());
        document.set("defense", 7.into());
        (document, entropy)
    }

    pub(super) fn shielded_payment(
        bundle: OrchardBundleParams,
        amount: u64,
    ) -> TokenShieldedPayment {
        TokenShieldedPayment {
            amount,
            actions: bundle.actions,
            anchor: bundle.anchor,
            proof: bundle.proof,
            binding_signature: bundle.binding_signature,
        }
    }

    pub(super) fn payment_info(shielded_payment: TokenShieldedPayment) -> TokenPaymentInfo {
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
    pub(super) async fn fund_and_shield(
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
            build_shield_bundle(
                SHIELDED,
                seed,
                TokenTransitionActionType::Shield,
                token_id,
                buyer.id(),
            ),
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
        let (document, entropy) = random_card(
            &mut rng,
            card_document_type,
            buyer.id(),
            2,
            platform_version,
        );

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

        assert_check_tx_accepts(&platform, &transition);
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
        let (replay_document, replay_entropy) = random_card(
            &mut rng,
            card_document_type,
            buyer.id(),
            3,
            platform_version,
        );
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
        let (document, entropy) = random_card(
            &mut rng,
            card_document_type,
            buyer.id(),
            2,
            platform_version,
        );

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
        let (document, entropy) = random_card(
            &mut rng,
            card_document_type,
            buyer.id(),
            2,
            platform_version,
        );

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
    async fn test_document_shielded_payment_rejected_before_protocol_version_14() {
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
        let (document, entropy) = random_card(
            &mut rng,
            card_document_type,
            buyer.id(),
            1,
            platform_version,
        );

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
        assert_check_tx_accepts, assert_tokens_conserved, build_shield_bundle, dummy_bundle,
        enable_shielded_pool, identity_token_balance, insert_token_pool_anchor, nullifier_is_spent,
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
        build_token_claim_to_pool_transition, build_token_direct_purchase_to_pool_transition,
        build_token_mint_to_pool_transition, build_token_shield_transition,
    };
    use dpp::shielded::token_pool_output_only_extra_sighash_data;
    use dpp::data_contract::associated_token::token_distribution_key::TokenDistributionType;
    use crate::execution::validation::state_transition::state_transitions::shielded_common::{
        reconstruct_and_verify_bundle, FLAGS_OUTPUTS_ONLY,
    };
    use dpp::state_transition::batch_transition::token_mint_to_pool_transition::v0::v0_methods::TokenMintToPoolTransitionV0Methods;
    use dpp::state_transition::batch_transition::token_claim_to_pool_transition::v0::v0_methods::TokenClaimToPoolTransitionV0Methods;
    use dpp::state_transition::batch_transition::token_direct_purchase_to_pool_transition::v0::v0_methods::TokenDirectPurchaseToPoolTransitionV0Methods;
    use dpp::shielded::builder::{
        build_token_purchase_from_shielded_pool_transition,
        build_token_shielded_transfer_with_shielded_fee_transition,
        build_token_unshield_with_shielded_fee_transition, OrchardProver, ShieldedFeePayer,
        SpendableNote, TokenPoolSpender,
    };
    use dpp::shielded::OrchardBundleParams;
    use dpp::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
    use dpp::state_transition::batch_transition::batched_transition::token_transition::TokenTransition;
    use dpp::state_transition::batch_transition::batched_transition::token_transition_action_type::TokenTransitionActionType;
    use dpp::state_transition::batch_transition::batched_transition::BatchedTransitionRef;
    use dpp::state_transition::batch_transition::token_shield_transition::v0::v0_methods::TokenShieldTransitionV0Methods;
    use dpp::state_transition::token_purchase_from_shielded_pool_transition::v0::TokenPurchaseFromShieldedPoolTransitionV0;
    use dpp::state_transition::token_purchase_from_shielded_pool_transition::TokenPurchaseFromShieldedPoolTransition;
    use dpp::state_transition::StateTransition;
    use dpp::tokens::token_pricing_schedule::TokenPricingSchedule;
    use dpp::version::feature_initial_protocol_versions::TOKEN_SHIELDED_POOL_INITIAL_PROTOCOL_VERSION;
    use grovedb_commitment_tree::{Anchor, MerklePath, Note, ProvingKey};
    use platform_version::version::PlatformVersion;
    use simple_signer::signer::SimpleSigner;

    /// A credit pool note the test wallet holds: enough for a purchase at 0.01 DASH a token.
    pub(super) const CREDIT_NOTE: u64 = 5_000_000_000;
    pub(super) const SHIELDED: u64 = 15;

    pub(super) struct Prover;

    impl OrchardProver for Prover {
        fn proving_key(&self) -> &ProvingKey {
            get_proving_key()
        }
    }

    pub(super) fn wallet_address() -> OrchardAddress {
        let (_, _, address) = spend_keys();
        OrchardAddress::from_raw_bytes(&address.to_raw_address_bytes())
            .expect("valid orchard address bytes")
    }

    pub(super) fn spendable(note: Note, merkle_path: MerklePath) -> SpendableNote {
        SpendableNote { note, merkle_path }
    }

    /// Puts a `CREDIT_NOTE` note of the test wallet into the credit pool: its anchor is
    /// recorded and the pool total covers it.
    pub(super) fn fund_credit_pool(
        platform: &TempPlatform<MockCoreRPCLike>,
        tag: u8,
    ) -> (Note, Anchor, MerklePath) {
        let (note, anchor, merkle_path) = spendable_note(CREDIT_NOTE, tag);
        insert_anchor_into_state(platform, &anchor.to_bytes());
        set_pool_total_balance(platform, CREDIT_NOTE);
        (note, anchor, merkle_path)
    }

    pub(super) fn credit_pool_balance(platform: &TempPlatform<MockCoreRPCLike>) -> u64 {
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
    pub(super) async fn fund_token_pool(
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
            build_shield_bundle(
                SHIELDED,
                seed,
                TokenTransitionActionType::Shield,
                token_id,
                holder.id(),
            ),
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
    async fn test_token_pool_paid_transitions_rejected_before_protocol_version_14() {
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

    /// The bundle a client actually ships must be the one consensus accepts. Every other test
    /// here hand-builds its bundle through the same helper the verifier's preimage comes from,
    /// so a mistake inside the public builder — the wrong kind, a missing field — would be
    /// invisible: the suite would stay green while every real shield was rejected at every
    /// node. This one goes through `build_token_shield_transition`, shows the bundle admitted
    /// by CheckTx and executed, and only then replays that same proven bundle into another
    /// pool. The positive half is what makes the negative half mean anything: the replay is
    /// refused for the pool it landed in, not because the bundle was never valid anywhere.
    #[tokio::test]
    async fn test_a_builder_made_shield_is_accepted_and_its_bundle_cannot_be_replayed() {
        let platform_version = PlatformVersion::latest();
        let mut platform = platform_with_latest_version();
        let mut rng = StdRng::seed_from_u64(9210);
        let amount = 4_200;

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

        let shield = build_token_shield_transition(
            token_id,
            owner.id(),
            contract.id(),
            0,
            &wallet_address(),
            amount,
            [0u8; 36],
            None,
            &key,
            2,
            0,
            &signer,
            &Prover,
            platform_version,
        )
        .await
        .expect("client-built token shield");

        // Admission and execution both rebuild the preimage; either disagreeing with the
        // builder would reject this.
        assert_check_tx_accepts(&platform, &shield);
        assert_matches!(
            process(&platform, &shield).execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        assert_eq!(pool_balance(&platform, token_id), amount);
        assert_tokens_conserved(&platform);

        // That exact bundle, proven valid above, is now offered to a second token's pool.
        let StateTransition::Batch(batch) = &shield else {
            panic!("expected a batch transition");
        };
        let BatchedTransitionRef::Token(TokenTransition::Shield(proven)) =
            batch.transitions_iter().next().expect("one transition")
        else {
            panic!("expected a token shield transition");
        };
        let proven_bundle = OrchardBundleParams {
            actions: proven.actions().to_vec(),
            anchor: *proven.anchor(),
            proof: proven.proof().to_vec(),
            binding_signature: *proven.binding_signature(),
        };

        // A second owner, because a contract id derives from its owner: the same owner would
        // give back the same contract and the same pool, and the test would prove nothing.
        let (other_owner, other_signer, other_key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (other_contract, other_token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            other_owner.id(),
            Some(enable_shielded_pool),
            None,
            None,
            None,
            platform_version,
        );
        assert_ne!(
            token_id, other_token_id,
            "the two tokens must own separate pools"
        );

        let replay = BatchTransition::new_token_shield_transition(
            other_token_id,
            other_owner.id(),
            other_contract.id(),
            0,
            amount,
            proven_bundle,
            &other_key,
            2,
            0,
            &other_signer,
            platform_version,
            None,
        )
        .await
        .expect("token shield transition");

        assert_matches!(
            process(&platform, &replay).execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(StateError::InvalidShieldedProofError(_)),
                ..
            }]
        );
        assert_eq!(pool_balance(&platform, other_token_id), 0);
        assert_tokens_conserved(&platform);
    }

    /// Each of the four public builders must bind the preimage its own kind's verifier
    /// rebuilds. Nothing in the types enforces that: each builder names its kind as a
    /// `TokenTransitionActionType` literal when it computes the preimage
    /// (`token_pool_output_only_extra_sighash_data`), so it cannot name a non-kind, but any one
    /// of them could name a *different* outputs-only kind and still compile. The failure is
    /// total and silent — every honest transition of that kind rejected at every node, with the
    /// suite green — so it is checked here directly, against the same verification consensus
    /// runs.
    #[tokio::test]
    async fn test_every_outputs_only_builder_binds_the_preimage_its_verifier_rebuilds() {
        let platform_version = PlatformVersion::latest();
        let mut platform = platform_with_latest_version();
        let (owner, signer, key) = setup_identity(&mut platform, 4242, dash_to_credits!(0.5));

        let token_id = Identifier::from([7u8; 32]);
        let contract_id = Identifier::from([8u8; 32]);
        let recipient = wallet_address();
        let amount = 5_000u64;

        let shield = build_token_shield_transition(
            token_id,
            owner.id(),
            contract_id,
            0,
            &recipient,
            amount,
            [0u8; 36],
            None,
            &key,
            2,
            0,
            &signer,
            &Prover,
            platform_version,
        )
        .await
        .expect("shield");
        let mint = build_token_mint_to_pool_transition(
            token_id,
            owner.id(),
            contract_id,
            0,
            &recipient,
            amount,
            [0u8; 36],
            None,
            None,
            None,
            &key,
            3,
            0,
            &signer,
            &Prover,
            platform_version,
        )
        .await
        .expect("mint to pool");
        let claim = build_token_claim_to_pool_transition(
            token_id,
            owner.id(),
            contract_id,
            0,
            &recipient,
            amount,
            TokenDistributionType::PreProgrammed,
            None,
            [0u8; 36],
            None,
            None,
            &key,
            4,
            0,
            &signer,
            &Prover,
            platform_version,
        )
        .await
        .expect("claim to pool");
        let purchase = build_token_direct_purchase_to_pool_transition(
            token_id,
            owner.id(),
            contract_id,
            0,
            &recipient,
            amount,
            1_000,
            [0u8; 36],
            None,
            &key,
            5,
            0,
            &signer,
            &Prover,
            platform_version,
        )
        .await
        .expect("direct purchase to pool");

        for (built, action_type) in [
            (&shield, TokenTransitionActionType::Shield),
            (&mint, TokenTransitionActionType::MintToPool),
            (&claim, TokenTransitionActionType::ClaimToPool),
            (&purchase, TokenTransitionActionType::DirectPurchaseToPool),
        ] {
            let StateTransition::Batch(batch) = built else {
                panic!("expected a batch transition");
            };
            let transition = batch.transitions_iter().next().expect("one transition");
            // Every one of the four pays `amount` into the pool, so the bundle's value
            // balance is the same. A claim states no amount of its own — consensus resolves
            // it against state — so it has to come from what the builder was handed.
            let value_balance = -(amount as i64);
            let (actions, anchor, proof, binding_signature) = match transition {
                BatchedTransitionRef::Token(TokenTransition::Shield(t)) => {
                    (t.actions(), t.anchor(), t.proof(), t.binding_signature())
                }
                BatchedTransitionRef::Token(TokenTransition::MintToPool(t)) => {
                    (t.actions(), t.anchor(), t.proof(), t.binding_signature())
                }
                BatchedTransitionRef::Token(TokenTransition::ClaimToPool(t)) => {
                    (t.actions(), t.anchor(), t.proof(), t.binding_signature())
                }
                BatchedTransitionRef::Token(TokenTransition::DirectPurchaseToPool(t)) => {
                    (t.actions(), t.anchor(), t.proof(), t.binding_signature())
                }
                other => panic!("unexpected transition {other:?}"),
            };

            // The preimage the verifier for this kind rebuilds, nothing the builder handed us.
            let expected = token_pool_output_only_extra_sighash_data(
                action_type,
                &token_id.to_buffer(),
                &owner.id().to_buffer(),
                platform_version,
            )
            .expect("outputs-only kind");

            reconstruct_and_verify_bundle(
                actions,
                FLAGS_OUTPUTS_ONLY,
                value_balance,
                anchor,
                proof,
                binding_signature,
                &expected,
            )
            .unwrap_or_else(|error| {
                panic!("{action_type} builder does not bind its verifier's preimage: {error:?}")
            });
        }
    }
}

/// An outputs-only bundle offered to the same pool, as the same kind, a second time.
///
/// The sighash of such a bundle binds its pool, its kind and its owner, but nothing that makes
/// it single-use, so its owner can submit the same bundle again in a new transition. The repeat
/// would land a second note with the same commitment and the same `rho`, hence the same
/// nullifier, and only one of the two could ever be spent: a wallet counting by commitment
/// would see a payment that does not exist. The pool records each bundle's dummy nullifiers
/// when it enters and refuses a bundle whose dummy nullifier is already there.
///
/// Somebody else's copy is refused by the sighash instead, before it can land; those tests are
/// the front-running ones below. Where the pool's record is read first, the record refuses a
/// copy too: block validation checks the dummy nullifiers before it verifies the bundle, and
/// CheckTx cannot verify a claim's bundle at all, so another claimant's copy of a claim that
/// has already landed is refused on its nullifier.
///
/// Every test first builds the original through the public builder and shows it admitted by
/// CheckTx and executed in a block. Without that, a refused repeat would prove nothing: a
/// bundle the verifier never accepted anywhere is refused for any reason at all. The repeat
/// must then fail on the original's first dummy nullifier, and nothing it carries may move.
mod token_pool_outputs_only_copy_tests {
    use super::token_pool_paid_transitions_tests::{
        credit_pool_balance, fund_credit_pool, spendable, wallet_address, Prover, CREDIT_NOTE,
    };
    use super::token_shielded_pool_tests::{
        assert_check_tx_accepts, assert_check_tx_rejects, assert_tokens_conserved,
        build_shield_bundle, build_spend_bundle,
        enable_shielded_pool, identity_token_balance, nullifier_is_spent,
        platform_with_latest_version, pool_balance, pool_notes_count, process, spend_keys,
        OWNER_INITIAL_BALANCE,
    };
    use super::*;
    use crate::platform_types::state_transitions_processing_result::StateTransitionsProcessingResult;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::fast_forward_to_block::fast_forward_to_block;
    use crate::execution::validation::state_transition::state_transitions::test_helpers::set_pool_total_balance;
    use crate::test::helpers::setup::TempPlatform;
    use dpp::block::epoch::Epoch;
    use dpp::data_contract::associated_token::token_configuration::accessors::v1::TokenConfigurationV1Setters;
    use dpp::data_contract::associated_token::token_distribution_key::TokenDistributionType;
    use dpp::data_contract::associated_token::token_pre_programmed_distribution::v0::TokenPreProgrammedDistributionV0;
    use dpp::data_contract::associated_token::token_pre_programmed_distribution::TokenPreProgrammedDistribution;
    use dpp::fee::Credits;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::identity_nonce::IDENTITY_NONCE_VALUE_FILTER;
    use dpp::shielded::serialized_actions_digest;
    use dpp::shielded::builder::{
        build_token_claim_to_pool_transition, build_token_direct_purchase_to_pool_transition,
        build_token_mint_to_pool_transition, build_token_purchase_from_shielded_pool_transition,
        build_token_shield_transition, ShieldedFeePayer,
    };
    use dpp::shielded::{
        compute_token_purchase_from_shielded_pool_fee, token_pool_fee_bundle_extra_sighash_data,
        OrchardBundleParams, TOKEN_PURCHASE_FROM_SHIELDED_POOL_TYPE,
    };
    use dpp::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
    use dpp::state_transition::batch_transition::batched_transition::token_transition::{
        TokenTransition, TokenTransitionV0Methods,
    };
    use dpp::state_transition::batch_transition::batched_transition::token_transition_action_type::TokenTransitionActionType;
    use dpp::state_transition::batch_transition::batched_transition::BatchedTransitionRef;
    use dpp::state_transition::batch_transition::token_claim_to_pool_transition::v0::v0_methods::TokenClaimToPoolTransitionV0Methods;
    use dpp::state_transition::batch_transition::token_direct_purchase_to_pool_transition::v0::v0_methods::TokenDirectPurchaseToPoolTransitionV0Methods;
    use dpp::state_transition::batch_transition::token_mint_to_pool_transition::v0::v0_methods::TokenMintToPoolTransitionV0Methods;
    use dpp::state_transition::batch_transition::TokenMintToPoolTransition;
    use dpp::state_transition::batch_transition::token_shield_transition::v0::v0_methods::TokenShieldTransitionV0Methods;
    use dpp::state_transition::token_purchase_from_shielded_pool_transition::v0::TokenPurchaseFromShieldedPoolTransitionV0;
    use dpp::state_transition::token_purchase_from_shielded_pool_transition::TokenPurchaseFromShieldedPoolTransition;
    use dpp::state_transition::StateTransition;
    use dpp::tokens::token_pricing_schedule::TokenPricingSchedule;
    use platform_version::version::PlatformVersion;

    fn total_supply(platform: &TempPlatform<MockCoreRPCLike>, token_id: Identifier) -> u64 {
        platform
            .drive
            .fetch_token_total_supply(token_id.to_buffer(), None, PlatformVersion::latest())
            .expect("total supply")
            .expect("supply exists")
    }

    /// The proven bundle a builder put into an outputs-only batch token transition.
    fn proven_bundle(transition: &StateTransition) -> OrchardBundleParams {
        let StateTransition::Batch(batch) = transition else {
            panic!("expected a batch transition");
        };
        let (actions, anchor, proof, binding_signature) =
            match batch.transitions_iter().next().expect("one transition") {
                BatchedTransitionRef::Token(TokenTransition::Shield(t)) => {
                    (t.actions(), t.anchor(), t.proof(), t.binding_signature())
                }
                BatchedTransitionRef::Token(TokenTransition::MintToPool(t)) => {
                    (t.actions(), t.anchor(), t.proof(), t.binding_signature())
                }
                BatchedTransitionRef::Token(TokenTransition::ClaimToPool(t)) => {
                    (t.actions(), t.anchor(), t.proof(), t.binding_signature())
                }
                BatchedTransitionRef::Token(TokenTransition::DirectPurchaseToPool(t)) => {
                    (t.actions(), t.anchor(), t.proof(), t.binding_signature())
                }
                other => panic!("not an outputs-only token pool transition: {other:?}"),
            };
        OrchardBundleParams {
            actions: actions.to_vec(),
            anchor: *anchor,
            proof: proof.to_vec(),
            binding_signature: *binding_signature,
        }
    }

    /// The original was admitted and executed: every one of its dummy nullifiers is now in the
    /// pool, which is the record a copy is checked against.
    fn assert_dummy_nullifiers_recorded(
        platform: &TempPlatform<MockCoreRPCLike>,
        token_id: Identifier,
        bundle: &OrchardBundleParams,
    ) {
        for action in &bundle.actions {
            assert!(
                nullifier_is_spent(platform, token_id, &action.nullifier),
                "an outputs-only bundle's dummy nullifiers must be recorded when it enters the pool"
            );
        }
    }

    fn credits(platform: &TempPlatform<MockCoreRPCLike>, identity_id: Identifier) -> Credits {
        platform
            .drive
            .fetch_identity_balance(identity_id.to_buffer(), None, PlatformVersion::latest())
            .expect("balance")
            .expect("identity exists")
    }

    /// The identity contract nonce `identity_id` has used last on `contract_id`.
    fn contract_nonce(
        platform: &TempPlatform<MockCoreRPCLike>,
        identity_id: Identifier,
        contract_id: Identifier,
    ) -> u64 {
        platform
            .drive
            .fetch_identity_contract_nonce(
                identity_id.to_buffer(),
                contract_id.to_buffer(),
                true,
                None,
                PlatformVersion::latest(),
            )
            .expect("identity contract nonce")
            .expect("the identity has used the contract")
            & IDENTITY_NONCE_VALUE_FILTER
    }

    /// Submits a bundle again after it has landed and returns what the submitter was charged for
    /// it.
    ///
    /// CheckTx admits it: it does not read the pool. Where CheckTx verifies the kind's bundle and
    /// the submitter is the bundle's owner, that admission also shows the bundle is valid as the
    /// submitter's transition; a claim's bundle CheckTx does not verify at all. Block execution
    /// must then refuse it on the original's first dummy nullifier, which it checks before the
    /// proof, as a paid failure: the submitter's nonce advances to `nonce` and its fee is
    /// charged, nothing else moves.
    fn assert_repeat_refused_in_block(
        platform: &TempPlatform<MockCoreRPCLike>,
        repeat: &StateTransition,
        submitter_id: Identifier,
        contract_id: Identifier,
        nonce: u64,
        original: &OrchardBundleParams,
        block_info: Option<&BlockInfo>,
    ) -> Credits {
        assert_check_tx_accepts(platform, repeat);
        let credits_before = credits(platform, submitter_id);
        let result = match block_info {
            Some(block_info) => process_at(platform, repeat, block_info),
            None => process(platform, repeat),
        };
        let first = original.actions[0].nullifier;
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(StateError::NullifierAlreadySpentError(error)),
                ..
            }] if error.nullifier() == first
        );
        assert_eq!(
            contract_nonce(platform, submitter_id, contract_id),
            nonce,
            "the refused repeat bumps the submitter's nonce"
        );
        let charged = credits_before - credits(platform, submitter_id);
        assert!(charged > 0, "a refused repeat is a paid failure");
        charged
    }

    /// Submits another identity's bundle, taken from the mempool before the original landed,
    /// and returns what the copier was charged for it.
    ///
    /// The bundle's sighash names its author, so the copy is invalid as the copier's transition
    /// wherever the bundle is verified: CheckTx refuses it when `verified_at_check_tx`, and
    /// block execution refuses it as a paid failure — the copier's nonce advances to `nonce` and
    /// its fee is charged.
    ///
    /// The two nullifier assertions do different jobs, and neither pins the order of checks:
    ///
    /// - Before the copy, none of the bundle's dummy nullifiers is in the pool. This is what
    ///   anchors the negative: at the moment of refusal the pool's record holds nothing to
    ///   refuse with, so the refusal can only be the preimage.
    /// - After the copy, still none is. A refused transition never reaches the pool write, so
    ///   this does not catch a binding check moved after the nullifier check. It guards a
    ///   different regression: recording the nullifiers during validation, ahead of the write,
    ///   which would let a copier burn the author's nullifiers and block the author for good.
    ///
    /// What shows nothing of the copy persisted is the caller's next step: the original then
    /// executes successfully.
    fn assert_front_run_refused(
        platform: &TempPlatform<MockCoreRPCLike>,
        copy: &StateTransition,
        copier_id: Identifier,
        contract_id: Identifier,
        nonce: u64,
        bundle: &OrchardBundleParams,
        verified_at_check_tx: bool,
        block_info: Option<&BlockInfo>,
    ) -> Credits {
        let token_id = copied_token_id(copy);
        assert!(
            bundle.actions.iter().all(|action| !nullifier_is_spent(
                platform,
                token_id,
                &action.nullifier
            )),
            "a front-run lands before the original, so none of its dummy nullifiers is recorded"
        );
        if verified_at_check_tx {
            assert_matches!(
                assert_check_tx_rejects(platform, copy).as_slice(),
                [ConsensusError::StateError(
                    StateError::InvalidShieldedProofError(_)
                )]
            );
        } else {
            assert_check_tx_accepts(platform, copy);
        }
        let credits_before = credits(platform, copier_id);
        let result = match block_info {
            Some(block_info) => process_at(platform, copy, block_info),
            None => process(platform, copy),
        };
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(StateError::InvalidShieldedProofError(_)),
                ..
            }]
        );
        assert_eq!(contract_nonce(platform, copier_id, contract_id), nonce);
        let charged = credits_before - credits(platform, copier_id);
        assert!(charged > 0, "a refused copy is a paid failure");
        assert!(
            bundle.actions.iter().all(|action| !nullifier_is_spent(
                platform,
                token_id,
                &action.nullifier
            )),
            "a refused copy must not record the bundle's dummy nullifiers"
        );
        charged
    }

    /// The token an outputs-only batch token transition pays into.
    fn copied_token_id(transition: &StateTransition) -> Identifier {
        let StateTransition::Batch(batch) = transition else {
            panic!("expected a batch transition");
        };
        match batch.transitions_iter().next().expect("one transition") {
            BatchedTransitionRef::Token(token_transition) => token_transition.token_id(),
            other => panic!("not a token transition: {other:?}"),
        }
    }

    /// Processes one transition at `block_info`, for a claim whose release has to be due.
    fn process_at(
        platform: &TempPlatform<MockCoreRPCLike>,
        transition: &StateTransition,
        block_info: &BlockInfo,
    ) -> StateTransitionsProcessingResult {
        process_in_one_block(platform, &[transition], block_info)
    }

    /// Processes `transitions` in order in one block transaction, then commits it: each is
    /// validated against what the ones before it wrote, not only against committed state.
    fn process_in_one_block(
        platform: &TempPlatform<MockCoreRPCLike>,
        transitions: &[&StateTransition],
        block_info: &BlockInfo,
    ) -> StateTransitionsProcessingResult {
        let platform_version = PlatformVersion::latest();
        let platform_state = platform.state.load();
        let serialized: Vec<Vec<u8>> = transitions
            .iter()
            .map(|transition| transition.serialize_to_bytes().expect("serialize"))
            .collect();
        let transaction = platform.drive.grove.start_transaction();
        let result = platform
            .platform
            .process_raw_state_transitions(
                &serialized,
                &platform_state,
                block_info,
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

    /// The owner submits its own landed bundle again with a fresh nonce, as a wallet retrying a
    /// submission it believes lost would. The owner binding cannot tell the two apart, so the
    /// pool's record refuses the repeat, and it is a paid failure: the retry is charged its fee,
    /// though nothing else moves.
    #[tokio::test]
    async fn test_a_shield_bundle_resubmitted_by_its_owner_is_refused() {
        let platform_version = PlatformVersion::latest();
        let mut platform = platform_with_latest_version();
        let mut rng = StdRng::seed_from_u64(9501);
        let amount = 4_200;

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

        let shield = build_token_shield_transition(
            token_id,
            owner.id(),
            contract.id(),
            0,
            &wallet_address(),
            amount,
            [0u8; 36],
            None,
            &key,
            2,
            0,
            &signer,
            &Prover,
            platform_version,
        )
        .await
        .expect("client-built token shield");
        assert_check_tx_accepts(&platform, &shield);
        assert_matches!(
            process(&platform, &shield).execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        assert_eq!(
            identity_token_balance(&platform, token_id, owner.id()),
            Some(OWNER_INITIAL_BALANCE - amount)
        );
        let bundle = proven_bundle(&shield);
        assert_dummy_nullifiers_recorded(&platform, token_id, &bundle);
        let notes_after_original = pool_notes_count(&platform, token_id);

        // The owner still holds far more than the repeat shields, so a refusal cannot be its
        // balance.
        let repeat = BatchTransition::new_token_shield_transition(
            token_id,
            owner.id(),
            contract.id(),
            0,
            amount,
            bundle.clone(),
            &key,
            3,
            0,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("token shield transition");
        assert_repeat_refused_in_block(
            &platform,
            &repeat,
            owner.id(),
            contract.id(),
            3,
            &bundle,
            None,
        );

        assert_eq!(pool_balance(&platform, token_id), amount);
        assert_eq!(pool_notes_count(&platform, token_id), notes_after_original);
        assert_eq!(
            identity_token_balance(&platform, token_id, owner.id()),
            Some(OWNER_INITIAL_BALANCE - amount)
        );

        // A pool that already holds recorded dummy nullifiers still takes a new bundle from the
        // same owner: what is refused is the repeat, not every bundle after the first.
        let own_shield = build_token_shield_transition(
            token_id,
            owner.id(),
            contract.id(),
            0,
            &wallet_address(),
            amount,
            [0u8; 36],
            None,
            &key,
            4,
            0,
            &signer,
            &Prover,
            platform_version,
        )
        .await
        .expect("client-built token shield");
        assert_check_tx_accepts(&platform, &own_shield);
        assert_matches!(
            process(&platform, &own_shield)
                .execution_results()
                .as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        assert_eq!(pool_balance(&platform, token_id), 2 * amount);
        assert_dummy_nullifiers_recorded(&platform, token_id, &proven_bundle(&own_shield));
        assert_tokens_conserved(&platform);
    }

    /// A repeat submitted before its original has landed reaches the same block. CheckTx admits
    /// both, since neither has entered the pool yet, so what refuses the second is the check
    /// reading the first one's write in the block's own transaction.
    #[tokio::test]
    async fn test_a_shield_bundle_and_its_repeat_in_one_block_land_only_once() {
        let platform_version = PlatformVersion::latest();
        let mut platform = platform_with_latest_version();
        let mut rng = StdRng::seed_from_u64(9508);
        let amount = 3_100;

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

        let shield = build_token_shield_transition(
            token_id,
            owner.id(),
            contract.id(),
            0,
            &wallet_address(),
            amount,
            [0u8; 36],
            None,
            &key,
            2,
            0,
            &signer,
            &Prover,
            platform_version,
        )
        .await
        .expect("client-built token shield");
        let bundle = proven_bundle(&shield);
        let repeat = BatchTransition::new_token_shield_transition(
            token_id,
            owner.id(),
            contract.id(),
            0,
            amount,
            bundle.clone(),
            &key,
            3,
            0,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("token shield transition");
        assert_check_tx_accepts(&platform, &shield);
        assert_check_tx_accepts(&platform, &repeat);

        let first = bundle.actions[0].nullifier;
        assert_matches!(
            process_in_one_block(&platform, &[&shield, &repeat], &BlockInfo::default())
                .execution_results()
                .as_slice(),
            [
                StateTransitionExecutionResult::SuccessfulExecution { .. },
                StateTransitionExecutionResult::PaidConsensusError {
                    error: ConsensusError::StateError(StateError::NullifierAlreadySpentError(error)),
                    ..
                },
            ] if error.nullifier() == first
        );

        assert_dummy_nullifiers_recorded(&platform, token_id, &bundle);
        assert_eq!(pool_balance(&platform, token_id), amount);
        assert_eq!(
            pool_notes_count(&platform, token_id),
            bundle.actions.len() as u64
        );
        assert_eq!(
            identity_token_balance(&platform, token_id, owner.id()),
            Some(OWNER_INITIAL_BALANCE - amount)
        );
        assert_tokens_conserved(&platform);
    }

    #[tokio::test]
    async fn test_a_mint_to_pool_bundle_resubmitted_by_its_minter_is_refused() {
        let platform_version = PlatformVersion::latest();
        let mut platform = platform_with_latest_version();
        let mut rng = StdRng::seed_from_u64(9502);
        let amount = 1_337;

        let (minter, signer, key) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        // A group the minter alone can close, so each proposal mints at once and the repeat is
        // a complete mint of its own.
        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            minter.id(),
            Some(|configuration: &mut TokenConfiguration| {
                enable_shielded_pool(configuration);
                configuration.set_manual_minting_rules(ChangeControlRules::V0(
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
                        members: [(minter.id(), 1)].into(),
                        required_power: 1,
                    }),
                )]
                .into(),
            ),
            None,
            platform_version,
        );

        let mint = build_token_mint_to_pool_transition(
            token_id,
            minter.id(),
            contract.id(),
            0,
            &wallet_address(),
            amount,
            [0u8; 36],
            None,
            None,
            Some(GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(0)),
            &key,
            2,
            0,
            &signer,
            &Prover,
            platform_version,
        )
        .await
        .expect("client-built token mint to pool");
        assert_check_tx_accepts(&platform, &mint);
        assert_matches!(
            process(&platform, &mint).execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        assert_eq!(pool_balance(&platform, token_id), amount);
        let bundle = proven_bundle(&mint);
        assert_dummy_nullifiers_recorded(&platform, token_id, &bundle);
        let notes_after_original = pool_notes_count(&platform, token_id);

        let repeat = BatchTransition::new_token_mint_to_pool_transition(
            token_id,
            minter.id(),
            contract.id(),
            0,
            amount,
            bundle.clone(),
            None,
            Some(GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(0)),
            &key,
            3,
            0,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("token mint to pool transition");
        assert_repeat_refused_in_block(
            &platform,
            &repeat,
            minter.id(),
            contract.id(),
            3,
            &bundle,
            None,
        );

        assert_eq!(pool_balance(&platform, token_id), amount);
        assert_eq!(pool_notes_count(&platform, token_id), notes_after_original);
        assert_eq!(
            total_supply(&platform, token_id),
            OWNER_INITIAL_BALANCE + amount,
            "a refused repeat must not print supply"
        );
        assert_tokens_conserved(&platform);
    }

    /// Another member of the group takes a proposal's bundle and proposes it as its own. That
    /// member is as entitled to mint as the proposer, and the proposal it copied has not closed,
    /// so no dummy nullifier is on record to refuse it: only the minter the bundle's sighash
    /// names can. Were it accepted, the group could close either proposal, and whichever closed
    /// second would be refused as a repeat.
    #[tokio::test]
    async fn test_a_mint_to_pool_bundle_proposed_again_by_another_member_is_refused() {
        let platform_version = PlatformVersion::latest();
        let mut platform = platform_with_latest_version();
        let mut rng = StdRng::seed_from_u64(9512);
        let amount = 1_111;

        let (proposer, proposer_signer, proposer_key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (copier, copier_signer, copier_key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (confirmer, confirmer_signer, confirmer_key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            proposer.id(),
            Some(|configuration: &mut TokenConfiguration| {
                enable_shielded_pool(configuration);
                configuration.set_manual_minting_rules(ChangeControlRules::V0(
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
                        members: [(proposer.id(), 1), (copier.id(), 1), (confirmer.id(), 1)].into(),
                        required_power: 2,
                    }),
                )]
                .into(),
            ),
            None,
            platform_version,
        );
        let supply_before = total_supply(&platform, token_id);

        let proposer_nonce = 2;
        let proposal = build_token_mint_to_pool_transition(
            token_id,
            proposer.id(),
            contract.id(),
            0,
            &wallet_address(),
            amount,
            [0u8; 36],
            None,
            None,
            Some(GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(0)),
            &proposer_key,
            proposer_nonce,
            0,
            &proposer_signer,
            &Prover,
            platform_version,
        )
        .await
        .expect("client-built token mint to pool proposal");
        assert_check_tx_accepts(&platform, &proposal);
        assert_matches!(
            process(&platform, &proposal).execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        let bundle = proven_bundle(&proposal);

        let copy = BatchTransition::new_token_mint_to_pool_transition(
            token_id,
            copier.id(),
            contract.id(),
            0,
            amount,
            bundle.clone(),
            None,
            Some(GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(0)),
            &copier_key,
            1,
            0,
            &copier_signer,
            platform_version,
            None,
        )
        .await
        .expect("token mint to pool proposal");
        assert_front_run_refused(
            &platform,
            &copy,
            copier.id(),
            contract.id(),
            1,
            &bundle,
            true,
            None,
        );
        // One signature of two could not mint either way, so the pool and the supply cannot tell
        // a refused proposal from an accepted one; the error asserted above is what does.

        // The proposal the copier took from still closes with its own bundle.
        let confirmation = BatchTransition::new_token_mint_to_pool_transition(
            token_id,
            confirmer.id(),
            contract.id(),
            0,
            amount,
            bundle.clone(),
            None,
            Some(
                GroupStateTransitionInfoStatus::GroupStateTransitionInfoOtherSigner(
                    GroupStateTransitionInfo {
                        group_contract_position: 0,
                        action_id: TokenMintToPoolTransition::calculate_action_id_with_fields(
                            token_id.as_bytes(),
                            proposer.id().as_bytes(),
                            proposer_nonce,
                            amount,
                            &serialized_actions_digest(&bundle.actions),
                        ),
                        action_is_proposer: false,
                    },
                ),
            ),
            &confirmer_key,
            1,
            0,
            &confirmer_signer,
            platform_version,
            None,
        )
        .await
        .expect("token mint to pool confirmation");
        assert_check_tx_accepts(&platform, &confirmation);
        assert_matches!(
            process(&platform, &confirmation)
                .execution_results()
                .as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        assert_eq!(pool_balance(&platform, token_id), amount);
        assert_eq!(total_supply(&platform, token_id), supply_before + amount);
        assert_dummy_nullifiers_recorded(&platform, token_id, &bundle);
        assert_tokens_conserved(&platform);
    }

    /// A group mint carries one bundle through several signers: every confirmer resubmits the
    /// proposer's bundle unchanged, so its dummy nullifiers are offered to the pool once per
    /// signer. Only the signature that closes the action mints, and only it may record them;
    /// were they recorded earlier, the closing signature would be refused as a copy and no
    /// group mint into a pool could ever complete.
    ///
    /// The bundle's sighash names the proposer, not the confirmer whose batch carries it here.
    /// Admission must therefore leave a confirmer's bundle to state validation, which reads the
    /// proposer from the stored group action; were either of the two to bind the batch owner
    /// instead, this confirmation would be refused and no group mint could ever close.
    #[tokio::test]
    async fn test_a_group_mint_to_pool_closes_with_the_proposers_bundle_resubmitted() {
        let platform_version = PlatformVersion::latest();
        let mut platform = platform_with_latest_version();
        let mut rng = StdRng::seed_from_u64(9507);
        let amount = 2_024;

        let (proposer, proposer_signer, proposer_key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (confirmer, confirmer_signer, confirmer_key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            proposer.id(),
            Some(|configuration: &mut TokenConfiguration| {
                enable_shielded_pool(configuration);
                configuration.set_manual_minting_rules(ChangeControlRules::V0(
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

        let proposer_nonce = 2;
        let proposal = build_token_mint_to_pool_transition(
            token_id,
            proposer.id(),
            contract.id(),
            0,
            &wallet_address(),
            amount,
            [0u8; 36],
            None,
            None,
            Some(GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(0)),
            &proposer_key,
            proposer_nonce,
            0,
            &proposer_signer,
            &Prover,
            platform_version,
        )
        .await
        .expect("client-built token mint to pool proposal");
        assert_check_tx_accepts(&platform, &proposal);
        assert_matches!(
            process(&platform, &proposal).execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        let bundle = proven_bundle(&proposal);
        // One signature of two: nothing is minted and nothing is recorded yet.
        assert_eq!(pool_balance(&platform, token_id), 0);
        assert!(!nullifier_is_spent(
            &platform,
            token_id,
            &bundle.actions[0].nullifier
        ));

        let action_id = TokenMintToPoolTransition::calculate_action_id_with_fields(
            token_id.as_bytes(),
            proposer.id().as_bytes(),
            proposer_nonce,
            amount,
            &serialized_actions_digest(&bundle.actions),
        );
        let confirmation = BatchTransition::new_token_mint_to_pool_transition(
            token_id,
            confirmer.id(),
            contract.id(),
            0,
            amount,
            bundle.clone(),
            None,
            Some(
                GroupStateTransitionInfoStatus::GroupStateTransitionInfoOtherSigner(
                    GroupStateTransitionInfo {
                        group_contract_position: 0,
                        action_id,
                        action_is_proposer: false,
                    },
                ),
            ),
            &confirmer_key,
            1,
            0,
            &confirmer_signer,
            platform_version,
            None,
        )
        .await
        .expect("token mint to pool confirmation");
        assert_check_tx_accepts(&platform, &confirmation);
        assert_matches!(
            process(&platform, &confirmation)
                .execution_results()
                .as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        assert_eq!(pool_balance(&platform, token_id), amount);
        assert_eq!(
            total_supply(&platform, token_id),
            OWNER_INITIAL_BALANCE + amount
        );
        assert_dummy_nullifiers_recorded(&platform, token_id, &bundle);
        assert_tokens_conserved(&platform);
    }

    /// The group action pins only the digest of the bundle's actions, not its proof or binding
    /// signature, and the stateless proof check skips a confirmer's bundle because it cannot
    /// see the proposer the bundle is bound to. State validation is therefore the only place a
    /// confirmer's bundle is verified: a confirmer that keeps the proposer's actions but swaps
    /// the proof must be refused there, or the group would mint against a bundle nobody proved.
    #[tokio::test]
    async fn test_a_group_mint_to_pool_confirmation_with_a_tampered_proof_is_refused() {
        let platform_version = PlatformVersion::latest();
        let mut platform = platform_with_latest_version();
        let mut rng = StdRng::seed_from_u64(9515);
        let amount = 3_003;

        let (proposer, proposer_signer, proposer_key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (tamperer, tamperer_signer, tamperer_key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (confirmer, confirmer_signer, confirmer_key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            proposer.id(),
            Some(|configuration: &mut TokenConfiguration| {
                enable_shielded_pool(configuration);
                configuration.set_manual_minting_rules(ChangeControlRules::V0(
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
                        members: [(proposer.id(), 1), (tamperer.id(), 1), (confirmer.id(), 1)]
                            .into(),
                        required_power: 2,
                    }),
                )]
                .into(),
            ),
            None,
            platform_version,
        );
        let supply_before = total_supply(&platform, token_id);

        let proposer_nonce = 2;
        let proposal = build_token_mint_to_pool_transition(
            token_id,
            proposer.id(),
            contract.id(),
            0,
            &wallet_address(),
            amount,
            [0u8; 36],
            None,
            None,
            Some(GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(0)),
            &proposer_key,
            proposer_nonce,
            0,
            &proposer_signer,
            &Prover,
            platform_version,
        )
        .await
        .expect("client-built token mint to pool proposal");
        assert_check_tx_accepts(&platform, &proposal);
        assert_matches!(
            process(&platform, &proposal).execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        let bundle = proven_bundle(&proposal);
        let closing_signature = || {
            GroupStateTransitionInfoStatus::GroupStateTransitionInfoOtherSigner(
                GroupStateTransitionInfo {
                    group_contract_position: 0,
                    action_id: TokenMintToPoolTransition::calculate_action_id_with_fields(
                        token_id.as_bytes(),
                        proposer.id().as_bytes(),
                        proposer_nonce,
                        amount,
                        &serialized_actions_digest(&bundle.actions),
                    ),
                    action_is_proposer: false,
                },
            )
        };

        // Same actions, so the group action's digest still matches; only the proof differs.
        let mut tampered = bundle.clone();
        tampered.proof[0] ^= 0x01;
        let tampered_confirmation = BatchTransition::new_token_mint_to_pool_transition(
            token_id,
            tamperer.id(),
            contract.id(),
            0,
            amount,
            tampered,
            None,
            Some(closing_signature()),
            &tamperer_key,
            1,
            0,
            &tamperer_signer,
            platform_version,
            None,
        )
        .await
        .expect("token mint to pool confirmation");
        // CheckTx cannot tell whose sighash a confirmer's bundle is bound to, so it does not
        // verify it at all: a bundle that no verifier would accept is admitted to the mempool,
        // and the block is where it is refused and charged.
        assert_check_tx_accepts(&platform, &tampered_confirmation);
        assert_matches!(
            process(&platform, &tampered_confirmation)
                .execution_results()
                .as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(StateError::InvalidShieldedProofError(_)),
                ..
            }]
        );
        assert_eq!(pool_balance(&platform, token_id), 0);
        assert_eq!(
            total_supply(&platform, token_id),
            supply_before,
            "a refused confirmation would otherwise have closed the action and minted"
        );
        assert!(!nullifier_is_spent(
            &platform,
            token_id,
            &bundle.actions[0].nullifier
        ));

        // The untampered bundle still closes the same action.
        let confirmation = BatchTransition::new_token_mint_to_pool_transition(
            token_id,
            confirmer.id(),
            contract.id(),
            0,
            amount,
            bundle.clone(),
            None,
            Some(closing_signature()),
            &confirmer_key,
            1,
            0,
            &confirmer_signer,
            platform_version,
            None,
        )
        .await
        .expect("token mint to pool confirmation");
        assert_check_tx_accepts(&platform, &confirmation);
        assert_matches!(
            process(&platform, &confirmation)
                .execution_results()
                .as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        assert_eq!(pool_balance(&platform, token_id), amount);
        assert_eq!(total_supply(&platform, token_id), supply_before + amount);
        assert_dummy_nullifiers_recorded(&platform, token_id, &bundle);
        assert_tokens_conserved(&platform);
    }

    /// A signer other than the proposer proves the proposer's notes again, from the same
    /// randomness, but bound to itself. It cannot then match the group action: every action of
    /// an outputs-only bundle carries a spend authorization signature over the sighash, and the
    /// group action's digest covers those signatures, so a bundle bound to another owner has
    /// other actions. Keeping the proposer's actions verbatim and swapping in the rest of its own
    /// bundle matches the digest instead, but then the actions' signatures only verify against
    /// the proposer's sighash and the binding signature only against its own. Either way the
    /// only bundle a signer other than the proposer can close the action with is the
    /// proposer's own.
    #[tokio::test]
    async fn test_a_group_mint_to_pool_confirmation_with_a_bundle_proved_for_the_confirmer_is_refused(
    ) {
        let platform_version = PlatformVersion::latest();
        let mut platform = platform_with_latest_version();
        let mut rng = StdRng::seed_from_u64(9518);
        let amount = 2_718;
        let bundle_seed = 77;

        let (proposer, proposer_signer, proposer_key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (rebinder, rebinder_signer, rebinder_key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (confirmer, confirmer_signer, confirmer_key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            proposer.id(),
            Some(|configuration: &mut TokenConfiguration| {
                enable_shielded_pool(configuration);
                configuration.set_manual_minting_rules(ChangeControlRules::V0(
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
                        members: [(proposer.id(), 1), (rebinder.id(), 1), (confirmer.id(), 1)]
                            .into(),
                        required_power: 2,
                    }),
                )]
                .into(),
            ),
            None,
            platform_version,
        );
        let supply_before = total_supply(&platform, token_id);

        // Seeded, so the same notes can be proven a second time; the public builder draws fresh
        // randomness and could not reproduce them.
        let bundle = build_shield_bundle(
            amount,
            bundle_seed,
            TokenTransitionActionType::MintToPool,
            token_id,
            proposer.id(),
        );
        let proposer_nonce = 2;
        let proposal = BatchTransition::new_token_mint_to_pool_transition(
            token_id,
            proposer.id(),
            contract.id(),
            0,
            amount,
            bundle.clone(),
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
        .expect("token mint to pool proposal");
        assert_check_tx_accepts(&platform, &proposal);
        assert_matches!(
            process(&platform, &proposal).execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );

        let rebound = build_shield_bundle(
            amount,
            bundle_seed,
            TokenTransitionActionType::MintToPool,
            token_id,
            rebinder.id(),
        );
        assert_eq!(rebound.actions.len(), bundle.actions.len());
        for (own, proposers) in rebound.actions.iter().zip(&bundle.actions) {
            assert_eq!(
                (
                    own.nullifier,
                    own.rk,
                    own.cmx,
                    &own.encrypted_note,
                    own.cv_net
                ),
                (
                    proposers.nullifier,
                    proposers.rk,
                    proposers.cmx,
                    &proposers.encrypted_note,
                    proposers.cv_net
                ),
                "the same randomness proves the same notes"
            );
            assert_ne!(
                own.spend_auth_sig, proposers.spend_auth_sig,
                "each action signs the sighash, which names the owner"
            );
        }
        assert_ne!(
            serialized_actions_digest(&rebound.actions),
            serialized_actions_digest(&bundle.actions)
        );

        let closing_signature = || {
            GroupStateTransitionInfoStatus::GroupStateTransitionInfoOtherSigner(
                GroupStateTransitionInfo {
                    group_contract_position: 0,
                    action_id: TokenMintToPoolTransition::calculate_action_id_with_fields(
                        token_id.as_bytes(),
                        proposer.id().as_bytes(),
                        proposer_nonce,
                        amount,
                        &serialized_actions_digest(&bundle.actions),
                    ),
                    action_is_proposer: false,
                },
            )
        };
        let confirm_with = |bundle: OrchardBundleParams, nonce| {
            BatchTransition::new_token_mint_to_pool_transition(
                token_id,
                rebinder.id(),
                contract.id(),
                0,
                amount,
                bundle,
                None,
                Some(closing_signature()),
                &rebinder_key,
                nonce,
                0,
                &rebinder_signer,
                platform_version,
                None,
            )
        };

        // Its own bundle, whole: the group action refuses other actions.
        let own_bundle = confirm_with(rebound.clone(), 1)
            .await
            .expect("token mint to pool confirmation");
        assert_check_tx_accepts(&platform, &own_bundle);
        assert_matches!(
            process(&platform, &own_bundle)
                .execution_results()
                .as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(
                    StateError::ModificationOfGroupActionMainParametersNotPermittedError(_)
                ),
                ..
            }]
        );

        // The proposer's actions with its own proof and binding signature: the digest matches,
        // and the bundle verifies against neither owner.
        let spliced = OrchardBundleParams {
            actions: bundle.actions.clone(),
            anchor: rebound.anchor,
            proof: rebound.proof.clone(),
            binding_signature: rebound.binding_signature,
        };
        let spliced_confirmation = confirm_with(spliced, 2)
            .await
            .expect("token mint to pool confirmation");
        assert_check_tx_accepts(&platform, &spliced_confirmation);
        assert_matches!(
            process(&platform, &spliced_confirmation)
                .execution_results()
                .as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(StateError::InvalidShieldedProofError(_)),
                ..
            }]
        );

        assert_eq!(pool_balance(&platform, token_id), 0);
        assert_eq!(
            total_supply(&platform, token_id),
            supply_before,
            "either confirmation would otherwise have closed the action and minted"
        );
        assert!(!nullifier_is_spent(
            &platform,
            token_id,
            &bundle.actions[0].nullifier
        ));

        // The proposer's bundle, unchanged, still closes the action.
        let confirmation = BatchTransition::new_token_mint_to_pool_transition(
            token_id,
            confirmer.id(),
            contract.id(),
            0,
            amount,
            bundle.clone(),
            None,
            Some(closing_signature()),
            &confirmer_key,
            1,
            0,
            &confirmer_signer,
            platform_version,
            None,
        )
        .await
        .expect("token mint to pool confirmation");
        assert_check_tx_accepts(&platform, &confirmation);
        assert_matches!(
            process(&platform, &confirmation)
                .execution_results()
                .as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        assert_eq!(pool_balance(&platform, token_id), amount);
        assert_eq!(total_supply(&platform, token_id), supply_before + amount);
        assert_dummy_nullifiers_recorded(&platform, token_id, &bundle);
        assert_tokens_conserved(&platform);
    }

    /// Two group actions can carry the same bundle: its proposer may propose it twice, and the
    /// group action pins only the bundle's digest. Once one of them closes and mints, the
    /// signature that would close the other is a repeat like any other and must be refused, or
    /// the group would mint the amount twice into notes of which only one of each pair could be
    /// spent. Confirmers go through the check as proposers do.
    ///
    /// Both proposals come from the same member on purpose: another member proposing the same
    /// bundle is refused on the owner the bundle's sighash names before it could reach the
    /// nullifier check, and that case lives in
    /// `test_a_mint_to_pool_bundle_proposed_again_by_another_member_is_refused`.
    #[tokio::test]
    async fn test_a_second_group_mint_to_pool_carrying_a_minted_bundle_cannot_close() {
        let platform_version = PlatformVersion::latest();
        let mut platform = platform_with_latest_version();
        let mut rng = StdRng::seed_from_u64(9509);
        let amount = 777;

        let (first, first_signer, first_key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (second, second_signer, second_key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (third, third_signer, third_key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            first.id(),
            Some(|configuration: &mut TokenConfiguration| {
                enable_shielded_pool(configuration);
                configuration.set_manual_minting_rules(ChangeControlRules::V0(
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
                        members: [(first.id(), 1), (second.id(), 1), (third.id(), 1)].into(),
                        required_power: 2,
                    }),
                )]
                .into(),
            ),
            None,
            platform_version,
        );

        let first_nonce = 2;
        let first_proposal = build_token_mint_to_pool_transition(
            token_id,
            first.id(),
            contract.id(),
            0,
            &wallet_address(),
            amount,
            [0u8; 36],
            None,
            None,
            Some(GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(0)),
            &first_key,
            first_nonce,
            0,
            &first_signer,
            &Prover,
            platform_version,
        )
        .await
        .expect("client-built token mint to pool proposal");
        assert_matches!(
            process(&platform, &first_proposal)
                .execution_results()
                .as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        let bundle = proven_bundle(&first_proposal);
        let digest = serialized_actions_digest(&bundle.actions);

        // A second proposal of the same amount and the same bundle, by the same member.
        let second_nonce = 3;
        let second_proposal = BatchTransition::new_token_mint_to_pool_transition(
            token_id,
            first.id(),
            contract.id(),
            0,
            amount,
            bundle.clone(),
            None,
            Some(GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(0)),
            &first_key,
            second_nonce,
            0,
            &first_signer,
            platform_version,
            None,
        )
        .await
        .expect("token mint to pool proposal");
        assert_matches!(
            process(&platform, &second_proposal)
                .execution_results()
                .as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        assert_eq!(pool_balance(&platform, token_id), 0);

        let confirmation = |proposer: Identifier, proposer_nonce| {
            GroupStateTransitionInfoStatus::GroupStateTransitionInfoOtherSigner(
                GroupStateTransitionInfo {
                    group_contract_position: 0,
                    action_id: TokenMintToPoolTransition::calculate_action_id_with_fields(
                        token_id.as_bytes(),
                        proposer.as_bytes(),
                        proposer_nonce,
                        amount,
                        &digest,
                    ),
                    action_is_proposer: false,
                },
            )
        };

        // The third member closes the second proposal: it mints and records the bundle.
        let close_second = BatchTransition::new_token_mint_to_pool_transition(
            token_id,
            third.id(),
            contract.id(),
            0,
            amount,
            bundle.clone(),
            None,
            Some(confirmation(first.id(), second_nonce)),
            &third_key,
            1,
            0,
            &third_signer,
            platform_version,
            None,
        )
        .await
        .expect("token mint to pool confirmation");
        assert_matches!(
            process(&platform, &close_second)
                .execution_results()
                .as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        assert_eq!(pool_balance(&platform, token_id), amount);
        assert_dummy_nullifiers_recorded(&platform, token_id, &bundle);
        let notes_after_mint = pool_notes_count(&platform, token_id);

        // The second member's signature would close the first proposal with the same bundle.
        let close_first = BatchTransition::new_token_mint_to_pool_transition(
            token_id,
            second.id(),
            contract.id(),
            0,
            amount,
            bundle.clone(),
            None,
            Some(confirmation(first.id(), first_nonce)),
            &second_key,
            1,
            0,
            &second_signer,
            platform_version,
            None,
        )
        .await
        .expect("token mint to pool confirmation");
        assert_check_tx_accepts(&platform, &close_first);
        let first_nullifier = bundle.actions[0].nullifier;
        assert_matches!(
            process(&platform, &close_first)
                .execution_results()
                .as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(StateError::NullifierAlreadySpentError(error)),
                ..
            }] if error.nullifier() == first_nullifier
        );

        assert_eq!(pool_balance(&platform, token_id), amount);
        assert_eq!(pool_notes_count(&platform, token_id), notes_after_mint);
        assert_eq!(
            total_supply(&platform, token_id),
            OWNER_INITIAL_BALANCE + amount,
            "the group must mint the bundle once"
        );
        assert_tokens_conserved(&platform);
    }

    /// CheckTx cannot verify a claim's bundle, whose amount is only known against state, so it
    /// admits another claimant's copy; in the block, the original's recorded dummy nullifier
    /// refuses the copy before its proof is looked at.
    #[tokio::test]
    async fn test_a_claim_to_pool_bundle_copied_by_another_claimant_after_it_landed_is_refused() {
        let platform_version = PlatformVersion::latest();
        let mut platform = platform_with_latest_version();
        let mut rng = StdRng::seed_from_u64(9503);
        let release = 445;

        let (owner, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (claimant, signer, key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (copier, copier_signer, copier_key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (claimant_id, copier_id) = (claimant.id(), copier.id());
        // Both are due the same release, so the copier's claim resolves to exactly the amount
        // the copied bundle carries.
        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            owner.id(),
            Some(move |configuration: &mut TokenConfiguration| {
                configuration.set_has_shielded_pool(true);
                configuration
                    .distribution_rules_mut()
                    .set_pre_programmed_distribution(Some(TokenPreProgrammedDistribution::V0(
                        TokenPreProgrammedDistributionV0 {
                            distributions: [(
                                100,
                                [(claimant_id, release), (copier_id, release)].into(),
                            )]
                            .into(),
                        },
                    )));
            }),
            None,
            None,
            None,
            platform_version,
        );

        fast_forward_to_block(&platform, 100, 40, 42, 1, false);
        let block_info = BlockInfo {
            time_ms: 200,
            height: 41,
            core_height: 42,
            epoch: Epoch::new(1).unwrap(),
        };

        let claim = build_token_claim_to_pool_transition(
            token_id,
            claimant_id,
            contract.id(),
            0,
            &wallet_address(),
            release,
            TokenDistributionType::PreProgrammed,
            None,
            [0u8; 36],
            None,
            None,
            &key,
            2,
            0,
            &signer,
            &Prover,
            platform_version,
        )
        .await
        .expect("client-built token claim to pool");
        assert_check_tx_accepts(&platform, &claim);
        assert_matches!(
            process_at(&platform, &claim, &block_info)
                .execution_results()
                .as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        assert_eq!(pool_balance(&platform, token_id), release);
        let bundle = proven_bundle(&claim);
        assert_dummy_nullifiers_recorded(&platform, token_id, &bundle);
        let notes_after_original = pool_notes_count(&platform, token_id);

        let copy = BatchTransition::new_token_claim_to_pool_transition(
            token_id,
            copier_id,
            contract.id(),
            0,
            TokenDistributionType::PreProgrammed,
            None,
            bundle.clone(),
            None,
            &copier_key,
            1,
            0,
            &copier_signer,
            platform_version,
            None,
        )
        .await
        .expect("token claim to pool transition");
        assert_repeat_refused_in_block(
            &platform,
            &copy,
            copier_id,
            contract.id(),
            1,
            &bundle,
            Some(&block_info),
        );
        assert_eq!(pool_balance(&platform, token_id), release);
        assert_eq!(pool_notes_count(&platform, token_id), notes_after_original);
        assert_eq!(
            total_supply(&platform, token_id),
            OWNER_INITIAL_BALANCE + release
        );

        // The copier was due the release all along and can still take it with a bundle of its
        // own: what was refused is the copied bundle, not the claim.
        let own_claim = build_token_claim_to_pool_transition(
            token_id,
            copier_id,
            contract.id(),
            0,
            &wallet_address(),
            release,
            TokenDistributionType::PreProgrammed,
            None,
            [0u8; 36],
            None,
            None,
            &copier_key,
            2,
            0,
            &copier_signer,
            &Prover,
            platform_version,
        )
        .await
        .expect("client-built token claim to pool");
        assert_check_tx_accepts(&platform, &own_claim);
        assert_matches!(
            process_at(&platform, &own_claim, &block_info)
                .execution_results()
                .as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        assert_eq!(pool_balance(&platform, token_id), 2 * release);
        assert_eq!(
            total_supply(&platform, token_id),
            OWNER_INITIAL_BALANCE + 2 * release
        );
        assert_tokens_conserved(&platform);
    }

    #[tokio::test]
    async fn test_a_direct_purchase_to_pool_bundle_resubmitted_by_its_buyer_is_refused() {
        let platform_version = PlatformVersion::latest();
        let mut platform = platform_with_latest_version();
        let mut rng = StdRng::seed_from_u64(9504);
        let (token_count, price) = (3, dash_to_credits!(0.03));

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
        assert_matches!(
            process(&platform, &set_price)
                .execution_results()
                .as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );

        let seller_credits_before = credits(&platform, seller.id());
        let purchase = build_token_direct_purchase_to_pool_transition(
            token_id,
            buyer.id(),
            contract.id(),
            0,
            &wallet_address(),
            token_count,
            price,
            [0u8; 36],
            None,
            &key,
            1,
            0,
            &signer,
            &Prover,
            platform_version,
        )
        .await
        .expect("client-built token direct purchase to pool");
        assert_check_tx_accepts(&platform, &purchase);
        assert_matches!(
            process(&platform, &purchase).execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        assert_eq!(pool_balance(&platform, token_id), token_count);
        let bundle = proven_bundle(&purchase);
        assert_dummy_nullifiers_recorded(&platform, token_id, &bundle);
        let notes_after_original = pool_notes_count(&platform, token_id);
        let seller_credits_after_original = credits(&platform, seller.id());
        assert_eq!(seller_credits_after_original, seller_credits_before + price);

        // The buyer can still afford the price, so a refusal cannot be its credits.
        let repeat = BatchTransition::new_token_direct_purchase_to_pool_transition(
            token_id,
            buyer.id(),
            contract.id(),
            0,
            token_count,
            price,
            bundle.clone(),
            &key,
            2,
            0,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("token direct purchase to pool transition");
        let charged = assert_repeat_refused_in_block(
            &platform,
            &repeat,
            buyer.id(),
            contract.id(),
            2,
            &bundle,
            None,
        );
        assert!(
            charged < price,
            "a refused repeat costs its fee, never the price"
        );

        assert_eq!(pool_balance(&platform, token_id), token_count);
        assert_eq!(pool_notes_count(&platform, token_id), notes_after_original);
        assert_eq!(
            total_supply(&platform, token_id),
            OWNER_INITIAL_BALANCE + token_count
        );
        assert_eq!(
            credits(&platform, seller.id()),
            seller_credits_after_original,
            "a refused repeat must not pay the seller"
        );
        assert_tokens_conserved(&platform);
    }

    /// A shield taken from the mempool and submitted by another holder ahead of its author.
    ///
    /// Before the author's shield lands its dummy nullifiers are on record nowhere, so the
    /// pool's record cannot refuse the copy; only the owner in the sighash can. Had the copy
    /// landed, it would have paid the author's recipient out of the copier's tokens and left the
    /// author's own shield to be refused as a repeat, charged its fee. The author's shield is
    /// admitted by CheckTx before the copy and executes after it, which is what shows the bundle
    /// was valid all along and the copy was refused for its owner alone.
    #[tokio::test]
    async fn test_a_shield_bundle_front_run_by_another_holder_is_refused_and_the_original_lands() {
        let platform_version = PlatformVersion::latest();
        let mut platform = platform_with_latest_version();
        let mut rng = StdRng::seed_from_u64(9511);
        let amount = 4_200;

        let (owner, signer, key) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (copier, copier_signer, copier_key) =
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

        // The copier holds as many tokens as the copy shields, so a refusal cannot be its
        // balance.
        let transfer = BatchTransition::new_token_transfer_transition(
            token_id,
            owner.id(),
            contract.id(),
            0,
            amount,
            copier.id(),
            None,
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
        .expect("token transfer transition");
        assert_matches!(
            process(&platform, &transfer).execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );

        let shield = build_token_shield_transition(
            token_id,
            owner.id(),
            contract.id(),
            0,
            &wallet_address(),
            amount,
            [0u8; 36],
            None,
            &key,
            3,
            0,
            &signer,
            &Prover,
            platform_version,
        )
        .await
        .expect("client-built token shield");
        assert_check_tx_accepts(&platform, &shield);
        let bundle = proven_bundle(&shield);

        let copy = BatchTransition::new_token_shield_transition(
            token_id,
            copier.id(),
            contract.id(),
            0,
            amount,
            bundle.clone(),
            &copier_key,
            1,
            0,
            &copier_signer,
            platform_version,
            None,
        )
        .await
        .expect("token shield transition");
        assert_front_run_refused(
            &platform,
            &copy,
            copier.id(),
            contract.id(),
            1,
            &bundle,
            true,
            None,
        );
        assert_eq!(pool_balance(&platform, token_id), 0);
        assert_eq!(pool_notes_count(&platform, token_id), 0);
        assert_eq!(
            identity_token_balance(&platform, token_id, copier.id()),
            Some(amount),
            "a refused copy must not take the copier's tokens"
        );

        assert_matches!(
            process(&platform, &shield).execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        assert_eq!(pool_balance(&platform, token_id), amount);
        assert_dummy_nullifiers_recorded(&platform, token_id, &bundle);
        assert_eq!(
            identity_token_balance(&platform, token_id, owner.id()),
            Some(OWNER_INITIAL_BALANCE - 2 * amount)
        );

        // The bundle that just executed, offered again under the copier: CheckTx, which never
        // reads the pool's record, still refuses it on the owner.
        let late_copy = BatchTransition::new_token_shield_transition(
            token_id,
            copier.id(),
            contract.id(),
            0,
            amount,
            bundle,
            &copier_key,
            2,
            0,
            &copier_signer,
            platform_version,
            None,
        )
        .await
        .expect("token shield transition");
        assert_matches!(
            assert_check_tx_rejects(&platform, &late_copy).as_slice(),
            [ConsensusError::StateError(
                StateError::InvalidShieldedProofError(_)
            )]
        );
        assert_tokens_conserved(&platform);
    }

    /// A claim into the pool taken from the mempool and submitted by another identity due the
    /// same release, ahead of its author. CheckTx cannot verify a claim's bundle, so it admits
    /// the copy; block validation refuses it on the owner the bundle names, before the author's
    /// claim has recorded anything the pool could refuse it with.
    #[tokio::test]
    async fn test_a_claim_to_pool_bundle_front_run_by_another_claimant_is_refused_and_the_original_lands(
    ) {
        let platform_version = PlatformVersion::latest();
        let mut platform = platform_with_latest_version();
        let mut rng = StdRng::seed_from_u64(9513);
        let release = 445;

        let (owner, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (claimant, signer, key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (copier, copier_signer, copier_key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (claimant_id, copier_id) = (claimant.id(), copier.id());
        // Both are due the same release, so the copier's claim resolves to exactly the amount
        // the copied bundle carries and only the bundle's owner can be why it is refused.
        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            owner.id(),
            Some(move |configuration: &mut TokenConfiguration| {
                configuration.set_has_shielded_pool(true);
                configuration
                    .distribution_rules_mut()
                    .set_pre_programmed_distribution(Some(TokenPreProgrammedDistribution::V0(
                        TokenPreProgrammedDistributionV0 {
                            distributions: [(
                                100,
                                [(claimant_id, release), (copier_id, release)].into(),
                            )]
                            .into(),
                        },
                    )));
            }),
            None,
            None,
            None,
            platform_version,
        );

        fast_forward_to_block(&platform, 100, 40, 42, 1, false);
        let block_info = BlockInfo {
            time_ms: 200,
            height: 41,
            core_height: 42,
            epoch: Epoch::new(1).unwrap(),
        };
        let supply_before = total_supply(&platform, token_id);

        let claim = build_token_claim_to_pool_transition(
            token_id,
            claimant_id,
            contract.id(),
            0,
            &wallet_address(),
            release,
            TokenDistributionType::PreProgrammed,
            None,
            [0u8; 36],
            None,
            None,
            &key,
            2,
            0,
            &signer,
            &Prover,
            platform_version,
        )
        .await
        .expect("client-built token claim to pool");
        assert_check_tx_accepts(&platform, &claim);
        let bundle = proven_bundle(&claim);

        let copy = BatchTransition::new_token_claim_to_pool_transition(
            token_id,
            copier_id,
            contract.id(),
            0,
            TokenDistributionType::PreProgrammed,
            None,
            bundle.clone(),
            None,
            &copier_key,
            1,
            0,
            &copier_signer,
            platform_version,
            None,
        )
        .await
        .expect("token claim to pool transition");
        assert_front_run_refused(
            &platform,
            &copy,
            copier_id,
            contract.id(),
            1,
            &bundle,
            false,
            Some(&block_info),
        );
        assert_eq!(pool_balance(&platform, token_id), 0);
        assert_eq!(
            total_supply(&platform, token_id),
            supply_before,
            "a refused copy must not release anything"
        );

        assert_matches!(
            process_at(&platform, &claim, &block_info)
                .execution_results()
                .as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        assert_eq!(pool_balance(&platform, token_id), release);
        assert_eq!(total_supply(&platform, token_id), supply_before + release);
        assert_dummy_nullifiers_recorded(&platform, token_id, &bundle);
        assert_tokens_conserved(&platform);
    }

    /// A purchase into the pool taken from the mempool and submitted by another buyer ahead of
    /// its author. Had it landed, the copier would have paid the price for tokens minted to the
    /// author's recipient and the author's own purchase would have been refused as a repeat.
    #[tokio::test]
    async fn test_a_direct_purchase_to_pool_bundle_front_run_by_another_buyer_is_refused_and_the_original_lands(
    ) {
        let platform_version = PlatformVersion::latest();
        let mut platform = platform_with_latest_version();
        let mut rng = StdRng::seed_from_u64(9514);
        let (token_count, price) = (3, dash_to_credits!(0.03));

        let (seller, seller_signer, seller_key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (buyer, signer, key) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(1.0));
        let (copier, copier_signer, copier_key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(1.0));
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
        assert_matches!(
            process(&platform, &set_price)
                .execution_results()
                .as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        let supply_before = total_supply(&platform, token_id);
        let seller_credits_before = credits(&platform, seller.id());

        let purchase = build_token_direct_purchase_to_pool_transition(
            token_id,
            buyer.id(),
            contract.id(),
            0,
            &wallet_address(),
            token_count,
            price,
            [0u8; 36],
            None,
            &key,
            1,
            0,
            &signer,
            &Prover,
            platform_version,
        )
        .await
        .expect("client-built token direct purchase to pool");
        assert_check_tx_accepts(&platform, &purchase);
        let bundle = proven_bundle(&purchase);

        // The copier can afford the price, so a refusal cannot be its credits.
        let copy = BatchTransition::new_token_direct_purchase_to_pool_transition(
            token_id,
            copier.id(),
            contract.id(),
            0,
            token_count,
            price,
            bundle.clone(),
            &copier_key,
            1,
            0,
            &copier_signer,
            platform_version,
            None,
        )
        .await
        .expect("token direct purchase to pool transition");
        let charged = assert_front_run_refused(
            &platform,
            &copy,
            copier.id(),
            contract.id(),
            1,
            &bundle,
            true,
            None,
        );
        assert!(
            charged < price,
            "a refused copy costs its fee, never the price"
        );
        assert_eq!(pool_balance(&platform, token_id), 0);
        assert_eq!(total_supply(&platform, token_id), supply_before);
        assert_eq!(
            credits(&platform, seller.id()),
            seller_credits_before,
            "a refused copy must not pay the seller"
        );

        assert_matches!(
            process(&platform, &purchase).execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        assert_eq!(pool_balance(&platform, token_id), token_count);
        assert_eq!(
            total_supply(&platform, token_id),
            supply_before + token_count
        );
        assert_eq!(
            credits(&platform, seller.id()),
            seller_credits_before + price
        );
        assert_dummy_nullifiers_recorded(&platform, token_id, &bundle);
        assert_tokens_conserved(&platform);
    }

    /// A token that owns a pool, sold directly at `price_per_token` by its contract owner.
    /// Returns the seller's, the contract's and the token's ids.
    async fn token_for_sale(
        platform: &mut TempPlatform<MockCoreRPCLike>,
        seller_seed: u64,
        price_per_token: Credits,
    ) -> (Identifier, Identifier, Identifier) {
        let platform_version = PlatformVersion::latest();
        let (seller, seller_signer, seller_key) =
            setup_identity(platform, seller_seed, dash_to_credits!(0.5));
        let (contract, token_id) = create_token_contract_with_owner_identity(
            platform,
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
        assert_matches!(
            process(platform, &set_price).execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        (seller.id(), contract.id(), token_id)
    }

    /// A client-built purchase of `token_count` tokens for `price`, paid from a fresh credit
    /// pool note (`tag`). Returns the transition and the fee it carries.
    fn build_purchase(
        platform: &TempPlatform<MockCoreRPCLike>,
        token_id: Identifier,
        contract_id: Identifier,
        token_count: u64,
        price: Credits,
        tag: u8,
    ) -> (StateTransition, Credits) {
        let (fvk, ask, _) = spend_keys();
        let address = wallet_address();
        let (credit_note, credit_anchor, credit_path) = fund_credit_pool(platform, tag);
        build_token_purchase_from_shielded_pool_transition(
            token_id,
            contract_id,
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
            PlatformVersion::latest(),
        )
        .expect("build transition")
    }

    fn purchase_v0(purchase: &StateTransition) -> &TokenPurchaseFromShieldedPoolTransitionV0 {
        let StateTransition::TokenPurchaseFromShieldedPool(
            TokenPurchaseFromShieldedPoolTransition::V0(v0),
        ) = purchase
        else {
            panic!("expected a token purchase from the shielded pool");
        };
        v0
    }

    /// The original's token bundle verbatim, paid for by another payer: a fresh fee bundle
    /// spending another credit note (`tag`), bound to the original's token actions and carrying
    /// the same price and fee out of the credit pool. Returns the copy and its fee nullifier.
    fn purchase_copy_with_another_fee_bundle(
        platform: &TempPlatform<MockCoreRPCLike>,
        original: &TokenPurchaseFromShieldedPoolTransitionV0,
        price: Credits,
        fee: Credits,
        tag: u8,
        seed: u64,
    ) -> (StateTransition, [u8; 32]) {
        let platform_version = PlatformVersion::latest();
        let (other_note, other_anchor, other_path) = fund_credit_pool(platform, tag);
        let fee_extra = token_pool_fee_bundle_extra_sighash_data(
            TOKEN_PURCHASE_FROM_SHIELDED_POOL_TYPE,
            &original.token_id.to_buffer(),
            &original.token_actions,
            platform_version,
        )
        .expect("fee bundle sighash data");
        let credits_leaving = price + fee;
        let (fee_bundle, fee_value_balance) = build_spend_bundle(
            other_note,
            other_path,
            other_anchor,
            credits_leaving,
            &fee_extra,
            seed,
        );
        assert_eq!(fee_value_balance, credits_leaving as i64);
        assert_eq!(
            compute_token_purchase_from_shielded_pool_fee(
                original.token_actions.len(),
                fee_bundle.actions.len(),
                platform_version,
            )
            .expect("fee"),
            fee,
            "the copy must pay exactly the fee the original paid"
        );
        let copy_fee_nullifier = fee_bundle.actions[0].nullifier;
        let mut copy = original.clone();
        copy.fee_actions = fee_bundle.actions;
        copy.fee_anchor = fee_bundle.anchor;
        copy.fee_proof = fee_bundle.proof;
        copy.fee_binding_signature = fee_bundle.binding_signature;
        (
            StateTransition::TokenPurchaseFromShieldedPool(
                TokenPurchaseFromShieldedPoolTransition::V0(copy),
            ),
            copy_fee_nullifier,
        )
    }

    /// The identity-less purchase (state transition type 28) mints into the pool through the
    /// same drive operation as a batch mint, and its token bundle is outputs-only too. Its
    /// sighash binds the token, the count and the price but no payer, and its fee bundle binds
    /// only the type, the token and a digest of the token bundle's actions — which anybody can
    /// reproduce with credit notes of their own. So the copy here keeps the original's token
    /// bundle verbatim and pays for it with a fresh fee bundle from another note.
    #[tokio::test]
    async fn test_a_purchase_from_shielded_pool_token_bundle_copied_with_another_fee_bundle_is_refused(
    ) {
        let platform_version = PlatformVersion::latest();
        let mut platform = platform_with_latest_version();
        let price_per_token = dash_to_credits!(0.01);
        let (seller_id, contract_id, token_id) =
            token_for_sale(&mut platform, 9505, price_per_token).await;
        let token_count = 3;
        let price = price_per_token * token_count;

        let (purchase, fee) =
            build_purchase(&platform, token_id, contract_id, token_count, price, 15);
        assert_check_tx_accepts(&platform, &purchase);
        assert_matches!(
            process(&platform, &purchase).execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        assert_eq!(pool_balance(&platform, token_id), token_count);
        assert_eq!(credit_pool_balance(&platform), CREDIT_NOTE - price - fee);

        let original = purchase_v0(&purchase);
        let token_bundle = OrchardBundleParams {
            actions: original.token_actions.clone(),
            anchor: original.token_anchor,
            proof: original.token_proof.clone(),
            binding_signature: original.token_binding_signature,
        };
        assert_dummy_nullifiers_recorded(&platform, token_id, &token_bundle);
        let notes_after_original = pool_notes_count(&platform, token_id);

        let (copy, copy_fee_nullifier) =
            purchase_copy_with_another_fee_bundle(&platform, original, price, fee, 16, 9506);

        let seller_credits_after_original = credits(&platform, seller_id);
        let credit_pool_before_copy = credit_pool_balance(&platform);
        let first = token_bundle.actions[0].nullifier;

        // Unlike a batch, this transition's pools are checked at admission, so the copy never
        // reaches the mempool. A proposer can still put it in a block, where it must fail too.
        assert_matches!(
            assert_check_tx_rejects(&platform, &copy).as_slice(),
            [ConsensusError::StateError(StateError::NullifierAlreadySpentError(error))]
                if error.nullifier() == first
        );
        assert_matches!(
            process(&platform, &copy).execution_results().as_slice(),
            [StateTransitionExecutionResult::UnpaidConsensusError(
                ConsensusError::StateError(StateError::NullifierAlreadySpentError(error))
            )] if error.nullifier() == first
        );

        assert_eq!(pool_balance(&platform, token_id), token_count);
        assert_eq!(pool_notes_count(&platform, token_id), notes_after_original);
        assert_eq!(
            total_supply(&platform, token_id),
            OWNER_INITIAL_BALANCE + token_count
        );
        assert_eq!(
            credit_pool_balance(&platform),
            credit_pool_before_copy,
            "a refused copy must not spend the payer's credit note"
        );
        assert!(
            !platform
                .drive
                .has_nullifier(&copy_fee_nullifier, None, &mut vec![], platform_version)
                .expect("credit pool nullifier lookup"),
            "a refused copy must leave the payer's credit note unspent"
        );
        assert_eq!(
            credits(&platform, seller_id),
            seller_credits_after_original,
            "a refused copy must not pay the seller"
        );
        assert_tokens_conserved(&platform);
    }

    /// The type 28 copy, lifted from the mempool before its original executed, reaches the
    /// same block. CheckTx admits both, since neither token bundle is in the pool yet; the
    /// block must land the original and refuse the copy on the original's write in the block's
    /// own transaction.
    #[tokio::test]
    async fn test_a_purchase_from_shielded_pool_and_its_copy_in_one_block_land_only_once() {
        let platform_version = PlatformVersion::latest();
        let mut platform = platform_with_latest_version();
        let price_per_token = dash_to_credits!(0.01);
        let (seller_id, contract_id, token_id) =
            token_for_sale(&mut platform, 9510, price_per_token).await;
        let token_count = 3;
        let price = price_per_token * token_count;

        let (purchase, fee) =
            build_purchase(&platform, token_id, contract_id, token_count, price, 15);
        let original = purchase_v0(&purchase);
        let (copy, copy_fee_nullifier) =
            purchase_copy_with_another_fee_bundle(&platform, original, price, fee, 16, 9511);
        // Both payers' notes are in the credit pool and its total covers both purchases, so
        // only the token bundle can tell the two apart.
        set_pool_total_balance(&platform, 2 * CREDIT_NOTE);
        assert_check_tx_accepts(&platform, &purchase);
        assert_check_tx_accepts(&platform, &copy);

        let seller_credits_before = credits(&platform, seller_id);
        let first = original.token_actions[0].nullifier;
        assert_matches!(
            process_in_one_block(&platform, &[&purchase, &copy], &BlockInfo::default())
                .execution_results()
                .as_slice(),
            [
                StateTransitionExecutionResult::SuccessfulExecution { .. },
                StateTransitionExecutionResult::UnpaidConsensusError(ConsensusError::StateError(
                    StateError::NullifierAlreadySpentError(error)
                )),
            ] if error.nullifier() == first
        );

        assert_eq!(pool_balance(&platform, token_id), token_count);
        assert_eq!(
            pool_notes_count(&platform, token_id),
            original.token_actions.len() as u64
        );
        assert_eq!(
            total_supply(&platform, token_id),
            OWNER_INITIAL_BALANCE + token_count
        );
        assert_eq!(
            credit_pool_balance(&platform),
            2 * CREDIT_NOTE - price - fee
        );
        assert!(
            !platform
                .drive
                .has_nullifier(&copy_fee_nullifier, None, &mut vec![], platform_version)
                .expect("credit pool nullifier lookup"),
            "a refused copy must leave the payer's credit note unspent"
        );
        assert_eq!(
            credits(&platform, seller_id),
            seller_credits_before + price,
            "the seller is paid once"
        );
        assert_tokens_conserved(&platform);
    }

    /// The same two type 28 transitions in the other order: the copy is sequenced ahead of the
    /// original, which the block proposer can do with anything it lifted from the mempool.
    /// Nothing in the token bundle's sighash names who pays, so the copy lands and pays for the
    /// original's notes. What matters is who bears the refusal that follows: the original is
    /// refused on the dummy nullifier the copy recorded, and that refusal must be unpaid — the
    /// honest payer's credit note stays unspent — or a stranger could make the honest payer pay
    /// for a purchase it never got to make.
    #[tokio::test]
    async fn test_a_purchase_from_shielded_pool_copy_sequenced_first_does_not_charge_the_original_payer(
    ) {
        let platform_version = PlatformVersion::latest();
        let mut platform = platform_with_latest_version();
        let price_per_token = dash_to_credits!(0.01);
        let (seller_id, contract_id, token_id) =
            token_for_sale(&mut platform, 9516, price_per_token).await;
        let token_count = 3;
        let price = price_per_token * token_count;

        let (purchase, fee) =
            build_purchase(&platform, token_id, contract_id, token_count, price, 15);
        let original = purchase_v0(&purchase);
        let original_fee_nullifier = original.fee_actions[0].nullifier;
        let (copy, copy_fee_nullifier) =
            purchase_copy_with_another_fee_bundle(&platform, original, price, fee, 16, 9517);
        // Both payers' notes are in the credit pool and its total covers both purchases, so
        // only the token bundle can tell the two apart.
        set_pool_total_balance(&platform, 2 * CREDIT_NOTE);
        assert_check_tx_accepts(&platform, &purchase);
        assert_check_tx_accepts(&platform, &copy);

        let seller_credits_before = credits(&platform, seller_id);
        let first = original.token_actions[0].nullifier;
        assert_matches!(
            process_in_one_block(&platform, &[&copy, &purchase], &BlockInfo::default())
                .execution_results()
                .as_slice(),
            [
                StateTransitionExecutionResult::SuccessfulExecution { .. },
                StateTransitionExecutionResult::UnpaidConsensusError(ConsensusError::StateError(
                    StateError::NullifierAlreadySpentError(error)
                )),
            ] if error.nullifier() == first
        );

        assert!(
            !platform
                .drive
                .has_nullifier(&original_fee_nullifier, None, &mut vec![], platform_version)
                .expect("credit pool nullifier lookup"),
            "the refused original must leave its payer's credit note unspent"
        );
        assert!(
            platform
                .drive
                .has_nullifier(&copy_fee_nullifier, None, &mut vec![], platform_version)
                .expect("credit pool nullifier lookup"),
            "the copy that landed is paid from the copier's own credit note"
        );
        assert_eq!(
            credit_pool_balance(&platform),
            2 * CREDIT_NOTE - price - fee,
            "one purchase is paid for, once"
        );
        assert_eq!(pool_balance(&platform, token_id), token_count);
        assert_eq!(
            total_supply(&platform, token_id),
            OWNER_INITIAL_BALANCE + token_count
        );
        assert_eq!(
            credits(&platform, seller_id),
            seller_credits_before + price,
            "the seller is paid once"
        );
        assert_tokens_conserved(&platform);
    }
}
