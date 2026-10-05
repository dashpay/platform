//! End-to-end coverage for the `generatedFrom` property keyword (protocol
//! version 14): a string property the platform generates with a built-in
//! function of other properties of the same document, here
//! `sys.stringTransformations.homographSafeASCII`. When a created or replaced
//! document leaves it out, the action transformer generates it from its params
//! before anything reads the document; when the document supplies it, the
//! document validation checks it after the JSON schema, and a wrong value, or
//! one without its params, is consensus-rejected and leaves the stored document
//! untouched.

use super::*;

mod generated_from_tests {
    use super::*;
    use crate::execution::validation::state_transition::batch::action_validation::document::document_replace_transition_action::DocumentReplaceTransitionActionValidation;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TempPlatform;
    use dpp::consensus::basic::BasicError;
    use dpp::consensus::state::state_error::StateError;
    use dpp::data_contract::schema::DataContractSchemaMethodsV0;
    use dpp::document::Document;
    use dpp::identity::{Identity, IdentityPublicKey, SecurityLevel};
    use dpp::platform_value::platform_value;
    use dpp::prelude::Identifier;
    use dpp::prelude::{DataContract, IdentityNonce};
    use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
    use dpp::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
    use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransitionV0Methods;
    use dpp::state_transition::batch_transition::batched_transition::document_create_transition::v0::v0_methods::DocumentCreateTransitionV0Methods;
    use dpp::state_transition::batch_transition::batched_transition::{
        BatchedTransitionMutRef, BatchedTransitionRef,
    };
    use dpp::state_transition::proof_result::StateTransitionProofResult;
    use dpp::state_transition::StateTransition;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
    use drive::drive::contract::DataContractFetchInfo;
    use drive::drive::Drive;
    use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::{DocumentBaseTransitionAction, DocumentBaseTransitionActionV0};
    use drive::state_transition_action::batch::batched_transition::document_transition::document_create_transition_action::{DocumentCreateTransitionAction, DocumentCreateTransitionActionAccessorsV0};
    use drive::state_transition_action::batch::batched_transition::document_transition::document_replace_transition_action::{DocumentReplaceTransitionAction, DocumentReplaceTransitionActionAccessorsV0, DocumentReplaceTransitionActionV0};
    use drive::state_transition_action::batch::batched_transition::document_transition::DocumentTransitionAction;
    use drive::state_transition_action::batch::batched_transition::BatchedTransitionAction;
    use drive::util::storage_flags::StorageFlags;
    use simple_signer::signer::SimpleSigner;
    use std::collections::{BTreeMap, BTreeSet};
    use std::sync::Arc;

    /// A mutable `handle` type shaped like DPNS's domain: a `label` and its
    /// required `normalizedLabel`, unique across handles, and an optional
    /// `parent` with its optional `normalizedParent`, each generated from its
    /// counterpart. The params' patterns hold them to ASCII, as DPNS's do; the
    /// generated properties need none of their own, since each can only hold
    /// what its function generates.
    fn handle_schema() -> Value {
        platform_value!({
            "type": "object",
            "documentsMutable": true,
            "indices": [
                {
                    "name": "byNormalizedLabel",
                    "properties": [{ "normalizedLabel": "asc" }],
                    "unique": true
                }
            ],
            "properties": {
                "label": {
                    "type": "string",
                    "pattern": "^[a-zA-Z0-9-]{1,32}$",
                    "maxLength": 32,
                    "position": 0
                },
                "normalizedLabel": {
                    "type": "string",
                    "maxLength": 32,
                    "generatedFrom": {
                        "function": "sys.stringTransformations.homographSafeASCII",
                        "params": ["label"]
                    },
                    "position": 1
                },
                "parent": {
                    "type": "string",
                    "pattern": "^[a-zA-Z0-9-]{1,32}$",
                    "maxLength": 32,
                    "position": 2
                },
                "normalizedParent": {
                    "type": "string",
                    "maxLength": 32,
                    "generatedFrom": {
                        "function": "sys.stringTransformations.homographSafeASCII",
                        "params": ["parent"]
                    },
                    "position": 3
                }
            },
            "required": ["label", "normalizedLabel"],
            "additionalProperties": false
        })
    }

