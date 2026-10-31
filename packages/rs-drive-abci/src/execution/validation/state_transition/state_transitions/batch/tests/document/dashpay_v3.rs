//! The DashPay system contract's checks as the contract states them from
//! protocol version 15 (DashPay v3), against the data triggers that made them up
//! to protocol version 14 (DashPay v2, data trigger bindings 2).
//!
//! - A contact request to oneself: `distinctFrom: "$ownerId"` on `toUserId`
//!   (10419), where the trigger refused it (40500).
//! - A contact request to an identity that does not exist: `refersTo` identity on
//!   `toUserId` (40120), where the trigger refused it (40500).
//! - An account label that is not an IV plus whole AES blocks: `encryptedFor`
//!   (10420), accepted up to protocol version 14.
//! - A profile payment address whose type byte is not 0x00 or 0x01, on create and
//!   on replace: the `byteAt` rules (10422), where the trigger refused it (40500).

use super::*;

mod dashpay_v3_tests {
    use super::*;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TempPlatform;
    use dpp::consensus::basic::BasicError;
    use dpp::consensus::state::data_trigger::DataTriggerError;
    use dpp::data_contract::document_type::DocumentPropertyReferenceTarget;
    use dpp::document::Document;
    use dpp::identity::{Identity, IdentityPublicKey};
    use dpp::prelude::{DataContract, IdentityNonce};
    use dpp::state_transition::StateTransition;
    use simple_signer::signer::SimpleSigner;
    use std::sync::Arc;

    /// The last protocol version whose DashPay checks are data triggers.
    const PROTOCOL_VERSION_WITH_TRIGGERS: u32 = 14;

    struct Fixture {
        platform: TempPlatform<MockCoreRPCLike>,
        platform_version: &'static PlatformVersion,
        dashpay: Arc<DataContract>,
        sender: Identity,
        signer: SimpleSigner,
        key: IdentityPublicKey,
        recipient: Identity,
    }

    impl Fixture {
        fn new(protocol_version: u32) -> Self {
            let platform_version =
                PlatformVersion::get(protocol_version).expect("expected a known protocol version");
            let mut platform = TestPlatformBuilder::new()
                .with_initial_protocol_version(protocol_version)
                .build_with_mock_rpc()
                .set_genesis_state();

            let (sender, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));
            let (recipient, ..) = setup_identity(&mut platform, 495, dash_to_credits!(0.1));

            let dashpay = platform
                .drive
                .cache
                .system_data_contracts
                .load_dashpay(platform_version)
                .expect("expected the dashpay system contract");

            Fixture {
                platform,
                platform_version,
                dashpay,
                sender,
                signer,
                key,
                recipient,
            }
        }

        fn latest() -> Self {
            Self::new(PlatformVersion::latest().protocol_version)
        }

        fn with_triggers() -> Self {
            Self::new(PROTOCOL_VERSION_WITH_TRIGGERS)
        }

        /// A random document of the sender, its id derived for a create under `nonce`.
        fn random_document(
            &self,
            document_type_name: &str,
            seed: u64,
            nonce: IdentityNonce,
        ) -> (Document, Bytes32) {
            let document_type = self
                .dashpay
                .document_type_for_name(document_type_name)
                .expect("expected the document type");
            let mut rng = StdRng::seed_from_u64(seed);
            let entropy = Bytes32::random_with_rng(&mut rng);
            let mut document = document_type
                .random_document_with_identifier_and_entropy(
                    &mut rng,
                    self.sender.id(),
                    entropy,
                    DocumentFieldFillType::FillIfNotRequired,
                    DocumentFieldFillSize::AnyDocumentFillSize,
                    self.platform_version,
                )
                .expect("expected a random document");
            document
                .set_id_for_creation(document_type, &entropy.0, nonce, self.platform_version)
                .expect("expected to set the document id");
            (document, entropy)
        }

