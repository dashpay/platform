//! indexOnly document types through the full ABCI pipeline: signed
//! transitions in, `process_raw_state_transitions`, committed state out.
//!
//! Runs against the yappr-likes fixture shared with rs-drive's storage
//! suite. What the pipeline adds on top of that suite: the create-side
//! any-entry-exists probes (`DuplicateUniqueIndexError`), the delete-side
//! owner-bearing probe (existence AND ownership from one read), the
//! factory's selection of the indexOnlyDelete (delete-by-values) KIND for
//! indexOnly types, and the structure gates pairing each delete kind with
//! its storage mode — in both directions.

use super::*;

mod index_only_tests {
    use super::*;
    use crate::platform_types::platform_state::PlatformState;
    use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TempPlatform;
    use dpp::consensus::basic::BasicError;
    use dpp::consensus::state::state_error::StateError;
    use dpp::consensus::ConsensusError;
    use dpp::data_contract::accessors::v0::DataContractV0Setters;
    use dpp::document::Document;
    use dpp::identifier::Identifier;
    use dpp::identity::SecurityLevel;
    use dpp::platform_value;
    use dpp::prelude::DataContract;
    use dpp::state_transition::batch_transition::batched_transition::document_delete_transition::{
        DocumentDeleteTransition, DocumentDeleteTransitionV0,
    };
    use dpp::state_transition::batch_transition::batched_transition::document_index_only_delete_transition::{
        DocumentIndexOnlyDeleteTransition, DocumentIndexOnlyDeleteTransitionV0,
    };
    use dpp::state_transition::batch_transition::batched_transition::DocumentTransition;
    use dpp::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
    use dpp::state_transition::batch_transition::BatchTransitionV0;
    use dpp::state_transition::StateTransition;
    use simple_signer::signer::SimpleSigner;

    /// Shared with rs-drive's indexOnly e2e suite — the same fixture both
    /// layers are written against.
    const YAPPR_LIKES_CONTRACT: &str =
        "../rs-drive/tests/supporting_files/contract/yappr-likes/yappr-likes-contract.json";

