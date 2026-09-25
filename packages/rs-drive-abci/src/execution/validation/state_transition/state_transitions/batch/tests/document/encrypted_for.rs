//! End-to-end coverage for the `encryptedFor` property keyword (protocol
//! version 14): a byte array property declaring how its ciphertext was made.
//! Consensus checks only the shape of the bytes, on every create and replace:
//! at least the IV plus one block and a multiple of the block for AES-CBC.
//! A value of the right shape is accepted, a value of any other shape is
//! consensus-rejected and leaves the stored document untouched.

use super::*;

mod encrypted_for_tests {
    use super::*;
    use crate::execution::validation::state_transition::batch::action_validation::document::document_replace_transition_action::DocumentReplaceTransitionActionValidation;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TempPlatform;
    use dpp::consensus::basic::BasicError;
    use dpp::tokens::gas_fees_paid_by::GasFeesPaidBy;
    use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::{DocumentBaseTransitionAction, DocumentBaseTransitionActionV0};
    use drive::state_transition_action::batch::batched_transition::document_transition::document_replace_transition_action::{DocumentReplaceTransitionAction, DocumentReplaceTransitionActionV0};
    use std::collections::{BTreeMap, BTreeSet};
    use dpp::data_contract::schema::DataContractSchemaMethodsV0;
    use dpp::document::Document;
    use dpp::identity::{Identity, IdentityPublicKey};
    use dpp::platform_value::platform_value;
    use dpp::prelude::Identifier;
    use dpp::prelude::{DataContract, IdentityNonce};
    use dpp::state_transition::StateTransition;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use drive::util::storage_flags::StorageFlags;
    use simple_signer::signer::SimpleSigner;

