//! Deleting and erasing keep-history documents through signed transitions.
//!
//! Erasure is authorized once. The document's owner commits it, and the record
//! that commitment leaves in state is what lets any identity finish the work,
//! so an owner who loses their keys, their funds or their permission cannot
//! strand a half-erased document.

use super::*;
use crate::rpc::core::MockCoreRPCLike;
use crate::test::helpers::setup::TempPlatform;
use dpp::consensus::basic::BasicError;
use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::DataContract;
use dpp::document::Document;
use dpp::identifier::Identifier;
use dpp::identity::{Identity, IdentityPublicKey, SecurityLevel};
use dpp::prelude::IdentityNonce;
use dpp::state_transition::StateTransition;
use dpp::tokens::token_payment_info::v0::TokenPaymentInfoV0;
use dpp::tokens::token_payment_info::TokenPaymentInfo;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use drive::drive::document::history::{
    DocumentHistoryQueryV1, DocumentHistorySelector, DocumentHistoryState,
};
use drive::drive::document::lifecycle::DocumentLifecycleState;
use drive::util::storage_flags::StorageFlags;
use rand::rngs::StdRng;
use rand::SeedableRng;
use simple_signer::signer::SimpleSigner;

const ERASABLE_CONTRACT: &str =
    "tests/supporting_files/contract/note/note-contract-keep-history-erasable.json";

fn process(
    platform: &mut TempPlatform<MockCoreRPCLike>,
    serialized: Vec<u8>,
) -> StateTransitionExecutionResult {
    let state = platform.state.load();
    let version = state.current_platform_version().unwrap();
    let transaction = platform.drive.grove.start_transaction();
    let result = platform
        .platform
        .process_raw_state_transitions(
            &[serialized],
            &state,
            &BlockInfo::default(),
            &transaction,
            version,
            false,
            None,
        )
        .expect("expected transition processing");
    platform
        .drive
        .grove
        .commit_transaction(transaction)
        .unwrap()
        .unwrap();
    assert_eq!(result.execution_results().len(), 1);
    result.into_execution_results().remove(0)
}

fn assert_successful(result: &StateTransitionExecutionResult, context: &str) {
    assert_matches!(
        result,
        StateTransitionExecutionResult::SuccessfulExecution { .. },
        "{context}"
    );
}

/// A platform at protocol 14 with the erasable note contract applied, one
/// identity owning a `note` document, and a second identity that owns nothing.
struct Fixture {
    platform: TempPlatform<MockCoreRPCLike>,
    contract: DataContract,
    owner: Identity,
    owner_key: IdentityPublicKey,
    owner_signer: SimpleSigner,
    stranger: Identity,
    stranger_key: IdentityPublicKey,
    stranger_signer: SimpleSigner,
    document: Document,
    entropy: Bytes32,
    nonce: IdentityNonce,
    stranger_nonce: IdentityNonce,
}

impl Fixture {
    async fn new(document_type_name: &str) -> Self {
        let platform_version = PlatformVersion::get(14).expect("protocol 14 exists");
        let mut platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(14)
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let contract = json_document_to_contract(ERASABLE_CONTRACT, true, platform_version)
            .expect("the erasable note contract must pass full validation at protocol 14");
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

        let (owner, owner_signer, owner_key) =
            setup_identity(&mut platform, 1001, dash_to_credits!(1.0));
        let (stranger, stranger_signer, stranger_key) =
            setup_identity(&mut platform, 1002, dash_to_credits!(1.0));

        let mut rng = StdRng::seed_from_u64(1291);
        let document_type = contract
            .document_type_for_name(document_type_name)
            .expect("expected the document type");
        let entropy = Bytes32::random_with_rng(&mut rng);
        let document = document_type
            .random_document_with_identifier_and_entropy(
                &mut rng,
                owner.id(),
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random note");

        let mut fixture = Fixture {
            platform,
            contract,
            owner,
            owner_key,
            owner_signer,
            stranger,
            stranger_key,
            stranger_signer,
            document: document.clone(),
            entropy,
            nonce: 1,
            stranger_nonce: 1,
        };
        fixture.create(document, entropy, document_type_name).await;
        fixture
    }

    fn document_type(&self, name: &str) -> DocumentTypeRef<'_> {
        self.contract
            .document_type_for_name(name)
            .expect("expected the document type")
    }

