//! End-to-end coverage for the `propertyConstraints` doctype keyword (protocol
//! version 14): a document type names rules its documents' integer properties
//! must meet, each a comparison of two integer expressions. A create or replace
//! that breaks one is consensus-rejected with
//! `DocumentPropertyConstraintViolatedError` (basic code 10422), naming the rule
//! and why, and leaves the stored document untouched. A property the document
//! leaves out counts as 0, or as its `ifAbsent` value.

use super::*;

mod property_constraints_tests {
    use super::*;
    use crate::execution::validation::state_transition::batch::action_validation::document::document_replace_transition_action::DocumentReplaceTransitionActionValidation;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TempPlatform;
    use dpp::consensus::basic::document::PropertyConstraintViolation;
    use dpp::consensus::basic::BasicError;
    use dpp::consensus::codes::ErrorWithCode;
    use dpp::data_contract::schema::DataContractSchemaMethodsV0;
    use dpp::document::Document;
    use dpp::document::DocumentV0Setters;
    use dpp::identity::{Identity, IdentityPublicKey};
    use dpp::platform_value::platform_value;
    use dpp::prelude::{DataContract, Identifier, IdentityNonce};
    use dpp::state_transition::StateTransition;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
    use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::{DocumentBaseTransitionAction, DocumentBaseTransitionActionV0};
    use drive::state_transition_action::batch::batched_transition::document_transition::document_replace_transition_action::{DocumentReplaceTransitionAction, DocumentReplaceTransitionActionV0};
    use drive::util::storage_flags::StorageFlags;
    use simple_signer::signer::SimpleSigner;
    use std::collections::{BTreeMap, BTreeSet};

    /// A mutable `offer` type whose rules, checked in name order, are:
    ///
    /// * `boostCapped`: `price * ifAbsent(boost, 1) <= 100000`
    /// * `boostPower`: `ifAbsent(boost, 1) ^ 20 >= 1`, which overflows for a large boost
    /// * `depositCoversOrder`: `(price + fee) * quantity <= deposit`
    /// * `discountBelowPrice`: `discount < price`, an absent discount counting as 0
    /// * `perUnitDeposit`: `deposit / quantity >= 1`, which divides by zero for no quantity
    fn offer_schema() -> Value {
        platform_value!({
            "type": "object",
            "documentsMutable": true,
            "properties": {
                "price": { "type": "integer", "minimum": 0, "maximum": 1000000, "position": 0 },
                "fee": { "type": "integer", "minimum": 0, "maximum": 1000, "position": 1 },
                "quantity": { "type": "integer", "minimum": 0, "maximum": 100, "position": 2 },
                "deposit": { "type": "integer", "minimum": 0, "position": 3 },
                "discount": { "type": "integer", "minimum": 0, "maximum": 1000000, "position": 4 },
                "boost": { "type": "integer", "minimum": 0, "maximum": 100, "position": 5 }
            },
            "required": ["price", "fee", "quantity", "deposit"],
            "propertyConstraints": {
                "boostCapped": {
                    "lessThanOrEqual": [
                        { "multiply": ["price", { "ifAbsent": ["boost", 1] }] },
                        100000
                    ]
                },
                "boostPower": {
                    "greaterThanOrEqual": [{ "power": [{ "ifAbsent": ["boost", 1] }, 20] }, 1]
                },
                "depositCoversOrder": {
                    "lessThanOrEqual": [
                        { "multiply": [{ "add": ["price", "fee"] }, "quantity"] },
                        "deposit"
                    ]
                },
                "discountBelowPrice": { "lessThan": ["discount", "price"] },
                "perUnitDeposit": {
                    "greaterThanOrEqual": [{ "divide": ["deposit", "quantity"] }, 1]
                }
            },
            "additionalProperties": false
        })
    }

    /// An offer that meets every rule: (100 + 10) * 2 = 220.
    fn set_valid_offer(document: &mut Document) {
        document.set("price", Value::U64(100));
        document.set("fee", Value::U64(10));
        document.set("quantity", Value::U64(2));
        document.set("deposit", Value::U64(220));
    }

    /// One identity and one contract whose `offer` type declares the rules
    /// above. `create` writes a document, `replace` rewrites the last accepted
    /// one.
    struct OfferFixture {
        platform: TempPlatform<MockCoreRPCLike>,
        identity: Identity,
        signer: SimpleSigner,
        key: IdentityPublicKey,
        contract: DataContract,
        /// The document as last accepted by the chain, once one was.
        document: Option<Document>,
        /// The identity contract nonce the next transition uses. Every
        /// processed transition consumes one, including the ones that fail
        /// with a paid consensus error.
        next_nonce: IdentityNonce,
    }

