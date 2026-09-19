//! The owner gate on `contract` references through the full ABCI pipeline:
//! `refersTo: { type: "contract", propertyAgreement: { "$ownerId": "$ownerId" } }`
//! lets only the referenced contract's owner create or replace the referring
//! document. Exercised on the app-connect system contract's `appManifest`,
//! the first document type to carry it, and on a fixture contract.

use super::*;

mod contract_owner_gate_tests {
    use super::*;
    use crate::platform_types::platform_state::PlatformState;
    use crate::platform_types::state_transitions_processing_result::StateTransitionsProcessingResult;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TempPlatform;
    use dpp::data_contract::accessors::v0::DataContractV0Setters;
    use dpp::data_contract::config::DataContractConfig;
    use dpp::data_contracts::SystemDataContract;
    use dpp::document::Document;
    use dpp::identifier::Identifier;
    use dpp::identity::signer::Signer;
    use dpp::identity::IdentityPublicKey;
    use dpp::prelude::DataContract;
    use dpp::state_transition::StateTransition;
    use dpp::system_data_contracts::load_system_data_contract;

    /// `manifest.appContractId` carries the owner gate.
    const OWNER_GATE_CONTRACT_PATH: &str = "tests/supporting_files/contract/reference-validation/reference-validation-contract-owner-gate-valid.json";

    /// The contract whose owner the gate names: any registered contract
    /// will do, so the basic token fixture stands in for an "app contract".
    const APP_CONTRACT_PATH: &str = "tests/supporting_files/contract/basic-token/basic-token.json";

    /// Same declaration on a transferable document type: the one shape on
    /// which the gate's replace-time re-check can refuse anyone.
    const OWNER_GATE_TRANSFERABLE_CONTRACT_PATH: &str = "tests/supporting_files/contract/reference-validation/reference-validation-contract-owner-gate-transferable.json";

