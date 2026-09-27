//! End-to-end coverage for the `distinctFrom` property keyword (protocol
//! version 14): an identifier property's value must differ from the value of
//! a named property of the same document, or from the document's `$ownerId`.
//! A create or replace whose two values are equal is consensus-rejected with
//! `DocumentPropertyNotDistinctError` (basic code 10419) and leaves the stored
//! document untouched; a declaration whose named property is absent from the
//! document passes.

use super::*;

mod distinct_from_tests {
    use super::*;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TempPlatform;
    use crate::execution::validation::state_transition::batch::action_validation::document::document_replace_transition_action::DocumentReplaceTransitionActionValidation;
    use dpp::consensus::basic::BasicError;
    use dpp::consensus::codes::ErrorWithCode;
    use dpp::document::DocumentV0Setters;
    use dpp::fee::Credits;
    use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
    use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::{DocumentBaseTransitionAction, DocumentBaseTransitionActionV0};
    use crate::execution::validation::state_transition::batch::action_validation::document::document_purchase_transition_action::DocumentPurchaseTransitionActionValidation;
    use crate::execution::validation::state_transition::batch::action_validation::document::document_transfer_transition_action::DocumentTransferTransitionActionValidation;
    use drive::state_transition_action::batch::batched_transition::document_transition::document_purchase_transition_action::{DocumentPurchaseTransitionAction, DocumentPurchaseTransitionActionV0};
    use drive::state_transition_action::batch::batched_transition::document_transition::document_replace_transition_action::{DocumentReplaceTransitionAction, DocumentReplaceTransitionActionV0};
    use drive::state_transition_action::batch::batched_transition::document_transition::document_transfer_transition_action::{DocumentTransferTransitionAction, DocumentTransferTransitionActionV0};
    use std::collections::{BTreeMap, BTreeSet};
    use dpp::data_contract::schema::DataContractSchemaMethodsV0;
    use dpp::document::Document;
    use dpp::identity::{Identity, IdentityPublicKey};
    use dpp::platform_value::platform_value;
    use dpp::prelude::{DataContract, Identifier, IdentityNonce};
    use dpp::state_transition::StateTransition;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use drive::util::storage_flags::StorageFlags;
    use simple_signer::signer::SimpleSigner;

    fn identifier_property(position: u32, distinct_from: Option<&str>) -> Value {
        let mut property = platform_value!({
            "type": "array",
            "byteArray": true,
            "minItems": 32_u32,
            "maxItems": 32_u32,
            "contentMediaType": "application/x.dash.dpp.identifier",
            "position": position
        });
        if let Some(distinct_from) = distinct_from {
            property
                .insert("distinctFrom".to_string(), distinct_from.into())
                .expect("expected to insert distinctFrom");
        }
        property
    }

    /// A mutable, transferable and purchasable `delegation` type: `delegateId`
    /// must differ from the owner, `backupId` from `delegateId`, the nested
    /// `meta.reviewerId` from its sibling `meta.approverId`, and every element
    /// of the `members` identifier array from the owner. Only the string `note`
    /// is required, so any identifier may be left out of a document.
    fn delegation_schema() -> Value {
        platform_value!({
            "type": "object",
            "documentsMutable": true,
            "transferable": 1,
            "tradeMode": 1,
            "properties": {
                "delegateId": identifier_property(0, Some("$ownerId")),
                "backupId": identifier_property(1, Some("delegateId")),
                "note": {"type": "string", "position": 2, "maxLength": 63_u32},
                "meta": {
                    "type": "object",
                    "position": 3,
                    "properties": {
                        "reviewerId": identifier_property(0, Some("meta.approverId")),
                        "approverId": identifier_property(1, None)
                    },
                    "additionalProperties": false
                },
                "members": {
                    "type": "array",
                    "maxItems": 8_u32,
                    "position": 4,
                    "items": {
                        "type": "array",
                        "byteArray": true,
                        "minItems": 32_u32,
                        "maxItems": 32_u32,
                        "contentMediaType": "application/x.dash.dpp.identifier",
                        "distinctFrom": "$ownerId"
                    }
                }
            },
            "required": ["note"],
            "additionalProperties": false
        })
    }

    fn id(byte: u8) -> Value {
        Value::Identifier([byte; 32])
    }

    /// One identity and one contract whose `delegation` type declares the
    /// rules above. `create` writes a document, `replace` rewrites the last
    /// accepted one.
    struct DelegationFixture {
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

    impl DelegationFixture {
        fn new() -> Self {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.5));

