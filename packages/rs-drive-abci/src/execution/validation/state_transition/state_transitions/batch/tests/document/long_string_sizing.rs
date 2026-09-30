//! Document writes on a type whose string property holds 16384 or more
//! characters and declares no `maxBytes`. Such a contract is valid at every
//! protocol version: the meta-schemas bound `maxLength` only when `pattern` or
//! `format` is set. At four bytes a character its byte bound does not fit in
//! the `u16` a document type's estimated size is computed in, which the fee
//! estimate of every create, replace and delete reads, in `check_tx` and in
//! the block. From protocol version 14 the estimate holds that bound at
//! `u16::MAX`; before it, the estimate fails and so does the write.

use super::*;

mod long_string_sizing_tests {
    use super::*;
    use crate::error::Error;
    use crate::execution::check_tx::{CheckTxLevel, CheckTxResult};
    use crate::platform_types::platform::PlatformRef;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TempPlatform;
    use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
    use dpp::data_contract::document_type::DocumentTypeRef;
    use dpp::data_contract::DataContractFactory;
    use dpp::document::Document;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::identity::{Identity, IdentityPublicKey, IdentityV0};
    use dpp::platform_value::platform_value;
    use dpp::prelude::{DataContract, IdentityNonce};
    use dpp::state_transition::data_contract_create_transition::methods::DataContractCreateTransitionMethodsV0;
    use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
    use dpp::state_transition::StateTransition;
    use dpp::validation::ValidationResult;
    use dpp::version::ProtocolVersion;
    use drive::util::object_size_info::DocumentInfo::DocumentRefInfo;
    use drive::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
    use simple_signer::signer::SimpleSigner;
    use std::collections::BTreeMap;

    /// What one transition met in `check_tx` and in the block.
    struct Outcome {
        check_tx: Result<ValidationResult<CheckTxResult, ConsensusError>, Error>,
        processed: StateTransitionExecutionResult,
    }

    impl Outcome {
        fn assert_successful(&self) {
            assert_matches!(&self.check_tx, Ok(result) if result.is_valid());
            assert_matches!(
                self.processed,
                StateTransitionExecutionResult::SuccessfulExecution { .. }
            );
        }

        /// The estimated size failing on the string's byte bound, in both.
        fn assert_size_overflow(&self) {
            assert_matches!(
                &self.check_tx,
                Err(error) if error.to_string().contains("max_byte_size overflow")
            );
            assert_matches!(
                &self.processed,
                StateTransitionExecutionResult::InternalError(error)
                    if error.contains("max_byte_size overflow")
            );
        }
    }

    /// One identity and one contract registered through a contract create
    /// transition, whose `note` type holds a `text` of up to 20000 characters
    /// without `maxBytes`.
    struct NoteFixture {
        platform: TempPlatform<MockCoreRPCLike>,
        platform_version: &'static PlatformVersion,
        signer: SimpleSigner,
        key: IdentityPublicKey,
        identity: Identity,
        contract: DataContract,
        /// The identity contract nonce the next document transition uses.
        next_nonce: IdentityNonce,
    }

    impl NoteFixture {
        async fn new(protocol_version: ProtocolVersion) -> Self {
            let platform_version =
                PlatformVersion::get(protocol_version).expect("expected a known version");
            let platform = TestPlatformBuilder::new()
                .with_initial_protocol_version(protocol_version)
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(20000);
            let mut signer = SimpleSigner::default();
            let (master_key, master_private_key) =
                IdentityPublicKey::random_ecdsa_master_authentication_key_with_rng(
                    0,
                    &mut rng,
                    platform_version,
                )
                .expect("expected a master key");
            signer.add_identity_public_key(master_key.clone(), master_private_key);
            let (key, private_key) =
                IdentityPublicKey::random_ecdsa_critical_level_authentication_key_with_rng(
                    1,
                    &mut rng,
                    platform_version,
                )
                .expect("expected a critical key");
            signer.add_identity_public_key(key.clone(), private_key);
            let identity: Identity = IdentityV0 {
                id: Identifier::random_with_rng(&mut rng),
                public_keys: BTreeMap::from([(0, master_key), (1, key.clone())]),
                balance: dash_to_credits!(1),
                revision: 0,
            }
            .into();
            platform
                .drive
                .add_new_identity(
                    identity.clone(),
                    false,
                    &BlockInfo::default(),
                    true,
                    None,
                    platform_version,
                )
                .expect("expected to add the identity");

            let contract = DataContractFactory::new(protocol_version)
                .expect("expected a factory")
                .create_with_value_config(
                    identity.id(),
                    1,
                    platform_value!({
                        "note": {
                            "type": "object",
                            "documentsMutable": true,
                            "canBeDeleted": true,
                            "properties": {
                                "text": {
                                    "type": "string",
                                    "maxLength": 20000,
                                    "position": 0
                                }
                            },
                            "required": ["text"],
                            "additionalProperties": false
                        }
                    }),
                    None,
                    None,
                )
                .expect("expected the contract to be created")
                .data_contract_owned();
            let transition = DataContractCreateTransition::new_from_data_contract(
                contract.clone(),
                1,
                &identity.clone().into_partial_identity_info(),
                key.id(),
                &signer,
                platform_version,
                None,
            )
            .await
            .expect("expected the contract create transition");

            let fixture = Self {
                platform,
                platform_version,
                signer,
                key,
                identity,
                contract,
                // The contract create took nonce 1 of the new contract
                next_nonce: 2,
            };
            // The contract itself registers at every protocol version
            fixture.check_and_process(&transition).assert_successful();
            fixture
        }