    /// A mutable `secret` type: the recipient identifier, the two key ids,
    /// the `encryptedMessage` declared `encryptedFor`, and an optional `meta`
    /// object whose `blob` is declared the same way (a nested, dotted path).
    /// The byte arrays' own `minItems` are left at 1 so that every rejection
    /// below is the shape check's: the JSON schema's bounds run first, and a
    /// 16-byte value against `minItems: 32` would be refused as a schema error
    /// before the shape check ever saw it.
    fn secret_schema() -> Value {
        platform_value!({
            "type": "object",
            "documentsMutable": true,
            "properties": {
                "recipientId": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "position": 0
                },
                "recipientKeyId": { "type": "integer", "minimum": 0, "maximum": 4294967295_u64, "position": 1 },
                "senderKeyId": { "type": "integer", "minimum": 0, "maximum": 4294967295_u64, "position": 2 },
                "encryptedMessage": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 1,
                    "maxItems": 1040,
                    "position": 3,
                    "encryptedFor": {
                        "recipient": "recipientId",
                        "recipientKey": "recipientKeyId",
                        "senderKey": "senderKeyId",
                        "scheme": "ecdh-secp256k1-aes256-cbc"
                    }
                },
                "meta": {
                    "type": "object",
                    "position": 4,
                    "properties": {
                        "blob": {
                            "type": "array",
                            "byteArray": true,
                            "minItems": 1,
                            "maxItems": 1040,
                            "position": 0,
                            "encryptedFor": {
                                "recipient": "recipientId",
                                "recipientKey": "recipientKeyId",
                                "senderKey": "senderKeyId",
                                "scheme": "ecdh-secp256k1-aes256-cbc"
                            }
                        }
                    },
                    "additionalProperties": false
                }
            },
            "required": ["recipientId", "recipientKeyId", "senderKeyId", "encryptedMessage"],
            "additionalProperties": false
        })
    }

    /// One identity and one contract whose `secret` type is the one above.
    struct SecretFixture {
        platform: TempPlatform<MockCoreRPCLike>,
        signer: SimpleSigner,
        key: IdentityPublicKey,
        identity: Identity,
        contract: DataContract,
        /// The identity contract nonce the next transition uses. Every
        /// processed transition consumes one, including the ones that fail
        /// with a paid consensus error.
        next_nonce: IdentityNonce,
    }

    impl SecretFixture {
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
                    "secret",
                    secret_schema(),
                    true,
                    &mut Vec::new(),
                    platform_version,
                )
                .expect("expected to add the secret document type");
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

        /// A secret addressed to a fixed recipient, whose `encryptedMessage`
        /// is `ciphertext_length` bytes long.
        fn secret(&self, ciphertext_length: usize) -> (Document, [u8; 32]) {
            let platform_version = PlatformVersion::latest();
            let secret_type = self
                .contract
                .document_type_for_name("secret")
                .expect("expected the secret document type");
            let mut rng = StdRng::seed_from_u64(433);
            let entropy = Bytes32::random_with_rng(&mut rng);
            let mut secret = secret_type
                .random_document_with_identifier_and_entropy(
                    &mut rng,
                    self.identity.id(),
                    entropy,
                    DocumentFieldFillType::DoNotFillIfNotRequired,
                    DocumentFieldFillSize::AnyDocumentFillSize,
                    platform_version,
                )
                .expect("expected a random secret");
            secret
                .set_id_for_creation(secret_type, &entropy.0, self.next_nonce, platform_version)
                .expect("expected to set the document id");
            secret.set("recipientId", Value::Identifier([7u8; 32]));
            secret.set("recipientKeyId", Value::U32(3));
            secret.set("senderKeyId", Value::U32(1));
            secret.set(
                "encryptedMessage",
                Value::Bytes(vec![0xAB; ciphertext_length]),
            );
            (secret, entropy.0)
        }

        async fn create(
            &mut self,
            ciphertext_length: usize,
        ) -> (Document, StateTransitionExecutionResult) {
            self.create_with(ciphertext_length, |_| {}).await
        }

        /// Like `create`, with `mutate` applied to the secret before it is sent.
        async fn create_with(
            &mut self,
            ciphertext_length: usize,
            mutate: impl FnOnce(&mut Document),
        ) -> (Document, StateTransitionExecutionResult) {
            let platform_version = PlatformVersion::latest();
            let (mut secret, entropy) = self.secret(ciphertext_length);
            mutate(&mut secret);
            let transition = {
                let secret_type = self
                    .contract
                    .document_type_for_name("secret")
                    .expect("expected the secret document type");
                BatchTransition::new_document_creation_transition_from_document(
                    secret.clone(),
                    secret_type,
                    entropy,
                    &self.key,
                    self.next_nonce,
                    0,
                    None,
                    &self.signer,
                    platform_version,
                    None,
                )
                .await
                .expect("expected the create transition")
            };
            self.next_nonce += 1;
            (secret, self.process(&transition))
        }

        /// Replaces `stored` with its `encryptedMessage` set to
        /// `ciphertext_length` bytes and the revision bumped.
        async fn replace(
            &mut self,
            stored: &Document,
            ciphertext_length: usize,
        ) -> StateTransitionExecutionResult {
            let platform_version = PlatformVersion::latest();
            let mut replacement = stored.clone();
            replacement.set(
                "encryptedMessage",
                Value::Bytes(vec![0xCD; ciphertext_length]),
            );
            replacement
                .increment_revision()
                .expect("expected the revision to increment");
            let transition = {
                let secret_type = self
                    .contract
                    .document_type_for_name("secret")
                    .expect("expected the secret document type");
                BatchTransition::new_document_replacement_transition_from_document(
                    replacement,
                    secret_type,
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
            self.process(&transition)
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

        /// The stored secrets, read back from Drive.
        fn stored_secrets(&self) -> Vec<Document> {
            let platform_version = PlatformVersion::latest();
            let query = DriveDocumentQuery::from_sql_expr(
                "select * from secret",
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

    fn expect_shape_error(result: StateTransitionExecutionResult, actual_length: u32) {
        expect_shape_error_on(result, "encryptedMessage", actual_length);
    }

    fn expect_shape_error_on(
        result: StateTransitionExecutionResult,
        property: &str,
        actual_length: u32,
    ) {
        let StateTransitionExecutionResult::PaidConsensusError { error, .. } = result else {
            panic!("expected a paid consensus error, got {result:?}");
        };
        let ConsensusError::BasicError(BasicError::InvalidEncryptedPropertyShapeError(error)) =
            error
        else {
            panic!("expected an InvalidEncryptedPropertyShapeError, got {error:?}");
        };
        assert_eq!(error.property(), property);
        assert_eq!(error.scheme(), "ecdh-secp256k1-aes256-cbc");
        assert_eq!(error.actual_length(), actual_length);
        assert_eq!(error.minimum_length(), 32);
        assert_eq!(error.block_length(), 16);
    }

    #[tokio::test]
    async fn should_create_a_document_whose_ciphertext_is_an_iv_plus_whole_blocks() {
        let mut fixture = SecretFixture::new();

        let (secret, result) = fixture.create(48).await;

        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        let stored = fixture.stored_secrets();
        assert_eq!(stored.len(), 1);
        assert_eq!(
            stored[0].get("encryptedMessage"),
            secret.get("encryptedMessage")
        );
    }

    #[tokio::test]
    async fn should_refuse_a_ciphertext_that_is_not_a_multiple_of_the_block() {
        let mut fixture = SecretFixture::new();

        let (_, result) = fixture.create(47).await;

        expect_shape_error(result, 47);
        assert!(fixture.stored_secrets().is_empty());
    }

    #[tokio::test]
    async fn should_refuse_a_ciphertext_of_the_iv_alone() {
        let mut fixture = SecretFixture::new();

        let (_, result) = fixture.create(16).await;

        expect_shape_error(result, 16);
        assert!(fixture.stored_secrets().is_empty());
    }

    /// The nested declaration is checked through its dotted path, and a
    /// declared property the document leaves out is not checked at all.
    #[tokio::test]
    async fn should_check_a_nested_encrypted_property_and_skip_an_omitted_one() {
        let mut fixture = SecretFixture::new();

        let (_, result) = fixture
            .create_with(48, |secret| {
                secret.set(
                    "meta",
                    platform_value!({ "blob": Value::Bytes(vec![0xEF; 47]) }),
                );
            })
            .await;
        expect_shape_error_on(result, "meta.blob", 47);
        assert!(fixture.stored_secrets().is_empty());

        let (_, result) = fixture
            .create_with(48, |secret| {
                secret.set(
                    "meta",
                    platform_value!({ "blob": Value::Bytes(vec![0xEF; 64]) }),
                );
            })
            .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        assert_eq!(fixture.stored_secrets().len(), 1);
    }

    /// The replace structure dispatcher on both sides of the gate: structure
    /// generation 0 gained the shape check in place, so at protocol version 13
    /// it must still accept the action (no property parsed there carries the
    /// keyword and the dpp gate is `None`), and at 14 refuse it. The action is
    /// built by hand the way the transformer would build it, against the
    /// contract as Drive hands it back.
    #[test]
    fn should_not_check_the_ciphertext_shape_on_replace_before_protocol_version_14() {
        let platform_version = PlatformVersion::latest();
        let fixture = SecretFixture::new();
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
                document_type_name: "secret".to_string(),
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
                ("recipientId".to_string(), Value::Identifier([7u8; 32])),
                ("recipientKeyId".to_string(), Value::U32(3)),
                ("senderKeyId".to_string(), Value::U32(1)),
                ("encryptedMessage".to_string(), Value::Bytes(vec![0xAB; 47])),
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
            "structure generation 0 must not check the ciphertext shape: {:?}",
            before.errors
        );

        let at = action
            .validate_structure(owner_id, platform_version)
            .expect("structure validation should run");
        assert_matches!(
            at.errors.as_slice(),
            [ConsensusError::BasicError(BasicError::InvalidEncryptedPropertyShapeError(e))]
                if e.property() == "encryptedMessage" && e.actual_length() == 47
        );
    }

    #[tokio::test]
    async fn should_refuse_a_replace_that_shrinks_the_ciphertext_below_the_iv_plus_a_block() {
        let mut fixture = SecretFixture::new();
        let (secret, result) = fixture.create(48).await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );

        let result = fixture.replace(&secret, 16).await;

        expect_shape_error(result, 16);
        let stored = fixture.stored_secrets();
        assert_eq!(stored.len(), 1);
        assert_eq!(
            stored[0].get("encryptedMessage"),
            secret.get("encryptedMessage"),
            "the refused replace must leave the stored ciphertext untouched"
        );

        // A replace of the right shape still goes through
        let result = fixture.replace(&secret, 32).await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
    }
}