    impl OfferFixture {
        fn new() -> Self {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();

            let (identity, signer, key) = setup_identity(&mut platform, 959, dash_to_credits!(0.5));

            let mut contract = get_data_contract_fixture(
                Some(identity.id()),
                0,
                platform_version.protocol_version,
            )
            .data_contract_owned();
            contract
                .set_document_schema(
                    "offer",
                    offer_schema(),
                    true,
                    &mut Vec::new(),
                    platform_version,
                )
                .expect("expected to add the offer document type");
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
                identity,
                signer,
                key,
                contract,
                document: None,
                next_nonce: 1,
            }
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

        /// Creates a valid offer changed by `fill`. On success it becomes the
        /// fixture's document.
        async fn create(
            &mut self,
            fill: impl FnOnce(&mut Document),
        ) -> StateTransitionExecutionResult {
            let platform_version = PlatformVersion::latest();
            let offer_type = self
                .contract
                .document_type_for_name("offer")
                .expect("expected the offer document type");

            let mut rng = StdRng::seed_from_u64(434);
            let entropy = Bytes32::random_with_rng(&mut rng);
            let mut document = offer_type
                .random_document_with_identifier_and_entropy(
                    &mut rng,
                    self.identity.id(),
                    entropy,
                    DocumentFieldFillType::DoNotFillIfNotRequired,
                    DocumentFieldFillSize::AnyDocumentFillSize,
                    platform_version,
                )
                .expect("expected a random offer");
            document
                .set_id_for_creation(offer_type, &entropy.0, self.next_nonce, platform_version)
                .expect("expected to set the document id");
            set_valid_offer(&mut document);
            fill(&mut document);

            let transition = BatchTransition::new_document_creation_transition_from_document(
                document.clone(),
                offer_type,
                entropy.0,
                &self.key,
                self.next_nonce,
                0,
                None,
                &self.signer,
                platform_version,
                None,
            )
            .await
            .expect("expected the create transition");
            self.next_nonce += 1;

            let result = self.process(&transition);
            if matches!(
                result,
                StateTransitionExecutionResult::SuccessfulExecution { .. }
            ) {
                self.document = Some(document);
            }
            result
        }

        /// Replaces the stored offer with `mutate` applied to a copy of it and
        /// the revision bumped. On success the fixture's document becomes the
        /// accepted version.
        async fn replace(
            &mut self,
            mutate: impl FnOnce(&mut Document),
        ) -> StateTransitionExecutionResult {
            let platform_version = PlatformVersion::latest();
            let mut replacement = self
                .document
                .clone()
                .expect("a document must have been created first");
            mutate(&mut replacement);
            replacement
                .increment_revision()
                .expect("expected the revision to increment");

            let transition = {
                let offer_type = self
                    .contract
                    .document_type_for_name("offer")
                    .expect("expected the offer document type");
                BatchTransition::new_document_replacement_transition_from_document(
                    replacement.clone(),
                    offer_type,
                    &self.key,
                    self.next_nonce,
                    0,
                    None,
                    &self.signer,
                    platform_version,
                    None,
                )
                .await
                .expect("expected the replace transition")
            };
            self.next_nonce += 1;

            let result = self.process(&transition);
            if matches!(
                result,
                StateTransitionExecutionResult::SuccessfulExecution { .. }
            ) {
                self.document = Some(replacement);
            }
            result
        }

        fn stored_offers(&self) -> Vec<Document> {
            let platform_version = PlatformVersion::latest();
            let query = DriveDocumentQuery::from_sql_expr(
                "select * from offer",
                &self.contract,
                Some(&self.platform.config.drive),
                platform_version,
            )
            .expect("expected a document query");
            self.platform
                .drive
                .query_documents(query, None, false, None, None)
                .expect("expected a query result")
                .documents()
                .to_vec()
        }
    }

    /// An integer property of a stored document, whatever width it was read back at.
    fn integer(document: &Document, property: &str) -> Option<u64> {
        document
            .get(property)
            .and_then(|value| value.to_integer::<u64>().ok())
    }

