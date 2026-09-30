//! End-to-end coverage for the `immutableAfter` document-type keyword
//! (protocol version 14): on a mutable document type each listed top-level
//! property may be changed by a replace until the document's `$createdAt`
//! plus the property's window, and not after. The block time a transition is
//! processed at is set per transition, so each test places its replaces
//! inside, at the edge of, and past a window.

use super::*;

mod immutable_after_tests {
    use super::*;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TempPlatform;
    use dpp::consensus::state::document::document_property_edit_window_elapsed_error::DocumentPropertyEditWindowElapsedError;
    use dpp::data_contract::accessors::v0::DataContractV0Setters;
    use dpp::data_contract::schema::DataContractSchemaMethodsV0;
    use dpp::document::Document;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::identity::{Identity, IdentityPublicKey};
    use dpp::platform_value::platform_value;
    use dpp::prelude::{DataContract, IdentityNonce};
    use dpp::state_transition::data_contract_update_transition::methods::DataContractUpdateTransitionMethodsV0;
    use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
    use dpp::state_transition::StateTransition;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use simple_signer::signer::SimpleSigner;

    /// The block time every post is created at.
    const CREATED_AT: u64 = 1_700_000_000_000;

    /// `text` may be edited for five minutes, `mood` for one, `author` never
    /// and `score` always.
    fn post_windows() -> Value {
        platform_value!({ "text": 300, "mood": 60, "replyTo": 300 })
    }

