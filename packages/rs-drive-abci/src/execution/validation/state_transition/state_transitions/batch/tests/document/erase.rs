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
