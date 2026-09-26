//! End-to-end coverage of the document type `ttl` keyword (protocol version 14): a document
//! created through a batch transition is stored without storage flags, indexed in the
//! documents expirations tree, priced for the time it lives with its deletion prepaid, and
//! deleted by the platform after the block its time to live passes in, refunding nobody.

use super::*;

mod document_ttl_tests {
    use super::*;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TempPlatform;
    use dpp::data_contract::document_type::DocumentTypeRef;
    use dpp::data_contract::schema::DataContractSchemaMethodsV0;
    use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
    use dpp::document::Document;
    use dpp::fee::fee_result::FeeResult;
    use dpp::identity::{Identity, IdentityPublicKey};
    use dpp::platform_value::platform_value;
    use dpp::prelude::{DataContract, IdentityNonce};
    use dpp::state_transition::StateTransition;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use drive::drive::document::expiration::paths::{
        documents_expirations_path_vec, encode_expiration_time,
    };
    use drive::drive::document::expiration::pricing::document_expiration_cleanup_fee;
    use drive::drive::RootTree;
    use drive::grovedb::Element;
    use drive::util::grove_operations::DirectQueryType;
    use drive::util::storage_flags::StorageFlags;
    use simple_signer::signer::SimpleSigner;

    const START_MS: u64 = 1_700_000_000_000;
    const HOUR_S: u64 = 3_600;

    /// A mutable, transferable and tradeable `note` type expiring an hour after creation, and
    /// a `memo` type identical but for the `ttl`.
    fn note_schema(ttl: Option<u64>) -> Value {
        let mut schema = platform_value!({
            "type": "object",
            "documentsMutable": true,
            "transferable": 1,
            "tradeMode": 1,
            "properties": {
                "text": { "type": "string", "maxLength": 63, "position": 0 },
            },
            "indices": [
                { "name": "byText", "properties": [{ "text": "asc" }] },
            ],
            "required": ["$createdAt", "text"],
            "additionalProperties": false,
        });
        if let Some(ttl) = ttl {
            schema
                .insert("ttl".to_string(), Value::U64(ttl))
                .expect("expected to set the ttl");
        }
        schema
    }

    struct NotesFixture {
        platform: TempPlatform<MockCoreRPCLike>,
        signer: SimpleSigner,
        key: IdentityPublicKey,
        identity: Identity,
        contract: DataContract,
        next_nonce: IdentityNonce,
        /// A second identity, to buy and receive notes
        buyer: Identity,
        buyer_signer: SimpleSigner,
        buyer_key: IdentityPublicKey,
        buyer_next_nonce: IdentityNonce,
    }

    impl NotesFixture {
        fn new() -> Self {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_initial_state_structure();

            let (identity, signer, key) = setup_identity(&mut platform, 971, dash_to_credits!(0.5));
            let (buyer, buyer_signer, buyer_key) =
                setup_identity(&mut platform, 972, dash_to_credits!(0.5));

            let mut contract = get_data_contract_fixture(
                Some(identity.id()),
                0,
                platform_version.protocol_version,
            )
            .data_contract_owned();
            for (name, ttl) in [("note", Some(HOUR_S)), ("memo", None)] {
                contract
                    .set_document_schema(
                        name,
                        note_schema(ttl),
                        true,
                        &mut Vec::new(),
                        platform_version,
                    )
                    .expect("expected to add the document type");
            }
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
                buyer,
                buyer_signer,
                buyer_key,
                buyer_next_nonce: 1,
            }
        }