    async fn create(&mut self, document: Document, entropy: Bytes32, document_type_name: &str) {
        let platform_version = PlatformVersion::get(14).unwrap();
        let transition = BatchTransition::new_document_creation_transition_from_document(
            document,
            self.document_type(document_type_name),
            entropy.0,
            &self.owner_key,
            self.nonce,
            0,
            None,
            &self.owner_signer,
            platform_version,
            None,
        )
        .await
        .expect("expected a create transition");
        self.nonce += 1;
        let result = process(
            &mut self.platform,
            transition.serialize_to_bytes().expect("serialized"),
        );
        assert_successful(&result, "the create must succeed");
    }

    async fn delete_as_owner(
        &mut self,
        document_type_name: &str,
    ) -> StateTransitionExecutionResult {
        let platform_version = PlatformVersion::get(14).unwrap();
        let mut document = self.document.clone();
        document.set_revision(Some(1));
        let transition = BatchTransition::new_document_deletion_transition_from_document(
            document,
            self.document_type(document_type_name),
            &self.owner_key,
            self.nonce,
            0,
            None,
            &self.owner_signer,
            platform_version,
            None,
        )
        .await
        .expect("expected a delete transition");
        self.nonce += 1;
        process(
            &mut self.platform,
            transition.serialize_to_bytes().expect("serialized"),
        )
    }

    async fn erase(
        &mut self,
        document_type_name: &str,
        as_owner: bool,
        token_payment_info: Option<TokenPaymentInfo>,
    ) -> StateTransitionExecutionResult {
        let platform_version = PlatformVersion::get(14).unwrap();
        let mut document = self.document.clone();
        document.set_revision(Some(1));
        if !as_owner {
            document.set_owner_id(self.stranger.id());
        }
        let transition = if let Some(token_payment_info) = token_payment_info {
            // The erase factory refuses to carry token payment information, so
            // a transition that does has to be built as a delete and rewritten
            // into an erase, exactly as a hostile client would.
            let delete = BatchTransition::new_document_deletion_transition_from_document(
                document,
                self.document_type(document_type_name),
                &self.owner_key,
                self.nonce,
                0,
                Some(token_payment_info),
                &self.owner_signer,
                platform_version,
                None,
            )
            .await
            .expect("expected a delete transition to rewrite");
            self.nonce += 1;
            rewrite_delete_as_erase(
                delete,
                &self.owner_key,
                &self.owner_signer,
                self.document_type(document_type_name)
                    .security_level_requirement(),
            )
            .await
        } else if as_owner {
            let transition = BatchTransition::new_document_erase_transition_from_document(
                document,
                self.document_type(document_type_name),
                &self.owner_key,
                self.nonce,
                0,
                &self.owner_signer,
                platform_version,
                None,
            )
            .await
            .expect("expected an erase transition");
            self.nonce += 1;
            transition
        } else {
            let transition = BatchTransition::new_document_erase_transition_from_document(
                document,
                self.document_type(document_type_name),
                &self.stranger_key,
                self.stranger_nonce,
                0,
                &self.stranger_signer,
                platform_version,
                None,
            )
            .await
            .expect("expected an erase transition");
            self.stranger_nonce += 1;
            transition
        };
        process(
            &mut self.platform,
            transition.serialize_to_bytes().expect("serialized"),
        )
    }

