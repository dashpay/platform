//! Contract moderation through the whole processing pipeline: the transition that edits a
//! contract's banlist, suspension list and warning list, and the document gate that enforces
//! the first two.

use crate::execution::check_tx::CheckTxLevel::FirstTimeCheck;
use crate::execution::validation::state_transition::tests::setup_identity;
use crate::platform_types::platform::PlatformRef;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
use crate::rpc::core::MockCoreRPCLike;
use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
use dpp::block::block_info::BlockInfo;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::ConsensusError;
use dpp::dash_to_credits;
use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
use dpp::data_contract::accessors::v1::DataContractV1Setters;
use dpp::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
use dpp::data_contract::config::moderation::{
    ContractBan, ContractDocumentRemoval, ContractDocumentRestoration, ContractModerationConfig,
    ContractModerationDocument, ContractModerationList, ContractModerationListStatus,
    ContractModerationListStatuses, ContractModerationReason, ContractModerationStatus,
    ContractModerators, ContractSuspension, ContractWarning, ElectedModerators, InterimModerators,
    ModerationAbility, DEFAULT_ELECTION_WINDOW_SECONDS,
};
use dpp::data_contract::config::v2::DataContractConfigGettersV2;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::random_document::{
    CreateRandomDocument, DocumentFieldFillSize, DocumentFieldFillType,
};
use dpp::data_contract::schema::DataContractSchemaMethodsV0;
use dpp::data_contract::DataContract;
use dpp::document::document_methods::DocumentMethodsV0;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::Document;
use dpp::document::{DocumentV0Getters, DocumentV0Setters};
use dpp::fee::Credits;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::{Identity, IdentityPublicKey, KeyID, TimestampMillis};
use dpp::platform_value::{platform_value, BinaryData, Bytes32, Identifier, Value};
use dpp::prelude::IdentityNonce;
use dpp::serialization::PlatformSerializable;
use dpp::state_transition::batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
use dpp::state_transition::batch_transition::BatchTransition;
use dpp::state_transition::contract_user_moderation_transition::methods::ContractUserModerationTransitionMethodsV0;
use dpp::state_transition::contract_user_moderation_transition::{
    ContractUserModerationAction, ContractUserModerationTransition,
};
use dpp::state_transition::data_contract_create_transition::methods::DataContractCreateTransitionMethodsV0;
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
use dpp::state_transition::data_contract_update_transition::methods::DataContractUpdateTransitionMethodsV0;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::state_transition::StateTransition;
use dpp::tests::fixtures::get_data_contract_fixture;
use dpp::tests::json_document::json_document_to_contract;
use dpp::util::hash::hash_double;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use drive::drive::contract::moderation::types::{
    ContractDocumentRemovalsQuery, ContractDocumentRemovalsSelection,
};
use drive::drive::document::query::QueryDocumentsOutcomeV0Methods;
use drive::drive::Drive;
use drive::grovedb::Transaction;
use drive::query::DriveDocumentQuery;
use drive::util::storage_flags::StorageFlags;
use rand::rngs::StdRng;
use rand::SeedableRng;
use simple_signer::signer::SimpleSigner;
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};

mod seated_team;

const DATA_CONTRACT_NOT_PRESENT: u32 = 10400;
const CONTRACT_MODERATION_SELF_TARGET: u32 = 10901;
const CONTRACT_MODERATION_REASON_TOO_LONG: u32 = 10903;
const INVALID_CONTRACT_MODERATION_REASON_DOCUMENTS: u32 = 10904;
const OVERFLOW: u32 = 10700;
const CONTRACT_MODERATION_NOT_ENABLED: u32 = 41100;
const IDENTITY_NOT_CONTRACT_MODERATOR: u32 = 41101;
const CONTRACT_MODERATION_TARGET_NOT_ALLOWED: u32 = 41102;
const CONTRACT_USER_ALREADY_BANNED: u32 = 41103;
const CONTRACT_USER_NOT_BANNED: u32 = 41104;
const CONTRACT_USER_NOT_SUSPENDED: u32 = 41105;
const CONTRACT_SUSPENSION_NOT_IN_FUTURE: u32 = 41106;
const CONTRACT_USER_BANNED: u32 = 41107;
const CONTRACT_USER_SUSPENDED: u32 = 41108;
const CONTRACT_MODERATION_TARGET_NOT_FOUND: u32 = 41109;
const CONTRACT_MODERATOR_IDENTITY_NOT_FOUND: u32 = 41110;
const CONTRACT_MODERATION_COUNTERPARTY_BARRED: u32 = 41114;
const DOCUMENT_TYPE_NOT_DELETABLE_BY_MODERATORS: u32 = 41115;
const DOCUMENT_MODERATION_WINDOW_ELAPSED: u32 = 41116;
const CONTRACT_USER_NOT_WARNED: u32 = 41117;
const CONTRACT_USER_WARNING_LIMIT_REACHED: u32 = 41118;
const CONTRACT_MODERATED_DOCUMENT_TYPE_NOT_YET_USABLE: u32 = 41200;
const CONTRACT_DOCUMENT_REMOVAL_NOT_FOUND: u32 = 41119;
const DOCUMENT_RESTORE_WINDOW_ELAPSED: u32 = 41120;
const DOCUMENT_RESTORE_HASH_MISMATCH: u32 = 41121;
const CONTRACT_DOCUMENT_ALREADY_RESTORED: u32 = 41122;
const DECODING_DOCUMENT: u32 = 10223;
const DUPLICATE_UNIQUE_INDEX: u32 = 40105;
const INVALID_DOCUMENT_TYPE: u32 = 10406;
const INVALID_CONTRACT_STRUCTURE: u32 = 10231;
const INVALID_CONTRACT_MODERATION_CONFIG: u32 = 10900;
const DOCUMENT_TYPE_UPDATE: u32 = 40212;
const REFERENCED_DOCUMENT_TYPE_DELETABLE: u32 = 40122;

pub(crate) const CRITICAL_KEY_ID: KeyID = 1;
const DOCUMENT_TYPE: &str = "niceDocument";
/// The document type `Setup::new_with_posts` adds: moderators may delete its documents.
const POST: &str = "post";
const BLOCK_TIME_MS: TimestampMillis = 1_000_000;

const BOTH: [ContractModerationList; 2] = [
    ContractModerationList::Banlist,
    ContractModerationList::Suspensions,
];

/// A registered identity with its signer and its CRITICAL key, plus the nonces the tests hand
/// out: gaps are allowed, reuse is not.
pub(crate) struct Actor {
    pub(crate) identity: Identity,
    pub(crate) signer: SimpleSigner,
    pub(crate) key: IdentityPublicKey,
    next_identity_nonce: Cell<IdentityNonce>,
    pub(crate) next_contract_nonce: Cell<IdentityNonce>,
}

impl Actor {
    pub(crate) fn new(platform: &mut TempPlatform<MockCoreRPCLike>, seed: u64) -> Self {
        let (identity, signer, key) = setup_identity(platform, seed, dash_to_credits!(10));
        Self {
            identity,
            signer,
            key,
            next_identity_nonce: Cell::new(1),
            next_contract_nonce: Cell::new(1),
        }
    }

    pub(crate) fn id(&self) -> Identifier {
        self.identity.id()
    }

    pub(crate) fn identity_nonce(&self) -> IdentityNonce {
        let nonce = self.next_identity_nonce.get();
        self.next_identity_nonce.set(nonce + 1);
        nonce
    }

    pub(crate) fn contract_nonce(&self) -> IdentityNonce {
        let nonce = self.next_contract_nonce.get();
        self.next_contract_nonce.set(nonce + 1);
        nonce
    }
}

/// A platform with an owner, a named moderator, a user and a stranger, and one contract the
/// owner created with the given moderation declaration.
struct Setup {
    platform: TempPlatform<MockCoreRPCLike>,
    owner: Actor,
    moderator: Actor,
    user: Actor,
    stranger: Actor,
    contract: DataContract,
    rng: RefCell<StdRng>,
}

/// Stands for `Setup::moderator` in a config handed to `Setup::new`, which is built before the
/// actors exist. `Setup` swaps it for the real identity: a named moderator must exist.
const THE_MODERATOR: Identifier = Identifier::new([7; 32]);
/// Stands for `Setup::owner` in the same way.
const THE_OWNER: Identifier = Identifier::new([8; 32]);

fn moderation(banlist: bool, suspensions: bool, moderator: Identifier) -> ContractModerationConfig {
    moderation_with_warnings(banlist, suspensions, false, moderator)
}

fn moderation_with_warnings(
    banlist: bool,
    suspensions: bool,
    warnings: bool,
    moderator: Identifier,
) -> ContractModerationConfig {
    ContractModerationConfig {
        banlist,
        suspensions,
        warnings,
        moderators: ContractModerators::AppointedModerators([moderator].into_iter().collect()),
    }
}

impl Setup {
    async fn new(moderation: Option<ContractModerationConfig>) -> Self {
        Self::new_at(moderation, PlatformVersion::latest()).await
    }

    async fn new_at(
        moderation: Option<ContractModerationConfig>,
        platform_version: &PlatformVersion,
    ) -> Self {
        Self::new_at_with(moderation, platform_version, |_| {}).await
    }

    /// A contract that also has the `post` document type, whose documents moderators may
    /// delete
    async fn new_with_posts(moderation: Option<ContractModerationConfig>) -> Self {
        Self::new_at_with(moderation, PlatformVersion::latest(), |contract| {
            add_document_type(contract, POST, post_schema(true))
        })
        .await
    }

    async fn new_at_with(
        moderation: Option<ContractModerationConfig>,
        platform_version: &PlatformVersion,
        modify_contract: impl FnOnce(&mut DataContract),
    ) -> Self {
        let mut platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(platform_version.protocol_version)
            .build_with_mock_rpc()
            .set_genesis_state();
        let owner = Actor::new(&mut platform, 11);
        let moderator = Actor::new(&mut platform, 12);
        let user = Actor::new(&mut platform, 13);
        let stranger = Actor::new(&mut platform, 14);

        let identity_nonce = owner.identity_nonce();
        let mut contract = get_data_contract_fixture(
            Some(owner.id()),
            identity_nonce,
            platform_version.protocol_version,
        )
        .data_contract_owned();
        let moderation = moderation.map(|mut moderation| {
            let named = match &mut moderation.moderators {
                ContractModerators::AppointedModerators(ids) => Some(ids),
                ContractModerators::Elected(elected) => match &mut elected.interim {
                    InterimModerators::AppointedModerators(ids) => Some(ids),
                    InterimModerators::ContractOwner
                    | InterimModerators::NotYetUsable
                    | InterimModerators::NoModeration => None,
                },
                ContractModerators::ContractOwner => None,
            };
            if let Some(ids) = named {
                if ids.remove(&THE_MODERATOR) {
                    ids.insert(moderator.id());
                }
                if ids.remove(&THE_OWNER) {
                    ids.insert(owner.id());
                }
            }
            moderation
        });
        contract.set_config(contract.config().clone().with_moderation(moderation));
        // After the config: a document type moderators can delete from needs the declaration.
        modify_contract(&mut contract);

        let mut setup = Self {
            platform,
            owner,
            moderator,
            user,
            stranger,
            contract,
            rng: RefCell::new(StdRng::seed_from_u64(99)),
        };

        let transaction = setup.platform.drive.grove.start_transaction();
        let create = setup
            .contract_create(identity_nonce, platform_version)
            .await;
        assert_success(&setup.process_at(&create, BLOCK_TIME_MS, &transaction));
        setup.commit(transaction);
        // The create sets the owner's nonce for the contract to the identity nonce it used.
        setup.owner.next_contract_nonce.set(identity_nonce + 1);
        setup
    }

    async fn contract_create(
        &self,
        identity_nonce: IdentityNonce,
        platform_version: &PlatformVersion,
    ) -> StateTransition {
        DataContractCreateTransition::new_from_data_contract(
            self.contract.clone(),
            identity_nonce,
            &self.owner.identity.clone().into_partial_identity_info(),
            CRITICAL_KEY_ID,
            &self.owner.signer,
            platform_version,
            None,
        )
        .await
        .expect("expected to build the contract create")
    }

    /// A contract update carrying `contract`, signed by the owner
    async fn contract_update(&self, contract: DataContract) -> StateTransition {
        DataContractUpdateTransition::new_from_data_contract(
            contract,
            &self.owner.identity.clone().into_partial_identity_info(),
            CRITICAL_KEY_ID,
            self.owner.contract_nonce(),
            0,
            &self.owner.signer,
            PlatformVersion::latest(),
            None,
        )
        .await
        .expect("expected to build the contract update")
    }

    /// A moderation transition by `actor`, signed with its CRITICAL key
    async fn moderate(
        &self,
        actor: &Actor,
        action: ContractUserModerationAction,
    ) -> StateTransition {
        ContractUserModerationTransition::try_from_identity_with_signer(
            &actor.identity,
            &CRITICAL_KEY_ID,
            self.contract.id(),
            action,
            actor.contract_nonce(),
            0,
            &actor.signer,
            PlatformVersion::latest(),
            None,
        )
        .await
        .expect("expected to build the moderation transition")
    }

    /// A document creation by `actor` on the contract
    async fn create_document(&self, actor: &Actor) -> StateTransition {
        self.create_document_keeping_it(actor).await.1
    }

    /// A document creation by `actor` on the contract, and the document it creates
    async fn create_document_keeping_it(&self, actor: &Actor) -> (Document, StateTransition) {
        self.create_document_of_type(actor, DOCUMENT_TYPE).await
    }

    /// A creation by `actor` of a document of `document_type_name`, and the document it creates
    async fn create_document_of_type(
        &self,
        actor: &Actor,
        document_type_name: &str,
    ) -> (Document, StateTransition) {
        self.create_document_of_type_with(actor, document_type_name, |_| {})
            .await
    }

    /// The same, with `modify` applied to the document before its creation is built
    async fn create_document_of_type_with(
        &self,
        actor: &Actor,
        document_type_name: &str,
        modify: impl FnOnce(&mut Document),
    ) -> (Document, StateTransition) {
        let platform_version = PlatformVersion::latest();
        let document_type = self
            .contract
            .document_type_for_name(document_type_name)
            .expect("expected the document type");
        let creation_nonce = actor.contract_nonce();
        // The borrow ends before the await below (clippy::await_holding_refcell_ref).
        let (entropy, document) = {
            let mut rng = self.rng.borrow_mut();
            let entropy = Bytes32::random_with_rng(&mut rng);
            let mut document = document_type
                .random_document_with_identifier_and_entropy(
                    &mut rng,
                    actor.id(),
                    entropy,
                    DocumentFieldFillType::FillIfNotRequired,
                    DocumentFieldFillSize::MinDocumentFillSize,
                    platform_version,
                )
                .expect("expected a random document");
            modify(&mut document);
            document
                .set_id_for_creation(document_type, &entropy.0, creation_nonce, platform_version)
                .expect("expected to set the document id");
            (entropy, document)
        };
        let transition = BatchTransition::new_document_creation_transition_from_document(
            document.clone(),
            document_type,
            entropy.0,
            &actor.key,
            creation_nonce,
            0,
            None,
            &actor.signer,
            platform_version,
            None,
        )
        .await
        .expect("expected to build the document creation");
        (document, transition)
    }

    /// The record of a moderator's deletion of `document_id`, if there is one
    fn post_removal(
        &self,
        document_id: Identifier,
        transaction: Option<&Transaction>,
    ) -> Option<ContractDocumentRemoval> {
        self.platform
            .drive
            .fetch_contract_document_removals(
                self.contract.id(),
                &ContractDocumentRemovalsQuery {
                    document_type_name: POST.to_string(),
                    selection: ContractDocumentRemovalsSelection::DocumentIds(vec![document_id]),
                },
                transaction,
                PlatformVersion::latest(),
            )
            .expect("expected to fetch the removal")
            .pop()
            .map(|entry| entry.removal)
    }

    /// The document of `document_type_name` stored at `document_id`, if there is one
    fn stored_document(
        &self,
        document_type_name: &str,
        document_id: Identifier,
        transaction: Option<&Transaction>,
    ) -> Option<Document> {
        let document_type = self
            .contract
            .document_type_for_name(document_type_name)
            .expect("expected the document type");
        let query = DriveDocumentQuery::new_primary_key_single_item_query(
            &self.contract,
            document_type,
            document_id,
        );
        self.platform
            .drive
            .query_documents(query, None, false, transaction, None)
            .expect("expected to query the document")
            .documents_owned()
            .pop()
    }

