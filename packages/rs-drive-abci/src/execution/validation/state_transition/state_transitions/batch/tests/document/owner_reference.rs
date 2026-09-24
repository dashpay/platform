//! `ownerRefersTo` and `creatorRefersTo` (protocol version 14) through the
//! full ABCI pipeline: a document type's own `refersTo` declaration, whose
//! value is the writer or the creator. The
//! fixture's `addedModerator` is unique on (`electedCharterId`, `memberId`),
//! and a `resignationRequest` may only be written by the `memberId` of an
//! `addedModerator` for its own `electedCharterId`, the moderation charters'
//! rule: the lookup takes the writer for `"."`. Neither type can be
//! transferred or traded, so the writer stays the owner. `roleResignation`
//! adds a `propertyAgreement` checked against the moderator the lookup finds,
//! and `note` declares an identity target, which every writer meets.
//! `stepDownNotice` composes with `anyOf`: its writer is an added moderator
//! or the charter's founder (`founderSeat`). `seatNotice` may only be written
//! by the `memberId` of a `deletableSeat`, which the founder can delete to
//! take the seat back: a `deletableDocument` found through a lookup.
//! `moderatorBadge` can be transferred, so it declares `creatorRefersTo`
//! instead: only a seated moderator may mint one, and whoever holds it later,
//! the check is against that creator. `creatorNote` declares an identity
//! target on the creator.
//!
//! A writer or creator the target does not accept is refused, paid, with the
//! error the target reports for a property, `ReferencedEntityNotFoundError`
//! (40120) for a lookup that finds nothing, naming `$ownerId` or `$creatorId`.

use super::*;

mod owner_reference_tests {
    use super::*;
    use crate::execution::types::execution_operation::ValidationOperation;
    use crate::execution::types::state_transition_execution_context::{
        StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
    };
    use crate::execution::validation::state_transition::batch::action_validation::document::document_reference_validation::DocumentReferenceValidation;
    use crate::platform_types::platform::PlatformStateRef;
    use dpp::data_contract::document_type::{DocumentPropertyReferenceTarget, DocumentTypeRef};
    use dpp::document::Document;
    use dpp::identifier::Identifier;
    use dpp::identity::{Identity, IdentityPublicKey};
    use dpp::prelude::{DataContract, IdentityNonce};
    use dpp::state_transition::StateTransition;
    use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
    use dpp::validation::SimpleConsensusValidationResult;
    use dpp::version::DefaultForPlatformVersion;
    use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::{DocumentBaseTransitionAction, DocumentBaseTransitionActionV0};
    use simple_signer::signer::SimpleSigner;
    use std::collections::{BTreeMap, BTreeSet};

    const CONTRACT_PATH: &str = "tests/supporting_files/contract/reference-validation/reference-validation-contract-owner-refers-to.json";

    /// The identities of the fixture: the `Founder` seats moderators, the
    /// `Member` is seated for charter 1, and the `Stranger` for nothing.
    #[derive(Clone, Copy)]
    enum Who {
        Founder,
        Member,
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

    impl Writer {
        fn next_nonce(&mut self) -> IdentityNonce {
            let this_one = self.next_nonce;
            self.next_nonce += 1;
            this_one
        }
    }

    struct OwnerReferenceFixture {
        platform: TempPlatform<MockCoreRPCLike>,
        contract: DataContract,
        rng: StdRng,
        founder: Writer,
        member: Writer,
        stranger: Writer,
    }

    fn id_value(id: Identifier) -> Value {
        Value::Identifier(id.to_buffer())
    }

    fn charter_id(byte: u8) -> Identifier {
        Identifier::from([byte; 32])
    }

    impl OwnerReferenceFixture {
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
            let stranger = writer(1960);

            // Parsed with full validation, so the contract-level lookup checks
            // run on the fixture too
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
                rng: StdRng::seed_from_u64(5533),
                founder,
                member,
                stranger,
            }
        }

        fn id(&self, who: Who) -> Identifier {
            match who {
                Who::Founder => self.founder.identity.id(),
                Who::Member => self.member.identity.id(),
                Who::Stranger => self.stranger.identity.id(),
            }
        }

