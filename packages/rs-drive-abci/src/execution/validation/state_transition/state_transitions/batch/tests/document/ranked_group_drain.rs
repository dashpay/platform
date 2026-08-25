//! What the network can actually do to a ranked index group, driven by real
//! signed batch transitions through `process_raw_state_transitions`.
//!
//! Drive's `batched_group_drain` suite shows that several document operations
//! sharing one GroveDB batch can jointly empty a ranked group and leave the
//! group tree behind — a document-less group that keeps ranking at zero and
//! still proves against the live root hash. Those cases are `#[ignore]`d
//! because the shape cannot be produced from the network, and this module is
//! why: a batch state transition may carry only one transition
//! (`max_transitions_in_documents_batch`), and two transitions in the same
//! block are applied as two separate GroveDB batches, because `execute_event`
//! calls `apply_drive_operations` once per state transition.
//!
//! Both halves of that argument are executed here rather than read off the
//! source: the cap is observed refusing a two-transition batch, and the
//! reachable near-miss — two identities draining the same group in one block —
//! is observed producing the correct index. Everything downstream of the
//! signed transition is real: advanced structure validation, the state
//! transformer, `into_high_level_drive_operations`, `apply_drive_operations`,
//! and grovedb's final batch application.

use super::*;

use crate::platform_types::platform_state::PlatformState;
use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult::UnpaidConsensusError;
use crate::rpc::core::MockCoreRPCLike;
use crate::test::helpers::setup::TempPlatform;
use dpp::consensus::basic::BasicError;
use dpp::data_contract::accessors::v0::DataContractV0Setters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::document::Document;
use dpp::identifier::Identifier;
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::platform_value::BinaryData;
use dpp::prelude::DataContract;
use dpp::state_transition::batch_transition::batched_transition::BatchedTransition;
use dpp::state_transition::batch_transition::document_delete_transition::DocumentDeleteTransition;
use dpp::state_transition::batch_transition::BatchTransitionV1;
use dpp::state_transition::StateTransition;
use drive::drive::RootTree;
use drive::grovedb::Element;

/// The group that gets emptied.
const G: &str = "beta";
/// The bystander group, so the ranked index is never globally empty.
const H: &str = "alpha";
/// The single index property every doctype in the fixture ranks by.
const GROUP_PROPERTY: &str = "restaurantId";

/// Shared with rs-drive's ranked suite rather than copied — this is the
/// fixture both layers of the argument are written against.
const RESTAURANTS_CONTRACT: &str =
    "../rs-drive/tests/supporting_files/contract/restaurants/restaurants-contract.json";

/// The path of the terminal property-name tree: the indexed tree whose
/// children are the groups and whose secondary carries the ranking.
fn indexed_property_name_tree_path(
    contract_id: Identifier,
    document_type_name: &str,
) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::DataContractDocuments as u8],
        contract_id.as_bytes().to_vec(),
        vec![1],
        document_type_name.as_bytes().to_vec(),
        GROUP_PROPERTY.as_bytes().to_vec(),
    ]
}

/// `SELECT COUNT(*) GROUP BY restaurantId ORDER BY $count DESC LIMIT 100`
/// straight off the secondary, with no query grammar in between.
fn ranked_count_groups(
    platform: &TempPlatform<MockCoreRPCLike>,
    path: &[Vec<u8>],
    platform_version: &PlatformVersion,
) -> Vec<(u64, String)> {
    let path_refs: Vec<&[u8]> = path.iter().map(|v| v.as_slice()).collect();
    platform
        .drive
        .grove
        .indexed_count_top_k(
            path_refs.as_slice(),
            100,
            true,
            None,
            &platform_version.drive.grove_version,
        )
        .unwrap()
        .expect("the ranked count read must succeed")
        .into_iter()
        .map(|entry| entry.key_pair())
        .map(|(count, key)| {
            (
                count,
                String::from_utf8(key).expect("fixture group keys are utf-8"),
            )
        })
        .collect()
}

/// The group's own value tree in the primary, or `None` if the group is gone.
fn primary_group_element(
    platform: &TempPlatform<MockCoreRPCLike>,
    path: &[Vec<u8>],
    group: &str,
    platform_version: &PlatformVersion,
) -> Option<Element> {
    let path_refs: Vec<&[u8]> = path.iter().map(|v| v.as_slice()).collect();
    platform
        .drive
        .grove
        .get_raw_optional(
            path_refs.as_slice().into(),
            group.as_bytes(),
            None,
            &platform_version.drive.grove_version,
        )
        .unwrap()
        .expect("the raw read must succeed")
}