    /// `document` serialized under its type: what a restore carries, and what a removal
    /// record hashes
    fn document_bytes(&self, document_type_name: &str, document: &Document) -> Vec<u8> {
        let document_type = self
            .contract
            .document_type_for_name(document_type_name)
            .expect("expected the document type");
        document
            .serialize(document_type, &self.contract, PlatformVersion::latest())
            .expect("expected to serialize the document")
    }

    fn balance(&self, identity_id: Identifier, transaction: Option<&Transaction>) -> Credits {
        self.platform
            .drive
            .fetch_identity_balance(
                identity_id.to_buffer(),
                transaction,
                PlatformVersion::latest(),
            )
            .expect("expected to fetch the balance")
            .expect("expected the identity to have a balance")
    }

    /// Proves the committed state for a document deletion or restore and checks the proof
    /// shows its record. A restore's verifier reads the document's id out of the bytes under
    /// the contract's document type, so it is given the contract, as a client holding it is;
    /// a deletion's needs none.
    fn assert_removal_proved(&self, transition: &StateTransition) -> ContractDocumentRemoval {
        let platform_version = PlatformVersion::latest();
        let proof = self
            .platform
            .drive
            .prove_state_transition(transition, None, platform_version)
            .expect("expected to prove the state transition")
            .into_data()
            .expect("expected proof bytes");
        let known_contracts: BTreeMap<Identifier, DataContract> =
            BTreeMap::from([(self.contract.id(), self.contract.clone())]);
        let (_, outcome) = Drive::verify_state_transition_was_executed_with_proof(
            transition,
            &BlockInfo::default(),
            &proof,
            &|id| Ok(known_contracts.get(id).cloned().map(std::sync::Arc::new)),
            platform_version,
        )
        .expect("expected the proof to verify");
        assert!(
            !outcome.is_execution_proved(),
            "expected AffectedState, got {:?}",
            outcome
        );
        assert_eq!(
            outcome.owner_balance().is_some(),
            platform_version.drive.methods.prove.prove_state_transition >= 1,
            "the proof carries the moderator's balance exactly from prover version 1"
        );
        match outcome.into_result() {
            StateTransitionProofResult::VerifiedContractDocumentRemoval(
                contract_id,
                document_type_name,
                _,
                removal,
            ) => {
                assert_eq!(contract_id, self.contract.id());
                assert_eq!(document_type_name, POST);
                removal
            }
            other => panic!("expected a document removal, got {other:?}"),
        }
    }

    /// The deletion of `document` by `actor`, its owner
    async fn delete_document(&self, actor: &Actor, document: Document) -> StateTransition {
        let document_type = self
            .contract
            .document_type_for_name(DOCUMENT_TYPE)
            .expect("expected the document type");
        BatchTransition::new_document_deletion_transition_from_document(
            document,
            document_type,
            &actor.key,
            actor.contract_nonce(),
            0,
            None,
            &actor.signer,
            PlatformVersion::latest(),
            None,
        )
        .await
        .expect("expected to build the document deletion")
    }