        fn note_type(&self) -> DocumentTypeRef<'_> {
            self.contract
                .document_type_for_name("note")
                .expect("expected the note type")
        }

        /// The note as stored now, the base of the next change to it.
        fn stored_note(&self, id: Identifier) -> Document {
            let Some(Element::Item(bytes, _)) = self.stored_by_id("note", id) else {
                panic!("expected the note to be stored as an item");
            };
            Document::from_bytes(&bytes, self.note_type(), PlatformVersion::latest())
                .expect("expected the stored note to decode")
        }

        async fn replace(
            &mut self,
            id: Identifier,
            text: &str,
            time_ms: u64,
        ) -> StateTransitionExecutionResult {
            let mut document = self.stored_note(id);
            // A price is not document data: a replace carries only the schema's properties.
            document.properties_mut().remove("$price");
            document.set("text", Value::Text(text.to_string()));
            document.bump_revision();
            let transition = BatchTransition::new_document_replacement_transition_from_document(
                document,
                self.note_type(),
                &self.key,
                self.next_nonce,
                0,
                None,
                &self.signer,
                PlatformVersion::latest(),
                None,
            )
            .await
            .expect("expected the replace transition");
            self.next_nonce += 1;
            self.process(&transition, time_ms)
        }

        async fn transfer(
            &mut self,
            id: Identifier,
            time_ms: u64,
        ) -> StateTransitionExecutionResult {
            let mut document = self.stored_note(id);
            document.bump_revision();
            let transition = BatchTransition::new_document_transfer_transition_from_document(
                document,
                self.note_type(),
                self.buyer.id(),
                &self.key,
                self.next_nonce,
                0,
                None,
                &self.signer,
                PlatformVersion::latest(),
                None,
            )
            .await
            .expect("expected the transfer transition");
            self.next_nonce += 1;
            self.process(&transition, time_ms)
        }

        async fn update_price(
            &mut self,
            id: Identifier,
            price: u64,
            time_ms: u64,
        ) -> StateTransitionExecutionResult {
            let mut document = self.stored_note(id);
            document.bump_revision();
            let transition = BatchTransition::new_document_update_price_transition_from_document(
                document,
                self.note_type(),
                price,
                &self.key,
                self.next_nonce,
                0,
                None,
                &self.signer,
                PlatformVersion::latest(),
                None,
            )
            .await
            .expect("expected the update price transition");
            self.next_nonce += 1;
            self.process(&transition, time_ms)
        }

        async fn purchase(
            &mut self,
            id: Identifier,
            price: u64,
            time_ms: u64,
        ) -> StateTransitionExecutionResult {
            let mut document = self.stored_note(id);
            document.bump_revision();
            let transition = BatchTransition::new_document_purchase_transition_from_document(
                document,
                self.note_type(),
                self.buyer.id(),
                price,
                &self.buyer_key,
                self.buyer_next_nonce,
                0,
                None,
                &self.buyer_signer,
                PlatformVersion::latest(),
                None,
            )
            .await
            .expect("expected the purchase transition");
            self.buyer_next_nonce += 1;
            self.process(&transition, time_ms)
        }

        fn block(&self, time_ms: u64) -> BlockInfo {
            BlockInfo {
                time_ms,
                ..Default::default()
            }
        }

        /// Creates a `document_type_name` document with `text` at `time_ms` and returns it
        /// with the execution result.
        async fn create(
            &mut self,
            document_type_name: &str,
            text: &str,
            time_ms: u64,
        ) -> (Document, StateTransitionExecutionResult) {
            let platform_version = PlatformVersion::latest();
            let document_type = self
                .contract
                .document_type_for_name(document_type_name)
                .expect("expected the document type");
            let mut rng = StdRng::seed_from_u64(7_000 + self.next_nonce);
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
            document.set("text", Value::Text(text.to_string()));

            let transition = BatchTransition::new_document_creation_transition_from_document(
                document.clone(),
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
            let result = self.process(&transition, time_ms);
            (document, result)
        }

        async fn delete(
            &mut self,
            document: &Document,
            time_ms: u64,
        ) -> StateTransitionExecutionResult {
            let platform_version = PlatformVersion::latest();
            let document_type = self
                .contract
                .document_type_for_name("note")
                .expect("expected the note type");
            let transition = BatchTransition::new_document_deletion_transition_from_document(
                document.clone(),
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
            .expect("expected the delete transition");
            self.next_nonce += 1;
            self.process(&transition, time_ms)
        }

        fn process(
            &self,
            transition: &StateTransition,
            time_ms: u64,
        ) -> StateTransitionExecutionResult {
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
                    &self.block(time_ms),
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

        /// Runs the block-end cleanup of a block at `time_ms`.
        fn expire(&self, time_ms: u64) {
            let transaction = self.platform.drive.grove.start_transaction();
            self.platform
                .platform
                .expire_documents(
                    &self.block(time_ms),
                    &transaction,
                    PlatformVersion::latest(),
                )
                .expect("expected the cleanup to run");
            self.platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit the transaction");
        }

        fn stored(&self, document_type_name: &str, document: &Document) -> Option<Element> {
            self.stored_by_id(document_type_name, document.id())
        }

        fn stored_by_id(&self, document_type_name: &str, id: Identifier) -> Option<Element> {
            // [DataContractDocuments, contract id, 1 (documents), document type, 0 (primary key)]
            let path = vec![
                vec![RootTree::DataContractDocuments as u8],
                self.contract.id().to_vec(),
                vec![1],
                document_type_name.as_bytes().to_vec(),
                vec![0],
            ];
            self.platform
                .drive
                .grove_get_raw_optional(
                    path.as_slice().into(),
                    id.as_slice(),
                    DirectQueryType::StatefulDirectQuery,
                    None,
                    &mut vec![],
                    &PlatformVersion::latest().drive,
                )
                .expect("expected to read the document")
        }

        fn balance(&self) -> u64 {
            self.platform
                .drive
                .fetch_identity_balance(
                    self.identity.id().to_buffer(),
                    None,
                    PlatformVersion::latest(),
                )
                .expect("expected to read the balance")
                .expect("expected a balance")
        }

        fn expiring_at(&self, time_ms: u64) -> Vec<Identifier> {
            self.platform
                .drive
                .fetch_expired_documents(time_ms, 128, None, &mut vec![], PlatformVersion::latest())
                .expect("expected to read the expirations")
                .into_iter()
                .map(|expired| expired.document_id)
                .collect()
        }
    }

    fn fee_of(result: &StateTransitionExecutionResult) -> FeeResult {
        match result {
            StateTransitionExecutionResult::SuccessfulExecution { fee_result, .. } => {
                fee_result.clone()
            }
            other => panic!("expected a successful execution, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn should_store_an_expiring_document_without_flags_and_index_its_expiry() {
        let mut fixture = NotesFixture::new();
        let (note, result) = fixture.create("note", "hello", START_MS).await;
        fee_of(&result);

        let Some(Element::Item(_, flags)) = fixture.stored("note", &note) else {
            panic!("expected the note to be stored as an item");
        };
        assert_eq!(
            flags, None,
            "a document with a time to live carries no storage flags"
        );

        let expires_at = START_MS + HOUR_S * 1000;
        assert!(fixture.expiring_at(expires_at - 1).is_empty());
        assert_eq!(fixture.expiring_at(expires_at), vec![note.id()]);
    }

    #[tokio::test]
    async fn should_price_an_hour_long_document_below_a_permanent_one_and_prepay_its_deletion() {
        let mut fixture = NotesFixture::new();
        // The identity's first transition on the contract stores its contract nonce, at the
        // perpetual storage price; the two compared below only replace it.
        fixture.create("memo", "warm up", START_MS).await;
        let (_, note_result) = fixture.create("note", "hello", START_MS).await;
        let (_, memo_result) = fixture.create("memo", "hello", START_MS).await;
        let note_fee = fee_of(&note_result);
        let memo_fee = fee_of(&memo_result);

        assert_eq!(
            note_fee.storage_fee, 0,
            "an hour of storage pays into the processing fees, not the storage pool"
        );
        assert!(memo_fee.storage_fee > 0);
        let cleanup_fee = document_expiration_cleanup_fee(
            fixture
                .contract
                .document_type_for_name("note")
                .expect("expected the note type"),
            &PlatformVersion::latest().fee_version,
        )
        .expect("expected the cleanup fee");
        // The note's own processing, its bytes' hour of storage included, stays near the
        // memo's; on top of it the note prepays its deletion. Drive's expiration tests pin
        // the prepaid amount exactly.
        let prepaid = note_fee
            .processing_fee
            .checked_sub(memo_fee.processing_fee)
            .expect("the note pays more processing than the memo");
        assert!(
            prepaid.abs_diff(cleanup_fee) <= 100_000,
            "the note prepays its deletion as processing: {prepaid} beyond the memo, the \
             deletion costs {cleanup_fee}"
        );
        assert!(
            note_fee.total_base_fee() < memo_fee.total_base_fee(),
            "an hour of storage costs less than perpetual storage"
        );
    }

    #[tokio::test]
    async fn should_delete_expired_documents_after_the_block_and_refund_nobody() {
        let mut fixture = NotesFixture::new();
        let (first, _) = fixture.create("note", "first", START_MS).await;
        let (second, _) = fixture.create("note", "second", START_MS).await;
        let (later, _) = fixture.create("note", "later", START_MS + 60_000).await;
        let (memo, _) = fixture.create("memo", "stays", START_MS).await;
        let expires_at = START_MS + HOUR_S * 1000;

        // A block a millisecond early deletes nothing.
        fixture.expire(expires_at - 1);
        assert!(fixture.stored("note", &first).is_some());

        let balance_before = fixture.balance();
        fixture.expire(expires_at);
        assert!(fixture.stored("note", &first).is_none());
        assert!(fixture.stored("note", &second).is_none());
        assert!(
            fixture.stored("note", &later).is_some(),
            "it expires a minute later"
        );
        assert!(
            fixture.stored("memo", &memo).is_some(),
            "a memo never expires"
        );
        assert_eq!(
            fixture.balance(),
            balance_before,
            "the cleanup refunds nothing and charges nobody"
        );
        assert_eq!(fixture.expiring_at(u64::MAX), vec![later.id()]);

        fixture.expire(expires_at + 60_000);
        assert!(fixture.stored("note", &later).is_none());
        assert!(fixture.expiring_at(u64::MAX).is_empty());
    }

    fn assert_expired(result: &StateTransitionExecutionResult) {
        match result {
            StateTransitionExecutionResult::PaidConsensusError { error, .. } => assert!(
                matches!(
                    error,
                    ConsensusError::StateError(StateError::DocumentExpiredError(_))
                ),
                "expected the document to be refused as expired, got {error:?}"
            ),
            other => panic!("expected a paid refusal, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn should_refuse_changing_an_expired_document_its_owner_may_still_delete() {
        let mut fixture = NotesFixture::new();
        let (note, _) = fixture.create("note", "hello", START_MS).await;
        let note_id = note.id();
        let expires_at = START_MS + HOUR_S * 1000;

        // While it has time to live, it changes as any note does.
        fee_of(&fixture.replace(note_id, "edited", START_MS + 60_000).await);
        fee_of(
            &fixture
                .update_price(note_id, 1_000_000, START_MS + 120_000)
                .await,
        );

        // From its expiry on, before the cleanup reaches it, nothing changes it any more.
        assert_expired(&fixture.replace(note_id, "too late", expires_at).await);
        assert_expired(&fixture.transfer(note_id, expires_at).await);
        assert_expired(&fixture.update_price(note_id, 2_000_000, expires_at).await);
        assert_expired(&fixture.purchase(note_id, 1_000_000, expires_at + 1).await);
        let stored = fixture.stored_note(note_id);
        assert_eq!(
            stored.owner_id(),
            fixture.identity.id(),
            "still the owner's"
        );
        assert_eq!(
            stored.properties().get("text"),
            Some(&Value::Text("edited".to_string()))
        );

        // Its owner may still delete it: that only removes it sooner.
        let result = fixture.delete(&stored, expires_at + 2).await;
        fee_of(&result);
        assert!(fixture.stored("note", &stored).is_none());
        assert!(fixture.expiring_at(u64::MAX).is_empty());
    }

    #[tokio::test]
    async fn should_sell_an_expiring_document_before_it_expires() {
        let mut fixture = NotesFixture::new();
        let (note, _) = fixture.create("note", "hello", START_MS).await;
        let note_id = note.id();
        fee_of(
            &fixture
                .update_price(note_id, 1_000_000, START_MS + 1_000)
                .await,
        );
        fee_of(&fixture.purchase(note_id, 1_000_000, START_MS + 2_000).await);
        let stored = fixture.stored_note(note_id);
        assert_eq!(stored.owner_id(), fixture.buyer.id());
        // The buyer bought what was left of its life: it expires when it always did.
        assert_eq!(fixture.expiring_at(START_MS + HOUR_S * 1000), vec![note_id]);
    }

    #[tokio::test]
    async fn should_let_the_owner_delete_an_expiring_document_without_a_refund() {
        let mut fixture = NotesFixture::new();
        let (note, _) = fixture.create("note", "hello", START_MS).await;

        let result = fixture.delete(&note, START_MS + 60_000).await;
        let fee = fee_of(&result);
        assert!(
            fee.fee_refunds.0.is_empty(),
            "a document with a time to live refunds nothing to its owner"
        );
        assert!(fixture.stored("note", &note).is_none());
        assert!(
            fixture.expiring_at(u64::MAX).is_empty(),
            "the deletion removes the document's expirations tree entry"
        );
        // It was the last entry of its expiry time, so the tree of that time went with it.
        let expiry_tree = fixture
            .platform
            .drive
            .grove_get_raw_optional(
                documents_expirations_path_vec().as_slice().into(),
                &encode_expiration_time(START_MS + HOUR_S * 1000),
                DirectQueryType::StatefulDirectQuery,
                None,
                &mut vec![],
                &PlatformVersion::latest().drive,
            )
            .expect("expected to read the expirations tree");
        assert!(expiry_tree.is_none());
    }
}
