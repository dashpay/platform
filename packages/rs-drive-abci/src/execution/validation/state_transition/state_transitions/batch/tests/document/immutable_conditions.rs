//! End-to-end coverage for the conditional entries of the `immutable`
//! document-type keyword (protocol version 14): `{ "property", "when" }`
//! freezes a top-level property of a mutable document type for any replace
//! its condition holds for, judged on the document the replace writes, with
//! the stored one read through `$old.`. The block time a transition is
//! processed at is set per transition, so a condition reading the times can be
//! placed inside, at the edge of, and past a window.

use super::*;

mod immutable_conditions_tests {
    use super::*;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TempPlatform;
    use dpp::data_contract::schema::DataContractSchemaMethodsV0;
    use dpp::document::Document;
    use dpp::identity::{Identity, IdentityPublicKey};
    use dpp::platform_value::platform_value;
    use dpp::prelude::{DataContract, IdentityNonce};
    use dpp::state_transition::StateTransition;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use simple_signer::signer::SimpleSigner;

    /// The block time every post is created at.
    const CREATED_AT: u64 = 1_700_000_000_000;

    /// Frozen five minutes after the document was created: on the document a
    /// replace writes, `$updatedAt` is the replace's block time.
    fn five_minutes_after_creation() -> Value {
        platform_value!({
            "greaterThan": [{ "subtract": ["$updatedAt", "$createdAt"] }, 300000]
        })
    }

    /// A mutable `post`: `author` and `text` required, `mood` an optional
    /// string, `status` a draft or published, `score` an optional integer,
    /// and the given `immutable` list. The type records `$createdAt` and
    /// `$updatedAt`, which a condition reads.
    fn post_schema(immutable: Value) -> Value {
        platform_value!({
            "type": "object",
            "documentsMutable": true,
            "properties": {
                "author": {"type": "string", "position": 0, "maxLength": 63_u32},
                "text": {"type": "string", "position": 1, "maxLength": 500_u32},
                "mood": {"type": "string", "position": 2, "maxLength": 30_u32},
                "status": {"type": "string", "position": 3, "enum": ["draft", "published"]},
                "score": {"type": "integer", "position": 4, "minimum": 0, "maximum": 1000}
            },
            "required": ["author", "text", "$createdAt", "$updatedAt"],
            "immutable": immutable,
            "additionalProperties": false
        })
    }

    /// One identity, one contract whose `post` type lists `immutable` as
    /// given, and one post created at [`CREATED_AT`] with `author` and `text`
    /// set plus whatever `fill` adds.
    struct PostFixture {
        platform: TempPlatform<MockCoreRPCLike>,
        signer: SimpleSigner,
        key: IdentityPublicKey,
        contract: DataContract,
        /// The post as last accepted by the chain.
        post: Document,
        /// The identity contract nonce the next transition uses. Every
        /// processed transition consumes one, including the ones that fail
        /// with a paid consensus error.
        next_nonce: IdentityNonce,
    }

