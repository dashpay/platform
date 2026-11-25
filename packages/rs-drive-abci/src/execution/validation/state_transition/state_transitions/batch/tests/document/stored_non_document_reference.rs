//! A contract registered at protocol version 14 whose identity references carry `contractId`
//! and `documentType` written `"$self.<property>"` (on a property, in an operand of a
//! reference expression and in `ownerRefersTo`), keys protocol version 14 does not read on a
//! non-document reference. From protocol version 15 the contract loads as it did, its
//! documents are judged as they were, and its expiring documents expire.

use super::*;

mod stored_non_document_reference_tests {
    use super::*;
    use crate::platform_types::platform_state::PlatformState;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TempPlatform;
    use dpp::consensus::codes::ErrorWithCode;
    use dpp::data_contract::accessors::v0::DataContractV0Setters;
    use dpp::data_contract::document_type::DocumentTypeRef;
    use dpp::data_contract::schema::DataContractSchemaMethodsV0;
    use dpp::document::Document;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::identity::{Identity, IdentityPublicKey};
    use dpp::platform_value::platform_value;
    use dpp::prelude::{DataContract, IdentityNonce};
    use dpp::state_transition::data_contract_create_transition::methods::DataContractCreateTransitionMethodsV0;
    use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
    use dpp::state_transition::StateTransition;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use drive::drive::RootTree;
    use drive::util::grove_operations::DirectQueryType;
    use simple_signer::signer::SimpleSigner;
    use std::collections::BTreeMap;
    use std::sync::Arc;

    const START_MS: u64 = 1_700_000_000_000;
    const HOUR_MS: u64 = 3_600_000;
    /// The identity nonce the contract is created with; the identity's nonce for the
    /// contract starts after it.
    const CREATE_NONCE: IdentityNonce = 1;

    fn identifier_property(position: u64) -> Value {
        platform_value!({
            "type": "array",
            "byteArray": true,
            "minItems": 32,
            "maxItems": 32,
            "contentMediaType": "application/x.dash.dpp.identifier",
            "position": position,
        })
    }

    /// An identity reference, with `contractId` and `documentType` written `"$self."` when
    /// `self_keys`.
    fn identity_reference(self_keys: bool) -> Value {
        let mut reference = platform_value!({ "type": "identity" });
        if self_keys {
            for (key, read) in [("contractId", "$self.text"), ("documentType", "$self.text")] {
                reference
                    .insert(key.to_string(), Value::Text(read.to_string()))
                    .expect("expected to set the key");
            }
        }
        reference
    }

    /// `note`'s properties: `authorId`, an identity reference, `subjectId`, an identity or a
    /// `post`, and `text`; the identity references carrying `"$self."` keys when `self_keys`.
    fn note_properties(self_keys: bool) -> Value {
        let mut author_id = identifier_property(0);
        author_id
            .insert("refersTo".to_string(), identity_reference(self_keys))
            .expect("expected to declare the reference");
        let mut subject_id = identifier_property(1);
        subject_id
            .insert(
                "refersTo".to_string(),
                platform_value!({ "anyOf": [
                    identity_reference(self_keys),
                    { "type": "permanentDocument", "documentType": "post" },
                ] }),
            )
            .expect("expected to declare the reference");
        platform_value!({
            "authorId": author_id,
            "subjectId": subject_id,
            "text": { "type": "string", "maxLength": 63, "position": 2 },
        })
    }

    /// A `note` type whose documents expire an hour after creation, its identity references
    /// carrying the `"$self."` keys.
    fn note_schema() -> Value {
        let text = |value: &str| Value::Text(value.to_string());
        Value::Map(vec![
            (text("type"), text("object")),
            (text("ttl"), Value::U64(HOUR_MS / 1000)),
            (text("properties"), note_properties(true)),
            (text("properties"), note_properties(false)),
            (text("ownerRefersTo"), identity_reference(true)),
            (text("ownerRefersTo"), identity_reference(false)),
            (
                text("required"),
                platform_value!(["$createdAt", "authorId", "subjectId", "text"]),
            ),
            (text("additionalProperties"), Value::Bool(false)),
        ])
    }

    fn post_schema() -> Value {
        platform_value!({
            "type": "object",
            "canBeDeleted": false,
            "documentsMutable": false,
            "properties": {
                "title": { "type": "string", "maxLength": 63, "position": 0 },
            },
            "additionalProperties": false,
        })
    }

    fn block(time_ms: u64) -> BlockInfo {
        BlockInfo {
            time_ms,
            ..Default::default()
        }
    }

    fn code_of(result: &StateTransitionExecutionResult) -> u32 {
        match result {
            StateTransitionExecutionResult::PaidConsensusError { error, .. } => error.code(),
            other => panic!("expected a paid consensus error, got {other:?}"),
        }
    }

    fn assert_success(result: &StateTransitionExecutionResult) {
        assert!(
            matches!(
                result,
                StateTransitionExecutionResult::SuccessfulExecution { .. }
            ),
            "expected a successful execution, got {result:?}"
        );
    }

    struct NotesFixture {
        platform: TempPlatform<MockCoreRPCLike>,
        signer: SimpleSigner,
        key: IdentityPublicKey,
        identity: Identity,
        contract: DataContract,
        next_nonce: IdentityNonce,
        /// The protocol version the chain runs and every write is processed at.
        platform_version: &'static PlatformVersion,
    }

