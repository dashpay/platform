//! Document references resolved through a unique index (`refersTo.lookup`,
//! protocol version 14) through the full ABCI pipeline. The fixture's
//! `joinRequest` is unique on (`submittedCharterId`, `$ownerId`), and the
//! `charter` type refers to it by the requester instead of by the request's
//! id: `memberId` must be the owner of a join request for the
//! charter's own `submittedCharterId`, and `members` is the moderation
//! charter's list of them, a typed array whose elements carry the same
//! declaration. `agreedMemberId` adds a `propertyAgreement` checked against the
//! request the lookup finds, `requestedCharterId` reads the writer as a key
//! part (which is why `charter` can be neither transferred nor traded) and
//! `metaMemberId` a nested one, `meta.charterId`. The second contract,
//! `vote`, refers to the same requests from another contract.
//!
//! A value the index finds no document for is refused, paid, with
//! `ReferencedEntityNotFoundError` (40120) naming the property, or the element
//! by its list path.

use super::*;

mod lookup_reference_tests {
    use super::*;
    use crate::execution::types::execution_operation::ValidationOperation;
    use crate::execution::types::state_transition_execution_context::{
        StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
    };
    use crate::execution::validation::state_transition::batch::action_validation::document::document_reference_validation::DocumentReferenceValidation;
    use crate::platform_types::platform::PlatformStateRef;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TempPlatform;
    use dpp::data_contract::document_type::{DocumentPropertyReferenceTarget, DocumentTypeRef};
    use dpp::document::Document;
    use dpp::identifier::Identifier;
    use dpp::identity::{Identity, IdentityPublicKey};
    use dpp::platform_value::platform_value;
    use dpp::prelude::{DataContract, IdentityNonce};
    use dpp::state_transition::StateTransition;
    use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
    use dpp::version::DefaultForPlatformVersion;
    use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::{DocumentBaseTransitionAction, DocumentBaseTransitionActionV0};
    use simple_signer::signer::SimpleSigner;
    use std::collections::BTreeMap;

    const CONTRACT_PATH: &str =
        "tests/supporting_files/contract/reference-validation/reference-validation-contract-lookup.json";

    /// A `vote` type whose `voterId` looks join requests up in the contract
    /// above, by its id: the lookup into another contract.
    const VOTING_CONTRACT_PATH: &str = "tests/supporting_files/contract/reference-validation/reference-validation-contract-lookup-registration-foreign-valid.json";

