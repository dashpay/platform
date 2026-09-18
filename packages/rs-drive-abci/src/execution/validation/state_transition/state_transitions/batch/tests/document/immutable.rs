//! End-to-end coverage for the `immutable` document-type keyword (protocol
//! version 14): on a mutable document type the listed top-level properties
//! are frozen at creation. A replace that leaves them alone succeeds; one
//! that changes, adds or removes any of them is consensus-rejected and leaves
//! the stored document untouched; and a contract update may add to the list
//! but never remove from it.

use super::*;

mod immutable_tests {
    use super::*;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TempPlatform;
    use dpp::data_contract::accessors::v0::DataContractV0Setters;
    use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
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
    use drive::util::storage_flags::StorageFlags;
    use simple_signer::signer::SimpleSigner;
    use std::collections::BTreeSet;

    /// A mutable `post` type: `author` and `body` required, `mood` an
    /// optional string, `meta` an optional nested object, and the given
    /// `immutable` and `immutableAllowSetting` lists.
    fn post_schema(immutable: Value, allow_setting: Value) -> Value {
        platform_value!({
            "type": "object",
            "documentsMutable": true,
            "properties": {
                "author": {"type": "string", "position": 0, "maxLength": 63_u32},
                "body": {"type": "string", "position": 1, "maxLength": 500_u32},
                "mood": {"type": "string", "position": 2, "maxLength": 30_u32},
                "meta": {
                    "type": "object",
                    "position": 3,
                    "properties": {
                        "tag": {"type": "string", "position": 0, "maxLength": 30_u32}
                    },
                    "additionalProperties": false
                }
            },
            "required": ["author", "body"],
            "immutable": immutable,
            "immutableAllowSetting": allow_setting,
            "additionalProperties": false
        })
    }