    fn process_at(
        &self,
        transition: &StateTransition,
        time_ms: TimestampMillis,
        transaction: &Transaction,
    ) -> StateTransitionExecutionResult {
        let state = self.platform.state.load();
        let platform_version = state.current_platform_version().expect("version");
        let result = self
            .platform
            .platform
            .process_raw_state_transitions(
                &vec![transition
                    .serialize_to_bytes()
                    .expect("expected to serialize")],
                &state,
                &BlockInfo {
                    time_ms,
                    ..Default::default()
                },
                transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process the state transition");
        result.execution_results()[0].clone()
    }

    fn process(
        &self,
        transition: &StateTransition,
        transaction: &Transaction,
    ) -> StateTransitionExecutionResult {
        self.process_at(transition, BLOCK_TIME_MS, transaction)
    }

    fn commit(&self, transaction: Transaction) {
        self.platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");
    }

    fn check_tx(&self, transition: &StateTransition) -> Vec<ConsensusError> {
        let version = PlatformVersion::latest();
        let state = self.platform.state.load();
        let platform_ref = PlatformRef {
            drive: &self.platform.drive,
            state: &state,
            config: &self.platform.config,
            core_rpc: &self.platform.core_rpc,
        };
        let raw = transition
            .serialize_to_bytes()
            .expect("expected to serialize");
        self.platform
            .check_tx(&raw, FirstTimeCheck, &platform_ref, version)
            .expect("expected to check tx")
            .errors
    }

    fn status(
        &self,
        identity_id: Identifier,
        transaction: Option<&Transaction>,
    ) -> ContractModerationStatus {
        self.status_on(identity_id, &BOTH, transaction)
    }

    fn status_on(
        &self,
        identity_id: Identifier,
        lists: &[ContractModerationList],
        transaction: Option<&Transaction>,
    ) -> ContractModerationStatus {
        self.platform
            .drive
            .fetch_contract_moderation_status(
                self.contract.id(),
                identity_id,
                lists,
                transaction,
                PlatformVersion::latest(),
            )
            .expect("expected to fetch the status")
    }

    /// Proves the committed state for `transition` and checks the proof shows its outcome.
    fn assert_execution_proved(
        &self,
        transition: &StateTransition,
    ) -> ContractModerationListStatuses {
        let platform_version = PlatformVersion::latest();
        let proof = self
            .platform
            .drive
            .prove_state_transition(transition, None, platform_version)
            .expect("expected to prove the state transition")
            .into_data()
            .expect("expected proof bytes");
        // A ban's proof covers every list the contract keeps, which the verifier reads from
        // the contract, as a client holding the contract does.
        let known_contracts: BTreeMap<Identifier, DataContract> =
            BTreeMap::from([(self.contract.id(), self.contract.clone())]);
        let (_, outcome) = Drive::verify_state_transition_was_executed_with_proof(
            transition,
            &BlockInfo::default(),
            &proof,
            &|id| Ok(known_contracts.get(id).cloned().map(std::sync::Arc::new)),
            platform_version,
        )
        .expect("expected the proof to verify");
        assert!(
            !outcome.is_execution_proved(),
            "expected AffectedState, got {:?}",
            outcome
        );
        assert_eq!(
            outcome.owner_balance().is_some(),
            platform_version.drive.methods.prove.prove_state_transition >= 1,
            "the proof carries the moderator's balance exactly from prover version 1"
        );
        match outcome.into_result() {
            StateTransitionProofResult::VerifiedContractModerationListStatuses(_, _, status) => {
                status
            }
            other => panic!("expected a moderation status, got {other:?}"),
        }
    }
}

pub(crate) fn assert_success(execution: &StateTransitionExecutionResult) {
    assert!(
        matches!(
            execution,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        ),
        "expected a successful execution, got {execution:?}"
    );
}

pub(crate) fn assert_paid_with_code(execution: &StateTransitionExecutionResult, code: u32) {
    assert!(
        matches!(execution, StateTransitionExecutionResult::PaidConsensusError { error, .. } if error.code() == code),
        "expected a paid error {code}, got {execution:?}"
    );
}

pub(crate) fn assert_unpaid_with_code(execution: &StateTransitionExecutionResult, code: u32) {
    assert!(
        matches!(execution, StateTransitionExecutionResult::UnpaidConsensusError(error) if error.code() == code),
        "expected an unpaid error {code}, got {execution:?}"
    );
}

/// A moderation transition by `actor` on `contract_id`, signed with its CRITICAL key
async fn moderation_by(
    actor: &Actor,
    contract_id: Identifier,
    action: ContractUserModerationAction,
) -> StateTransition {
    ContractUserModerationTransition::try_from_identity_with_signer(
        &actor.identity,
        &CRITICAL_KEY_ID,
        contract_id,
        action,
        actor.contract_nonce(),
        0,
        &actor.signer,
        PlatformVersion::latest(),
        None,
    )
    .await
    .expect("expected to build the moderation transition")
}

/// The reason `ban_action` gives: a text, no code.
fn ban_reason() -> ContractModerationReason {
    ContractModerationReason::from_text("spam")
}

/// The reason `suspend_action` gives: a text and a code, which nothing checks.
fn suspension_reason() -> ContractModerationReason {
    ContractModerationReason {
        code: Some(7),
        text: "flooding".to_string(),
        documents: vec![],
        reason_document_id: None,
    }
}

/// The banlist entry `ban_action` leaves.
fn banned() -> Option<ContractBan> {
    Some(ContractBan {
        reason: ban_reason(),
    })
}

/// The suspension list entry `suspend_action` leaves.
fn suspended(until: TimestampMillis) -> Option<ContractSuspension> {
    Some(ContractSuspension {
        until,
        reason: suspension_reason(),
    })
}

fn ban_action(identity_id: Identifier) -> ContractUserModerationAction {
    ContractUserModerationAction::Ban {
        identity_id,
        reason: ban_reason(),
    }
}

fn unban_action(identity_id: Identifier) -> ContractUserModerationAction {
    ContractUserModerationAction::Unban { identity_id }
}

fn suspend_action(identity_id: Identifier, until: TimestampMillis) -> ContractUserModerationAction {
    ContractUserModerationAction::Suspend {
        identity_id,
        until,
        reason: suspension_reason(),
    }
}

fn unsuspend_action(identity_id: Identifier) -> ContractUserModerationAction {
    ContractUserModerationAction::Unsuspend { identity_id }
}

fn warn_action(identity_id: Identifier, text: &str) -> ContractUserModerationAction {
    ContractUserModerationAction::Warn {
        identity_id,
        reason: ContractModerationReason::from_text(text),
    }
}

fn clear_warnings_action(identity_id: Identifier) -> ContractUserModerationAction {
    ContractUserModerationAction::ClearWarnings { identity_id }
}

/// The warning a `warn_action` for `text` leaves in a block at `time_ms`.
fn warning(time_ms: TimestampMillis, text: &str) -> ContractWarning {
    ContractWarning {
        warned_at: time_ms,
        reason: ContractModerationReason::from_text(text),
    }
}

/// The status of an identity that carries `warnings` and nothing else.
fn warned_with(warnings: Vec<ContractWarning>) -> ContractModerationStatus {
    ContractModerationStatus {
        ban: None,
        suspension: None,
        warnings,
    }
}

#[tokio::test]
async fn should_ban_a_user_refuse_its_documents_and_let_them_through_again_after_an_unban() {
    let setup = Setup::new(Some(moderation(true, true, THE_MODERATOR))).await;
    let user_id = setup.user.id();

    let transaction = setup.platform.drive.grove.start_transaction();
    let before = setup.create_document(&setup.user).await;
    assert_success(&setup.process(&before, &transaction));

    let ban = setup.moderate(&setup.owner, ban_action(user_id)).await;
    assert_success(&setup.process(&ban, &transaction));
    setup.commit(transaction);
    // A ban proves every list the contract keeps: banned, and no suspension left behind.
    assert_eq!(
        setup.assert_execution_proved(&ban),
        ContractModerationListStatuses(vec![
            ContractModerationListStatus::Banlist { ban: banned() },
            ContractModerationListStatus::Suspensions { suspension: None },
        ])
    );

    // The mempool refuses the banned user's documents, paid, before a block does.
    let refused = setup.create_document(&setup.user).await;
    let mempool_errors = setup.check_tx(&refused);
    assert_eq!(mempool_errors.len(), 1, "{mempool_errors:?}");
    assert_eq!(mempool_errors[0].code(), CONTRACT_USER_BANNED);
    let transaction = setup.platform.drive.grove.start_transaction();
    assert_paid_with_code(&setup.process(&refused, &transaction), CONTRACT_USER_BANNED);

    let unban = setup.moderate(&setup.owner, unban_action(user_id)).await;
    assert_success(&setup.process(&unban, &transaction));
    assert_eq!(
        setup.status(user_id, Some(&transaction)),
        ContractModerationStatus::default()
    );

    let after = setup.create_document(&setup.user).await;
    assert_success(&setup.process(&after, &transaction));
    setup.commit(transaction);
    assert_eq!(
        setup.assert_execution_proved(&unban),
        ContractModerationListStatuses(vec![ContractModerationListStatus::Banlist { ban: None }])
    );
}

#[tokio::test]
async fn should_suspend_until_a_block_time_and_sweep_the_suspension_once_it_lapsed() {
    let setup = Setup::new(Some(moderation(true, true, THE_MODERATOR))).await;
    let user_id = setup.user.id();
    let until = BLOCK_TIME_MS + 10_000;

    let transaction = setup.platform.drive.grove.start_transaction();
    let suspend = setup
        .moderate(&setup.owner, suspend_action(user_id, until))
        .await;
    assert_success(&setup.process(&suspend, &transaction));
    setup.commit(transaction);
    assert_eq!(
        setup.assert_execution_proved(&suspend),
        ContractModerationListStatuses(vec![ContractModerationListStatus::Suspensions {
            suspension: suspended(until)
        }])
    );

    let refused = setup.create_document(&setup.user).await;
    assert_eq!(setup.check_tx(&refused)[0].code(), CONTRACT_USER_SUSPENDED);
    let transaction = setup.platform.drive.grove.start_transaction();
    assert_paid_with_code(
        &setup.process_at(&refused, until - 1, &transaction),
        CONTRACT_USER_SUSPENDED,
    );

    // At `until` the suspension has lapsed: the document goes through and sweeps the entry.
    let allowed = setup.create_document(&setup.user).await;
    assert_success(&setup.process_at(&allowed, until, &transaction));
    assert_eq!(
        setup.status(user_id, Some(&transaction)),
        ContractModerationStatus::default()
    );
    setup.commit(transaction);

    // Nothing is left to unsuspend.
    let transaction = setup.platform.drive.grove.start_transaction();
    let unsuspend = setup
        .moderate(&setup.owner, unsuspend_action(user_id))
        .await;
    assert_paid_with_code(
        &setup.process(&unsuspend, &transaction),
        CONTRACT_USER_NOT_SUSPENDED,
    );
}

#[tokio::test]
async fn should_warn_a_user_without_barring_it_accumulate_the_warnings_and_clear_them() {
    let setup = Setup::new(Some(moderation_with_warnings(
        true,
        false,
        true,
        THE_MODERATOR,
    )))
    .await;
    let user_id = setup.user.id();
    let warnings = [ContractModerationList::Warnings];

    let transaction = setup.platform.drive.grove.start_transaction();
    let first = setup
        .moderate(&setup.owner, warn_action(user_id, "first strike"))
        .await;
    assert_success(&setup.process(&first, &transaction));
    // A warned user carries on: its documents pass, in the mempool and in a block.
    let document = setup.create_document(&setup.user).await;
    assert!(setup.check_tx(&document).is_empty());
    assert_success(&setup.process(&document, &transaction));

    // A second warning, by the named moderator, in a later block: the entry grows.
    let second = setup
        .moderate(&setup.moderator, warn_action(user_id, "second strike"))
        .await;
    assert_success(&setup.process_at(&second, BLOCK_TIME_MS + 5_000, &transaction));
    setup.commit(transaction);
    assert_eq!(
        setup.status_on(user_id, &warnings, None),
        warned_with(vec![
            warning(BLOCK_TIME_MS, "first strike"),
            warning(BLOCK_TIME_MS + 5_000, "second strike"),
        ])
    );
    // The banlist says nothing: warnings bar nothing.
    assert_eq!(
        setup.status(user_id, None),
        ContractModerationStatus::default()
    );
    // The execution proof shows the warning list alone, with the latest warning last.
    assert_eq!(
        setup.assert_execution_proved(&second),
        ContractModerationListStatuses(vec![ContractModerationListStatus::Warnings {
            warnings: vec![
                warning(BLOCK_TIME_MS, "first strike"),
                warning(BLOCK_TIME_MS + 5_000, "second strike"),
            ],
        }])
    );

    let transaction = setup.platform.drive.grove.start_transaction();
    let clear = setup
        .moderate(&setup.owner, clear_warnings_action(user_id))
        .await;
    assert_success(&setup.process(&clear, &transaction));
    assert_eq!(
        setup.status_on(user_id, &warnings, Some(&transaction)),
        ContractModerationStatus::default()
    );
    // Nothing is left to clear.
    let clear_again = setup
        .moderate(&setup.owner, clear_warnings_action(user_id))
        .await;
    assert_paid_with_code(
        &setup.process(&clear_again, &transaction),
        CONTRACT_USER_NOT_WARNED,
    );
    setup.commit(transaction);
    assert_eq!(
        setup.assert_execution_proved(&clear),
        ContractModerationListStatuses(vec![ContractModerationListStatus::Warnings {
            warnings: vec![],
        }])
    );
}

#[tokio::test]
async fn should_refuse_a_warning_past_the_limit_until_the_warnings_are_cleared() {
    let setup = Setup::new(Some(moderation_with_warnings(
        false,
        false,
        true,
        THE_MODERATOR,
    )))
    .await;
    let user_id = setup.user.id();
    let max_warnings = PlatformVersion::latest()
        .system_limits
        .max_contract_warnings_per_identity;

    let transaction = setup.platform.drive.grove.start_transaction();
    for index in 0..max_warnings {
        let warn = setup
            .moderate(
                &setup.owner,
                warn_action(user_id, &format!("strike {index}")),
            )
            .await;
        assert_success(&setup.process(&warn, &transaction));
    }
    setup.commit(transaction);
    assert_eq!(
        setup
            .status_on(user_id, &[ContractModerationList::Warnings], None)
            .warnings
            .len(),
        usize::from(max_warnings)
    );
    // The mempool refuses the one too many before a block does.
    let one_too_many = setup
        .moderate(&setup.owner, warn_action(user_id, "one too many"))
        .await;
    let mempool_errors = setup.check_tx(&one_too_many);
    assert_eq!(mempool_errors.len(), 1, "{mempool_errors:?}");
    assert_eq!(
        mempool_errors[0].code(),
        CONTRACT_USER_WARNING_LIMIT_REACHED
    );
    let transaction = setup.platform.drive.grove.start_transaction();
    assert_paid_with_code(
        &setup.process(&one_too_many, &transaction),
        CONTRACT_USER_WARNING_LIMIT_REACHED,
    );

    let clear = setup
        .moderate(&setup.owner, clear_warnings_action(user_id))
        .await;
    assert_success(&setup.process(&clear, &transaction));
    let again = setup
        .moderate(&setup.owner, warn_action(user_id, "a fresh start"))
        .await;
    assert_success(&setup.process(&again, &transaction));
}

#[tokio::test]
async fn should_keep_the_warning_list_out_of_bans_and_the_gate() {
    let setup = Setup::new(Some(moderation_with_warnings(
        true,
        true,
        true,
        THE_MODERATOR,
    )))
    .await;
    let user_id = setup.user.id();
    let every_list = [
        ContractModerationList::Banlist,
        ContractModerationList::Suspensions,
        ContractModerationList::Warnings,
    ];

    let transaction = setup.platform.drive.grove.start_transaction();
    let warn = setup
        .moderate(&setup.owner, warn_action(user_id, "first strike"))
        .await;
    assert_success(&setup.process(&warn, &transaction));
    // A ban of a warned user leaves the warnings, which say how it came to that.
    let ban = setup.moderate(&setup.owner, ban_action(user_id)).await;
    assert_success(&setup.process(&ban, &transaction));
    assert_eq!(
        setup.status_on(user_id, &every_list, Some(&transaction)),
        ContractModerationStatus {
            ban: banned(),
            suspension: None,
            warnings: vec![warning(BLOCK_TIME_MS, "first strike")],
        }
    );
    // A banned user may still be warned: the warning outlives the ban.
    let warn_banned = setup
        .moderate(&setup.owner, warn_action(user_id, "and again"))
        .await;
    assert_success(&setup.process(&warn_banned, &transaction));
    setup.commit(transaction);

    // The ban's proof covers the barring lists and leaves the warning list unknown.
    let proved = setup.assert_execution_proved(&ban);
    assert_eq!(proved.banned(), Some(true));
    assert_eq!(proved.suspended_until(), Some(None));
    assert_eq!(proved.warnings(), None);

    // Unbanned, the user carries on with its two warnings on record.
    let transaction = setup.platform.drive.grove.start_transaction();
    let unban = setup.moderate(&setup.owner, unban_action(user_id)).await;
    assert_success(&setup.process(&unban, &transaction));
    let document = setup.create_document(&setup.user).await;
    assert_success(&setup.process(&document, &transaction));
    assert_eq!(
        setup
            .status_on(
                user_id,
                &[ContractModerationList::Warnings],
                Some(&transaction)
            )
            .warnings
            .len(),
        2
    );
}

#[tokio::test]
async fn should_store_the_documents_a_reason_cites_without_looking_them_up() {
    let setup = Setup::new_with_posts(Some(moderation_with_warnings(
        true,
        false,
        true,
        THE_MODERATOR,
    )))
    .await;
    let user_id = setup.user.id();
    let max_documents = PlatformVersion::latest()
        .system_limits
        .max_contract_moderation_reason_documents;
    let cite = |seeds: &[u8]| {
        seeds
            .iter()
            .map(|seed| ContractModerationDocument {
                document_type_name: POST.to_string(),
                document_id: Identifier::from([*seed; 32]),
            })
            .collect::<Vec<_>>()
    };

    let transaction = setup.platform.drive.grove.start_transaction();
    // The user's real post, and one that never existed: neither is looked up.
    let (post, create) = setup.create_document_of_type(&setup.user, POST).await;
    assert_success(&setup.process(&create, &transaction));
    let mut documents = cite(&[0xEE]);
    documents.push(ContractModerationDocument {
        document_type_name: POST.to_string(),
        document_id: post.id(),
    });
    let reason = ContractModerationReason::from_text("these posts").with_documents(documents);
    let warn = setup
        .moderate(
            &setup.owner,
            ContractUserModerationAction::Warn {
                identity_id: user_id,
                reason: reason.clone(),
            },
        )
        .await;
    assert_success(&setup.process(&warn, &transaction));
    assert_eq!(
        setup
            .status_on(
                user_id,
                &[ContractModerationList::Warnings],
                Some(&transaction)
            )
            .warnings[0]
            .reason,
        reason
    );
    // A ban cites documents the same way, and its proof carries them.
    let ban = setup
        .moderate(
            &setup.owner,
            ContractUserModerationAction::Ban {
                identity_id: user_id,
                reason: reason.clone(),
            },
        )
        .await;
    assert_success(&setup.process(&ban, &transaction));
    setup.commit(transaction);
    assert_eq!(
        setup
            .assert_execution_proved(&ban)
            .ban()
            .flatten()
            .map(|ban| &ban.reason),
        Some(&reason)
    );

    // Too many, or one twice: refused unpaid, in the mempool as in a block.
    let transaction = setup.platform.drive.grove.start_transaction();
    for documents in [
        cite(&(1..=max_documents as u8 + 1).collect::<Vec<u8>>()),
        cite(&[1, 1]),
    ] {
        let refused = setup
            .moderate(
                &setup.owner,
                ContractUserModerationAction::Warn {
                    identity_id: setup.stranger.id(),
                    reason: ContractModerationReason::from_text("spam").with_documents(documents),
                },
            )
            .await;
        assert_eq!(
            setup.check_tx(&refused)[0].code(),
            INVALID_CONTRACT_MODERATION_REASON_DOCUMENTS
        );
        assert_unpaid_with_code(
            &setup.process(&refused, &transaction),
            INVALID_CONTRACT_MODERATION_REASON_DOCUMENTS,
        );
    }
}

#[tokio::test]
async fn should_refuse_a_warning_that_breaks_a_rule() {
    let setup = Setup::new(Some(moderation_with_warnings(
        true,
        false,
        true,
        THE_MODERATOR,
    )))
    .await;
    let user_id = setup.user.id();
    let max_length = PlatformVersion::latest()
        .system_limits
        .max_contract_moderation_reason_length as usize;
    let transaction = setup.platform.drive.grove.start_transaction();

    // Only a moderator warns, and neither the owner nor a moderator can be warned.
    let by_stranger = setup
        .moderate(&setup.stranger, warn_action(user_id, "spam"))
        .await;
    assert_paid_with_code(
        &setup.process(&by_stranger, &transaction),
        IDENTITY_NOT_CONTRACT_MODERATOR,
    );
    let owner_warns_moderator = setup
        .moderate(&setup.owner, warn_action(setup.moderator.id(), "spam"))
        .await;
    assert_paid_with_code(
        &setup.process(&owner_warns_moderator, &transaction),
        CONTRACT_MODERATION_TARGET_NOT_ALLOWED,
    );
    let self_target = setup
        .moderate(&setup.owner, warn_action(setup.owner.id(), "spam"))
        .await;
    assert_unpaid_with_code(
        &setup.process(&self_target, &transaction),
        CONTRACT_MODERATION_SELF_TARGET,
    );
    let unknown = setup
        .moderate(
            &setup.owner,
            warn_action(Identifier::from([0xEE; 32]), "spam"),
        )
        .await;
    assert_paid_with_code(
        &setup.process(&unknown, &transaction),
        CONTRACT_MODERATION_TARGET_NOT_FOUND,
    );
    // The reason of a warning is bounded like a ban's, and refused unpaid.
    let too_long = setup
        .moderate(
            &setup.owner,
            warn_action(user_id, &"x".repeat(max_length + 1)),
        )
        .await;
    assert_unpaid_with_code(
        &setup.process(&too_long, &transaction),
        CONTRACT_MODERATION_REASON_TOO_LONG,
    );
    assert_eq!(
        setup.check_tx(&too_long)[0].code(),
        CONTRACT_MODERATION_REASON_TOO_LONG
    );

    // A contract without a warning list refuses a warning and a clearing alike.
    let without = Setup::new(Some(moderation(true, true, THE_MODERATOR))).await;
    let transaction = without.platform.drive.grove.start_transaction();
    let warn = without
        .moderate(&without.owner, warn_action(without.user.id(), "spam"))
        .await;
    assert_paid_with_code(
        &without.process(&warn, &transaction),
        CONTRACT_MODERATION_NOT_ENABLED,
    );
    let clear = without
        .moderate(&without.owner, clear_warnings_action(without.user.id()))
        .await;
    assert_paid_with_code(
        &without.process(&clear, &transaction),
        CONTRACT_MODERATION_NOT_ENABLED,
    );
}

#[tokio::test]
async fn should_let_a_named_moderator_moderate_and_refuse_everyone_else() {
    let mut setup = Setup::new(None).await;
    let moderator_id = setup.moderator.id();
    setup = Setup::new(Some(moderation(true, true, moderator_id))).await;
    let user_id = setup.user.id();

    let transaction = setup.platform.drive.grove.start_transaction();
    let by_stranger = setup.moderate(&setup.stranger, ban_action(user_id)).await;
    assert_paid_with_code(
        &setup.process(&by_stranger, &transaction),
        IDENTITY_NOT_CONTRACT_MODERATOR,
    );

    let by_moderator = setup.moderate(&setup.moderator, ban_action(user_id)).await;
    assert_success(&setup.process(&by_moderator, &transaction));
    assert!(setup.status(user_id, Some(&transaction)).banned());

    let owner_bans_moderator = setup.moderate(&setup.owner, ban_action(moderator_id)).await;
    assert_paid_with_code(
        &setup.process(&owner_bans_moderator, &transaction),
        CONTRACT_MODERATION_TARGET_NOT_ALLOWED,
    );
    let moderator_bans_owner = setup
        .moderate(&setup.moderator, ban_action(setup.owner.id()))
        .await;
    assert_paid_with_code(
        &setup.process(&moderator_bans_owner, &transaction),
        CONTRACT_MODERATION_TARGET_NOT_ALLOWED,
    );

    let self_target = setup
        .moderate(&setup.owner, ban_action(setup.owner.id()))
        .await;
    assert_unpaid_with_code(
        &setup.process(&self_target, &transaction),
        CONTRACT_MODERATION_SELF_TARGET,
    );
    assert_eq!(
        setup.check_tx(&self_target)[0].code(),
        CONTRACT_MODERATION_SELF_TARGET
    );
}

#[tokio::test]
async fn should_refuse_a_list_the_contract_does_not_keep_and_an_unmoderated_contract() {
    let setup = Setup::new(Some(moderation(true, false, THE_MODERATOR))).await;
    let user_id = setup.user.id();
    let transaction = setup.platform.drive.grove.start_transaction();
    let suspend = setup
        .moderate(&setup.owner, suspend_action(user_id, BLOCK_TIME_MS + 1))
        .await;
    assert_paid_with_code(
        &setup.process(&suspend, &transaction),
        CONTRACT_MODERATION_NOT_ENABLED,
    );
    let ban = setup.moderate(&setup.owner, ban_action(user_id)).await;
    assert_success(&setup.process(&ban, &transaction));

    let setup = Setup::new(None).await;
    let transaction = setup.platform.drive.grove.start_transaction();
    let ban = setup
        .moderate(&setup.owner, ban_action(setup.user.id()))
        .await;
    assert_paid_with_code(
        &setup.process(&ban, &transaction),
        CONTRACT_MODERATION_NOT_ENABLED,
    );
    assert_eq!(
        setup.check_tx(&ban)[0].code(),
        CONTRACT_MODERATION_NOT_ENABLED
    );
}

#[tokio::test]
async fn should_refuse_actions_that_do_not_fit_the_targets_status() {
    let setup = Setup::new(Some(moderation(true, true, THE_MODERATOR))).await;
    let user_id = setup.user.id();
    let transaction = setup.platform.drive.grove.start_transaction();

    let unban = setup.moderate(&setup.owner, unban_action(user_id)).await;
    assert_paid_with_code(
        &setup.process(&unban, &transaction),
        CONTRACT_USER_NOT_BANNED,
    );

    let past = setup
        .moderate(&setup.owner, suspend_action(user_id, BLOCK_TIME_MS))
        .await;
    assert_paid_with_code(
        &setup.process(&past, &transaction),
        CONTRACT_SUSPENSION_NOT_IN_FUTURE,
    );

    let suspend_first = setup
        .moderate(&setup.owner, suspend_action(user_id, BLOCK_TIME_MS + 5))
        .await;
    assert_success(&setup.process(&suspend_first, &transaction));
    let suspend_again = setup
        .moderate(&setup.owner, suspend_action(user_id, BLOCK_TIME_MS + 50))
        .await;
    assert_success(&setup.process(&suspend_again, &transaction));
    assert_eq!(
        setup.status(user_id, Some(&transaction)).suspended_until(),
        Some(BLOCK_TIME_MS + 50)
    );

    // A ban supersedes the suspension.
    let ban = setup.moderate(&setup.owner, ban_action(user_id)).await;
    assert_success(&setup.process(&ban, &transaction));
    assert_eq!(
        setup.status(user_id, Some(&transaction)),
        ContractModerationStatus {
            ban: banned(),
            suspension: None,
            warnings: vec![],
        }
    );
    let ban_again = setup.moderate(&setup.owner, ban_action(user_id)).await;
    assert_paid_with_code(
        &setup.process(&ban_again, &transaction),
        CONTRACT_USER_ALREADY_BANNED,
    );
    let suspend_banned = setup
        .moderate(&setup.owner, suspend_action(user_id, BLOCK_TIME_MS + 5))
        .await;
    // Not a duplicate ban: a suspend blocked by a ban gets the "is banned" code.
    assert_paid_with_code(
        &setup.process(&suspend_banned, &transaction),
        CONTRACT_USER_BANNED,
    );

    let unknown = setup
        .moderate(&setup.owner, ban_action(Identifier::from([0xEE; 32])))
        .await;
    assert_paid_with_code(
        &setup.process(&unknown, &transaction),
        CONTRACT_MODERATION_TARGET_NOT_FOUND,
    );
}

#[tokio::test]
async fn should_fix_the_lists_a_contract_keeps_when_it_is_created() {
    let config_update_refused = |execution: StateTransitionExecutionResult, what: &str| {
        assert!(
            matches!(
                &execution,
                StateTransitionExecutionResult::PaidConsensusError {
                    error: ConsensusError::StateError(StateError::DataContractConfigUpdateError(_)),
                    ..
                }
            ),
            "expected {what} to be refused, got {execution:?}"
        );
    };

    // An unmoderated contract stays unmoderated: whoever wrote documents under it was never
    // told they could be barred from it.
    let setup = Setup::new(None).await;
    let transaction = setup.platform.drive.grove.start_transaction();
    let mut moderated = setup.contract.clone();
    moderated.set_version(2);
    moderated.set_config(moderated.config().clone().with_moderation(Some(moderation(
        true,
        true,
        setup.moderator.id(),
    ))));
    let update = setup.contract_update(moderated).await;
    config_update_refused(
        setup.process(&update, &transaction),
        "moderation enabled by an update",
    );
    // Nothing was switched on: the user's documents still go through.
    let allowed = setup.create_document(&setup.user).await;
    assert_success(&setup.process(&allowed, &transaction));
    drop(transaction);

    // A contract that keeps one list can neither add the other nor drop the one it has.
    let setup = Setup::new(Some(moderation(true, false, THE_MODERATOR))).await;
    let transaction = setup.platform.drive.grove.start_transaction();
    for (banlist, suspensions, what) in [
        (true, true, "a second list turned on"),
        (false, true, "the banlist swapped for a suspension list"),
    ] {
        let mut changed = setup.contract.clone();
        changed.set_version(2);
        changed.set_config(changed.config().clone().with_moderation(Some(moderation(
            banlist,
            suspensions,
            setup.moderator.id(),
        ))));
        let update = setup.contract_update(changed).await;
        config_update_refused(setup.process(&update, &transaction), what);
    }
    // The warning list is fixed at creation like the others.
    let mut with_warnings = setup.contract.clone();
    with_warnings.set_version(2);
    with_warnings.set_config(with_warnings.config().clone().with_moderation(Some(
        moderation_with_warnings(true, false, true, setup.moderator.id()),
    )));
    let update = setup.contract_update(with_warnings).await;
    config_update_refused(
        setup.process(&update, &transaction),
        "a warning list turned on",
    );
}

#[tokio::test]
async fn should_not_be_active_before_protocol_version_14() {
    let platform_version = PlatformVersion::get(13).expect("protocol version 13");
    let setup = Setup::new_at(None, platform_version).await;
    let transaction = setup.platform.drive.grove.start_transaction();
    let ban = setup
        .moderate(&setup.owner, ban_action(setup.user.id()))
        .await;
    let execution = setup.process(&ban, &transaction);
    // The transition type does not exist before protocol version 14, so the node cannot even
    // decode it as active: the same refusal every transition added since the fork gets.
    assert!(
        matches!(
            &execution,
            StateTransitionExecutionResult::InternalError(message)
                if message.contains("ContractUserModeration") && message.contains("not active")
        ),
        "expected the transition to be inactive before protocol version 14, got {execution:?}"
    );
}

#[tokio::test]
async fn should_not_admit_a_moderated_contract_before_protocol_version_14() {
    let platform_version = PlatformVersion::get(13).expect("protocol version 13");
    let mut setup = Setup::new_at(None, platform_version).await;
    let transaction = setup.platform.drive.grove.start_transaction();
    let declaration = moderation(true, true, setup.moderator.id());

    // A version 2 config is bytes a binary from before protocol version 14 cannot decode, so
    // neither a create nor an update carrying one may be admitted there.
    let assert_inactive = |execution: StateTransitionExecutionResult, name: &str| {
        assert!(
            matches!(
                &execution,
                StateTransitionExecutionResult::InternalError(message)
                    if message.contains(name) && message.contains("not active")
            ),
            "expected {name} to be inactive before protocol version 14, got {execution:?}"
        );
    };

    let mut moderated = setup.contract.clone();
    moderated.set_version(2);
    moderated.set_config(
        moderated
            .config()
            .clone()
            .with_moderation(Some(declaration)),
    );
    let update = setup.contract_update(moderated.clone()).await;
    assert_inactive(setup.process(&update, &transaction), "DataContractUpdate");

    setup.contract = moderated;
    let create = setup
        .contract_create(setup.owner.identity_nonce(), PlatformVersion::latest())
        .await;
    assert_inactive(setup.process(&create, &transaction), "DataContractCreate");

    // Nothing was stored: the contract on disk still declares no moderation.
    let stored = setup
        .platform
        .drive
        .fetch_contract(
            setup.contract.id().to_buffer(),
            None,
            None,
            Some(&transaction),
            platform_version,
        )
        .unwrap()
        .expect("expected to fetch the contract")
        .expect("expected the contract to exist");
    assert!(stored.contract.config().moderation().is_none());
}

#[tokio::test]
async fn should_prove_only_the_edited_list_and_leave_the_other_unknown() {
    let setup = Setup::new(Some(moderation(true, true, THE_MODERATOR))).await;
    let user_id = setup.user.id();

    let transaction = setup.platform.drive.grove.start_transaction();
    let suspend = setup
        .moderate(
            &setup.owner,
            suspend_action(user_id, BLOCK_TIME_MS + 10_000),
        )
        .await;
    assert_success(&setup.process(&suspend, &transaction));
    let unsuspend = setup
        .moderate(&setup.owner, unsuspend_action(user_id))
        .await;
    assert_success(&setup.process(&unsuspend, &transaction));
    let ban = setup.moderate(&setup.owner, ban_action(user_id)).await;
    assert_success(&setup.process(&ban, &transaction));
    setup.commit(transaction);

    // The proof of the unsuspend holds the suspension entry alone. It must not read as "not
    // banned": the identity is banned in the very state the proof was made from.
    assert_eq!(
        setup.assert_execution_proved(&unsuspend),
        ContractModerationListStatuses(vec![ContractModerationListStatus::Suspensions {
            suspension: None
        }])
    );
    assert!(setup.status(user_id, None).banned());
}

#[tokio::test]
async fn should_let_an_entry_be_lifted_from_an_identity_an_update_made_moderator() {
    let mut setup = Setup::new(Some(moderation(true, true, THE_MODERATOR))).await;
    let user_id = setup.user.id();

    let transaction = setup.platform.drive.grove.start_transaction();
    let ban = setup.moderate(&setup.owner, ban_action(user_id)).await;
    assert_success(&setup.process(&ban, &transaction));

    // The banned user becomes a moderator: its ban stays, and still binds it.
    let mut promoted = setup.contract.clone();
    promoted.set_version(2);
    promoted.set_config(
        promoted
            .config()
            .clone()
            .with_moderation(Some(moderation(true, true, user_id))),
    );
    let update = setup.contract_update(promoted.clone()).await;
    assert_success(&setup.process(&update, &transaction));
    setup.contract = promoted;
    let refused = setup.create_document(&setup.user).await;
    assert_paid_with_code(&setup.process(&refused, &transaction), CONTRACT_USER_BANNED);

    // A moderator still cannot be put on a list, but the entry it already carries can be
    // lifted without demoting it first.
    let suspend = setup
        .moderate(
            &setup.owner,
            suspend_action(user_id, BLOCK_TIME_MS + 10_000),
        )
        .await;
    assert_paid_with_code(
        &setup.process(&suspend, &transaction),
        CONTRACT_MODERATION_TARGET_NOT_ALLOWED,
    );
    let unban = setup.moderate(&setup.owner, unban_action(user_id)).await;
    assert_success(&setup.process(&unban, &transaction));
    let allowed = setup.create_document(&setup.user).await;
    assert_success(&setup.process(&allowed, &transaction));
}

#[tokio::test]
async fn should_refuse_a_contract_naming_a_moderator_that_does_not_exist() {
    let mut setup = Setup::new(None).await;
    let transaction = setup.platform.drive.grove.start_transaction();
    let unknown = Identifier::from([0x77; 32]);

    // A second contract of the owner, naming an identity nobody created.
    setup.contract.set_config(
        setup
            .contract
            .config()
            .clone()
            .with_moderation(Some(moderation(true, true, unknown))),
    );
    let create = setup
        .contract_create(setup.owner.identity_nonce(), PlatformVersion::latest())
        .await;
    assert_paid_with_code(
        &setup.process(&create, &transaction),
        CONTRACT_MODERATOR_IDENTITY_NOT_FOUND,
    );
}

#[tokio::test]
async fn should_refuse_an_update_adding_a_moderator_that_does_not_exist() {
    let setup = Setup::new(Some(moderation(true, true, THE_MODERATOR))).await;
    let transaction = setup.platform.drive.grove.start_transaction();
    let unknown = Identifier::from([0x77; 32]);

    let mut widened = setup.contract.clone();
    widened.set_version(2);
    widened.set_config(
        widened
            .config()
            .clone()
            .with_moderation(Some(ContractModerationConfig {
                banlist: true,
                suspensions: true,
                moderators: ContractModerators::AppointedModerators(
                    [setup.moderator.id(), unknown].into_iter().collect(),
                ),
                warnings: false,
            })),
    );
    let update = setup.contract_update(widened).await;
    assert_paid_with_code(
        &setup.process(&update, &transaction),
        CONTRACT_MODERATOR_IDENTITY_NOT_FOUND,
    );
}

#[tokio::test]
async fn should_accept_an_update_that_keeps_the_existing_moderators() {
    let setup = Setup::new(Some(moderation(true, true, THE_MODERATOR))).await;
    let transaction = setup.platform.drive.grove.start_transaction();

    // Same moderators, plus the user: only the user is new.
    let mut widened = setup.contract.clone();
    widened.set_version(2);
    widened.set_config(
        widened
            .config()
            .clone()
            .with_moderation(Some(ContractModerationConfig {
                banlist: true,
                suspensions: true,
                moderators: ContractModerators::AppointedModerators(
                    [setup.moderator.id(), setup.user.id()]
                        .into_iter()
                        .collect(),
                ),
                warnings: false,
            })),
    );
    let update = setup.contract_update(widened).await;
    assert_success(&setup.process(&update, &transaction));

    // And an update that leaves the moderators as they are.
    let mut unchanged = setup.contract.clone();
    unchanged.set_version(3);
    unchanged.set_config(
        unchanged
            .config()
            .clone()
            .with_moderation(Some(ContractModerationConfig {
                banlist: true,
                suspensions: true,
                moderators: ContractModerators::AppointedModerators(
                    [setup.moderator.id(), setup.user.id()]
                        .into_iter()
                        .collect(),
                ),
                warnings: false,
            })),
    );
    let update = setup.contract_update(unchanged).await;
    assert_success(&setup.process(&update, &transaction));
}

#[tokio::test]
async fn should_accept_the_owner_named_among_the_moderators() {
    let setup = Setup::new(Some(ContractModerationConfig {
        banlist: true,
        suspensions: true,
        moderators: ContractModerators::AppointedModerators(
            [THE_OWNER, THE_MODERATOR].into_iter().collect(),
        ),
        warnings: false,
    }))
    .await;
    let transaction = setup.platform.drive.grove.start_transaction();
    let owner_id = setup.owner.id();

    // Naming the owner changes nothing about its authority or its protection.
    let ban = setup
        .moderate(&setup.owner, ban_action(setup.user.id()))
        .await;
    assert_success(&setup.process(&ban, &transaction));
    let ban_owner = setup.moderate(&setup.moderator, ban_action(owner_id)).await;
    assert_paid_with_code(
        &setup.process(&ban_owner, &transaction),
        CONTRACT_MODERATION_TARGET_NOT_ALLOWED,
    );
}

#[tokio::test]
async fn should_refuse_a_suspension_ending_past_the_json_safe_range() {
    let setup = Setup::new(Some(moderation(true, true, THE_MODERATOR))).await;
    let user_id = setup.user.id();
    let max_until = PlatformVersion::latest()
        .system_limits
        .max_contract_suspension_until;

    let transaction = setup.platform.drive.grove.start_transaction();
    let too_late = setup
        .moderate(&setup.owner, suspend_action(user_id, max_until + 1))
        .await;
    assert_unpaid_with_code(&setup.process(&too_late, &transaction), OVERFLOW);

    let latest = setup
        .moderate(&setup.owner, suspend_action(user_id, max_until))
        .await;
    assert_success(&setup.process(&latest, &transaction));
}

#[tokio::test]
async fn should_store_the_reason_of_a_ban_and_of_a_suspension_with_any_code() {
    let setup = Setup::new(Some(moderation(true, true, THE_MODERATOR))).await;
    let user_id = setup.user.id();
    let until = BLOCK_TIME_MS + 10_000;

    // No contract declares ban codes, so nothing checks the code a moderator writes.
    let suspension_reason = ContractModerationReason {
        code: Some(u16::MAX),
        text: "flooding the feed".to_string(),
        documents: vec![],
        reason_document_id: None,
    };
    let transaction = setup.platform.drive.grove.start_transaction();
    let suspend = setup
        .moderate(
            &setup.moderator,
            ContractUserModerationAction::Suspend {
                identity_id: user_id,
                until,
                reason: suspension_reason.clone(),
            },
        )
        .await;
    assert_success(&setup.process(&suspend, &transaction));
    assert_eq!(
        setup.status(user_id, Some(&transaction)),
        ContractModerationStatus {
            ban: None,
            suspension: Some(ContractSuspension {
                until,
                reason: suspension_reason,
            }),
            warnings: vec![],
        }
    );

    // An empty reason is a reason.
    let ban = setup
        .moderate(
            &setup.owner,
            ContractUserModerationAction::Ban {
                identity_id: user_id,
                reason: ContractModerationReason::default(),
            },
        )
        .await;
    assert_success(&setup.process(&ban, &transaction));
    assert_eq!(
        setup.status(user_id, Some(&transaction)),
        ContractModerationStatus {
            ban: Some(ContractBan::default()),
            suspension: None,
            warnings: vec![],
        }
    );
}

#[tokio::test]
async fn should_refuse_a_reason_longer_than_the_limit() {
    let setup = Setup::new(Some(moderation(true, true, THE_MODERATOR))).await;
    let user_id = setup.user.id();
    let max_length = PlatformVersion::latest()
        .system_limits
        .max_contract_moderation_reason_length as usize;

    let transaction = setup.platform.drive.grove.start_transaction();
    let too_long = setup
        .moderate(
            &setup.owner,
            ContractUserModerationAction::Ban {
                identity_id: user_id,
                reason: ContractModerationReason::from_text("x".repeat(max_length + 1)),
            },
        )
        .await;
    assert_unpaid_with_code(
        &setup.process(&too_long, &transaction),
        CONTRACT_MODERATION_REASON_TOO_LONG,
    );
    let too_long_suspension = setup
        .moderate(
            &setup.owner,
            ContractUserModerationAction::Suspend {
                identity_id: user_id,
                until: BLOCK_TIME_MS + 10_000,
                reason: ContractModerationReason::from_text("x".repeat(max_length + 1)),
            },
        )
        .await;
    assert_unpaid_with_code(
        &setup.process(&too_long_suspension, &transaction),
        CONTRACT_MODERATION_REASON_TOO_LONG,
    );

    // The limit itself is fine, and the moderator pays for every byte of it.
    let storage_fee_of = |execution: StateTransitionExecutionResult| match execution {
        StateTransitionExecutionResult::SuccessfulExecution { fee_result, .. } => {
            fee_result.storage_fee
        }
        other => panic!("expected a successful execution, got {other:?}"),
    };
    let short = setup.moderate(&setup.owner, ban_action(user_id)).await;
    let short_fee = storage_fee_of(setup.process(&short, &transaction));
    let longest = setup
        .moderate(
            &setup.owner,
            ContractUserModerationAction::Ban {
                identity_id: setup.stranger.id(),
                reason: ContractModerationReason::from_text("x".repeat(max_length)),
            },
        )
        .await;
    let longest_fee = storage_fee_of(setup.process(&longest, &transaction));
    assert!(
        longest_fee > short_fee,
        "{longest_fee} for the longest reason, {short_fee} for a short one"
    );
}

#[tokio::test]
async fn should_charge_the_same_fee_whether_or_not_the_contract_is_cached() {
    let setup = Setup::new(Some(moderation(true, true, THE_MODERATOR))).await;
    let ban = setup
        .moderate(&setup.owner, ban_action(setup.user.id()))
        .await;
    let fee_of = |execution: StateTransitionExecutionResult| match execution {
        StateTransitionExecutionResult::SuccessfulExecution { fee_result, .. } => fee_result,
        other => panic!("expected a successful execution, got {other:?}"),
    };

    // A node that created the contract holds it in its cache without a fee (the cache refresh
    // fetches it with no epoch); a node that restarted pulls it from disk, with one. Both
    // must charge the moderator the same, or their app hashes part.
    let transaction = setup.platform.drive.grove.start_transaction();
    let warm = fee_of(setup.process(&ban, &transaction));
    drop(transaction);

    setup.platform.drive.cache.data_contracts.clear();
    let transaction = setup.platform.drive.grove.start_transaction();
    let cold = fee_of(setup.process(&ban, &transaction));

    assert_eq!(warm, cold);
}

#[tokio::test]
async fn should_refuse_a_contract_that_does_not_exist_and_charge_for_it() {
    let mut setup = Setup::new(None).await;
    setup.contract.set_id(Identifier::from([0x66; 32]));
    let transaction = setup.platform.drive.grove.start_transaction();
    let ban = setup
        .moderate(&setup.owner, ban_action(setup.user.id()))
        .await;
    assert_paid_with_code(
        &setup.process(&ban, &transaction),
        DATA_CONTRACT_NOT_PRESENT,
    );
}

#[tokio::test]
async fn should_let_a_barred_identity_delete_its_own_documents() {
    let setup = Setup::new(Some(moderation(true, true, THE_MODERATOR))).await;
    let user_id = setup.user.id();

    let transaction = setup.platform.drive.grove.start_transaction();
    let (banned_with, create) = setup.create_document_keeping_it(&setup.user).await;
    assert_success(&setup.process(&create, &transaction));
    let (suspended_with, create) = setup.create_document_keeping_it(&setup.user).await;
    assert_success(&setup.process(&create, &transaction));

    // Suspended: nothing new, but what it wrote can come down.
    let suspend = setup
        .moderate(
            &setup.owner,
            suspend_action(user_id, BLOCK_TIME_MS + 10_000),
        )
        .await;
    assert_success(&setup.process(&suspend, &transaction));
    let refused = setup.create_document(&setup.user).await;
    assert_paid_with_code(
        &setup.process(&refused, &transaction),
        CONTRACT_USER_SUSPENDED,
    );
    let delete = setup.delete_document(&setup.user, suspended_with).await;
    assert_success(&setup.process(&delete, &transaction));

    // Banned: the same.
    let ban = setup.moderate(&setup.owner, ban_action(user_id)).await;
    assert_success(&setup.process(&ban, &transaction));
    let refused = setup.create_document(&setup.user).await;
    assert_paid_with_code(&setup.process(&refused, &transaction), CONTRACT_USER_BANNED);
    let delete = setup.delete_document(&setup.user, banned_with).await;
    assert_success(&setup.process(&delete, &transaction));
    setup.commit(transaction);

    // And the mempool lets the deletion of a banned identity through as a block does.
    let transaction = setup.platform.drive.grove.start_transaction();
    let unban = setup.moderate(&setup.owner, unban_action(user_id)).await;
    assert_success(&setup.process(&unban, &transaction));
    let (document, create) = setup.create_document_keeping_it(&setup.user).await;
    assert_success(&setup.process(&create, &transaction));
    let ban = setup.moderate(&setup.owner, ban_action(user_id)).await;
    assert_success(&setup.process(&ban, &transaction));
    setup.commit(transaction);
    let delete = setup.delete_document(&setup.user, document).await;
    assert!(setup.check_tx(&delete).is_empty());
}

#[tokio::test]
async fn should_prove_that_a_ban_removed_the_suspension() {
    let setup = Setup::new(Some(moderation(true, true, THE_MODERATOR))).await;
    let user_id = setup.user.id();

    let transaction = setup.platform.drive.grove.start_transaction();
    let suspend = setup
        .moderate(
            &setup.owner,
            suspend_action(user_id, BLOCK_TIME_MS + 10_000),
        )
        .await;
    assert_success(&setup.process(&suspend, &transaction));
    let ban = setup.moderate(&setup.owner, ban_action(user_id)).await;
    assert_success(&setup.process(&ban, &transaction));
    setup.commit(transaction);

    // The ban's proof covers both lists: it shows the ban and that the suspension is gone,
    // the two things a ban does.
    assert_eq!(
        setup.assert_execution_proved(&ban),
        ContractModerationListStatuses(vec![
            ContractModerationListStatus::Banlist { ban: banned() },
            ContractModerationListStatus::Suspensions { suspension: None },
        ])
    );
}

/// A moderated copy of the crypto card game, whose cards can be transferred and sold: the
/// counterparty gate needs documents that move between identities.
#[tokio::test]
async fn should_keep_a_barred_identity_from_receiving_or_selling_documents() {
    let platform_version = PlatformVersion::latest();
    let mut platform = TestPlatformBuilder::new()
        .build_with_mock_rpc()
        .set_genesis_state();
    let owner = Actor::new(&mut platform, 21);
    let seller = Actor::new(&mut platform, 22);
    let buyer = Actor::new(&mut platform, 23);

    let mut contract = json_document_to_contract(
        "tests/supporting_files/contract/crypto-card-game/crypto-card-game-direct-purchase.json",
        true,
        platform_version,
    )
    .expect("expected the card game contract");
    contract.set_owner_id(owner.id());
    contract.set_config(contract.config().clone().with_moderation(Some(moderation(
        true,
        true,
        owner.id(),
    ))));
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
    let card_type = contract
        .document_type_for_name("card")
        .expect("expected the card document type");
    let mut rng = StdRng::seed_from_u64(7);
    let process = |transition: &StateTransition, transaction: &Transaction| {
        let state = platform.state.load();
        let result = platform
            .platform
            .process_raw_state_transitions(
                &vec![transition.serialize_to_bytes().expect("serialize")],
                &state,
                &BlockInfo {
                    time_ms: BLOCK_TIME_MS,
                    ..Default::default()
                },
                transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process the state transition");
        result.execution_results()[0].clone()
    };
    // The seller mints a card.
    let entropy = Bytes32::random_with_rng(&mut rng);
    let mut card = card_type
        .random_document_with_identifier_and_entropy(
            &mut rng,
            seller.id(),
            entropy,
            DocumentFieldFillType::DoNotFillIfNotRequired,
            DocumentFieldFillSize::AnyDocumentFillSize,
            platform_version,
        )
        .expect("expected a random card");
    let creation_nonce = seller.contract_nonce();
    card.set_id_for_creation(card_type, &entropy.0, creation_nonce, platform_version)
        .expect("expected to set the document id");
    card.set("attack", 4.into());
    card.set("defense", 7.into());
    let create = BatchTransition::new_document_creation_transition_from_document(
        card.clone(),
        card_type,
        entropy.0,
        &seller.key,
        creation_nonce,
        0,
        None,
        &seller.signer,
        platform_version,
        None,
    )
    .await
    .expect("expected the card creation");
    let transaction = platform.drive.grove.start_transaction();
    assert_success(&process(&create, &transaction));

    // A banned buyer receives nothing: the seller's transfer to it is refused, paid.
    let ban_buyer = moderation_by(&owner, contract.id(), ban_action(buyer.id())).await;
    assert_success(&process(&ban_buyer, &transaction));
    card.set_revision(Some(2));
    let transfer = BatchTransition::new_document_transfer_transition_from_document(
        card.clone(),
        card_type,
        buyer.id(),
        &seller.key,
        seller.contract_nonce(),
        0,
        None,
        &seller.signer,
        platform_version,
        None,
    )
    .await
    .expect("expected the transfer");
    assert_paid_with_code(
        &process(&transfer, &transaction),
        CONTRACT_MODERATION_COUNTERPARTY_BARRED,
    );

    // Unbanned, the buyer may be sold to; but a banned seller sells nothing.
    let unban_buyer = moderation_by(&owner, contract.id(), unban_action(buyer.id())).await;
    assert_success(&process(&unban_buyer, &transaction));
    let set_price = BatchTransition::new_document_update_price_transition_from_document(
        card.clone(),
        card_type,
        dash_to_credits!(0.1),
        &seller.key,
        seller.contract_nonce(),
        0,
        None,
        &seller.signer,
        platform_version,
        None,
    )
    .await
    .expect("expected the price update");
    assert_success(&process(&set_price, &transaction));
    let ban_seller = moderation_by(&owner, contract.id(), ban_action(seller.id())).await;
    assert_success(&process(&ban_seller, &transaction));
    card.set_revision(Some(3));
    let purchase = BatchTransition::new_document_purchase_transition_from_document(
        card.clone(),
        card_type,
        buyer.id(),
        dash_to_credits!(0.1),
        &buyer.key,
        buyer.contract_nonce(),
        0,
        None,
        &buyer.signer,
        platform_version,
        None,
    )
    .await
    .expect("expected the purchase");
    assert_paid_with_code(
        &process(&purchase, &transaction),
        CONTRACT_MODERATION_COUNTERPARTY_BARRED,
    );

    // With nobody barred the same purchase goes through.
    let unban_seller = moderation_by(&owner, contract.id(), unban_action(seller.id())).await;
    assert_success(&process(&unban_seller, &transaction));
    let purchase = BatchTransition::new_document_purchase_transition_from_document(
        card,
        card_type,
        buyer.id(),
        dash_to_credits!(0.1),
        &buyer.key,
        buyer.contract_nonce(),
        0,
        None,
        &buyer.signer,
        platform_version,
        None,
    )
    .await
    .expect("expected the purchase");
    assert_success(&process(&purchase, &transaction));
}

// ---- document deletion by moderators ----------------------------------------------------

fn post_schema(deletable_by_moderators: bool) -> Value {
    platform_value!({
        "type": "object",
        "properties": {
            "text": { "type": "string", "maxLength": 50, "position": 0 },
        },
        "required": ["text"],
        "additionalProperties": false,
        "canBeDeletedByModerators": deletable_by_moderators,
    })
}

fn add_document_type(contract: &mut DataContract, name: &str, schema: Value) {
    contract
        .set_document_schema(name, schema, true, &mut vec![], PlatformVersion::latest())
        .expect("expected to add the document type");
}

fn delete_action(
    document_type_name: &str,
    document_id: Identifier,
) -> ContractUserModerationAction {
    ContractUserModerationAction::DeleteDocument {
        document_type_name: document_type_name.to_string(),
        document_id,
        reason: deletion_reason(),
    }
}

fn restore_action(document_type_name: &str, document: Vec<u8>) -> ContractUserModerationAction {
    ContractUserModerationAction::RestoreDocument {
        document_type_name: document_type_name.to_string(),
        document: BinaryData::new(document),
    }
}

fn deletion_reason() -> ContractModerationReason {
    ContractModerationReason {
        code: Some(3),
        text: "spam".to_string(),
        documents: vec![],
        reason_document_id: None,
    }
}

/// No list at all: the moderators of this contract only delete posts.
fn moderators_without_lists() -> ContractModerationConfig {
    moderation(false, false, THE_MODERATOR)
}

#[tokio::test]
async fn should_let_a_moderator_delete_a_post_leave_its_record_and_refund_nobody() {
    let setup = Setup::new_with_posts(Some(moderators_without_lists())).await;
    let user_id = setup.user.id();

    let transaction = setup.platform.drive.grove.start_transaction();
    let (post, create) = setup.create_document_of_type(&setup.user, POST).await;
    assert_success(&setup.process(&create, &transaction));
    setup.commit(transaction);
    let balance_before = setup.balance(user_id, None);
    // What the record will commit to: the post as stored, timestamps the block gave it
    // included, serialized under its type.
    let stored = setup
        .stored_document(POST, post.id(), None)
        .expect("expected the post to be stored");
    let document_hash = hash_double(setup.document_bytes(POST, &stored));

    let delete = setup
        .moderate(&setup.moderator, delete_action(POST, post.id()))
        .await;
    // The mempool takes it, as it does a ban.
    assert!(setup.check_tx(&delete).is_empty());

    let transaction = setup.platform.drive.grove.start_transaction();
    assert_success(&setup.process(&delete, &transaction));
    setup.commit(transaction);

    // The post is gone: its author can not delete it any more.
    let transaction = setup.platform.drive.grove.start_transaction();
    let own_delete = BatchTransition::new_document_deletion_transition_from_document(
        post.clone(),
        setup
            .contract
            .document_type_for_name(POST)
            .expect("expected the post type"),
        &setup.user.key,
        setup.user.contract_nonce(),
        0,
        None,
        &setup.user.signer,
        PlatformVersion::latest(),
        None,
    )
    .await
    .expect("expected to build the document deletion");
    let execution = setup.process(&own_delete, &transaction);
    assert!(
        matches!(&execution, StateTransitionExecutionResult::PaidConsensusError { error, .. }
            if matches!(error, ConsensusError::StateError(StateError::DocumentNotFoundError(_)))),
        "expected the post to be gone, got {execution:?}"
    );
    drop(transaction);

    // Its record says whose it was, who removed it, why and when, and what it was.
    let expected = ContractDocumentRemoval {
        document_owner_id: user_id,
        moderator_id: setup.moderator.id(),
        reason: deletion_reason(),
        removed_at: BLOCK_TIME_MS,
        document_hash,
        restoration: None,
    };
    assert_eq!(setup.post_removal(post.id(), None), Some(expected.clone()));
    assert_eq!(setup.assert_removal_proved(&delete), expected);

    // The author paid for the post's storage and gets nothing back.
    assert_eq!(setup.balance(user_id, None), balance_before);
}

#[tokio::test]
async fn should_refund_the_author_who_deletes_the_same_post_themselves() {
    // The control of the test above: the forfeiture is the moderator's deletion, not the
    // document type.
    let setup = Setup::new_with_posts(Some(moderators_without_lists())).await;
    let user_id = setup.user.id();

    let transaction = setup.platform.drive.grove.start_transaction();
    let (post, create) = setup.create_document_of_type(&setup.user, POST).await;
    assert_success(&setup.process(&create, &transaction));
    setup.commit(transaction);
    let balance_before = setup.balance(user_id, None);

    let transaction = setup.platform.drive.grove.start_transaction();
    let own_delete = BatchTransition::new_document_deletion_transition_from_document(
        post.clone(),
        setup
            .contract
            .document_type_for_name(POST)
            .expect("expected the post type"),
        &setup.user.key,
        setup.user.contract_nonce(),
        0,
        None,
        &setup.user.signer,
        PlatformVersion::latest(),
        None,
    )
    .await
    .expect("expected to build the document deletion");
    assert_success(&setup.process(&own_delete, &transaction));
    setup.commit(transaction);

    assert!(
        setup.balance(user_id, None) > balance_before,
        "the storage refund outweighs the deletion's processing fee"
    );
    assert_eq!(setup.post_removal(post.id(), None), None);
}

#[tokio::test]
async fn should_refuse_a_document_deletion_that_breaks_a_rule() {
    let setup = Setup::new_with_posts(Some(moderators_without_lists())).await;

    let transaction = setup.platform.drive.grove.start_transaction();
    let (post, create) = setup.create_document_of_type(&setup.user, POST).await;
    assert_success(&setup.process(&create, &transaction));
    let (owners_post, create) = setup.create_document_of_type(&setup.owner, POST).await;
    assert_success(&setup.process(&create, &transaction));
    let (moderators_post, create) = setup.create_document_of_type(&setup.moderator, POST).await;
    assert_success(&setup.process(&create, &transaction));
    let (nice_document, create) = setup.create_document_keeping_it(&setup.user).await;
    assert_success(&setup.process(&create, &transaction));

    // Nobody but the owner and the moderators.
    let by_stranger = setup
        .moderate(&setup.stranger, delete_action(POST, post.id()))
        .await;
    assert_paid_with_code(
        &setup.process(&by_stranger, &transaction),
        IDENTITY_NOT_CONTRACT_MODERATOR,
    );
    assert_eq!(
        setup.check_tx(&by_stranger)[0].code(),
        IDENTITY_NOT_CONTRACT_MODERATOR
    );
    // Not the author through this transition either: an author uses a document deletion.
    let by_author = setup
        .moderate(&setup.user, delete_action(POST, post.id()))
        .await;
    assert_paid_with_code(
        &setup.process(&by_author, &transaction),
        IDENTITY_NOT_CONTRACT_MODERATOR,
    );

    // Only a document type that says so.
    let of_another_type = setup
        .moderate(
            &setup.moderator,
            delete_action(DOCUMENT_TYPE, nice_document.id()),
        )
        .await;
    assert_paid_with_code(
        &setup.process(&of_another_type, &transaction),
        DOCUMENT_TYPE_NOT_DELETABLE_BY_MODERATORS,
    );
    let of_no_type = setup
        .moderate(&setup.moderator, delete_action("comment", post.id()))
        .await;
    assert_paid_with_code(
        &setup.process(&of_no_type, &transaction),
        INVALID_DOCUMENT_TYPE,
    );

    // Only a document that exists.
    let of_nothing = setup
        .moderate(
            &setup.moderator,
            delete_action(POST, Identifier::from([0x5a; 32])),
        )
        .await;
    let execution = setup.process(&of_nothing, &transaction);
    assert!(
        matches!(&execution, StateTransitionExecutionResult::PaidConsensusError { error, .. }
            if matches!(error, ConsensusError::StateError(StateError::DocumentNotFoundError(_)))),
        "expected a paid document not found error, got {execution:?}"
    );

    // Never the owner's or a moderator's, whoever asks.
    for (actor, document) in [
        (&setup.moderator, &owners_post),
        (&setup.owner, &moderators_post),
    ] {
        let protected = setup
            .moderate(actor, delete_action(POST, document.id()))
            .await;
        assert_paid_with_code(
            &setup.process(&protected, &transaction),
            CONTRACT_MODERATION_TARGET_NOT_ALLOWED,
        );
    }

    // A reason within the limit, refused before anything is paid.
    let too_long = setup
        .moderate(
            &setup.moderator,
            ContractUserModerationAction::DeleteDocument {
                document_type_name: POST.to_string(),
                document_id: post.id(),
                reason: ContractModerationReason::from_text(
                    &"x".repeat(
                        PlatformVersion::latest()
                            .system_limits
                            .max_contract_moderation_reason_length as usize
                            + 1,
                    ),
                ),
            },
        )
        .await;
    assert_unpaid_with_code(
        &setup.process(&too_long, &transaction),
        CONTRACT_MODERATION_REASON_TOO_LONG,
    );

    // None of it touched the post, and the owner may delete it as any moderator may, with no
    // reason at all.
    assert_eq!(setup.post_removal(post.id(), Some(&transaction)), None);
    let by_owner = setup
        .moderate(
            &setup.owner,
            ContractUserModerationAction::DeleteDocument {
                document_type_name: POST.to_string(),
                document_id: post.id(),
                reason: ContractModerationReason::default(),
            },
        )
        .await;
    assert_success(&setup.process(&by_owner, &transaction));
    let removal = setup
        .post_removal(post.id(), Some(&transaction))
        .expect("expected the record");
    assert_eq!(removal.moderator_id, setup.owner.id());
    assert_eq!(removal.reason, ContractModerationReason::default());
}

#[tokio::test]
async fn should_tie_the_document_type_keyword_to_the_moderation_declaration() {
    // A document type moderators could delete from on a contract without moderation is
    // refused where the document type is parsed (`moderators_delete_tests` in dpp), with the
    // consensus error a contract create turns into a paid refusal.
    let mut unmoderated = get_data_contract_fixture(None, 0, 14).data_contract_owned();
    let refusal = unmoderated
        .set_document_schema(
            POST,
            post_schema(true),
            true,
            &mut vec![],
            PlatformVersion::latest(),
        )
        .expect_err("expected the document type to be refused");
    assert!(
        matches!(&refusal, ProtocolError::ConsensusError(error)
            if error.code() == INVALID_CONTRACT_STRUCTURE),
        "expected an invalid contract structure, got {refusal:?}"
    );

    // No list and no such document type: the moderators would have nothing to do.
    let setup = Setup::new(None).await;
    let transaction = setup.platform.drive.grove.start_transaction();
    let identity_nonce = setup.owner.identity_nonce();
    let mut idle = get_data_contract_fixture(
        Some(setup.owner.id()),
        identity_nonce,
        PlatformVersion::latest().protocol_version,
    )
    .data_contract_owned();
    idle.set_config(
        idle.config()
            .clone()
            .with_moderation(Some(ContractModerationConfig {
                banlist: false,
                suspensions: false,
                moderators: ContractModerators::ContractOwner,
                warnings: false,
            })),
    );
    let create = DataContractCreateTransition::new_from_data_contract(
        idle,
        identity_nonce,
        &setup.owner.identity.clone().into_partial_identity_info(),
        CRITICAL_KEY_ID,
        &setup.owner.signer,
        PlatformVersion::latest(),
        None,
    )
    .await
    .expect("expected to build the contract create");
    assert_unpaid_with_code(
        &setup.process(&create, &transaction),
        INVALID_CONTRACT_MODERATION_CONFIG,
    );
}

#[tokio::test]
async fn should_fix_the_keyword_of_a_document_type_and_let_an_update_add_a_type_with_it() {
    // Lists only to start with.
    let mut setup = Setup::new(Some(moderation(true, false, THE_MODERATOR))).await;

    // An existing document type can not become one moderators delete from.
    let transaction = setup.platform.drive.grove.start_transaction();
    let mut flipped = setup.contract.clone();
    flipped.increment_version();
    let mut nice_document_schema = flipped
        .document_type_for_name(DOCUMENT_TYPE)
        .expect("expected the document type")
        .schema()
        .clone();
    nice_document_schema
        .insert("canBeDeletedByModerators".to_string(), Value::Bool(true))
        .expect("expected to set the keyword");
    add_document_type(&mut flipped, DOCUMENT_TYPE, nice_document_schema);
    let update = setup.contract_update(flipped).await;
    assert_paid_with_code(&setup.process(&update, &transaction), DOCUMENT_TYPE_UPDATE);
    drop(transaction);

    // A document type the update adds may be one, and then its documents can be deleted.
    let transaction = setup.platform.drive.grove.start_transaction();
    let mut with_posts = setup.contract.clone();
    with_posts.increment_version();
    // `canBeDeleted: false`, so that what follows about references depends on the moderators'
    // keyword alone.
    add_document_type(
        &mut with_posts,
        POST,
        post_schema_with(platform_value!({ "canBeDeleted": false })),
    );
    let update = setup.contract_update(with_posts.clone()).await;
    assert_success(&setup.process(&update, &transaction));
    setup.contract = with_posts;

    let (post, create) = setup.create_document_of_type(&setup.user, POST).await;
    assert_success(&setup.process(&create, &transaction));
    let delete = setup
        .moderate(&setup.moderator, delete_action(POST, post.id()))
        .await;
    assert_success(&setup.process(&delete, &transaction));
    assert!(setup.post_removal(post.id(), Some(&transaction)).is_some());

    // And it can not be the target of a permanent reference: its documents can vanish.
    let mut referring = setup.contract.clone();
    referring.increment_version();
    add_document_type(
        &mut referring,
        "bookmark",
        platform_value!({
            "type": "object",
            "properties": {
                "postId": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "refersTo": { "type": "permanentDocument", "documentType": POST },
                    "position": 0,
                },
            },
            "required": ["postId"],
            "additionalProperties": false,
        }),
    );
    let update = setup.contract_update(referring.clone()).await;
    assert_paid_with_code(
        &setup.process(&update, &transaction),
        REFERENCED_DOCUMENT_TYPE_DELETABLE,
    );

    // A deletable reference is what points at it: deletable means by anyone, the moderators
    // included, whatever `canBeDeleted` says about a post's own author.
    let bookmark_schema = referring
        .document_type_for_name("bookmark")
        .expect("expected the bookmark type")
        .schema()
        .clone();
    let mut deletable_reference = bookmark_schema;
    deletable_reference
        .get_mut("properties")
        .ok()
        .flatten()
        .and_then(|properties| properties.get_mut("postId").ok().flatten())
        .and_then(|post_id| post_id.get_mut("refersTo").ok().flatten())
        .expect("expected the refersTo declaration")
        .insert(
            "type".to_string(),
            Value::Text("deletableDocument".to_string()),
        )
        .expect("expected to set the reference kind");
    let mut referring = setup.contract.clone();
    referring.increment_version();
    add_document_type(&mut referring, "bookmark", deletable_reference);
    let update = setup.contract_update(referring.clone()).await;
    assert_success(&setup.process(&update, &transaction));
    setup.contract = referring;

    // And at document write: a bookmark of a post that exists is admitted.
    let (kept_post, create) = setup.create_document_of_type(&setup.user, POST).await;
    assert_success(&setup.process(&create, &transaction));
    let post_id = kept_post.id();
    let (_, bookmark) = setup
        .create_document_of_type_with(&setup.stranger, "bookmark", |bookmark| {
            bookmark.set("postId", post_id.into())
        })
        .await;
    assert_success(&setup.process(&bookmark, &transaction));
}

/// `post_schema(true)` with further document type keywords
fn post_schema_with(extra: Value) -> Value {
    let mut schema = post_schema(true);
    if let (Value::Map(schema_map), Value::Map(extra_map)) = (&mut schema, extra) {
        for (key, value) in extra_map {
            // A keyword given again replaces the one the base schema has.
            schema_map.retain(|(existing, _)| existing != &key);
            schema_map.push((key, value));
        }
    }
    schema
}

/// The deletion of `document`, of the `post` type, by `actor`, its owner
async fn own_post_deletion(setup: &Setup, actor: &Actor, document: Document) -> StateTransition {
    BatchTransition::new_document_deletion_transition_from_document(
        document,
        setup
            .contract
            .document_type_for_name(POST)
            .expect("expected the post type"),
        &actor.key,
        actor.contract_nonce(),
        0,
        None,
        &actor.signer,
        PlatformVersion::latest(),
        None,
    )
    .await
    .expect("expected to build the document deletion")
}

#[tokio::test]
async fn should_let_a_moderator_delete_a_post_its_author_can_not_delete() {
    // `canBeDeleted` is the author's rule: a post nobody can retract that moderation can
    // remove is what the keyword is for, and Drive's own guard on `canBeDeleted` must not
    // stand in the moderators' way.
    let setup = Setup::new_at_with(
        Some(moderators_without_lists()),
        PlatformVersion::latest(),
        |contract| {
            add_document_type(
                contract,
                POST,
                post_schema_with(platform_value!({ "canBeDeleted": false })),
            )
        },
    )
    .await;

    let transaction = setup.platform.drive.grove.start_transaction();
    let (post, create) = setup.create_document_of_type(&setup.user, POST).await;
    assert_success(&setup.process(&create, &transaction));

    let own_delete = own_post_deletion(&setup, &setup.user, post.clone()).await;
    let execution = setup.process(&own_delete, &transaction);
    assert!(
        matches!(
            &execution,
            StateTransitionExecutionResult::PaidConsensusError { .. }
        ),
        "expected the author's deletion to be refused, got {execution:?}"
    );
    assert_eq!(setup.post_removal(post.id(), Some(&transaction)), None);

    let delete = setup
        .moderate(&setup.moderator, delete_action(POST, post.id()))
        .await;
    assert_success(&setup.process(&delete, &transaction));
    setup.commit(transaction);
    // The mempool's fee estimate runs the same deletion: it takes the next one too.
    let (another, create) = setup.create_document_of_type(&setup.user, POST).await;
    let transaction = setup.platform.drive.grove.start_transaction();
    assert_success(&setup.process(&create, &transaction));
    setup.commit(transaction);
    let delete_another = setup
        .moderate(&setup.moderator, delete_action(POST, another.id()))
        .await;
    assert!(setup.check_tx(&delete_another).is_empty());

    let removal = setup
        .post_removal(post.id(), None)
        .expect("expected the record");
    assert_eq!(removal.document_owner_id, setup.user.id());
}

#[tokio::test]
async fn should_delete_a_transferred_post_and_record_the_owner_it_had() {
    let setup = Setup::new_at_with(
        Some(moderators_without_lists()),
        PlatformVersion::latest(),
        |contract| {
            add_document_type(
                contract,
                POST,
                post_schema_with(platform_value!({
                    "transferable": 1,
                    "documentsMutable": false,
                })),
            )
        },
    )
    .await;

    let transaction = setup.platform.drive.grove.start_transaction();
    let (mut post, create) = setup.create_document_of_type(&setup.user, POST).await;
    assert_success(&setup.process(&create, &transaction));

    post.set_revision(Some(2));
    let transfer = BatchTransition::new_document_transfer_transition_from_document(
        post.clone(),
        setup
            .contract
            .document_type_for_name(POST)
            .expect("expected the post type"),
        setup.stranger.id(),
        &setup.user.key,
        setup.user.contract_nonce(),
        0,
        None,
        &setup.user.signer,
        PlatformVersion::latest(),
        None,
    )
    .await
    .expect("expected the transfer");
    assert_success(&setup.process(&transfer, &transaction));
    setup.commit(transaction);

    let author_before = setup.balance(setup.user.id(), None);
    let holder_before = setup.balance(setup.stranger.id(), None);

    let transaction = setup.platform.drive.grove.start_transaction();
    let delete = setup
        .moderate(&setup.moderator, delete_action(POST, post.id()))
        .await;
    assert_success(&setup.process(&delete, &transaction));
    setup.commit(transaction);

    // The record names who held the post when it was removed, and neither that identity nor
    // the one that first paid for the post is refunded.
    let removal = setup
        .post_removal(post.id(), None)
        .expect("expected the record");
    assert_eq!(removal.document_owner_id, setup.stranger.id());
    assert_eq!(setup.balance(setup.user.id(), None), author_before);
    assert_eq!(setup.balance(setup.stranger.id(), None), holder_before);
}

#[tokio::test]
async fn should_charge_a_moderator_no_token_for_a_post_whose_deletion_costs_tokens() {
    let mut setup = Setup::new(None).await;
    let platform_version = PlatformVersion::latest();

    // A second contract with a token, whose posts cost their author tokens to delete. It goes
    // straight into Drive: the token's own declaration is not what is tested here.
    let mut contract = get_data_contract_fixture(
        Some(setup.owner.id()),
        77,
        platform_version.protocol_version,
    )
    .data_contract_owned();
    contract.set_config(contract.config().clone().with_moderation(Some(moderation(
        false,
        false,
        setup.moderator.id(),
    ))));
    contract.add_token(
        0,
        TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive()),
    );
    add_document_type(
        &mut contract,
        POST,
        post_schema_with(platform_value!({
            "tokenCost": { "delete": { "tokenPosition": 0, "amount": 5 } },
        })),
    );
    setup
        .platform
        .drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            platform_version,
        )
        .expect("expected to store the contract");
    setup.contract = contract;

    let transaction = setup.platform.drive.grove.start_transaction();
    let (post, create) = setup.create_document_of_type(&setup.user, POST).await;
    assert_success(&setup.process(&create, &transaction));

    // The author holds no token and can not pay for the deletion.
    let own_delete = own_post_deletion(&setup, &setup.user, post.clone()).await;
    let execution = setup.process(&own_delete, &transaction);
    assert!(
        matches!(
            &execution,
            StateTransitionExecutionResult::PaidConsensusError { .. }
        ),
        "expected the author's unpaid deletion to be refused, got {execution:?}"
    );

    // The moderator holds none either, and is asked for none.
    let delete = setup
        .moderate(&setup.moderator, delete_action(POST, post.id()))
        .await;
    assert_success(&setup.process(&delete, &transaction));
    assert!(setup.post_removal(post.id(), Some(&transaction)).is_some());
}

