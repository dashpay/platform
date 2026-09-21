//! The proof waitForStateTransitionResult returns for a document batch
//! carries the credit balance of the batch's owner next to the document: the
//! prover merges the owner's balance into the document proof and the verifier
//! reads both from one state, so a wallet learns what the write left it with
//! without a second query.

use super::*;
use crate::platform_types::platform_state::PlatformState;
use crate::rpc::core::MockCoreRPCLike;
use crate::test::helpers::setup::TempPlatform;
use dpp::document::Document;
use dpp::identifier::Identifier;
use dpp::prelude::DataContract;
use dpp::state_transition::proof_result::{
    StateTransitionProofOutcome, StateTransitionProofResult,
};
use dpp::state_transition::StateTransition;
use drive::drive::Drive;
use std::collections::BTreeMap;
use std::sync::Arc;

fn process_and_commit(
    platform: &TempPlatform<MockCoreRPCLike>,
    platform_state: &PlatformState,
    transition: &StateTransition,
    platform_version: &PlatformVersion,
) {
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
        .expect("expected to process the batch");
    platform
        .drive
        .grove
        .commit_transaction(transaction)
        .unwrap()
        .expect("expected to commit the batch");
    assert_matches!(
        processing_result.execution_results().as_slice(),
        [StateTransitionExecutionResult::SuccessfulExecution { .. }],
        "the batch must execute: {:?}",
        processing_result.execution_results()
    );
}

fn stored_balance(
    platform: &TempPlatform<MockCoreRPCLike>,
    identity_id: Identifier,
    platform_version: &PlatformVersion,
) -> Credits {
    platform
        .drive
        .fetch_identity_balance(identity_id.to_buffer(), None, platform_version)
        .expect("expected to fetch the owner's balance")
        .expect("the owner has a balance")
}

/// Proves the executed batch the way the node answers a wait, and verifies
/// the proof the way the SDK does: the executed document and the owner's
/// balance, execution-proved.
fn prove_and_verify(
    platform: &TempPlatform<MockCoreRPCLike>,
    transition: &StateTransition,
    contract: &Arc<DataContract>,
    platform_version: &PlatformVersion,
) -> (BTreeMap<Identifier, Option<Document>>, Credits) {
    let proof = platform
        .drive
        .prove_state_transition(transition, None, platform_version)
        .expect("expected to prove the executed batch")
        .into_data()
        .expect("expected proof bytes");
    let lookup = |_id: &Identifier| Ok(Some(Arc::clone(contract)));
    let (root_hash, outcome) = Drive::verify_state_transition_was_executed_with_proof(
        transition,
        &BlockInfo::default(),
        proof.as_slice(),
        &lookup,
        platform_version,
    )
    .expect("expected the batch proof to verify");
    assert_ne!(root_hash, [0u8; 32]);
    let StateTransitionProofOutcome::ExecutionProved(
        StateTransitionProofResult::VerifiedDocuments(documents, owner_balance),
    ) = outcome
    else {
        panic!("expected an execution-proved documents result, got {outcome:?}");
    };
    (documents, owner_balance)
}

#[tokio::test]
async fn should_prove_the_owner_balance_next_to_a_created_replaced_and_deleted_document() {
    let platform_version = PlatformVersion::latest();
    let mut platform = TestPlatformBuilder::new()
        .with_latest_protocol_version()
        .build_with_mock_rpc()
        .set_genesis_state();
    let platform_state = platform.state.load();
    let mut rng = StdRng::seed_from_u64(4799);

    let initial_credits = dash_to_credits!(0.1);
    let (identity, signer, key) = setup_identity(&mut platform, 958, initial_credits);

    let dashpay = platform
        .drive
        .cache
        .system_data_contracts
        .load_dashpay(platform_version)
        .expect("expected the dashpay system contract");
    let profile = dashpay
        .document_type_for_name("profile")
        .expect("expected a profile document type");

    // ── create ────────────────────────────────────────────────────────
    let entropy = Bytes32::random_with_rng(&mut rng);
    let mut document = profile
        .random_document_with_identifier_and_entropy(
            &mut rng,
            identity.id(),
            entropy,
            DocumentFieldFillType::FillIfNotRequired,
            DocumentFieldFillSize::AnyDocumentFillSize,
            platform_version,
        )
        .expect("expected a random document");
    document
        .set_id_for_creation(profile, &entropy.0, 2, platform_version)
        .expect("expected to set the document id");
    set_valid_profile_payment_addresses(&mut document, profile);
    document.set("avatarUrl", "http://test.com/bob.jpg".into());

    let create = BatchTransition::new_document_creation_transition_from_document(
        document.clone(),
        profile,
        entropy.0,
        &key,
        2,
        0,
        None,
        &signer,
        platform_version,
        None,
    )
    .await
    .expect("expected the create transition");
    process_and_commit(&platform, &platform_state, &create, platform_version);

    let (documents, owner_balance) =
        prove_and_verify(&platform, &create, &dashpay, platform_version);
    let (proved_id, proved_document) = documents.into_iter().next().expect("one document");
    assert_eq!(proved_id, document.id());
    assert_eq!(
        proved_document
            .expect("the created profile is present")
            .id(),
        document.id()
    );
    assert_eq!(
        owner_balance,
        stored_balance(&platform, identity.id(), platform_version),
        "the proved balance is the owner's stored balance"
    );
    assert!(
        owner_balance < initial_credits,
        "the create's fee left the owner with less than it started with"
    );
    let balance_after_create = owner_balance;

    // ── replace ───────────────────────────────────────────────────────
    document
        .increment_revision()
        .expect("expected to bump the revision");
    document.set("avatarUrl", "http://test.com/alice.jpg".into());
    let replace = BatchTransition::new_document_replacement_transition_from_document(
        document.clone(),
        profile,
        &key,
        3,
        0,
        None,
        &signer,
        platform_version,
        None,
    )
    .await
    .expect("expected the replace transition");
    process_and_commit(&platform, &platform_state, &replace, platform_version);

    let (documents, owner_balance) =
        prove_and_verify(&platform, &replace, &dashpay, platform_version);
    let (_, proved_document) = documents.into_iter().next().expect("one document");
    assert_eq!(
        proved_document
            .expect("the replaced profile is present")
            .revision(),
        document.revision()
    );
    assert_eq!(
        owner_balance,
        stored_balance(&platform, identity.id(), platform_version)
    );
    assert!(
        owner_balance < balance_after_create,
        "the replace's fee left the owner with less than after the create"
    );

    // ── delete ────────────────────────────────────────────────────────
    let delete = BatchTransition::new_document_deletion_transition_from_document(
        document.clone(),
        profile,
        &key,
        4,
        0,
        None,
        &signer,
        platform_version,
        None,
    )
    .await
    .expect("expected the delete transition");
    process_and_commit(&platform, &platform_state, &delete, platform_version);

    let (documents, owner_balance) =
        prove_and_verify(&platform, &delete, &dashpay, platform_version);
    let (proved_id, proved_document) = documents.into_iter().next().expect("one entry");
    assert_eq!(proved_id, document.id());
    assert!(
        proved_document.is_none(),
        "the deleted profile is proven absent"
    );
    assert_eq!(
        owner_balance,
        stored_balance(&platform, identity.id(), platform_version)
    );
}

