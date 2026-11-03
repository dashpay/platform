//! End-to-end coverage for `generatedFrom` with `sys.random.uniform` (protocol version 15): an
//! integer property the platform writes with a placeholder, its range's minimum, when the
//! document is created, and draws from the block's random beacon once the block's transitions
//! are accepted (`Drive::draw_random_document_properties`). A client leaves the property out
//! and takes the drawn value from the proof of its own transition; one that sends a value is
//! refused, and a replace keeps the drawn value. The block that switches to protocol version 15
//! receives no random beacon, so it refuses a create of such a type, which the next block
//! accepts.

use super::*;

mod generated_from_random_tests {
    use super::*;
    use crate::platform_types::platform_state::PlatformState;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TempPlatform;
    use dpp::consensus::basic::BasicError;
    use dpp::data_contract::schema::DataContractSchemaMethodsV0;
    use dpp::document::Document;
    use dpp::identity::{Identity, IdentityPublicKey, SecurityLevel};
    use dpp::platform_value::platform_value;
    use dpp::prelude::{DataContract, Identifier, IdentityNonce};
    use dpp::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
    use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransitionV0Methods;
    use dpp::state_transition::batch_transition::batched_transition::BatchedTransitionMutRef;
    use dpp::state_transition::proof_result::StateTransitionProofResult;
    use dpp::state_transition::StateTransition;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use drive::drive::document::draw_random_document_properties::RandomDocumentRequest;
    use drive::drive::Drive;
    use drive::util::storage_flags::StorageFlags;
    use simple_signer::signer::SimpleSigner;
    use std::collections::BTreeMap;
    use std::sync::Arc;

    const BEACON: [u8; 96] = [7; 96];