    fn expect_violated(
        result: StateTransitionExecutionResult,
        expected_constraint: &str,
        expected_violation: PropertyConstraintViolation,
    ) {
        let StateTransitionExecutionResult::PaidConsensusError { error, .. } = result else {
            panic!("expected a paid consensus error, got {result:?}");
        };
        let ConsensusError::BasicError(BasicError::DocumentPropertyConstraintViolatedError(error)) =
            error
        else {
            panic!("expected DocumentPropertyConstraintViolatedError, got {error:?}");
        };
        assert_eq!(error.document_type_name(), "offer");
        assert_eq!(error.constraint(), expected_constraint);
        assert_eq!(error.violation(), expected_violation);
        assert_eq!(ConsensusError::from(error).code(), 10422);
    }

    #[tokio::test]
    async fn should_accept_a_create_that_meets_every_rule() {
        let mut fixture = OfferFixture::new();

        assert_matches!(
            fixture.create(|_| {}).await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        let stored = fixture.stored_offers();
        assert_eq!(stored.len(), 1);
        assert_eq!(integer(&stored[0], "deposit"), Some(220));
    }

    #[tokio::test]
    async fn should_reject_a_create_whose_comparison_does_not_hold() {
        let mut fixture = OfferFixture::new();

        // (100 + 10) * 2 = 220 > 219
        let result = fixture
            .create(|document| document.set("deposit", Value::U64(219)))
            .await;

        expect_violated(
            result,
            "depositCoversOrder",
            PropertyConstraintViolation::NotMet,
        );
        assert!(
            fixture.stored_offers().is_empty(),
            "the refused document must not be stored"
        );
    }

    /// `discount` is absent, so it counts as 0, and `0 < 0` does not hold.
    #[tokio::test]
    async fn should_count_an_absent_property_as_zero() {
        let mut fixture = OfferFixture::new();

        let result = fixture
            .create(|document| {
                document.set("price", Value::U64(0));
                document.set("fee", Value::U64(0));
            })
            .await;

        expect_violated(
            result,
            "discountBelowPrice",
            PropertyConstraintViolation::NotMet,
        );
        assert!(fixture.stored_offers().is_empty());
    }

    /// `boost` is absent, so `ifAbsent` makes it 1: `200000 * 1` is above the cap,
    /// where a boost of 0 would have met it. A boost the document carries is used.
    #[tokio::test]
    async fn should_take_the_if_absent_value_for_an_absent_property() {
        let mut fixture = OfferFixture::new();

        let result = fixture
            .create(|document| {
                document.set("price", Value::U64(200000));
                document.set("deposit", Value::U64(500000));
            })
            .await;
        expect_violated(result, "boostCapped", PropertyConstraintViolation::NotMet);

        // 50000 * 3 = 150000 is above the cap as well
        let result = fixture
            .create(|document| {
                document.set("price", Value::U64(50000));
                document.set("deposit", Value::U64(200000));
                document.set("boost", Value::U64(3));
            })
            .await;
        expect_violated(result, "boostCapped", PropertyConstraintViolation::NotMet);

        // 50000 * 2 = 100000 meets it
        assert_matches!(
            fixture
                .create(|document| {
                    document.set("price", Value::U64(50000));
                    document.set("deposit", Value::U64(200000));
                    document.set("boost", Value::U64(2));
                })
                .await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        assert_eq!(fixture.stored_offers().len(), 1);
    }

    /// A zero quantity divides the deposit by zero, and a boost of 100 raised to the
    /// power 20 does not fit 128 bits: both refuse the document instead of wrapping.
    #[tokio::test]
    async fn should_reject_a_create_that_divides_by_zero_or_overflows() {
        let mut fixture = OfferFixture::new();

        let result = fixture
            .create(|document| document.set("quantity", Value::U64(0)))
            .await;
        expect_violated(
            result,
            "perUnitDeposit",
            PropertyConstraintViolation::DivisionByZero,
        );

        let result = fixture
            .create(|document| document.set("boost", Value::U64(100)))
            .await;
        expect_violated(result, "boostPower", PropertyConstraintViolation::Overflow);

        assert!(fixture.stored_offers().is_empty());
    }

    #[tokio::test]
    async fn should_judge_a_replace_against_the_rules() {
        let mut fixture = OfferFixture::new();
        assert_matches!(
            fixture.create(|_| {}).await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );

        // Doubling the quantity without the deposit breaks the rule
        let result = fixture
            .replace(|document| document.set("quantity", Value::U64(4)))
            .await;
        expect_violated(
            result,
            "depositCoversOrder",
            PropertyConstraintViolation::NotMet,
        );
        let stored = fixture.stored_offers();
        assert_eq!(stored.len(), 1);
        assert_eq!(
            integer(&stored[0], "quantity"),
            Some(2),
            "the refused replace must leave the stored document untouched"
        );

        // Doubling both meets it
        assert_matches!(
            fixture
                .replace(|document| {
                    document.set("quantity", Value::U64(4));
                    document.set("deposit", Value::U64(440));
                })
                .await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        assert_eq!(integer(&fixture.stored_offers()[0], "quantity"), Some(4));
    }

    /// A replace carries the whole document, so a property it leaves out is absent
    /// from the rules' point of view even though the stored document had it: it counts
    /// as 0, or as its `ifAbsent` value, never as the stored value. Each replace below
    /// is accepted only that way: the stored `discount` (50) would break
    /// `discountBelowPrice` at a price of 40, and the stored `boost` (2) would break
    /// `boostCapped` at a price of 60000.
    #[tokio::test]
    async fn should_judge_a_replace_that_leaves_out_an_operand_by_its_absent_value() {
        let mut fixture = OfferFixture::new();
        assert_matches!(
            fixture
                .create(|document| {
                    document.set("discount", Value::U64(50));
                    document.set("boost", Value::U64(2));
                })
                .await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );

        // discount absent, 0 < 40; (40 + 10) * 2 = 100 <= 220
        assert_matches!(
            fixture
                .replace(|document| {
                    document.remove("discount");
                    document.set("price", Value::U64(40));
                })
                .await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );

        // boost absent, 60000 * 1 <= 100000; (60000 + 10) * 2 = 120020
        assert_matches!(
            fixture
                .replace(|document| {
                    document.remove("boost");
                    document.set("price", Value::U64(60000));
                    document.set("deposit", Value::U64(120020));
                })
                .await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );

        let stored = fixture.stored_offers();
        assert_eq!(stored.len(), 1);
        assert_eq!(integer(&stored[0], "price"), Some(60000));
        assert_eq!(stored[0].get("discount"), None);
        assert_eq!(stored[0].get("boost"), None);
    }

    /// The replace structure dispatcher on both sides of the gate: structure
    /// generation 0 reaches the `propertyConstraints` check through
    /// `DataContract::validate_document_properties` 0, extended in place, so at
    /// protocol version 13 it must still accept the action (no type parsed there
    /// carries a rule and the dpp gate is `None`), and at 14 refuse it. The
    /// action is built by hand the way the transformer would build it, against
    /// the contract as Drive hands it back.
    #[test]
    fn should_not_judge_property_constraints_on_replace_before_protocol_version_14() {
        let platform_version = PlatformVersion::latest();
        let fixture = OfferFixture::new();
        let owner_id = fixture.identity.id();

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
        let contract_fetch_info = contract_fetch_info.expect("the contract is in state");

        let action = DocumentReplaceTransitionAction::V0(DocumentReplaceTransitionActionV0 {
            base: DocumentBaseTransitionAction::V0(DocumentBaseTransitionActionV0 {
                id: Identifier::from([0xAA; 32]),
                identity_contract_nonce: 1,
                document_type_name: "offer".to_string(),
                data_contract: contract_fetch_info,
                token_cost: None,
                gas_fees_paid_by: GasFeesPaidBy::default(),
                contract_gas_fees_paid_by: GasFeesPaidBy::default(),
                declared_action_fee: None,
            }),
            revision: 2,
            created_at: None,
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            // (100 + 10) * 2 = 220 > 1
            data: BTreeMap::from([
                ("price".to_string(), Value::U64(100)),
                ("fee".to_string(), Value::U64(10)),
                ("quantity".to_string(), Value::U64(2)),
                ("deposit".to_string(), Value::U64(1)),
            ]),
            changed_data_fields: BTreeSet::new(),
            added_data_fields: BTreeSet::new(),
            removed_identifier_fields: BTreeMap::new(),
            stored_changed_values: BTreeMap::new(),
            creator_id: None,
        });

        let before = action
            .validate_structure(
                owner_id,
                PlatformVersion::get(13).expect("platform version 13 should exist"),
            )
            .expect("structure validation should run");
        assert!(
            before.is_valid(),
            "structure generation 0 must not judge propertyConstraints: {:?}",
            before.errors
        );

        let at = action
            .validate_structure(owner_id, platform_version)
            .expect("structure validation should run");
        assert_matches!(
            at.errors.as_slice(),
            [ConsensusError::BasicError(BasicError::DocumentPropertyConstraintViolatedError(e))]
                if e.constraint() == "depositCoversOrder"
                    && e.violation() == PropertyConstraintViolation::NotMet
        );
    }
}