            let mut contract = get_data_contract_fixture(
                Some(identity.id()),
                0,
                platform_version.protocol_version,
            )
            .data_contract_owned();
            contract
                .set_document_schema(
                    "delegation",
                    delegation_schema(),
                    true,
                    &mut Vec::new(),
                    platform_version,
                )
                .expect("expected to add the delegation document type");
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

        fn owner_id(&self) -> Value {
            Value::Identifier(self.identity.id().to_buffer())
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

        /// Creates a delegation with `note` set and whatever `fill` adds. On
        /// success it becomes the fixture's document.
        async fn create(
            &mut self,
            fill: impl FnOnce(&mut Document),
        ) -> StateTransitionExecutionResult {
            let platform_version = PlatformVersion::latest();
            let delegation_type = self
                .contract
                .document_type_for_name("delegation")
                .expect("expected the delegation document type");

            let mut rng = StdRng::seed_from_u64(433);
            let entropy = Bytes32::random_with_rng(&mut rng);
            let mut document = delegation_type
                .random_document_with_identifier_and_entropy(
                    &mut rng,
                    self.identity.id(),
                    entropy,
                    DocumentFieldFillType::DoNotFillIfNotRequired,
                    DocumentFieldFillSize::AnyDocumentFillSize,
                    platform_version,
                )
                .expect("expected a random delegation");
            document
                .set_id_for_creation(
                    delegation_type,
                    &entropy.0,
                    self.next_nonce,
                    platform_version,
                )
                .expect("expected to set the document id");
            document.set("note", "first draft".into());
            fill(&mut document);

            let transition = BatchTransition::new_document_creation_transition_from_document(
                document.clone(),
                delegation_type,
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

        /// Replaces the stored delegation with `mutate` applied to a copy of
        /// it and the revision bumped. On success the fixture's document
        /// becomes the accepted version.
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
                let delegation_type = self
                    .contract
                    .document_type_for_name("delegation")
                    .expect("expected the delegation document type");
                BatchTransition::new_document_replacement_transition_from_document(
                    replacement.clone(),
                    delegation_type,
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

        /// Transfers the stored delegation to `recipient`. On success the
        /// fixture's document becomes the transferred version.
        async fn transfer(&mut self, recipient: Identifier) -> StateTransitionExecutionResult {
            let platform_version = PlatformVersion::latest();
            let mut transferred = self
                .document
                .clone()
                .expect("a document must have been created first");
            transferred
                .increment_revision()
                .expect("expected the revision to increment");

            let transition = {
                let delegation_type = self
                    .contract
                    .document_type_for_name("delegation")
                    .expect("expected the delegation document type");
                BatchTransition::new_document_transfer_transition_from_document(
                    transferred.clone(),
                    delegation_type,
                    recipient,
                    &self.key,
                    self.next_nonce,
                    0,
                    None,
                    &self.signer,
                    platform_version,
                    None,
                )
                .await
                .expect("expected the transfer transition")
            };
            self.next_nonce += 1;

            let result = self.process(&transition);
            if matches!(
                result,
                StateTransitionExecutionResult::SuccessfulExecution { .. }
            ) {
                transferred.set_owner_id(recipient);
                self.document = Some(transferred);
            }
            result
        }

        /// Puts the stored delegation up for sale at `price`.
        async fn set_price(&mut self, price: Credits) {
            let platform_version = PlatformVersion::latest();
            let mut priced = self
                .document
                .clone()
                .expect("a document must have been created first");
            priced
                .increment_revision()
                .expect("expected the revision to increment");

            let transition = {
                let delegation_type = self
                    .contract
                    .document_type_for_name("delegation")
                    .expect("expected the delegation document type");
                BatchTransition::new_document_update_price_transition_from_document(
                    priced.clone(),
                    delegation_type,
                    price,
                    &self.key,
                    self.next_nonce,
                    0,
                    None,
                    &self.signer,
                    platform_version,
                    None,
                )
                .await
                .expect("expected the update price transition")
            };
            self.next_nonce += 1;

            assert_matches!(
                self.process(&transition),
                StateTransitionExecutionResult::SuccessfulExecution { .. },
                "setting the price must succeed"
            );
            self.document = Some(priced);
        }

        /// A second funded identity on the fixture's platform.
        fn other_identity(&mut self, seed: u64) -> (Identity, SimpleSigner, IdentityPublicKey) {
            setup_identity(&mut self.platform, seed, dash_to_credits!(0.5))
        }

        /// `buyer` purchases the stored delegation at `price` (its first
        /// transition, so nonce 1).
        async fn purchase_by(
            &mut self,
            buyer: &(Identity, SimpleSigner, IdentityPublicKey),
            price: Credits,
        ) -> StateTransitionExecutionResult {
            let platform_version = PlatformVersion::latest();
            let (buyer_identity, buyer_signer, buyer_key) = buyer;
            let mut bought = self
                .document
                .clone()
                .expect("a document must have been created first");
            bought
                .increment_revision()
                .expect("expected the revision to increment");

            let transition = {
                let delegation_type = self
                    .contract
                    .document_type_for_name("delegation")
                    .expect("expected the delegation document type");
                BatchTransition::new_document_purchase_transition_from_document(
                    bought,
                    delegation_type,
                    buyer_identity.id(),
                    price,
                    buyer_key,
                    1,
                    0,
                    None,
                    buyer_signer,
                    platform_version,
                    None,
                )
                .await
                .expect("expected the purchase transition")
            };

            self.process(&transition)
        }

        /// The stored delegations, read back from Drive.
        fn stored_delegations(&self) -> Vec<Document> {
            let platform_version = PlatformVersion::latest();
            let query = DriveDocumentQuery::from_sql_expr(
                "select * from delegation",
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

    fn expect_not_distinct_error(
        result: StateTransitionExecutionResult,
        expected_property: &str,
        expected_distinct_from: &str,
    ) {
        let StateTransitionExecutionResult::PaidConsensusError { error, .. } = result else {
            panic!("expected a paid consensus error, got {result:?}");
        };
        let ConsensusError::BasicError(BasicError::DocumentPropertyNotDistinctError(error)) = error
        else {
            panic!("expected DocumentPropertyNotDistinctError, got {error:?}");
        };
        assert_eq!(error.document_type_name(), "delegation");
        assert_eq!(error.property(), expected_property);
        assert_eq!(error.distinct_from(), expected_distinct_from);
        assert_eq!(ConsensusError::from(error).code(), 10419);
    }

    /// The replace structure dispatcher on both sides of the gate: structure
    /// generation 0 gained the `distinctFrom` judgement in place, so at protocol
    /// version 13 it must still accept the action (no property parsed there
    /// carries the keyword and the dpp gate is `None`), and at 14 refuse it.
    /// The action is built by hand the way the transformer would build it,
    /// against the contract as Drive hands it back.
    #[test]
    fn should_not_judge_distinct_from_on_replace_before_protocol_version_14() {
        let platform_version = PlatformVersion::latest();
        let fixture = DelegationFixture::new();
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
                document_type_name: "delegation".to_string(),
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
            data: BTreeMap::from([
                ("note".to_string(), "draft".into()),
                ("delegateId".to_string(), fixture.owner_id()),
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
            "structure generation 0 must not judge distinctFrom: {:?}",
            before.errors
        );

        let at = action
            .validate_structure(owner_id, platform_version)
            .expect("structure validation should run");
        assert_matches!(
            at.errors.as_slice(),
            [ConsensusError::BasicError(BasicError::DocumentPropertyNotDistinctError(e))]
                if e.property() == "delegateId" && e.distinct_from() == "$ownerId"
        );
    }

    /// Transfer and purchase structure validation v0 gained the `distinctFrom`
    /// judgement in place. At protocol version 13 the same module must still
    /// accept the action, as no property parsed there carries the keyword and
    /// the dpp gate is `None`; at 14 it refuses the equal pair.
    #[tokio::test]
    async fn should_not_judge_distinct_from_on_transfer_or_purchase_before_protocol_version_14() {
        let platform_version = PlatformVersion::latest();
        let platform_version_13 =
            PlatformVersion::get(13).expect("platform version 13 should exist");
        let mut fixture = DelegationFixture::new();
        let recipient = Identifier::from([0xEE; 32]);
        assert_matches!(
            fixture
                .create(|document| {
                    document.set("delegateId", Value::Identifier(recipient.to_buffer()))
                })
                .await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        // The transformer hands the validators the stored document already
        // re-owned by the recipient
        let mut transferred = fixture
            .document
            .clone()
            .expect("the delegation was created");
        transferred.set_owner_id(recipient);

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
        let base = || {
            DocumentBaseTransitionAction::V0(DocumentBaseTransitionActionV0 {
                id: transferred.id(),
                identity_contract_nonce: 2,
                document_type_name: "delegation".to_string(),
                data_contract: contract_fetch_info.clone(),
                token_cost: None,
                gas_fees_paid_by: GasFeesPaidBy::default(),
                contract_gas_fees_paid_by: GasFeesPaidBy::default(),
                declared_action_fee: None,
            })
        };

        let transfer = DocumentTransferTransitionAction::V0(DocumentTransferTransitionActionV0 {
            base: base(),
            document: transferred.clone(),
        });
        let purchase = DocumentPurchaseTransitionAction::V0(DocumentPurchaseTransitionActionV0 {
            base: base(),
            document: transferred.clone(),
            original_owner_id: fixture.identity.id(),
            price: 1,
        });

        for (name, before, at) in [
            (
                "transfer",
                transfer.validate_structure(platform_version_13),
                transfer.validate_structure(platform_version),
            ),
            (
                "purchase",
                purchase.validate_structure(platform_version_13),
                purchase.validate_structure(platform_version),
            ),
        ] {
            let before = before.expect("structure validation should run");
            assert!(
                before.is_valid(),
                "{name}: v0 must not judge distinctFrom before protocol version 14: {:?}",
                before.errors
            );
            let at = at.expect("structure validation should run");
            assert_matches!(
                at.errors.as_slice(),
                [ConsensusError::BasicError(BasicError::DocumentPropertyNotDistinctError(e))]
                    if e.property() == "delegateId" && e.distinct_from() == "$ownerId",
                "{name}"
            );
        }
    }

    #[tokio::test]
    async fn should_reject_a_create_whose_property_equals_the_owner_id() {
        let mut fixture = DelegationFixture::new();
        let owner_id = fixture.owner_id();

        let result = fixture
            .create(|document| document.set("delegateId", owner_id))
            .await;

        expect_not_distinct_error(result, "delegateId", "$ownerId");
        assert!(
            fixture.stored_delegations().is_empty(),
            "the refused document must not be stored"
        );
    }

    #[tokio::test]
    async fn should_accept_a_create_whose_property_differs_from_the_owner_id() {
        let mut fixture = DelegationFixture::new();

        let result = fixture
            .create(|document| document.set("delegateId", id(1)))
            .await;

        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        let stored = fixture.stored_delegations();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].get("delegateId"), Some(&id(1)));
    }

    #[tokio::test]
    async fn should_reject_a_create_whose_property_equals_the_named_sibling_property() {
        let mut fixture = DelegationFixture::new();

        let result = fixture
            .create(|document| {
                document.set("delegateId", id(1));
                document.set("backupId", id(1));
            })
            .await;

        expect_not_distinct_error(result, "backupId", "delegateId");
        assert!(fixture.stored_delegations().is_empty());
    }

    #[tokio::test]
    async fn should_accept_a_create_when_the_named_sibling_property_is_absent() {
        let mut fixture = DelegationFixture::new();

        // `backupId` must differ from `delegateId`, which the document leaves
        // out: there is nothing to differ from
        let result = fixture
            .create(|document| document.set("backupId", id(1)))
            .await;

        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        assert_eq!(fixture.stored_delegations().len(), 1);
    }

    #[tokio::test]
    async fn should_accept_a_create_whose_sibling_properties_differ() {
        let mut fixture = DelegationFixture::new();

        let result = fixture
            .create(|document| {
                document.set("delegateId", id(1));
                document.set("backupId", id(2));
                document.set(
                    "meta",
                    platform_value!({ "reviewerId": id(3), "approverId": id(4) }),
                );
            })
            .await;

        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
    }

    #[tokio::test]
    async fn should_reject_a_create_whose_nested_property_equals_its_named_sibling() {
        let mut fixture = DelegationFixture::new();

        let result = fixture
            .create(|document| {
                document.set(
                    "meta",
                    platform_value!({ "reviewerId": id(3), "approverId": id(3) }),
                );
            })
            .await;

        expect_not_distinct_error(result, "meta.reviewerId", "meta.approverId");
    }

    #[tokio::test]
    async fn should_reject_a_replace_that_makes_the_two_properties_equal() {
        let mut fixture = DelegationFixture::new();
        assert_matches!(
            fixture
                .create(|document| {
                    document.set("delegateId", id(1));
                    document.set("backupId", id(2));
                })
                .await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );

        let result = fixture
            .replace(|document| document.set("backupId", id(1)))
            .await;

        expect_not_distinct_error(result, "backupId", "delegateId");
        let stored = fixture.stored_delegations();
        assert_eq!(stored.len(), 1);
        assert_eq!(
            stored[0].get("backupId"),
            Some(&id(2)),
            "the refused replace must leave the stored document untouched"
        );
    }

    #[tokio::test]
    async fn should_reject_a_replace_that_sets_the_property_to_the_owner_id() {
        let mut fixture = DelegationFixture::new();
        assert_matches!(
            fixture
                .create(|document| document.set("delegateId", id(1)))
                .await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        let owner_id = fixture.owner_id();

        let result = fixture
            .replace(|document| document.set("delegateId", owner_id))
            .await;

        expect_not_distinct_error(result, "delegateId", "$ownerId");
        assert_eq!(
            fixture.stored_delegations()[0].get("delegateId"),
            Some(&id(1))
        );
    }

    #[tokio::test]
    async fn should_reject_a_create_whose_array_element_equals_the_owner_id() {
        let mut fixture = DelegationFixture::new();
        let owner_id = fixture.owner_id();

        let result = fixture
            .create(|document| document.set("members", Value::Array(vec![id(1), owner_id])))
            .await;

        expect_not_distinct_error(result, "members", "$ownerId");
        assert!(fixture.stored_delegations().is_empty());
    }

    #[tokio::test]
    async fn should_accept_a_create_whose_array_elements_all_differ_from_the_owner_id() {
        let mut fixture = DelegationFixture::new();

        let result = fixture
            .create(|document| document.set("members", Value::Array(vec![id(1), id(2)])))
            .await;

        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        assert_eq!(
            fixture.stored_delegations()[0].get("members"),
            Some(&Value::Array(vec![id(1), id(2)]))
        );
    }

    #[tokio::test]
    async fn should_reject_a_transfer_to_the_identity_the_property_must_differ_from() {
        let mut fixture = DelegationFixture::new();
        let (recipient, _, _) = fixture.other_identity(450);
        assert_matches!(
            fixture
                .create(|document| {
                    document.set("delegateId", Value::Identifier(recipient.id().to_buffer()))
                })
                .await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );

        let result = fixture.transfer(recipient.id()).await;

        expect_not_distinct_error(result, "delegateId", "$ownerId");
        assert_eq!(
            fixture.stored_delegations()[0].owner_id(),
            fixture.identity.id(),
            "the refused transfer must leave the owner unchanged"
        );
    }

    #[tokio::test]
    async fn should_accept_a_transfer_to_another_identity() {
        let mut fixture = DelegationFixture::new();
        let (recipient, _, _) = fixture.other_identity(450);
        assert_matches!(
            fixture
                .create(|document| document.set("delegateId", id(1)))
                .await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );

        let result = fixture.transfer(recipient.id()).await;

        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        assert_eq!(fixture.stored_delegations()[0].owner_id(), recipient.id());
    }

    #[tokio::test]
    async fn should_reject_a_purchase_by_the_identity_the_property_must_differ_from() {
        let mut fixture = DelegationFixture::new();
        let buyer = fixture.other_identity(450);
        assert_matches!(
            fixture
                .create(|document| {
                    document.set("delegateId", Value::Identifier(buyer.0.id().to_buffer()))
                })
                .await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        fixture.set_price(dash_to_credits!(0.1)).await;

        let result = fixture.purchase_by(&buyer, dash_to_credits!(0.1)).await;

        expect_not_distinct_error(result, "delegateId", "$ownerId");
        assert_eq!(
            fixture.stored_delegations()[0].owner_id(),
            fixture.identity.id(),
            "the refused purchase must leave the owner unchanged"
        );
    }

    #[tokio::test]
    async fn should_accept_a_purchase_by_another_identity() {
        let mut fixture = DelegationFixture::new();
        let buyer = fixture.other_identity(450);
        assert_matches!(
            fixture
                .create(|document| document.set("delegateId", id(1)))
                .await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        fixture.set_price(dash_to_credits!(0.1)).await;

        let result = fixture.purchase_by(&buyer, dash_to_credits!(0.1)).await;

        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        assert_eq!(fixture.stored_delegations()[0].owner_id(), buyer.0.id());
    }

    #[tokio::test]
    async fn should_accept_a_replace_that_keeps_the_properties_distinct() {
        let mut fixture = DelegationFixture::new();
        assert_matches!(
            fixture
                .create(|document| {
                    document.set("delegateId", id(1));
                    document.set("backupId", id(2));
                })
                .await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );

        let result = fixture
            .replace(|document| {
                document.set("delegateId", id(2));
                document.set("backupId", id(3));
            })
            .await;

        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        let stored = fixture.stored_delegations();
        assert_eq!(stored[0].get("delegateId"), Some(&id(2)));
        assert_eq!(stored[0].get("backupId"), Some(&id(3)));
    }
}
