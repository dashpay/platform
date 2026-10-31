//! End-to-end coverage for `generatedFrom` with `sys.tokens.decimals` (protocol version 15):
//! an integer property the platform fills with the decimals of the token an identifier
//! property names, read from the token's contract when the document is created and kept
//! after, unless a replace names another token. A client leaves the property out; one that
//! sends a value other than the token's is refused, and so is a token that does not exist.

use super::*;

mod generated_from_token_tests {
    use super::*;
    use crate::execution::validation::state_transition::tests::create_token_contract_with_owner_identity;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TempPlatform;
    use dpp::consensus::basic::BasicError;
    use dpp::consensus::state::state_error::StateError;
    use dpp::data_contract::accessors::v0::DataContractV0Setters;
    use dpp::data_contract::accessors::v1::DataContractV1Getters;
    use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
    use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
    use dpp::data_contract::associated_token::token_configuration_convention::accessors::v0::TokenConfigurationConventionV0Setters;
    use dpp::data_contract::schema::DataContractSchemaMethodsV0;
    use dpp::document::Document;
    use dpp::identity::{Identity, IdentityPublicKey, SecurityLevel};
    use dpp::platform_value::platform_value;
    use dpp::prelude::Identifier;
    use dpp::prelude::{DataContract, IdentityNonce};
    use dpp::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
    use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransitionV0Methods;
    use dpp::state_transition::batch_transition::batched_transition::BatchedTransitionMutRef;
    use dpp::state_transition::proof_result::StateTransitionProofResult;
    use dpp::state_transition::StateTransition;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use drive::drive::Drive;
    use drive::util::storage_flags::StorageFlags;
    use simple_signer::signer::SimpleSigner;
    use std::collections::BTreeMap;
    use std::sync::Arc;