/// The window the posts of the tests below give their moderators
const MODERATION_WINDOW_SECONDS: u64 = 60;
const MODERATION_WINDOW_MS: TimestampMillis = MODERATION_WINDOW_SECONDS * 1_000;

/// A contract whose posts are mutable and can be deleted by moderators for a minute after
/// their last modification
async fn setup_with_a_moderation_window() -> Setup {
    Setup::new_at_with(
        Some(moderators_without_lists()),
        PlatformVersion::latest(),
        |contract| {
            add_document_type(
                contract,
                POST,
                post_schema_with(platform_value!({
                    "canBeDeletedByModeratorsFor": MODERATION_WINDOW_SECONDS,
                    "documentsMutable": true,
                    "required": ["text", "$updatedAt"],
                })),
            )
        },
    )
    .await
}

#[tokio::test]
async fn should_let_moderators_delete_a_post_only_within_the_window_after_its_last_modification() {
    let setup = setup_with_a_moderation_window().await;

    // Two posts, written at the block time of the tests.
    let transaction = setup.platform.drive.grove.start_transaction();
    let (in_time, create) = setup.create_document_of_type(&setup.user, POST).await;
    assert_success(&setup.process(&create, &transaction));
    let (settled, create) = setup.create_document_of_type(&setup.user, POST).await;
    assert_success(&setup.process(&create, &transaction));

    // To the millisecond the window ends on, a moderator may delete.
    let delete = setup
        .moderate(&setup.moderator, delete_action(POST, in_time.id()))
        .await;
    assert_success(&setup.process_at(&delete, BLOCK_TIME_MS + MODERATION_WINDOW_MS, &transaction));

    // One millisecond later the post is settled: no moderator deletes it, the owner of the
    // contract included, and the refusal is paid.
    let past_the_window = BLOCK_TIME_MS + MODERATION_WINDOW_MS + 1;
    for actor in [&setup.moderator, &setup.owner] {
        let too_late = setup
            .moderate(actor, delete_action(POST, settled.id()))
            .await;
        assert_paid_with_code(
            &setup.process_at(&too_late, past_the_window, &transaction),
            DOCUMENT_MODERATION_WINDOW_ELAPSED,
        );
    }
    assert_eq!(setup.post_removal(settled.id(), Some(&transaction)), None);

    // Its author can still delete it: the window is the moderators', not the author's.
    let own_delete = own_post_deletion(&setup, &setup.user, settled.clone()).await;
    assert_success(&setup.process_at(&own_delete, past_the_window, &transaction));
}

