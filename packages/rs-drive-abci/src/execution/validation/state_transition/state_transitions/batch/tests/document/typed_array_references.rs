//! `refersTo` on the elements of a typed array through the full ABCI
//! pipeline. The fixture's `submittedCharter.reasons` is a list of
//! `permanentDocument` references to `reason`, `gatedCharter.reasons` binds
//! the WRITER to every referenced reason's owner, `topicCharter.reasons`
//! binds the charter's `topic` to every reason's, `draftList.drafts` is a
//! list of `deletableDocument` references to `draft`, `nestedCharter` holds
//! its list inside the `team` object, `foreignCharter.reasons` refers to the
//! `note` documents of the permanent-document foreign fixture contract,
//! `contractList.contracts` to contracts the writer owns, and
//! `repeatCharter.reasons` may repeat an element. `plainCharter` and
//! `repeatPlainCh` have the shapes of `submittedCharter` and
//! `repeatCharter` without `refersTo`, as fee baselines.
//!
//! Each element is checked as a single reference is, in list order, and the
//! first one that fails refuses the write with the single reference's
//! error, naming the element by its list path.

use super::*;

mod typed_array_reference_tests {
    use super::super::reference_test_setup::{
        assert_successful, create_document, register_contract_at, ReferenceTestSetup as Setup,
    };
    use super::*;
    use crate::platform_types::state_transitions_processing_result::StateTransitionsProcessingResult;
    use dpp::consensus::codes::ErrorWithCode;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::identifier::Identifier;
    use dpp::prelude::DataContract;

    /// Shared with the contract-create registration test, which pins that
    /// the declarations themselves register.
    const CONTRACT_PATH: &str = "tests/supporting_files/contract/reference-validation/reference-validation-contract-typed-array-elements.json";

    /// The contract `foreignCharter` refers into, applied with an owner
    /// other than the writer's. It declares an empty `indices` list, which
    /// only a non-validating load admits, as its own registration test loads
    /// it.
    const FOREIGN_CONTRACT_PATH: &str = "tests/supporting_files/contract/reference-validation/reference-validation-contract-permanent-doc-foreign.json";

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

    impl Setup {
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

        /// The foreign fixture contract, owned by an identity other than
        /// the writer, with `count` notes the writer creates in it.
        async fn foreign_notes(&mut self, count: usize) -> (DataContract, Vec<Identifier>) {
            let platform_version = PlatformVersion::latest();
            let foreign = register_contract_at(
                &self.platform,
                FOREIGN_CONTRACT_PATH,
                Identifier::new([7; 32]),
                false,
                platform_version,
            );
            let mut notes = Vec::with_capacity(count);
            // Identity contract nonces count per contract
            for nonce in 2..2 + count as u64 {
                let (note, result) = create_document(
                    &self.platform,
                    &self.platform_state,
                    &foreign,
                    "note",
                    &[("content", "dash".into())],
                    self.owner,
                    &self.key,
                    nonce,
                    &self.signer,
                    &mut self.rng,
                    platform_version,
                )
                .await;
                assert_successful(&result, "the foreign note is created");
                notes.push(note.id());
            }
            (foreign, notes)
        }
    }