    /// An indexOnly `entry` type: a `name` and its `normalizedName`, both
    /// stored only in the one index, whose terminal is the owner.
    fn entry_schema() -> Value {
        platform_value!({
            "type": "object",
            "indexOnly": true,
            "documentsMutable": false,
            "indices": [
                {
                    "name": "byNormalizedName",
                    "properties": [{ "normalizedName": "asc" }, { "name": "asc" }],
                    "terminal": "$ownerId"
                }
            ],
            "properties": {
                "name": { "type": "string", "pattern": "^[a-zA-Z0-9-]{1,32}$", "maxLength": 32, "position": 0 },
                "normalizedName": {
                    "type": "string",
                    "maxLength": 32,
                    "generatedFrom": {
                        "function": "sys.stringTransformations.homographSafeASCII",
                        "params": ["name"]
                    },
                    "position": 1
                }
            },
            "required": ["name", "normalizedName"],
            "additionalProperties": false
        })
    }

    /// An immutable `name` type whose unique index over `normalizedLabel` is
    /// contested, as DPNS's `domain` is: a short normalized label opens a
    /// masternode vote.
    fn name_schema() -> Value {
        platform_value!({
            "type": "object",
            "documentsMutable": false,
            "indices": [
                {
                    "name": "byNormalizedLabel",
                    "properties": [{ "normalizedLabel": "asc" }],
                    "unique": true,
                    "contested": {
                        "fieldMatches": [
                            { "field": "normalizedLabel", "regexPattern": "^[a-zA-Z01-]{3,19}$" }
                        ],
                        "resolution": 0
                    }
                }
            ],
            "properties": {
                "label": { "type": "string", "pattern": "^[a-zA-Z0-9-]{3,32}$", "maxLength": 32, "position": 0 },
                "normalizedLabel": {
                    "type": "string",
                    "maxLength": 32,
                    "generatedFrom": {
                        "function": "sys.stringTransformations.homographSafeASCII",
                        "params": ["label"]
                    },
                    "position": 1
                }
            },
            "required": ["label", "normalizedLabel"],
            "additionalProperties": false
        })
    }

    fn text(value: &str) -> Value {
        Value::Text(value.to_string())
    }

    /// One identity and one contract holding the `handle`, `entry` and `name`
    /// types above.
    struct HandleFixture {
        platform: TempPlatform<MockCoreRPCLike>,
        signer: SimpleSigner,
        key: IdentityPublicKey,
        identity: Identity,
        contract: DataContract,
        /// The identity contract nonce the next transition uses. Every
        /// processed transition consumes one, including the ones that fail
        /// with a paid consensus error.
        next_nonce: IdentityNonce,
    }

    impl HandleFixture {
        fn new() -> Self {
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
            for (name, schema) in [
                ("handle", handle_schema()),
                ("entry", entry_schema()),
                ("name", name_schema()),
            ] {
                contract
                    .set_document_schema(name, schema, true, &mut Vec::new(), platform_version)
                    .unwrap_or_else(|e| panic!("expected to add the {name} document type: {e}"));
            }
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

            Self {
                platform,
                signer,
                key,
                identity,
                contract,
                next_nonce: 1,
            }
        }