        /// A contact request from the sender to `to_user_id` whose account label
        /// is `label_length` bytes.
        fn contact_request(
            &self,
            to_user_id: Identifier,
            label_length: usize,
        ) -> (Document, Bytes32) {
            let (mut document, entropy) = self.random_document("contactRequest", 437, 2);
            document.set("toUserId", Value::Identifier(to_user_id.to_buffer()));
            document.set("senderKeyIndex", Value::U32(1));
            document.set("recipientKeyIndex", Value::U32(1));
            document.set("accountReference", Value::U32(0));
            document.set("encryptedPublicKey", Value::Bytes(vec![7u8; 96]));
            document.set(
                "encryptedAccountLabel",
                Value::Bytes(vec![8u8; label_length]),
            );
            (document, entropy)
        }

        /// A profile of the sender whose two payment addresses are valid, to be
        /// created under `nonce`.
        fn profile(&self, nonce: IdentityNonce) -> (Document, Bytes32) {
            let (mut document, entropy) = self.random_document("profile", 438, nonce);
            set_valid_profile_payment_addresses(
                &mut document,
                self.dashpay
                    .document_type_for_name("profile")
                    .expect("expected the profile document type"),
            );
            document.set("avatarUrl", "http://test.com/bob.jpg".into());
            (document, entropy)
        }

        async fn create(
            &self,
            document_type_name: &str,
            document: Document,
            entropy: Bytes32,
            nonce: IdentityNonce,
        ) -> StateTransitionExecutionResult {
            let transition = BatchTransition::new_document_creation_transition_from_document(
                document,
                self.dashpay
                    .document_type_for_name(document_type_name)
                    .expect("expected the document type"),
                entropy.0,
                &self.key,
                nonce,
                0,
                None,
                &self.signer,
                self.platform_version,
                None,
            )
            .await
            .expect("expect to create documents batch transition");
            self.process(transition)
        }

        async fn replace(
            &self,
            document_type_name: &str,
            document: Document,
            nonce: IdentityNonce,
        ) -> StateTransitionExecutionResult {
            let transition = BatchTransition::new_document_replacement_transition_from_document(
                document,
                self.dashpay
                    .document_type_for_name(document_type_name)
                    .expect("expected the document type"),
                &self.key,
                nonce,
                0,
                None,
                &self.signer,
                self.platform_version,
                None,
            )
            .await
            .expect("expect to create documents batch transition");
            self.process(transition)
        }