    #[tokio::test]
    async fn should_accept_a_list_whose_every_element_exists() {
        let mut setup = Setup::new(CONTRACT_PATH, 8101);
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
        let mut setup = Setup::new(CONTRACT_PATH, 8102);
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
        let mut setup = Setup::new(CONTRACT_PATH, 8103);

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
        let mut setup = Setup::new(CONTRACT_PATH, 8104);
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
        let mut setup = Setup::new(CONTRACT_PATH, 8105);
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
        let mut setup = Setup::new(CONTRACT_PATH, 8106);
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
        let mut setup = Setup::new(CONTRACT_PATH, 8107);
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

    /// The processing fee of creating a `type_name` document whose `reasons`
    /// list holds `pick` of four own reasons, or of four foreign notes, on a
    /// fresh platform with the same history every time (both sets are always
    /// created), so that only the list differs between two runs.
    async fn creation_fee(type_name: &str, pick: &[usize], foreign: bool) -> i128 {
        let mut setup = Setup::new(CONTRACT_PATH, 8108);
        let reasons = setup.reasons(4).await;
        let (_, notes) = setup.foreign_notes(4).await;
        let source = if foreign { &notes } else { &reasons };
        let list: Vec<Identifier> = pick.iter().map(|index| source[*index]).collect();
        let (_, result) = setup
            .create(type_name, &[("reasons", identifier_list(&list))])
            .await;
        assert_successful(&result, "the list is created");
        i128::from(result.aggregated_fees().processing_fee)
    }

    /// Each element is a billed read: what a list of references costs over
    /// the same list without `refersTo` grows with the element count.
    #[tokio::test]
    async fn should_charge_more_for_more_referenced_elements() {
        let mut surcharges = Vec::new();
        let picks: [&[usize]; 4] = [&[], &[0], &[0, 1], &[0, 1, 2, 3]];
        for pick in picks {
            let referenced = creation_fee("submittedCharter", pick, false).await;
            let plain = creation_fee("plainCharter", pick, false).await;
            surcharges.push(referenced - plain);
        }
        // With no element there is nothing to read: what is left is the
        // two document types' own difference
        assert!(
            surcharges.windows(2).all(|pair| pair[0] < pair[1]),
            "the surcharge grows with every element: {surcharges:?}"
        );
    }

    /// The elements of one list share their declaration, so a foreign
    /// contract holding the referenced document type is fetched, and billed,
    /// once per list: three foreign elements cost one contract fetch over
    /// three own ones, as one does over one.
    #[tokio::test]
    async fn should_fetch_a_foreign_contract_once_per_list() {
        let one = creation_fee("foreignCharter", &[0], true).await
            - creation_fee("submittedCharter", &[0], false).await;
        let three = creation_fee("foreignCharter", &[0, 1, 2], true).await
            - creation_fee("submittedCharter", &[0, 1, 2], false).await;

        let platform_version = PlatformVersion::latest();
        let setup = Setup::new(CONTRACT_PATH, 8109);
        let foreign = register_contract_at(
            &setup.platform,
            FOREIGN_CONTRACT_PATH,
            Identifier::new([7; 32]),
            false,
            platform_version,
        );
        let (fee, _) = setup
            .platform
            .drive
            .get_contract_with_fetch_info_and_fee(
                foreign.id().to_buffer(),
                Some(&BlockInfo::default().epoch),
                false,
                None,
                platform_version,
            )
            .expect("expected to fetch the foreign contract");
        let contract_fetch = i128::from(
            fee.expect("a fetch with an epoch carries its fee")
                .processing_fee,
        );

        assert!(
            (three - one).abs() < contract_fetch,
            "two more foreign elements cost {} over their own counterparts, less than one \
             contract fetch ({contract_fetch})",
            three - one
        );
    }

    /// An element repeating an earlier one of the same list has that one's
    /// outcome, so it is not fetched, or billed, again.
    #[tokio::test]
    async fn should_fetch_a_repeated_element_once() {
        let mut surcharges = Vec::new();
        let picks: [&[usize]; 3] = [&[], &[0], &[0, 0]];
        for pick in picks {
            surcharges.push(
                creation_fee("repeatCharter", pick, false).await
                    - creation_fee("repeatPlainCh", pick, false).await,
            );
        }
        let one_fetch = surcharges[1] - surcharges[0];
        let second_copy = surcharges[2] - surcharges[1];
        assert!(
            second_copy < one_fetch / 2,
            "the repeated element costs {second_copy}, a fetch costs {one_fetch}"
        );
    }

    /// A replace that only adds an element checks the new one: the elements
    /// the stored list already held are unchanged references, and an
    /// unchanged single reference is not re-validated either, so a reason
    /// that stopped agreeing since it was written does not block the list.
    #[tokio::test]
    async fn should_leave_the_elements_the_stored_list_held_alone_when_a_replace_adds_one() {
        let mut setup = Setup::new(CONTRACT_PATH, 8110);
        let (mut drifting, result) = setup.create("reason", &[("topic", "dash".into())]).await;
        assert_successful(&result, "the first reason is created");
        let (mut charter, result) = setup
            .create(
                "topicCharter",
                &[
                    ("reasons", identifier_list(&[drifting.id()])),
                    ("topic", "dash".into()),
                ],
            )
            .await;
        assert_successful(&result, "the charter is created");

        drifting.set("topic", "btc".into());
        let result = setup.replace("reason", &mut drifting).await;
        assert_successful(&result, "the reason's owner changes its topic");

        let added = setup.reason("dash").await;
        charter.set("reasons", identifier_list(&[drifting.id(), added]));
        let result = setup.replace("topicCharter", &mut charter).await;
        assert_successful(&result, "only the added reason is checked, and it agrees");

        // A missing addition is still refused, named at its new position
        let missing = Identifier::random_with_rng(&mut setup.rng);
        charter.set("reasons", identifier_list(&[missing, drifting.id(), added]));
        let result = setup.replace("topicCharter", &mut charter).await;
        assert_element_not_found(&result, "reasons[0]", missing, "the new element is checked");
    }

    /// A list inside an object is named by its dotted path, and replacing
    /// the object re-validates it.
    #[tokio::test]
    async fn should_name_an_element_of_a_list_nested_in_an_object() {
        let mut setup = Setup::new(CONTRACT_PATH, 8111);
        let reasons = setup.reasons(2).await;
        let missing = Identifier::random_with_rng(&mut setup.rng);
        let team = |ids: &[Identifier]| {
            Value::Map(vec![(
                Value::Text("members".to_string()),
                identifier_list(ids),
            )])
        };

        let (_, result) = setup
            .create("nestedCharter", &[("team", team(&[reasons[0], missing]))])
            .await;
        assert_element_not_found(
            &result,
            "team.members[1]",
            missing,
            "the nested list's second member does not exist",
        );

        let (mut charter, result) = setup
            .create("nestedCharter", &[("team", team(&reasons))])
            .await;
        assert_successful(&result, "every member exists");
        charter.set("team", team(&[reasons[0], reasons[1], missing]));
        let result = setup.replace("nestedCharter", &mut charter).await;
        assert_element_not_found(
            &result,
            "team.members[2]",
            missing,
            "replacing the object re-validates the list inside it",
        );
    }

    /// A contract element's `contractRequirements` hold for every element
    /// (40135), named at the element.
    #[tokio::test]
    async fn should_check_contract_requirements_on_every_element() {
        let mut setup = Setup::new(CONTRACT_PATH, 8112);
        let (foreign, _) = setup.foreign_notes(0).await;
        let own = setup.contract.id();

        let (_, result) = setup
            .create("contractList", &[("contracts", identifier_list(&[own]))])
            .await;
        assert_successful(&result, "the writer owns the contract");

        let (_, result) = setup
            .create(
                "contractList",
                &[("contracts", identifier_list(&[own, foreign.id()]))],
            )
            .await;
        let [StateTransitionExecutionResult::PaidConsensusError { error, .. }] =
            result.execution_results().as_slice()
        else {
            panic!("expected one paid consensus error, got {result:?}");
        };
        assert_eq!(error.code(), 40135, "{error}");
        assert_matches!(
            error,
            ConsensusError::StateError(StateError::ReferencedContractRequirementNotMetError(
                unmet
            )) if unmet.path() == "contracts[1]",
            "the second contract is owned by someone else"
        );
    }
}
