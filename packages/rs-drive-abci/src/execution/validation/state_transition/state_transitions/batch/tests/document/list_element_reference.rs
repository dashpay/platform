//! References to an element of a list of a referenced document (`refersTo:
//! listElement`, protocol version 14) through the full ABCI pipeline. In the
//! fixture an `electedCharter` holds its `members`, and it can never be
//! deleted or replaced. A `resignation` names its charter by id
//! (`electedCharterId`), and `memberId` must be one of that charter's
//! members; each of `witnesses`, a typed array, must be too. `lookedUpMemberId`
//! reads its list through `lookedUpCharterId`, which finds the charter through a
//! unique index rather than by id, and `ownedCharterMemberId` through
//! `ownedCharterId`, a lookup keyed by the charter's owner and the
//! resignation's `charterKey`. `metaMemberId` reads the charter's nested
//! `seats.members` through the nested `meta.charterId`. The second contract's
//! `ballot` does the same with a charter of the first contract.
//!
//! A value the list does not hold is refused, paid, with
//! `ReferencedEntityNotFoundError` (40120) naming the property, or the element
//! by its list path, and the list element declaration as the entity type.

use super::*;

mod list_element_reference_tests {
    use super::*;
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
    use dpp::prelude::{DataContract, IdentityNonce};
    use dpp::state_transition::StateTransition;
    use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
    use dpp::version::DefaultForPlatformVersion;
    use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::{DocumentBaseTransitionAction, DocumentBaseTransitionActionV0};
    use simple_signer::signer::SimpleSigner;
    use std::collections::BTreeMap;

    const CONTRACT_PATH: &str = "tests/supporting_files/contract/reference-validation/reference-validation-contract-list-element.json";

    /// A `ballot` type whose `voterId` must be a member of a charter of the
    /// contract above: the list in another contract.
    const BALLOT_CONTRACT_PATH: &str = "tests/supporting_files/contract/reference-validation/reference-validation-contract-list-element-registration-foreign-valid.json";

    /// The identities of the fixture: the `Founder` writes charters,
    /// resignations and ballots; the `Member` and the `Stranger` are listed,
    /// or not, in them.
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

    struct ListElementFixture {
        platform: TempPlatform<MockCoreRPCLike>,
        contract: DataContract,
        ballot_contract: DataContract,
        rng: StdRng,
        founder: Writer,
        member: Writer,
        stranger: Writer,
    }

    fn id_value(id: Identifier) -> Value {
        Value::Identifier(id.to_buffer())
    }

    fn ids(ids: &[Identifier]) -> Value {
        Value::Array(ids.iter().copied().map(id_value).collect())
    }

    impl ListElementFixture {
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
            let founder = writer(1958);
            let member = writer(1959);
            let stranger = writer(1960);