#[tokio::test]
async fn should_open_the_window_again_when_the_author_modifies_the_post() {
    let setup = setup_with_a_moderation_window().await;

    let transaction = setup.platform.drive.grove.start_transaction();
    let (mut post, create) = setup.create_document_of_type(&setup.user, POST).await;
    assert_success(&setup.process(&create, &transaction));

    // Long after the window closed, the author rewrites the post. What it says now is new
    // content, which the moderators get their minute for.
    let edited_at = BLOCK_TIME_MS + 10 * MODERATION_WINDOW_MS;
    post.set("text", "rewritten".into());
    post.increment_revision()
        .expect("expected to bump the revision");
    let replace = BatchTransition::new_document_replacement_transition_from_document(
        post.clone(),
        setup
            .contract
            .document_type_for_name(POST)
            .expect("expected the post type"),
        &setup.user.key,
        setup.user.contract_nonce(),
        0,
        None,
        &setup.user.signer,
        PlatformVersion::latest(),
        None,
    )
    .await
    .expect("expected to build the replacement");
    assert_success(&setup.process_at(&replace, edited_at, &transaction));

    let delete = setup
        .moderate(&setup.moderator, delete_action(POST, post.id()))
        .await;
    assert_success(&setup.process_at(&delete, edited_at + MODERATION_WINDOW_MS, &transaction));
    let removal = setup
        .post_removal(post.id(), Some(&transaction))
        .expect("expected the record");
    assert_eq!(removal.removed_at, edited_at + MODERATION_WINDOW_MS);
}

