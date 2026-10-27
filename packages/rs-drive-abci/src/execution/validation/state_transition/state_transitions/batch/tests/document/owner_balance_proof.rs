//! The proof waitForStateTransitionResult returns for a document batch
//! carries the credit balance of the batch's owner next to the document: the
//! prover merges the owner's balance into the document proof and the verifier
//! reads both from one state, so a wallet learns what the write left it with
//! without a second query.

use super::*;
use crate::platform_types::platform_state::PlatformState;
use crate::rpc::core::MockCoreRPCLike;
use crate::test::helpers::setup::TempPlatform;
use dpp::data_contract::accessors::v0::DataContractV0Setters;
use dpp::document::Document;
use dpp::identifier::Identifier;
use dpp::identity::SecurityLevel;
use dpp::prelude::DataContract;
use dpp::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
use dpp::state_transition::batch_transition::document_create_transition::DocumentCreateTransitionV0;
use dpp::state_transition::batch_transition::{BatchTransitionV0, DocumentCreateTransition};
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::state_transition::StateTransition;
use dpp::util::hash::hash_double;
use dpp::util::strings::convert_to_homograph_safe_chars;
use drive::drive::Drive;
use drive::query::{SingleDocumentDriveQuery, SingleDocumentDriveQueryContestedStatus};
use std::collections::BTreeMap;
use std::sync::Arc;

fn process_and_commit(
    platform: &TempPlatform<MockCoreRPCLike>,
    platform_state: &PlatformState,
    transition: &StateTransition,
    platform_version: &PlatformVersion,
) {
    process_and_commit_at(
        platform,
        platform_state,
        transition,
        &BlockInfo::default(),
        platform_version,
    )
}

fn process_and_commit_at(
    platform: &TempPlatform<MockCoreRPCLike>,
    platform_state: &PlatformState,
    transition: &StateTransition,
    block_info: &BlockInfo,
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
            block_info,
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
/// balance, execution-proved. The balance is checked against the stored one.
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
    assert!(
        outcome.is_execution_proved(),
        "expected an execution-proved result, got {outcome:?}"
    );
    let owner_balance = outcome.owner_balance();
    let StateTransitionProofResult::VerifiedDocuments(documents) = outcome.into_result() else {
        panic!("expected a documents result");
    };
    let owner_balance = owner_balance.expect("a batch proof carries the owner's balance");
    let owner_id = transition
        .owner_id()
        .expect("a document write has an owner");
    assert_eq!(
        owner_balance,
        stored_balance(platform, owner_id, platform_version),
        "the proved balance is the owner's stored balance"
    );
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
    let _ = owner_balance;
}

/// A batch proof names the balance of the batch's owner: a proof made for
/// another identity's batch carries a different balance entry and does not
/// verify against this batch, even though the document it shows is the same
/// (the document part of that proof verifies on its own). Depending on what
/// the other owner's balance proof happens to cover, the strict verification
/// of the merged query either finds no data for the owner's key or proves it
/// absent, which the verifier reports as a missing balance.
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

    // The document part of that proof is sound on its own.
    let (_, proved_document) = SingleDocumentDriveQuery {
        contract_id: dashpay.id().to_buffer(),
        document_type_name: "profile".to_string(),
        document_type_keeps_history: profile.documents_keep_history(),
        document_id: document.id().to_buffer(),
        block_time_ms: None,
        contested_status: SingleDocumentDriveQueryContestedStatus::NotContested,
    }
    .verify_proof(true, proof.as_slice(), profile, platform_version)
    .expect("the document part of the proof verifies on its own");
    assert_eq!(
        proved_document.map(|document| document.id()),
        Some(document.id())
    );

    let lookup = |_id: &Identifier| Ok(Some(Arc::clone(&dashpay)));
    let error = Drive::verify_state_transition_was_executed_with_proof(
        &create,
        &BlockInfo::default(),
        proof.as_slice(),
        &lookup,
        platform_version,
    )
    .expect_err("a proof of another owner's balance is not a proof of this batch");
    assert!(
        matches!(
            error,
            drive::error::Error::GroveDB(_)
                | drive::error::Error::Proof(drive::error::proof::ProofError::IncorrectProof(_))
        ),
        "expected the owner's balance to be reported as not proven, got: {error:?}"
    );
}

