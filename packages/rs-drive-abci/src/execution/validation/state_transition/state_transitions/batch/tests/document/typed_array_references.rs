//! `refersTo` on the elements of a typed array through the full ABCI
//! pipeline. The fixture's `submittedCharter.reasons` is a list of
//! `permanentDocument` references to `reason`, `gatedCharter.reasons` binds
//! the WRITER to every referenced reason's owner, `topicCharter.reasons`
//! binds the charter's `topic` to every reason's, and `draftList.drafts` is
//! a list of `deletableDocument` references to `draft`. `plainCharter` has
//! the `reasons` shape without `refersTo`, as the fee baseline.
//!
//! Each element is checked as a single reference is, in list order, and the
//! first one that fails refuses the write with the single reference's
//! error, naming the element by its list path.

use super::*;

mod typed_array_reference_tests {
    use super::*;
    use crate::platform_types::platform_state::PlatformState;
    use crate::platform_types::state_transitions_processing_result::StateTransitionsProcessingResult;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TempPlatform;
    use dpp::consensus::codes::ErrorWithCode;
    use dpp::data_contract::accessors::v0::DataContractV0Setters;
    use dpp::document::Document;
    use dpp::identifier::Identifier;
    use dpp::identity::signer::Signer;
    use dpp::identity::IdentityPublicKey;
    use dpp::prelude::DataContract;
    use dpp::state_transition::StateTransition;
    use simple_signer::signer::SimpleSigner;
    use std::sync::Arc;

    /// Shared with the contract-create registration test, which pins that
    /// the declarations themselves register.
    const CONTRACT_PATH: &str = "tests/supporting_files/contract/reference-validation/reference-validation-contract-typed-array-elements.json";