/// Register the restaurants fixture owned by `owner_id`.
fn register_restaurants(
    platform: &TempPlatform<MockCoreRPCLike>,
    owner_id: Identifier,
    platform_version: &PlatformVersion,
) -> DataContract {
    let mut contract = json_document_to_contract(RESTAURANTS_CONTRACT, true, platform_version)
        .expect("expected to parse the restaurants contract");
    contract.set_owner_id(owner_id);
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
        .expect("expected to apply the restaurants contract");
    contract
}

/// A real, signed batch transition carrying **one delete transition per
/// document**. With more than one document it is the multi-transition shape
/// `into_high_level_drive_operations` flattens.
///
/// All transitions share one `identity_contract_nonce`: nonce validation reads
/// the *committed* nonce for every transition in the batch, so a batch that
/// incremented per transition would be rejected for the wrong reason.
async fn signed_delete_batch<S: Signer<IdentityPublicKey>>(
    documents: Vec<Document>,
    document_type: DocumentTypeRef<'_>,
    key: &IdentityPublicKey,
    identity_contract_nonce: u64,
    signer: &S,
    platform_version: &PlatformVersion,
) -> Vec<u8> {
    let owner_id = documents.first().expect("at least one document").owner_id();

    let transitions = documents
        .into_iter()
        .map(|document| {
            let delete: DocumentDeleteTransition = DocumentDeleteTransition::from_document(
                document,
                document_type,
                None,
                identity_contract_nonce,
                platform_version,
                None,
                None,
            )
            .expect("expected to build a delete transition");
            BatchedTransition::Document(delete.into())
        })
        .collect();

    let batch: BatchTransition = BatchTransitionV1 {
        owner_id,
        transitions,
        user_fee_increase: 0,
        signature_public_key_id: 0,
        signature: BinaryData::default(),
    }
    .into();

    let mut state_transition: StateTransition = batch.into();
    let required_security_level = document_type.security_level_requirement();
    state_transition
        .sign_external(
            key,
            signer,
            Some(|_: Identifier, _: String| Ok(required_security_level)),
        )
        .await
        .expect("expected to sign the batch");

    state_transition
        .serialize_to_bytes()
        .expect("expected to serialize the batch")
}

/// Process `state_transitions` as one block against one grovedb transaction,
/// commit, and return the results so the caller can assert on them.
fn process_block(
    platform: &TempPlatform<MockCoreRPCLike>,
    platform_state: &PlatformState,
    state_transitions: Vec<Vec<u8>>,
    platform_version: &PlatformVersion,
) -> Vec<StateTransitionExecutionResult> {
    let transaction = platform.drive.grove.start_transaction();
    let result = platform
        .platform
        .process_raw_state_transitions(
            &state_transitions,
            platform_state,
            &BlockInfo::default(),
            &transaction,
            platform_version,
            false,
            None,
        )
        .expect("expected to process the block");
    let results = result.into_execution_results();
    platform
        .drive
        .grove
        .commit_transaction(transaction)
        .unwrap()
        .expect("expected to commit the block");
    results
}

/// Create one `visit` document through a real create transition in its own
/// block, and return it.
#[allow(clippy::too_many_arguments)]
async fn create_visit<S: Signer<IdentityPublicKey>>(
    platform: &TempPlatform<MockCoreRPCLike>,
    platform_state: &PlatformState,
    visit: DocumentTypeRef<'_>,
    owner_id: Identifier,
    key: &IdentityPublicKey,
    signer: &S,
    identity_contract_nonce: u64,
    group: &str,
    guests: u64,
    rng: &mut StdRng,
    platform_version: &PlatformVersion,
) -> Document {
    let entropy = Bytes32::random_with_rng(rng);
    let mut document = visit
        .random_document_with_identifier_and_entropy(
            rng,
            owner_id,
            entropy,
            DocumentFieldFillType::FillIfNotRequired,
            DocumentFieldFillSize::AnyDocumentFillSize,
            platform_version,
        )
        .expect("expected a random visit document");
    document.set(GROUP_PROPERTY, Value::Text(group.to_string()));
    document.set("guests", Value::U64(guests));

    let create = BatchTransition::new_document_creation_transition_from_document(
        document.clone(),
        visit,
        entropy.0,
        key,
        identity_contract_nonce,
        0,
        None,
        signer,
        platform_version,
        None,
    )
    .await
    .expect("expected to build the create transition");

    let results = process_block(
        platform,
        platform_state,
        vec![create
            .serialize_to_bytes()
            .expect("expected to serialize the create transition")],
        platform_version,
    );
    assert_matches!(
        results.as_slice(),
        [StateTransitionExecutionResult::SuccessfulExecution { .. }],
        "the create must be accepted"
    );

    document
}