/// A keeps-history document type stores each revision under the document's
/// key, so its single-document query carries a sub-query: the merged proof
/// must still prove the latest revision next to the owner's balance, on the
/// create and on the replace.
#[tokio::test]
async fn should_prove_the_owner_balance_next_to_a_keeps_history_document() {
    let platform_version = PlatformVersion::latest();
    let mut platform = TestPlatformBuilder::new()
        .with_latest_protocol_version()
        .build_with_mock_rpc()
        .set_genesis_state();
    let platform_state = platform.state.load();
    let mut rng = StdRng::seed_from_u64(4801);

    let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

    // The fixture predates the v14 schema rules, so it is loaded without
    // validation; Drive stores and serves it all the same.
    let mut contract = json_document_to_contract(
        "tests/supporting_files/contract/note/note-contract-keep-history-and-can-be-deleted.json",
        false,
        platform_version,
    )
    .expect("expected the keeps-history note contract");
    contract.set_owner_id(identity.id());
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
    let contract = Arc::new(contract);
    let person = contract
        .document_type_for_name("note")
        .expect("expected a note document type");
    assert!(person.documents_keep_history());

    let entropy = Bytes32::random_with_rng(&mut rng);
    let mut document = person
        .random_document_with_identifier_and_entropy(
            &mut rng,
            identity.id(),
            entropy,
            DocumentFieldFillType::FillIfNotRequired,
            DocumentFieldFillSize::AnyDocumentFillSize,
            platform_version,
        )
        .expect("expected a random note");
    document
        .set_id_for_creation(person, &entropy.0, 2, platform_version)
        .expect("expected to set the document id");

    let create = BatchTransition::new_document_creation_transition_from_document(
        document.clone(),
        person,
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
    let (documents, _) = prove_and_verify(&platform, &create, &contract, platform_version);
    let (_, proved) = documents.into_iter().next().expect("one document");
    assert_eq!(
        proved.expect("the created note is present").revision(),
        document.revision()
    );

    document
        .increment_revision()
        .expect("expected to bump the revision");
    document.set("message", "the second revision".into());
    let replace = BatchTransition::new_document_replacement_transition_from_document(
        document.clone(),
        person,
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
    let (documents, _) = prove_and_verify(&platform, &replace, &contract, platform_version);
    let (_, proved) = documents.into_iter().next().expect("one document");
    let proved = proved.expect("the replaced note is present");
    assert_eq!(proved.revision(), document.revision());
    assert_eq!(
        proved.properties().get("message"),
        document.properties().get("message"),
        "the latest revision is the proven one"
    );
}

/// A contested create (a prefunded voting balance) stores the document under
/// the vote poll's tree, another path the merged proof must carry next to the
/// owner's balance.
#[tokio::test]
async fn should_prove_the_owner_balance_next_to_a_contested_document() {
    let platform_version = PlatformVersion::latest();
    let mut platform = TestPlatformBuilder::new()
        .with_latest_protocol_version()
        .build_with_mock_rpc()
        .set_genesis_state();
    let mut rng = StdRng::seed_from_u64(4802);

    let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.5));
    let dpns = platform
        .drive
        .cache
        .system_data_contracts
        .load_dpns(platform_version)
        .expect("expected the dpns system contract");
    let domain = dpns
        .document_type_for_name("domain")
        .expect("expected a domain document type");
    let preorder = dpns
        .document_type_for_name("preorder")
        .expect("expected a preorder document type");
    let platform_state = platform.state.load();

    // DPNS registers a name in two steps: the salted preorder, then the domain.
    let name = "quantum";
    let salt: [u8; 32] = rng.gen();
    let mut salted_domain = salt.to_vec();
    salted_domain.extend((convert_to_homograph_safe_chars(name) + ".dash").as_bytes());
    let preorder_entropy = Bytes32::random_with_rng(&mut rng);
    let mut preorder_document = preorder
        .random_document_with_identifier_and_entropy(
            &mut rng,
            identity.id(),
            preorder_entropy,
            DocumentFieldFillType::FillIfNotRequired,
            DocumentFieldFillSize::AnyDocumentFillSize,
            platform_version,
        )
        .expect("expected a random preorder document");
    preorder_document.set("saltedDomainHash", hash_double(salted_domain).into());
    preorder_document
        .set_id_for_creation(preorder, &preorder_entropy.0, 2, platform_version)
        .expect("expected to set the preorder id");
    let preorder_create = BatchTransition::new_document_creation_transition_from_document(
        preorder_document,
        preorder,
        preorder_entropy.0,
        &key,
        2,
        0,
        None,
        &signer,
        platform_version,
        None,
    )
    .await
    .expect("expected the preorder create transition");
    process_and_commit(
        &platform,
        &platform_state,
        &preorder_create,
        platform_version,
    );

    let entropy = Bytes32::random_with_rng(&mut rng);
    let mut document = domain
        .random_document_with_identifier_and_entropy(
            &mut rng,
            identity.id(),
            entropy,
            DocumentFieldFillType::FillIfNotRequired,
            DocumentFieldFillSize::AnyDocumentFillSize,
            platform_version,
        )
        .expect("expected a random domain document");
    document.set("parentDomainName", "dash".into());
    document.set("normalizedParentDomainName", "dash".into());
    document.set("label", name.into());
    document.set(
        "normalizedLabel",
        convert_to_homograph_safe_chars(name).into(),
    );
    document.set("records.identity", document.owner_id().into());
    document.set("subdomainRules.allowSubdomains", false.into());
    document.set("preorderSalt", salt.into());
    document
        .set_id_for_creation(domain, &entropy.0, 3, platform_version)
        .expect("expected to set the document id");

    let create_transition: DocumentCreateTransition = DocumentCreateTransitionV0 {
        base: DocumentBaseTransition::from_document(
            &document,
            domain,
            None,
            3,
            platform_version,
            None,
        )
        .expect("expected a base transition"),
        entropy: entropy.0,
        data: document.clone().properties_consumed(),
        prefunded_voting_balance: Some((
            "parentNameAndLabel".to_string(),
            platform_version
                .fee_version
                .vote_resolution_fund_fees
                .contested_document_vote_resolution_fund_required_amount,
        )),
    }
    .into();
    let batch: BatchTransition = BatchTransitionV0 {
        owner_id: identity.id(),
        transitions: vec![create_transition.into()],
        user_fee_increase: 0,
        signature_public_key_id: 0,
        signature: Default::default(),
    }
    .into();
    let mut create: StateTransition = batch.into();
    create
        .sign_external(&key, &signer, Some(|_, _| Ok(SecurityLevel::HIGH)))
        .await
        .expect("expected to sign the contested create");

    process_and_commit_at(
        &platform,
        &platform_state,
        &create,
        &BlockInfo::default_with_time(3000),
        platform_version,
    );

    let (documents, _) = prove_and_verify(&platform, &create, &dpns, platform_version);
    let (proved_id, proved) = documents.into_iter().next().expect("one document");
    assert_eq!(proved_id, document.id());
    assert_eq!(
        proved.expect("the contested domain is present").id(),
        document.id()
    );
}