    /// Writes revisions straight into storage so the document retains more than
    /// one erase chunk can remove. How they got there does not matter to an
    /// erase: it reads what the history holds.
    fn retain_revisions(&mut self, document_type_name: &str, revisions: u64) {
        let platform_version = PlatformVersion::get(14).unwrap();
        let contract = self.contract.clone();
        let document_type = contract
            .document_type_for_name(document_type_name)
            .expect("expected the document type");
        // The same flags the create wrote, so overwriting the current pointer
        // replaces exactly as many bytes as it holds.
        let flags = Some(std::borrow::Cow::Owned(StorageFlags::new_single_epoch(
            0,
            Some(self.owner.id().to_buffer()),
        )));
        let mut document = self.document.clone();
        for revision in 2..=revisions {
            document.set_revision(Some(revision));
            self.platform
                .drive
                .add_document_for_contract(
                    drive::util::object_size_info::DocumentAndContractInfo {
                        owned_document_info: drive::util::object_size_info::OwnedDocumentInfo {
                            document_info:
                                drive::util::object_size_info::DocumentInfo::DocumentRefInfo((
                                    &document,
                                    flags.clone(),
                                )),
                            owner_id: None,
                        },
                        contract: &contract,
                        document_type,
                    },
                    true,
                    BlockInfo::default_with_time(1_000 + revision),
                    true,
                    None,
                    platform_version,
                    None,
                )
                .expect("expected to retain a revision");
        }
    }

    /// Builds an erase without processing it, so several can share one block.
    async fn erase_transition(&mut self, document_type_name: &str, as_owner: bool) -> Vec<u8> {
        let platform_version = PlatformVersion::get(14).unwrap();
        let mut document = self.document.clone();
        document.set_revision(Some(1));
        let (key, signer, nonce) = if as_owner {
            let nonce = self.nonce;
            self.nonce += 1;
            (&self.owner_key, &self.owner_signer, nonce)
        } else {
            document.set_owner_id(self.stranger.id());
            let nonce = self.stranger_nonce;
            self.stranger_nonce += 1;
            (&self.stranger_key, &self.stranger_signer, nonce)
        };
        BatchTransition::new_document_erase_transition_from_document(
            document,
            self.document_type(document_type_name),
            key,
            nonce,
            0,
            signer,
            platform_version,
            None,
        )
        .await
        .expect("expected an erase transition")
        .serialize_to_bytes()
        .expect("serialized")
    }

    fn remaining_revisions(&self, document_type_name: &str) -> u64 {
        let query = DocumentHistoryQueryV1 {
            contract_id: self.contract.id().to_buffer(),
            document_type_name: document_type_name.to_string(),
            document_id: self.document.id().to_buffer(),
            selector: DocumentHistorySelector::StartAtTime(0),
            limit: Some(1),
        };
        self.platform
            .drive
            .fetch_document_history_v1(
                &query,
                self.document_type(document_type_name),
                None,
                PlatformVersion::get(14).unwrap(),
            )
            .expect("expected to read the history")
            .lifecycle
            .remaining_revisions
    }

    fn history_state(&self, document_type_name: &str) -> DocumentHistoryState {
        let query = DocumentHistoryQueryV1 {
            contract_id: self.contract.id().to_buffer(),
            document_type_name: document_type_name.to_string(),
            document_id: self.document.id().to_buffer(),
            selector: DocumentHistorySelector::StartAtTime(0),
            limit: Some(1),
        };
        self.platform
            .drive
            .fetch_document_history_v1(
                &query,
                self.document_type(document_type_name),
                None,
                PlatformVersion::get(14).unwrap(),
            )
            .expect("expected to read the history")
            .lifecycle
            .state
    }

    fn lifecycle(&self, document_type_name: &str) -> DocumentLifecycleState {
        self.platform
            .drive
            .fetch_document_lifecycle(
                &self.contract,
                self.document_type(document_type_name),
                self.document.id(),
                None,
                None,
                PlatformVersion::get(14).unwrap(),
            )
            .expect("expected to read the lifecycle")
            .0
    }
}

