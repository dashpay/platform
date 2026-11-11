//! `where` entries over typed arrays through the full ABCI pipeline (protocol
//! version 14). The fixture's `post` holds `tags`, `members` and `scores`; a
//! `like` echoes its post's `tags` (an equality between two typed arrays),
//! a `copy` its `members` and `scores`, a `tagged` names one of its tags
//! (`contains`), a `memberNote` is written only by one of its members
//! (`contains` of the writer), and a `subset` names some of its tags
//! (`containsAll`). Lists are compared element by element, by value, and a
//! list a document leaves out is the empty one.

use super::*;

mod typed_array_where_tests {
    use super::super::reference_test_setup::{
        assert_successful, create_transition, process_and_commit, ReferenceTestSetup as Setup,
    };
    use super::*;
    use crate::platform_types::state_transitions_processing_result::StateTransitionsProcessingResult;
    use dpp::identifier::Identifier;
    use dpp::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
    use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
    use dpp::state_transition::batch_transition::batched_transition::BatchedTransitionRef;
    use dpp::state_transition::StateTransition;
    use drive::state_transition_action::batch::batched_transition::document_transition::document_create_transition_action::{
        DocumentCreateTransitionAction, DocumentCreateTransitionActionAccessorsV0,
    };
    use drive::state_transition_action::batch::batched_transition::document_transition::DocumentTransitionAction;
    use drive::state_transition_action::batch::batched_transition::BatchedTransitionAction;
    use crate::execution::types::execution_operation::ValidationOperation;
    use crate::execution::types::state_transition_execution_context::{
        StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
    };
    use crate::execution::validation::state_transition::batch::action_validation::document::document_reference_validation::DocumentReferenceValidation;
    use crate::platform_types::platform::PlatformStateRef;
    use dpp::data_contract::document_type::property_constraints::DocumentSystemValues;
    use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
    use dpp::fee::fee_result::FeeResult;
    use dpp::validation::SimpleConsensusValidationResult;
    use dpp::version::DefaultForPlatformVersion;
    use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::{
        DocumentBaseTransitionAction, DocumentBaseTransitionActionV0,
    };
    use std::collections::BTreeMap;

    const CONTRACT_PATH: &str = "tests/supporting_files/contract/reference-validation/reference-validation-contract-where-typed-arrays.json";