    /// The scenario every test starts from: one identity, one contract whose
    /// `post` type lists `immutable` as given, and one stored post with
    /// `author` and `body` set plus whatever `fill` adds.
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
    }

    impl PostFixture {
        async fn new(immutable: Value, fill: impl FnOnce(&mut Document)) -> Self {
            Self::new_with_lists(immutable, platform_value!([]), fill).await
        }

        async fn new_with_lists(
            immutable: Value,
            allow_setting: Value,
            fill: impl FnOnce(&mut Document),
        ) -> Self {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.5));

            let mut contract = get_data_contract_fixture(
                Some(identity.id()),
                0,
                platform_version.protocol_version,
            )
            .data_contract_owned();
            contract
                .set_document_schema(
                    "post",
                    post_schema(immutable, allow_setting),
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

            let mut rng = StdRng::seed_from_u64(433);
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
            post.set("author", "alice".into());
            post.set("body", "first draft".into());
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
                identity,
                signer,
                key,
                contract,
                post,
                next_nonce: 2,
            };
            assert_matches!(
                fixture.process(&transition),
                StateTransitionExecutionResult::SuccessfulExecution { .. },
                "creating the post must succeed"
            );
            fixture
        }

        fn process(&self, transition: &StateTransition) -> StateTransitionExecutionResult {
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
                    &BlockInfo::default(),
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
        /// the revision bumped. On success the fixture's post becomes the
        /// accepted version.
        async fn replace(
            &mut self,
            mutate: impl FnOnce(&mut Document),
        ) -> StateTransitionExecutionResult {
            let platform_version = PlatformVersion::latest();
            let mut replacement = self.post.clone();
            mutate(&mut replacement);
            replacement
                .increment_revision()
                .expect("expected the revision to increment");

            let transition = {
                let post_type = self
                    .contract
                    .document_type_for_name("post")
                    .expect("expected the post document type");
                BatchTransition::new_document_replacement_transition_from_document(
                    replacement.clone(),
                    post_type,
                    &self.key,
                    self.next_nonce,
                    0,
                    None,
                    &self.signer,
                    platform_version,
                    None,
                )
                .await
                .expect("expected the replace transition")
            };
            self.next_nonce += 1;

            let result = self.process(&transition);
            if matches!(
                result,
                StateTransitionExecutionResult::SuccessfulExecution { .. }
            ) {
                self.post = replacement;
            }
            result
        }

        /// Re-registers the contract one version up with the `post` type's
        /// `immutable` list replaced. On success the fixture's contract
        /// becomes the new version.
        async fn update_immutable_list(
            &mut self,
            immutable: Value,
        ) -> StateTransitionExecutionResult {
            self.update_lists(immutable, platform_value!([])).await
        }

        /// Like `update_immutable_list`, replacing the `immutableAllowSetting`
        /// list as well.
        async fn update_lists(
            &mut self,
            immutable: Value,
            allow_setting: Value,
        ) -> StateTransitionExecutionResult {
            let platform_version = PlatformVersion::latest();
            let mut updated = self.contract.clone();
            updated.set_version(self.contract.version() + 1);
            updated
                .set_document_schema(
                    "post",
                    post_schema(immutable, allow_setting),
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

            let result = self.process(&transition);
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

    fn expect_immutable_property_error(
        result: StateTransitionExecutionResult,
        expected_property: &str,
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
        assert_eq!(error.property(), expected_property);
        assert_eq!(error.document_type_name(), "post");
        assert_eq!(error.document_id(), fixture.post.id());
    }

    fn text(value: &str) -> Value {
        Value::Text(value.to_string())
    }

    #[tokio::test]
    async fn should_replace_mutable_properties_while_the_immutable_one_stays() {
        let mut fixture = PostFixture::new(platform_value!(["author"]), |_| {}).await;

        let result = fixture
            .replace(|post| post.set("body", "second draft".into()))
            .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. },
            "changing only mutable properties must succeed"
        );

        let stored = fixture.stored_post();
        assert_eq!(stored.properties().get("body"), Some(&text("second draft")));
        assert_eq!(stored.properties().get("author"), Some(&text("alice")));
        assert_eq!(stored.revision(), Some(2));
    }

    #[tokio::test]
    async fn should_reject_a_replace_that_changes_an_immutable_property() {
        let mut fixture = PostFixture::new(platform_value!(["author"]), |_| {}).await;

        let result = fixture
            .replace(|post| {
                post.set("author", "mallory".into());
                post.set("body", "hijacked".into());
            })
            .await;
        expect_immutable_property_error(result, "author", &fixture);

        // Nothing of the rejected replace reached storage.
        let stored = fixture.stored_post();
        assert_eq!(stored.properties().get("author"), Some(&text("alice")));
        assert_eq!(stored.properties().get("body"), Some(&text("first draft")));
        assert_eq!(stored.revision(), Some(1));
    }

    #[tokio::test]
    async fn should_reject_a_replace_that_removes_an_optional_immutable_property() {
        let mut fixture = PostFixture::new(platform_value!(["mood"]), |post| {
            post.set("mood", "cheerful".into())
        })
        .await;

        let result = fixture
            .replace(|post| {
                post.remove("mood");
            })
            .await;
        expect_immutable_property_error(result, "mood", &fixture);
    }

    #[tokio::test]
    async fn should_reject_a_replace_that_adds_an_absent_immutable_property() {
        let mut fixture = PostFixture::new(platform_value!(["mood"]), |_| {}).await;

        let result = fixture
            .replace(|post| post.set("mood", "cheerful".into()))
            .await;
        expect_immutable_property_error(result, "mood", &fixture);
    }

    #[tokio::test]
    async fn should_freeze_an_immutable_object_property_whole() {
        let mut fixture = PostFixture::new(platform_value!(["meta"]), |post| {
            post.set("meta", platform_value!({"tag": "intro"}))
        })
        .await;

        // Leaving the object alone keeps the replace valid, even though the
        // object round-trips through serialization on both sides of the
        // comparison.
        let result = fixture
            .replace(|post| post.set("body", "second draft".into()))
            .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. },
            "an unchanged immutable object must not block the replace"
        );

        // Changing a nested value changes the top-level object.
        let result = fixture
            .replace(|post| post.set("meta", platform_value!({"tag": "outro"})))
            .await;
        expect_immutable_property_error(result, "meta", &fixture);
    }

    #[tokio::test]
    async fn should_name_the_first_changed_immutable_property_in_name_order() {
        let mut fixture = PostFixture::new(platform_value!(["author", "mood"]), |post| {
            post.set("mood", "cheerful".into())
        })
        .await;

        let result = fixture
            .replace(|post| {
                post.set("mood", "gloomy".into());
                post.set("author", "mallory".into());
            })
            .await;
        expect_immutable_property_error(result, "author", &fixture);
    }

    #[tokio::test]
    async fn should_reject_a_contract_update_that_removes_an_immutable_property() {
        let mut fixture = PostFixture::new(platform_value!(["author", "mood"]), |_| {}).await;

        let result = fixture
            .update_immutable_list(platform_value!(["author"]))
            .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(StateError::DocumentTypeUpdateError(_)),
                ..
            },
            "shrinking the immutable list must be refused"
        );

        // The list is unchanged, so `mood` is still frozen (here: absent).
        let result = fixture
            .replace(|post| post.set("mood", "cheerful".into()))
            .await;
        expect_immutable_property_error(result, "mood", &fixture);
    }

    #[tokio::test]
    async fn should_enforce_an_immutable_property_added_by_a_contract_update() {
        let mut fixture = PostFixture::new(platform_value!(["author"]), |post| {
            post.set("mood", "cheerful".into())
        })
        .await;

        // Before the update `mood` is editable.
        let result = fixture
            .replace(|post| post.set("mood", "curious".into()))
            .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. },
            "mood is editable before the update"
        );

        let result = fixture
            .update_immutable_list(platform_value!(["author", "mood"]))
            .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. },
            "growing the immutable list must be accepted"
        );
        let expected: BTreeSet<String> = ["author", "mood"].into_iter().map(String::from).collect();
        assert_eq!(
            fixture
                .contract
                .document_type_for_name("post")
                .expect("expected the post document type")
                .immutable_fields(),
            &expected
        );

        // After it, `mood` is frozen at its current value while `body` stays
        // editable.
        let result = fixture
            .replace(|post| post.set("mood", "gloomy".into()))
            .await;
        expect_immutable_property_error(result, "mood", &fixture);

        let result = fixture
            .replace(|post| post.set("body", "third draft".into()))
            .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. },
            "body stays editable after the update"
        );
        assert_eq!(
            fixture.stored_post().properties().get("mood"),
            Some(&text("curious"))
        );
    }

    // ── immutableAllowSetting ───────────────────────────────────────────

    #[tokio::test]
    async fn should_allow_setting_an_absent_allow_setting_property_once() {
        // `mood` is immutable but may be set while absent; created without it.
        let mut fixture = PostFixture::new_with_lists(
            platform_value!(["author", "mood"]),
            platform_value!(["mood"]),
            |_| {},
        )
        .await;

        // The first-time set is accepted and stored.
        let result = fixture
            .replace(|post| post.set("mood", "cheerful".into()))
            .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. },
            "setting an absent allow-setting property must succeed"
        );
        assert_eq!(
            fixture.stored_post().properties().get("mood"),
            Some(&text("cheerful"))
        );

        // From here on it is frozen: neither a change nor a removal passes.
        let result = fixture
            .replace(|post| post.set("mood", "gloomy".into()))
            .await;
        expect_immutable_property_error(result, "mood", &fixture);

        let result = fixture
            .replace(|post| {
                post.remove("mood");
            })
            .await;
        expect_immutable_property_error(result, "mood", &fixture);

        // The rest of the document stays editable throughout.
        let result = fixture
            .replace(|post| post.set("body", "second draft".into()))
            .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        assert_eq!(
            fixture.stored_post().properties().get("mood"),
            Some(&text("cheerful"))
        );
    }

    #[tokio::test]
    async fn should_keep_an_allow_setting_property_frozen_when_set_at_creation() {
        // Created WITH `mood`: the allowance never applies because the stored
        // document always had a value.
        let mut fixture = PostFixture::new_with_lists(
            platform_value!(["mood"]),
            platform_value!(["mood"]),
            |post| post.set("mood", "cheerful".into()),
        )
        .await;

        let result = fixture
            .replace(|post| post.set("mood", "gloomy".into()))
            .await;
        expect_immutable_property_error(result, "mood", &fixture);

        let result = fixture
            .replace(|post| {
                post.remove("mood");
            })
            .await;
        expect_immutable_property_error(result, "mood", &fixture);
    }

    #[tokio::test]
    async fn should_not_let_allow_setting_relax_a_different_immutable_property() {
        // Only `mood` may be set late; `author` is fully frozen, and an absent
        // immutable property outside the allowance (`meta`) still cannot be
        // added.
        let mut fixture = PostFixture::new_with_lists(
            platform_value!(["author", "mood", "meta"]),
            platform_value!(["mood"]),
            |_| {},
        )
        .await;

        let result = fixture
            .replace(|post| post.set("author", "mallory".into()))
            .await;
        expect_immutable_property_error(result, "author", &fixture);

        let result = fixture
            .replace(|post| post.set("meta", platform_value!({"tag": "intro"})))
            .await;
        expect_immutable_property_error(result, "meta", &fixture);
    }

    #[tokio::test]
    async fn should_reject_a_contract_update_that_allows_setting_an_already_immutable_property() {
        let mut fixture = PostFixture::new(platform_value!(["author", "mood"]), |_| {}).await;

        let result = fixture
            .update_lists(
                platform_value!(["author", "mood"]),
                platform_value!(["mood"]),
            )
            .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(StateError::DocumentTypeUpdateError(_)),
                ..
            },
            "an already-immutable property must not start allowing a set"
        );

        // Still fully frozen.
        let result = fixture
            .replace(|post| post.set("mood", "cheerful".into()))
            .await;
        expect_immutable_property_error(result, "mood", &fixture);
    }

    #[tokio::test]
    async fn should_accept_a_contract_update_adding_a_newly_immutable_property_that_allows_setting()
    {
        let mut fixture = PostFixture::new(platform_value!(["author"]), |_| {}).await;

        let result = fixture
            .update_lists(
                platform_value!(["author", "mood"]),
                platform_value!(["mood"]),
            )
            .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. },
            "a newly immutable property may arrive with the allowance"
        );
        let expected: BTreeSet<String> = ["mood"].into_iter().map(String::from).collect();
        assert_eq!(
            fixture
                .contract
                .document_type_for_name("post")
                .expect("expected the post document type")
                .immutable_fields_allow_setting(),
            &expected
        );

        // Set once, then frozen.
        let result = fixture
            .replace(|post| post.set("mood", "cheerful".into()))
            .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        let result = fixture
            .replace(|post| post.set("mood", "gloomy".into()))
            .await;
        expect_immutable_property_error(result, "mood", &fixture);
    }
}
