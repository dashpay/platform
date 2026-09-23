//! `ownerRefersTo` (protocol version 14) through the full ABCI pipeline: a
//! document type's own `refersTo` declaration, whose value is the writer. The
//! fixture's `addedModerator` is unique on (`electedCharterId`, `memberId`),
//! and a `resignationRequest` may only be written by the `memberId` of an
//! `addedModerator` for its own `electedCharterId`, the moderation charters'
//! rule: the lookup takes the writer for `"."`. `resignationRequest` can be
//! transferred, so its owner can change without a write. `roleResignation`
//! adds a `propertyAgreement` checked against the moderator the lookup finds,
//! and `note` declares an identity target, which every writer meets.
//!
//! A writer the target does not accept is refused, paid, with the error the
//! target reports for a property, `ReferencedEntityNotFoundError` (40120) for
//! a lookup that finds nothing, naming `$ownerId`.

use super::*;

mod owner_reference_tests {
    use super::*;
    use dpp::data_contract::document_type::{DocumentPropertyReferenceTarget, DocumentTypeRef};
    use dpp::document::Document;
    use dpp::identifier::Identifier;
    use dpp::identity::{Identity, IdentityPublicKey};
    use dpp::prelude::{DataContract, IdentityNonce};
    use dpp::state_transition::StateTransition;
    use simple_signer::signer::SimpleSigner;
    use std::collections::BTreeMap;

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
    async fn should_refuse_a_replace_by_a_writer_who_no_longer_meets_the_owner_reference_even_when_no_field_changed(
    ) {
        let mut fixture = OwnerReferenceFixture::new();
        fixture.seat(Who::Member, charter_id(1), "chair").await;
        let stranger = fixture.id(Who::Stranger);

        let (request, result) = fixture
            .request_resignation(Who::Member, charter_id(1))
            .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );

        // The member still meets it: a replace changing nothing passes
        let result = fixture
            .replace(Who::Member, "resignationRequest", &request, |_| {})
            .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        let mut request = request;
        request
            .increment_revision()
            .expect("the revision increments");

        // A transfer is not checked: the reference governs writing
        let result = fixture
            .transfer(Who::Member, Who::Stranger, "resignationRequest", &request)
            .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        request
            .increment_revision()
            .expect("the revision increments");
        request.set_owner_id(stranger);

        // The new owner writes no field, and is still refused: the writer is
        // checked on every replace
        let result = fixture
            .replace(Who::Stranger, "resignationRequest", &request, |_| {})
            .await;
        assert_writer_not_found(result, stranger);
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
    }
}
