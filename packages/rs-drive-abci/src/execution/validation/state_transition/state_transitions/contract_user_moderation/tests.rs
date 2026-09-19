//! Contract moderation through the whole processing pipeline: the transition that edits a
//! contract's banlist and suspension list, and the document gate that enforces them.

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
use dpp::data_contract::config::moderation::{
    ContractModerationConfig, ContractModerationList, ContractModerationListStatus,
    ContractModerationListStatuses, ContractModerationStatus, ContractModerators,
};
use dpp::data_contract::config::v2::DataContractConfigGettersV2;
use dpp::data_contract::document_type::random_document::{
    CreateRandomDocument, DocumentFieldFillSize, DocumentFieldFillType,
};
use dpp::data_contract::DataContract;
use dpp::document::Document;
use dpp::document::DocumentV0Setters;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::{Identity, IdentityPublicKey, KeyID, TimestampMillis};
use dpp::platform_value::{Bytes32, Identifier};
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
use dpp::state_transition::proof_result::{
    StateTransitionProofOutcome, StateTransitionProofResult,
};
use dpp::state_transition::StateTransition;
use dpp::tests::fixtures::get_data_contract_fixture;
use dpp::tests::json_document::json_document_to_contract;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::grovedb::Transaction;
use drive::util::storage_flags::StorageFlags;
use rand::rngs::StdRng;
use rand::SeedableRng;
use simple_signer::signer::SimpleSigner;
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;

const DATA_CONTRACT_NOT_PRESENT: u32 = 10400;
const CONTRACT_MODERATION_SELF_TARGET: u32 = 10463;
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

const CRITICAL_KEY_ID: KeyID = 1;
const DOCUMENT_TYPE: &str = "niceDocument";
const BLOCK_TIME_MS: TimestampMillis = 1_000_000;

const BOTH: [ContractModerationList; 2] = [
    ContractModerationList::Banlist,
    ContractModerationList::Suspensions,
];

/// A registered identity with its signer and its CRITICAL key, plus the nonces the tests hand
/// out: gaps are allowed, reuse is not.
struct Actor {
    identity: Identity,
    signer: SimpleSigner,
    key: IdentityPublicKey,
    next_identity_nonce: Cell<IdentityNonce>,
    next_contract_nonce: Cell<IdentityNonce>,
}

impl Actor {
    fn new(platform: &mut TempPlatform<MockCoreRPCLike>, seed: u64) -> Self {
        let (identity, signer, key) = setup_identity(platform, seed, dash_to_credits!(10));
        Self {
            identity,
            signer,
            key,
            next_identity_nonce: Cell::new(1),
            next_contract_nonce: Cell::new(1),
        }
    }

    fn id(&self) -> Identifier {
        self.identity.id()
    }

    fn identity_nonce(&self) -> IdentityNonce {
        let nonce = self.next_identity_nonce.get();
        self.next_identity_nonce.set(nonce + 1);
        nonce
    }

