//! The identity key limits update through the whole processing pipeline.

use crate::config::{PlatformConfig, PlatformTestConfig};
use crate::execution::check_tx::CheckTxLevel::FirstTimeCheck;
use crate::execution::validation::state_transition::tests::setup_identity_without_adding_it;
use crate::platform_types::platform::PlatformRef;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
use crate::rpc::core::MockCoreRPCLike;
use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
use assert_matches::assert_matches;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::signature::SignatureError;
use dpp::consensus::ConsensusError;
use dpp::dash_to_credits;
use dpp::fee::Credits;
use dpp::identity::accessors::{IdentityGettersV0, IdentitySettersV0};
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::identity_public_key::accessors::v1::IdentityPublicKeyGettersV1;
use dpp::identity::signer::Signer;
use dpp::identity::{Identity, IdentityPublicKey, KeyID, TimestampMillis};
use dpp::serialization::{PlatformSerializable, Signable};
use dpp::state_transition::identity_key_limits_update_transition::methods::IdentityKeyLimitsUpdateTransitionMethodsV0;
use dpp::state_transition::identity_key_limits_update_transition::v0::IdentityKeyLimitsUpdateTransitionV0;
use dpp::state_transition::identity_key_limits_update_transition::IdentityKeyLimitsUpdateTransition;
use dpp::state_transition::proof_result::{
    StateTransitionProofOutcome, StateTransitionProofResult,
};
use dpp::state_transition::{StateTransition, StateTransitionSingleSigned};
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use drive::drive::identity::key::fetch::{IdentityKeysRequest, KeyIDIdentityPublicKeyPairBTreeMap};
use drive::drive::Drive;
use rand::rngs::StdRng;
use rand::SeedableRng;
use simple_signer::signer::SimpleSigner;
use std::cell::Cell;

const IDENTITY_KEY_LIMITS_UPDATE_EMPTY: u32 = 10539;
const INVALID_IDENTITY_PUBLIC_KEY_BUDGET: u32 = 10537;
const PUBLIC_KEY_WITH_LIMITS_CANNOT_UPDATE_KEY_LIMITS: u32 = 20017;
const INVALID_IDENTITY_REVISION: u32 = 40203;
const IDENTITY_PUBLIC_KEY_IS_DISABLED: u32 = 40208;
const MISSING_IDENTITY_PUBLIC_KEY_IDS: u32 = 40209;
const IDENTITY_PUBLIC_KEY_ALREADY_EXPIRED: u32 = 40219;
const IDENTITY_PUBLIC_KEY_LIMIT_NOT_SET: u32 = 40220;
const IDENTITY_PUBLIC_KEY_LIMIT_NOT_RAISED: u32 = 40221;

const MASTER_KEY_ID: KeyID = 0;
const CRITICAL_KEY_ID: KeyID = 1;
const LIMITED_KEY_ID: KeyID = 2;
const HIGH_KEY_ID: KeyID = 3;

const BLOCK_TIME_MS: TimestampMillis = 1_000_000;
const BUDGET: Credits = 1_000_000;

/// An identity with a MASTER key (0), an unlimited CRITICAL key (1), a CRITICAL key with the
/// given limits (2) and an unlimited HIGH key (3). The signer holds all four private keys.
struct Setup {
    platform: TempPlatform<MockCoreRPCLike>,
    identity: Identity,
    signer: SimpleSigner,
    high_key: IdentityPublicKey,
    /// Each transition takes the next identity nonce, whether or not the previous one was
    /// executed or paid for: gaps are allowed, reuse is not.
    next_nonce: Cell<u64>,
}

impl Setup {
    fn new(total_budget: Option<Credits>, expires_at: Option<TimestampMillis>) -> Self {
        Self::new_at(total_budget, expires_at, PlatformVersion::latest())
    }