/// Rebuilds a signed batch, swapping its single delete transition for an erase
/// carrying the same base — including the token payment information an erase
/// must refuse.
async fn rewrite_delete_as_erase(
    transition: StateTransition,
    key: &IdentityPublicKey,
    signer: &SimpleSigner,
    security_level: SecurityLevel,
) -> StateTransition {
    use dpp::state_transition::batch_transition::batched_transition::document_erase_transition::DocumentEraseTransitionV0;
    use dpp::state_transition::batch_transition::batched_transition::{
        BatchedTransition, DocumentEraseTransition, DocumentTransition,
    };
    use dpp::state_transition::batch_transition::document_base_transition::document_base_transition_trait::DocumentBaseTransitionAccessors;
    use dpp::state_transition::batch_transition::BatchTransition;

    let StateTransition::Batch(BatchTransition::V1(mut batch)) = transition else {
        panic!("expected a v1 batch transition");
    };
    let BatchedTransition::Document(DocumentTransition::Delete(delete)) =
        batch.transitions.remove(0)
    else {
        panic!("expected a single document delete");
    };
    batch
        .transitions
        .push(BatchedTransition::Document(DocumentTransition::Erase(
            DocumentEraseTransition::V0(DocumentEraseTransitionV0 {
                base: delete.base().clone(),
            }),
        )));
    let mut rebuilt: StateTransition = BatchTransition::V1(batch).into();
    rebuilt
        .sign_external(
            key,
            signer,
            Some(|_: Identifier, _: String| Ok(security_level)),
        )
        .await
        .expect("expected to re-sign the rewritten batch");
    rebuilt
}

#[tokio::test]
async fn should_delete_then_erase_a_keep_history_document() {
    let mut fixture = Fixture::new("note").await;
    assert_matches!(fixture.lifecycle("note"), DocumentLifecycleState::Active(_));

    assert_successful(
        &fixture.delete_as_owner("note").await,
        "the delete must succeed",
    );
    assert_matches!(
        fixture.lifecycle("note"),
        DocumentLifecycleState::Deleted(_)
    );

    assert_successful(
        &fixture.erase("note", true, None).await,
        "the erase must succeed",
    );
    assert_matches!(fixture.lifecycle("note"), DocumentLifecycleState::Absent);
}

/// A delete never escalates into anything that removes a revision, so asking
/// twice is a paid error rather than a second, deeper removal.
#[tokio::test]
async fn should_reject_a_second_delete_of_the_same_document() {
    let mut fixture = Fixture::new("note").await;
    assert_successful(&fixture.delete_as_owner("note").await, "the first delete");

    let result = fixture.delete_as_owner("note").await;
    assert_matches!(
        result,
        StateTransitionExecutionResult::PaidConsensusError {
            error: ConsensusError::StateError(StateError::DocumentNotFoundError(_)),
            ..
        }
    );
}

/// The id stays taken while the revisions are retained, even though nothing in
/// the primary-key tree says so.
#[tokio::test]
async fn should_reject_a_create_over_a_deleted_documents_id() {
    let mut fixture = Fixture::new("note").await;
    assert_successful(&fixture.delete_as_owner("note").await, "the delete");

    let platform_version = PlatformVersion::get(14).unwrap();
    let document = fixture.document.clone();
    // The same id the deleted document had: a document id is derived from its
    // creator, contract, type and entropy, so re-creating with the same inputs
    // targets exactly the reserved id.
    let entropy = fixture.entropy;
    let transition = BatchTransition::new_document_creation_transition_from_document(
        document,
        fixture.document_type("note"),
        entropy.0,
        &fixture.owner_key,
        fixture.nonce,
        0,
        None,
        &fixture.owner_signer,
        platform_version,
        None,
    )
    .await
    .expect("expected a create transition");
    let result = process(
        &mut fixture.platform,
        transition.serialize_to_bytes().expect("serialized"),
    );
    assert_matches!(
        result,
        StateTransitionExecutionResult::PaidConsensusError {
            error: ConsensusError::StateError(StateError::DocumentAlreadyPresentError(_)),
            ..
        }
    );
}

/// Delete and erase stay two intents. Erasing a document that is still current
/// would let one signature do the work of two.
#[tokio::test]
async fn should_reject_an_erase_of_a_document_that_has_not_been_deleted() {
    let mut fixture = Fixture::new("note").await;

    let result = fixture.erase("note", true, None).await;
    assert_matches!(
        result,
        StateTransitionExecutionResult::PaidConsensusError {
            error: ConsensusError::BasicError(BasicError::InvalidDocumentTransitionActionError(_)),
            ..
        }
    );
    assert_matches!(fixture.lifecycle("note"), DocumentLifecycleState::Active(_));
}