        /// A create transition for a document of `type_name` holding `properties`,
        /// and the document. The builder generates a property the
        /// document leaves out, as the platform would; `as_given` sends the
        /// document exactly as `properties` says instead, so what the test sees is
        /// the platform's own computation.
        async fn create_transition_of(
            &mut self,
            type_name: &str,
            properties: Value,
            seed: u64,
            as_given: bool,
        ) -> (Document, StateTransition) {
            let platform_version = PlatformVersion::latest();
            let document_type = self
                .contract
                .document_type_for_name(type_name)
                .expect("expected the document type");
            let mut rng = StdRng::seed_from_u64(seed);
            let entropy = Bytes32::random_with_rng(&mut rng);
            let mut document = document_type
                .random_document_with_identifier_and_entropy(
                    &mut rng,
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
            let properties = properties
                .into_btree_string_map()
                .expect("the properties are a map");
            *document.properties_mut() = properties.clone();

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
            let transition = if as_given {
                self.send_as_given(transition, properties).await
            } else {
                transition
            };
            (document, transition)
        }

        /// A create transition for a handle holding exactly `properties`.
        async fn create_transition(&mut self, properties: Value, seed: u64) -> StateTransition {
            self.create_transition_of("handle", properties, seed, true)
                .await
                .1
        }

        /// `transition` with its document's data set to exactly `data`, signed
        /// again.
        async fn send_as_given(
            &self,
            mut transition: StateTransition,
            data: BTreeMap<String, Value>,
        ) -> StateTransition {
            {
                let StateTransition::Batch(batch) = &mut transition else {
                    panic!("expected a batch transition");
                };
                let Some(BatchedTransitionMutRef::Document(document_transition)) =
                    batch.first_transition_mut()
                else {
                    panic!("expected a document transition");
                };
                *document_transition
                    .data_mut()
                    .expect("the document transition carries data") = data;
            }
            transition
                .sign_external(
                    &self.key,
                    &self.signer,
                    Some(|_, _| Ok(SecurityLevel::HIGH)),
                )
                .await
                .expect("expected to sign the transition again");
            transition
        }

        async fn create(&mut self, properties: Value, seed: u64) -> StateTransitionExecutionResult {
            let transition = self.create_transition(properties, seed).await;
            self.process(&transition)
        }

        /// A replace transition for `stored` with `mutate` applied and the revision
        /// bumped, sent as given.
        async fn replace_transition(
            &mut self,
            stored: &Document,
            mutate: impl FnOnce(&mut Document),
        ) -> StateTransition {
            let platform_version = PlatformVersion::latest();
            let mut replacement = stored.clone();
            mutate(&mut replacement);
            replacement
                .increment_revision()
                .expect("expected the revision to increment");
            let handle_type = self
                .contract
                .document_type_for_name("handle")
                .expect("expected the handle document type");
            let properties = replacement.properties().clone();
            let transition = BatchTransition::new_document_replacement_transition_from_document(
                replacement,
                handle_type,
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
            self.send_as_given(transition, properties).await
        }

        /// Replaces `stored` with `mutate` applied and the revision bumped.
        async fn replace(
            &mut self,
            stored: &Document,
            mutate: impl FnOnce(&mut Document),
        ) -> StateTransitionExecutionResult {
            let transition = self.replace_transition(stored, mutate).await;
            self.process(&transition)
        }

        /// Proves the committed state `transition` wrote and verifies the proof as
        /// a client holding the contract does.
        fn verify_proof(&self, transition: &StateTransition) -> StateTransitionProofResult {
            let platform_version = PlatformVersion::latest();
            let proof = self
                .platform
                .drive
                .prove_state_transition(transition, None, platform_version)
                .expect("expected to prove the state transition")
                .into_data()
                .expect("expected proof bytes");
            let known_contracts: BTreeMap<Identifier, DataContract> =
                BTreeMap::from([(self.contract.id(), self.contract.clone())]);
            let (_, outcome) = Drive::verify_state_transition_was_executed_with_proof(
                transition,
                &BlockInfo::default(),
                &proof,
                &|id| Ok(known_contracts.get(id).cloned().map(Arc::new)),
                platform_version,
            )
            .expect("expected the proof to verify");
            outcome.into_result()
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

        /// The stored handles, read back from Drive.
        fn stored_handles(&self) -> Vec<Document> {
            let platform_version = PlatformVersion::latest();
            let query = DriveDocumentQuery::from_sql_expr(
                "select * from handle",
                &self.contract,
                Some(&self.platform.config.drive),
                platform_version,
            )
            .expect("expected a document query");
            self.platform
                .drive
                .query_documents(query, None, false, None, None)
                .expect("expected a query result")
                .documents()
                .to_vec()
        }

        /// The contract as Drive hands it to the transformers and validators.
        fn contract_fetch_info(&self) -> Arc<DataContractFetchInfo> {
            let (_, contract_fetch_info) = self
                .platform
                .drive
                .get_contract_with_fetch_info_and_fee(
                    self.contract.id().to_buffer(),
                    None,
                    false,
                    None,
                    PlatformVersion::latest(),
                )
                .expect("expected to fetch the contract");
            contract_fetch_info.expect("the contract is in state")
        }
    }

    fn assert_success(result: &StateTransitionExecutionResult) {
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. },
            "{result:?}"
        );
    }