    /// A mutable `card` type: its `name`, a `rarity` drawn from 1 to 100 and indexed, and a
    /// free `note`.
    fn card_schema() -> Value {
        platform_value!({
            "type": "object",
            "documentsMutable": true,
            "indices": [{ "name": "byRarity", "properties": [{ "rarity": "asc" }] }],
            "properties": {
                "name": { "type": "string", "maxLength": 32, "position": 0 },
                "rarity": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 100,
                    "generatedFrom": { "function": "sys.random.uniform" },
                    "position": 1
                },
                "note": { "type": "string", "maxLength": 32, "position": 2 }
            },
            "required": ["name", "rarity"],
            "additionalProperties": false
        })
    }

    /// One identity holding a `card` contract.
    struct CardFixture {
        platform: TempPlatform<MockCoreRPCLike>,
        signer: SimpleSigner,
        key: IdentityPublicKey,
        identity: Identity,
        contract: DataContract,
        next_nonce: IdentityNonce,
    }

    impl CardFixture {
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
                    "card",
                    card_schema(),
                    true,
                    &mut Vec::new(),
                    platform_version,
                )
                .expect("expected to add the card document type");
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
                next_nonce: 1,
            }
        }

        fn card_data(rarity: Option<u8>, note: Option<&str>) -> BTreeMap<String, Value> {
            let mut data = BTreeMap::from([("name".to_string(), Value::Text("Ace".to_string()))]);
            if let Some(rarity) = rarity {
                data.insert("rarity".to_string(), Value::U8(rarity));
            }
            if let Some(note) = note {
                data.insert("note".to_string(), Value::Text(note.to_string()));
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

        /// A create transition for a card holding exactly `data`.
        async fn create_transition(
            &mut self,
            data: BTreeMap<String, Value>,
            seed: u64,
        ) -> StateTransition {
            let platform_version = PlatformVersion::latest();
            let document_type = self
                .contract
                .document_type_for_name("card")
                .expect("expected the card document type");
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
                .document_type_for_name("card")
                .expect("expected the card document type");
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
            self.process_in_block(transition, &self.platform.state.load())
        }

        /// Processes `transition` in a block whose platform state is `platform_state`.
        fn process_in_block(
            &self,
            transition: &StateTransition,
            platform_state: &PlatformState,
        ) -> StateTransitionExecutionResult {
            let platform_version = PlatformVersion::latest();
            let serialized = transition
                .serialize_to_bytes()
                .expect("expected the transition to serialize");
            let transaction = self.platform.drive.grove.start_transaction();
            let processing_result = self
                .platform
                .platform
                .process_raw_state_transitions(
                    &[serialized],
                    platform_state,
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

        /// Draws the rarity of `card` from [`BEACON`], as the block that created it does once
        /// its transitions are accepted.
        fn draw(&self, card: &Document) {
            let drawn = self
                .platform
                .drive
                .draw_random_document_properties(
                    &BEACON,
                    &[RandomDocumentRequest {
                        contract_id: self.contract.id(),
                        document_type_name: "card".to_string(),
                        document_id: card.id(),
                    }],
                    &BlockInfo::default(),
                    None,
                    PlatformVersion::latest(),
                )
                .expect("expected the draw to run");
            assert_eq!(drawn, 1);
        }

        /// Proves the committed state `transition` wrote and verifies the proof as a client
        /// holding the card contract does: it cannot know the drawn value.
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

        fn stored_cards(&self) -> Vec<Document> {
            let platform_version = PlatformVersion::latest();
            let query = DriveDocumentQuery::from_sql_expr(
                "select * from card",
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

    fn rarity_of(document: &Document) -> Option<u8> {
        document
            .properties()
            .get("rarity")
            .and_then(|value| value.to_integer::<u8>().ok())
    }

    /// The rarity of the single document `proof_result` proves, which must be `card_id`'s and
    /// present.
    fn proved_rarity(proof_result: StateTransitionProofResult, card_id: Identifier) -> Option<u8> {
        let StateTransitionProofResult::VerifiedDocuments(documents) = proof_result else {
            panic!("expected the proof to show the documents, got {proof_result:?}");
        };
        assert_eq!(
            documents.len(),
            1,
            "the proof shows the transition's document alone: {documents:?}"
        );
        let document = documents
            .get(&card_id)
            .expect("the proof shows the card the transition wrote")
            .as_ref()
            .expect("the proof shows the card present");
        rarity_of(document)
    }

    fn assert_success(result: &StateTransitionExecutionResult) {
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. },
            "{result:?}"
        );
    }

    fn assert_not_generated(result: StateTransitionExecutionResult) {
        let StateTransitionExecutionResult::PaidConsensusError { error, .. } = result else {
            panic!("expected a paid consensus error, got {result:?}");
        };
        assert_matches!(
            error,
            ConsensusError::BasicError(BasicError::DocumentPropertyNotGeneratedError(e))
                if e.property() == "rarity" && e.function() == "sys.random.uniform"
        );
    }

    #[tokio::test]
    async fn should_write_the_placeholder_then_prove_the_drawn_value() {
        let mut fixture = CardFixture::new();

        let transition = fixture
            .create_transition(CardFixture::card_data(None, None), 1)
            .await;
        assert_success(&fixture.process(&transition));
        let stored = fixture.stored_cards();
        assert_eq!(stored.len(), 1);
        assert_eq!(rarity_of(&stored[0]), Some(1), "the placeholder");
        let card_id = stored[0].id();

        fixture.draw(&stored[0]);
        let drawn = rarity_of(&fixture.stored_cards()[0]).expect("a drawn rarity");
        assert!((1..=100).contains(&drawn));

        // The client that sent the create, which left the rarity out, reads the drawn value
        // from the proof of its own transition
        assert_eq!(
            proved_rarity(fixture.verify_proof(&transition), card_id),
            Some(drawn)
        );
    }

    #[tokio::test]
    async fn should_refuse_a_create_that_picks_the_value() {
        let mut fixture = CardFixture::new();

        let transition = fixture
            .create_transition(CardFixture::card_data(Some(100), None), 2)
            .await;
        assert_not_generated(fixture.process(&transition));
        assert!(fixture.stored_cards().is_empty());
    }

    #[tokio::test]
    async fn should_keep_the_drawn_value_on_a_replace() {
        let mut fixture = CardFixture::new();

        let transition = fixture
            .create_transition(CardFixture::card_data(None, None), 3)
            .await;
        assert_success(&fixture.process(&transition));
        fixture.draw(&fixture.stored_cards()[0]);
        let drawn = fixture.stored_cards().remove(0);
        let rarity = rarity_of(&drawn).expect("a drawn rarity");

        // Left out, the drawn value is kept
        let transition = fixture
            .replace_transition(&drawn, CardFixture::card_data(None, Some("kept")))
            .await;
        assert_success(&fixture.process(&transition));
        let replaced = fixture.stored_cards().remove(0);
        assert_eq!(replaced.get("note"), Some(&Value::Text("kept".to_string())));
        assert_eq!(rarity_of(&replaced), Some(rarity));
        assert_eq!(
            proved_rarity(fixture.verify_proof(&transition), replaced.id()),
            Some(rarity)
        );

        // Another value cannot be picked
        let other = if rarity == 100 { 99 } else { rarity + 1 };
        let transition = fixture
            .replace_transition(&replaced, CardFixture::card_data(Some(other), None))
            .await;
        assert_not_generated(fixture.process(&transition));
        assert_eq!(rarity_of(&fixture.stored_cards()[0]), Some(rarity));
    }

    /// The block that switches to protocol version 15 still runs under Tenderdash consensus
    /// version 1, which sends it no random beacon, so nothing would draw the rarity there and
    /// the card would keep its placeholder: the create is refused unpaid, before anything is
    /// written, and the same create is accepted in the next block, which receives a beacon.
    #[tokio::test]
    async fn should_refuse_a_random_document_in_the_block_switching_to_protocol_version_15_and_accept_it_in_the_next(
    ) {
        let mut fixture = CardFixture::new();
        let transition = fixture
            .create_transition(CardFixture::card_data(None, None), 4)
            .await;

        // The state of the block that switches from protocol version 14
        let mut switching_block_state = PlatformState::clone(&fixture.platform.state.load());
        switching_block_state.set_switched_from_protocol_version(Some(14));
        let result = fixture.process_in_block(&transition, &switching_block_state);
        let StateTransitionExecutionResult::UnpaidConsensusError(error) = result else {
            panic!("expected the create to be refused unpaid, got {result:?}");
        };
        assert_matches!(
            error,
            ConsensusError::BasicError(BasicError::RandomBeaconUnavailableError(e))
                if e.protocol_version() == 15
        );
        assert!(fixture.stored_cards().is_empty());

        // The next block runs under consensus version 2: the same create goes through
        assert_success(&fixture.process(&transition));
        let stored = fixture.stored_cards();
        assert_eq!(stored.len(), 1);
        assert_eq!(
            rarity_of(&stored[0]),
            Some(1),
            "the placeholder, until the draw"
        );
    }
}
