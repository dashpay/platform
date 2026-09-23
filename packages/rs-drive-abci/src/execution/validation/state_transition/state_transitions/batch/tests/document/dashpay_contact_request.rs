//! The DashPay contact request as the system contract declares it from
//! protocol version 14 (DashPay v2). Up to protocol version 13 a data trigger
//! refused a request to oneself or to an identity that does not exist. From 14
//! `toUserId` declares both checks, `distinctFrom: "$ownerId"` and an
//! `identityPublicKey` `refersTo` naming `recipientKeyIndex`, which also
//! requires the recipient's key to exist. The two ECDH fields declare
//! `encryptedFor`, so their bytes must be an IV followed by whole AES blocks.

use super::*;

mod dashpay_contact_request_tests {
    use super::*;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TempPlatform;
    use dpp::consensus::basic::BasicError;
    use dpp::document::Document;
    use dpp::identity::{Identity, IdentityPublicKey};
    use dpp::prelude::DataContract;
    use simple_signer::signer::SimpleSigner;
    use std::sync::Arc;

    struct Fixture {
        platform: TempPlatform<MockCoreRPCLike>,
        dashpay: Arc<DataContract>,
        sender: Identity,
        signer: SimpleSigner,
        key: IdentityPublicKey,
        recipient: Identity,
    }

    impl Fixture {
        fn new() -> Self {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let (sender, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));
            // Keys 0 and 1
            let (recipient, ..) = setup_identity(&mut platform, 495, dash_to_credits!(0.1));

            let dashpay = platform
                .drive
                .cache
                .system_data_contracts
                .load_dashpay(platform_version)
                .expect("expected the dashpay system contract");

            Fixture {
                platform,
                dashpay,
                sender,
                signer,
                key,
                recipient,
            }
        }

        /// A contact request from the sender to `to_user_id`'s key
        /// `recipient_key_index` whose account label is `label_length` bytes.
        fn contact_request(
            &self,
            to_user_id: Identifier,
            recipient_key_index: u32,
            label_length: usize,
        ) -> (Document, Bytes32) {
            let platform_version = PlatformVersion::latest();
            let contact_request = self
                .dashpay
                .document_type_for_name("contactRequest")
                .expect("expected the contactRequest document type");

            let mut rng = StdRng::seed_from_u64(437);
            let entropy = Bytes32::random_with_rng(&mut rng);
            let mut document = contact_request
                .random_document_with_identifier_and_entropy(
                    &mut rng,
                    self.sender.id(),
                    entropy,
                    DocumentFieldFillType::FillIfNotRequired,
                    DocumentFieldFillSize::AnyDocumentFillSize,
                    platform_version,
                )
                .expect("expected a random document");
            document
                .set_id_for_creation(contact_request, &entropy.0, 2, platform_version)
                .expect("expected to set the document id");

            document.set("toUserId", Value::Identifier(to_user_id.to_buffer()));
            document.set("senderKeyIndex", Value::U32(1));
            document.set("recipientKeyIndex", Value::U32(recipient_key_index));
            document.set("accountReference", Value::U32(0));
            document.set("encryptedPublicKey", Value::Bytes(vec![7u8; 96]));
            document.set(
                "encryptedAccountLabel",
                Value::Bytes(vec![8u8; label_length]),
            );
            (document, entropy)
        }

        async fn create(
            &self,
            document: Document,
            entropy: Bytes32,
        ) -> StateTransitionExecutionResult {
            let platform_version = PlatformVersion::latest();
            let transition = BatchTransition::new_document_creation_transition_from_document(
                document,
                self.dashpay
                    .document_type_for_name("contactRequest")
                    .expect("expected the contactRequest document type"),
                entropy.0,
                &self.key,
                2,
                0,
                None,
                &self.signer,
                platform_version,
                None,
            )
            .await
            .expect("expect to create documents batch transition");

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
                    platform_version,
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

    #[tokio::test]
    async fn should_create_a_contact_request_to_an_existing_key_of_another_identity() {
        let fixture = Fixture::new();
        let (document, entropy) = fixture.contact_request(fixture.recipient.id(), 1, 48);

        assert_matches!(
            fixture.create(document, entropy).await,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
    }

    /// The data trigger's first check, now `distinctFrom: "$ownerId"`.
    #[tokio::test]
    async fn should_refuse_a_contact_request_to_oneself() {
        let fixture = Fixture::new();
        let (document, entropy) = fixture.contact_request(fixture.sender.id(), 1, 48);

        assert_matches!(
            paid_error(fixture.create(document, entropy).await),
            ConsensusError::BasicError(BasicError::DocumentPropertyNotDistinctError(e))
                if e.document_type_name() == "contactRequest"
                    && e.property() == "toUserId"
                    && e.distinct_from() == "$ownerId"
        );
    }

    /// The data trigger's second check, now the `identityPublicKey` reference:
    /// an identity that does not exist has no key either.
    #[tokio::test]
    async fn should_refuse_a_contact_request_to_an_identity_that_does_not_exist() {
        let fixture = Fixture::new();
        let missing = Identifier::from([0xAB; 32]);
        let (document, entropy) = fixture.contact_request(missing, 1, 48);

        assert_matches!(
            paid_error(fixture.create(document, entropy).await),
            ConsensusError::StateError(StateError::ReferencedIdentityKeyNotFoundError(e))
                if *e.identity_id() == missing && e.key_id() == 1 && e.path() == "toUserId"
        );
    }

    /// New from protocol version 14: the trigger only asked whether the
    /// recipient identity exists, never whether it has the named key.
    #[tokio::test]
    async fn should_refuse_a_contact_request_to_a_key_the_recipient_does_not_have() {
        let fixture = Fixture::new();
        let (document, entropy) = fixture.contact_request(fixture.recipient.id(), 7, 48);

        assert_matches!(
            paid_error(fixture.create(document, entropy).await),
            ConsensusError::StateError(StateError::ReferencedIdentityKeyNotFoundError(e))
                if *e.identity_id() == fixture.recipient.id()
                    && e.key_id() == 7
                    && e.path() == "toUserId"
        );
    }

    /// New from protocol version 14: an account label of 50 bytes is within the
    /// schema's 48 to 80 but is not an IV plus whole AES-CBC blocks.
    #[tokio::test]
    async fn should_refuse_an_account_label_that_is_not_an_iv_plus_whole_blocks() {
        let fixture = Fixture::new();
        let (document, entropy) = fixture.contact_request(fixture.recipient.id(), 1, 50);

        assert_matches!(
            paid_error(fixture.create(document, entropy).await),
            ConsensusError::BasicError(BasicError::InvalidEncryptedPropertyShapeError(e))
                if e.property() == "encryptedAccountLabel" && e.actual_length() == 50
        );
    }
}
