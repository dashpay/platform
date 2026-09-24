//! Reference expressions (`refersTo: { "anyOf": [...] }` / `{ "allOf": [...] }`,
//! nestable, protocol version 14) through the full ABCI pipeline. The
//! fixture's `resignation` names members of the moderation charter it is for,
//! where a member is the owner of a `joinRequest` for the charter or the
//! `moderatorId` of an `addedModerator` document for it:
//!
//! - `memberId`: `anyOf` of the two lookups, and `members` the same on the
//!   elements of a typed array;
//! - `signerOrRequestId`: `anyOf` of an identity and a join request's id;
//! - `agreedMemberId`: the two lookups, a `propertyAgreement` on the first alone;
//! - `vettedMemberId`: `allOf` of the two lookups, both must hold;
//! - `identifiedMemberId`: `allOf(identity, anyOf(joinRequest, addedModerator))`;
//! - `deepMemberId`: four combinators deep,
//!   `anyOf(addedModerator, allOf(identity, anyOf(joinRequest, allOf(identity, addedModerator))))`.
//!
//! An `anyOf` checks its operands in declared order and stops at the first
//! that holds, refusing with the last operand's error when none does; an
//! `allOf` stops at the first that fails and refuses with its error. Every
//! read is billed, the failed operands' included. A refusal is paid.

use super::*;

mod reference_expression_tests {
    use super::super::reference_test_setup::{
        create_document, register_contract_at, replace_document,
    };
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
    use dpp::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
    use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
    use dpp::validation::SimpleConsensusValidationResult;
    use dpp::version::DefaultForPlatformVersion;
    use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::{DocumentBaseTransitionAction, DocumentBaseTransitionActionV0};
    use simple_signer::signer::SimpleSigner;
    use std::collections::BTreeMap;

    const CONTRACT_PATH: &str = "tests/supporting_files/contract/reference-validation/reference-validation-contract-reference-expression.json";

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

    impl Writer {
        fn take_nonce(&mut self) -> IdentityNonce {
            let nonce = self.next_nonce;
            self.next_nonce += 1;
            nonce
        }
    }

    struct ExpressionFixture {
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

    impl ExpressionFixture {
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
            // every leaf run on the fixture too
            let contract = register_contract_at(
                &platform,
                CONTRACT_PATH,
                founder.identity.id(),
                true,
                platform_version,
            );

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

        /// Creates a `type_name` document owned by `who`, with `values` set
        /// over the random required ones, through the shared harness.
        async fn create(
            &mut self,
            who: Who,
            type_name: &str,
            values: &[(&str, Value)],
        ) -> (Document, StateTransitionExecutionResult) {
            let platform_version = PlatformVersion::latest();
            let platform_state = self.platform.state.load();
            let owner = self.id(who);
            let writer = match who {
                Who::Founder => &mut self.founder,
                Who::Member => &mut self.member,
                Who::Moderator => &mut self.moderator,
                Who::Stranger => &mut self.stranger,
            };
            let nonce = writer.take_nonce();
            let (document, result) = create_document(
                &self.platform,
                &platform_state,
                &self.contract,
                type_name,
                values,
                owner,
                &writer.key,
                nonce,
                &writer.signer,
                &mut self.rng,
                platform_version,
            )
            .await;
            (document, result.into_execution_results().remove(0))
        }

        /// Replaces the founder's `resignation`, as last accepted, with
        /// `change` applied.
        async fn replace(
            &mut self,
            resignation: &Document,
            change: impl FnOnce(&mut Document),
        ) -> StateTransitionExecutionResult {
            let platform_version = PlatformVersion::latest();
            let platform_state = self.platform.state.load();
            let mut replacement = resignation.clone();
            replacement
                .increment_revision()
                .expect("the revision increments");
            change(&mut replacement);
            let nonce = self.founder.take_nonce();
            replace_document(
                &self.platform,
                &platform_state,
                &self.contract,
                "resignation",
                &replacement,
                &self.founder.key,
                nonce,
                &self.founder.signer,
                platform_version,
            )
            .await
            .into_execution_results()
            .remove(0)
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
            assert_successful(&result);
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
            assert_successful(&result);
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

        /// The member has joined charter 1, the moderator was added to it,
        /// and the stranger is a moderator of charter 2 only.
        async fn with_members() -> (Self, Identifier, Identifier, Identifier) {
            let mut fixture = Self::new();
            let member = fixture.id(Who::Member);
            let moderator = fixture.id(Who::Moderator);
            let stranger = fixture.id(Who::Stranger);
            fixture
                .request_to_join(Who::Member, charter_id(1), "let me in")
                .await;
            fixture.add_moderator(charter_id(1), moderator).await;
            fixture.add_moderator(charter_id(2), stranger).await;
            (fixture, member, moderator, stranger)
        }
    }