/// A type that keeps history but never opted into erasure keeps its deleted
/// documents' revisions forever.
#[tokio::test]
async fn should_reject_an_erase_of_a_type_that_did_not_ask_for_it() {
    let mut fixture = Fixture::new("permanentNote").await;
    assert!(!fixture
        .document_type("permanentNote")
        .documents_can_be_erased());
    assert_successful(
        &fixture.delete_as_owner("permanentNote").await,
        "the delete must still succeed",
    );

    let result = fixture.erase("permanentNote", true, None).await;
    assert_matches!(
        result,
        StateTransitionExecutionResult::PaidConsensusError {
            error: ConsensusError::BasicError(BasicError::InvalidDocumentTransitionActionError(_)),
            ..
        }
    );
    assert_matches!(
        fixture.lifecycle("permanentNote"),
        DocumentLifecycleState::Deleted(_)
    );
}

/// Erase has no token cost of its own. Accepting one would let a continuation,
/// which any identity may submit, move somebody else's tokens.
#[tokio::test]
async fn should_reject_an_erase_that_carries_token_payment_information() {
    let mut fixture = Fixture::new("note").await;
    assert_successful(&fixture.delete_as_owner("note").await, "the delete");

    let token_payment_info = TokenPaymentInfo::V0(TokenPaymentInfoV0 {
        payment_token_contract_id: Some(Identifier::new([5u8; 32])),
        token_contract_position: 0,
        minimum_token_cost: None,
        maximum_token_cost: Some(10),
        gas_fees_paid_by: Default::default(),
    });
    let result = fixture.erase("note", true, Some(token_payment_info)).await;
    assert_matches!(
        result,
        StateTransitionExecutionResult::PaidConsensusError {
            error: ConsensusError::BasicError(BasicError::InvalidDocumentTransitionActionError(_)),
            ..
        }
    );
    assert_matches!(
        fixture.lifecycle("note"),
        DocumentLifecycleState::Deleted(_)
    );
}

/// The first erase is the authorized act, so it must come from the owner.
#[tokio::test]
async fn should_reject_an_erase_start_by_an_identity_that_does_not_own_the_document() {
    let mut fixture = Fixture::new("note").await;
    assert_successful(&fixture.delete_as_owner("note").await, "the delete");

    let result = fixture.erase("note", false, None).await;
    assert_matches!(
        result,
        StateTransitionExecutionResult::PaidConsensusError {
            error: ConsensusError::StateError(StateError::DocumentOwnerIdMismatchError(_)),
            ..
        }
    );
    assert_matches!(
        fixture.lifecycle("note"),
        DocumentLifecycleState::Deleted(_)
    );
}

/// A document already invisible to every read is not there to be deleted
/// again, whether or not its erasure has begun.
#[tokio::test]
async fn should_reject_a_delete_of_a_document_that_has_already_been_deleted() {
    let mut fixture = Fixture::new("note").await;
    assert_successful(&fixture.delete_as_owner("note").await, "the delete");
    assert_successful(&fixture.erase("note", true, None).await, "the erase");

    let result = fixture.delete_as_owner("note").await;
    assert_matches!(
        result,
        StateTransitionExecutionResult::PaidConsensusError {
            error: ConsensusError::StateError(StateError::DocumentNotFoundError(_)),
            ..
        }
    );
}

/// An erase of an id that holds nothing is not silently accepted.
#[tokio::test]
async fn should_reject_an_erase_of_an_id_that_holds_nothing() {
    let mut fixture = Fixture::new("note").await;
    assert_successful(&fixture.delete_as_owner("note").await, "the delete");
    assert_successful(&fixture.erase("note", true, None).await, "the erase");

    let result = fixture.erase("note", true, None).await;
    assert_matches!(
        result,
        StateTransitionExecutionResult::PaidConsensusError {
            error: ConsensusError::StateError(StateError::DocumentNotFoundError(_)),
            ..
        }
    );
}