/// A batch proof names the balance of the batch's owner: a proof made for
/// another identity's batch carries a different balance entry and does not
/// verify against this batch, even though the document it shows is the same.
/// GroveDB may find no data for the owner's balance key in it, or the
/// verifier reports the balance as missing; either way it is rejected.
#[tokio::test]
async fn should_reject_a_document_proof_that_carries_another_identitys_balance() {
    let platform_version = PlatformVersion::latest();
    let mut platform = TestPlatformBuilder::new()
        .with_latest_protocol_version()
        .build_with_mock_rpc()
        .set_genesis_state();
    let platform_state = platform.state.load();
    let mut rng = StdRng::seed_from_u64(4800);

    let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));
    let (other, _other_signer, _other_key) =
        setup_identity(&mut platform, 959, dash_to_credits!(0.1));

    let dashpay = platform
        .drive
        .cache
        .system_data_contracts
        .load_dashpay(platform_version)
        .expect("expected the dashpay system contract");
    let profile = dashpay
        .document_type_for_name("profile")
        .expect("expected a profile document type");

    let entropy = Bytes32::random_with_rng(&mut rng);
    let mut document = profile
        .random_document_with_identifier_and_entropy(
            &mut rng,
            identity.id(),
            entropy,
            DocumentFieldFillType::FillIfNotRequired,
            DocumentFieldFillSize::AnyDocumentFillSize,
            platform_version,
        )
        .expect("expected a random document");
    document
        .set_id_for_creation(profile, &entropy.0, 2, platform_version)
        .expect("expected to set the document id");
    set_valid_profile_payment_addresses(&mut document, profile);
    document.set("avatarUrl", "http://test.com/bob.jpg".into());

    let create = BatchTransition::new_document_creation_transition_from_document(
        document.clone(),
        profile,
        entropy.0,
        &key,
        2,
        0,
        None,
        &signer,
        platform_version,
        None,
    )
    .await
    .expect("expected the create transition");
    process_and_commit(&platform, &platform_state, &create, platform_version);

    // The same document, claimed by a batch of another owner: the node proves
    // that batch's owner balance, which the verifier of the real batch does
    // not find.
    let StateTransition::Batch(mut misattributed) = create.clone() else {
        panic!("a document create is a batch");
    };
    match &mut misattributed {
        BatchTransition::V0(batch) => batch.owner_id = other.id(),
        BatchTransition::V1(batch) => batch.owner_id = other.id(),
    }
    let misattributed = StateTransition::Batch(misattributed);
    let proof = platform
        .drive
        .prove_state_transition(&misattributed, None, platform_version)
        .expect("expected to prove the misattributed batch")
        .into_data()
        .expect("expected proof bytes");

    let lookup = |_id: &Identifier| Ok(Some(Arc::clone(&dashpay)));
    let error = Drive::verify_state_transition_was_executed_with_proof(
        &create,
        &BlockInfo::default(),
        proof.as_slice(),
        &lookup,
        platform_version,
    )
    .expect_err("a proof of another owner's balance is not a proof of this batch");
    let message = error.to_string();
    assert!(
        message.contains("balance of the document batch owner") || message.contains("missing data"),
        "expected the missing owner balance to be reported, got: {message}"
    );
}