    /// A mutable `order` type: the `tokenId` it is for, the token's `decimals`, generated
    /// from it, and a free `note`.
    fn order_schema() -> Value {
        platform_value!({
            "type": "object",
            "documentsMutable": true,
            "indices": [
                {
                    "name": "byToken",
                    "properties": [{ "tokenId": "asc" }, { "decimals": "asc" }]
                }
            ],
            "properties": {
                "tokenId": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "position": 0
                },
                "decimals": {
                    "type": "integer",
                    "minimum": 0,
                    "maximum": 255,
                    "generatedFrom": {
                        "function": "sys.tokens.decimals",
                        "params": ["tokenId"]
                    },
                    "position": 1
                },
                "note": { "type": "string", "maxLength": 32, "position": 2 }
            },
            "required": ["tokenId", "decimals"],
            "additionalProperties": false
        })
    }

    /// One identity holding an `order` contract, and two tokens of other contracts: one of
    /// 6 decimals, one of 18.
    struct OrderFixture {
        platform: TempPlatform<MockCoreRPCLike>,
        signer: SimpleSigner,
        key: IdentityPublicKey,
        identity: Identity,
        contract: DataContract,
        token_contract: DataContract,
        token_id: Identifier,
        other_token_id: Identifier,
        next_nonce: IdentityNonce,
    }

    impl OrderFixture {
        fn new() -> Self {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();

            let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.5));
            let (issuer, _, _) = setup_identity(&mut platform, 959, dash_to_credits!(0.5));
            let (other_issuer, _, _) = setup_identity(&mut platform, 960, dash_to_credits!(0.5));
            let with_decimals = |decimals: u8| {
                move |configuration: &mut TokenConfiguration| {
                    configuration.conventions_mut().set_decimals(decimals)
                }
            };
            let (token_contract, token_id) = create_token_contract_with_owner_identity(
                &mut platform,
                issuer.id(),
                Some(with_decimals(6)),
                None,
                None,
                None,
                platform_version,
            );
            let (_, other_token_id) = create_token_contract_with_owner_identity(
                &mut platform,
                other_issuer.id(),
                Some(with_decimals(18)),
                None,
                None,
                None,
                platform_version,
            );

            let mut contract = get_data_contract_fixture(
                Some(identity.id()),
                0,
                platform_version.protocol_version,
            )
            .data_contract_owned();
            contract
                .set_document_schema(
                    "order",
                    order_schema(),
                    true,
                    &mut Vec::new(),
                    platform_version,
                )
                .expect("expected to add the order document type");
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
                signer,
                key,
                identity,
                contract,
                token_contract,
                token_id,
                other_token_id,
                next_nonce: 1,
            }
        }

        fn order_data(token_id: Identifier, decimals: Option<u8>) -> BTreeMap<String, Value> {
            let mut data = BTreeMap::from([(
                "tokenId".to_string(),
                Value::Identifier(token_id.to_buffer()),
            )]);
            if let Some(decimals) = decimals {
                data.insert("decimals".to_string(), Value::U8(decimals));
            }
            data
        }

        /// `transition` with its document's data set to exactly `data`, signed again.
        async fn send_as_given(
            &self,
            mut transition: StateTransition,
            data: BTreeMap<String, Value>,
        ) -> StateTransition {
            {
                let StateTransition::Batch(batch) = &mut transition else {
                    panic!("expected a batch transition");
                };
                let Some(BatchedTransitionMutRef::Document(document_transition)) =
                    batch.first_transition_mut()
                else {
                    panic!("expected a document transition");
                };
                *document_transition
                    .data_mut()
                    .expect("the document transition carries data") = data;
            }
            transition
                .sign_external(
                    &self.key,
                    &self.signer,
                    Some(|_, _| Ok(SecurityLevel::HIGH)),
                )
                .await
                .expect("expected to sign the transition again");
            transition
        }

        /// A create transition for an order holding exactly `data`.
        async fn create_transition(
            &mut self,
            data: BTreeMap<String, Value>,
            seed: u64,
        ) -> StateTransition {
            let platform_version = PlatformVersion::latest();
            let document_type = self
                .contract
                .document_type_for_name("order")
                .expect("expected the order document type");
            let mut rng = StdRng::seed_from_u64(seed);
            let entropy = Bytes32::random_with_rng(&mut rng);
            let mut document = document_type
                .random_document_with_identifier_and_entropy(
                    &mut rng,
                    self.identity.id(),
                    entropy,
                    DocumentFieldFillType::DoNotFillIfNotRequired,
                    DocumentFieldFillSize::AnyDocumentFillSize,
                    platform_version,
                )
                .expect("expected a random document");
            document
                .set_id_for_creation(document_type, &entropy.0, self.next_nonce, platform_version)
                .expect("expected to set the document id");
            *document.properties_mut() = data.clone();

            let transition = BatchTransition::new_document_creation_transition_from_document(
                document,
                document_type,
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
            self.send_as_given(transition, data).await
        }

        /// A replace transition for `stored`, its revision bumped, holding exactly `data`.
        async fn replace_transition(
            &mut self,
            stored: &Document,
            data: BTreeMap<String, Value>,
        ) -> StateTransition {
            let platform_version = PlatformVersion::latest();
            let mut replacement = stored.clone();
            replacement
                .increment_revision()
                .expect("expected the revision to increment");
            let document_type = self
                .contract
                .document_type_for_name("order")
                .expect("expected the order document type");
            let transition = BatchTransition::new_document_replacement_transition_from_document(
                replacement,
                document_type,
                &self.key,
                self.next_nonce,
                0,
                None,
                &self.signer,
                platform_version,
                None,
            )
            .await
            .expect("expected the replace transition");
            self.next_nonce += 1;
            self.send_as_given(transition, data).await
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

        /// Proves the committed state `transition` wrote and verifies the proof as a client
        /// holding the order contract does: it holds no state to generate the decimals from.
        fn verify_proof(&self, transition: &StateTransition) -> StateTransitionProofResult {
            let platform_version = PlatformVersion::latest();
            let proof = self
                .platform
                .drive
                .prove_state_transition(transition, None, platform_version)
                .expect("expected to prove the state transition")
                .into_data()
                .expect("expected proof bytes");
            let known_contracts: BTreeMap<Identifier, DataContract> =
                BTreeMap::from([(self.contract.id(), self.contract.clone())]);
            let (_, outcome) = Drive::verify_state_transition_was_executed_with_proof(
                transition,
                &BlockInfo::default(),
                &proof,
                &|id| Ok(known_contracts.get(id).cloned().map(Arc::new)),
                platform_version,
            )
            .expect("expected the proof to verify");
            outcome.into_result()
        }

        fn stored_orders(&self) -> Vec<Document> {
            let platform_version = PlatformVersion::latest();
            let query = DriveDocumentQuery::from_sql_expr(
                "select * from order",
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

        /// Sets the decimals of the first token to `decimals`, as a `Conventions` update
        /// of its contract would.
        fn change_token_decimals(&mut self, decimals: u8) {
            let platform_version = PlatformVersion::latest();
            let mut token_contract = self.token_contract.clone();
            token_contract
                .token_configuration_mut(0)
                .expect("expected the token configuration")
                .conventions_mut()
                .set_decimals(decimals);
            token_contract.increment_version();
            self.platform
                .drive
                .apply_contract(
                    &token_contract,
                    BlockInfo::default(),
                    true,
                    StorageFlags::optional_default_as_cow(),
                    None,
                    platform_version,
                )
                .expect("expected to update the token contract");
            self.token_contract = token_contract;
        }
    }

    fn decimals_of(document: &Document) -> Option<&Value> {
        document.properties().get("decimals")
    }

    fn assert_success(result: &StateTransitionExecutionResult) {
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. },
            "{result:?}"
        );
    }

    #[tokio::test]
    async fn should_write_the_token_decimals_on_a_create_that_leaves_them_out() {
        let mut fixture = OrderFixture::new();

        let transition = fixture
            .create_transition(OrderFixture::order_data(fixture.token_id, None), 1)
            .await;
        assert_success(&fixture.process(&transition));

        let stored = fixture.stored_orders();
        assert_eq!(stored.len(), 1);
        assert_eq!(
            decimals_of(&stored[0]).and_then(|value| value.to_integer::<u8>().ok()),
            Some(6)
        );
        // A client verifying the proof takes the decimals from the proved document
        assert_matches!(
            fixture.verify_proof(&transition),
            StateTransitionProofResult::VerifiedDocuments(documents)
                if documents.values().flatten().all(|document| {
                    decimals_of(document).and_then(|value| value.to_integer::<u8>().ok())
                        == Some(6)
                })
        );
    }

    #[tokio::test]
    async fn should_accept_the_token_decimals_sent_and_refuse_others() {
        let mut fixture = OrderFixture::new();

        let transition = fixture
            .create_transition(OrderFixture::order_data(fixture.token_id, Some(6)), 2)
            .await;
        assert_success(&fixture.process(&transition));

        let transition = fixture
            .create_transition(OrderFixture::order_data(fixture.token_id, Some(18)), 3)
            .await;
        let result = fixture.process(&transition);
        let StateTransitionExecutionResult::PaidConsensusError { error, .. } = result else {
            panic!("expected a paid consensus error, got {result:?}");
        };
        assert_matches!(
            error,
            ConsensusError::BasicError(BasicError::DocumentPropertyNotGeneratedError(e))
                if e.property() == "decimals" && e.function() == "sys.tokens.decimals"
        );
        assert_eq!(fixture.stored_orders().len(), 1);
    }

    #[tokio::test]
    async fn should_refuse_a_token_that_does_not_exist() {
        let mut fixture = OrderFixture::new();

        let unknown = Identifier::new([77; 32]);
        let transition = fixture
            .create_transition(OrderFixture::order_data(unknown, None), 4)
            .await;
        let result = fixture.process(&transition);
        let StateTransitionExecutionResult::PaidConsensusError { error, .. } = result else {
            panic!("expected a paid consensus error, got {result:?}");
        };
        assert_matches!(
            error,
            ConsensusError::StateError(StateError::ReferencedEntityNotFoundError(e))
                if e.entity_id() == &unknown && e.path() == "tokenId"
        );
        assert!(fixture.stored_orders().is_empty());
    }

    /// The token's decimals change after the order is created: a replace that keeps the
    /// token keeps the decimals the order was created with, a new order takes the new ones,
    /// and a replace naming another token takes that token's.
    #[tokio::test]
    async fn should_keep_the_decimals_until_a_replace_names_another_token() {
        let mut fixture = OrderFixture::new();

        let transition = fixture
            .create_transition(OrderFixture::order_data(fixture.token_id, None), 5)
            .await;
        assert_success(&fixture.process(&transition));
        fixture.change_token_decimals(2);

        let stored = fixture.stored_orders().remove(0);
        let mut data = OrderFixture::order_data(fixture.token_id, None);
        data.insert("note".to_string(), Value::Text("kept".to_string()));
        let transition = fixture.replace_transition(&stored, data).await;
        assert_success(&fixture.process(&transition));
        let replaced = fixture.stored_orders().remove(0);
        assert_eq!(replaced.get("note"), Some(&Value::Text("kept".to_string())));
        assert_eq!(
            decimals_of(&replaced).and_then(|value| value.to_integer::<u8>().ok()),
            Some(6)
        );
        assert_matches!(
            fixture.verify_proof(&transition),
            StateTransitionProofResult::VerifiedDocuments(_)
        );

        // The token's current decimals are refused while the order keeps the token
        let transition = fixture
            .replace_transition(
                &replaced,
                OrderFixture::order_data(fixture.token_id, Some(2)),
            )
            .await;
        assert_matches!(
            fixture.process(&transition),
            StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::BasicError(BasicError::DocumentPropertyNotGeneratedError(_)),
                ..
            }
        );

        // A new order takes the new decimals
        let transition = fixture
            .create_transition(OrderFixture::order_data(fixture.token_id, None), 6)
            .await;
        assert_success(&fixture.process(&transition));
        assert!(fixture.stored_orders().iter().any(|order| {
            decimals_of(order).and_then(|value| value.to_integer::<u8>().ok()) == Some(2)
        }));

        // A replace naming another token takes that token's decimals
        let replaced = fixture
            .stored_orders()
            .into_iter()
            .find(|order| order.id() == stored.id())
            .expect("expected the first order");
        let transition = fixture
            .replace_transition(
                &replaced,
                OrderFixture::order_data(fixture.other_token_id, None),
            )
            .await;
        assert_success(&fixture.process(&transition));
        let moved = fixture
            .stored_orders()
            .into_iter()
            .find(|order| order.id() == stored.id())
            .expect("expected the first order");
        assert_eq!(
            decimals_of(&moved).and_then(|value| value.to_integer::<u8>().ok()),
            Some(18)
        );
    }
}