    impl NotesFixture {
        /// A chain at protocol version 14 on which the notes contract was registered through a
        /// contract create.
        async fn registered_at_protocol_version_14() -> Self {
            let platform_version = PlatformVersion::get(14).expect("protocol version 14");
            let mut platform = TestPlatformBuilder::new()
                .with_initial_protocol_version(platform_version.protocol_version)
                .build_with_mock_rpc()
                .with_no_core_masternodes()
                .set_genesis_state();
            let (identity, signer, key) = setup_identity(&mut platform, 5_281, dash_to_credits!(1));

            let mut contract = get_data_contract_fixture(
                Some(identity.id()),
                CREATE_NONCE,
                platform_version.protocol_version,
            )
            .data_contract_owned();
            contract.set_id(DataContract::generate_data_contract_id_v0(
                identity.id(),
                CREATE_NONCE,
            ));
            contract
                .set_document_schemas(
                    BTreeMap::from([
                        ("note".to_string(), note_schema()),
                        ("post".to_string(), post_schema()),
                    ]),
                    None,
                    false,
                    &mut Vec::new(),
                    platform_version,
                )
                .expect("expected to set the document types");

            let mut fixture = Self {
                platform,
                signer,
                key,
                identity,
                contract,
                next_nonce: CREATE_NONCE + 1,
                platform_version,
            };
            let create = DataContractCreateTransition::new_from_data_contract(
                fixture.contract.clone(),
                CREATE_NONCE,
                &fixture.identity.clone().into_partial_identity_info(),
                fixture.key.id(),
                &fixture.signer,
                platform_version,
                None,
            )
            .await
            .expect("expected the contract create");
            assert_success(&fixture.process(&create, START_MS));
            fixture
        }

        /// Runs the first block of protocol version 15, which reads every contract afresh,
        /// and moves the chain to it.
        fn upgrade_to_the_latest_protocol_version(&mut self) {
            let latest = PlatformVersion::latest();
            let transaction = self.platform.drive.grove.start_transaction();
            let platform_state = self.platform.state.load();
            let mut block_platform_state = PlatformState::clone(&platform_state);
            block_platform_state.set_current_protocol_version_in_consensus(latest.protocol_version);
            self.platform
                .perform_events_on_first_block_of_protocol_change(
                    &platform_state,
                    &mut block_platform_state,
                    &block(START_MS + 1_000),
                    &transaction,
                    self.platform_version.protocol_version,
                    latest,
                )
                .expect("expected the protocol change to run");
            self.platform
                .drive
                .commit_transaction(transaction, &latest.drive)
                .expect("expected to commit the protocol change");
            self.platform.state.store(Arc::new(block_platform_state));
            self.platform_version = latest;
        }

        fn note_type(&self) -> DocumentTypeRef<'_> {
            self.contract
                .document_type_for_name("note")
                .expect("expected the note type")
        }

        /// Creates a note by the fixture's identity whose `authorId` is `author_id` at
        /// `time_ms`.
        async fn create_note(
            &mut self,
            author_id: Identifier,
            time_ms: u64,
        ) -> (Document, StateTransitionExecutionResult) {
            let platform_version = self.platform_version;
            let document_type = self.note_type();
            let mut rng = StdRng::seed_from_u64(5_300 + self.next_nonce);
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
            document.set("authorId", Value::Identifier(author_id.to_buffer()));
            document.set(
                "subjectId",
                Value::Identifier(self.identity.id().to_buffer()),
            );
            document.set("text", Value::Text("hello".to_string()));

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

        fn process(
            &self,
            transition: &StateTransition,
            time_ms: u64,
        ) -> StateTransitionExecutionResult {
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
                    &block(time_ms),
                    &transaction,
                    self.platform_version,
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
                .expire_documents(&block(time_ms), &transaction, self.platform_version)
                .expect("expected the cleanup to run");
            self.platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit the transaction");
        }

        fn note_is_stored(&self, id: Identifier) -> bool {
            // [DataContractDocuments, contract id, 1 (documents), document type, 0 (primary key)]
            let path = vec![
                vec![RootTree::DataContractDocuments as u8],
                self.contract.id().to_vec(),
                vec![1],
                b"note".to_vec(),
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
                    &self.platform_version.drive,
                )
                .expect("expected to read the note")
                .is_some()
        }
    }

    #[tokio::test]
    async fn should_load_judge_and_expire_documents_of_a_contract_registered_at_protocol_version_14_with_self_keys_on_identity_references(
    ) {
        let mut fixture = NotesFixture::registered_at_protocol_version_14().await;

        // At protocol version 14: a note by an existing author is written, one by an
        // identity that does not exist refused
        let author = fixture.identity.id();
        let (expiring, result) = fixture.create_note(author, START_MS).await;
        assert_success(&result);
        let (_, result) = fixture
            .create_note(Identifier::new([0xA7; 32]), START_MS)
            .await;
        let refusal_at_14 = code_of(&result);

        fixture.upgrade_to_the_latest_protocol_version();

        // The contract loads at protocol version 15
        fixture
            .platform
            .drive
            .fetch_contract(
                fixture.contract.id().to_buffer(),
                None,
                None,
                None,
                fixture.platform_version,
            )
            .value
            .expect("the contract should load at the latest protocol version")
            .expect("the contract should exist");

        // Its documents are judged as they were
        let (_, result) = fixture.create_note(author, START_MS + 2_000).await;
        assert_success(&result);
        let (_, result) = fixture
            .create_note(Identifier::new([0xA8; 32]), START_MS + 2_000)
            .await;
        assert_eq!(code_of(&result), refusal_at_14);

        // and the note written at protocol version 14 expires
        assert!(fixture.note_is_stored(expiring.id()));
        fixture.expire(START_MS + HOUR_MS);
        assert!(
            !fixture.note_is_stored(expiring.id()),
            "the block's cleanup deletes the expired note"
        );
    }
}