#[tokio::test]
async fn should_measure_the_window_from_the_creation_of_a_post_that_never_changes() {
    // An immutable type needs no `$updatedAt`: nothing modifies a post after it is created, so
    // `$createdAt` is its last modification.
    let setup = Setup::new_at_with(
        Some(moderators_without_lists()),
        PlatformVersion::latest(),
        |contract| {
            add_document_type(
                contract,
                POST,
                post_schema_with(platform_value!({
                    "canBeDeletedByModeratorsFor": MODERATION_WINDOW_SECONDS,
                    "documentsMutable": false,
                    "required": ["text", "$createdAt"],
                })),
            )
        },
    )
    .await;

    let transaction = setup.platform.drive.grove.start_transaction();
    let (in_time, create) = setup.create_document_of_type(&setup.user, POST).await;
    assert_success(&setup.process(&create, &transaction));
    let (settled, create) = setup.create_document_of_type(&setup.user, POST).await;
    assert_success(&setup.process(&create, &transaction));

    let delete = setup
        .moderate(&setup.moderator, delete_action(POST, in_time.id()))
        .await;
    assert_success(&setup.process_at(&delete, BLOCK_TIME_MS + MODERATION_WINDOW_MS, &transaction));

    let too_late = setup
        .moderate(&setup.moderator, delete_action(POST, settled.id()))
        .await;
    let execution = setup.process_at(
        &too_late,
        BLOCK_TIME_MS + MODERATION_WINDOW_MS + 1,
        &transaction,
    );
    assert_paid_with_code(&execution, DOCUMENT_MODERATION_WINDOW_ELAPSED);
    // The refusal names the creation time as the last modification.
    assert!(
        matches!(&execution, StateTransitionExecutionResult::PaidConsensusError { error, .. }
            if matches!(error, ConsensusError::StateError(
                StateError::DocumentModerationWindowElapsedError(e)
            ) if e.last_modified_at() == BLOCK_TIME_MS)),
        "expected the creation time as the last modification, got {execution:?}"
    );
}