/// Once an erasure is committed, the committed record is the authorization, so
/// an identity that owns nothing can finish the work. An owner who loses their
/// keys, their funds or their permission cannot strand a half-erased document.
#[tokio::test]
async fn should_let_any_identity_finish_an_erasure_its_owner_started() {
    let chunk = PlatformVersion::get(14)
        .unwrap()
        .system_limits
        .max_document_revisions_erased_per_transition
        .expect("protocol 14 bounds the chunk") as u64;

    let mut fixture = Fixture::new("note").await;
    fixture.retain_revisions("note", chunk + 1);
    assert_eq!(
        fixture.remaining_revisions("note"),
        chunk + 1,
        "the fixture must retain more than one chunk can remove"
    );
    assert_successful(&fixture.delete_as_owner("note").await, "the delete");

    // A stranger cannot start one.
    assert_matches!(
        fixture.erase("note", false, None).await,
        StateTransitionExecutionResult::PaidConsensusError {
            error: ConsensusError::StateError(StateError::DocumentOwnerIdMismatchError(_)),
            ..
        }
    );

    assert_successful(
        &fixture.erase("note", true, None).await,
        "the owner commits the erasure",
    );
    assert_eq!(fixture.history_state("note"), DocumentHistoryState::Erasing);

    // And now the same stranger can finish it.
    assert_successful(
        &fixture.erase("note", false, None).await,
        "a continuation needs no authorization of its own",
    );
    assert_matches!(fixture.lifecycle("note"), DocumentLifecycleState::Absent);
}

/// A document whose erasure has begun is not there to be deleted again.
#[tokio::test]
async fn should_reject_a_delete_of_a_document_whose_erasure_has_begun() {
    let chunk = PlatformVersion::get(14)
        .unwrap()
        .system_limits
        .max_document_revisions_erased_per_transition
        .expect("protocol 14 bounds the chunk") as u64;

    let mut fixture = Fixture::new("note").await;
    fixture.retain_revisions("note", chunk + 1);
    assert_successful(&fixture.delete_as_owner("note").await, "the delete");
    assert_successful(&fixture.erase("note", true, None).await, "the erase start");
    assert_eq!(fixture.history_state("note"), DocumentHistoryState::Erasing);

    assert_matches!(
        fixture.delete_as_owner("note").await,
        StateTransitionExecutionResult::PaidConsensusError {
            error: ConsensusError::StateError(StateError::DocumentNotFoundError(_)),
            ..
        }
    );
}

/// Two erases in one block each see the previous one's effect, because each
/// transition is applied into the block transaction before the next is
/// validated.
#[tokio::test]
async fn should_finish_an_erasure_across_two_transitions_in_one_block() {
    let chunk = PlatformVersion::get(14)
        .unwrap()
        .system_limits
        .max_document_revisions_erased_per_transition
        .expect("protocol 14 bounds the chunk") as u64;

    let mut fixture = Fixture::new("note").await;
    fixture.retain_revisions("note", chunk + 1);
    assert_successful(&fixture.delete_as_owner("note").await, "the delete");

    let first = fixture.erase_transition("note", true).await;
    let second = fixture.erase_transition("note", false).await;

    let state = fixture.platform.state.load();
    let version = state.current_platform_version().unwrap();
    let transaction = fixture.platform.drive.grove.start_transaction();
    let result = fixture
        .platform
        .platform
        .process_raw_state_transitions(
            &[first, second],
            &state,
            &BlockInfo::default(),
            &transaction,
            version,
            false,
            None,
        )
        .expect("expected transition processing");
    fixture
        .platform
        .drive
        .grove
        .commit_transaction(transaction)
        .unwrap()
        .unwrap();

    assert_eq!(result.valid_count(), 2, "both erases must execute");
    assert_matches!(fixture.lifecycle("note"), DocumentLifecycleState::Absent);
}