    fn expect_not_generated_error(
        result: StateTransitionExecutionResult,
        property: &str,
        param: &str,
    ) {
        let StateTransitionExecutionResult::PaidConsensusError { error, .. } = result else {
            panic!("expected a paid consensus error, got {result:?}");
        };
        assert_matches!(
            error,
            ConsensusError::BasicError(BasicError::DocumentPropertyNotGeneratedError(e))
                if e.property() == property
                    && e.params() == [param.to_string()]
                    && e.function() == "sys.stringTransformations.homographSafeASCII"
        );
    }

    #[tokio::test]
    async fn should_generate_a_left_out_property_on_arrival() {
        let mut fixture = HandleFixture::new();

        let result = fixture
            .create(platform_value!({ "label": "Bob", "parent": "Dash-Oil" }), 1)
            .await;

        assert_success(&result);
        let stored = fixture.stored_handles();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].get("normalizedLabel"), Some(&text("b0b")));
        assert_eq!(stored[0].get("normalizedParent"), Some(&text("dash-011")));
    }

    #[tokio::test]
    async fn should_accept_a_supplied_property_equal_to_the_generated_one() {
        let mut fixture = HandleFixture::new();

        let result = fixture
            .create(
                platform_value!({ "label": "Alice", "normalizedLabel": "a11ce" }),
                2,
            )
            .await;

        assert_success(&result);
        let stored = fixture.stored_handles();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].get("normalizedLabel"), Some(&text("a11ce")));
        // Its source is absent, so it is too
        assert_eq!(stored[0].get("normalizedParent"), None);
    }

    /// "bob" is "Bob" lowercased without the homograph mapping. The normalized
    /// property declares no pattern, so the keyword is what refuses it.
    #[tokio::test]
    async fn should_refuse_a_supplied_property_that_differs_from_the_generated_one() {
        let mut fixture = HandleFixture::new();

        let result = fixture
            .create(
                platform_value!({ "label": "Bob", "normalizedLabel": "bob" }),
                3,
            )
            .await;

        expect_not_generated_error(result, "normalizedLabel", "label");
        assert!(fixture.stored_handles().is_empty());
    }

    #[tokio::test]
    async fn should_refuse_a_generated_property_without_its_param() {
        let mut fixture = HandleFixture::new();

        let result = fixture
            .create(
                platform_value!({ "label": "Carl", "normalizedParent": "dash" }),
                4,
            )
            .await;

        expect_not_generated_error(result, "normalizedParent", "parent");
        assert!(fixture.stored_handles().is_empty());
    }

    /// The computed value is what the unique index holds: a second handle whose
    /// label only differs by case and homographs collides with the first, though
    /// neither transition carried the normalized form.
    #[tokio::test]
    async fn should_find_a_unique_index_collision_through_the_computed_value() {
        let mut fixture = HandleFixture::new();

        assert_success(&fixture.create(platform_value!({ "label": "Bob" }), 5).await);

        let result = fixture.create(platform_value!({ "label": "B0B" }), 6).await;
        let StateTransitionExecutionResult::PaidConsensusError { error, .. } = result else {
            panic!("expected a paid consensus error, got {result:?}");
        };
        assert_matches!(
            error,
            ConsensusError::StateError(StateError::DuplicateUniqueIndexError(_))
        );
        assert_eq!(fixture.stored_handles().len(), 1);
    }

    #[tokio::test]
    async fn should_regenerate_a_left_out_property_on_replace() {
        let mut fixture = HandleFixture::new();
        assert_success(&fixture.create(platform_value!({ "label": "Bob" }), 7).await);
        let stored = fixture.stored_handles().remove(0);

        // A new label with the normalized form left out: computed again
        let result = fixture
            .replace(&stored, |handle| {
                handle.set("label", text("Bobby-Lee"));
                handle.remove("normalizedLabel");
            })
            .await;
        assert_success(&result);
        let replaced = fixture.stored_handles().remove(0);
        assert_eq!(replaced.get("normalizedLabel"), Some(&text("b0bby-1ee")));

        // A stale normalized form sent with the new label is refused
        let result = fixture
            .replace(&replaced, |handle| {
                handle.set("label", text("Robin"));
            })
            .await;
        expect_not_generated_error(result, "normalizedLabel", "label");
        let after = fixture.stored_handles().remove(0);
        assert_eq!(after.get("label"), Some(&text("Bobby-Lee")));
        assert_eq!(after.get("normalizedLabel"), Some(&text("b0bby-1ee")));
    }

    /// A client reading the create back with a proof rebuilds the document the
    /// transition wrote, computed property included, and the proof verifies.
    #[tokio::test]
    async fn should_verify_the_proof_of_a_create_that_left_the_generated_property_out() {
        let mut fixture = HandleFixture::new();
        let transition = fixture
            .create_transition(platform_value!({ "label": "Olive" }), 8)
            .await;
        assert_success(&fixture.process(&transition));

        let StateTransitionProofResult::VerifiedDocuments(documents) =
            fixture.verify_proof(&transition)
        else {
            panic!("expected verified documents");
        };
        let document = documents
            .into_values()
            .next()
            .flatten()
            .expect("the proof holds the created document");
        assert_eq!(document.get("normalizedLabel"), Some(&text("011ve")));
    }

    /// The replace a client proves is rebuilt from the transition with the
    /// property it left out computed from the new source.
    #[tokio::test]
    async fn should_verify_the_proof_of_a_replace_that_left_the_generated_property_out() {
        let mut fixture = HandleFixture::new();
        assert_success(
            &fixture
                .create(platform_value!({ "label": "Bob" }), 10)
                .await,
        );
        let stored = fixture.stored_handles().remove(0);

        let transition = fixture
            .replace_transition(&stored, |handle| {
                handle.set("label", text("Oliver"));
                handle.remove("normalizedLabel");
            })
            .await;
        assert_success(&fixture.process(&transition));

        let StateTransitionProofResult::VerifiedDocuments(documents) =
            fixture.verify_proof(&transition)
        else {
            panic!("expected verified documents");
        };
        let document = documents
            .into_values()
            .next()
            .flatten()
            .expect("the proof holds the replaced document");
        assert_eq!(document.get("normalizedLabel"), Some(&text("011ver")));
    }

    /// An indexOnly entry has no primary row: its create and delete are proved
    /// and executed through the entry its values produce, so the property the
    /// transitions leave out must be computed on both sides, by the node and by
    /// the verifier. The delete names the entry without the normalized value and
    /// still finds it (a delete of a missing entry is refused).
    #[tokio::test]
    async fn should_create_prove_and_delete_an_index_only_entry_that_leaves_the_generated_property_out(
    ) {
        let platform_version = PlatformVersion::latest();
        let mut fixture = HandleFixture::new();
        let (entry, create) = fixture
            .create_transition_of("entry", platform_value!({ "name": "Bob" }), 11, true)
            .await;
        assert_success(&fixture.process(&create));

        let StateTransitionProofResult::VerifiedDocuments(documents) =
            fixture.verify_proof(&create)
        else {
            panic!("expected verified documents");
        };
        let proved = documents
            .into_values()
            .next()
            .flatten()
            .expect("the proof holds the created entry");
        assert_eq!(proved.get("normalizedName"), Some(&text("b0b")));

        let entry_type = fixture
            .contract
            .document_type_for_name("entry")
            .expect("expected the entry document type");
        let delete = BatchTransition::new_document_deletion_transition_from_document(
            entry.clone(),
            entry_type,
            &fixture.key,
            fixture.next_nonce,
            0,
            None,
            &fixture.signer,
            platform_version,
            None,
        )
        .await
        .expect("expected the delete transition");
        fixture.next_nonce += 1;
        let delete = fixture
            .send_as_given(delete, entry.properties().clone())
            .await;
        assert_success(&fixture.process(&delete));

        let StateTransitionProofResult::VerifiedDocuments(documents) =
            fixture.verify_proof(&delete)
        else {
            panic!("expected verified documents");
        };
        assert_eq!(documents.into_values().next(), Some(None));
    }

    /// A contested index over the generated property: a document built by the
    /// SDK without the property carries its contest, because the builder
    /// computes the property before it resolves the contest; and a transition
    /// sent without the property, with its contest named, is resolved by the
    /// node against the value it computes.
    #[tokio::test]
    async fn should_resolve_the_contest_of_a_document_that_leaves_the_generated_property_out() {
        let mut fixture = HandleFixture::new();

        let (_, built) = fixture
            .create_transition_of("name", platform_value!({ "label": "Bob" }), 12, false)
            .await;
        let StateTransition::Batch(batch) = &built else {
            panic!("expected a batch transition");
        };
        let Some(BatchedTransitionRef::Document(DocumentTransition::Create(create))) =
            batch.first_transition()
        else {
            panic!("expected a document create");
        };
        assert_eq!(create.data().get("normalizedLabel"), Some(&text("b0b")));
        assert_eq!(
            create
                .prefunded_voting_balance()
                .as_ref()
                .map(|(index, _)| index.as_str()),
            Some("byNormalizedLabel")
        );
        assert_success(&fixture.process(&built));

        let (_, as_given) = fixture
            .create_transition_of("name", platform_value!({ "label": "Alice" }), 13, true)
            .await;
        assert_success(&fixture.process(&as_given));
    }

    /// The create transformer computes the property only from protocol version
    /// 14: before it, the data goes on as sent.
    #[tokio::test]
    async fn should_not_generate_the_property_before_protocol_version_14() {
        let mut fixture = HandleFixture::new();
        let transition = fixture
            .create_transition(platform_value!({ "label": "Bob" }), 9)
            .await;
        let StateTransition::Batch(batch) = &transition else {
            panic!("expected a batch transition");
        };
        let create_transition = match batch.first_transition() {
            Some(BatchedTransitionRef::Document(DocumentTransition::Create(create))) => create,
            other => panic!("expected a document create, got {other:?}"),
        };
        let contract_fetch_info = fixture.contract_fetch_info();

        let action_data = |platform_version: &PlatformVersion| {
            let (result, _) =
                DocumentCreateTransitionAction::try_from_document_borrowed_create_transition_with_contract_lookup(
                    &fixture.platform.drive,
                    fixture.identity.id(),
                    None,
                    create_transition,
                    &BlockInfo::default(),
                    0,
                    |_| Ok(contract_fetch_info.clone()),
                    platform_version,
                )
                .expect("expected the transformer to run");
            match result.into_data().expect("expected an action") {
                BatchedTransitionAction::DocumentAction(
                    DocumentTransitionAction::CreateAction(action),
                ) => action.data().clone(),
                other => panic!("expected a create action, got {other:?}"),
            }
        };

        let before = action_data(PlatformVersion::get(13).expect("protocol version 13"));
        assert_eq!(before.get("normalizedLabel"), None);

        let at = action_data(PlatformVersion::latest());
        assert_eq!(at.get("normalizedLabel"), Some(&text("b0b")));
    }

    /// The replace transformer, edited in place, generates the property only
    /// from protocol version 14: before it, the data goes on as sent.
    #[tokio::test]
    async fn should_not_regenerate_the_property_on_replace_before_protocol_version_14() {
        let mut fixture = HandleFixture::new();
        assert_success(
            &fixture
                .create(platform_value!({ "label": "Bob" }), 20)
                .await,
        );
        let stored = fixture.stored_handles().remove(0);
        let transition = fixture
            .replace_transition(&stored, |handle| {
                handle.set("label", text("Robin"));
                handle.remove("normalizedLabel");
            })
            .await;
        let StateTransition::Batch(batch) = &transition else {
            panic!("expected a batch transition");
        };
        let replace_transition = match batch.first_transition() {
            Some(BatchedTransitionRef::Document(DocumentTransition::Replace(replace))) => replace,
            other => panic!("expected a document replace, got {other:?}"),
        };
        let contract_fetch_info = fixture.contract_fetch_info();

        let action_data = |platform_version: &PlatformVersion| {
            let (result, _) =
                DocumentReplaceTransitionAction::try_from_borrowed_document_replace_transition(
                    replace_transition,
                    fixture.identity.id(),
                    &stored,
                    &BlockInfo::default(),
                    0,
                    |_| Ok(contract_fetch_info.clone()),
                    platform_version,
                )
                .expect("expected the transformer to run");
            match result.into_data().expect("expected an action") {
                BatchedTransitionAction::DocumentAction(
                    DocumentTransitionAction::ReplaceAction(action),
                ) => action.data().clone(),
                other => panic!("expected a replace action, got {other:?}"),
            }
        };

        let before = action_data(PlatformVersion::get(13).expect("protocol version 13"));
        assert_eq!(before.get("label"), Some(&text("Robin")));
        assert_eq!(before.get("normalizedLabel"), None);

        let at = action_data(PlatformVersion::latest());
        assert_eq!(at.get("normalizedLabel"), Some(&text("r0b1n")));
    }

    /// The replace structure dispatcher on both sides of the gate: the document
    /// validation it runs gained the check in place, so at protocol version 13
    /// it must still accept the action (the dpp gate is `None` there), and at
    /// 14 refuse it. The action is built by hand the way the transformer would
    /// build it, against the contract as Drive hands it back.
    #[test]
    fn should_not_check_the_generated_property_before_protocol_version_14() {
        let fixture = HandleFixture::new();
        let owner_id = fixture.identity.id();
        let contract_fetch_info = fixture.contract_fetch_info();

        let action = || {
            DocumentReplaceTransitionAction::V0(DocumentReplaceTransitionActionV0 {
                base: DocumentBaseTransitionAction::V0(DocumentBaseTransitionActionV0 {
                    id: Identifier::from([0xAA; 32]),
                    identity_contract_nonce: 1,
                    document_type_name: "handle".to_string(),
                    data_contract: contract_fetch_info.clone(),
                    token_cost: None,
                    shielded_token_payment: None,
                    gas_fees_paid_by: GasFeesPaidBy::default(),
                    contract_gas_fees_paid_by: GasFeesPaidBy::default(),
                    declared_action_fee: None,
                }),
                revision: 2,
                created_at: None,
                updated_at: None,
                transferred_at: None,
                created_at_block_height: None,
                updated_at_block_height: None,
                transferred_at_block_height: None,
                created_at_core_block_height: None,
                updated_at_core_block_height: None,
                transferred_at_core_block_height: None,
                data: BTreeMap::from([
                    ("label".to_string(), text("Bob")),
                    ("normalizedLabel".to_string(), text("b1b")),
                ]),
                changed_data_fields: BTreeSet::new(),
                removed_identifier_fields: BTreeMap::new(),
                stored_changed_values: BTreeMap::new(),
                creator_id: None,
                moderated_at: None,
                moderated_by: None,
                property_constraint_aggregates: Default::default(),
            })
        };

        let before = action()
            .validate_structure(
                owner_id,
                PlatformVersion::get(13).expect("platform version 13 should exist"),
            )
            .expect("structure validation should run");
        assert!(
            before.is_valid(),
            "the document validation must not check generatedFrom before 14: {:?}",
            before.errors
        );

        let at = action()
            .validate_structure(owner_id, PlatformVersion::latest())
            .expect("structure validation should run");
        assert_matches!(
            at.errors.as_slice(),
            [ConsensusError::BasicError(BasicError::DocumentPropertyNotGeneratedError(e))]
                if e.property() == "normalizedLabel"
        );
    }
}