#[tokio::test]
async fn should_fix_the_window_of_a_document_type() {
    let setup = setup_with_a_moderation_window().await;
    let transaction = setup.platform.drive.grove.start_transaction();

    // A longer window would reopen posts that had settled; none at all, every post ever.
    for window in [Some(10 * MODERATION_WINDOW_SECONDS), None] {
        let mut changed = setup.contract.clone();
        changed.increment_version();
        let mut schema = changed
            .document_type_for_name(POST)
            .expect("expected the post type")
            .schema()
            .clone();
        match window {
            Some(seconds) => {
                schema
                    .insert("canBeDeletedByModeratorsFor".to_string(), seconds.into())
                    .expect("expected to set the window");
            }
            None => {
                if let Value::Map(map) = &mut schema {
                    map.retain(|(key, _)| {
                        key != &Value::Text("canBeDeletedByModeratorsFor".to_string())
                    });
                }
            }
        }
        add_document_type(&mut changed, POST, schema);
        let update = setup.contract_update(changed).await;
        assert_paid_with_code(&setup.process(&update, &transaction), DOCUMENT_TYPE_UPDATE);
    }
}

/// An elected declaration keeping both lists, allowing bans and suspensions, moderating
/// `moderated`, with `interim` until a team is seated
fn elected(interim: InterimModerators, moderated: &[&str]) -> ContractModerationConfig {
    ContractModerationConfig {
        banlist: true,
        suspensions: true,
        warnings: false,
        moderators: ContractModerators::Elected(Box::new(ElectedModerators {
            join_window: DEFAULT_ELECTION_WINDOW_SECONDS,
            vote_window: DEFAULT_ELECTION_WINDOW_SECONDS,
            challenge_cool_down: Some(1_209_600),
            moderated_document_types: moderated
                .iter()
                .map(|name| {
                    (
                        name.to_string(),
                        BTreeSet::from([ModerationAbility::Ban, ModerationAbility::Suspend]),
                    )
                })
                .collect(),
            interim,
            election_delay: None,
            max_added_moderators: 0,
            owner_protected: false,
        })),
    }
}

fn assert_config_update_refused(execution: &StateTransitionExecutionResult, what: &str) {
    assert!(
        matches!(
            execution,
            StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(StateError::DataContractConfigUpdateError(_)),
                ..
            }
        ),
        "expected {what} to be refused, got {execution:?}"
    );
}

/// The owner moderates an elected contract until a team is seated: it bans, nobody else may,
/// the moderated type is usable, and the declaration is frozen against every update that
/// touches it while one that leaves it alone goes through.
#[tokio::test]
async fn should_let_the_owner_moderate_an_elected_contract_in_its_interim() {
    let setup = Setup::new(Some(elected(
        InterimModerators::ContractOwner,
        &[DOCUMENT_TYPE],
    )))
    .await;
    let transaction = setup.platform.drive.grove.start_transaction();

    // The moderated type is usable: somebody moderates.
    let post = setup.create_document(&setup.user).await;
    assert_success(&setup.process(&post, &transaction));

    let ban = setup
        .moderate(&setup.owner, ban_action(setup.user.id()))
        .await;
    assert_success(&setup.process(&ban, &transaction));
    assert_eq!(
        setup.status(setup.user.id(), Some(&transaction)).ban,
        banned()
    );
    let refused = setup.create_document(&setup.user).await;
    assert_paid_with_code(&setup.process(&refused, &transaction), CONTRACT_USER_BANNED);

    // The interim names nobody else: the moderator of the other tests is a stranger here.
    for actor in [&setup.moderator, &setup.stranger] {
        let ban = setup.moderate(actor, ban_action(setup.user.id())).await;
        assert_paid_with_code(
            &setup.process(&ban, &transaction),
            IDENTITY_NOT_CONTRACT_MODERATOR,
        );
    }

    // Frozen: a changed field, a changed interim, and leaving elected moderation.
    let with_config = |moderation: ContractModerationConfig| {
        let mut changed = setup.contract.clone();
        changed.set_version(2);
        changed.set_config(changed.config().clone().with_moderation(Some(moderation)));
        changed
    };
    let mut longer_cool_down = elected(InterimModerators::ContractOwner, &[DOCUMENT_TYPE]);
    if let ContractModerators::Elected(declaration) = &mut longer_cool_down.moderators {
        declaration.challenge_cool_down = declaration.challenge_cool_down.map(|c| c + 1);
    }
    let mut permanent_seat = elected(InterimModerators::ContractOwner, &[DOCUMENT_TYPE]);
    if let ContractModerators::Elected(declaration) = &mut permanent_seat.moderators {
        declaration.challenge_cool_down = None;
    }
    let mut protected_owner = elected(InterimModerators::ContractOwner, &[DOCUMENT_TYPE]);
    if let ContractModerators::Elected(declaration) = &mut protected_owner.moderators {
        declaration.owner_protected = true;
    }
    for (moderation, what) in [
        (longer_cool_down, "a longer cool-down"),
        (permanent_seat, "the seat made uncontestable"),
        (protected_owner, "the owner flag turned on"),
        (
            elected(
                InterimModerators::AppointedModerators([setup.moderator.id()].into()),
                &[DOCUMENT_TYPE],
            ),
            "an appointed interim set",
        ),
        (
            moderation(true, true, setup.moderator.id()),
            "leaving elected moderation",
        ),
    ] {
        let update = setup.contract_update(with_config(moderation)).await;
        assert_config_update_refused(&setup.process(&update, &transaction), what);
    }
    // A wider moderated set is frozen too, even when the same update adds the type it names
    // (without the type, the declaration's own validation refuses first).
    let mut wider = with_config(elected(
        InterimModerators::ContractOwner,
        &[DOCUMENT_TYPE, POST],
    ));
    add_document_type(&mut wider, POST, post_schema(false));
    let update = setup.contract_update(wider).await;
    assert_config_update_refused(
        &setup.process(&update, &transaction),
        "a wider moderated set",
    );

    // An update that leaves the declaration alone goes through, and may add a type.
    let mut widened = setup.contract.clone();
    widened.set_version(2);
    add_document_type(&mut widened, POST, post_schema(false));
    let update = setup.contract_update(widened).await;
    assert_success(&setup.process(&update, &transaction));
}

/// An appointed interim set moderates as an appointed set does: the moderator and the owner
/// ban, a stranger may not, and both are protected from each other's bans.
#[tokio::test]
async fn should_let_an_appointed_interim_set_moderate_an_elected_contract() {
    let setup = Setup::new(Some(elected(
        InterimModerators::AppointedModerators([THE_MODERATOR].into()),
        &[DOCUMENT_TYPE],
    )))
    .await;
    let transaction = setup.platform.drive.grove.start_transaction();

    let ban = setup
        .moderate(&setup.moderator, ban_action(setup.user.id()))
        .await;
    assert_success(&setup.process(&ban, &transaction));
    let ban = setup
        .moderate(&setup.owner, ban_action(setup.stranger.id()))
        .await;
    assert_success(&setup.process(&ban, &transaction));
    let ban = setup
        .moderate(&setup.stranger, ban_action(setup.user.id()))
        .await;
    assert_paid_with_code(
        &setup.process(&ban, &transaction),
        IDENTITY_NOT_CONTRACT_MODERATOR,
    );
    let ban = setup
        .moderate(&setup.owner, ban_action(setup.moderator.id()))
        .await;
    assert_paid_with_code(
        &setup.process(&ban, &transaction),
        CONTRACT_MODERATION_TARGET_NOT_ALLOWED,
    );
    let ban = setup
        .moderate(&setup.moderator, ban_action(setup.owner.id()))
        .await;
    assert_paid_with_code(
        &setup.process(&ban, &transaction),
        CONTRACT_MODERATION_TARGET_NOT_ALLOWED,
    );

    // The interim set shares the moderators pot, as an appointed set does.
    use dpp::data_contract::document_type::action_fees::ContractFeePot;
    assert_eq!(
        ContractFeePot::Moderators.recipients(&setup.contract),
        [setup.moderator.id()].into()
    );
}

/// With nobody named in the interim, the moderated type waits for a team: its creates are
/// refused, paid, in the mempool and in a block, another type works, nobody moderates, and
/// nobody claims the moderators pot.
#[tokio::test]
async fn should_block_the_moderated_types_of_an_elected_contract_until_a_team_is_seated() {
    let setup = Setup::new(Some(elected(
        InterimModerators::NotYetUsable,
        &[DOCUMENT_TYPE],
    )))
    .await;
    let transaction = setup.platform.drive.grove.start_transaction();

    let blocked = setup.create_document(&setup.user).await;
    let mempool_errors = setup.check_tx(&blocked);
    assert_eq!(mempool_errors.len(), 1, "{mempool_errors:?}");
    assert_eq!(
        mempool_errors[0].code(),
        CONTRACT_MODERATED_DOCUMENT_TYPE_NOT_YET_USABLE
    );
    assert_paid_with_code(
        &setup.process(&blocked, &transaction),
        CONTRACT_MODERATED_DOCUMENT_TYPE_NOT_YET_USABLE,
    );

    let (_, allowed) = setup
        .create_document_of_type(&setup.user, "prettyDocument")
        .await;
    assert_success(&setup.process(&allowed, &transaction));

    // Nobody moderates: the owner included.
    for actor in [&setup.owner, &setup.moderator] {
        let ban = setup.moderate(actor, ban_action(setup.user.id())).await;
        assert_paid_with_code(
            &setup.process(&ban, &transaction),
            IDENTITY_NOT_CONTRACT_MODERATOR,
        );
    }
    use dpp::data_contract::document_type::action_fees::ContractFeePot;
    assert!(ContractFeePot::Moderators
        .recipients(&setup.contract)
        .is_empty());
}

/// Under a `noModeration` interim the moderated types are used, unmoderated: nobody may
/// moderate, the owner included, and nobody claims the pot.
#[tokio::test]
async fn should_leave_the_moderated_types_usable_and_unmoderated_under_no_moderation() {
    let setup = Setup::new(Some(elected(
        InterimModerators::NoModeration,
        &[DOCUMENT_TYPE],
    )))
    .await;
    let transaction = setup.platform.drive.grove.start_transaction();

    let post = setup.create_document(&setup.user).await;
    assert!(setup.check_tx(&post).is_empty());
    assert_success(&setup.process(&post, &transaction));

    for actor in [&setup.owner, &setup.moderator] {
        let ban = setup.moderate(actor, ban_action(setup.user.id())).await;
        assert_paid_with_code(
            &setup.process(&ban, &transaction),
            IDENTITY_NOT_CONTRACT_MODERATOR,
        );
    }
    use dpp::data_contract::document_type::action_fees::ContractFeePot;
    assert!(ContractFeePot::Moderators
        .recipients(&setup.contract)
        .is_empty());
}

