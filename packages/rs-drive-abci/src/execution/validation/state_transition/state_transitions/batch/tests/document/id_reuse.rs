//! A document id can be produced at most once (protocol version 14).
//!
//! Up to protocol version 13 the id of a new document hashed the contract,
//! owner, document type and entropy only, and the create check only asks
//! whether a document exists under the id right now. The owner of a deleted
//! document could therefore create another one under the same id by reusing
//! the entropy, with different content, and whatever referenced the id then
//! pointed at the new content: content substitution on a document type with
//! `documentsMutable: false`. From protocol version 14 the id also commits to
//! the identity contract nonce of the create transition, which is consumed at
//! most once.

use super::*;

mod id_reuse_tests {
    use super::*;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TempPlatform;
    use dpp::consensus::basic::BasicError;
    use dpp::document::Document;
    use dpp::identity::{Identity, IdentityPublicKey};
    use dpp::prelude::{DataContract, Identifier, IdentityNonce};
    use dpp::state_transition::StateTransition;
    use drive::query::DriveDocumentQuery;
    use simple_signer::signer::SimpleSigner;

    const CONTRACT_PATH: &str = "tests/supporting_files/contract/dashpay/dashpay-contract-contact-request-not-mutable-and-can-be-deleted.json";

    struct Setup {
        platform: TempPlatform<MockCoreRPCLike>,
        contract: DataContract,
        identity: Identity,
        other_identity: Identity,
        signer: SimpleSigner,
        key: IdentityPublicKey,
    }