    /// The identities of the fixture: the `Founder` writes charters and votes,
    /// the `Member` and the `Stranger` join requests where a test needs them.
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
        /// The identity contract nonce the next transition uses, per contract;
        /// every processed transition consumes one, a refused one included.
        next_nonces: BTreeMap<Identifier, IdentityNonce>,
    }

    impl Writer {
        fn next_nonce(&mut self, contract_id: Identifier) -> IdentityNonce {
            let nonce = self.next_nonces.entry(contract_id).or_insert(1);
            let this_one = *nonce;
            *nonce += 1;
            this_one
        }
    }

    struct LookupFixture {
        platform: TempPlatform<MockCoreRPCLike>,
        contract: DataContract,
        voting_contract: DataContract,
        rng: StdRng,
        founder: Writer,
        member: Writer,
        stranger: Writer,
    }

    fn id_value(id: Identifier) -> Value {
        Value::Identifier(id.to_buffer())
    }

    impl LookupFixture {
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
                    next_nonces: BTreeMap::new(),
                }
            };
            let founder = writer(958);
            let member = writer(959);
            let stranger = writer(960);

            // Parsed with full validation, so the contract-level lookup
            // checks run on the fixtures too
            let [contract, voting_contract] = [CONTRACT_PATH, VOTING_CONTRACT_PATH].map(|path| {
                let contract = json_document_to_contract(path, true, platform_version)
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
                contract
            });

            Self {
                platform,
                contract,
                voting_contract,
                rng: StdRng::seed_from_u64(4433),
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

        /// The document type named `type_name`, in whichever fixture contract
        /// has it, with the contract's id, `who`'s writer and the random source:
        /// the parts a transition is built from, borrowed at once.
        fn parts(
            &mut self,
            who: Who,
            type_name: &str,
        ) -> (DocumentTypeRef<'_>, Identifier, &mut Writer, &mut StdRng) {
            let (document_type, contract_id) = [&self.contract, &self.voting_contract]
                .into_iter()
                .find_map(|contract| {
                    contract
                        .document_type_optional_for_name(type_name)
                        .map(|document_type| (document_type, contract.id()))
                })
                .expect("expected the document type");
            let writer = match who {
                Who::Founder => &mut self.founder,
                Who::Member => &mut self.member,
                Who::Stranger => &mut self.stranger,
            };
            (document_type, contract_id, writer, &mut self.rng)
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
            let (document_type, contract_id, writer, rng) = self.parts(who, type_name);
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
            let nonce = writer.next_nonce(contract_id);
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
            let (document_type, contract_id, writer, _) = self.parts(who, type_name);
            let nonce = writer.next_nonce(contract_id);
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

        /// A charter by the founder for `charter_id`, with `values` set.
        async fn create_charter(
            &mut self,
            charter_id: Identifier,
            values: &[(&str, Value)],
        ) -> (Document, StateTransitionExecutionResult) {
            let mut all = vec![
                ("submittedCharterId", id_value(charter_id)),
                ("title", "charter".into()),
            ];
            all.extend(values.iter().cloned());
            self.create(Who::Founder, "charter", &all).await
        }
    }

    fn charter_id(byte: u8) -> Identifier {
        Identifier::from([byte; 32])
    }

    /// The refusal of a value the index found no document for, naming the
    /// property and the value.
    fn assert_not_found(result: StateTransitionExecutionResult, path: &str, value: Identifier) {
        assert_matches!(
            result,
            PaidConsensusError {
                error: ConsensusError::StateError(StateError::ReferencedEntityNotFoundError(e)),
                ..
            } if e.path() == path
                && *e.entity_id() == value
                && matches!(
                    e.entity_type(),
                    DocumentPropertyReferenceTarget::PermanentDocumentLookup { .. }
                ),
            "expected 40120 at {path}"
        );
    }

    #[tokio::test]
    async fn should_create_a_document_whose_lookup_value_owns_a_request_for_its_target() {
        let mut fixture = LookupFixture::new();
        let member = fixture.id(Who::Member);
        fixture
            .request_to_join(Who::Member, charter_id(1), "let me in")
            .await;

        let (_, result) = fixture
            .create_charter(charter_id(1), &[("memberId", id_value(member))])
            .await;

        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
    }

    #[tokio::test]
    async fn should_refuse_a_lookup_value_that_owns_no_request_for_its_target() {
        let mut fixture = LookupFixture::new();
        let stranger = fixture.id(Who::Stranger);
        fixture
            .request_to_join(Who::Member, charter_id(1), "let me in")
            .await;

        let (_, result) = fixture
            .create_charter(charter_id(1), &[("memberId", id_value(stranger))])
            .await;

        assert_not_found(result, "memberId", stranger);
    }

    #[tokio::test]
    async fn should_not_resolve_a_lookup_through_a_request_for_another_target() {
        let mut fixture = LookupFixture::new();
        let member = fixture.id(Who::Member);
        fixture
            .request_to_join(Who::Member, charter_id(2), "let me in")
            .await;

        // The member asked to join charter 2, not charter 1
        let (_, result) = fixture
            .create_charter(charter_id(1), &[("memberId", id_value(member))])
            .await;

        assert_not_found(result, "memberId", member);
    }

    #[tokio::test]
    async fn should_refuse_a_replace_that_moves_the_target_to_one_with_no_request() {
        let mut fixture = LookupFixture::new();
        let member = fixture.id(Who::Member);
        fixture
            .request_to_join(Who::Member, charter_id(1), "let me in")
            .await;
        let (charter, result) = fixture
            .create_charter(charter_id(1), &[("memberId", id_value(member))])
            .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );

        // Touching neither the value nor a key part leaves the reference alone
        let result = fixture
            .replace(Who::Founder, "charter", &charter, |charter| {
                charter.set("title", "renamed".into());
            })
            .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        let mut charter = charter;
        charter
            .increment_revision()
            .expect("the revision increments");
        charter.set("title", "renamed".into());

        // Moving the key part re-validates it: the member never asked to join
        // charter 2
        let result = fixture
            .replace(Who::Founder, "charter", &charter, |charter| {
                charter.set("submittedCharterId", id_value(charter_id(2)));
            })
            .await;
        assert_not_found(result, "memberId", member);
    }

    #[tokio::test]
    async fn should_check_a_property_agreement_beside_the_lookup_against_the_resolved_document() {
        let mut fixture = LookupFixture::new();
        let member = fixture.id(Who::Member);
        fixture
            .request_to_join(Who::Member, charter_id(1), "welcome")
            .await;

        let (_, agreeing) = fixture
            .create_charter(
                charter_id(1),
                &[
                    ("title", "welcome".into()),
                    ("agreedMemberId", id_value(member)),
                ],
            )
            .await;
        assert_matches!(
            agreeing,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );

        let (_, disagreeing) = fixture
            .create_charter(
                charter_id(1),
                &[
                    ("title", "goodbye".into()),
                    ("agreedMemberId", id_value(member)),
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
            } if e.path() == "agreedMemberId"
                && e.referring_property() == "title"
                && e.referenced_property() == "message"
        );
    }

    #[tokio::test]
    async fn should_resolve_a_lookup_whose_key_reads_the_writer() {
        let mut fixture = LookupFixture::new();
        // `requestedCharterId` must be a charter the writer itself asked to join
        fixture
            .request_to_join(Who::Founder, charter_id(1), "my own")
            .await;

        let (_, asked) = fixture
            .create_charter(
                charter_id(9),
                &[("requestedCharterId", id_value(charter_id(1)))],
            )
            .await;
        assert_matches!(
            asked,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );

        let (_, never_asked) = fixture
            .create_charter(
                charter_id(9),
                &[("requestedCharterId", id_value(charter_id(2)))],
            )
            .await;
        assert_not_found(never_asked, "requestedCharterId", charter_id(2));
    }

    #[tokio::test]
    async fn should_read_a_nested_key_part() {
        let mut fixture = LookupFixture::new();
        let member = fixture.id(Who::Member);
        fixture
            .request_to_join(Who::Member, charter_id(2), "let me in")
            .await;

        // `metaMemberId` is keyed by `meta.charterId`, not by the charter's
        // own `submittedCharterId`
        let (_, nested) = fixture
            .create_charter(
                charter_id(1),
                &[
                    ("meta", platform_value!({ "charterId": charter_id(2) })),
                    ("metaMemberId", id_value(member)),
                ],
            )
            .await;
        assert_matches!(
            nested,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );

        let (_, elsewhere) = fixture
            .create_charter(
                charter_id(2),
                &[
                    ("meta", platform_value!({ "charterId": charter_id(1) })),
                    ("metaMemberId", id_value(member)),
                ],
            )
            .await;
        assert_not_found(elsewhere, "metaMemberId", member);
    }

    /// The charter contract's `members`: every element must be the owner of a
    /// join request for the charter's own submitted charter.
    #[tokio::test]
    async fn should_look_up_every_element_of_a_members_list() {
        let mut fixture = LookupFixture::new();
        let member = fixture.id(Who::Member);
        let stranger = fixture.id(Who::Stranger);
        fixture
            .request_to_join(Who::Member, charter_id(1), "let me in")
            .await;

        // One element without a request refuses the write, the error naming
        // the element by its list path
        let (_, one_missing) = fixture
            .create_charter(
                charter_id(1),
                &[(
                    "members",
                    Value::Array(vec![id_value(member), id_value(stranger)]),
                )],
            )
            .await;
        assert_not_found(one_missing, "members[1]", stranger);

        fixture
            .request_to_join(Who::Stranger, charter_id(1), "me too")
            .await;
        let (charter, all_asked) = fixture
            .create_charter(
                charter_id(1),
                &[(
                    "members",
                    Value::Array(vec![id_value(member), id_value(stranger)]),
                )],
            )
            .await;
        assert_matches!(
            all_asked,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );

        // Moving the charter's key part re-validates every element, not only
        // the ones the stored list did not hold
        let moved = fixture
            .replace(Who::Founder, "charter", &charter, |charter| {
                charter.set("submittedCharterId", id_value(charter_id(2)));
            })
            .await;
        assert_not_found(moved, "members[0]", member);
    }

    /// A lookup into another contract resolves in that contract's documents.
    #[tokio::test]
    async fn should_look_up_documents_of_another_contract() {
        let mut fixture = LookupFixture::new();
        let member = fixture.id(Who::Member);
        let stranger = fixture.id(Who::Stranger);
        fixture
            .request_to_join(Who::Member, charter_id(1), "let me in")
            .await;

        let (_, voted) = fixture
            .create(
                Who::Founder,
                "vote",
                &[
                    ("submittedCharterId", id_value(charter_id(1))),
                    ("voterId", id_value(member)),
                ],
            )
            .await;
        assert_matches!(
            voted,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );

        let (_, not_a_member) = fixture
            .create(
                Who::Founder,
                "vote",
                &[
                    ("submittedCharterId", id_value(charter_id(1))),
                    ("voterId", id_value(stranger)),
                ],
            )
            .await;
        assert_not_found(not_a_member, "voterId", stranger);
    }

    /// The lookup is billed like the id fetch it replaces: one document query
    /// whose processing cost lands in the execution context.
    #[tokio::test]
    async fn should_bill_the_lookup_as_a_document_fetch() {
        let mut fixture = LookupFixture::new();
        let platform_version = PlatformVersion::latest();
        let member = fixture.id(Who::Member);
        let founder = fixture.id(Who::Founder);
        let stranger = fixture.id(Who::Stranger);
        fixture
            .request_to_join(Who::Member, charter_id(1), "let me in")
            .await;

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
            document_type_name: "charter".to_string(),
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
        let validate = |data: BTreeMap<String, Value>| {
            let mut execution_context =
                StateTransitionExecutionContext::default_for_platform_version(platform_version)
                    .expect("expected an execution context");
            let result = base
                .validate_document_references(
                    &data,
                    founder,
                    // The charter type records no creator ids
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
            (result, execution_context)
        };

        let charter = |member_id: Option<Identifier>| {
            let mut data = BTreeMap::from([
                ("submittedCharterId".to_string(), id_value(charter_id(1))),
                ("title".to_string(), Value::Text("charter".to_string())),
            ]);
            if let Some(member_id) = member_id {
                data.insert("memberId".to_string(), id_value(member_id));
            }
            data
        };

        let (result, execution_context) = validate(charter(Some(member)));
        assert!(result.is_valid(), "{:?}", result.errors);
        assert_matches!(
            execution_context.operations_slice(),
            [ValidationOperation::PrecalculatedOperation(fee)] if fee.processing_fee > 0,
            "the lookup query is the one billed operation"
        );

        // A refused lookup is billed the same way
        let (result, execution_context) = validate(charter(Some(stranger)));
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::StateError(
                StateError::ReferencedEntityNotFoundError(_)
            )]
        );
        assert_matches!(
            execution_context.operations_slice(),
            [ValidationOperation::PrecalculatedOperation(fee)] if fee.processing_fee > 0
        );

        // Without the reference there is nothing to look up
        let (result, execution_context) = validate(charter(None));
        assert!(result.is_valid());
        assert!(execution_context.operations_slice().is_empty());
    }
}
