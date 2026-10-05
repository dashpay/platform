//! The ABCI harness the `refersTo` suites share: a platform at the latest
//! protocol version with one funded identity and a fixture contract applied
//! directly, and create, replace and delete transitions processed and
//! committed one at a time.

use super::*;
use crate::platform_types::platform_state::PlatformState;
use crate::platform_types::state_transitions_processing_result::StateTransitionsProcessingResult;
use crate::rpc::core::MockCoreRPCLike;
use crate::test::helpers::setup::TempPlatform;
use dpp::data_contract::accessors::v0::DataContractV0Setters;
use dpp::document::Document;
use dpp::identifier::Identifier;
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::prelude::DataContract;
use dpp::state_transition::StateTransition;
use simple_signer::signer::SimpleSigner;
use std::sync::Arc;

/// Applies the fixture contract at `path` owned by `owner_id`, parsed with
/// full validation when `validate` is set.
pub(super) fn register_contract_at(
    platform: &TempPlatform<MockCoreRPCLike>,
    path: &str,
    owner_id: Identifier,
    validate: bool,
    platform_version: &PlatformVersion,
) -> DataContract {
    let mut contract = json_document_to_contract(path, validate, platform_version)
        .expect("expected to parse the fixture contract");
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
        .expect("expected to apply the fixture contract");
    contract
}

pub(super) fn process_and_commit(
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

pub(super) fn assert_successful(result: &StateTransitionsProcessingResult, because: &str) {
    assert_matches!(
        result.execution_results().as_slice(),
        [StateTransitionExecutionResult::SuccessfulExecution { .. }],
        "{because}"
    );
}

/// Creates a document of `type_name` with exactly `properties` set and
/// returns it with the processing result.
#[allow(clippy::too_many_arguments)]
pub(super) async fn create_document<S: Signer<IdentityPublicKey>>(
    platform: &TempPlatform<MockCoreRPCLike>,
    platform_state: &PlatformState,
    contract: &DataContract,
    type_name: &str,
    properties: &[(&str, Value)],
    owner: Identifier,
    key: &IdentityPublicKey,
    nonce: u64,
    signer: &S,
    rng: &mut StdRng,
    platform_version: &PlatformVersion,
) -> (Document, StateTransitionsProcessingResult) {
    let document_type = contract
        .document_type_for_name(type_name)
        .expect("doctype exists");
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
    for (property, value) in properties {
        document.set(property, value.clone());
    }
    // The id commits to the create transition's nonce: give the local
    // copy the id the transition will carry, since the tests reference
    // and act on it afterwards.
    document
        .set_id_for_creation(document_type, &entropy.0, nonce, platform_version)
        .expect("expected the creation id");
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
    let result = process_and_commit(platform, platform_state, &create, platform_version);
    (document, result)
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn replace_document<S: Signer<IdentityPublicKey>>(
    platform: &TempPlatform<MockCoreRPCLike>,
    platform_state: &PlatformState,
    contract: &DataContract,
    type_name: &str,
    document: &Document,
    key: &IdentityPublicKey,
    nonce: u64,
    signer: &S,
    platform_version: &PlatformVersion,
) -> StateTransitionsProcessingResult {
    let document_type = contract
        .document_type_for_name(type_name)
        .expect("doctype exists");
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

#[allow(clippy::too_many_arguments)]
pub(super) async fn delete_document<S: Signer<IdentityPublicKey>>(
    platform: &TempPlatform<MockCoreRPCLike>,
    platform_state: &PlatformState,
    contract: &DataContract,
    type_name: &str,
    document: &Document,
    key: &IdentityPublicKey,
    nonce: u64,
    signer: &S,
    platform_version: &PlatformVersion,
) -> StateTransitionsProcessingResult {
    let document_type = contract
        .document_type_for_name(type_name)
        .expect("doctype exists");
    let delete = BatchTransition::new_document_deletion_transition_from_document(
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
    .expect("expected the delete transition");
    process_and_commit(platform, platform_state, &delete, platform_version)
}

/// A fresh platform with one funded identity owning the fixture contract at
/// `contract_path`, and the nonce and randomness its transitions use.
pub(super) struct ReferenceTestSetup {
    pub(super) platform: TempPlatform<MockCoreRPCLike>,
    pub(super) platform_state: Arc<PlatformState>,
    pub(super) contract: DataContract,
    pub(super) owner: Identifier,
    pub(super) signer: SimpleSigner,
    pub(super) key: IdentityPublicKey,
    pub(super) rng: StdRng,
    pub(super) nonce: u64,
}

impl ReferenceTestSetup {
    pub(super) fn new(contract_path: &str, seed: u64) -> Self {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();
        let platform_state = platform.state.load_full();
        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(1.0));
        let contract = register_contract_at(
            &platform,
            contract_path,
            identity.id(),
            true,
            platform_version,
        );
        Self {
            platform,
            platform_state,
            contract,
            owner: identity.id(),
            signer,
            key,
            rng: StdRng::seed_from_u64(seed),
            nonce: 1,
        }
    }

    pub(super) fn next_nonce(&mut self) -> u64 {
        self.nonce += 1;
        self.nonce
    }

    pub(super) async fn create(
        &mut self,
        type_name: &str,
        properties: &[(&str, Value)],
    ) -> (Document, StateTransitionsProcessingResult) {
        let nonce = self.next_nonce();
        create_document(
            &self.platform,
            &self.platform_state,
            &self.contract,
            type_name,
            properties,
            self.owner,
            &self.key,
            nonce,
            &self.signer,
            &mut self.rng,
            PlatformVersion::latest(),
        )
        .await
    }

    /// Bumps the revision and replaces `document` as it now stands.
    pub(super) async fn replace(
        &mut self,
        type_name: &str,
        document: &mut Document,
    ) -> StateTransitionsProcessingResult {
        document.increment_revision().expect("revision increments");
        let nonce = self.next_nonce();
        let result = replace_document(
            &self.platform,
            &self.platform_state,
            &self.contract,
            type_name,
            document,
            &self.key,
            nonce,
            &self.signer,
            PlatformVersion::latest(),
        )
        .await;
        // A refused replace leaves the stored revision where it was.
        if result.valid_count() == 0 {
            let revision = document.revision().expect("revision set");
            document.set_revision(Some(revision - 1));
        }
        result
    }

    pub(super) async fn delete(
        &mut self,
        type_name: &str,
        document: &Document,
    ) -> StateTransitionsProcessingResult {
        let nonce = self.next_nonce();
        delete_document(
            &self.platform,
            &self.platform_state,
            &self.contract,
            type_name,
            document,
            &self.key,
            nonce,
            &self.signer,
            PlatformVersion::latest(),
        )
        .await
    }
}
