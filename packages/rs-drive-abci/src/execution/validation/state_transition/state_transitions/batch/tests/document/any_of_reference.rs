//! `anyOf` reference targets (`refersTo: { "anyOf": [target, ...] }`, protocol
//! version 14) through the full ABCI pipeline. The fixture's `resignation` must
//! name a member of the moderation charter it is for: `memberId` is the owner
//! of a `joinRequest` for the charter (the first target) or the `moderatorId`
//! of an `addedModerator` document for it (the second). `members` declares the
//! same on the elements of a typed array, `signerOrRequestId` an identity or a
//! join request's id, and `agreedMemberId` the two lookups with a
//! `propertyAgreement` on the first alone.
//!
//! The targets are checked in declared order and the first that holds ends the
//! check; every read is billed, the failed targets' included. When none holds,
//! the write is refused, paid, with the error the last target gives.

use super::*;

mod any_of_reference_tests {
    use super::*;
    use crate::execution::types::execution_operation::ValidationOperation;
    use crate::execution::types::state_transition_execution_context::{
        StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
    };
    use crate::execution::validation::state_transition::batch::action_validation::document::document_reference_validation::DocumentReferenceValidation;
    use crate::platform_types::platform::PlatformStateRef;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TempPlatform;
    use dpp::data_contract::document_type::DocumentPropertyReferenceTarget;
    use dpp::document::Document;
    use dpp::identifier::Identifier;
    use dpp::identity::{Identity, IdentityPublicKey};
    use dpp::prelude::{DataContract, IdentityNonce};
    use dpp::state_transition::StateTransition;
    use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
    use dpp::version::DefaultForPlatformVersion;
    use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::{DocumentBaseTransitionAction, DocumentBaseTransitionActionV0};
    use simple_signer::signer::SimpleSigner;
    use std::collections::BTreeMap;

    const CONTRACT_PATH: &str =
        "tests/supporting_files/contract/reference-validation/reference-validation-contract-any-of.json";

    /// The identities of the fixture: the `Founder` writes resignations and
    /// adds moderators, the `Member` asks to join, the `Moderator` is added,
    /// and the `Stranger` is neither.
    #[derive(Clone, Copy)]
    enum Who {
        Founder,
        Member,
        Moderator,
        Stranger,
    }

    struct Writer {
        identity: Identity,
        signer: SimpleSigner,
        key: IdentityPublicKey,
        /// The identity contract nonce the next transition uses; every
        /// processed transition consumes one, a refused one included.
        next_nonce: IdentityNonce,
    }

    struct AnyOfFixture {
        platform: TempPlatform<MockCoreRPCLike>,
        contract: DataContract,
        rng: StdRng,
        founder: Writer,
        member: Writer,
        moderator: Writer,
        stranger: Writer,
    }

    fn id_value(id: Identifier) -> Value {
        Value::Identifier(id.to_buffer())
    }

    fn charter_id(byte: u8) -> Identifier {
        Identifier::from([byte; 32])
    }

    impl AnyOfFixture {
        fn new() -> Self {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut writer = |seed| {
                let (identity, signer, key) =
                    setup_identity(&mut platform, seed, dash_to_credits!(0.5));
                Writer {
                    identity,
                    signer,
                    key,
                    next_nonce: 1,
                }
            };
            let founder = writer(1958);
            let member = writer(1959);
            let moderator = writer(1960);
            let stranger = writer(1961);

            // Parsed with full validation, so the contract-level checks of
            // every target run on the fixture too
            let contract = json_document_to_contract(CONTRACT_PATH, true, platform_version)
                .expect("expected to parse the contract");
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
                contract,
                rng: StdRng::seed_from_u64(4434),
                founder,
                member,
                moderator,
                stranger,
            }
        }

        fn writer(&mut self, who: Who) -> &mut Writer {
            match who {
                Who::Founder => &mut self.founder,
                Who::Member => &mut self.member,
                Who::Moderator => &mut self.moderator,
                Who::Stranger => &mut self.stranger,
            }
        }