/// **The cap, observed.**
///
/// `G` holds two `visit` documents and `H` holds one; a single signed batch
/// transition tries to delete both of `G`'s documents. It never reaches the
/// write path: basic structure validation caps a batch transition at
/// `max_transitions_in_documents_batch`, which is 1 at every protocol version,
/// so the batch is refused with `MaxDocumentsTransitionsExceededError` before
/// any drive operation is built.
///
/// That refusal is the whole reason the phantom-group cases in rs-drive's
/// `batched_group_drain` suite are ignored rather than fixed. If this test
/// starts failing because the cap was raised, those cases become live.
#[tokio::test]
async fn a_multi_document_batch_transition_is_refused_by_the_one_transition_limit() {
    let mut platform = TestPlatformBuilder::new()
        .with_latest_protocol_version()
        .build_with_mock_rpc()
        .set_initial_state_structure();

    let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(1.0));

    let platform_state = platform.state.load();
    let platform_version = platform_state
        .current_platform_version()
        .expect("expected the current platform version");

    // Asserted before anything is built, because everything below only has
    // meaning while this holds: once the cap is above 1 the batch is accepted,
    // and an assertion placed after the refusal would simply never run.
    assert_eq!(
        platform_version
            .system_limits
            .max_transitions_in_documents_batch,
        1,
        "raising this cap lets two document operations share one grovedb batch, where the \
         index-group emptiness check cannot see its siblings — leaving a document-less \
         group that still ranks and still proves. Un-ignore rs-drive's batched_group_drain \
         cases and make them pass before raising it"
    );

    let contract = register_restaurants(&platform, identity.id(), platform_version);
    let visit = contract
        .document_type_for_name("visit")
        .expect("expected the visit doctype");

    let mut rng = StdRng::seed_from_u64(4266);

    let mut documents = Vec::new();
    for (index, (group, guests)) in [(H, 2u64), (G, 4), (G, 6)].into_iter().enumerate() {
        documents.push(
            create_visit(
                &platform,
                &platform_state,
                visit,
                identity.id(),
                &key,
                &signer,
                (index + 1) as u64,
                group,
                guests,
                &mut rng,
                platform_version,
            )
            .await,
        );
    }

    let path = indexed_property_name_tree_path(contract.id(), "visit");
    assert_eq!(
        ranked_count_groups(&platform, &path, platform_version),
        vec![(2, G.to_string()), (1, H.to_string())],
        "baseline: G holds two visits, H holds one"
    );

    let drain = signed_delete_batch(
        documents[1..3].to_vec(),
        visit,
        &key,
        4,
        &signer,
        platform_version,
    )
    .await;

    let results = process_block(&platform, &platform_state, vec![drain], platform_version);

    // The error carries the limit it enforced, which pins that the check reads
    // the version's declared cap rather than a constant of its own. The cap's
    // *value* is pinned above, and by platform-version's own unit test.
    assert_matches!(
        results.as_slice(),
        [UnpaidConsensusError(ConsensusError::BasicError(
            BasicError::MaxDocumentsTransitionsExceededError(error)
        ))] if error.max_transitions()
            == platform_version.system_limits.max_transitions_in_documents_batch,
        "a two-transition documents batch must be refused by the max-transitions limit, \
         reporting the limit the version declares. If this batch was accepted, the cap was \
         raised and the phantom-group defect in rs-drive's batched_group_drain suite is now \
         reachable from the network; got {results:?}"
    );

    assert_eq!(
        ranked_count_groups(&platform, &path, platform_version),
        vec![(2, G.to_string()), (1, H.to_string())],
        "a rejected batch must not touch the ranked secondary"
    );
}