        fn process(&self, transition: StateTransition) -> StateTransitionExecutionResult {
            let platform_state = self.platform.state.load();
            let transaction = self.platform.drive.grove.start_transaction();
            let processing_result = self
                .platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition
                        .serialize_to_bytes()
                        .expect("expected documents batch serialized state transition")],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    self.platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");
            self.platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");
            processing_result.into_execution_results().remove(0)
        }
    }

    fn paid_error(result: StateTransitionExecutionResult) -> ConsensusError {
        let StateTransitionExecutionResult::PaidConsensusError { error, .. } = result else {
            panic!("expected a paid consensus error, got {result:?}");
        };
        error
    }

    fn is_data_trigger_condition_error(error: &ConsensusError) -> bool {
        matches!(
            error,
            ConsensusError::StateError(StateError::DataTriggerError(
                DataTriggerError::DataTriggerConditionError(_)
            ))
        )
    }

    /// A payment address of the given type byte followed by a 20-byte hash.
    fn payment_address(type_byte: u8) -> Value {
        let mut address = vec![type_byte];
        address.extend([0u8; 20]);
        Value::Bytes(address)
    }

    #[tokio::test]
    async fn should_create_a_contact_request_to_another_identity() {
        for fixture in [Fixture::with_triggers(), Fixture::latest()] {
            let (document, entropy) = fixture.contact_request(fixture.recipient.id(), 48);

            assert_matches!(
                fixture.create("contactRequest", document, entropy, 2).await,
                StateTransitionExecutionResult::SuccessfulExecution { .. },
                "at protocol version {}",
                fixture.platform_version.protocol_version
            );
        }
    }

    /// The trigger's first check, from protocol version 15 `distinctFrom: "$ownerId"`.
    #[tokio::test]
    async fn should_refuse_a_contact_request_to_oneself() {
        let fixture = Fixture::with_triggers();
        let (document, entropy) = fixture.contact_request(fixture.sender.id(), 48);
        let error = paid_error(fixture.create("contactRequest", document, entropy, 2).await);
        assert!(is_data_trigger_condition_error(&error), "{error:?}");

        let fixture = Fixture::latest();
        let (document, entropy) = fixture.contact_request(fixture.sender.id(), 48);
        assert_matches!(
            paid_error(fixture.create("contactRequest", document, entropy, 2).await),
            ConsensusError::BasicError(BasicError::DocumentPropertyNotDistinctError(e))
                if e.document_type_name() == "contactRequest"
                    && e.property() == "toUserId"
                    && e.distinct_from() == "$ownerId"
        );
    }

    /// The trigger's second check, from protocol version 15 the `refersTo`
    /// identity declaration on `toUserId`.
    #[tokio::test]
    async fn should_refuse_a_contact_request_to_an_identity_that_does_not_exist() {
        let missing = Identifier::from([0xAB; 32]);

        let fixture = Fixture::with_triggers();
        let (document, entropy) = fixture.contact_request(missing, 48);
        let error = paid_error(fixture.create("contactRequest", document, entropy, 2).await);
        assert!(is_data_trigger_condition_error(&error), "{error:?}");

        let fixture = Fixture::latest();
        let (document, entropy) = fixture.contact_request(missing, 48);
        assert_matches!(
            paid_error(fixture.create("contactRequest", document, entropy, 2).await),
            ConsensusError::StateError(StateError::ReferencedEntityNotFoundError(e))
                if *e.entity_id() == missing
                    && *e.entity_type() == DocumentPropertyReferenceTarget::Identity
                    && e.path() == "toUserId"
        );
    }

    /// New from protocol version 15: an account label of 50 bytes is within the
    /// schema's 48 to 80 but is not an IV plus whole AES-CBC blocks.
    #[tokio::test]
    async fn should_refuse_an_account_label_that_is_not_an_iv_plus_whole_blocks() {
        let fixture = Fixture::with_triggers();
        let (document, entropy) = fixture.contact_request(fixture.recipient.id(), 50);
        assert_matches!(
            fixture.create("contactRequest", document, entropy, 2).await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );

        let fixture = Fixture::latest();
        let (document, entropy) = fixture.contact_request(fixture.recipient.id(), 50);
        assert_matches!(
            paid_error(fixture.create("contactRequest", document, entropy, 2).await),
            ConsensusError::BasicError(BasicError::InvalidEncryptedPropertyShapeError(e))
                if e.property() == "encryptedAccountLabel" && e.actual_length() == 50
        );
    }

    /// The profile trigger checked both addresses on create and on replace; from
    /// protocol version 15 the rules do.
    #[tokio::test]
    async fn should_refuse_a_profile_payment_address_of_an_unknown_type_on_create_and_replace() {
        for (field, rule) in [
            ("corePaymentAddress", "corePaymentAddressType"),
            ("platformPaymentAddress", "platformPaymentAddressType"),
        ] {
            for fixture in [Fixture::with_triggers(), Fixture::latest()] {
                let protocol_version = fixture.platform_version.protocol_version;
                let expect_refusal = |error: ConsensusError| {
                    if protocol_version <= PROTOCOL_VERSION_WITH_TRIGGERS {
                        assert!(is_data_trigger_condition_error(&error), "{error:?}");
                    } else {
                        assert_matches!(
                            error,
                            ConsensusError::BasicError(
                                BasicError::DocumentPropertyConstraintViolatedError(e)
                            ) if e.document_type_name() == "profile" && e.constraint() == rule
                        );
                    }
                };

                // a refused write is paid for and uses its nonce, so each write
                // takes the next one
                let (mut document, entropy) = fixture.profile(2);
                document.set(field, payment_address(0x02));
                expect_refusal(paid_error(
                    fixture.create("profile", document, entropy, 2).await,
                ));

                let (mut document, entropy) = fixture.profile(3);
                assert_matches!(
                    fixture
                        .create("profile", document.clone(), entropy, 3)
                        .await,
                    StateTransitionExecutionResult::SuccessfulExecution { .. },
                    "a profile with valid addresses at protocol version {protocol_version}"
                );
                document.increment_revision().expect("expected a revision");
                document.set(field, payment_address(0xff));
                expect_refusal(paid_error(fixture.replace("profile", document, 4).await));
            }
        }
    }
}
