//! A Batch naming a private document type (protocol version 15). A private type's documents
//! are encrypted entries only the shielded transition appends, so the Batch is refused as soon
//! as the type is resolved, before any document is read or written.

use super::*;
use crate::execution::check_tx::CheckTxLevel::FirstTimeCheck;
use crate::platform_types::platform::PlatformRef;
use dpp::consensus::basic::BasicError;
use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
use dpp::document::{Document, DocumentV0};
use std::collections::BTreeMap;

/// A contract whose only type, `secret`, is private, with one plaintext property, `note`.
const PRIVATE_CONTRACT_PATH: &str =
    "tests/supporting_files/contract/reference-validation/reference-validation-contract-private-foreign.json";

#[tokio::test]
async fn should_refuse_a_batch_create_on_a_private_document_type() {
    let platform_version = PlatformVersion::latest();
    let mut platform = TestPlatformBuilder::new()
        .build_with_mock_rpc()
        .set_initial_state_structure();
    let (identity, signer, key) = setup_identity(&mut platform, 967, dash_to_credits!(0.5));

    let contract = json_document_to_contract(PRIVATE_CONTRACT_PATH, true, platform_version)
        .expect("expected the private contract to pass full validation");
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
    let secret_type = contract
        .document_type_for_name("secret")
        .expect("expected the secret type");
    assert!(secret_type.is_private());

    // Built by hand: the type names no system field, so the document carries only its note
    let entropy = [7u8; 32];
    let mut document = Document::V0(DocumentV0 {
        owner_id: identity.id(),
        properties: BTreeMap::from([("note".to_string(), Value::Text("hello".to_string()))]),
        ..Default::default()
    });
    document
        .set_id_for_creation(secret_type, &entropy, 1, platform_version)
        .expect("expected to set the document id");
    let create = BatchTransition::new_document_creation_transition_from_document(
        document,
        secret_type,
        entropy,
        &key,
        1,
        0,
        None,
        &signer,
        platform_version,
        None,
    )
    .await
    .expect("expected the create transition");
    let raw = create
        .serialize_to_bytes()
        .expect("expected to serialize the create transition");

    let platform_state = platform.state.load();
    let platform_ref = PlatformRef {
        drive: &platform.drive,
        state: &platform_state,
        config: &platform.config,
        core_rpc: &platform.core_rpc,
    };
    let check_tx_errors = platform
        .check_tx(&raw, FirstTimeCheck, &platform_ref, platform_version)
        .expect("expected check_tx to return a result")
        .errors;

    let fetch_balance = |platform: &TempPlatform<MockCoreRPCLike>| {
        platform
            .drive
            .fetch_identity_balance(identity.id().to_buffer(), None, platform_version)
            .expect("expected to fetch the balance")
            .expect("expected the identity to have a balance")
    };
    let balance_before = fetch_balance(&platform);

    let transaction = platform.drive.grove.start_transaction();
    let processing_result = platform
        .platform
        .process_raw_state_transitions(
            &[raw],
            &platform_state,
            &BlockInfo::default(),
            &transaction,
            platform_version,
            false,
            None,
        )
        .expect("expected to process the transition");
    platform
        .drive
        .grove
        .commit_transaction(transaction)
        .unwrap()
        .expect("expected to commit the transaction");

    assert_matches!(
        check_tx_errors.as_slice(),
        [ConsensusError::BasicError(BasicError::DocumentTypePrivateError(e))]
            if e.document_type_name() == "secret" && e.data_contract_id() == contract.id(),
        "check_tx: {check_tx_errors:?}"
    );

    // Refused unpaid, as a type the contract does not have is (`InvalidDocumentTypeError`): the
    // refusal carries no action to charge, and check_tx refuses the batch first
    assert_matches!(
        processing_result.execution_results().as_slice(),
        [StateTransitionExecutionResult::UnpaidConsensusError(
            ConsensusError::BasicError(BasicError::DocumentTypePrivateError(e))
        )] if e.document_type_name() == "secret" && e.data_contract_id() == contract.id(),
        "block: {:?}",
        processing_result.execution_results()
    );
    assert_eq!(fetch_balance(&platform), balance_before);
    assert_eq!(
        platform
            .drive
            .fetch_identity_contract_nonce(
                identity.id().to_buffer(),
                contract.id().to_buffer(),
                true,
                None,
                platform_version,
            )
            .expect("expected to fetch the contract nonce"),
        None,
        "the refusal records no contract nonce"
    );

    // Nothing is written to the type
    let query = DriveDocumentQuery::from_sql_expr(
        "select * from secret",
        &contract,
        Some(&platform.config.drive),
        platform_version,
    )
    .expect("expected the query");
    let stored = platform
        .drive
        .query_documents(
            query,
            None,
            false,
            None,
            Some(platform_version.protocol_version),
        )
        .expect("expected to query the type")
        .documents()
        .to_vec();
    assert!(stored.is_empty(), "nothing is written: {stored:?}");
}