    /// A mutable `post`: `author` frozen at creation, `text`, the optional
    /// `mood` and the optional `replyTo` reference to a deletable `draft`
    /// frozen once their `immutableAfter` windows pass, and `score` always
    /// editable.
    fn post_schema(immutable_after: Value) -> Value {
        platform_value!({
            "type": "object",
            "documentsMutable": true,
            "properties": {
                "author": {"type": "string", "position": 0, "maxLength": 63_u32},
                "text": {"type": "string", "position": 1, "maxLength": 500_u32},
                "mood": {"type": "string", "position": 2, "maxLength": 30_u32},
                "score": {"type": "integer", "position": 3, "minimum": 0, "maximum": 1000},
                "replyTo": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "refersTo": { "type": "deletableDocument", "documentType": "draft" },
                    "position": 4
                }
            },
            "required": ["author", "text", "$createdAt"],
            "immutable": ["author"],
            "immutableAfter": immutable_after,
            "additionalProperties": false
        })
    }

    fn draft_schema() -> Value {
        platform_value!({
            "type": "object",
            "documentsMutable": true,
            "canBeDeleted": true,
            "properties": {
                "topic": {"type": "string", "position": 0, "maxLength": 30_u32}
            },
            "required": ["topic"],
            "additionalProperties": false
        })
    }

    /// One identity, one contract with the `draft` and `post` types, and one
    /// post created at [`CREATED_AT`] with `author` and `text` set plus
    /// whatever `fill` adds.
    struct PostFixture {
        platform: TempPlatform<MockCoreRPCLike>,
        identity: Identity,
        signer: SimpleSigner,
        key: IdentityPublicKey,
        contract: DataContract,
        /// The post as last accepted by the chain.
        post: Document,
        /// The identity contract nonce the next transition uses. Every
        /// processed transition consumes one, including the ones that fail
        /// with a paid consensus error.
        next_nonce: IdentityNonce,
        /// Seeds the entropy of every document the fixture creates.
        rng: StdRng,
        /// The draft created a second before the post, when asked for.
        draft: Option<Document>,
    }

    impl PostFixture {
        async fn new(fill: impl FnOnce(&mut Document)) -> Self {
            Self::new_with(false, fill).await
        }

        /// With `with_draft`, a `draft` is created a second before the post.
        async fn new_with(with_draft: bool, fill: impl FnOnce(&mut Document)) -> Self {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();

            let (identity, signer, key) = setup_identity(&mut platform, 959, dash_to_credits!(0.5));

            let mut contract = get_data_contract_fixture(
                Some(identity.id()),
                0,
                platform_version.protocol_version,
            )
            .data_contract_owned();
            contract
                .set_document_schema(
                    "draft",
                    draft_schema(),
                    true,
                    &mut Vec::new(),
                    platform_version,
                )
                .expect("expected to add the draft document type");
            contract
                .set_document_schema(
                    "post",
                    post_schema(post_windows()),
                    true,
                    &mut Vec::new(),
                    platform_version,
                )
                .expect("expected to add the post document type");
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
                .expect("expected to apply the contract");

            let mut fixture = Self {
                platform,
                identity,
                signer,
                key,
                contract,
                post: Document::V0(Default::default()),
                next_nonce: 1,
                rng: StdRng::seed_from_u64(434),
                draft: None,
            };
            if with_draft {
                let (draft, result) = fixture
                    .create(
                        "draft",
                        |draft| draft.set("topic", "dash".into()),
                        CREATED_AT - 1_000,
                    )
                    .await;
                assert_successful(result, "the draft is created");
                fixture.draft = Some(draft);
            }
            let (post, result) = fixture
                .create(
                    "post",
                    |post| {
                        post.set("author", "alice".into());
                        post.set("text", "first draft".into());
                        fill(post);
                    },
                    CREATED_AT,
                )
                .await;
            assert_matches!(
                result,
                StateTransitionExecutionResult::SuccessfulExecution { .. },
                "creating the post must succeed"
            );
            fixture.post = post;
            fixture
        }

        fn process(
            &self,
            transition: &StateTransition,
            time_ms: u64,
        ) -> StateTransitionExecutionResult {
            let platform_version = PlatformVersion::latest();
            let platform_state = self.platform.state.load();
            let serialized = transition
                .serialize_to_bytes()
                .expect("expected the transition to serialize");
            let transaction = self.platform.drive.grove.start_transaction();
            let processing_result = self
                .platform
                .platform
                .process_raw_state_transitions(
                    &[serialized],
                    &platform_state,
                    &BlockInfo::default_with_time(time_ms),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process the state transition");
            self.platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit the transaction");
            processing_result.into_execution_results().remove(0)
        }

        /// Creates a document of `document_type_name` with `fill` applied, at
        /// block time `time_ms`.
        async fn create(
            &mut self,
            document_type_name: &str,
            fill: impl FnOnce(&mut Document),
            time_ms: u64,
        ) -> (Document, StateTransitionExecutionResult) {
            let platform_version = PlatformVersion::latest();
            let document_type = self
                .contract
                .document_type_for_name(document_type_name)
                .expect("expected the document type");
            let entropy = Bytes32::random_with_rng(&mut self.rng);
            let mut document = document_type
                .random_document_with_identifier_and_entropy(
                    &mut self.rng,
                    self.identity.id(),
                    entropy,
                    DocumentFieldFillType::DoNotFillIfNotRequired,
                    DocumentFieldFillSize::AnyDocumentFillSize,
                    platform_version,
                )
                .expect("expected a random document");
            document
                .set_id_for_creation(document_type, &entropy.0, self.next_nonce, platform_version)
                .expect("expected to set the document id");
            fill(&mut document);

            let transition = BatchTransition::new_document_creation_transition_from_document(
                document.clone(),
                document_type,
                entropy.0,
                &self.key,
                self.next_nonce,
                0,
                None,
                &self.signer,
                platform_version,
                None,
            )
            .await
            .expect("expected the create transition");
            self.next_nonce += 1;
            let result = self.process(&transition, time_ms);
            (document, result)
        }

        /// Deletes `document` of `document_type_name` at block time `time_ms`.
        async fn delete(
            &mut self,
            document_type_name: &str,
            document: &Document,
            time_ms: u64,
        ) -> StateTransitionExecutionResult {
            let platform_version = PlatformVersion::latest();
            let transition = BatchTransition::new_document_deletion_transition_from_document(
                document.clone(),
                self.contract
                    .document_type_for_name(document_type_name)
                    .expect("expected the document type"),
                &self.key,
                self.next_nonce,
                0,
                None,
                &self.signer,
                platform_version,
                None,
            )
            .await
            .expect("expected the delete transition");
            self.next_nonce += 1;
            self.process(&transition, time_ms)
        }

        /// Replaces the stored post with `mutate` applied to a copy of it and
        /// the revision bumped, `after_creation_ms` after the post was
        /// created. On success the fixture's post becomes the accepted
        /// version.
        async fn replace_at(
            &mut self,
            after_creation_ms: u64,
            mutate: impl FnOnce(&mut Document),
        ) -> StateTransitionExecutionResult {
            let platform_version = PlatformVersion::latest();
            let mut replacement = self.post.clone();
            mutate(&mut replacement);
            replacement
                .increment_revision()
                .expect("expected the revision to increment");

            let transition = BatchTransition::new_document_replacement_transition_from_document(
                replacement.clone(),
                self.contract
                    .document_type_for_name("post")
                    .expect("expected the post document type"),
                &self.key,
                self.next_nonce,
                0,
                None,
                &self.signer,
                platform_version,
                None,
            )
            .await
            .expect("expected the replace transition");
            self.next_nonce += 1;

            let result = self.process(&transition, CREATED_AT + after_creation_ms);
            if matches!(
                result,
                StateTransitionExecutionResult::SuccessfulExecution { .. }
            ) {
                self.post = replacement;
            }
            result
        }

        /// Re-registers the contract one version up with the `post` type's
        /// `immutableAfter` windows replaced. On success the fixture's
        /// contract becomes the new version.
        async fn update_windows(
            &mut self,
            immutable_after: Value,
        ) -> StateTransitionExecutionResult {
            let platform_version = PlatformVersion::latest();
            let mut updated = self.contract.clone();
            updated.set_version(self.contract.version() + 1);
            updated
                .set_document_schema(
                    "post",
                    post_schema(immutable_after),
                    true,
                    &mut Vec::new(),
                    platform_version,
                )
                .expect("expected to update the post document type");

            let transition = DataContractUpdateTransition::new_from_data_contract(
                updated.clone(),
                &self.identity.clone().into_partial_identity_info(),
                self.key.id(),
                self.next_nonce,
                0,
                &self.signer,
                platform_version,
                None,
            )
            .await
            .expect("expected the contract update transition");
            self.next_nonce += 1;

            let result = self.process(&transition, CREATED_AT);
            if matches!(
                result,
                StateTransitionExecutionResult::SuccessfulExecution { .. }
            ) {
                self.contract = updated;
            }
            result
        }

        /// The one stored post, read back from Drive.
        fn stored_post(&self) -> Document {
            let platform_version = PlatformVersion::latest();
            let query = DriveDocumentQuery::from_sql_expr(
                "select * from post",
                &self.contract,
                Some(&self.platform.config.drive),
                platform_version,
            )
            .expect("expected a document query");
            let mut documents = self
                .platform
                .drive
                .query_documents(query, None, false, None, None)
                .expect("expected a query result")
                .documents()
                .to_vec();
            assert_eq!(documents.len(), 1, "exactly one post is stored");
            documents.remove(0)
        }
    }

    fn assert_successful(result: StateTransitionExecutionResult, because: &str) {
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. },
            "{because}"
        );
    }

    /// The window error for `property` of the fixture's post, judged at
    /// `after_creation_ms` after its creation.
    fn expect_window_elapsed(
        result: StateTransitionExecutionResult,
        property: &str,
        window_seconds: u32,
        after_creation_ms: u64,
        fixture: &PostFixture,
    ) {
        let StateTransitionExecutionResult::PaidConsensusError { error, .. } = result else {
            panic!("expected a paid consensus error, got {result:?}");
        };
        let ConsensusError::StateError(StateError::DocumentPropertyEditWindowElapsedError(error)) =
            error
        else {
            panic!("expected DocumentPropertyEditWindowElapsedError, got {error:?}");
        };
        assert_eq!(
            error,
            DocumentPropertyEditWindowElapsedError::new(
                fixture.post.id(),
                "post".to_string(),
                property.to_string(),
                CREATED_AT,
                window_seconds,
                CREATED_AT + after_creation_ms,
            )
        );
    }

    fn text(value: &str) -> Value {
        Value::Text(value.to_string())
    }

    #[tokio::test]
    async fn should_let_a_replace_change_the_property_within_its_window() {
        let mut fixture = PostFixture::new(|_| {}).await;
        assert_eq!(fixture.stored_post().created_at(), Some(CREATED_AT));

        let result = fixture
            .replace_at(60_000, |post| post.set("text", "second draft".into()))
            .await;
        assert_successful(result, "an edit a minute in is within the window");

        // The window is inclusive: at exactly `$createdAt` plus the window the
        // property may still change
        let result = fixture
            .replace_at(300_000, |post| post.set("text", "last draft".into()))
            .await;
        assert_successful(result, "an edit at the window's last millisecond passes");

        let stored = fixture.stored_post();
        assert_eq!(stored.properties().get("text"), Some(&text("last draft")));
        assert_eq!(stored.revision(), Some(3));
        // A replace never moves the clock the window is measured on
        assert_eq!(stored.created_at(), Some(CREATED_AT));
    }

    #[tokio::test]
    async fn should_refuse_a_replace_changing_the_property_after_its_window() {
        let mut fixture = PostFixture::new(|_| {}).await;

        let result = fixture
            .replace_at(300_001, |post| post.set("text", "too late".into()))
            .await;
        expect_window_elapsed(result, "text", 300, 300_001, &fixture);

        // Nothing of the refused replace reached storage
        let stored = fixture.stored_post();
        assert_eq!(stored.properties().get("text"), Some(&text("first draft")));
        assert_eq!(stored.revision(), Some(1));
    }

    #[tokio::test]
    async fn should_keep_the_other_properties_editable_after_the_windows() {
        let mut fixture = PostFixture::new(|_| {}).await;

        let result = fixture
            .replace_at(86_400_000, |post| post.set("score", 42.into()))
            .await;
        assert_successful(
            result,
            "a property outside `immutableAfter` stays editable a day later",
        );
        // Stored at the width its bounds give it
        assert_eq!(
            fixture.stored_post().properties().get("score"),
            Some(&Value::U16(42))
        );
    }

    /// Each property has its own window: two minutes in, `mood` (one minute)
    /// is frozen while `text` (five minutes) is not.
    #[tokio::test]
    async fn should_judge_each_property_by_its_own_window() {
        let mut fixture = PostFixture::new(|post| post.set("mood", "calm".into())).await;

        let result = fixture
            .replace_at(120_000, |post| post.set("text", "second draft".into()))
            .await;
        assert_successful(result, "text is still within its five minutes");

        let result = fixture
            .replace_at(120_000, |post| {
                post.set("text", "third draft".into());
                post.set("mood", "cross".into());
            })
            .await;
        expect_window_elapsed(result, "mood", 60, 120_000, &fixture);
    }

    /// Past its window an optional property can be neither removed nor set
    /// from absent: "changes" covers both, as it does for `immutable`.
    #[tokio::test]
    async fn should_refuse_adding_or_removing_the_property_after_its_window() {
        let mut fixture = PostFixture::new(|post| post.set("mood", "calm".into())).await;
        let result = fixture
            .replace_at(60_001, |post| {
                post.remove("mood");
            })
            .await;
        expect_window_elapsed(result, "mood", 60, 60_001, &fixture);

        let mut fixture = PostFixture::new(|_| {}).await;
        let result = fixture
            .replace_at(60_001, |post| post.set("mood", "calm".into()))
            .await;
        expect_window_elapsed(result, "mood", 60, 60_001, &fixture);
    }

    /// `immutable` still wins inside the window: `author` is frozen from the
    /// creation on, whatever `immutableAfter` says of other properties.
    #[tokio::test]
    async fn should_keep_immutable_properties_frozen_within_the_windows() {
        let mut fixture = PostFixture::new(|_| {}).await;

        let result = fixture
            .replace_at(1_000, |post| post.set("author", "mallory".into()))
            .await;
        let StateTransitionExecutionResult::PaidConsensusError { error, .. } = result else {
            panic!("expected a paid consensus error, got {result:?}");
        };
        assert_matches!(
            error,
            ConsensusError::StateError(StateError::DocumentImmutablePropertyChangedError(_))
        );
    }

    /// A shortened window applies to the documents already stored: a post
    /// that was still editable is frozen by the update.
    #[tokio::test]
    async fn should_apply_a_shortened_window_to_stored_documents() {
        let mut fixture = PostFixture::new(|_| {}).await;

        let result = fixture
            .update_windows(platform_value!({ "text": 30, "mood": 60, "replyTo": 300 }))
            .await;
        assert_successful(result, "shortening a window is accepted");

        let result = fixture
            .replace_at(60_000, |post| post.set("text", "second draft".into()))
            .await;
        expect_window_elapsed(result, "text", 30, 60_000, &fixture);
    }

    /// A longer window would reopen documents already frozen.
    #[tokio::test]
    async fn should_refuse_an_update_lengthening_a_window() {
        let mut fixture = PostFixture::new(|_| {}).await;

        let result = fixture
            .update_windows(platform_value!({ "text": 600, "mood": 60, "replyTo": 300 }))
            .await;
        let StateTransitionExecutionResult::PaidConsensusError { error, .. } = result else {
            panic!("expected a paid consensus error, got {result:?}");
        };
        assert_matches!(
            error,
            ConsensusError::StateError(StateError::DocumentTypeUpdateError(_))
        );
    }

    /// Past its window a `deletableDocument` reference by id is frozen like an
    /// `immutable` one: it cannot be repointed, but once its draft is deleted
    /// it may be cleared, the one change that lets the post be replaced again
    /// (every replace re-validates the reference).
    #[tokio::test]
    async fn should_let_a_dead_reference_be_cleared_after_its_window() {
        let mut fixture = PostFixture::new_with(true, |_| {}).await;
        let draft = fixture.draft.clone().expect("expected the draft");

        // The reply is set within the window
        let draft_id = Value::Identifier(draft.id().to_buffer());
        let result = fixture
            .replace_at(1_000, |post| post.set("replyTo", draft_id.clone()))
            .await;
        assert_successful(result, "the reply is set within its window");

        // Past the window, while the draft lives, it cannot be cleared
        let result = fixture
            .replace_at(300_001, |post| {
                post.remove("replyTo");
            })
            .await;
        expect_window_elapsed(result, "replyTo", 300, 300_001, &fixture);

        let result = fixture.delete("draft", &draft, CREATED_AT + 300_002).await;
        assert_successful(result, "the draft is deleted");

        let result = fixture
            .replace_at(300_003, |post| {
                post.remove("replyTo");
            })
            .await;
        assert_successful(
            result,
            "clearing a dead reference is allowed after the window",
        );
        assert_eq!(fixture.stored_post().properties().get("replyTo"), None);
    }
}