    fn new_at(
        total_budget: Option<Credits>,
        expires_at: Option<TimestampMillis>,
        platform_version: &PlatformVersion,
    ) -> Self {
        let platform_config = PlatformConfig {
            testing_configs: PlatformTestConfig {
                disable_instant_lock_signature_verification: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let platform = TestPlatformBuilder::new()
            .with_config(platform_config)
            .with_initial_protocol_version(platform_version.protocol_version)
            .build_with_mock_rpc()
            .set_genesis_state();
        let (mut identity, mut signer, _) =
            setup_identity_without_adding_it(958, dash_to_credits!(1));
        let mut rng = StdRng::seed_from_u64(77);
        let (limited_key, limited_private_key) =
            IdentityPublicKey::random_ecdsa_critical_level_authentication_key_with_rng(
                LIMITED_KEY_ID,
                &mut rng,
                platform_version,
            )
            .expect("expected a key pair");
        let limited_key = limited_key.with_limits(total_budget, expires_at);
        signer.add_identity_public_key(limited_key.clone(), limited_private_key);
        identity.add_public_key(limited_key);
        let (high_key, high_private_key) =
            IdentityPublicKey::random_ecdsa_high_level_authentication_key_with_rng(
                HIGH_KEY_ID,
                &mut rng,
                platform_version,
            )
            .expect("expected a key pair");
        signer.add_identity_public_key(high_key.clone(), high_private_key);
        identity.add_public_key(high_key.clone());
        platform
            .drive
            .add_new_identity(
                identity.clone(),
                false,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to add the identity");
        Self {
            platform,
            identity,
            signer,
            high_key,
            next_nonce: Cell::new(1),
        }
    }

    fn take_nonce(&self) -> u64 {
        let nonce = self.next_nonce.get();
        self.next_nonce.set(nonce + 1);
        nonce
    }

    /// A limits update of `key_id`, built and signed the way an SDK would
    async fn update(
        &self,
        signing_key_id: KeyID,
        key_id: KeyID,
        total_budget: Option<Credits>,
        expires_at: Option<TimestampMillis>,
    ) -> StateTransition {
        self.try_update(signing_key_id, key_id, total_budget, expires_at)
            .await
            .expect("expected to build the transition")
    }

    async fn try_update(
        &self,
        signing_key_id: KeyID,
        key_id: KeyID,
        total_budget: Option<Credits>,
        expires_at: Option<TimestampMillis>,
    ) -> Result<StateTransition, ProtocolError> {
        IdentityKeyLimitsUpdateTransition::try_from_identity_with_signer(
            &self.identity,
            &signing_key_id,
            key_id,
            total_budget,
            expires_at,
            self.take_nonce(),
            0,
            &self.signer,
            PlatformVersion::latest(),
            None,
        )
        .await
    }

    /// A limits update built by hand, so tests can claim any revision or signer
    async fn update_signed_by(
        &self,
        signing_key: &IdentityPublicKey,
        revision: u64,
        key_id: KeyID,
        total_budget: Option<Credits>,
        expires_at: Option<TimestampMillis>,
    ) -> StateTransition {
        let mut transition: StateTransition = IdentityKeyLimitsUpdateTransitionV0 {
            identity_id: self.identity.id(),
            revision,
            nonce: self.take_nonce(),
            key_id,
            total_budget,
            expires_at,
            user_fee_increase: 0,
            signature_public_key_id: signing_key.id(),
            signature: Default::default(),
        }
        .into();
        let signable_bytes = transition
            .signable_bytes()
            .expect("expected signable bytes");
        let signature = self
            .signer
            .sign(signing_key, &signable_bytes)
            .await
            .expect("expected to sign");
        transition.set_signature(signature);
        transition
    }

    fn process(
        &self,
        transition: &StateTransition,
        tx: &drive::grovedb::Transaction,
    ) -> StateTransitionExecutionResult {
        let version = self
            .platform
            .state
            .load()
            .current_platform_version()
            .expect("version");
        let state = self.platform.state.load();
        let result = self
            .platform
            .platform
            .process_raw_state_transitions(
                &vec![transition
                    .serialize_to_bytes()
                    .expect("expected to serialize")],
                &state,
                &BlockInfo {
                    time_ms: BLOCK_TIME_MS,
                    ..Default::default()
                },
                tx,
                version,
                false,
                None,
            )
            .expect("expected to process the state transition");
        result.execution_results()[0].clone()
    }

    fn stored_key(&self, key_id: KeyID, tx: &drive::grovedb::Transaction) -> IdentityPublicKey {
        self.platform
            .drive
            .fetch_identity_keys::<KeyIDIdentityPublicKeyPairBTreeMap>(
                IdentityKeysRequest::new_specific_key_query(self.identity.id().as_bytes(), key_id),
                Some(tx),
                PlatformVersion::latest(),
            )
            .expect("expected to fetch the key")
            .remove(&key_id)
            .expect("expected the key")
    }

    fn remaining_budget(&self, key_id: KeyID, tx: &drive::grovedb::Transaction) -> Option<Credits> {
        self.platform
            .drive
            .fetch_identity_key_remaining_budget(
                self.identity.id().to_buffer(),
                key_id,
                Some(tx),
                PlatformVersion::latest(),
            )
            .expect("expected to fetch the remaining budget")
    }

    fn revision(&self, tx: &drive::grovedb::Transaction) -> Option<u64> {
        self.platform
            .drive
            .fetch_identity_revision(
                self.identity.id().to_buffer(),
                true,
                Some(tx),
                PlatformVersion::latest(),
            )
            .expect("expected to fetch the revision")
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

#[tokio::test]
async fn should_raise_the_budget_of_a_spent_key_so_it_can_sign_again() {
    let setup = Setup::new(Some(BUDGET), None);
    let version = PlatformVersion::latest();
    let tx = setup.platform.drive.grove.start_transaction();
    setup
        .platform
        .drive
        .deduct_from_identity_key_budget(
            setup.identity.id().to_buffer(),
            LIMITED_KEY_ID,
            BUDGET,
            Some(&tx),
            version,
        )
        .expect("expected to spend the whole budget");
    assert_eq!(setup.remaining_budget(LIMITED_KEY_ID, &tx), Some(0));

    let transition = setup
        .update(MASTER_KEY_ID, LIMITED_KEY_ID, Some(3 * BUDGET), None)
        .await;
    assert_success(&setup.process(&transition, &tx));

    let key = setup.stored_key(LIMITED_KEY_ID, &tx);
    assert_eq!(key.total_budget(), Some(3 * BUDGET));
    assert_eq!(key.expires_at(), None);
    assert_eq!(
        setup.remaining_budget(LIMITED_KEY_ID, &tx),
        Some(2 * BUDGET),
        "what was spent stays spent"
    );
    assert_eq!(
        setup.revision(&tx),
        Some(1),
        "the identity revision is bumped"
    );

    // The key is usable again: its transitions are admitted to the mempool.
    let transition = setup
        .update(LIMITED_KEY_ID, HIGH_KEY_ID, Some(1), None)
        .await;
    assert!(
        setup
            .check_tx(&transition)
            .iter()
            .all(|error| error.code() != 20015),
        "a topped up key is no longer refused as exhausted"
    );
}

#[tokio::test]
async fn should_revive_an_expired_key_by_extending_its_expiry() {
    let setup = Setup::new(Some(BUDGET), Some(BLOCK_TIME_MS - 1));
    let tx = setup.platform.drive.grove.start_transaction();

    let transition = setup
        .update(MASTER_KEY_ID, LIMITED_KEY_ID, None, Some(BLOCK_TIME_MS * 2))
        .await;
    assert_success(&setup.process(&transition, &tx));

    let key = setup.stored_key(LIMITED_KEY_ID, &tx);
    assert_eq!(key.expires_at(), Some(BLOCK_TIME_MS * 2));
    assert_eq!(key.total_budget(), Some(BUDGET), "the budget is untouched");
    assert_eq!(setup.remaining_budget(LIMITED_KEY_ID, &tx), Some(BUDGET));
}

#[tokio::test]
async fn should_raise_both_limits_at_once() {
    let setup = Setup::new(Some(BUDGET), Some(BLOCK_TIME_MS + 1));
    let tx = setup.platform.drive.grove.start_transaction();

    let transition = setup
        .update(
            MASTER_KEY_ID,
            LIMITED_KEY_ID,
            Some(BUDGET + 1),
            Some(BLOCK_TIME_MS + 2),
        )
        .await;
    assert_success(&setup.process(&transition, &tx));

    let key = setup.stored_key(LIMITED_KEY_ID, &tx);
    assert_eq!(key.total_budget(), Some(BUDGET + 1));
    assert_eq!(key.expires_at(), Some(BLOCK_TIME_MS + 2));
    assert_eq!(
        setup.remaining_budget(LIMITED_KEY_ID, &tx),
        Some(BUDGET + 1)
    );
}

#[tokio::test]
async fn should_refuse_to_top_up_a_key_that_stays_expired() {
    let setup = Setup::new(Some(BUDGET), Some(BLOCK_TIME_MS - 1));
    let tx = setup.platform.drive.grove.start_transaction();

    let transition = setup
        .update(MASTER_KEY_ID, LIMITED_KEY_ID, Some(BUDGET + 1), None)
        .await;
    assert_paid_with_code(
        &setup.process(&transition, &tx),
        IDENTITY_PUBLIC_KEY_ALREADY_EXPIRED,
    );
    assert_eq!(
        setup.stored_key(LIMITED_KEY_ID, &tx).total_budget(),
        Some(BUDGET)
    );

    // An extension that ends in the past is refused for the same reason
    let transition = setup
        .update(MASTER_KEY_ID, LIMITED_KEY_ID, None, Some(BLOCK_TIME_MS))
        .await;
    assert_paid_with_code(
        &setup.process(&transition, &tx),
        IDENTITY_PUBLIC_KEY_ALREADY_EXPIRED,
    );
}

#[tokio::test]
async fn should_refuse_a_value_that_does_not_raise_the_limit() {
    let setup = Setup::new(Some(BUDGET), Some(BLOCK_TIME_MS * 2));
    let tx = setup.platform.drive.grove.start_transaction();

    let transition = setup
        .update(MASTER_KEY_ID, LIMITED_KEY_ID, Some(BUDGET), None)
        .await;
    assert_paid_with_code(
        &setup.process(&transition, &tx),
        IDENTITY_PUBLIC_KEY_LIMIT_NOT_RAISED,
    );

    let transition = setup
        .update(
            MASTER_KEY_ID,
            LIMITED_KEY_ID,
            None,
            Some(BLOCK_TIME_MS * 2 - 1),
        )
        .await;
    assert_paid_with_code(
        &setup.process(&transition, &tx),
        IDENTITY_PUBLIC_KEY_LIMIT_NOT_RAISED,
    );
    assert_eq!(setup.remaining_budget(LIMITED_KEY_ID, &tx), Some(BUDGET));
}

#[tokio::test]
async fn should_refuse_to_add_a_limit_the_key_does_not_have() {
    // A key with a budget only can not be given an expiry
    let setup = Setup::new(Some(BUDGET), None);
    let tx = setup.platform.drive.grove.start_transaction();
    let transition = setup
        .update(MASTER_KEY_ID, LIMITED_KEY_ID, None, Some(BLOCK_TIME_MS * 2))
        .await;
    assert_paid_with_code(
        &setup.process(&transition, &tx),
        IDENTITY_PUBLIC_KEY_LIMIT_NOT_SET,
    );

    // A key with an expiry only can not be given a budget
    let setup = Setup::new(None, Some(BLOCK_TIME_MS * 2));
    let tx = setup.platform.drive.grove.start_transaction();
    let transition = setup
        .update(MASTER_KEY_ID, LIMITED_KEY_ID, Some(BUDGET), None)
        .await;
    assert_paid_with_code(
        &setup.process(&transition, &tx),
        IDENTITY_PUBLIC_KEY_LIMIT_NOT_SET,
    );

    // A version 0 key has no limits at all
    let transition = setup
        .update(MASTER_KEY_ID, CRITICAL_KEY_ID, Some(BUDGET), None)
        .await;
    assert_paid_with_code(
        &setup.process(&transition, &tx),
        IDENTITY_PUBLIC_KEY_LIMIT_NOT_SET,
    );
}

#[tokio::test]
async fn should_refuse_a_missing_or_disabled_key() {
    let setup = Setup::new(Some(BUDGET), None);
    let version = PlatformVersion::latest();
    let tx = setup.platform.drive.grove.start_transaction();

    let transition = setup.update(MASTER_KEY_ID, 9, Some(BUDGET + 1), None).await;
    assert_paid_with_code(
        &setup.process(&transition, &tx),
        MISSING_IDENTITY_PUBLIC_KEY_IDS,
    );

    setup
        .platform
        .drive
        .disable_identity_keys(
            setup.identity.id().to_buffer(),
            vec![LIMITED_KEY_ID],
            1,
            &BlockInfo::default(),
            true,
            Some(&tx),
            version,
        )
        .expect("expected to disable the key");
    let transition = setup
        .update(MASTER_KEY_ID, LIMITED_KEY_ID, Some(BUDGET + 1), None)
        .await;
    assert_paid_with_code(
        &setup.process(&transition, &tx),
        IDENTITY_PUBLIC_KEY_IS_DISABLED,
    );
}

#[tokio::test]
async fn should_refuse_a_stale_revision_before_reading_the_key() {
    let setup = Setup::new(Some(BUDGET), None);
    let tx = setup.platform.drive.grove.start_transaction();
    let master_key = setup.identity.public_keys()[&MASTER_KEY_ID].clone();

    // The identity is at revision 0, so the transition must claim 1
    let transition = setup
        .update_signed_by(&master_key, 2, LIMITED_KEY_ID, Some(BUDGET + 1), None)
        .await;
    assert_paid_with_code(&setup.process(&transition, &tx), INVALID_IDENTITY_REVISION);
    assert_eq!(
        setup.stored_key(LIMITED_KEY_ID, &tx).total_budget(),
        Some(BUDGET)
    );
}

#[tokio::test]
async fn should_refuse_an_update_that_changes_nothing_or_zeroes_the_budget_unpaid() {
    let setup = Setup::new(Some(BUDGET), None);
    let tx = setup.platform.drive.grove.start_transaction();

    let transition = setup
        .update(MASTER_KEY_ID, LIMITED_KEY_ID, None, None)
        .await;
    assert_unpaid_with_code(
        &setup.process(&transition, &tx),
        IDENTITY_KEY_LIMITS_UPDATE_EMPTY,
    );

    let transition = setup
        .update(MASTER_KEY_ID, LIMITED_KEY_ID, Some(0), None)
        .await;
    assert_unpaid_with_code(
        &setup.process(&transition, &tx),
        INVALID_IDENTITY_PUBLIC_KEY_BUDGET,
    );
}

#[tokio::test]
async fn should_accept_an_unlimited_critical_signer_and_refuse_limited_or_lower_ones() {
    let setup = Setup::new(Some(BUDGET), None);
    let tx = setup.platform.drive.grove.start_transaction();

    // A CRITICAL key without limits may raise limits
    let transition = setup
        .update(CRITICAL_KEY_ID, LIMITED_KEY_ID, Some(BUDGET + 1), None)
        .await;
    assert_success(&setup.process(&transition, &tx));

    // A key with limits may not, not even its own
    let transition = setup
        .update(LIMITED_KEY_ID, LIMITED_KEY_ID, Some(BUDGET + 2), None)
        .await;
    let execution = setup.process(&transition, &tx);
    assert_unpaid_with_code(&execution, PUBLIC_KEY_WITH_LIMITS_CANNOT_UPDATE_KEY_LIMITS);
    assert_eq!(
        setup.stored_key(LIMITED_KEY_ID, &tx).total_budget(),
        Some(BUDGET + 1)
    );

    // The builder refuses a HIGH key up front, and consensus refuses one that was signed by hand
    let result = setup
        .try_update(HIGH_KEY_ID, LIMITED_KEY_ID, Some(BUDGET + 2), None)
        .await;
    assert_matches!(
        result,
        Err(ProtocolError::InvalidSignaturePublicKeySecurityLevelError(
            _
        ))
    );
    let transition = setup
        .update_signed_by(&setup.high_key, 2, LIMITED_KEY_ID, Some(BUDGET + 2), None)
        .await;
    assert_matches!(
        setup.process(&transition, &tx),
        StateTransitionExecutionResult::UnpaidConsensusError(ConsensusError::SignatureError(
            SignatureError::InvalidSignaturePublicKeySecurityLevelError(_)
        ))
    );
}

#[tokio::test]
async fn should_keep_a_limited_signer_out_of_the_mempool() {
    let setup = Setup::new(Some(BUDGET), None);

    let transition = setup
        .update(MASTER_KEY_ID, LIMITED_KEY_ID, Some(BUDGET + 1), None)
        .await;
    assert!(setup.check_tx(&transition).is_empty());

    let transition = setup
        .update(LIMITED_KEY_ID, LIMITED_KEY_ID, Some(BUDGET + 1), None)
        .await;
    let errors = setup.check_tx(&transition);
    assert!(
        matches!(errors.as_slice(), [error] if error.code() == PUBLIC_KEY_WITH_LIMITS_CANNOT_UPDATE_KEY_LIMITS),
        "{errors:?}"
    );
}

#[tokio::test]
async fn should_prove_the_rewritten_key_and_the_revision() {
    let setup = Setup::new(Some(BUDGET), Some(BLOCK_TIME_MS * 2));
    let version = PlatformVersion::latest();
    let tx = setup.platform.drive.grove.start_transaction();

    let transition = setup
        .update(
            MASTER_KEY_ID,
            LIMITED_KEY_ID,
            Some(BUDGET * 2),
            Some(BLOCK_TIME_MS * 3),
        )
        .await;
    assert_success(&setup.process(&transition, &tx));
    setup
        .platform
        .drive
        .grove
        .commit_transaction(tx)
        .unwrap()
        .expect("expected to commit");

    let proof = setup
        .platform
        .drive
        .prove_state_transition(&transition, None, version)
        .expect("expected to prove")
        .into_data()
        .expect("expected proof bytes");
    let (root_hash, outcome) = Drive::verify_state_transition_was_executed_with_proof(
        &transition,
        &BlockInfo::default(),
        &proof,
        &|_| Ok(None),
        version,
    )
    .expect("expected the proof to verify");
    assert_eq!(
        root_hash,
        setup
            .platform
            .drive
            .grove
            .root_hash(None, &version.drive.grove_version)
            .unwrap()
            .expect("expected a root hash")
    );
    let StateTransitionProofOutcome::ExecutionProved(
        StateTransitionProofResult::VerifiedPartialIdentity(identity),
    ) = outcome
    else {
        panic!("expected the execution to be proved, got {outcome:?}");
    };
    assert_eq!(identity.revision, Some(1));
    let key = &identity.loaded_public_keys[&LIMITED_KEY_ID];
    assert_eq!(key.total_budget(), Some(BUDGET * 2));
    assert_eq!(key.expires_at(), Some(BLOCK_TIME_MS * 3));

    // A proof of the state before the update does not verify for it
    let mut stale = setup.identity.clone();
    stale.set_revision(1);
    let other = setup
        .update_signed_by(
            &setup.identity.public_keys()[&MASTER_KEY_ID].clone(),
            1,
            LIMITED_KEY_ID,
            Some(BUDGET * 3),
            None,
        )
        .await;
    assert!(
        Drive::verify_state_transition_was_executed_with_proof(
            &other,
            &BlockInfo::default(),
            &proof,
            &|_| Ok(None),
            version,
        )
        .is_err(),
        "a different total budget must not verify against the proof"
    );
}

#[tokio::test]
async fn should_not_be_active_before_protocol_version_14() {
    let platform_version = PlatformVersion::get(13).expect("protocol version 13");
    let setup = Setup::new_at(None, None, platform_version);
    let tx = setup.platform.drive.grove.start_transaction();

    let transition = setup
        .update(MASTER_KEY_ID, CRITICAL_KEY_ID, Some(BUDGET), None)
        .await;
    // Below the activation version the transition is refused at decode time by the
    // `active_version_range` check, before any consensus validation.
    assert_matches!(
        setup.process(&transition, &tx),
        StateTransitionExecutionResult::InternalError(message)
            if message.contains("IdentityKeyLimitsUpdate") && message.contains("not active")
    );
}