/// A delete and the erase that follows it can share a block: the erase reads
/// the lifecycle record the delete has already written into the same
/// transaction.
#[tokio::test]
async fn should_delete_and_erase_in_one_block() {
    let mut fixture = Fixture::new("note").await;

    let platform_version = PlatformVersion::get(14).unwrap();
    let mut document = fixture.document.clone();
    document.set_revision(Some(1));
    let delete = BatchTransition::new_document_deletion_transition_from_document(
        document.clone(),
        fixture.document_type("note"),
        &fixture.owner_key,
        fixture.nonce,
        0,
        None,
        &fixture.owner_signer,
        platform_version,
        None,
    )
    .await
    .expect("expected a delete transition")
    .serialize_to_bytes()
    .expect("serialized");
    fixture.nonce += 1;
    let erase = fixture.erase_transition("note", true).await;

    let state = fixture.platform.state.load();
    let version = state.current_platform_version().unwrap();
    let transaction = fixture.platform.drive.grove.start_transaction();
    let result = fixture
        .platform
        .platform
        .process_raw_state_transitions(
            &[delete, erase],
            &state,
            &BlockInfo::default(),
            &transaction,
            version,
            false,
            None,
        )
        .expect("expected transition processing");
    fixture
        .platform
        .drive
        .grove
        .commit_transaction(transaction)
        .unwrap()
        .unwrap();

    assert_eq!(
        result.valid_count(),
        2,
        "the erase must see the delete that shares its block"
    );
    assert_matches!(fixture.lifecycle("note"), DocumentLifecycleState::Absent);
}

/// The balance updates an erase's refunds cause land outside its own fee
/// result, against identities that had nothing to do with it. The chunk bound
/// limits how many there can be; this pins that the submitter pays for them,
/// in the estimate that admits the transition and in the fee actually charged.
#[tokio::test]
async fn should_charge_an_erase_for_the_refund_recipients_it_can_credit() {
    use dpp::block::epoch::Epoch;

    let mut fixture = Fixture::new("note").await;
    assert_successful(&fixture.delete_as_owner("note").await, "the delete");

    let platform_version = PlatformVersion::get(14).unwrap();
    let recipient_cost = fixture
        .platform
        .drive
        .erase_refund_recipient_cost(&Epoch::new(0).unwrap(), platform_version)
        .expect("expected the recipient work to be priced");
    assert!(recipient_cost.processing_fee > 0);

    let result = fixture.erase("note", true, None).await;
    let StateTransitionExecutionResult::SuccessfulExecution {
        estimated_fees,
        fee_result,
        ..
    } = result
    else {
        panic!("expected the erase to succeed, got {result:?}");
    };
    let estimated_fees = estimated_fees.expect("an erase is admitted against an estimate");

    assert!(
        fee_result.processing_fee >= recipient_cost.processing_fee,
        "the charged fee must include the recipient work: {} is below {}",
        fee_result.processing_fee,
        recipient_cost.processing_fee
    );
    assert!(
        estimated_fees.processing_fee >= recipient_cost.processing_fee,
        "so must the estimate that admits it: {} is below {}",
        estimated_fees.processing_fee,
        recipient_cost.processing_fee
    );
}

/// A paid failure has to leave its nonce bump behind, or the same rejected
/// transition can be replayed for free. The refusals the lifecycle adds are no
/// exception.
#[tokio::test]
async fn should_persist_the_nonce_bump_on_a_refused_erase() {
    let mut fixture = Fixture::new("note").await;

    let nonce_after = |fixture: &Fixture| {
        fixture
            .platform
            .drive
            .fetch_identity_contract_nonce(
                fixture.owner.id().to_buffer(),
                fixture.contract.id().to_buffer(),
                true,
                None,
                PlatformVersion::get(14).unwrap(),
            )
            .expect("expected to read the identity contract nonce")
            .expect("the create already wrote one")
    };
    let before = nonce_after(&fixture);

    // An erase of a document that has not been deleted.
    assert_matches!(
        fixture.erase("note", true, None).await,
        StateTransitionExecutionResult::PaidConsensusError { .. }
    );
    let after_refusal = nonce_after(&fixture);
    assert_eq!(
        after_refusal,
        before + 1,
        "a refused erase must still consume its nonce"
    );

    // And an erase whose token payment is refused in the transformer, which is
    // a different refusal path with its own nonce-bump action.
    let token_payment_info = TokenPaymentInfo::V0(TokenPaymentInfoV0 {
        payment_token_contract_id: Some(Identifier::new([5u8; 32])),
        token_contract_position: 0,
        minimum_token_cost: None,
        maximum_token_cost: Some(10),
        gas_fees_paid_by: Default::default(),
    });
    assert_matches!(
        fixture.erase("note", true, Some(token_payment_info)).await,
        StateTransitionExecutionResult::PaidConsensusError { .. }
    );
    assert_eq!(
        nonce_after(&fixture),
        after_refusal + 1,
        "the transformer's refusal must consume its nonce too"
    );
}