    impl PostFixture {
        async fn new(immutable: Value, fill: impl FnOnce(&mut Document)) -> Self {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();

            let (identity, signer, key): (Identity, SimpleSigner, IdentityPublicKey) =
                setup_identity(&mut platform, 959, dash_to_credits!(0.5));

            let mut contract = get_data_contract_fixture(
                Some(identity.id()),
                0,
                platform_version.protocol_version,
            )
            .data_contract_owned();
            contract
                .set_document_schema(
                    "post",
                    post_schema(immutable),
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

            let post_type = contract
                .document_type_for_name("post")
                .expect("expected the post document type");
            let mut rng = StdRng::seed_from_u64(434);
            let entropy = Bytes32::random_with_rng(&mut rng);
            let mut post = post_type
                .random_document_with_identifier_and_entropy(
                    &mut rng,
                    identity.id(),
                    entropy,
                    DocumentFieldFillType::DoNotFillIfNotRequired,
                    DocumentFieldFillSize::AnyDocumentFillSize,
                    platform_version,
                )
                .expect("expected a random post");
            post.set_id_for_creation(post_type, &entropy.0, 1, platform_version)
                .expect("expected to set the document id");
            post.set("author", "alice".into());
            post.set("text", "first draft".into());
            fill(&mut post);

            let transition = BatchTransition::new_document_creation_transition_from_document(
                post.clone(),
                post_type,
                entropy.0,
                &key,
                1,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .await
            .expect("expected the create transition");

            let fixture = Self {
                platform,
                signer,
                key,
                contract,
                post,
                next_nonce: 2,
            };
            assert_matches!(
                fixture.process(&transition, CREATED_AT),
                StateTransitionExecutionResult::SuccessfulExecution { .. },
                "creating the post must succeed"
            );
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

    /// A frozen property is refused with the error an `immutable` property
    /// without a condition gets, naming it.
    fn expect_frozen(
        result: StateTransitionExecutionResult,
        property: &str,
        fixture: &PostFixture,
    ) {
        let StateTransitionExecutionResult::PaidConsensusError { error, .. } = result else {
            panic!("expected a paid consensus error, got {result:?}");
        };
        let ConsensusError::StateError(StateError::DocumentImmutablePropertyChangedError(error)) =
            error
        else {
            panic!("expected DocumentImmutablePropertyChangedError, got {error:?}");
        };
        assert_eq!(error.property(), property);
        assert_eq!(error.document_type_name(), "post");
        assert_eq!(error.document_id(), fixture.post.id());
    }

    fn text(value: &str) -> Value {
        Value::Text(value.to_string())
    }

    #[tokio::test]
    async fn should_let_a_replace_change_the_property_until_five_minutes_after_creation() {
        let mut fixture = PostFixture::new(
            platform_value!(["author", { "property": "text", "when": five_minutes_after_creation() }]),
            |_| {},
        )
        .await;
        assert_eq!(fixture.stored_post().created_at(), Some(CREATED_AT));

        let result = fixture
            .replace_at(60_000, |post| post.set("text", "second draft".into()))
            .await;
        assert_successful(result, "an edit a minute in is within the window");

        // `greaterThan`: at exactly five minutes the condition does not hold yet
        let result = fixture
            .replace_at(300_000, |post| post.set("text", "last draft".into()))
            .await;
        assert_successful(result, "an edit at the window's last millisecond passes");

        let result = fixture
            .replace_at(300_001, |post| post.set("text", "too late".into()))
            .await;
        expect_frozen(result, "text", &fixture);

        // Nothing of the refused replace reached storage, and a property the
        // list does not hold stays editable
        let result = fixture
            .replace_at(86_400_000, |post| post.set("score", 42.into()))
            .await;
        assert_successful(result, "a property immutable does not list stays editable");
        let stored = fixture.stored_post();
        assert_eq!(stored.properties().get("text"), Some(&text("last draft")));
        assert_eq!(stored.revision(), Some(4));
        assert_eq!(stored.created_at(), Some(CREATED_AT));
    }

    /// Judged on the stored document: the replace that publishes may still
    /// edit the text in the same step, the ones after it may not.
    #[tokio::test]
    async fn should_freeze_a_property_once_the_stored_document_is_published() {
        let published = platform_value!({ "equal": ["$old.status", { "const": "published" }] });
        let mut fixture = PostFixture::new(
            platform_value!([{ "property": "text", "when": published }]),
            |post| post.set("status", "draft".into()),
        )
        .await;

        let result = fixture
            .replace_at(1_000, |post| {
                post.set("status", "published".into());
                post.set("text", "final text".into());
            })
            .await;
        assert_successful(result, "the stored post is a draft, so the text is free");

        let result = fixture
            .replace_at(2_000, |post| post.set("text", "an edit".into()))
            .await;
        expect_frozen(result, "text", &fixture);
    }

    /// Judged on the document written: a replace that publishes may not
    /// change the text; one that leaves the post a draft may.
    #[tokio::test]
    async fn should_freeze_a_property_in_a_replace_writing_a_published_document() {
        let published = platform_value!({ "equal": ["status", { "const": "published" }] });
        let mut fixture = PostFixture::new(
            platform_value!([{ "property": "text", "when": published }]),
            |post| post.set("status", "draft".into()),
        )
        .await;

        let result = fixture
            .replace_at(1_000, |post| {
                post.set("status", "published".into());
                post.set("text", "final text".into());
            })
            .await;
        expect_frozen(result, "text", &fixture);

        let result = fixture
            .replace_at(2_000, |post| post.set("text", "a draft edit".into()))
            .await;
        assert_successful(result, "a draft's text is free");
    }

    /// Frozen while the condition holds: when it stops holding, the property
    /// is free again. A contract that wants a lasting freeze also freezes what
    /// its condition reads.
    #[tokio::test]
    async fn should_free_the_property_again_when_the_condition_stops_holding() {
        let published = platform_value!({ "equal": ["$old.status", { "const": "published" }] });
        let mut fixture = PostFixture::new(
            platform_value!([{ "property": "text", "when": published }]),
            |post| post.set("status", "published".into()),
        )
        .await;

        let result = fixture
            .replace_at(1_000, |post| post.set("text", "an edit".into()))
            .await;
        expect_frozen(result, "text", &fixture);

        // `status` itself is free, so the post can go back to a draft...
        let result = fixture
            .replace_at(2_000, |post| post.set("status", "draft".into()))
            .await;
        assert_successful(result, "status is not listed");

        // ...and a draft's text is free again
        let result = fixture
            .replace_at(3_000, |post| post.set("text", "an edit".into()))
            .await;
        assert_successful(result, "the stored post is a draft again");
        assert_eq!(
            fixture.stored_post().properties().get("text"),
            Some(&text("an edit"))
        );
    }

    /// A condition that faults (a division by zero) counts as holding: a
    /// fault never frees a property.
    #[tokio::test]
    async fn should_keep_the_property_frozen_when_its_condition_faults() {
        // 10 / score < 5: holds for a score above 2, not for 1 or 2, and
        // faults for a score of 0 (left out, a missing integer reads as 0)
        let condition = platform_value!({ "lessThan": [{ "divide": [10, "score"] }, 5] });
        let mut fixture = PostFixture::new(
            platform_value!([{ "property": "mood", "when": condition }]),
            |_| {},
        )
        .await;

        let result = fixture
            .replace_at(1_000, |post| post.set("mood", "calm".into()))
            .await;
        expect_frozen(result, "mood", &fixture);

        let result = fixture
            .replace_at(2_000, |post| {
                post.set("score", 1.into());
                post.set("mood", "calm".into());
            })
            .await;
        assert_successful(result, "10 / 1 is not below 5, so mood is free");

        let result = fixture
            .replace_at(3_000, |post| {
                post.set("score", 10.into());
                post.set("mood", "cross".into());
            })
            .await;
        expect_frozen(result, "mood", &fixture);
    }
}