    fn register_likes(
        platform: &TempPlatform<MockCoreRPCLike>,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> DataContract {
        let mut contract = json_document_to_contract(YAPPR_LIKES_CONTRACT, true, platform_version)
            .expect("expected to parse the yappr-likes contract");
        contract.set_owner_id(owner_id);
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
            .expect("expected to apply the yappr-likes contract");
        contract
    }

    /// A like on `POST_A` under `#dash` owned by `owner`, with its id
    /// derived from `entropy` exactly as the create transition demands.
    fn build_like(
        contract: &DataContract,
        owner: Identifier,
        post_id: Identifier,
        entropy: Bytes32,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Document {
        let like_type = contract
            .document_type_for_name("like")
            .expect("like doctype exists");
        let mut document = like_type
            .random_document_with_identifier_and_entropy(
                rng,
                owner,
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random like");
        document.set("hashtag", "dash".into());
        document.set(
            "postId",
            platform_value::Value::Identifier(post_id.to_buffer()),
        );
        document
    }

    /// Create a real `post` (the permanent document the likes refer to) and
    /// return it — the like's `postId` carries a `refersTo`, so a like on a
    /// nonexistent post is rejected with ReferencedEntityNotFoundError (as
    /// this suite's first draft usefully proved).
    async fn create_post<S: dpp::identity::signer::Signer<dpp::identity::IdentityPublicKey>>(
        platform: &TempPlatform<MockCoreRPCLike>,
        platform_state: &PlatformState,
        contract: &DataContract,
        owner: Identifier,
        key: &dpp::identity::IdentityPublicKey,
        nonce: u64,
        signer: &S,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> Document {
        let post_type = contract
            .document_type_for_name("post")
            .expect("post doctype exists");
        let entropy = Bytes32::random_with_rng(rng);
        let mut post = post_type
            .random_document_with_identifier_and_entropy(
                rng,
                owner,
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random post");
        post.set("hashtag", "dash".into());
        let create = BatchTransition::new_document_creation_transition_from_document(
            post.clone(),
            post_type,
            entropy.0,
            key,
            nonce,
            0,
            None,
            signer,
            platform_version,
            None,
        )
        .await
        .expect("expected the post create transition");
        let result = process_and_commit(platform, platform_state, &create, platform_version);
        assert_eq!(
            result.valid_count(),
            1,
            "the referenced post must be created: {:?}",
            result.execution_results()
        );
        post
    }

    fn process_and_commit(
        platform: &TempPlatform<MockCoreRPCLike>,
        platform_state: &PlatformState,
        transition: &StateTransition,
        platform_version: &PlatformVersion,
    ) -> crate::platform_types::state_transitions_processing_result::StateTransitionsProcessingResult
    {
        let serialized = transition
            .serialize_to_bytes()
            .expect("expected the batch transition to serialize");
        let transaction = platform.drive.grove.start_transaction();
        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![serialized],
                platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");
        processing_result
    }

    #[tokio::test]
    async fn test_index_only_like_lifecycle() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();
        let platform_state = platform.state.load();
        let mut rng = StdRng::seed_from_u64(4242);

        let (alice, alice_signer, alice_key) =
            setup_identity(&mut platform, 958, dash_to_credits!(1.0));
        let (bob, bob_signer, bob_key) = setup_identity(&mut platform, 450, dash_to_credits!(1.0));

        let contract = register_likes(&platform, alice.id(), platform_version);
        let like_type = contract
            .document_type_for_name("like")
            .expect("like doctype exists");

        let post = create_post(
            &platform,
            &platform_state,
            &contract,
            alice.id(),
            &alice_key,
            2,
            &alice_signer,
            &mut rng,
            platform_version,
        )
        .await;

        // ── Alice likes the post ───────────────────────────────────────
        let entropy = Bytes32::random_with_rng(&mut rng);
        let alice_like = build_like(
            &contract,
            alice.id(),
            post.id(),
            entropy,
            &mut rng,
            platform_version,
        );

        let create = BatchTransition::new_document_creation_transition_from_document(
            alice_like.clone(),
            like_type,
            entropy.0,
            &alice_key,
            3,
            0,
            None,
            &alice_signer,
            platform_version,
            None,
        )
        .await
        .expect("expected the create transition");

        let result = process_and_commit(&platform, &platform_state, &create, platform_version);
        assert_eq!(
            result.valid_count(),
            1,
            "the like must be created: {:?}",
            result.execution_results()
        );

        // ── the same like again (fresh entropy, same values) collides ──
        let entropy_2 = Bytes32::random_with_rng(&mut rng);
        let alice_like_again = build_like(
            &contract,
            alice.id(),
            post.id(),
            entropy_2,
            &mut rng,
            platform_version,
        );
        let create_again = BatchTransition::new_document_creation_transition_from_document(
            alice_like_again,
            like_type,
            entropy_2.0,
            &alice_key,
            4,
            0,
            None,
            &alice_signer,
            platform_version,
            None,
        )
        .await
        .expect("expected the create transition");

        let result =
            process_and_commit(&platform, &platform_state, &create_again, platform_version);
        assert_eq!(result.invalid_paid_count(), 1);
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(StateError::DuplicateUniqueIndexError(_)),
                ..
            }],
            "re-liking the same post must collide on an existing entry"
        );

        // ── Bob may like the same post ─────────────────────────────────
        let bob_entropy = Bytes32::random_with_rng(&mut rng);
        let bob_like = build_like(
            &contract,
            bob.id(),
            post.id(),
            bob_entropy,
            &mut rng,
            platform_version,
        );
        let bob_create = BatchTransition::new_document_creation_transition_from_document(
            bob_like.clone(),
            like_type,
            bob_entropy.0,
            &bob_key,
            2,
            0,
            None,
            &bob_signer,
            platform_version,
            None,
        )
        .await
        .expect("expected the create transition");
        let result = process_and_commit(&platform, &platform_state, &bob_create, platform_version);
        assert_eq!(
            result.valid_count(),
            1,
            "a second owner's like must go through: {:?}",
            result.execution_results()
        );

        // ── Deletes are owner-scoped ───────────────────────────────────
        // Same values, signed by Bob: the owner-bearing probe runs with
        // owner = Bob, so it removes Bob's entry and leaves Alice's.
        let mut alices_values_for_bob = alice_like.clone();
        alices_values_for_bob.set_owner_id(bob.id());
        let bob_deletes_alices = BatchTransition::new_document_deletion_transition_from_document(
            alices_values_for_bob,
            like_type,
            &bob_key,
            3,
            0,
            None,
            &bob_signer,
            platform_version,
            None,
        )
        .await
        .expect("expected the delete transition");
        // Bob has his own like with the same values, so this actually
        // deletes BOB's like — proving deletes are owner-scoped. Delete it
        // and then show Alice's is still there by deleting it as Alice.
        let result = process_and_commit(
            &platform,
            &platform_state,
            &bob_deletes_alices,
            platform_version,
        );
        assert_eq!(
            result.valid_count(),
            1,
            "a delete with identical values signed by Bob removes BOB's entries only"
        );

        // Bob deleting again: nothing left under his owner key.
        let mut alices_values_for_bob = alice_like.clone();
        alices_values_for_bob.set_owner_id(bob.id());
        let bob_deletes_again = BatchTransition::new_document_deletion_transition_from_document(
            alices_values_for_bob,
            like_type,
            &bob_key,
            4,
            0,
            None,
            &bob_signer,
            platform_version,
            None,
        )
        .await
        .expect("expected the delete transition");
        let result = process_and_commit(
            &platform,
            &platform_state,
            &bob_deletes_again,
            platform_version,
        );
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(StateError::DocumentNotFoundError(_)),
                ..
            }],
            "an owner with no entry gets DocumentNotFound"
        );

        // ── Alice unlikes ──────────────────────────────────────────────
        let alice_delete = BatchTransition::new_document_deletion_transition_from_document(
            alice_like.clone(),
            like_type,
            &alice_key,
            5,
            0,
            None,
            &alice_signer,
            platform_version,
            None,
        )
        .await
        .expect("expected the delete transition");
        let result =
            process_and_commit(&platform, &platform_state, &alice_delete, platform_version);
        assert_eq!(
            result.valid_count(),
            1,
            "Alice must be able to unlike: {:?}",
            result.execution_results()
        );

        // And re-liking after the unlike works again.
        let entropy_3 = Bytes32::random_with_rng(&mut rng);
        let alice_relike = build_like(
            &contract,
            alice.id(),
            post.id(),
            entropy_3,
            &mut rng,
            platform_version,
        );
        let re_create = BatchTransition::new_document_creation_transition_from_document(
            alice_relike,
            like_type,
            entropy_3.0,
            &alice_key,
            6,
            0,
            None,
            &alice_signer,
            platform_version,
            None,
        )
        .await
        .expect("expected the create transition");
        let result = process_and_commit(&platform, &platform_state, &re_create, platform_version);
        assert_eq!(result.valid_count(), 1);

        let issues = platform
            .drive
            .grove
            .verify_grovedb(None, true, false, &platform_version.drive.grove_version)
            .expect("verify_grovedb must run");
        assert!(
            issues.is_empty(),
            "grovedb must stay consistent: {issues:?}"
        );
    }

    /// The structure gate pairs delete KINDS with storage modes: a plain
    /// (by-id) delete on an indexOnly type is refused. (The factory
    /// auto-selects the indexOnlyDelete kind — this test assembles the
    /// wrong kind by hand.)
    #[tokio::test]
    async fn test_by_id_delete_on_index_only_type_is_refused() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();
        let platform_state = platform.state.load();
        let mut rng = StdRng::seed_from_u64(777);

        let (alice, alice_signer, alice_key) =
            setup_identity(&mut platform, 958, dash_to_credits!(1.0));
        let contract = register_likes(&platform, alice.id(), platform_version);
        let like_type = contract
            .document_type_for_name("like")
            .expect("like doctype exists");

        let post = create_post(
            &platform,
            &platform_state,
            &contract,
            alice.id(),
            &alice_key,
            2,
            &alice_signer,
            &mut rng,
            platform_version,
        )
        .await;

        let entropy = Bytes32::random_with_rng(&mut rng);
        let alice_like = build_like(
            &contract,
            alice.id(),
            post.id(),
            entropy,
            &mut rng,
            platform_version,
        );
        let create = BatchTransition::new_document_creation_transition_from_document(
            alice_like.clone(),
            like_type,
            entropy.0,
            &alice_key,
            3,
            0,
            None,
            &alice_signer,
            platform_version,
            None,
        )
        .await
        .expect("expected the create transition");
        let result = process_and_commit(&platform, &platform_state, &create, platform_version);
        assert_eq!(result.valid_count(), 1);

        // The factory would auto-select the indexOnlyDelete kind for this
        // doctype, so the wrong kind has to be assembled by hand: a plain
        // by-id delete naming the indexOnly `like` type.
        let by_id_delete: DocumentTransition =
            DocumentDeleteTransition::V0(DocumentDeleteTransitionV0 {
                base: DocumentBaseTransition::from_document(
                    &alice_like,
                    like_type,
                    None,
                    4,
                    platform_version,
                    None,
                )
                .expect("expected a base transition"),
            })
            .into();
        let by_id_delete = sign_batch(
            BatchTransitionV0 {
                owner_id: alice.id(),
                transitions: vec![by_id_delete.into()],
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: Default::default(),
            },
            &alice_key,
            &alice_signer,
        )
        .await;

        let result =
            process_and_commit(&platform, &platform_state, &by_id_delete, platform_version);
        assert_eq!(result.valid_count(), 0);
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::BasicError(
                    BasicError::InvalidDocumentTransitionActionError(_)
                ),
                ..
            }],
            "a by-id delete on an indexOnly type must be refused by the structure gate"
        );
    }

    /// The mirror direction: an indexOnlyDelete (delete-by-values) on a
    /// STORED document type is refused — a stored document is deleted by
    /// id, and carried values are nothing its pipeline validates against.
    #[tokio::test]
    async fn test_index_only_delete_on_stored_type_is_refused() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();
        let platform_state = platform.state.load();
        let mut rng = StdRng::seed_from_u64(778);

        let (alice, alice_signer, alice_key) =
            setup_identity(&mut platform, 958, dash_to_credits!(1.0));
        let contract = register_likes(&platform, alice.id(), platform_version);
        let post_type = contract
            .document_type_for_name("post")
            .expect("post doctype exists");

        // A real stored post to aim the wrong-kind delete at.
        let post = create_post(
            &platform,
            &platform_state,
            &contract,
            alice.id(),
            &alice_key,
            2,
            &alice_signer,
            &mut rng,
            platform_version,
        )
        .await;

        let index_only_delete: DocumentTransition =
            DocumentIndexOnlyDeleteTransition::V0(DocumentIndexOnlyDeleteTransitionV0 {
                base: DocumentBaseTransition::from_document(
                    &post,
                    post_type,
                    None,
                    3,
                    platform_version,
                    None,
                )
                .expect("expected a base transition"),
                data: post.properties().clone(),
            })
            .into();
        let index_only_delete = sign_batch(
            BatchTransitionV0 {
                owner_id: alice.id(),
                transitions: vec![index_only_delete.into()],
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: Default::default(),
            },
            &alice_key,
            &alice_signer,
        )
        .await;

        let result = process_and_commit(
            &platform,
            &platform_state,
            &index_only_delete,
            platform_version,
        );
        assert_eq!(result.valid_count(), 0);
        // Pin the storage-mode pairing branch by its message: the fixture's
        // `post` doctype also has `canBeDeleted: false`, whose earlier check
        // returns the same error TYPE — matching on the type alone would
        // keep this test green even with the pairing gate deleted.
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::BasicError(
                    BasicError::InvalidDocumentTransitionActionError(error)
                ),
                ..
            }] if error.action().contains("indexOnlyDelete is only for indexOnly types"),
            "an indexOnlyDelete on a stored type must be refused by the storage-mode pairing gate"
        );
    }

    /// Signs a manually assembled batch. The factory methods cannot build a
    /// delete whose value payload is deliberately malformed.
    async fn sign_batch(
        batch: BatchTransitionV0,
        key: &dpp::identity::IdentityPublicKey,
        signer: &SimpleSigner,
    ) -> StateTransition {
        let mut state_transition: StateTransition = BatchTransition::from(batch).into();
        state_transition
            .sign_external(key, signer, Some(|_, _| Ok(SecurityLevel::HIGH)))
            .await
            .expect("expected to sign the batch");
        state_transition
    }

    /// An indexOnlyDelete's value payload is untrusted and selects the storage
    /// entries every probe and removal touches — malformed values must be
    /// refused by structure validation as consensus errors, never surface
    /// later as internal errors from index-key derivation.
    #[tokio::test]
    async fn test_malformed_delete_values_are_refused_at_structure() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();
        let platform_state = platform.state.load();
        let mut rng = StdRng::seed_from_u64(31339);

        let (alice, alice_signer, alice_key) =
            setup_identity(&mut platform, 958, dash_to_credits!(1.0));
        let contract = register_likes(&platform, alice.id(), platform_version);
        let like_type = contract
            .document_type_for_name("like")
            .expect("like doctype exists");

        let post = create_post(
            &platform,
            &platform_state,
            &contract,
            alice.id(),
            &alice_key,
            2,
            &alice_signer,
            &mut rng,
            platform_version,
        )
        .await;

        let entropy = Bytes32::random_with_rng(&mut rng);
        let like = build_like(
            &contract,
            alice.id(),
            post.id(),
            entropy,
            &mut rng,
            platform_version,
        );

        // Missing required property, and a $createdAt the type never uses:
        // each must die in structure validation with a consensus error.
        let mut missing_property = like.properties().clone();
        missing_property.remove("postId");
        let mut spurious_created_at = like.properties().clone();
        spurious_created_at.insert("$createdAt".to_string(), platform_value::Value::U64(1));

        for (nonce, data) in [(3u64, missing_property), (4u64, spurious_created_at)] {
            let delete_transition: DocumentIndexOnlyDeleteTransition =
                DocumentIndexOnlyDeleteTransitionV0 {
                    base: DocumentBaseTransition::from_document(
                        &like,
                        like_type,
                        None,
                        nonce,
                        platform_version,
                        None,
                    )
                    .expect("expected a base transition"),
                    data,
                }
                .into();
            let batch = sign_batch(
                BatchTransitionV0 {
                    owner_id: alice.id(),
                    transitions: vec![delete_transition.into()],
                    user_fee_increase: 0,
                    signature_public_key_id: 0,
                    signature: Default::default(),
                },
                &alice_key,
                &alice_signer,
            )
            .await;

            let result = process_and_commit(&platform, &platform_state, &batch, platform_version);
            assert_matches!(
                result.execution_results().as_slice(),
                [StateTransitionExecutionResult::PaidConsensusError {
                    error: ConsensusError::BasicError(_),
                    ..
                }],
                "malformed delete values must be a consensus error: {:?}",
                result.execution_results()
            );
        }
    }
}