/// A declaration outside a bound or naming a type the contract does not have is refused,
/// unpaid, at the create; a contract that was not born elected can not become so.
#[tokio::test]
async fn should_refuse_an_elected_declaration_the_contract_can_not_back() {
    let mut setup = Setup::new(None).await;
    let transaction = setup.platform.drive.grove.start_transaction();
    let contract = setup.contract.clone();
    let with = |modify: fn(&mut ElectedModerators)| {
        let mut moderation = elected(InterimModerators::ContractOwner, &[DOCUMENT_TYPE]);
        if let ContractModerators::Elected(declaration) = &mut moderation.moderators {
            modify(declaration);
        }
        moderation
    };
    for (moderation, what) in [
        (
            with(|d| d.join_window = 86_399),
            "a join window under a day",
        ),
        (
            with(|d| d.vote_window = 86_399),
            "a vote window under a day",
        ),
        (
            with(|d| d.challenge_cool_down = Some(94_608_001)),
            "a cool-down over three years",
        ),
        (
            with(|d| {
                d.moderated_document_types = BTreeMap::from([(
                    "comment".to_string(),
                    BTreeSet::from([ModerationAbility::Ban]),
                )]);
            }),
            "an unknown moderated type",
        ),
        (
            with(|d| {
                d.moderated_document_types
                    .insert(DOCUMENT_TYPE.to_string(), BTreeSet::new());
            }),
            "an empty ability set",
        ),
        (
            with(|d| {
                d.moderated_document_types
                    .get_mut(DOCUMENT_TYPE)
                    .expect("moderated")
                    .insert(ModerationAbility::DeleteDocuments);
            }),
            "deletions on a type moderators can not delete from",
        ),
    ] {
        setup
            .contract
            .set_config(contract.config().clone().with_moderation(Some(moderation)));
        let create = setup
            .contract_create(setup.owner.identity_nonce(), PlatformVersion::latest())
            .await;
        let execution = setup.process(&create, &transaction);
        assert_unpaid_with_code(&execution, INVALID_CONTRACT_MODERATION_CONFIG);
        assert!(
            matches!(&execution, StateTransitionExecutionResult::UnpaidConsensusError(error) if error.to_string().contains("elected moderation")),
            "expected {what} to name the declaration, got {execution:?}"
        );
    }
    // The bounds hold: the same declaration at its minimums is accepted.
    setup
        .contract
        .set_config(contract.config().clone().with_moderation(Some(with(|d| {
            d.join_window = 86_400;
            d.vote_window = 86_400;
        }))));
    let create = setup
        .contract_create(setup.owner.identity_nonce(), PlatformVersion::latest())
        .await;
    assert_success(&setup.process(&create, &transaction));
    // A seat that can not be contested again has no cool-down to bound.
    setup
        .contract
        .set_config(contract.config().clone().with_moderation(Some(with(|d| {
            d.challenge_cool_down = None;
        }))));
    let create = setup
        .contract_create(setup.owner.identity_nonce(), PlatformVersion::latest())
        .await;
    assert_success(&setup.process(&create, &transaction));
    drop(transaction);

    // Entering elected moderation by an update is refused.
    let setup = Setup::new(Some(moderation(true, true, THE_MODERATOR))).await;
    let transaction = setup.platform.drive.grove.start_transaction();
    let mut entering = setup.contract.clone();
    entering.set_version(2);
    entering.set_config(entering.config().clone().with_moderation(Some(elected(
        InterimModerators::ContractOwner,
        &[DOCUMENT_TYPE],
    ))));
    let update = setup.contract_update(entering).await;
    assert_config_update_refused(
        &setup.process(&update, &transaction),
        "entering elected moderation",
    );
}
/// How long after a moderator's deletion a document can be restored: the protocol's week.
fn restore_window_ms() -> TimestampMillis {
    PlatformVersion::latest()
        .system_limits
        .contract_document_restore_window_ms
}

#[tokio::test]
async fn should_let_any_moderator_restore_a_deleted_post_within_a_week_and_delete_it_again() {
    let setup = Setup::new_with_posts(Some(moderators_without_lists())).await;
    let user_id = setup.user.id();

    let transaction = setup.platform.drive.grove.start_transaction();
    let (post, create) = setup.create_document_of_type(&setup.user, POST).await;
    assert_success(&setup.process(&create, &transaction));
    setup.commit(transaction);
    // The post as stored is what comes back: a client keeps it, or its bytes, before deleting.
    let stored = setup
        .stored_document(POST, post.id(), None)
        .expect("expected the post to be stored");
    let bytes = setup.document_bytes(POST, &stored);

    let delete = setup
        .moderate(&setup.moderator, delete_action(POST, post.id()))
        .await;
    let transaction = setup.platform.drive.grove.start_transaction();
    assert_success(&setup.process(&delete, &transaction));
    setup.commit(transaction);
    let removal = setup
        .post_removal(post.id(), None)
        .expect("expected the record");
    assert_eq!(removal.document_hash, hash_double(&bytes));
    assert_eq!(removal.restoration, None);
    assert_eq!(setup.stored_document(POST, post.id(), None), None);

    // Restored by the contract owner, not the moderator that deleted it, to the millisecond
    // the window ends on. The mempool takes it, as it does a ban.
    let restored_at = BLOCK_TIME_MS + restore_window_ms();
    let author_before = setup.balance(user_id, None);
    let owner_before = setup.balance(setup.owner.id(), None);
    let restore = setup
        .moderate(&setup.owner, restore_action(POST, bytes.clone()))
        .await;
    assert!(setup.check_tx(&restore).is_empty());
    let transaction = setup.platform.drive.grove.start_transaction();
    assert_success(&setup.process_at(&restore, restored_at, &transaction));
    setup.commit(transaction);

    // The post is back as it was, and its record says who brought it back and when.
    assert_eq!(setup.stored_document(POST, post.id(), None), Some(stored));
    let expected = ContractDocumentRemoval {
        restoration: Some(ContractDocumentRestoration {
            moderator_id: setup.owner.id(),
            restored_at,
        }),
        ..removal.clone()
    };
    assert_eq!(setup.post_removal(post.id(), None), Some(expected.clone()));
    assert_eq!(setup.assert_removal_proved(&restore), expected);
    // The owner paid for the post's storage; its author paid nothing and got nothing.
    assert!(setup.balance(setup.owner.id(), None) < owner_before);
    assert_eq!(setup.balance(user_id, None), author_before);

    // Deleted again: a fresh record in place of the restored one, the author refunded nothing.
    let removed_again_at = restored_at + 1;
    let delete_again = setup
        .moderate(&setup.moderator, delete_action(POST, post.id()))
        .await;
    let transaction = setup.platform.drive.grove.start_transaction();
    assert_success(&setup.process_at(&delete_again, removed_again_at, &transaction));
    setup.commit(transaction);
    assert_eq!(
        setup.post_removal(post.id(), None),
        Some(ContractDocumentRemoval {
            removed_at: removed_again_at,
            restoration: None,
            ..removal
        })
    );
    assert_eq!(setup.stored_document(POST, post.id(), None), None);
    assert_eq!(setup.balance(user_id, None), author_before);
}

#[tokio::test]
async fn should_refund_the_author_who_deletes_a_restored_post() {
    // The restored post's storage flags name its author, as they did before: the refund of the
    // author's own deletion is the author's, though a moderator paid to put the post back.
    let setup = Setup::new_with_posts(Some(moderators_without_lists())).await;
    let user_id = setup.user.id();

    let transaction = setup.platform.drive.grove.start_transaction();
    let (post, create) = setup.create_document_of_type(&setup.user, POST).await;
    assert_success(&setup.process(&create, &transaction));
    setup.commit(transaction);
    let stored = setup
        .stored_document(POST, post.id(), None)
        .expect("expected the post to be stored");
    let bytes = setup.document_bytes(POST, &stored);

    let transaction = setup.platform.drive.grove.start_transaction();
    let delete = setup
        .moderate(&setup.moderator, delete_action(POST, post.id()))
        .await;
    assert_success(&setup.process(&delete, &transaction));
    let restore = setup
        .moderate(&setup.moderator, restore_action(POST, bytes))
        .await;
    assert_success(&setup.process_at(&restore, BLOCK_TIME_MS + 1, &transaction));
    setup.commit(transaction);

    let author_before = setup.balance(user_id, None);
    let moderator_before = setup.balance(setup.moderator.id(), None);
    let transaction = setup.platform.drive.grove.start_transaction();
    let own_delete = own_post_deletion(&setup, &setup.user, stored).await;
    assert_success(&setup.process(&own_delete, &transaction));
    setup.commit(transaction);
    assert!(
        setup.balance(user_id, None) > author_before,
        "the storage refund outweighs the deletion's processing fee"
    );
    assert_eq!(setup.balance(setup.moderator.id(), None), moderator_before);
    // The author's deletion is no moderation: the record stays as the restore left it.
    assert_eq!(
        setup
            .post_removal(post.id(), None)
            .and_then(|removal| removal.restoration)
            .map(|restoration| restoration.moderator_id),
        Some(setup.moderator.id())
    );
}

#[tokio::test]
async fn should_refuse_a_document_restore_that_breaks_a_rule() {
    let setup = Setup::new_with_posts(Some(moderators_without_lists())).await;

    let transaction = setup.platform.drive.grove.start_transaction();
    let (post, create) = setup.create_document_of_type(&setup.user, POST).await;
    assert_success(&setup.process(&create, &transaction));
    let (nice_document, create) = setup.create_document_keeping_it(&setup.user).await;
    assert_success(&setup.process(&create, &transaction));
    let stored = setup
        .stored_document(POST, post.id(), Some(&transaction))
        .expect("expected the post to be stored");
    let bytes = setup.document_bytes(POST, &stored);

    // Nothing to restore: the post is live and has no record.
    let live = setup
        .moderate(&setup.moderator, restore_action(POST, bytes.clone()))
        .await;
    assert_paid_with_code(
        &setup.process(&live, &transaction),
        CONTRACT_DOCUMENT_REMOVAL_NOT_FOUND,
    );

    let delete = setup
        .moderate(&setup.moderator, delete_action(POST, post.id()))
        .await;
    assert_success(&setup.process(&delete, &transaction));

    // Nobody but the owner and the moderators, the post's author included.
    for actor in [&setup.stranger, &setup.user] {
        let by_other = setup
            .moderate(actor, restore_action(POST, bytes.clone()))
            .await;
        assert_paid_with_code(
            &setup.process(&by_other, &transaction),
            IDENTITY_NOT_CONTRACT_MODERATOR,
        );
    }

    // Only a document type that says so, and one that exists.
    let nice_stored = setup
        .stored_document(DOCUMENT_TYPE, nice_document.id(), Some(&transaction))
        .expect("expected the document to be stored");
    let of_another_type = setup
        .moderate(
            &setup.moderator,
            restore_action(
                DOCUMENT_TYPE,
                setup.document_bytes(DOCUMENT_TYPE, &nice_stored),
            ),
        )
        .await;
    assert_paid_with_code(
        &setup.process(&of_another_type, &transaction),
        DOCUMENT_TYPE_NOT_DELETABLE_BY_MODERATORS,
    );
    let of_no_type = setup
        .moderate(&setup.moderator, restore_action("comment", bytes.clone()))
        .await;
    assert_paid_with_code(
        &setup.process(&of_no_type, &transaction),
        INVALID_DOCUMENT_TYPE,
    );

    // Bytes that do not decode under the type: a paid refusal, never an execution error.
    let garbage = setup
        .moderate(&setup.moderator, restore_action(POST, vec![]))
        .await;
    assert_paid_with_code(&setup.process(&garbage, &transaction), DECODING_DOCUMENT);

    // The post as it was, not an edit of it: the same post with other text decodes, and is
    // refused by its hash.
    let mut edited = stored.clone();
    edited.set("text", Value::Text("something else".to_string()));
    let of_an_edit = setup
        .moderate(
            &setup.moderator,
            restore_action(POST, setup.document_bytes(POST, &edited)),
        )
        .await;
    assert_paid_with_code(
        &setup.process(&of_an_edit, &transaction),
        DOCUMENT_RESTORE_HASH_MISMATCH,
    );

    // Not past the window, whoever asks.
    let past_the_window = BLOCK_TIME_MS + restore_window_ms() + 1;
    for actor in [&setup.moderator, &setup.owner] {
        let too_late = setup
            .moderate(actor, restore_action(POST, bytes.clone()))
            .await;
        assert_paid_with_code(
            &setup.process_at(&too_late, past_the_window, &transaction),
            DOCUMENT_RESTORE_WINDOW_ELAPSED,
        );
    }

    // None of it brought the post back or touched its record.
    assert_eq!(
        setup.stored_document(POST, post.id(), Some(&transaction)),
        None
    );
    assert_eq!(
        setup
            .post_removal(post.id(), Some(&transaction))
            .map(|removal| removal.restoration),
        Some(None)
    );

    // Once restored, restored: the record says so.
    let restore = setup
        .moderate(&setup.owner, restore_action(POST, bytes.clone()))
        .await;
    assert_success(&setup.process_at(&restore, BLOCK_TIME_MS + restore_window_ms(), &transaction));
    let twice = setup
        .moderate(&setup.moderator, restore_action(POST, bytes))
        .await;
    assert_paid_with_code(
        &setup.process(&twice, &transaction),
        CONTRACT_DOCUMENT_ALREADY_RESTORED,
    );
}

#[tokio::test]
async fn should_refuse_to_restore_a_post_whose_unique_value_another_post_took_meanwhile() {
    let setup = Setup::new_at_with(
        Some(moderators_without_lists()),
        PlatformVersion::latest(),
        |contract| {
            add_document_type(
                contract,
                POST,
                post_schema_with(platform_value!({
                    "indices": [
                        { "name": "byText", "properties": [{ "text": "asc" }], "unique": true },
                    ],
                })),
            )
        },
    )
    .await;
    let text = Value::Text("first!".to_string());

    let transaction = setup.platform.drive.grove.start_transaction();
    let (post, create) = setup
        .create_document_of_type_with(&setup.user, POST, |document| {
            document.set("text", text.clone())
        })
        .await;
    assert_success(&setup.process(&create, &transaction));
    let stored = setup
        .stored_document(POST, post.id(), Some(&transaction))
        .expect("expected the post to be stored");
    let bytes = setup.document_bytes(POST, &stored);

    let delete = setup
        .moderate(&setup.moderator, delete_action(POST, post.id()))
        .await;
    assert_success(&setup.process(&delete, &transaction));

    // The value is free while the post is gone, and a stranger takes it.
    let (_, create_other) = setup
        .create_document_of_type_with(&setup.stranger, POST, |document| {
            document.set("text", text.clone())
        })
        .await;
    assert_success(&setup.process(&create_other, &transaction));

    // The post can not come back beside it, and the refusal is paid.
    let restore = setup
        .moderate(&setup.moderator, restore_action(POST, bytes))
        .await;
    assert_paid_with_code(
        &setup.process_at(&restore, BLOCK_TIME_MS + 1, &transaction),
        DUPLICATE_UNIQUE_INDEX,
    );
    assert_eq!(
        setup.stored_document(POST, post.id(), Some(&transaction)),
        None
    );
}

#[tokio::test]
async fn should_bring_a_post_back_settled_when_its_deletion_window_ran_out_meanwhile() {
    // A restored post is the post as it was, `$updatedAt` included: the type's deletion
    // window, measured from that, may have run out on it while it was gone. The author can
    // still delete it; the moderators can not, until the author edits it.
    let setup = setup_with_a_moderation_window().await;

    let transaction = setup.platform.drive.grove.start_transaction();
    let (post, create) = setup.create_document_of_type(&setup.user, POST).await;
    assert_success(&setup.process(&create, &transaction));
    let stored = setup
        .stored_document(POST, post.id(), Some(&transaction))
        .expect("expected the post to be stored");
    let bytes = setup.document_bytes(POST, &stored);

    let delete = setup
        .moderate(&setup.moderator, delete_action(POST, post.id()))
        .await;
    assert_success(&setup.process(&delete, &transaction));

    // Restored after the deletion window, within the restore window.
    let restored_at = BLOCK_TIME_MS + MODERATION_WINDOW_MS + 1;
    let restore = setup
        .moderate(&setup.moderator, restore_action(POST, bytes))
        .await;
    assert_success(&setup.process_at(&restore, restored_at, &transaction));
    assert_eq!(
        setup.stored_document(POST, post.id(), Some(&transaction)),
        Some(stored.clone())
    );

    let delete_again = setup
        .moderate(&setup.moderator, delete_action(POST, post.id()))
        .await;
    assert_paid_with_code(
        &setup.process_at(&delete_again, restored_at + 1, &transaction),
        DOCUMENT_MODERATION_WINDOW_ELAPSED,
    );
    let own_delete = own_post_deletion(&setup, &setup.user, stored).await;
    assert_success(&setup.process_at(&own_delete, restored_at + 1, &transaction));
}