        fn id(&mut self, who: Who) -> Identifier {
            self.writer(who).identity.id()
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

        /// Creates a `type_name` document owned by `who`, with `values` set
        /// over the random required ones, and returns it with the result.
        async fn create(
            &mut self,
            who: Who,
            type_name: &str,
            values: &[(&str, Value)],
        ) -> (Document, StateTransitionExecutionResult) {
            let platform_version = PlatformVersion::latest();
            let owner_id = self.id(who);
            let document_type = self
                .contract
                .document_type_for_name(type_name)
                .expect("expected the document type");
            let entropy = Bytes32::random_with_rng(&mut self.rng);
            let mut document = document_type
                .random_document_with_identifier_and_entropy(
                    &mut self.rng,
                    owner_id,
                    entropy,
                    DocumentFieldFillType::DoNotFillIfNotRequired,
                    DocumentFieldFillSize::AnyDocumentFillSize,
                    platform_version,
                )
                .expect("expected a random document");
            for (property, value) in values {
                document.set(property, value.clone());
            }
            let writer = match who {
                Who::Founder => &mut self.founder,
                Who::Member => &mut self.member,
                Who::Moderator => &mut self.moderator,
                Who::Stranger => &mut self.stranger,
            };
            let nonce = writer.next_nonce;
            writer.next_nonce += 1;
            document
                .set_id_for_creation(document_type, &entropy.0, nonce, platform_version)
                .expect("expected to set the document id");
            let transition = BatchTransition::new_document_creation_transition_from_document(
                document.clone(),
                document_type,
                entropy.0,
                &writer.key,
                nonce,
                0,
                None,
                &writer.signer,
                platform_version,
                None,
            )
            .await
            .expect("expected the create transition");
            let result = self.process(&transition);
            (document, result)
        }

        /// Replaces `document`, as last accepted, with `change` applied, as
        /// the founder, its owner.
        async fn replace(
            &mut self,
            document: &Document,
            change: impl FnOnce(&mut Document),
        ) -> StateTransitionExecutionResult {
            let platform_version = PlatformVersion::latest();
            let mut replacement = document.clone();
            replacement
                .increment_revision()
                .expect("the revision increments");
            change(&mut replacement);
            let document_type = self
                .contract
                .document_type_for_name("resignation")
                .expect("expected the document type");
            let writer = &mut self.founder;
            let nonce = writer.next_nonce;
            writer.next_nonce += 1;
            let transition = BatchTransition::new_document_replacement_transition_from_document(
                replacement,
                document_type,
                &writer.key,
                nonce,
                0,
                None,
                &writer.signer,
                platform_version,
                None,
            )
            .await
            .expect("expected the replace transition");
            self.process(&transition)
        }

        /// A join request by `who` for the submitted charter `charter_id`.
        async fn request_to_join(
            &mut self,
            who: Who,
            charter_id: Identifier,
            message: &str,
        ) -> Document {
            let (request, result) = self
                .create(
                    who,
                    "joinRequest",
                    &[
                        ("submittedCharterId", id_value(charter_id)),
                        ("message", message.into()),
                    ],
                )
                .await;
            assert_matches!(
                result,
                StateTransitionExecutionResult::SuccessfulExecution { .. }
            );
            request
        }

        /// The founder adds `moderator` to the submitted charter `charter_id`.
        async fn add_moderator(&mut self, charter_id: Identifier, moderator: Identifier) {
            let (_, result) = self
                .create(
                    Who::Founder,
                    "addedModerator",
                    &[
                        ("submittedCharterId", id_value(charter_id)),
                        ("moderatorId", id_value(moderator)),
                    ],
                )
                .await;
            assert_matches!(
                result,
                StateTransitionExecutionResult::SuccessfulExecution { .. }
            );
        }

        /// A resignation by the founder for `charter_id` titled `title`, with
        /// `values` set.
        async fn resign(
            &mut self,
            charter_id: Identifier,
            title: &str,
            values: &[(&str, Value)],
        ) -> (Document, StateTransitionExecutionResult) {
            let mut all = vec![
                ("submittedCharterId", id_value(charter_id)),
                ("title", title.into()),
            ];
            all.extend(values.iter().cloned());
            self.create(Who::Founder, "resignation", &all).await
        }
    }