    fn register_contract(
        platform: &TempPlatform<MockCoreRPCLike>,
        path: &str,
        id: Identifier,
        owner_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> DataContract {
        let mut contract = json_document_to_contract(path, true, platform_version)
            .expect("expected to parse the contract");
        contract.set_id(id);
        contract.set_owner_id(owner_id);
        contract.set_config(
            DataContractConfig::default_for_version(platform_version)
                .expect("expected the default contract config"),
        );
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
        contract
    }

    fn process_and_commit(
        platform: &TempPlatform<MockCoreRPCLike>,
        platform_state: &PlatformState,
        transition: &StateTransition,
        platform_version: &PlatformVersion,
    ) -> StateTransitionsProcessingResult {
        let serialized = transition
            .serialize_to_bytes()
            .expect("expected the batch transition to serialize");
        let transaction = platform.drive.grove.start_transaction();
        let processing_result = platform
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
            .expect("expected to process state transition");
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");
        processing_result
    }

    /// Creates a document of `document_type_name` owned by `owner` whose
    /// `reference_property` names `referenced_contract_id`, with `name`
    /// set; returns the processing result and the document as submitted.
    #[allow(clippy::too_many_arguments)]
    async fn submit_gated_document<S: Signer<IdentityPublicKey>>(
        platform: &TempPlatform<MockCoreRPCLike>,
        platform_state: &PlatformState,
        contract: &DataContract,
        document_type_name: &str,
        reference_property: &str,
        referenced_contract_id: Identifier,
        owner: Identifier,
        key: &IdentityPublicKey,
        nonce: u64,
        signer: &S,
        rng: &mut StdRng,
        platform_version: &PlatformVersion,
    ) -> (StateTransitionsProcessingResult, Document) {
        let document_type = contract
            .document_type_for_name(document_type_name)
            .expect("document type exists");
        let entropy = Bytes32::random_with_rng(rng);
        let mut document = document_type
            .random_document_with_identifier_and_entropy(
                rng,
                owner,
                entropy,
                DocumentFieldFillType::DoNotFillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");
        document.set(
            reference_property,
            Value::Identifier(referenced_contract_id.to_buffer()),
        );
        document.set("name", "Yappr".into());
        // The random fill does not respect `maximum`; the manifest's bounds
        // kind must be 0..=3 or JSON-schema validation refuses the document
        // before the reference is ever checked.
        if document_type
            .flattened_properties()
            .contains_key("authBoundsKind")
        {
            document.set("authBoundsKind", Value::U8(0));
        }
        let create = BatchTransition::new_document_creation_transition_from_document(
            document.clone(),
            document_type,
            entropy.0,
            key,
            nonce,
            0,
            None,
            signer,
            platform_version,
            None,
        )
        .await
        .expect("expected the create transition");
        (
            process_and_commit(platform, platform_state, &create, platform_version),
            document,
        )
    }

    /// Replaces only `name` on `document`, signed by `key`'s identity.
    #[allow(clippy::too_many_arguments)]
    async fn replace_gated_document_name<S: Signer<IdentityPublicKey>>(
        platform: &TempPlatform<MockCoreRPCLike>,
        platform_state: &PlatformState,
        contract: &DataContract,
        document_type_name: &str,
        document: &mut Document,
        name: &str,
        key: &IdentityPublicKey,
        nonce: u64,
        signer: &S,
        platform_version: &PlatformVersion,
    ) -> StateTransitionsProcessingResult {
        let document_type = contract
            .document_type_for_name(document_type_name)
            .expect("document type exists");
        document.set("name", name.into());
        document
            .increment_revision()
            .expect("expected the revision to increment");
        let replace = BatchTransition::new_document_replacement_transition_from_document(
            document.clone(),
            document_type,
            key,
            nonce,
            0,
            None,
            signer,
            platform_version,
            None,
        )
        .await
        .expect("expected the replace transition");
        process_and_commit(platform, platform_state, &replace, platform_version)
    }

    /// Transfers `document` from its current owner (who signs) to `recipient`.
    #[allow(clippy::too_many_arguments)]
    async fn transfer_gated_document<S: Signer<IdentityPublicKey>>(
        platform: &TempPlatform<MockCoreRPCLike>,
        platform_state: &PlatformState,
        contract: &DataContract,
        document_type_name: &str,
        document: &mut Document,
        recipient: Identifier,
        key: &IdentityPublicKey,
        nonce: u64,
        signer: &S,
        platform_version: &PlatformVersion,
    ) {
        let document_type = contract
            .document_type_for_name(document_type_name)
            .expect("document type exists");
        document
            .increment_revision()
            .expect("expected the revision to increment");
        let transfer = BatchTransition::new_document_transfer_transition_from_document(
            document.clone(),
            document_type,
            recipient,
            key,
            nonce,
            0,
            None,
            signer,
            platform_version,
            None,
        )
        .await
        .expect("expected the transfer transition");
        let result = process_and_commit(platform, platform_state, &transfer, platform_version);
        assert_eq!(
            result.valid_count(),
            1,
            "the document must be transferred: {:?}",
            result.execution_results()
        );
        document.set_owner_id(recipient);
    }

    fn assert_owner_gate_refused(result: &StateTransitionsProcessingResult, because: &str) {
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(
                    StateError::ReferencedDocumentPropertyMismatchError(_)
                ),
                ..
            }],
            "{because}"
        );
    }

    /// The app-connect `appManifest`: its `appContractId` names the app's
    /// contract and carries the owner gate, so the contract's owner may
    /// publish the manifest and nobody else may.
    #[tokio::test]
    async fn test_app_manifest_is_only_writable_by_the_app_contract_owner() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();
        let platform_state = platform.state.load();
        let mut rng = StdRng::seed_from_u64(4831);

        let (app_owner, app_owner_signer, app_owner_key) =
            setup_identity(&mut platform, 958, dash_to_credits!(1.0));
        let (stranger, stranger_signer, stranger_key) =
            setup_identity(&mut platform, 450, dash_to_credits!(1.0));

        // The "app": a user contract owned by `app_owner`.
        let app_contract = register_contract(
            &platform,
            APP_CONTRACT_PATH,
            Identifier::from([7u8; 32]),
            app_owner.id(),
            platform_version,
        );

        // The system contract is registered at genesis; load the same
        // materialization to build transitions against.
        let app_connect =
            load_system_data_contract(SystemDataContract::AppConnect, platform_version)
                .expect("expected the app-connect contract");

        // A stranger publishing a manifest for a contract they do not own is refused.
        let (result, _) = submit_gated_document(
            &platform,
            &platform_state,
            &app_connect,
            "appManifest",
            "appContractId",
            app_contract.id(),
            stranger.id(),
            &stranger_key,
            2,
            &stranger_signer,
            &mut rng,
            platform_version,
        )
        .await;
        assert_owner_gate_refused(
            &result,
            "an identity that does not own the app contract must not publish its manifest",
        );

        // The app contract's owner may.
        let (result, mut manifest) = submit_gated_document(
            &platform,
            &platform_state,
            &app_connect,
            "appManifest",
            "appContractId",
            app_contract.id(),
            app_owner.id(),
            &app_owner_key,
            2,
            &app_owner_signer,
            &mut rng,
            platform_version,
        )
        .await;
        assert_eq!(
            result.valid_count(),
            1,
            "the app contract's owner must be able to publish the manifest: {:?}",
            result.execution_results()
        );

        // And may replace it later; the gate is re-checked on the replace.
        let result = replace_gated_document_name(
            &platform,
            &platform_state,
            &app_connect,
            "appManifest",
            &mut manifest,
            "Yappr, renamed",
            &app_owner_key,
            3,
            &app_owner_signer,
            platform_version,
        )
        .await;
        assert_eq!(
            result.valid_count(),
            1,
            "the owner must be able to replace the manifest: {:?}",
            result.execution_results()
        );
    }

    /// A manifest naming a contract that does not exist fails the existence
    /// check, not the gate: there is no owner to compare against.
    #[tokio::test]
    async fn test_app_manifest_for_a_missing_contract_is_not_found() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();
        let platform_state = platform.state.load();
        let mut rng = StdRng::seed_from_u64(4832);

        let (owner, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(1.0));
        let app_connect =
            load_system_data_contract(SystemDataContract::AppConnect, platform_version)
                .expect("expected the app-connect contract");

        let (result, _) = submit_gated_document(
            &platform,
            &platform_state,
            &app_connect,
            "appManifest",
            "appContractId",
            Identifier::from([9u8; 32]),
            owner.id(),
            &key,
            2,
            &signer,
            &mut rng,
            platform_version,
        )
        .await;
        assert_matches!(
            result.execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(StateError::ReferencedEntityNotFoundError(_)),
                ..
            }],
            "a manifest for a contract that does not exist must be refused as not found"
        );
    }

    /// The gate on a user contract's document type, through registration
    /// and then writes: the fixture's `manifest.appContractId` carries it.
    #[tokio::test]
    async fn test_contract_owner_gate_on_a_user_contract() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();
        let platform_state = platform.state.load();
        let mut rng = StdRng::seed_from_u64(4833);

        let (alice, alice_signer, alice_key) =
            setup_identity(&mut platform, 958, dash_to_credits!(1.0));
        let (bob, bob_signer, bob_key) = setup_identity(&mut platform, 450, dash_to_credits!(1.0));

        // Alice owns the referenced contract, Bob owns the declaring one:
        // the gate is about the referenced contract's owner, not the
        // declaring contract's.
        let referenced = register_contract(
            &platform,
            APP_CONTRACT_PATH,
            Identifier::from([7u8; 32]),
            alice.id(),
            platform_version,
        );
        let declaring = register_contract(
            &platform,
            OWNER_GATE_CONTRACT_PATH,
            Identifier::from([8u8; 32]),
            bob.id(),
            platform_version,
        );

        let (result, _) = submit_gated_document(
            &platform,
            &platform_state,
            &declaring,
            "manifest",
            "appContractId",
            referenced.id(),
            bob.id(),
            &bob_key,
            2,
            &bob_signer,
            &mut rng,
            platform_version,
        )
        .await;
        assert_owner_gate_refused(
            &result,
            "owning the declaring contract does not pass the gate on the referenced one",
        );

        let (result, mut document) = submit_gated_document(
            &platform,
            &platform_state,
            &declaring,
            "manifest",
            "appContractId",
            referenced.id(),
            alice.id(),
            &alice_key,
            2,
            &alice_signer,
            &mut rng,
            platform_version,
        )
        .await;
        assert_eq!(
            result.valid_count(),
            1,
            "the referenced contract's owner must pass the gate: {:?}",
            result.execution_results()
        );

        // A replace by the owner passes; the gate is checked on every replace.
        let result = replace_gated_document_name(
            &platform,
            &platform_state,
            &declaring,
            "manifest",
            &mut document,
            "renamed",
            &alice_key,
            3,
            &alice_signer,
            platform_version,
        )
        .await;
        assert_eq!(
            result.valid_count(),
            1,
            "the owner must be able to replace: {:?}",
            result.execution_results()
        );
    }

    /// The gate is re-checked on every replace, not only when the reference
    /// changes. A contract's owner never changes, so on a non-transferable
    /// type that re-check can never refuse anyone; on a transferable type it
    /// can: once the owner transfers a gated document to someone else, that
    /// holder is not the contract's owner and may not replace it, even an
    /// unrelated field. The permanent-document writer gate behaves the same.
    #[tokio::test]
    async fn test_contract_owner_gate_refuses_a_replace_by_a_transferee() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();
        let platform_state = platform.state.load();
        let mut rng = StdRng::seed_from_u64(4834);

        let (alice, alice_signer, alice_key) =
            setup_identity(&mut platform, 958, dash_to_credits!(1.0));
        let (bob, bob_signer, bob_key) = setup_identity(&mut platform, 450, dash_to_credits!(1.0));

        let referenced = register_contract(
            &platform,
            APP_CONTRACT_PATH,
            Identifier::from([7u8; 32]),
            alice.id(),
            platform_version,
        );
        let declaring = register_contract(
            &platform,
            OWNER_GATE_TRANSFERABLE_CONTRACT_PATH,
            Identifier::from([8u8; 32]),
            alice.id(),
            platform_version,
        );

        let (result, mut document) = submit_gated_document(
            &platform,
            &platform_state,
            &declaring,
            "manifest",
            "appContractId",
            referenced.id(),
            alice.id(),
            &alice_key,
            2,
            &alice_signer,
            &mut rng,
            platform_version,
        )
        .await;
        assert_eq!(
            result.valid_count(),
            1,
            "the contract owner must pass the gate on create: {:?}",
            result.execution_results()
        );

        transfer_gated_document(
            &platform,
            &platform_state,
            &declaring,
            "manifest",
            &mut document,
            bob.id(),
            &alice_key,
            3,
            &alice_signer,
            platform_version,
        )
        .await;

        // Bob holds the document but does not own the referenced contract:
        // a replace of an unrelated field is refused by the re-checked gate.
        let result = replace_gated_document_name(
            &platform,
            &platform_state,
            &declaring,
            "manifest",
            &mut document,
            "renamed by the transferee",
            &bob_key,
            2,
            &bob_signer,
            platform_version,
        )
        .await;
        assert_owner_gate_refused(
            &result,
            "a transferee that does not own the referenced contract must not replace the document",
        );
    }
}