    fn setup(platform_version: &'static PlatformVersion) -> Setup {
        let mut platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(platform_version.protocol_version)
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let contract = json_document_to_contract(CONTRACT_PATH, true, platform_version)
            .expect("expected to get data contract");
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
            .expect("expected to apply contract successfully");

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));
        let (other_identity, ..) = setup_identity(&mut platform, 495, dash_to_credits!(0.1));

        Setup {
            platform,
            contract,
            identity,
            other_identity,
            signer,
            key,
        }
    }

    /// A contact request whose content is told apart by `senderKeyIndex`.
    fn contact_request(
        setup: &Setup,
        entropy: Bytes32,
        sender_key_index: u32,
        platform_version: &PlatformVersion,
    ) -> Document {
        let document_type = setup
            .contract
            .document_type_for_name("contactRequest")
            .expect("expected the contactRequest document type");

        assert!(!document_type.documents_mutable());
        assert!(document_type.documents_can_be_deleted());

        let mut rng = StdRng::seed_from_u64(437);
        let mut document = document_type
            .random_document_with_identifier_and_entropy(
                &mut rng,
                setup.identity.id(),
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        document.set(
            "toUserId",
            Value::Identifier(setup.other_identity.id().to_buffer()),
        );
        document.set("recipientKeyIndex", Value::U32(1));
        document.set("senderKeyIndex", Value::U32(sender_key_index));
        document.set("accountReference", Value::U32(0));
        document
    }

    fn process(
        setup: &Setup,
        transition: StateTransition,
        platform_version: &PlatformVersion,
    ) -> Vec<StateTransitionExecutionResult> {
        let platform_state = setup.platform.state.load();
        let transaction = setup.platform.drive.grove.start_transaction();

        let processing_result = setup
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

        setup
            .platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        processing_result.into_execution_results()
    }

    async fn create(
        setup: &Setup,
        document: Document,
        entropy: Bytes32,
        nonce: IdentityNonce,
        build_version: &PlatformVersion,
    ) -> StateTransition {
        BatchTransition::new_document_creation_transition_from_document(
            document,
            setup
                .contract
                .document_type_for_name("contactRequest")
                .expect("expected the contactRequest document type"),
            entropy.0,
            &setup.key,
            nonce,
            0,
            None,
            &setup.signer,
            build_version,
            None,
        )
        .await
        .expect("expect to create documents batch transition")
    }

    async fn delete(
        setup: &Setup,
        document: Document,
        nonce: IdentityNonce,
        platform_version: &PlatformVersion,
    ) -> StateTransition {
        BatchTransition::new_document_deletion_transition_from_document(
            document,
            setup
                .contract
                .document_type_for_name("contactRequest")
                .expect("expected the contactRequest document type"),
            &setup.key,
            nonce,
            0,
            None,
            &setup.signer,
            platform_version,
            None,
        )
        .await
        .expect("expect to create documents batch transition")
    }

    /// The stored contact request with the given id, read back from Drive.
    fn stored(
        setup: &Setup,
        id: Identifier,
        platform_version: &PlatformVersion,
    ) -> Option<Document> {
        let query = DriveDocumentQuery::from_sql_expr(
            "select * from contactRequest",
            &setup.contract,
            Some(&setup.platform.config.drive),
            platform_version,
        )
        .expect("expected a document query");
        setup
            .platform
            .drive
            .query_documents(query, None, false, None, None)
            .expect("expected a query result")
            .documents()
            .iter()
            .find(|document| document.id() == id)
            .cloned()
    }

    #[tokio::test]
    async fn should_not_let_a_deleted_document_be_created_again_under_its_id() {
        let platform_version = PlatformVersion::latest();
        let setup = setup(platform_version);
        let entropy = Bytes32::new([9u8; 32]);

        let document = contact_request(&setup, entropy, 1, platform_version);
        let original_id = Document::generate_document_id(
            &setup.contract.id(),
            &setup.identity.id(),
            "contactRequest",
            entropy.as_slice(),
            2,
            platform_version,
        )
        .expect("expected an id");
        // the id the document was built with, before a nonce was assigned, is
        // a placeholder
        assert_ne!(document.id(), original_id);

        let results = process(
            &setup,
            create(&setup, document.clone(), entropy, 2, platform_version).await,
            platform_version,
        );
        assert_matches!(
            results.as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        assert!(stored(&setup, original_id, platform_version).is_some());
        assert!(stored(&setup, document.id(), platform_version).is_none());

        let mut to_delete = document.clone();
        to_delete.set_id(original_id);
        let results = process(
            &setup,
            delete(&setup, to_delete, 3, platform_version).await,
            platform_version,
        );
        assert_matches!(
            results.as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        assert!(stored(&setup, original_id, platform_version).is_none());

        // The same entropy with different content: the transition is built the
        // way protocol version 13 clients build it, so it claims the id the
        // entropy alone derives. Consensus no longer accepts that derivation.
        let substitute = contact_request(&setup, entropy, 2, platform_version);
        let results = process(
            &setup,
            create(
                &setup,
                substitute.clone(),
                entropy,
                4,
                PlatformVersion::get(13).expect("expected version 13"),
            )
            .await,
            platform_version,
        );
        assert_matches!(
            results.as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::BasicError(BasicError::InvalidDocumentTransitionIdError(_)),
                ..
            }]
        );

        // Built for this protocol version it is accepted, under another id.
        let results = process(
            &setup,
            create(&setup, substitute, entropy, 5, platform_version).await,
            platform_version,
        );
        assert_matches!(
            results.as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        assert!(
            stored(&setup, original_id, platform_version).is_none(),
            "the id of the deleted document must stay empty"
        );
    }

    /// Pins what protocol version 13 allowed, so that the replay of blocks
    /// from before the upgrade stays reproducible.
    #[tokio::test]
    async fn should_let_a_deleted_document_be_created_again_under_its_id_protocol_version_13() {
        let platform_version = PlatformVersion::get(13).expect("expected version 13");
        let setup = setup(platform_version);
        let entropy = Bytes32::new([9u8; 32]);

        let document = contact_request(&setup, entropy, 1, platform_version);
        let original_id = document.id();

        let results = process(
            &setup,
            create(&setup, document.clone(), entropy, 2, platform_version).await,
            platform_version,
        );
        assert_matches!(
            results.as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );

        let results = process(
            &setup,
            delete(&setup, document, 3, platform_version).await,
            platform_version,
        );
        assert_matches!(
            results.as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );

        let substitute = contact_request(&setup, entropy, 2, platform_version);
        assert_eq!(substitute.id(), original_id);
        let results = process(
            &setup,
            create(&setup, substitute, entropy, 4, platform_version).await,
            platform_version,
        );
        assert_matches!(
            results.as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );

        let recreated = stored(&setup, original_id, platform_version)
            .expect("expected a document under the id of the deleted one");
        assert_eq!(
            recreated
                .get("senderKeyIndex")
                .and_then(|value| value.to_integer::<u32>().ok()),
            Some(2),
            "the id now resolves to different content"
        );
    }
}