        fn note_type(&self) -> DocumentTypeRef<'_> {
            self.contract
                .document_type_for_name("note")
                .expect("expected the note type")
        }

        /// A new note, with the id its create transition at `next_nonce` gives it.
        fn new_note(&self, text: &str) -> (Document, [u8; 32]) {
            let entropy = [7u8; 32];
            let mut note = self
                .note_type()
                .create_document_from_data(
                    Value::from(BTreeMap::from([(
                        "text".to_string(),
                        Value::Text(text.to_string()),
                    )])),
                    self.identity.id(),
                    0,
                    0,
                    entropy,
                    self.platform_version,
                )
                .expect("expected a note");
            note.set_id_for_creation(
                self.note_type(),
                &entropy,
                self.next_nonce,
                self.platform_version,
            )
            .expect("expected the note id");
            (note, entropy)
        }

        async fn create(&mut self, note: Document, entropy: [u8; 32]) -> Outcome {
            let transition = BatchTransition::new_document_creation_transition_from_document(
                note,
                self.note_type(),
                entropy,
                &self.key,
                self.next_nonce,
                0,
                None,
                &self.signer,
                self.platform_version,
                None,
            )
            .await
            .expect("expected the create transition");
            self.next_nonce += 1;
            self.check_and_process(&transition)
        }

        async fn replace(&mut self, stored: &Document, text: &str) -> Outcome {
            let mut replacement = stored.clone();
            replacement.set("text", Value::Text(text.to_string()));
            replacement
                .increment_revision()
                .expect("expected the revision to increment");
            let transition = BatchTransition::new_document_replacement_transition_from_document(
                replacement,
                self.note_type(),
                &self.key,
                self.next_nonce,
                0,
                None,
                &self.signer,
                self.platform_version,
                None,
            )
            .await
            .expect("expected the replace transition");
            self.next_nonce += 1;
            self.check_and_process(&transition)
        }

        async fn delete(&mut self, stored: &Document) -> Outcome {
            let transition = BatchTransition::new_document_deletion_transition_from_document(
                stored.clone(),
                self.note_type(),
                &self.key,
                self.next_nonce,
                0,
                None,
                &self.signer,
                self.platform_version,
                None,
            )
            .await
            .expect("expected the delete transition");
            self.next_nonce += 1;
            self.check_and_process(&transition)
        }

        /// Writes `note` straight through Drive, which sizes nothing when it
        /// applies, so replace and delete can be exercised where the create
        /// transition fails.
        fn seed(&self, note: &Document) {
            self.platform
                .drive
                .add_document_for_contract(
                    DocumentAndContractInfo {
                        owned_document_info: OwnedDocumentInfo {
                            document_info: DocumentRefInfo((note, None)),
                            owner_id: None,
                        },
                        contract: &self.contract,
                        document_type: self.note_type(),
                    },
                    false,
                    BlockInfo::default(),
                    true,
                    None,
                    self.platform_version,
                    None,
                )
                .expect("expected to seed the note");
        }

        fn check_and_process(&self, transition: &StateTransition) -> Outcome {
            let serialized = transition
                .serialize_to_bytes()
                .expect("expected the transition to serialize");
            let platform_state = self.platform.state.load();
            let platform_ref = PlatformRef {
                drive: &self.platform.drive,
                state: &platform_state,
                config: &self.platform.config,
                core_rpc: &self.platform.core_rpc,
            };
            let check_tx = self.platform.check_tx(
                &serialized,
                CheckTxLevel::FirstTimeCheck,
                &platform_ref,
                self.platform_version,
            );

            let transaction = self.platform.drive.grove.start_transaction();
            let processing_result = self
                .platform
                .platform
                .process_raw_state_transitions(
                    &[serialized],
                    &platform_state,
                    &BlockInfo::default(),
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
            Outcome {
                check_tx,
                processed: processing_result.into_execution_results().remove(0),
            }
        }

        fn stored_notes(&self) -> Vec<Document> {
            let query = DriveDocumentQuery::from_sql_expr(
                "select * from note",
                &self.contract,
                Some(&self.platform.config.drive),
                self.platform_version,
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

    #[tokio::test]
    async fn should_create_replace_and_delete_a_document_with_a_string_of_20000_characters() {
        let mut fixture = NoteFixture::new(PlatformVersion::latest().protocol_version).await;

        let (note, entropy) = fixture.new_note("hello");
        fixture.create(note, entropy).await.assert_successful();
        let stored = fixture.stored_notes();
        assert_eq!(stored.len(), 1);

        fixture
            .replace(&stored[0], "hello again")
            .await
            .assert_successful();
        let stored = fixture.stored_notes();
        assert_eq!(
            stored[0].get("text"),
            Some(&Value::Text("hello again".to_string()))
        );

        fixture.delete(&stored[0]).await.assert_successful();
        assert!(fixture.stored_notes().is_empty());
    }

    /// Protocol version 13 selects the estimated size generation that fails
    /// on the bound, so each write still fails as an internal error there.
    #[tokio::test]
    async fn should_fail_writes_on_a_string_of_20000_characters_at_protocol_version_13() {
        let mut fixture = NoteFixture::new(13).await;

        let (note, entropy) = fixture.new_note("hello");
        fixture
            .create(note.clone(), entropy)
            .await
            .assert_size_overflow();
        assert!(fixture.stored_notes().is_empty());

        fixture.seed(&note);
        let stored = fixture.stored_notes();
        assert_eq!(stored.len(), 1);

        fixture
            .replace(&stored[0], "hello again")
            .await
            .assert_size_overflow();
        fixture.delete(&stored[0]).await.assert_size_overflow();
        assert_eq!(fixture.stored_notes(), stored);
    }
}