        /// The document type named `type_name`, `who`'s writer and the random
        /// source: the parts a transition is built from, borrowed at once.
        fn parts(
            &mut self,
            who: Who,
            type_name: &str,
        ) -> (DocumentTypeRef<'_>, &mut Writer, &mut StdRng) {
            let document_type = self
                .contract
                .document_type_for_name(type_name)
                .expect("expected the document type");
            let writer = match who {
                Who::Founder => &mut self.founder,
                Who::Member => &mut self.member,
                Who::Stranger => &mut self.stranger,
            };
            (document_type, writer, &mut self.rng)
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

        /// Creates a `type_name` document written by `who`, with `values` set
        /// over the random required ones, and returns it with the result.
        async fn create(
            &mut self,
            who: Who,
            type_name: &str,
            values: &[(&str, Value)],
        ) -> (Document, StateTransitionExecutionResult) {
            let platform_version = PlatformVersion::latest();
            let owner_id = self.id(who);
            let (document_type, writer, rng) = self.parts(who, type_name);
            let entropy = Bytes32::random_with_rng(rng);
            let mut document = document_type
                .random_document_with_identifier_and_entropy(
                    rng,
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
            let nonce = writer.next_nonce();
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
        /// `who`, its current owner.
        async fn replace(
            &mut self,
            who: Who,
            type_name: &str,
            document: &Document,
            change: impl FnOnce(&mut Document),
        ) -> StateTransitionExecutionResult {
            let platform_version = PlatformVersion::latest();
            let mut replacement = document.clone();
            replacement
                .increment_revision()
                .expect("the revision increments");
            change(&mut replacement);
            let (document_type, writer, _) = self.parts(who, type_name);
            let nonce = writer.next_nonce();
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

        /// Transfers `document`, as last accepted, from `from`, its owner,
        /// to `to`.
        async fn transfer(
            &mut self,
            from: Who,
            to: Who,
            type_name: &str,
            document: &Document,
        ) -> StateTransitionExecutionResult {
            let platform_version = PlatformVersion::latest();
            let recipient = self.id(to);
            let mut transferred = document.clone();
            transferred
                .increment_revision()
                .expect("the revision increments");
            let (document_type, writer, _) = self.parts(from, type_name);
            let nonce = writer.next_nonce();
            let transition = BatchTransition::new_document_transfer_transition_from_document(
                transferred,
                document_type,
                recipient,
                &writer.key,
                nonce,
                0,
                None,
                &writer.signer,
                platform_version,
                None,
            )
            .await
            .expect("expected the transfer transition");
            self.process(&transition)
        }

        /// Runs the document reference validation directly on `data`, a
        /// `type_name` document written by `who` and created by `creator` (on
        /// a type that records creators), as a create, or as a replace
        /// changing `changed_fields`, and returns the result with the execution
        /// context, whose operations are the reads it billed.
        fn validate_directly(
            &self,
            who: Who,
            creator: Option<Who>,
            type_name: &str,
            data: BTreeMap<String, Value>,
            changed_fields: Option<BTreeSet<String>>,
        ) -> (
            SimpleConsensusValidationResult,
            StateTransitionExecutionContext,
        ) {
            let platform_version = PlatformVersion::latest();
            let (_, contract_fetch_info) = self
                .platform
                .drive
                .get_contract_with_fetch_info_and_fee(
                    self.contract.id().to_buffer(),
                    None,
                    false,
                    None,
                    platform_version,
                )
                .expect("expected to fetch the contract");
            let base = DocumentBaseTransitionAction::V0(DocumentBaseTransitionActionV0 {
                id: Identifier::from([0xAB; 32]),
                identity_contract_nonce: 1,
                document_type_name: type_name.to_string(),
                data_contract: contract_fetch_info.expect("the contract is in state"),
                token_cost: None,
                gas_fees_paid_by: GasFeesPaidBy::default(),
                contract_gas_fees_paid_by: GasFeesPaidBy::default(),
                declared_action_fee: None,
            });
            let platform_state = self.platform.state.load();
            let platform_ref = PlatformStateRef {
                drive: &self.platform.drive,
                state: &platform_state,
                config: &self.platform.config,
            };
            let mut execution_context =
                StateTransitionExecutionContext::default_for_platform_version(platform_version)
                    .expect("expected an execution context");
            let result = base
                .validate_document_references(
                    &data,
                    self.id(who),
                    creator.map(|creator| self.id(creator)),
                    changed_fields.as_ref(),
                    None,
                    &platform_ref,
                    &BlockInfo::default(),
                    None,
                    &mut execution_context,
                    platform_version,
                )
                .expect("expected the references to be validated");
            (result, execution_context)
        }

        /// Seats `who` as a moderator of the elected charter `charter` in
        /// `role`, an `addedModerator` document written by the founder.
        async fn seat(&mut self, who: Who, charter: Identifier, role: &str) {
            let member_id = self.id(who);
            let (_, result) = self
                .create(
                    Who::Founder,
                    "addedModerator",
                    &[
                        ("electedCharterId", id_value(charter)),
                        ("memberId", id_value(member_id)),
                        ("role", role.into()),
                    ],
                )
                .await;
            assert_matches!(
                result,
                StateTransitionExecutionResult::SuccessfulExecution { .. }
            );
        }

        /// Deletes `document`, a `type_name` document owned by `who`.
        async fn delete(
            &mut self,
            who: Who,
            type_name: &str,
            document: &Document,
        ) -> StateTransitionExecutionResult {
            let platform_version = PlatformVersion::latest();
            let (document_type, writer, _) = self.parts(who, type_name);
            let nonce = writer.next_nonce();
            let transition = BatchTransition::new_document_deletion_transition_from_document(
                document.clone(),
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
            .expect("expected the delete transition");
            self.process(&transition)
        }

        /// A resignation request by `who` from the elected charter `charter`.
        async fn request_resignation(
            &mut self,
            who: Who,
            charter: Identifier,
        ) -> (Document, StateTransitionExecutionResult) {
            self.create(
                who,
                "resignationRequest",
                &[
                    ("electedCharterId", id_value(charter)),
                    ("reason", "stepping down".into()),
                ],
            )
            .await
        }
    }

    /// The refusal of a writer the owner reference's lookup found no
    /// moderator for, naming `$ownerId` and the writer.
    fn assert_writer_not_found(result: StateTransitionExecutionResult, writer: Identifier) {
        assert_lookup_not_found(result, "$ownerId", writer);
    }

    /// The refusal of an identity, at `path`, the reference's lookup found no
    /// moderator for.
    fn assert_lookup_not_found(
        result: StateTransitionExecutionResult,
        path: &str,
        identity: Identifier,
    ) {
        assert_matches!(
            result,
            PaidConsensusError {
                error: ConsensusError::StateError(StateError::ReferencedEntityNotFoundError(e)),
                ..
            } if e.path() == path
                && *e.entity_id() == identity
                && matches!(
                    e.entity_type(),
                    DocumentPropertyReferenceTarget::PermanentDocumentLookup { .. }
                ),
            "expected 40120 at {path}"
        );
    }

    #[tokio::test]
    async fn should_create_a_document_whose_writer_meets_the_owner_reference() {
        let mut fixture = OwnerReferenceFixture::new();
        fixture.seat(Who::Member, charter_id(1), "chair").await;

        let (_, result) = fixture
            .request_resignation(Who::Member, charter_id(1))
            .await;

        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
    }

    #[tokio::test]
    async fn should_refuse_a_writer_who_does_not_meet_the_owner_reference() {
        let mut fixture = OwnerReferenceFixture::new();
        fixture.seat(Who::Member, charter_id(1), "chair").await;
        let stranger = fixture.id(Who::Stranger);
        let member = fixture.id(Who::Member);

        // Nobody seated the stranger
        let (_, result) = fixture
            .request_resignation(Who::Stranger, charter_id(1))
            .await;
        assert_writer_not_found(result, stranger);

        // and the member is seated for charter 1, not charter 2: the key's
        // other part is the document's own `electedCharterId`
        let (_, result) = fixture
            .request_resignation(Who::Member, charter_id(2))
            .await;
        assert_writer_not_found(result, member);
    }

    #[tokio::test]
    async fn should_refuse_a_replace_that_moves_the_writer_off_the_owner_reference() {
        let mut fixture = OwnerReferenceFixture::new();
        fixture.seat(Who::Member, charter_id(1), "chair").await;
        let member = fixture.id(Who::Member);

        let (request, result) = fixture
            .request_resignation(Who::Member, charter_id(1))
            .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );

        // Touching nothing the lookup reads leaves the reference alone
        let result = fixture
            .replace(Who::Member, "resignationRequest", &request, |request| {
                request.set("reason", "changed my mind".into());
            })
            .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        let mut request = request;
        request
            .increment_revision()
            .expect("the revision increments");
        request.set("reason", "changed my mind".into());

        // Moving the key part re-validates it: the member is not seated for
        // charter 2
        let result = fixture
            .replace(Who::Member, "resignationRequest", &request, |request| {
                request.set("electedCharterId", id_value(charter_id(2)));
            })
            .await;
        assert_writer_not_found(result, member);
    }

    /// A replace that changes nothing the lookup reads cannot change its
    /// outcome (the writer stays the owner, the target can never be deleted
    /// and its key is fixed), so it reads nothing; one that moves a key part
    /// is billed the lookup.
    #[tokio::test]
    async fn should_read_nothing_on_a_replace_leaving_the_owner_lookup_keys_alone() {
        let mut fixture = OwnerReferenceFixture::new();
        fixture.seat(Who::Member, charter_id(1), "chair").await;
        let request = |charter: Identifier| {
            BTreeMap::from([
                ("electedCharterId".to_string(), id_value(charter)),
                (
                    "reason".to_string(),
                    Value::Text("stepping down".to_string()),
                ),
            ])
        };

        let (result, execution_context) = fixture.validate_directly(
            Who::Member,
            None,
            "resignationRequest",
            request(charter_id(1)),
            Some(BTreeSet::from(["reason".to_string()])),
        );
        assert!(result.is_valid(), "{:?}", result.errors);
        assert!(execution_context.operations_slice().is_empty());

        let (result, execution_context) = fixture.validate_directly(
            Who::Member,
            None,
            "resignationRequest",
            request(charter_id(1)),
            Some(BTreeSet::from(["electedCharterId".to_string()])),
        );
        assert!(result.is_valid(), "{:?}", result.errors);
        assert_matches!(
            execution_context.operations_slice(),
            [ValidationOperation::PrecalculatedOperation(fee)] if fee.processing_fee > 0,
            "the lookup query is the one billed operation"
        );
    }

    #[tokio::test]
    async fn should_check_a_property_agreement_of_the_owner_reference_against_the_document_found() {
        let mut fixture = OwnerReferenceFixture::new();
        fixture.seat(Who::Member, charter_id(1), "chair").await;

        // `role` must be the seated moderator's own, and the writer its member
        let (_, agreeing) = fixture
            .create(
                Who::Member,
                "roleResignation",
                &[
                    ("electedCharterId", id_value(charter_id(1))),
                    ("role", "chair".into()),
                ],
            )
            .await;
        assert_matches!(
            agreeing,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );

        let (_, disagreeing) = fixture
            .create(
                Who::Member,
                "roleResignation",
                &[
                    ("electedCharterId", id_value(charter_id(1))),
                    ("role", "scribe".into()),
                ],
            )
            .await;
        assert_matches!(
            disagreeing,
            PaidConsensusError {
                error: ConsensusError::StateError(
                    StateError::ReferencedDocumentPropertyMismatchError(e)
                ),
                ..
            } if e.path() == "$ownerId"
                && e.referring_property() == "role"
                && e.referenced_property() == "role"
        );
    }

    #[tokio::test]
    async fn should_refuse_nothing_extra_for_an_identity_owner_reference() {
        let mut fixture = OwnerReferenceFixture::new();

        // The writer always exists: whoever writes, the note is accepted
        for who in [Who::Founder, Who::Member, Who::Stranger] {
            let (_, result) = fixture
                .create(who, "note", &[("text", "hello".into())])
                .await;
            assert_matches!(
                result,
                StateTransitionExecutionResult::SuccessfulExecution { .. }
            );
        }

        // and nothing is read to find that out, on a create or a replace
        for changed_fields in [None, Some(BTreeSet::from(["text".to_string()]))] {
            let (result, execution_context) = fixture.validate_directly(
                Who::Stranger,
                None,
                "note",
                BTreeMap::from([("text".to_string(), Value::Text("hello".to_string()))]),
                changed_fields,
            );
            assert!(result.is_valid(), "{:?}", result.errors);
            assert!(execution_context.operations_slice().is_empty());
        }
    }

    /// A badge by `who` for the elected charter `charter`.
    async fn mint_badge(
        fixture: &mut OwnerReferenceFixture,
        who: Who,
        charter: Identifier,
    ) -> (Document, StateTransitionExecutionResult) {
        fixture
            .create(
                who,
                "moderatorBadge",
                &[
                    ("electedCharterId", id_value(charter)),
                    ("label", "seated".into()),
                ],
            )
            .await
    }

    #[tokio::test]
    async fn should_create_a_document_whose_creator_meets_the_creator_reference() {
        let mut fixture = OwnerReferenceFixture::new();
        fixture.seat(Who::Member, charter_id(1), "chair").await;
        let stranger = fixture.id(Who::Stranger);

        let (_, result) = mint_badge(&mut fixture, Who::Member, charter_id(1)).await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );

        let (_, result) = mint_badge(&mut fixture, Who::Stranger, charter_id(1)).await;
        assert_lookup_not_found(result, "$creatorId", stranger);
    }

    /// After a transfer the new owner writes, but the value checked is still
    /// the creator: a replace moving a key part is judged against the member
    /// who minted the badge, not the stranger who holds it.
    #[tokio::test]
    async fn should_check_the_creator_not_the_owner_after_a_transfer() {
        let mut fixture = OwnerReferenceFixture::new();
        fixture.seat(Who::Member, charter_id(1), "chair").await;
        fixture.seat(Who::Stranger, charter_id(2), "chair").await;
        let member = fixture.id(Who::Member);
        let stranger = fixture.id(Who::Stranger);

        let (badge, result) = mint_badge(&mut fixture, Who::Member, charter_id(1)).await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );

        // A transfer does not change the creator, so it needs no check
        let result = fixture
            .transfer(Who::Member, Who::Stranger, "moderatorBadge", &badge)
            .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        let mut badge = badge;
        badge.increment_revision().expect("the revision increments");
        badge.set_owner_id(stranger);

        // The new owner relabels it: nothing the lookup reads changed
        let result = fixture
            .replace(Who::Stranger, "moderatorBadge", &badge, |badge| {
                badge.set("label", "passed on".into());
            })
            .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        badge.increment_revision().expect("the revision increments");
        badge.set("label", "passed on".into());

        // Moving it to charter 2, where the stranger is seated but the creator
        // is not, is refused for the creator
        let result = fixture
            .replace(Who::Stranger, "moderatorBadge", &badge, |badge| {
                badge.set("electedCharterId", id_value(charter_id(2)));
            })
            .await;
        assert_lookup_not_found(result, "$creatorId", member);
    }

    #[tokio::test]
    async fn should_read_nothing_for_an_identity_creator_reference() {
        let fixture = OwnerReferenceFixture::new();

        // The creator existed when it wrote the document, and an identity is
        // never removed: nothing is read on a create or on a replace by
        // another owner
        for (writer, changed_fields) in [
            (Who::Stranger, None),
            (Who::Member, Some(BTreeSet::from(["text".to_string()]))),
        ] {
            let (result, execution_context) = fixture.validate_directly(
                writer,
                Some(Who::Stranger),
                "creatorNote",
                BTreeMap::from([("text".to_string(), Value::Text("hello".to_string()))]),
                changed_fields,
            );
            assert!(result.is_valid(), "{:?}", result.errors);
            assert!(execution_context.operations_slice().is_empty());
        }
    }

    /// `anyOf` on the writer: either operand admits the writer, checked in
    /// declared order, and a writer neither admits is refused with the last
    /// operand's error, at `$ownerId`.
    #[tokio::test]
    async fn should_admit_a_writer_meeting_any_operand_of_an_owner_reference_expression() {
        let mut fixture = OwnerReferenceFixture::new();
        fixture.seat(Who::Member, charter_id(1), "chair").await;
        let stranger = fixture.id(Who::Stranger);
        let founder = fixture.id(Who::Founder);
        let (_, result) = fixture
            .create(
                Who::Founder,
                "founderSeat",
                &[
                    ("electedCharterId", id_value(charter_id(1))),
                    ("founderId", id_value(stranger)),
                ],
            )
            .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );

        // Through the first operand, an added moderator, and through the
        // second, the founder's seat
        for who in [Who::Member, Who::Stranger] {
            let (_, result) = fixture
                .create(
                    who,
                    "stepDownNotice",
                    &[("electedCharterId", id_value(charter_id(1)))],
                )
                .await;
            assert_matches!(
                result,
                StateTransitionExecutionResult::SuccessfulExecution { .. }
            );
        }

        // The founder of the contract holds neither seat
        let (_, result) = fixture
            .create(
                Who::Founder,
                "stepDownNotice",
                &[("electedCharterId", id_value(charter_id(1)))],
            )
            .await;
        assert_matches!(
            result,
            PaidConsensusError {
                error: ConsensusError::StateError(StateError::ReferencedEntityNotFoundError(e)),
                ..
            } if e.path() == "$ownerId"
                && *e.entity_id() == founder
                && matches!(
                    e.entity_type(),
                    DocumentPropertyReferenceTarget::PermanentDocumentLookup {
                        document_type_name, ..
                    } if document_type_name == "founderSeat"
                ),
            "expected the last operand's 40120 at $ownerId"
        );
    }

