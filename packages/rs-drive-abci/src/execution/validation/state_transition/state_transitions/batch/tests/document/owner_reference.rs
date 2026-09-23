//! `ownerRefersTo` (protocol version 14) through the full ABCI pipeline: a
//! document type's own `refersTo` declaration, whose value is the writer. The
//! fixture's `addedModerator` is unique on (`electedCharterId`, `memberId`),
//! and a `resignationRequest` may only be written by the `memberId` of an
//! `addedModerator` for its own `electedCharterId`, the moderation charters'
//! rule: the lookup takes the writer for `"."`. Neither type can be
//! transferred or traded, so the writer stays the owner. `roleResignation`
//! adds a `propertyAgreement` checked against the moderator the lookup finds,
//! and `note` declares an identity target, which every writer meets.
//!
//! A writer the target does not accept is refused, paid, with the error the
//! target reports for a property, `ReferencedEntityNotFoundError` (40120) for
//! a lookup that finds nothing, naming `$ownerId`.

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

        /// Runs the document reference validation directly on `data`, a
        /// `type_name` document written by `who`, as a create, or as a replace
        /// changing `changed_fields`, and returns the result with the execution
        /// context, whose operations are the reads it billed.
        fn validate_directly(
            &self,
            who: Who,
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
                    // The fixture's types record no creator ids
                    None,
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
        assert_matches!(
            result,
            PaidConsensusError {
                error: ConsensusError::StateError(StateError::ReferencedEntityNotFoundError(e)),
                ..
            } if e.path() == "$ownerId"
                && *e.entity_id() == writer
                && matches!(
                    e.entity_type(),
                    DocumentPropertyReferenceTarget::PermanentDocumentLookup { .. }
                ),
            "expected 40120 at $ownerId"
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
            "resignationRequest",
            request(charter_id(1)),
            Some(BTreeSet::from(["reason".to_string()])),
        );
        assert!(result.is_valid(), "{:?}", result.errors);
        assert!(execution_context.operations_slice().is_empty());

        let (result, execution_context) = fixture.validate_directly(
            Who::Member,
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
                "note",
                BTreeMap::from([("text".to_string(), Value::Text("hello".to_string()))]),
                changed_fields,
            );
            assert!(result.is_valid(), "{:?}", result.errors);
            assert!(execution_context.operations_slice().is_empty());
        }
    }
}