            // Parsed with full validation, so the list checks run on the
            // fixtures too
            let [contract, ballot_contract] = [CONTRACT_PATH, BALLOT_CONTRACT_PATH].map(|path| {
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
                ballot_contract,
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

        /// The document type named `type_name`, in whichever fixture contract
        /// has it, with the contract's id, `who`'s writer and the random source:
        /// the parts a transition is built from, borrowed at once.
        fn parts(
            &mut self,
            who: Who,
            type_name: &str,
        ) -> (DocumentTypeRef<'_>, Identifier, &mut Writer, &mut StdRng) {
            let (document_type, contract_id) = [&self.contract, &self.ballot_contract]
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

        /// Creates a `type_name` document owned by the founder, with `values`
        /// set over the random required ones, and returns it with the result.
        async fn create(
            &mut self,
            type_name: &str,
            values: &[(&str, Value)],
        ) -> (Document, StateTransitionExecutionResult) {
            let platform_version = PlatformVersion::latest();
            let owner_id = self.id(Who::Founder);
            let (document_type, contract_id, writer, rng) = self.parts(Who::Founder, type_name);
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

        /// Replaces `document`, as last accepted, with `change` applied.
        async fn replace(
            &mut self,
            type_name: &str,
            document: &Document,
            change: impl FnOnce(&mut Document),
        ) -> (Document, StateTransitionExecutionResult) {
            let platform_version = PlatformVersion::latest();
            let mut replacement = document.clone();
            replacement
                .increment_revision()
                .expect("the revision increments");
            change(&mut replacement);
            let (document_type, contract_id, writer, _) = self.parts(Who::Founder, type_name);
            let nonce = writer.next_nonce(contract_id);
            let transition = BatchTransition::new_document_replacement_transition_from_document(
                replacement.clone(),
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
            let result = self.process(&transition);
            (replacement, result)
        }

        /// An elected charter for the submitted charter `submitted_charter_id`
        /// seating `members`; returns its id.
        async fn seat_charter(
            &mut self,
            submitted_charter_id: Identifier,
            members: &[Identifier],
        ) -> Identifier {
            let (charter, result) = self
                .create(
                    "electedCharter",
                    &[
                        ("submittedCharterId", id_value(submitted_charter_id)),
                        ("members", ids(members)),
                        (
                            "seats",
                            Value::Map(vec![(Value::Text("members".to_string()), ids(members))]),
                        ),
                    ],
                )
                .await;
            assert_succeeded(result);
            charter.id()
        }

        async fn resign(
            &mut self,
            values: &[(&str, Value)],
        ) -> (Document, StateTransitionExecutionResult) {
            self.create("resignation", values).await
        }
    }

    fn submitted_charter_id(byte: u8) -> Identifier {
        Identifier::from([byte; 32])
    }

    fn assert_succeeded(result: StateTransitionExecutionResult) {
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
    }

    /// The refusal of a value the list does not hold, naming the property (or
    /// the element) and the value, with the list element declaration.
    fn assert_not_listed(result: StateTransitionExecutionResult, path: &str, value: Identifier) {
        assert_matches!(
            result,
            PaidConsensusError {
                error: ConsensusError::StateError(StateError::ReferencedEntityNotFoundError(e)),
                ..
            } if e.path() == path
                && *e.entity_id() == value
                && matches!(e.entity_type(), DocumentPropertyReferenceTarget::ListElement(_)),
            "expected 40120 at {path}"
        );
    }

    #[tokio::test]
    async fn should_accept_a_value_the_referenced_list_holds() {
        let mut fixture = ListElementFixture::new();
        let member = fixture.id(Who::Member);
        let charter = fixture
            .seat_charter(submitted_charter_id(1), &[member])
            .await;

        let (_, result) = fixture
            .resign(&[
                ("electedCharterId", id_value(charter)),
                ("memberId", id_value(member)),
            ])
            .await;

        assert_succeeded(result);
    }

    #[tokio::test]
    async fn should_refuse_a_value_the_referenced_list_does_not_hold() {
        let mut fixture = ListElementFixture::new();
        let member = fixture.id(Who::Member);
        let stranger = fixture.id(Who::Stranger);
        let charter = fixture
            .seat_charter(submitted_charter_id(1), &[member])
            .await;
        // The stranger sits on another charter, which is not the one named
        fixture
            .seat_charter(submitted_charter_id(2), &[stranger])
            .await;

        let (_, result) = fixture
            .resign(&[
                ("electedCharterId", id_value(charter)),
                ("memberId", id_value(stranger)),
            ])
            .await;

        assert_not_listed(result, "memberId", stranger);
    }

    #[tokio::test]
    async fn should_refuse_a_value_set_while_the_document_property_is_not() {
        let mut fixture = ListElementFixture::new();
        let member = fixture.id(Who::Member);
        fixture
            .seat_charter(submitted_charter_id(1), &[member])
            .await;

        // No charter is named, so no list holds the member
        let (_, result) = fixture.resign(&[("memberId", id_value(member))]).await;

        assert_not_listed(result, "memberId", member);
    }

    #[tokio::test]
    async fn should_check_every_element_of_a_typed_array_against_the_list() {
        let mut fixture = ListElementFixture::new();
        let founder = fixture.id(Who::Founder);
        let member = fixture.id(Who::Member);
        let stranger = fixture.id(Who::Stranger);
        let charter = fixture
            .seat_charter(submitted_charter_id(1), &[founder, member])
            .await;

        let (_, all_listed) = fixture
            .resign(&[
                ("electedCharterId", id_value(charter)),
                ("witnesses", ids(&[founder, member])),
            ])
            .await;
        assert_succeeded(all_listed);

        // One element the list does not hold refuses the write, the error
        // naming the element by its list path
        let (_, one_unlisted) = fixture
            .resign(&[
                ("electedCharterId", id_value(charter)),
                ("witnesses", ids(&[member, stranger])),
            ])
            .await;
        assert_not_listed(one_unlisted, "witnesses[1]", stranger);
    }

    #[tokio::test]
    async fn should_refuse_a_replace_that_points_the_document_property_at_a_charter_not_listing_the_value(
    ) {
        let mut fixture = ListElementFixture::new();
        let founder = fixture.id(Who::Founder);
        let member = fixture.id(Who::Member);
        let stranger = fixture.id(Who::Stranger);
        let charter = fixture
            .seat_charter(submitted_charter_id(1), &[member, founder])
            .await;
        let other_charter = fixture
            .seat_charter(submitted_charter_id(2), &[stranger])
            .await;
        let (resignation, result) = fixture
            .resign(&[
                ("electedCharterId", id_value(charter)),
                ("memberId", id_value(member)),
                ("witnesses", ids(&[member])),
            ])
            .await;
        assert_succeeded(result);

        // Touching neither the value nor its charter leaves the reference alone
        let (resignation, untouched) = fixture
            .replace("resignation", &resignation, |resignation| {
                resignation.set("reason", "moving on".into());
            })
            .await;
        assert_succeeded(untouched);

        // Pointing the charter elsewhere re-validates the member against the
        // other charter's list, which does not hold it (`memberId` is judged
        // before `witnesses`, in declaration order)
        let (_, repointed) = fixture
            .replace("resignation", &resignation, |resignation| {
                resignation.set("electedCharterId", id_value(other_charter));
            })
            .await;
        assert_not_listed(repointed, "memberId", member);

        // Changing the value to one the charter does not list is refused too
        let (_, changed) = fixture
            .replace("resignation", &resignation, |resignation| {
                resignation.set("memberId", id_value(stranger));
            })
            .await;
        assert_not_listed(changed, "memberId", stranger);

        // and to one it lists is accepted: the charter is fetched through the
        // untouched `electedCharterId`, whose reference is validated with it
        let (resignation, changed_listed) = fixture
            .replace("resignation", &resignation, |resignation| {
                resignation.set("memberId", id_value(founder));
            })
            .await;
        assert_succeeded(changed_listed);

        // Pointing the charter elsewhere re-validates EVERY element, not only
        // the ones the stored list did not hold
        let (_, repointed_elements) = fixture
            .replace("resignation", &resignation, |resignation| {
                resignation.remove("memberId");
                resignation.set("electedCharterId", id_value(other_charter));
            })
            .await;
        assert_not_listed(repointed_elements, "witnesses[0]", member);

        // Repointing with values the other charter lists is accepted
        let (_, repointed_listed) = fixture
            .replace("resignation", &resignation, |resignation| {
                resignation.set("electedCharterId", id_value(other_charter));
                resignation.set("memberId", id_value(stranger));
                resignation.set("witnesses", ids(&[stranger]));
            })
            .await;
        assert_succeeded(repointed_listed);
    }

    #[tokio::test]
    async fn should_read_the_list_of_a_charter_found_through_a_lookup() {
        let mut fixture = ListElementFixture::new();
        let member = fixture.id(Who::Member);
        let stranger = fixture.id(Who::Stranger);
        fixture
            .seat_charter(submitted_charter_id(1), &[member])
            .await;

        // `lookedUpCharterId` names the charter by the submitted charter it
        // was elected for
        let (_, listed) = fixture
            .resign(&[
                ("lookedUpCharterId", id_value(submitted_charter_id(1))),
                ("lookedUpMemberId", id_value(member)),
            ])
            .await;
        assert_succeeded(listed);

        let (_, unlisted) = fixture
            .resign(&[
                ("lookedUpCharterId", id_value(submitted_charter_id(1))),
                ("lookedUpMemberId", id_value(stranger)),
            ])
            .await;
        assert_not_listed(unlisted, "lookedUpMemberId", stranger);
    }

    /// A replace that moves `ownedCharterId`'s lookup onto another charter
    /// (its key reads `charterKey`, which the replace changes) checks the list
    /// element again against the other charter's list, though neither the
    /// value nor `ownedCharterId` itself changed.
    #[tokio::test]
    async fn should_check_a_list_element_again_when_a_lookup_key_moves_its_charter() {
        let mut fixture = ListElementFixture::new();
        let founder = fixture.id(Who::Founder);
        let member = fixture.id(Who::Member);
        let stranger = fixture.id(Who::Stranger);
        fixture
            .seat_charter(submitted_charter_id(1), &[member])
            .await;
        fixture
            .seat_charter(submitted_charter_id(2), &[stranger])
            .await;

        // Both charters are the founder's: `charterKey` picks one
        let (resignation, result) = fixture
            .resign(&[
                ("charterKey", id_value(submitted_charter_id(1))),
                ("ownedCharterId", id_value(founder)),
                ("ownedCharterMemberId", id_value(member)),
            ])
            .await;
        assert_succeeded(result);

        let (_, moved) = fixture
            .replace("resignation", &resignation, |resignation| {
                resignation.set("charterKey", id_value(submitted_charter_id(2)));
            })
            .await;
        assert_not_listed(moved, "ownedCharterMemberId", member);
    }

    /// `metaMemberId` reads `seats.members` through `meta.charterId`: nested
    /// paths on both sides, and a replace of the whole `meta` object moves it.
    #[tokio::test]
    async fn should_read_a_nested_list_through_a_nested_document_property() {
        let mut fixture = ListElementFixture::new();
        let member = fixture.id(Who::Member);
        let stranger = fixture.id(Who::Stranger);
        let charter = fixture
            .seat_charter(submitted_charter_id(1), &[member])
            .await;
        let other_charter = fixture
            .seat_charter(submitted_charter_id(2), &[stranger])
            .await;
        let meta = |charter: Identifier| {
            Value::Map(vec![(
                Value::Text("charterId".to_string()),
                id_value(charter),
            )])
        };

        let (_, unlisted) = fixture
            .resign(&[
                ("meta", meta(charter)),
                ("metaMemberId", id_value(stranger)),
            ])
            .await;
        assert_not_listed(unlisted, "metaMemberId", stranger);

        let (resignation, listed) = fixture
            .resign(&[("meta", meta(charter)), ("metaMemberId", id_value(member))])
            .await;
        assert_succeeded(listed);

        let (_, moved) = fixture
            .replace("resignation", &resignation, |resignation| {
                resignation.set("meta", meta(other_charter));
            })
            .await;
        assert_not_listed(moved, "metaMemberId", member);
    }

    /// A list in another contract is read from that contract's document.
    #[tokio::test]
    async fn should_read_the_list_of_a_charter_of_another_contract() {
        let mut fixture = ListElementFixture::new();
        let member = fixture.id(Who::Member);
        let stranger = fixture.id(Who::Stranger);
        let charter = fixture
            .seat_charter(submitted_charter_id(1), &[member])
            .await;

        let (_, listed) = fixture
            .create(
                "ballot",
                &[
                    ("electedCharterId", id_value(charter)),
                    ("voterId", id_value(member)),
                ],
            )
            .await;
        assert_succeeded(listed);

        let (_, unlisted) = fixture
            .create(
                "ballot",
                &[
                    ("electedCharterId", id_value(charter)),
                    ("voterId", id_value(stranger)),
                ],
            )
            .await;
        assert_not_listed(unlisted, "voterId", stranger);
    }

    /// The list check reads the charter `electedCharterId`'s own reference
    /// fetched: a write with list elements is billed exactly what the same
    /// write without them is, one document fetch.
    #[tokio::test]
    async fn should_bill_no_read_for_the_list_check() {
        let mut fixture = ListElementFixture::new();
        let platform_version = PlatformVersion::latest();
        let founder = fixture.id(Who::Founder);
        let member = fixture.id(Who::Member);
        let stranger = fixture.id(Who::Stranger);
        let charter = fixture
            .seat_charter(submitted_charter_id(1), &[founder, member])
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
        let validate = |values: &[(&str, Value)]| {
            let data: BTreeMap<String, Value> = values
                .iter()
                .map(|(property, value)| (property.to_string(), value.clone()))
                .collect();
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
            (result, execution_context.operations_slice().to_vec())
        };

        let (result, charter_only) = validate(&[("electedCharterId", id_value(charter))]);
        assert!(result.is_valid(), "{:?}", result.errors);
        assert_eq!(charter_only.len(), 1, "one fetch, the charter");

        let (result, with_list_elements) = validate(&[
            ("electedCharterId", id_value(charter)),
            ("memberId", id_value(member)),
            ("witnesses", ids(&[founder, member])),
        ]);
        assert!(result.is_valid(), "{:?}", result.errors);
        assert_eq!(
            with_list_elements, charter_only,
            "the list elements add no read to the charter's fetch"
        );

        // A refused one is billed the same
        let (result, refused) = validate(&[
            ("electedCharterId", id_value(charter)),
            ("memberId", id_value(stranger)),
        ]);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::StateError(
                StateError::ReferencedEntityNotFoundError(_)
            )]
        );
        assert_eq!(refused, charter_only);
    }
}