    fn register_contract(
        platform: &TempPlatform<MockCoreRPCLike>,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> DataContract {
        let mut contract = json_document_to_contract(CONTRACT_PATH, true, platform_version)
            .expect("expected to parse the typed array reference contract");
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
            .expect("expected to apply the typed array reference contract");
        contract
    }

    fn process_and_commit(
        platform: &TempPlatform<MockCoreRPCLike>,
        platform_state: &PlatformState,
        transition: &StateTransition,
        platform_version: &PlatformVersion,
    ) -> StateTransitionsProcessingResult {
        let serialized = transition
            .serialize_to_bytes()
            .expect("expected the batch transition to serialize");
        let transaction = platform.drive.grove.start_transaction();
        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[serialized],
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

    fn assert_successful(result: &StateTransitionsProcessingResult, because: &str) {
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }],
            "{because}"
        );
    }

    /// The write was refused because the referenced entity `missing` named at
    /// `path` does not exist (code 40120).
    fn assert_element_not_found(
        result: &StateTransitionsProcessingResult,
        path: &str,
        missing: Identifier,
        because: &str,
    ) {
        let [StateTransitionExecutionResult::PaidConsensusError { error, .. }] =
            result.execution_results().as_slice()
        else {
            panic!("{because}: expected one paid consensus error, got {result:?}");
        };
        assert_eq!(error.code(), 40120, "{because}: {error}");
        assert_matches!(
            error,
            ConsensusError::StateError(StateError::ReferencedEntityNotFoundError(not_found))
                if not_found.path() == path && *not_found.entity_id() == missing,
            "{because}"
        );
    }

    /// A list of identifiers as a document stores it.
    fn identifier_list(ids: &[Identifier]) -> Value {
        Value::Array(
            ids.iter()
                .map(|id| Value::Identifier(id.to_buffer()))
                .collect(),
        )
    }

    /// Creates a document of `type_name` with exactly `properties` set and
    /// returns it with the processing result.
    #[allow(clippy::too_many_arguments)]
    async fn create_document<S: Signer<IdentityPublicKey>>(
        platform: &TempPlatform<MockCoreRPCLike>,
        platform_state: &PlatformState,
        contract: &DataContract,
        type_name: &str,
        properties: &[(&str, Value)],
        owner: Identifier,
        key: &IdentityPublicKey,
        nonce: u64,
        signer: &S,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> (Document, StateTransitionsProcessingResult) {
        let document_type = contract
            .document_type_for_name(type_name)
            .expect("doctype exists");
        let entropy = Bytes32::random_with_rng(rng);
        let mut document = document_type
            .random_document_with_identifier_and_entropy(
                rng,
                owner,
                entropy,
                DocumentFieldFillType::DoNotFillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");
        for (property, value) in properties {
            document.set(property, value.clone());
        }
        // The id commits to the create transition's nonce: give the local
        // copy the id the transition will carry, since the tests reference
        // and act on it afterwards.
        document
            .set_id_for_creation(document_type, &entropy.0, nonce, platform_version)
            .expect("expected the creation id");
        let create = BatchTransition::new_document_creation_transition_from_document(
            document.clone(),
            document_type,
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
        .expect("expected the create transition");
        let result = process_and_commit(platform, platform_state, &create, platform_version);
        (document, result)
    }

    struct Setup {
        platform: TempPlatform<MockCoreRPCLike>,
        platform_state: Arc<PlatformState>,
        contract: DataContract,
        owner: Identifier,
        signer: SimpleSigner,
        key: IdentityPublicKey,
        rng: StdRng,
        nonce: u64,
    }

    impl Setup {
        fn new(seed: u64) -> Self {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();
            let platform_state = platform.state.load_full();
            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(1.0));
            let contract = register_contract(&platform, identity.id(), platform_version);
            Self {
                platform,
                platform_state,
                contract,
                owner: identity.id(),
                signer,
                key,
                rng: StdRng::seed_from_u64(seed),
                nonce: 1,
            }
        }

        fn next_nonce(&mut self) -> u64 {
            self.nonce += 1;
            self.nonce
        }

        async fn create(
            &mut self,
            type_name: &str,
            properties: &[(&str, Value)],
        ) -> (Document, StateTransitionsProcessingResult) {
            let nonce = self.next_nonce();
            create_document(
                &self.platform,
                &self.platform_state,
                &self.contract,
                type_name,
                properties,
                self.owner,
                &self.key,
                nonce,
                &self.signer,
                &mut self.rng,
                PlatformVersion::latest(),
            )
            .await
        }

        /// Bumps the revision and replaces `document` as it now stands.
        async fn replace(
            &mut self,
            type_name: &str,
            document: &mut Document,
        ) -> StateTransitionsProcessingResult {
            let platform_version = PlatformVersion::latest();
            document.increment_revision().expect("revision increments");
            let nonce = self.next_nonce();
            let document_type = self
                .contract
                .document_type_for_name(type_name)
                .expect("doctype exists");
            let replace = BatchTransition::new_document_replacement_transition_from_document(
                document.clone(),
                document_type,
                &self.key,
                nonce,
                0,
                None,
                &self.signer,
                platform_version,
                None,
            )
            .await
            .expect("expected the replace transition");
            let result = process_and_commit(
                &self.platform,
                &self.platform_state,
                &replace,
                platform_version,
            );
            // A refused replace leaves the stored revision where it was.
            if result.valid_count() == 0 {
                let revision = document.revision().expect("revision set");
                document.set_revision(Some(revision - 1));
            }
            result
        }

        async fn delete(
            &mut self,
            type_name: &str,
            document: &Document,
        ) -> StateTransitionsProcessingResult {
            let platform_version = PlatformVersion::latest();
            let nonce = self.next_nonce();
            let document_type = self
                .contract
                .document_type_for_name(type_name)
                .expect("doctype exists");
            let delete = BatchTransition::new_document_deletion_transition_from_document(
                document.clone(),
                document_type,
                &self.key,
                nonce,
                0,
                None,
                &self.signer,
                platform_version,
                None,
            )
            .await
            .expect("expected the delete transition");
            process_and_commit(
                &self.platform,
                &self.platform_state,
                &delete,
                platform_version,
            )
        }

        /// A reason owned by the setup's identity.
        async fn reason(&mut self, topic: &str) -> Identifier {
            let (reason, result) = self.create("reason", &[("topic", topic.into())]).await;
            assert_successful(&result, "the reason is created");
            reason.id()
        }

        async fn reasons(&mut self, count: usize) -> Vec<Identifier> {
            let mut reasons = Vec::with_capacity(count);
            for _ in 0..count {
                reasons.push(self.reason("dash").await);
            }
            reasons
        }
    }

    #[tokio::test]
    async fn should_accept_a_list_whose_every_element_exists() {
        let mut setup = Setup::new(8101);
        let reasons = setup.reasons(3).await;

        let (_, result) = setup
            .create(
                "submittedCharter",
                &[
                    ("reasons", identifier_list(&reasons)),
                    ("title", "a charter".into()),
                ],
            )
            .await;
        assert_successful(&result, "every referenced reason exists");
    }

    #[tokio::test]
    async fn should_refuse_a_list_whose_third_element_is_missing_naming_that_element() {
        let mut setup = Setup::new(8102);
        let reasons = setup.reasons(2).await;
        let missing = Identifier::random_with_rng(&mut setup.rng);

        let (_, result) = setup
            .create(
                "submittedCharter",
                &[(
                    "reasons",
                    identifier_list(&[reasons[0], reasons[1], missing]),
                )],
            )
            .await;
        assert_element_not_found(
            &result,
            "reasons[2]",
            missing,
            "the third reason does not exist",
        );
    }

    #[tokio::test]
    async fn should_accept_an_empty_list() {
        let mut setup = Setup::new(8103);

        let (_, result) = setup
            .create("submittedCharter", &[("reasons", identifier_list(&[]))])
            .await;
        assert_successful(&result, "an empty list checks nothing");
    }

    /// A writer gate on the elements holds for every element: the writer
    /// must own each referenced reason.
    #[tokio::test]
    async fn should_refuse_an_element_writer_gate_when_the_writer_does_not_own_a_referenced_document(
    ) {
        let platform_version = PlatformVersion::latest();
        let mut setup = Setup::new(8104);
        let own_reason = setup.reason("dash").await;
        let (bob, bob_signer, bob_key) =
            setup_identity(&mut setup.platform, 450, dash_to_credits!(1.0));
        let (bobs_reason, result) = create_document(
            &setup.platform,
            &setup.platform_state,
            &setup.contract,
            "reason",
            &[("topic", "dash".into())],
            bob.id(),
            &bob_key,
            2,
            &bob_signer,
            &mut setup.rng,
            platform_version,
        )
        .await;
        assert_successful(&result, "bob's reason is created");

        let (_, result) = setup
            .create(
                "gatedCharter",
                &[("reasons", identifier_list(&[own_reason]))],
            )
            .await;
        assert_successful(&result, "the writer owns every referenced reason");

        let (_, result) = setup
            .create(
                "gatedCharter",
                &[("reasons", identifier_list(&[own_reason, bobs_reason.id()]))],
            )
            .await;
        let [StateTransitionExecutionResult::PaidConsensusError { error, .. }] =
            result.execution_results().as_slice()
        else {
            panic!("expected one paid consensus error, got {result:?}");
        };
        assert_eq!(error.code(), 40127, "{error}");
        assert_matches!(
            error,
            ConsensusError::StateError(StateError::ReferencedDocumentPropertyMismatchError(
                mismatch
            )) if mismatch.path() == "reasons[1]"
                && mismatch.referring_property() == "$ownerId"
                && mismatch.referenced_property() == "$ownerId",
            "the second reason is bob's"
        );
    }

    /// The referring side of an element agreement is a property of the
    /// charter, the same for every element; the referenced side is that
    /// element's reason's property.
    #[tokio::test]
    async fn should_hold_an_element_agreement_between_the_document_and_every_referenced_document() {
        let mut setup = Setup::new(8105);
        let dash = setup.reason("dash").await;
        let other_dash = setup.reason("dash").await;
        let btc = setup.reason("btc").await;

        let (mut charter, result) = setup
            .create(
                "topicCharter",
                &[
                    ("reasons", identifier_list(&[dash, other_dash])),
                    ("topic", "dash".into()),
                ],
            )
            .await;
        assert_successful(&result, "every reason is about dash");

        let (_, result) = setup
            .create(
                "topicCharter",
                &[
                    ("reasons", identifier_list(&[dash, btc])),
                    ("topic", "dash".into()),
                ],
            )
            .await;
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(
                    StateError::ReferencedDocumentPropertyMismatchError(mismatch)
                ),
                ..
            }] if mismatch.path() == "reasons[1]",
            "the second reason is about btc"
        );

        // Changing only the bound property re-validates every element
        charter.set("topic", "btc".into());
        let result = setup.replace("topicCharter", &mut charter).await;
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(
                    StateError::ReferencedDocumentPropertyMismatchError(mismatch)
                ),
                ..
            }] if mismatch.path() == "reasons[0]",
            "the reasons stay about dash"
        );
    }

    #[tokio::test]
    async fn should_refuse_a_replace_that_adds_a_missing_element() {
        let mut setup = Setup::new(8106);
        let reasons = setup.reasons(2).await;
        let missing = Identifier::random_with_rng(&mut setup.rng);

        let (mut charter, result) = setup
            .create(
                "submittedCharter",
                &[("reasons", identifier_list(&reasons))],
            )
            .await;
        assert_successful(&result, "the charter is created");

        charter.set(
            "reasons",
            identifier_list(&[reasons[0], reasons[1], missing]),
        );
        let result = setup.replace("submittedCharter", &mut charter).await;
        assert_element_not_found(
            &result,
            "reasons[2]",
            missing,
            "the added reason does not exist",
        );

        let another = setup.reason("dash").await;
        charter.set(
            "reasons",
            identifier_list(&[reasons[0], reasons[1], another]),
        );
        let result = setup.replace("submittedCharter", &mut charter).await;
        assert_successful(&result, "adding a reason that exists is fine");
    }

    /// Every replace re-validates a list of deletableDocument references,
    /// touched or not, as it does a single one: a dead element has to be
    /// dropped or repointed before the document can be replaced.
    #[tokio::test]
    async fn should_refuse_an_unrelated_replace_while_a_deletable_element_is_dead() {
        let mut setup = Setup::new(8107);
        let (first, result) = setup.create("draft", &[("topic", "dash".into())]).await;
        assert_successful(&result, "the first draft is created");
        let (second, result) = setup.create("draft", &[("topic", "dash".into())]).await;
        assert_successful(&result, "the second draft is created");

        let (mut list, result) = setup
            .create(
                "draftList",
                &[
                    ("drafts", identifier_list(&[first.id(), second.id()])),
                    ("body", "first".into()),
                ],
            )
            .await;
        assert_successful(&result, "the list is created");

        list.set("body", "second".into());
        let result = setup.replace("draftList", &mut list).await;
        assert_successful(&result, "a replace passes while both drafts exist");

        let result = setup.delete("draft", &second).await;
        assert_successful(&result, "a referenced draft can still be deleted");

        // Only `body` changes, but every element is re-validated anyway
        list.set("body", "third".into());
        let result = setup.replace("draftList", &mut list).await;
        assert_element_not_found(
            &result,
            "drafts[1]",
            second.id(),
            "a replace may not leave an element on the deleted draft",
        );

        list.set("drafts", identifier_list(&[first.id()]));
        let result = setup.replace("draftList", &mut list).await;
        assert_successful(&result, "dropping the dead element repairs the list");
    }

    /// The processing fee of creating a `type_name` document whose list holds
    /// the first `count` of four reasons, on a fresh platform with the same
    /// history every time, so that only the list differs between two runs.
    async fn creation_fee(type_name: &str, count: usize) -> u64 {
        let mut setup = Setup::new(8108);
        let reasons = setup.reasons(4).await;
        let (_, result) = setup
            .create(
                type_name,
                &[("reasons", identifier_list(&reasons[..count]))],
            )
            .await;
        assert_successful(&result, "the list is created");
        result.aggregated_fees().processing_fee
    }

    /// Each element is a billed read: what a list of references costs over
    /// the same list without `refersTo` grows with the element count.
    #[tokio::test]
    async fn should_charge_more_for_more_referenced_elements() {
        let mut surcharges = Vec::new();
        for count in [0, 1, 2, 4] {
            let referenced = creation_fee("submittedCharter", count).await;
            let plain = creation_fee("plainCharter", count).await;
            surcharges.push(i128::from(referenced) - i128::from(plain));
        }
        // With no element there is nothing to read: what is left is the
        // two document types' own difference
        assert!(
            surcharges.windows(2).all(|pair| pair[0] < pair[1]),
            "the surcharge grows with every element: {surcharges:?}"
        );
    }
}