    /// The write was refused because the post fails the `where` entry whose
    /// referring side is `referring`: a property for an equality, the
    /// condition as written for the others (code 40127).
    fn assert_mismatch(result: &StateTransitionsProcessingResult, referring: &str, because: &str) {
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(
                    StateError::ReferencedDocumentPropertyMismatchError(mismatch)
                ),
                ..
            }] if mismatch.referring_property() == referring,
            "{because}: got {result:?}"
        );
    }

    fn texts(texts: &[&str]) -> Value {
        Value::Array(
            texts
                .iter()
                .map(|text| Value::Text(text.to_string()))
                .collect(),
        )
    }

    impl Setup {
        /// A post holding exactly `properties`, created.
        async fn post(&mut self, properties: &[(&str, Value)]) -> Identifier {
            let (post, result) = self.create("post", properties).await;
            assert_successful(&result, "the post is created");
            post.id()
        }

        /// A document of `type_name` naming the post `post_id`, beside
        /// `properties`.
        async fn create_on(
            &mut self,
            type_name: &str,
            post_id: Identifier,
            properties: &[(&str, Value)],
        ) -> StateTransitionsProcessingResult {
            let mut all = vec![("postId", Value::Identifier(post_id.to_buffer()))];
            all.extend(properties.iter().cloned());
            self.create(type_name, &all).await.1
        }
    }

    /// A like echoes its post's tags exactly: the same tags in the same
    /// order. Reordered, fewer, more or none differ.
    #[tokio::test]
    async fn should_agree_on_a_list_with_the_same_elements_in_the_same_order() {
        let mut setup = Setup::new(CONTRACT_PATH, 6101);
        let post = setup.post(&[("tags", texts(&["dash", "news"]))]).await;

        let result = setup
            .create_on("like", post, &[("tags", texts(&["dash", "news"]))])
            .await;
        assert_successful(&result, "the same tags in the same order agree");

        for (tags, because) in [
            (
                Some(texts(&["news", "dash"])),
                "the order of the elements counts",
            ),
            (Some(texts(&["dash"])), "a missing element differs"),
            (
                Some(texts(&["dash", "news", "tech"])),
                "an extra element differs",
            ),
            (None, "a list left out is empty, and the post's is not"),
        ] {
            let properties: Vec<(&str, Value)> =
                tags.into_iter().map(|tags| ("tags", tags)).collect();
            let result = setup.create_on("like", post, &properties).await;
            assert_mismatch(&result, "tags", because);
        }
    }

    /// A list left out is the empty one: a like leaving out its tags agrees
    /// with a post that has none, and so does an empty list, either way round.
    #[tokio::test]
    async fn should_agree_on_lists_left_out_or_empty() {
        let mut setup = Setup::new(CONTRACT_PATH, 6102);
        let untagged = setup.post(&[("label", "plain".into())]).await;
        let result = setup.create_on("like", untagged, &[]).await;
        assert_successful(&result, "two lists left out agree");
        let result = setup
            .create_on("like", untagged, &[("tags", texts(&[]))])
            .await;
        assert_successful(&result, "an empty list agrees with one left out");

        let empty = setup.post(&[("tags", texts(&[]))]).await;
        let result = setup.create_on("like", empty, &[]).await;
        assert_successful(&result, "a list left out agrees with an empty one");

        let result = setup
            .create_on("like", untagged, &[("tags", texts(&["dash"]))])
            .await;
        assert_mismatch(&result, "tags", "a tag the post does not have differs");
    }

    /// Elements compare by value, not by how a transition carries them: an
    /// identifier equals its 32 bytes, and an integer is the same at any width.
    #[tokio::test]
    async fn should_compare_list_elements_by_value() {
        let mut setup = Setup::new(CONTRACT_PATH, 6103);
        let alice = Identifier::new([1; 32]);
        let bob = Identifier::new([2; 32]);
        let post = setup
            .post(&[
                (
                    "members",
                    Value::Array(vec![
                        Value::Identifier(alice.to_buffer()),
                        Value::Identifier(bob.to_buffer()),
                    ]),
                ),
                ("scores", Value::Array(vec![Value::U16(3), Value::U16(700)])),
            ])
            .await;

        let members = Value::Array(vec![
            Value::Bytes(alice.to_buffer().to_vec()),
            Value::Bytes(bob.to_buffer().to_vec()),
        ]);
        let scores = Value::Array(vec![Value::U64(3), Value::U64(700)]);
        let nonce = setup.next_nonce();
        let (_, create) = create_transition(
            &setup.contract,
            "copy",
            &[
                ("postId", Value::Identifier(post.to_buffer())),
                ("members", members.clone()),
                ("scores", scores.clone()),
            ],
            setup.owner,
            &setup.key,
            nonce,
            &setup.signer,
            &mut setup.rng,
            PlatformVersion::latest(),
        )
        .await;
        // The node decodes the transition and checks the references on its
        // action's data, which still holds the elements as they were carried
        let data = create_action_data(&setup, &create);
        assert_eq!(data.get("members"), Some(&members));
        assert_eq!(data.get("scores"), Some(&scores));
        let result = process_and_commit(
            &setup.platform,
            &setup.platform_state,
            &create,
            PlatformVersion::latest(),
        );
        assert_successful(&result, "equal values agree however they are carried");

        let result = setup
            .create_on(
                "copy",
                post,
                &[
                    (
                        "members",
                        Value::Array(vec![
                            Value::Identifier(alice.to_buffer()),
                            Value::Identifier(bob.to_buffer()),
                        ]),
                    ),
                    ("scores", Value::Array(vec![Value::U64(700), Value::U64(3)])),
                ],
            )
            .await;
        assert_mismatch(&result, "scores", "the order of the scores counts");
    }

    /// The data of the create action the node builds from `create` once
    /// decoded from its bytes as a block's transition is, which the reference
    /// check reads.
    fn create_action_data(setup: &Setup, create: &StateTransition) -> BTreeMap<String, Value> {
        let bytes = create
            .serialize_to_bytes()
            .expect("expected the transition to serialize");
        let StateTransition::Batch(batch) =
            StateTransition::deserialize_from_bytes_untrusted_exact_in_version(
                &bytes,
                PlatformVersion::latest(),
            )
            .expect("expected it to decode as the node does")
        else {
            panic!("expected a batch transition");
        };
        let Some(BatchedTransitionRef::Document(DocumentTransition::Create(create))) =
            batch.first_transition()
        else {
            panic!("expected a document create");
        };
        let (_, contract_fetch_info) = setup
            .platform
            .drive
            .get_contract_with_fetch_info_and_fee(
                setup.contract.id().to_buffer(),
                None,
                false,
                None,
                PlatformVersion::latest(),
            )
            .expect("expected to fetch the contract");
        let contract_fetch_info = contract_fetch_info.expect("the contract is in state");
        let (result, _) =
            DocumentCreateTransitionAction::try_from_document_borrowed_create_transition_with_contract_lookup(
                &setup.platform.drive,
                setup.owner,
                None,
                create,
                &BlockInfo::default(),
                0,
                |_| Ok(contract_fetch_info.clone()),
                PlatformVersion::latest(),
            )
            .expect("expected the transformer to run");
        match result.into_data().expect("expected an action") {
            BatchedTransitionAction::DocumentAction(DocumentTransitionAction::CreateAction(
                action,
            )) => action.data().clone(),
            other => panic!("expected a create action, got {other:?}"),
        }
    }

    /// Validates the references of a document of `type_name` holding `data`,
    /// written by the setup's identity, under `platform_version`, as a create's
    /// state validation does, and returns the result with what it billed.
    fn validate_references(
        setup: &Setup,
        type_name: &str,
        data: &[(&str, Value)],
        platform_version: &PlatformVersion,
    ) -> (
        SimpleConsensusValidationResult,
        StateTransitionExecutionContext,
    ) {
        let (_, contract_fetch_info) = setup
            .platform
            .drive
            .get_contract_with_fetch_info_and_fee(
                setup.contract.id().to_buffer(),
                None,
                false,
                None,
                platform_version,
            )
            .expect("expected to fetch the contract");
        let base = DocumentBaseTransitionAction::V0(DocumentBaseTransitionActionV0 {
            id: Identifier::from([0xAA; 32]),
            identity_contract_nonce: 1,
            document_type_name: type_name.to_string(),
            data_contract: contract_fetch_info.expect("the contract is in state"),
            token_cost: None,
            shielded_token_payment: None,
            gas_fees_paid_by: GasFeesPaidBy::default(),
            contract_gas_fees_paid_by: GasFeesPaidBy::default(),
            declared_action_fee: None,
        });
        let platform_state = setup.platform.state.load();
        let platform_ref = PlatformStateRef {
            drive: &setup.platform.drive,
            state: &platform_state,
            config: &setup.platform.config,
        };
        let data: BTreeMap<String, Value> = data
            .iter()
            .map(|(property, value)| (property.to_string(), value.clone()))
            .collect();
        let mut execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)
                .expect("expected an execution context");
        let result = base
            .validate_document_references(
                &data,
                setup.owner,
                Some(setup.owner),
                &DocumentSystemValues::default(),
                None,
                None,
                &platform_ref,
                &BlockInfo::default(),
                &mut Vec::new(),
                None,
                &mut BTreeMap::new(),
                None,
                &mut execution_context,
                platform_version,
            )
            .expect("expected the references to be validated");
        (result, execution_context)
    }

    /// The processing fee of everything the validation billed.
    fn processing_fee(execution_context: &StateTransitionExecutionContext) -> u64 {
        let mut fee = FeeResult::default();
        ValidationOperation::add_many_to_fee_result(
            execution_context.operations_slice(),
            &mut fee,
            PlatformVersion::latest(),
        )
        .expect("expected the operations to sum");
        fee.processing_fee
    }

    /// Every list element an entry over lists reads is billed before it is
    /// compared, on both sides, the refused write too: an equality reads both
    /// lists, a `containsAll` the post's and the document's, a `contains` the
    /// post's.
    #[tokio::test]
    async fn should_bill_every_list_element_a_where_entry_reads() {
        let mut setup = Setup::new(CONTRACT_PATH, 6108);
        let post = setup
            .post(&[("tags", texts(&["dash", "news", "tech"]))])
            .await;
        let post_id = ("postId", Value::Identifier(post.to_buffer()));
        let per_element = |credits: u64| {
            let mut platform_version = PlatformVersion::latest().clone();
            platform_version
                .fee_version
                .document_reference
                .where_list_credits_per_element = credits;
            platform_version
        };
        let (free, seven) = (per_element(0), per_element(7));

        for (type_name, data, holds, elements) in [
            ("like", ("tags", texts(&["dash", "news", "tech"])), true, 6),
            ("subset", ("tags", texts(&["news", "dash"])), true, 5),
            ("subset", ("tags", texts(&["sport"])), false, 4),
            ("tagged", ("tag", Value::Text("tech".to_string())), true, 3),
        ] {
            let data = [post_id.clone(), data];
            let (result, billed) = validate_references(&setup, type_name, &data, &seven);
            assert_eq!(result.is_valid(), holds, "{type_name}: {result:?}");
            let (_, unbilled) = validate_references(&setup, type_name, &data, &free);
            assert_eq!(
                processing_fee(&billed) - processing_fee(&unbilled),
                7 * elements,
                "{type_name} bills {elements} elements"
            );
        }
    }

    /// An integer list may be carried as whole floats, which compare with the
    /// post's integers as a scalar equality compares them: 1.0 is 1, `-0.0` is
    /// not 0. A `containsAll` of many repeated floats looks each up, billed by
    /// the elements read.
    #[tokio::test]
    async fn should_compare_floats_in_an_integer_list_as_scalars() {
        let mut setup = Setup::new(CONTRACT_PATH, 6109);
        let post = setup
            .post(&[("scores", Value::Array(vec![Value::U16(0), Value::U16(1)]))])
            .await;
        let post_id = ("postId", Value::Identifier(post.to_buffer()));

        let ones = Value::Array(vec![Value::Float(1.0); 1024]);
        let (result, _) = validate_references(
            &setup,
            "scoreSubset",
            &[post_id.clone(), ("scores", ones)],
            PlatformVersion::latest(),
        );
        assert!(result.is_valid(), "1.0 is the integer 1: {result:?}");

        let negative_zero = Value::Array(vec![Value::Float(-0.0)]);
        let (result, _) = validate_references(
            &setup,
            "scoreSubset",
            &[post_id, ("scores", negative_zero)],
            PlatformVersion::latest(),
        );
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::StateError(StateError::ReferencedDocumentPropertyMismatchError(e))]
                if e.referring_property() == "{ \"containsAll\": \"scores\" }",
            "-0.0 is not the integer 0"
        );
    }

    /// A `contains` holds when the post's list holds the referring value; a
    /// value left out is in no list, and a list left out holds nothing.
    #[tokio::test]
    async fn should_hold_contains_for_a_listed_value_only() {
        let mut setup = Setup::new(CONTRACT_PATH, 6104);
        let post = setup.post(&[("tags", texts(&["dash", "news"]))]).await;
        let condition = "{ \"contains\": \"tag\" }";

        let result = setup
            .create_on("tagged", post, &[("tag", "news".into())])
            .await;
        assert_successful(&result, "a listed tag is contained");

        let result = setup
            .create_on("tagged", post, &[("tag", "tech".into())])
            .await;
        assert_mismatch(&result, condition, "an unlisted tag is not contained");

        let result = setup.create_on("tagged", post, &[]).await;
        assert_mismatch(&result, condition, "a tag left out is in no list");

        let untagged = setup.post(&[("label", "plain".into())]).await;
        let result = setup
            .create_on("tagged", untagged, &[("tag", "dash".into())])
            .await;
        assert_mismatch(&result, condition, "a list left out holds nothing");
    }

    /// `contains` of `"$ownerId"` is a writer gate: only a listed member
    /// writes a note on the post.
    #[tokio::test]
    async fn should_let_only_a_listed_member_write() {
        let mut setup = Setup::new(CONTRACT_PATH, 6105);
        let writer = setup.owner;
        let condition = "{ \"contains\": \"$ownerId\" }";
        let with_writer = setup
            .post(&[(
                "members",
                Value::Array(vec![
                    Value::Identifier([9; 32]),
                    Value::Identifier(writer.to_buffer()),
                ]),
            )])
            .await;
        let result = setup
            .create_on("memberNote", with_writer, &[("text", "hi".into())])
            .await;
        assert_successful(&result, "a listed member writes");

        let without_writer = setup
            .post(&[("members", Value::Array(vec![Value::Identifier([9; 32])]))])
            .await;
        let result = setup
            .create_on("memberNote", without_writer, &[("text", "hi".into())])
            .await;
        assert_mismatch(&result, condition, "an unlisted writer is refused");

        let no_members = setup.post(&[("label", "plain".into())]).await;
        let result = setup
            .create_on("memberNote", no_members, &[("text", "hi".into())])
            .await;
        assert_mismatch(&result, condition, "a post without members admits no one");
    }

    /// A `containsAll` holds when the post's list holds every element of the
    /// referring list, in any order; every list holds all of an empty one or
    /// of one left out.
    #[tokio::test]
    async fn should_hold_contains_all_for_every_element_in_any_order() {
        let mut setup = Setup::new(CONTRACT_PATH, 6106);
        let post = setup
            .post(&[("tags", texts(&["dash", "news", "tech"]))])
            .await;
        let condition = "{ \"containsAll\": \"tags\" }";

        for (tags, because) in [
            (
                Some(texts(&["tech", "dash"])),
                "a subset in another order is held",
            ),
            (Some(texts(&["news", "news"])), "a repeated element is held"),
            (Some(texts(&[])), "every list holds all of an empty one"),
            (None, "a list left out is empty"),
        ] {
            let properties: Vec<(&str, Value)> =
                tags.into_iter().map(|tags| ("tags", tags)).collect();
            let result = setup.create_on("subset", post, &properties).await;
            assert_successful(&result, because);
        }

        let result = setup
            .create_on("subset", post, &[("tags", texts(&["dash", "sport"]))])
            .await;
        assert_mismatch(&result, condition, "an element the post lacks is not held");

        let untagged = setup.post(&[("label", "plain".into())]).await;
        let result = setup.create_on("subset", untagged, &[]).await;
        assert_successful(&result, "a list left out holds all of another left out");
        let result = setup
            .create_on("subset", untagged, &[("tags", texts(&["dash"]))])
            .await;
        assert_mismatch(&result, condition, "a list left out holds no element");
    }

    /// A replace changing the like's tags asks about the pair again, and the
    /// changed list must still be its post's.
    #[tokio::test]
    async fn should_check_a_changed_list_again_on_a_replace() {
        let mut setup = Setup::new(CONTRACT_PATH, 6107);
        let post = setup.post(&[("tags", texts(&["dash", "news"]))]).await;
        let (mut like, result) = setup
            .create(
                "like",
                &[
                    ("postId", Value::Identifier(post.to_buffer())),
                    ("tags", texts(&["dash", "news"])),
                ],
            )
            .await;
        assert_successful(&result, "the like is created");

        like.set("note", "first".into());
        let result = setup.replace("like", &mut like).await;
        assert_successful(&result, "a replace keeping the tags agrees");

        like.set("tags", texts(&["news", "dash"]));
        let result = setup.replace("like", &mut like).await;
        assert_mismatch(&result, "tags", "reordered tags no longer agree");
    }
}