    fn assert_successful(result: &StateTransitionExecutionResult) {
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. },
            "{result:?}"
        );
    }

    /// The refusal of a value no target holds for: the error the last target
    /// gives alone, a lookup into `last_document_type` that found nothing,
    /// naming the property (or element) and the value.
    fn assert_refused_by_the_last_target(
        result: StateTransitionExecutionResult,
        path: &str,
        value: Identifier,
        last_document_type: &str,
    ) {
        assert_matches!(
            result,
            PaidConsensusError {
                error: ConsensusError::StateError(StateError::ReferencedEntityNotFoundError(e)),
                ..
            } if e.path() == path
                && *e.entity_id() == value
                && matches!(
                    e.entity_type(),
                    DocumentPropertyReferenceTarget::PermanentDocumentLookup {
                        document_type_name,
                        ..
                    } if document_type_name == last_document_type
                ),
            "expected the last target's 40120 at {path}"
        );
    }

    #[tokio::test]
    async fn should_accept_a_value_the_first_target_holds_for() {
        let mut fixture = AnyOfFixture::new();
        let member = fixture.id(Who::Member);
        fixture
            .request_to_join(Who::Member, charter_id(1), "let me in")
            .await;

        let (_, result) = fixture
            .resign(
                charter_id(1),
                "resigning",
                &[("memberId", id_value(member))],
            )
            .await;

        assert_successful(&result);
    }

    #[tokio::test]
    async fn should_accept_a_value_only_the_second_target_holds_for() {
        let mut fixture = AnyOfFixture::new();
        let moderator = fixture.id(Who::Moderator);
        // The moderator never asked to join: only the addedModerator holds
        fixture.add_moderator(charter_id(1), moderator).await;

        let (_, result) = fixture
            .resign(
                charter_id(1),
                "resigning",
                &[("memberId", id_value(moderator))],
            )
            .await;

        assert_successful(&result);
    }

    /// Neither target holds: the write is refused, paid, with the error of
    /// the LAST target, a lookup into `addedModerator`, so the order the
    /// author declared decides which failure a writer is shown.
    #[tokio::test]
    async fn should_refuse_a_value_no_target_holds_for_with_the_last_targets_error() {
        let mut fixture = AnyOfFixture::new();
        let member = fixture.id(Who::Member);
        let stranger = fixture.id(Who::Stranger);
        fixture
            .request_to_join(Who::Member, charter_id(1), "let me in")
            .await;
        // A moderator of another charter is no moderator of this one
        fixture.add_moderator(charter_id(2), stranger).await;

        let (_, result) = fixture
            .resign(
                charter_id(1),
                "resigning",
                &[("memberId", id_value(stranger))],
            )
            .await;
        assert_refused_by_the_last_target(result, "memberId", stranger, "addedModerator");

        // A member of another charter is no member of this one either
        let (_, result) = fixture
            .resign(
                charter_id(2),
                "resigning",
                &[("memberId", id_value(member))],
            )
            .await;
        assert_refused_by_the_last_target(result, "memberId", member, "addedModerator");
    }

    /// Each element of a typed array meets the `anyOf` on its own, through
    /// whichever target holds for it; the first element no target holds for
    /// refuses the write, named by its list path.
    #[tokio::test]
    async fn should_check_every_element_against_the_targets_on_its_own() {
        let mut fixture = AnyOfFixture::new();
        let member = fixture.id(Who::Member);
        let moderator = fixture.id(Who::Moderator);
        let stranger = fixture.id(Who::Stranger);
        fixture
            .request_to_join(Who::Member, charter_id(1), "let me in")
            .await;
        fixture.add_moderator(charter_id(1), moderator).await;

        let (_, result) = fixture
            .resign(
                charter_id(1),
                "resigning",
                &[(
                    "members",
                    Value::Array(vec![id_value(member), id_value(moderator)]),
                )],
            )
            .await;
        assert_successful(&result);

        let (_, result) = fixture
            .resign(
                charter_id(1),
                "resigning",
                &[(
                    "members",
                    Value::Array(vec![
                        id_value(moderator),
                        id_value(member),
                        id_value(stranger),
                    ]),
                )],
            )
            .await;
        assert_refused_by_the_last_target(result, "members[2]", stranger, "addedModerator");
    }

    /// An identity or a document id: each target reads its own kind of
    /// entity, and a value neither holds for gets the last target's error,
    /// the document's.
    #[tokio::test]
    async fn should_accept_an_identity_or_a_document_id_and_refuse_neither() {
        let mut fixture = AnyOfFixture::new();
        let founder = fixture.id(Who::Founder);
        let request = fixture
            .request_to_join(Who::Member, charter_id(1), "let me in")
            .await;

        let (_, result) = fixture
            .resign(
                charter_id(1),
                "resigning",
                &[("signerOrRequestId", id_value(founder))],
            )
            .await;
        assert_successful(&result);

        let (_, result) = fixture
            .resign(
                charter_id(1),
                "resigning",
                &[("signerOrRequestId", id_value(request.id()))],
            )
            .await;
        assert_successful(&result);

        let nobody = Identifier::from([0x5A; 32]);
        let (_, result) = fixture
            .resign(
                charter_id(1),
                "resigning",
                &[("signerOrRequestId", id_value(nobody))],
            )
            .await;
        assert_matches!(
            result,
            PaidConsensusError {
                error: ConsensusError::StateError(StateError::ReferencedEntityNotFoundError(e)),
                ..
            } if e.path() == "signerOrRequestId"
                && *e.entity_id() == nobody
                && matches!(
                    e.entity_type(),
                    DocumentPropertyReferenceTarget::PermanentDocument { document_type_name, .. }
                        if document_type_name == "joinRequest"
                )
        );
    }

    /// A `propertyAgreement` belongs to its target: the first target's
    /// agreement fails a resignation whose title is not the join request's
    /// message, the second target has none, and when neither holds the error
    /// is the second's, not the first target's agreement mismatch.
    #[tokio::test]
    async fn should_check_an_agreement_only_against_its_own_targets_document() {
        let mut fixture = AnyOfFixture::new();
        let member = fixture.id(Who::Member);
        let moderator = fixture.id(Who::Moderator);
        fixture
            .request_to_join(Who::Member, charter_id(1), "let me in")
            .await;
        fixture.add_moderator(charter_id(1), moderator).await;

        let (_, result) = fixture
            .resign(
                charter_id(1),
                "let me in",
                &[("agreedMemberId", id_value(member))],
            )
            .await;
        assert_successful(&result);

        let (_, result) = fixture
            .resign(
                charter_id(1),
                "let me out",
                &[("agreedMemberId", id_value(member))],
            )
            .await;
        assert_refused_by_the_last_target(result, "agreedMemberId", member, "addedModerator");

        let (_, result) = fixture
            .resign(
                charter_id(1),
                "let me out",
                &[("agreedMemberId", id_value(moderator))],
            )
            .await;
        assert_successful(&result);
    }

    /// A replace re-validates the `anyOf` when a property one of its targets
    /// reads changed, and leaves it alone otherwise.
    #[tokio::test]
    async fn should_revalidate_an_any_of_when_a_key_part_one_target_reads_changes() {
        let mut fixture = AnyOfFixture::new();
        let member = fixture.id(Who::Member);
        fixture
            .request_to_join(Who::Member, charter_id(1), "let me in")
            .await;
        let (resignation, result) = fixture
            .resign(
                charter_id(1),
                "resigning",
                &[("memberId", id_value(member))],
            )
            .await;
        assert_successful(&result);

        let result = fixture
            .replace(&resignation, |resignation| {
                resignation.set("title", "resigning for good".into());
            })
            .await;
        assert_successful(&result);

        // The member asked to join charter 1, not charter 2
        let mut resignation = resignation;
        resignation.set("title", "resigning for good".into());
        resignation
            .increment_revision()
            .expect("the revision increments");
        let result = fixture
            .replace(&resignation, |resignation| {
                resignation.set("submittedCharterId", id_value(charter_id(2)));
            })
            .await;
        assert_refused_by_the_last_target(result, "memberId", member, "addedModerator");
    }

    /// Every read is billed, those of the targets that failed included: a
    /// value the second target holds for pays for the first target's query
    /// too, and a value neither holds for pays for both.
    #[tokio::test]
    async fn should_bill_the_reads_of_the_targets_that_failed() {
        let mut fixture = AnyOfFixture::new();
        let platform_version = PlatformVersion::latest();
        let founder = fixture.id(Who::Founder);
        let member = fixture.id(Who::Member);
        let moderator = fixture.id(Who::Moderator);
        let stranger = fixture.id(Who::Stranger);
        fixture
            .request_to_join(Who::Member, charter_id(1), "let me in")
            .await;
        fixture.add_moderator(charter_id(1), moderator).await;

        let (_, contract_fetch_info) = fixture
            .platform
            .drive
            .get_contract_with_fetch_info_and_fee(
                fixture.contract.id().to_buffer(),
                None,
                false,
                None,
                platform_version,
            )
            .expect("expected to fetch the contract");
        let base = DocumentBaseTransitionAction::V0(DocumentBaseTransitionActionV0 {
            id: Identifier::from([0xAA; 32]),
            identity_contract_nonce: 1,
            document_type_name: "resignation".to_string(),
            data_contract: contract_fetch_info.expect("the contract is in state"),
            token_cost: None,
            gas_fees_paid_by: GasFeesPaidBy::default(),
            contract_gas_fees_paid_by: GasFeesPaidBy::default(),
            declared_action_fee: None,
        });
        let platform_state = fixture.platform.state.load();
        let platform_ref = PlatformStateRef {
            drive: &fixture.platform.drive,
            state: &platform_state,
            config: &fixture.platform.config,
        };
        let validate = |member_id: Identifier| {
            let data = BTreeMap::from([
                ("submittedCharterId".to_string(), id_value(charter_id(1))),
                ("title".to_string(), Value::Text("resigning".to_string())),
                ("memberId".to_string(), id_value(member_id)),
            ]);
            let mut execution_context =
                StateTransitionExecutionContext::default_for_platform_version(platform_version)
                    .expect("expected an execution context");
            let result = base
                .validate_document_references(
                    &data,
                    founder,
                    // The resignation type records no creator ids
                    None,
                    None,
                    // A create: there is no stored document
                    None,
                    &platform_ref,
                    &BlockInfo::default(),
                    None,
                    &mut execution_context,
                    platform_version,
                )
                .expect("expected the references to be validated");
            let processing_fees = execution_context
                .operations_slice()
                .iter()
                .map(|operation| match operation {
                    ValidationOperation::PrecalculatedOperation(fee) => fee.processing_fee,
                    other => panic!("only document queries are billed here, found {other:?}"),
                })
                .collect::<Vec<_>>();
            (result, processing_fees)
        };

        // The first target holds: one query
        let (result, first_holds) = validate(member);
        assert!(result.is_valid(), "{:?}", result.errors);
        assert_eq!(first_holds.len(), 1, "the joinRequest query");

        // Only the second holds: the first target's failed query is billed too
        let (result, second_holds) = validate(moderator);
        assert!(result.is_valid(), "{:?}", result.errors);
        assert_eq!(
            second_holds.len(),
            2,
            "the failed joinRequest query and the addedModerator query"
        );
        assert!(second_holds.iter().all(|fee| *fee > 0));
        assert!(
            second_holds.iter().sum::<u64>() > first_holds.iter().sum::<u64>(),
            "a value the second target holds for costs more than one the first holds for"
        );

        // Neither holds: both queries are billed, and the write is refused
        let (result, neither_holds) = validate(stranger);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::StateError(
                StateError::ReferencedEntityNotFoundError(_)
            )]
        );
        assert_eq!(neither_holds.len(), 2);
    }
}