    fn contract_nonce(&self) -> IdentityNonce {
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
    ContractModerationConfig {
        banlist,
        suspensions,
        moderators: ContractModerators::OwnerAndIdentities([moderator].into_iter().collect()),
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
            if let ContractModerators::OwnerAndIdentities(ids) = &mut moderation.moderators {
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
        let platform_version = PlatformVersion::latest();
        let document_type = self
            .contract
            .document_type_for_name(DOCUMENT_TYPE)
            .expect("expected the document type");
        // The borrow ends before the await below (clippy::await_holding_refcell_ref).
        let (entropy, document) = {
            let mut rng = self.rng.borrow_mut();
            let entropy = Bytes32::random_with_rng(&mut rng);
            let document = document_type
                .random_document_with_identifier_and_entropy(
                    &mut rng,
                    actor.id(),
                    entropy,
                    DocumentFieldFillType::FillIfNotRequired,
                    DocumentFieldFillSize::MinDocumentFillSize,
                    platform_version,
                )
                .expect("expected a random document");
            (entropy, document)
        };
        let transition = BatchTransition::new_document_creation_transition_from_document(
            document.clone(),
            document_type,
            entropy.0,
            &actor.key,
            actor.contract_nonce(),
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
        self.platform
            .drive
            .fetch_contract_moderation_status(
                self.contract.id(),
                identity_id,
                &BOTH,
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
        match outcome {
            StateTransitionProofOutcome::AffectedState(
                StateTransitionProofResult::VerifiedContractModerationListStatuses(_, _, status),
            ) => status,
            other => panic!("expected a moderation status, got {other:?}"),
        }
    }
}

fn assert_success(execution: &StateTransitionExecutionResult) {
    assert!(
        matches!(
            execution,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        ),
        "expected a successful execution, got {execution:?}"
    );
}

fn assert_paid_with_code(execution: &StateTransitionExecutionResult, code: u32) {
    assert!(
        matches!(execution, StateTransitionExecutionResult::PaidConsensusError { error, .. } if error.code() == code),
        "expected a paid error {code}, got {execution:?}"
    );
}

fn assert_unpaid_with_code(execution: &StateTransitionExecutionResult, code: u32) {
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

fn ban_action(identity_id: Identifier) -> ContractUserModerationAction {
    ContractUserModerationAction::Ban { identity_id }
}

fn unban_action(identity_id: Identifier) -> ContractUserModerationAction {
    ContractUserModerationAction::Unban { identity_id }
}

fn suspend_action(identity_id: Identifier, until: TimestampMillis) -> ContractUserModerationAction {
    ContractUserModerationAction::Suspend { identity_id, until }
}

fn unsuspend_action(identity_id: Identifier) -> ContractUserModerationAction {
    ContractUserModerationAction::Unsuspend { identity_id }
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
            ContractModerationListStatus::Banlist { banned: true },
            ContractModerationListStatus::Suspensions {
                suspended_until: None
            },
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
        ContractModerationListStatuses(vec![ContractModerationListStatus::Banlist {
            banned: false
        }])
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
            suspended_until: Some(until)
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
    assert!(setup.status(user_id, Some(&transaction)).banned);

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
        setup.status(user_id, Some(&transaction)).suspended_until,
        Some(BLOCK_TIME_MS + 50)
    );

    // A ban supersedes the suspension.
    let ban = setup.moderate(&setup.owner, ban_action(user_id)).await;
    assert_success(&setup.process(&ban, &transaction));
    assert_eq!(
        setup.status(user_id, Some(&transaction)),
        ContractModerationStatus {
            banned: true,
            suspended_until: None,
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
    assert_paid_with_code(
        &setup.process(&suspend_banned, &transaction),
        CONTRACT_USER_ALREADY_BANNED,
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
            suspended_until: None
        }])
    );
    assert!(setup.status(user_id, None).banned);
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
                moderators: ContractModerators::OwnerAndIdentities(
                    [setup.moderator.id(), unknown].into_iter().collect(),
                ),
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
                moderators: ContractModerators::OwnerAndIdentities(
                    [setup.moderator.id(), setup.user.id()]
                        .into_iter()
                        .collect(),
                ),
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
                moderators: ContractModerators::OwnerAndIdentities(
                    [setup.moderator.id(), setup.user.id()]
                        .into_iter()
                        .collect(),
                ),
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
        moderators: ContractModerators::OwnerAndIdentities(
            [THE_OWNER, THE_MODERATOR].into_iter().collect(),
        ),
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
            ContractModerationListStatus::Banlist { banned: true },
            ContractModerationListStatus::Suspensions {
                suspended_until: None
            },
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
    card.set("attack", 4.into());
    card.set("defense", 7.into());
    let create = BatchTransition::new_document_creation_transition_from_document(
        card.clone(),
        card_type,
        entropy.0,
        &seller.key,
        seller.contract_nonce(),
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