    /// A writer met through a deletable document found by a lookup, as a moderation charter's
    /// added moderator the leader can take off: the writer passes while its seat exists, and
    /// every replace asks again, one leaving the lookup keys alone included, since the seat
    /// can be deleted. Once it is, the writer can no longer replace the document.
    #[tokio::test]
    async fn should_check_a_deletable_owner_lookup_on_every_replace() {
        let mut fixture = OwnerReferenceFixture::new();
        let member = fixture.id(Who::Member);
        let stranger = fixture.id(Who::Stranger);
        let (seat, result) = fixture
            .create(
                Who::Founder,
                "deletableSeat",
                &[
                    ("electedCharterId", id_value(charter_id(1))),
                    ("memberId", id_value(member)),
                ],
            )
            .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        let notice_values = [
            ("electedCharterId", id_value(charter_id(1))),
            ("text", Value::from("on duty")),
        ];
        let (notice, result) = fixture
            .create(Who::Member, "seatNotice", &notice_values)
            .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        let assert_seat_not_found = |result, writer: Identifier| {
            assert_matches!(
                result,
                PaidConsensusError {
                    error: ConsensusError::StateError(StateError::ReferencedEntityNotFoundError(e)),
                    ..
                } if e.path() == "$ownerId"
                    && *e.entity_id() == writer
                    && matches!(
                        e.entity_type(),
                        DocumentPropertyReferenceTarget::DeletableDocumentLookup {
                            document_type_name, ..
                        } if document_type_name == "deletableSeat"
                    ),
                "expected 40120 at $ownerId for a deletable seat"
            );
        };
        let (_, result) = fixture
            .create(Who::Stranger, "seatNotice", &notice_values)
            .await;
        assert_seat_not_found(result, stranger);

        // A replace touching nothing the lookup reads still reads the seat
        let (result, execution_context) = fixture.validate_directly(
            Who::Member,
            None,
            "seatNotice",
            notice_values
                .iter()
                .map(|(property, value)| (property.to_string(), value.clone()))
                .collect(),
            Some(BTreeSet::from(["text".to_string()])),
        );
        assert!(result.is_valid(), "{:?}", result.errors);
        assert_matches!(
            execution_context.operations_slice(),
            [ValidationOperation::PrecalculatedOperation(fee)] if fee.processing_fee > 0,
            "the lookup query is billed on every replace"
        );

        // The founder takes the seat back, and the member's next replace is refused
        let result = fixture.delete(Who::Founder, "deletableSeat", &seat).await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        let result = fixture
            .replace(Who::Member, "seatNotice", &notice, |notice| {
                notice.set("text", "off duty".into());
            })
            .await;
        assert_seat_not_found(result, member);
    }
}