/// A delete and a create of the same id in one block: the create sees the
/// delete's reservation through the block transaction and is refused, so an id
/// cannot be recycled while its revisions are retained.
#[tokio::test]
async fn should_reject_a_create_that_follows_a_delete_of_the_same_id_in_one_block() {
    let mut fixture = Fixture::new("note").await;

    let platform_version = PlatformVersion::get(14).unwrap();
    let mut document = fixture.document.clone();
    document.set_revision(Some(1));
    let delete = BatchTransition::new_document_deletion_transition_from_document(
        document,
        fixture.document_type("note"),
        &fixture.owner_key,
        fixture.nonce,
        0,
        None,
        &fixture.owner_signer,
        platform_version,
        None,
    )
    .await
    .expect("expected a delete transition")
    .serialize_to_bytes()
    .expect("serialized");
    fixture.nonce += 1;

    let create = BatchTransition::new_document_creation_transition_from_document(
        fixture.document.clone(),
        fixture.document_type("note"),
        fixture.entropy.0,
        &fixture.owner_key,
        fixture.nonce,
        0,
        None,
        &fixture.owner_signer,
        platform_version,
        None,
    )
    .await
    .expect("expected a create transition")
    .serialize_to_bytes()
    .expect("serialized");
    fixture.nonce += 1;

    let state = fixture.platform.state.load();
    let version = state.current_platform_version().unwrap();
    let transaction = fixture.platform.drive.grove.start_transaction();
    let result = fixture
        .platform
        .platform
        .process_raw_state_transitions(
            &[delete, create],
            &state,
            &BlockInfo::default(),
            &transaction,
            version,
            false,
            None,
        )
        .expect("expected transition processing");
    fixture
        .platform
        .drive
        .grove
        .commit_transaction(transaction)
        .unwrap()
        .unwrap();

    assert_eq!(result.valid_count(), 1, "only the delete may execute");
    assert_eq!(result.invalid_paid_count(), 1, "the create must be refused");
    assert_matches!(
        fixture.lifecycle("note"),
        DocumentLifecycleState::Deleted(_)
    );
}

/// Replacing a deleted document is refused for the same reason a second delete
/// is: nothing an ordinary read can see is there any more. This is the existing
/// not-found path, pinned here because the lifecycle is what makes the document
/// invisible while its revisions survive.
#[tokio::test]
async fn should_reject_a_replace_of_a_deleted_document() {
    let mut fixture = Fixture::new("note").await;
    assert_successful(&fixture.delete_as_owner("note").await, "the delete");

    let platform_version = PlatformVersion::get(14).unwrap();
    let mut replacement = fixture.document.clone();
    replacement.set_revision(Some(2));
    let transition = BatchTransition::new_document_replacement_transition_from_document(
        replacement,
        fixture.document_type("note"),
        &fixture.owner_key,
        fixture.nonce,
        0,
        None,
        &fixture.owner_signer,
        platform_version,
        None,
    )
    .await
    .expect("expected a replace transition");
    fixture.nonce += 1;

    let result = process(
        &mut fixture.platform,
        transition.serialize_to_bytes().expect("serialized"),
    );
    assert_matches!(
        result,
        StateTransitionExecutionResult::PaidConsensusError {
            error: ConsensusError::StateError(StateError::DocumentNotFoundError(_)),
            ..
        }
    );
    assert_matches!(
        fixture.lifecycle("note"),
        DocumentLifecycleState::Deleted(_)
    );
}