    fn assert_successful(result: &StateTransitionExecutionResult) {
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. },
            "{result:?}"
        );
    }

    /// The refusal of a value whose deciding leaf is a lookup into
    /// `document_type` that found nothing: the error that leaf gives alone,
    /// naming the property (or element) and the value.
    fn assert_refused_by_lookup(
        result: StateTransitionExecutionResult,
        path: &str,
        value: Identifier,
        document_type: &str,
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
                    } if document_type_name == document_type
                ),
            "expected the {document_type} lookup's 40120 at {path}"
        );
    }

    /// The refusal of a value whose deciding leaf is `identity`.
    fn assert_refused_as_no_identity(
        result: StateTransitionExecutionResult,
        path: &str,
        value: Identifier,
    ) {
        assert_matches!(
            result,
            PaidConsensusError {
                error: ConsensusError::StateError(StateError::ReferencedEntityNotFoundError(e)),
                ..
            } if e.path() == path
                && *e.entity_id() == value
                && matches!(e.entity_type(), DocumentPropertyReferenceTarget::Identity),
            "expected the identity leaf's 40120 at {path}"
        );
    }

    #[tokio::test]
    async fn should_accept_a_value_the_first_operand_of_an_any_of_holds_for() {
        let (mut fixture, member, _, _) = ExpressionFixture::with_members().await;
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
    async fn should_accept_a_value_only_the_second_operand_of_an_any_of_holds_for() {
        let (mut fixture, _, moderator, _) = ExpressionFixture::with_members().await;
        // The moderator never asked to join: only the addedModerator holds
        let (_, result) = fixture
            .resign(
                charter_id(1),
                "resigning",
                &[("memberId", id_value(moderator))],
            )
            .await;
        assert_successful(&result);
    }

    /// No operand holds: the write is refused, paid, with the error of the
    /// LAST operand, the `addedModerator` lookup, so the order the author
    /// declared decides which failure a writer is shown.
    #[tokio::test]
    async fn should_refuse_a_value_no_operand_of_an_any_of_holds_for_with_the_last_error() {
        let (mut fixture, member, _, stranger) = ExpressionFixture::with_members().await;
        // A moderator of another charter is no moderator of this one
        let (_, result) = fixture
            .resign(
                charter_id(1),
                "resigning",
                &[("memberId", id_value(stranger))],
            )
            .await;
        assert_refused_by_lookup(result, "memberId", stranger, "addedModerator");

        // A member of another charter is no member of this one either
        let (_, result) = fixture
            .resign(
                charter_id(2),
                "resigning",
                &[("memberId", id_value(member))],
            )
            .await;
        assert_refused_by_lookup(result, "memberId", member, "addedModerator");
    }

    /// Each element of a typed array meets the expression on its own; the
    /// first element it does not hold for refuses the write, named by its
    /// list path.
    #[tokio::test]
    async fn should_check_every_element_against_the_expression_on_its_own() {
        let (mut fixture, member, moderator, stranger) = ExpressionFixture::with_members().await;
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
        assert_refused_by_lookup(result, "members[2]", stranger, "addedModerator");
    }

    /// An identity or a document id: each leaf reads its own kind of entity,
    /// and a value neither holds for gets the last leaf's error, the
    /// document's.
    #[tokio::test]
    async fn should_accept_an_identity_or_a_document_id_and_refuse_neither() {
        let mut fixture = ExpressionFixture::new();
        let founder = fixture.id(Who::Founder);
        let request = fixture
            .request_to_join(Who::Member, charter_id(1), "let me in")
            .await;

        for value in [founder, request.id()] {
            let (_, result) = fixture
                .resign(
                    charter_id(1),
                    "resigning",
                    &[("signerOrRequestId", id_value(value))],
                )
                .await;
            assert_successful(&result);
        }

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

    /// A `propertyAgreement` belongs to its leaf: the first leaf's agreement
    /// fails a resignation whose title is not the join request's message, the
    /// second leaf has none, and when neither holds the error is the second
    /// leaf's, not the first leaf's agreement mismatch.
    #[tokio::test]
    async fn should_check_an_agreement_only_against_its_own_leafs_document() {
        let (mut fixture, member, moderator, _) = ExpressionFixture::with_members().await;

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
        assert_refused_by_lookup(result, "agreedMemberId", member, "addedModerator");

        let (_, result) = fixture
            .resign(
                charter_id(1),
                "let me out",
                &[("agreedMemberId", id_value(moderator))],
            )
            .await;
        assert_successful(&result);
    }

    /// Every operand of an `allOf` must hold for the same value: a member who
    /// both asked to join and was added passes, one who did only either is
    /// refused with the error of the first operand that fails.
    #[tokio::test]
    async fn should_require_every_operand_of_an_all_of_and_refuse_with_the_first_failure() {
        let (mut fixture, member, moderator, _) = ExpressionFixture::with_members().await;

        // The member only asked to join: the joinRequest holds, the
        // addedModerator, second, fails
        let (_, result) = fixture
            .resign(
                charter_id(1),
                "resigning",
                &[("vettedMemberId", id_value(member))],
            )
            .await;
        assert_refused_by_lookup(result, "vettedMemberId", member, "addedModerator");

        // The moderator never asked to join: the first operand fails and ends
        // the check
        let (_, result) = fixture
            .resign(
                charter_id(1),
                "resigning",
                &[("vettedMemberId", id_value(moderator))],
            )
            .await;
        assert_refused_by_lookup(result, "vettedMemberId", moderator, "joinRequest");

        // Once added, the member meets both
        fixture.add_moderator(charter_id(1), member).await;
        let (_, result) = fixture
            .resign(
                charter_id(1),
                "resigning",
                &[("vettedMemberId", id_value(member))],
            )
            .await;
        assert_successful(&result);
    }

    /// Operands nest: `allOf(identity, anyOf(joinRequest, addedModerator))`.
    /// A value that is no identity fails the first operand, whose error it
    /// gets; an identity that is no member fails the nested `anyOf`, whose
    /// error is its last operand's.
    #[tokio::test]
    async fn should_evaluate_a_nested_expression_operand_by_operand() {
        let (mut fixture, member, moderator, stranger) = ExpressionFixture::with_members().await;

        for value in [member, moderator] {
            let (_, result) = fixture
                .resign(
                    charter_id(1),
                    "resigning",
                    &[("identifiedMemberId", id_value(value))],
                )
                .await;
            assert_successful(&result);
        }

        let nobody = Identifier::from([0x5B; 32]);
        let (_, result) = fixture
            .resign(
                charter_id(1),
                "resigning",
                &[("identifiedMemberId", id_value(nobody))],
            )
            .await;
        assert_refused_as_no_identity(result, "identifiedMemberId", nobody);

        let (_, result) = fixture
            .resign(
                charter_id(1),
                "resigning",
                &[("identifiedMemberId", id_value(stranger))],
            )
            .await;
        assert_refused_by_lookup(result, "identifiedMemberId", stranger, "addedModerator");
    }

    /// Four combinators deep, the registration limit:
    /// `anyOf(addedModerator, allOf(identity, anyOf(joinRequest, allOf(identity, addedModerator))))`.
    #[tokio::test]
    async fn should_evaluate_an_expression_at_the_depth_limit() {
        let (mut fixture, member, moderator, stranger) = ExpressionFixture::with_members().await;
        let deep = fixture
            .contract
            .document_type_for_name("resignation")
            .expect("the resignation type")
            .flattened_properties()
            .get("deepMemberId")
            .and_then(|property| property.property_type.reference())
            .and_then(|reference| reference.target())
            .map(|target| target.expression_depth());
        assert_eq!(
            deep,
            Some(usize::from(
                PlatformVersion::latest()
                    .system_limits
                    .max_reference_expression_depth
            )),
            "the fixture's deepMemberId is written at the limit"
        );

        // The moderator through the first operand, the member three levels
        // down through the joinRequest
        for value in [moderator, member] {
            let (_, result) = fixture
                .resign(
                    charter_id(1),
                    "resigning",
                    &[("deepMemberId", id_value(value))],
                )
                .await;
            assert_successful(&result);
        }

        // The stranger fails every branch; the deciding error is the one the
        // evaluation ends on: the innermost allOf's addedModerator, the last
        // operand of the anyOf it is the last operand of, and so on up
        let (_, result) = fixture
            .resign(
                charter_id(1),
                "resigning",
                &[("deepMemberId", id_value(stranger))],
            )
            .await;
        assert_refused_by_lookup(result, "deepMemberId", stranger, "addedModerator");
    }

    /// A replace re-validates an expression when a property one of its leaves
    /// reads changed, and leaves it alone otherwise.
    #[tokio::test]
    async fn should_revalidate_an_expression_when_a_key_part_one_leaf_reads_changes() {
        let (mut fixture, member, _, _) = ExpressionFixture::with_members().await;
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
        assert_refused_by_lookup(result, "memberId", member, "addedModerator");
    }

    /// The document reference validation of `resignation` data holding
    /// `values`, as a create by the founder, with the billed operations'
    /// processing fees.
    fn validate_directly(
        fixture: &ExpressionFixture,
        founder: Identifier,
        values: &[(&str, Value)],
        platform_version: &PlatformVersion,
    ) -> (SimpleConsensusValidationResult, Vec<u64>) {
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
        let mut data = BTreeMap::from([
            ("submittedCharterId".to_string(), id_value(charter_id(1))),
            ("title".to_string(), Value::Text("resigning".to_string())),
        ]);
        data.extend(
            values
                .iter()
                .map(|(property, value)| (property.to_string(), value.clone())),
        );
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
            .collect();
        (result, processing_fees)
    }

    /// Every read is billed, those of the operands that failed included, and
    /// only the reads the evaluation makes: an `anyOf` value the second
    /// operand holds for pays for the first operand's query too, and an
    /// `allOf` that fails its first operand stops there.
    #[tokio::test]
    async fn should_bill_the_reads_of_the_operands_that_failed_and_no_others() {
        let (fixture, member, moderator, stranger) = ExpressionFixture::with_members().await;
        let platform_version = PlatformVersion::latest();
        let mut fixture = fixture;
        let founder = fixture.id(Who::Founder);

        let validate = |values: &[(&str, Value)]| {
            validate_directly(&fixture, founder, values, platform_version)
        };

        // anyOf: the first operand holds, one query
        let (result, first_holds) = validate(&[("memberId", id_value(member))]);
        assert!(result.is_valid(), "{:?}", result.errors);
        assert_eq!(first_holds.len(), 1, "the joinRequest query");

        // anyOf: only the second holds, the first operand's failed query is
        // billed too
        let (result, second_holds) = validate(&[("memberId", id_value(moderator))]);
        assert!(result.is_valid(), "{:?}", result.errors);
        assert_eq!(
            second_holds.len(),
            2,
            "the failed joinRequest query and the addedModerator query"
        );
        assert!(second_holds.iter().all(|fee| *fee > 0));
        assert!(
            second_holds.iter().sum::<u64>() > first_holds.iter().sum::<u64>(),
            "a value the second operand holds for costs more than one the first holds for"
        );

        // anyOf: neither holds, both queries are billed and the write refused
        let (result, neither_holds) = validate(&[("memberId", id_value(stranger))]);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::StateError(
                StateError::ReferencedEntityNotFoundError(_)
            )]
        );
        assert_eq!(neither_holds.len(), 2);

        // allOf: the first operand fails and ends the check, one query
        let (result, first_fails) = validate(&[("vettedMemberId", id_value(moderator))]);
        assert!(!result.is_valid());
        assert_eq!(first_fails.len(), 1, "only the failed joinRequest query");

        // allOf: the first holds and the second fails, both queries
        let (result, second_fails) = validate(&[("vettedMemberId", id_value(member))]);
        assert!(!result.is_valid());
        assert_eq!(second_fails.len(), 2);
    }

    /// At the last shipped protocol version the parser ignores `refersTo`, so
    /// a contract declaring expressions carries no reference at all, and the
    /// document reference validation (generation 0, which that version also
    /// selects) reads and bills nothing for it, whatever the values: the
    /// in-place changes to that generation are unreachable there. The typed
    /// array is left out, since typed arrays do not parse before version 14.
    #[test]
    fn should_validate_no_expression_at_protocol_version_13() {
        let platform_version = PlatformVersion::get(13).expect("protocol version 13 exists");
        let platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(13)
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut schema: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(CONTRACT_PATH).expect("the fixture reads"),
        )
        .expect("the fixture is JSON");
        schema["documentSchemas"]["resignation"]["properties"]
            .as_object_mut()
            .expect("an object")
            .remove("members");
        let contract = DataContract::from_value(
            dpp::platform_value::to_value(schema).expect("converts"),
            false,
            platform_version,
        )
        .expect("protocol version 13 parses the contract, ignoring refersTo");
        assert!(contract
            .document_type_for_name("resignation")
            .expect("the resignation type")
            .flattened_properties()
            .values()
            .all(|property| property.property_type.reference().is_none()));
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

        let (_, contract_fetch_info) = platform
            .drive
            .get_contract_with_fetch_info_and_fee(
                contract.id().to_buffer(),
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
        let platform_state = platform.state.load();
        let platform_ref = PlatformStateRef {
            drive: &platform.drive,
            state: &platform_state,
            config: &platform.config,
        };
        let nobody = id_value(Identifier::from([0x5C; 32]));
        let data = BTreeMap::from([
            ("submittedCharterId".to_string(), id_value(charter_id(1))),
            ("title".to_string(), Value::Text("resigning".to_string())),
            ("memberId".to_string(), nobody.clone()),
            ("vettedMemberId".to_string(), nobody.clone()),
            ("deepMemberId".to_string(), nobody),
        ]);
        let mut execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)
                .expect("expected an execution context");
        let result = base
            .validate_document_references(
                &data,
                Identifier::from([0x5D; 32]),
                None,
                None,
                None,
                &platform_ref,
                &BlockInfo::default(),
                None,
                &mut execution_context,
                platform_version,
            )
            .expect("expected the references to be validated");
        assert!(result.is_valid(), "{:?}", result.errors);
        assert!(execution_context.operations_slice().is_empty());
    }
}