/// **The reachable shape of the same scenario.**
///
/// Since one batch may carry only one document transition, the closest a real
/// network gets to "several mutations jointly empty a group" is several state
/// transitions in the same block: two identities each deleting their own
/// document from `G`, both landing in one `process_raw_state_transitions` call
/// against one grovedb transaction.
///
/// This is the case that decides whether the Drive-level defect matters in
/// production. `execute_event` calls `apply_drive_operations` once per state
/// transition, so each delete is its own grovedb batch and the second observes
/// the first through the transaction — but that is exactly the kind of
/// assumption worth executing rather than asserting.
#[tokio::test]
async fn two_state_transitions_in_one_block_drain_a_ranked_group_correctly() {
    let mut platform = TestPlatformBuilder::new()
        .with_latest_protocol_version()
        .build_with_mock_rpc()
        .set_initial_state_structure();

    let (owner, owner_signer, owner_key) =
        setup_identity(&mut platform, 958, dash_to_credits!(1.0));
    let (other, other_signer, other_key) =
        setup_identity(&mut platform, 450, dash_to_credits!(1.0));

    let platform_state = platform.state.load();
    let platform_version = platform_state
        .current_platform_version()
        .expect("expected the current platform version");

    let contract = register_restaurants(&platform, owner.id(), platform_version);
    let visit = contract
        .document_type_for_name("visit")
        .expect("expected the visit doctype");

    let mut rng = StdRng::seed_from_u64(4266);

    // H's single visit and G's first visit belong to `owner`; G's second
    // belongs to `other`, so the two drains are independent transitions with
    // independent nonces.
    let mut owner_documents = Vec::new();
    for (index, (group, guests)) in [(H, 2u64), (G, 4)].into_iter().enumerate() {
        owner_documents.push(
            create_visit(
                &platform,
                &platform_state,
                visit,
                owner.id(),
                &owner_key,
                &owner_signer,
                (index + 1) as u64,
                group,
                guests,
                &mut rng,
                platform_version,
            )
            .await,
        );
    }
    let other_document = create_visit(
        &platform,
        &platform_state,
        visit,
        other.id(),
        &other_key,
        &other_signer,
        1,
        G,
        6,
        &mut rng,
        platform_version,
    )
    .await;

    let path = indexed_property_name_tree_path(contract.id(), "visit");
    assert_eq!(
        ranked_count_groups(&platform, &path, platform_version),
        vec![(2, G.to_string()), (1, H.to_string())],
        "baseline: G holds two visits, H holds one"
    );

    // Both deletes in ONE block, one grovedb transaction, two transitions.
    let first = signed_delete_batch(
        vec![owner_documents[1].clone()],
        visit,
        &owner_key,
        3,
        &owner_signer,
        platform_version,
    )
    .await;
    let second = signed_delete_batch(
        vec![other_document],
        visit,
        &other_key,
        2,
        &other_signer,
        platform_version,
    )
    .await;

    let results = process_block(
        &platform,
        &platform_state,
        vec![first, second],
        platform_version,
    );
    assert_matches!(
        results.as_slice(),
        [
            StateTransitionExecutionResult::SuccessfulExecution { .. },
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        ],
        "both deletes must execute, got {results:?}"
    );

    let primary_g = primary_group_element(&platform, &path, G, platform_version);
    assert_eq!(
        primary_g, None,
        "G's primary value tree must be gone after both its documents are \
         deleted in one block; got {primary_g:?}"
    );
    assert_eq!(
        ranked_count_groups(&platform, &path, platform_version),
        vec![(1, H.to_string())],
        "two deletes in one block must not leave a zero-valued phantom group"
    );

    let issues = platform
        .drive
        .grove
        .verify_grovedb(None, true, false, &platform_version.drive.grove_version)
        .expect("verify_grovedb must run");
    assert!(
        issues.is_empty(),
        "grovedb integrity verification reported issues: {issues:?}"
    );
}

/// The control: the identical two deletes, one per batch transition, in
/// separate blocks. If this fails too, the behaviour under test is not
/// specific to how a block's transitions are batched.
#[tokio::test]
async fn deleting_the_same_two_documents_in_separate_blocks_removes_the_group() {
    let mut platform = TestPlatformBuilder::new()
        .with_latest_protocol_version()
        .build_with_mock_rpc()
        .set_initial_state_structure();

    let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(1.0));

    let platform_state = platform.state.load();
    let platform_version = platform_state
        .current_platform_version()
        .expect("expected the current platform version");

    let contract = register_restaurants(&platform, identity.id(), platform_version);
    let visit = contract
        .document_type_for_name("visit")
        .expect("expected the visit doctype");

    let mut rng = StdRng::seed_from_u64(4266);

    let mut documents = Vec::new();
    for (index, (group, guests)) in [(H, 2u64), (G, 4), (G, 6)].into_iter().enumerate() {
        documents.push(
            create_visit(
                &platform,
                &platform_state,
                visit,
                identity.id(),
                &key,
                &signer,
                (index + 1) as u64,
                group,
                guests,
                &mut rng,
                platform_version,
            )
            .await,
        );
    }

    for (offset, document) in documents[1..3].iter().enumerate() {
        let delete = signed_delete_batch(
            vec![document.clone()],
            visit,
            &key,
            (4 + offset) as u64,
            &signer,
            platform_version,
        )
        .await;

        let results = process_block(&platform, &platform_state, vec![delete], platform_version);
        assert_matches!(
            results.as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }],
            "the single delete must be accepted, got {results:?}"
        );
    }

    let path = indexed_property_name_tree_path(contract.id(), "visit");
    assert_eq!(
        primary_group_element(&platform, &path, G, platform_version),
        None,
        "the control must drain G's primary value tree"
    );
    assert_eq!(
        ranked_count_groups(&platform, &path, platform_version),
        vec![(1, H.to_string())],
        "the control must remove G from the ranked secondary"
    );
}
