//! Drive-backed tests for the `IdentityCreateFromShieldedPool` transition. Structural checks are
//! covered in the dpp `validate_structure` tests and op-level checks in the drive converter tests.
//!
//! Covered here:
//! - The two identity-creation state checks in `validate_state` (the success/failure branch that
//!   mirrors `IdentityCreateFromAddresses`): an identity already existing at the derived id is a
//!   free rejection, while a public-key hash already registered to another identity is NOT a free
//!   reject — the spend is finalized and the value is credited to
//!   `send_to_address_on_creation_failure` minus a penalty via a fallback `UnshieldAction`.
//! - Sum-tree credit conservation on BOTH the success path (pool->new-identity) and the failure
//!   path (the fallback Unshield, pool->address minus penalty): the converter ops applied through a
//!   real Drive keep `calculate_total_credits_balance().ok()` balanced (the end-of-block invariant
//!   that halts the chain) — the regression guard for the `AddToSystemCredits` over-mint.
//! - The execute -> prove -> verify result-proof roundtrip for the success path (synthetic action,
//!   no Halo 2 proving): the regression guard for the devnet bug where the verifier's absence-proof
//!   verification choked on the merged {nullifiers, full-identity} query ("terminal keys are not
//!   supported with unbounded ranges"), so every honest type-20 result proof failed client-side.
//!
//! The full build->prove->execute->prove/verify happy path (real Orchard proof) is deferred to the
//! shared shielded-strategy harness, a pre-existing repo-wide TODO that is disabled for every
//! shielded transition (the shielded `OperationType` build handlers are commented out in
//! `strategy.rs`).

use super::state::v0::IdentityCreateFromShieldedPoolStateTransitionStateValidationV0;
use super::transform_into_action::v0::IdentityCreateFromShieldedPoolStateTransitionTransformIntoActionValidationV0;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::state_transitions::test_helpers::{
    insert_anchor_into_state, insert_dummy_encrypted_notes, set_pool_total_balance, setup_platform,
};
use crate::platform_types::platform::PlatformRef;
use assert_matches::assert_matches;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::ConsensusError;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{Identity, IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::BinaryData;
use dpp::shielded::SerializedAction;
use dpp::state_transition::public_key_in_creation::v0::IdentityPublicKeyInCreationV0;
use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use dpp::state_transition::state_transitions::shielded::identity_create_from_shielded_pool_transition::derive_identity_id_from_actions;
use dpp::state_transition::state_transitions::shielded::identity_create_from_shielded_pool_transition::v0::IdentityCreateFromShieldedPoolTransitionV0;
use dpp::state_transition::state_transitions::shielded::identity_create_from_shielded_pool_transition::IdentityCreateFromShieldedPoolTransition;
use dpp::version::{DefaultForPlatformVersion, PlatformVersion};
use drive::state_transition_action::StateTransitionAction;
use rand::SeedableRng;

const DENOMINATION: u64 = 10_000_000_000;
const ANCHOR: [u8; 32] = [7u8; 32];
/// Fallback platform address credited (minus a penalty) when the unique-public-key-hash state
/// check fails. On that failure the transition produces an `UnshieldAction` paying this address.
const FALLBACK_ADDRESS: dpp::address_funds::PlatformAddress =
    dpp::address_funds::PlatformAddress::P2pkh([0x5C; 20]);

fn action(nullifier_seed: u8) -> SerializedAction {
    SerializedAction {
        nullifier: [nullifier_seed; 32],
        rk: [2u8; 32],
        cmx: [3u8; 32],
        encrypted_note: vec![4u8; 216],
        cv_net: [5u8; 32],
        spend_auth_sig: [6u8; 64],
    }
}

fn master_key() -> IdentityPublicKeyInCreation {
    IdentityPublicKeyInCreation::V0(IdentityPublicKeyInCreationV0 {
        id: 0,
        key_type: KeyType::ECDSA_SECP256K1,
        purpose: Purpose::AUTHENTICATION,
        security_level: SecurityLevel::MASTER,
        contract_bounds: None,
        read_only: false,
        data: BinaryData::new(vec![0u8; 33]),
        signature: BinaryData::default(),
    })
}

fn transition(
    public_keys: Vec<IdentityPublicKeyInCreation>,
    actions: Vec<SerializedAction>,
) -> IdentityCreateFromShieldedPoolTransition {
    let identity_id = derive_identity_id_from_actions(&actions);
    IdentityCreateFromShieldedPoolTransition::V0(IdentityCreateFromShieldedPoolTransitionV0 {
        public_keys,
        denomination: DENOMINATION,
        actions,
        anchor: ANCHOR,
        proof: vec![0u8; 100],
        binding_signature: [0u8; 64],
        send_to_address_on_creation_failure: FALLBACK_ADDRESS,
        identity_id,
    })
}

/// Build the optimistic SUCCESS action via `transform_into_action_v0` (the stateless + pool/anchor/
/// nullifier/balance checks). Used by the `validate_state` tests, which then run the success/failure
/// identity-creation branch against it. Requires the pool state to already be seeded so the
/// transform succeeds.
fn build_success_action(
    platform: &crate::test::helpers::setup::TempPlatform<crate::rpc::core::MockCoreRPCLike>,
    st: &IdentityCreateFromShieldedPoolTransition,
    execution_context: &mut StateTransitionExecutionContext,
    platform_version: &PlatformVersion,
) -> drive::state_transition_action::shielded::identity_create_from_shielded_pool::IdentityCreateFromShieldedPoolTransitionAction
{
    let result = st
        .transform_into_action_v0(&platform.drive, execution_context, None, platform_version)
        .expect("transform should not error");
    assert!(
        result.is_valid_with_data(),
        "transform should build a valid success action; got {:?}",
        result.errors
    );
    match result.into_data().expect("success action data") {
        StateTransitionAction::IdentityCreateFromShieldedPoolAction(action) => action,
        other => panic!("expected IdentityCreateFromShieldedPoolAction, got {other:?}"),
    }
}

#[test]
fn validate_state_rejects_when_identity_already_exists_at_derived_id() {
    let platform_version = PlatformVersion::latest();
    let platform = setup_platform();

    // Seed enough pool state (balance, the anchor, the minimum note count) that `transform` gets past
    // the pool/anchor/nullifier/balance checks and builds the success action; `validate_state` then
    // runs the identity-creation state checks.
    set_pool_total_balance(&platform, DENOMINATION * 10);
    insert_anchor_into_state(&platform, &ANCHOR);
    let min_notes = platform_version
        .drive_abci
        .validation_and_processing
        .event_constants
        .minimum_pool_notes_for_outgoing;
    insert_dummy_encrypted_notes(&platform, min_notes.max(1));

    let actions = vec![action(1), action(2)];
    let derived_id = derive_identity_id_from_actions(&actions);

    // Pre-create an identity AT the derived id (with valid random keys, so add_new_identity
    // succeeds) — the (cryptographically unreachable, but defended) collision case.
    let (random_identity, _): (Identity, Vec<(IdentityPublicKey, [u8; 32])>) =
        Identity::random_identity_with_main_keys_with_private_key(
            2,
            &mut rand::rngs::StdRng::seed_from_u64(7),
            platform_version,
        )
        .expect("random identity");
    let existing = Identity::new_with_id_and_keys(
        derived_id,
        random_identity.public_keys().clone(),
        platform_version,
    )
    .expect("identity at derived id");
    platform
        .drive
        .add_new_identity(
            existing,
            false,
            &BlockInfo::default(),
            true,
            None,
            platform_version,
        )
        .expect("should add the pre-existing identity");

    let st = transition(vec![master_key()], actions);
    let mut execution_context =
        StateTransitionExecutionContext::default_for_platform_version(platform_version)
            .expect("execution context");
    let action = build_success_action(&platform, &st, &mut execution_context, platform_version);

    let platform_state = platform.state.load();
    let platform_ref = PlatformRef {
        drive: &platform.drive,
        state: &platform_state,
        config: &platform.config,
        core_rpc: &platform.core_rpc,
    };
    let result = st
        .validate_state_v0(
            &platform_ref,
            action,
            &mut execution_context,
            None,
            platform_version,
        )
        .expect("validate_state should not error");

    // The identity-exists case is a FREE rejection (no chargeable fallback): an attacker cannot
    // choose a colliding derived id, so there is no spend to finalize.
    assert!(!result.is_valid(), "expected a consensus rejection");
    assert!(
        !result.has_data(),
        "an identity-id collision must stay a free rejection — no fallback action"
    );
    assert_matches!(
        result.errors.as_slice(),
        [ConsensusError::StateError(
            StateError::IdentityAlreadyExistsError(_)
        )],
        "got: {:?}",
        result.errors
    );
}

#[test]
fn validate_state_returns_unshield_fallback_on_duplicate_key_hash() {
    let platform_version = PlatformVersion::latest();
    let platform = setup_platform();

    set_pool_total_balance(&platform, DENOMINATION * 10);
    insert_anchor_into_state(&platform, &ANCHOR);
    let min_notes = platform_version
        .drive_abci
        .validation_and_processing
        .event_constants
        .minimum_pool_notes_for_outgoing;
    insert_dummy_encrypted_notes(&platform, min_notes.max(1));

    // Pre-register a different identity that owns an ECDSA_SECP256K1 key.
    let (existing_identity, keys_with_private): (Identity, Vec<(IdentityPublicKey, [u8; 32])>) =
        Identity::random_identity_with_main_keys_with_private_key(
            3,
            &mut rand::rngs::StdRng::seed_from_u64(50),
            platform_version,
        )
        .expect("random identity");
    let existing_key = keys_with_private
        .iter()
        .find(|(k, _)| k.key_type() == KeyType::ECDSA_SECP256K1)
        .map(|(k, _)| k.clone())
        .expect("an ECDSA_SECP256K1 key");
    platform
        .drive
        .add_new_identity(
            existing_identity,
            false,
            &BlockInfo::default(),
            true,
            None,
            platform_version,
        )
        .expect("should add the key-owning identity");

    // The new identity's key DUPLICATES that already-registered key's hash. Its derived id (from
    // these nullifiers) is free, so the identity-absence check passes and the unique-key-hash check
    // is the one that fails — triggering the chargeable fallback rather than a free rejection.
    let dup_key = IdentityPublicKeyInCreation::V0(IdentityPublicKeyInCreationV0 {
        id: 0,
        key_type: existing_key.key_type(),
        purpose: existing_key.purpose(),
        security_level: existing_key.security_level(),
        contract_bounds: None,
        read_only: false,
        data: existing_key.data().clone(),
        signature: BinaryData::default(),
    });

    let st = transition(vec![dup_key], vec![action(10), action(11)]);
    let mut execution_context =
        StateTransitionExecutionContext::default_for_platform_version(platform_version)
            .expect("execution context");
    let action = build_success_action(&platform, &st, &mut execution_context, platform_version);
    let expected_current_total_balance = action.current_total_balance();

    let platform_state = platform.state.load();
    let platform_ref = PlatformRef {
        drive: &platform.drive,
        state: &platform_state,
        config: &platform.config,
        core_rpc: &platform.core_rpc,
    };
    let result = st
        .validate_state_v0(
            &platform_ref,
            action,
            &mut execution_context,
            None,
            platform_version,
        )
        .expect("validate_state should not error");

    // The fallback path is NOT a free rejection: it returns a result that CARRIES an action (the
    // chargeable `UnshieldAction` — the spend is finalized, crediting the fallback address minus a
    // penalty) WITH the unique-key-hash collision errors attached for surfacing to the submitter.
    // (Errors-present means `is_valid()` is false, exactly like `IdentityCreateFromAddresses`'s
    // `new_with_data_and_errors` bump action; the processor still books the carried action.)
    assert!(
        result.has_data(),
        "duplicate-key-hash must return a chargeable fallback action, not a free reject; got {:?}",
        result.errors
    );
    assert!(
        !result.is_valid(),
        "the fallback action must still carry the collision errors for the submitter"
    );
    // The latest platform version dispatches the v1 unique-key-hash check, which reports the
    // collision as a StateError (v0 reported it as a BasicError).
    assert_matches!(
        result.errors.as_slice(),
        [ConsensusError::StateError(
            StateError::DuplicatedIdentityPublicKeyIdStateError(_)
        )],
        "expected the collision errors attached to the fallback action; got: {:?}",
        result.errors
    );

    let fallback = result.into_data().expect("fallback action data");
    let StateTransitionAction::UnshieldAction(unshield) = fallback else {
        panic!("expected a fallback UnshieldAction, got {fallback:?}");
    };

    use drive::state_transition_action::shielded::unshield::UnshieldTransitionAction;
    let UnshieldTransitionAction::V0(unshield_v0) = unshield;
    // The fallback Unshield is topologically identical to the would-be success exit: it spends the
    // full denomination from the pool, credits the bound fallback address (the recipient receives
    // `denomination - penalty`), reuses the same notes/anchor, and reads the same pool balance.
    assert_eq!(unshield_v0.output_address, FALLBACK_ADDRESS);
    assert_eq!(unshield_v0.amount, DENOMINATION);
    assert_eq!(unshield_v0.anchor, ANCHOR);
    assert_eq!(
        unshield_v0.current_total_balance,
        expected_current_total_balance
    );
    assert_eq!(
        unshield_v0.notes.len(),
        2,
        "the fallback must carry the original 2 spend notes"
    );
    // The penalty is the flat `unique_key_already_present` plus metered processing, capped at the
    // denomination so the converter's `amount - fee` can never underflow.
    let penalty_floor = platform_version
        .drive_abci
        .validation_and_processing
        .penalties
        .unique_key_already_present;
    assert!(
        unshield_v0.fee_amount >= penalty_floor && unshield_v0.fee_amount <= DENOMINATION,
        "penalty {} must be >= the flat floor {penalty_floor} and capped at the denomination {DENOMINATION}",
        unshield_v0.fee_amount
    );
}

/// BLOCKING regression: the fallback charge must EXECUTE through `execute_event` despite the
/// attached collision errors.
///
/// `validate_state` returns the fallback `UnshieldAction` WITH the unique-key-hash collision errors
/// (`new_with_data_and_errors`). That routes to `ExecutionEvent::PaidFromShieldedPool`, whose
/// execution arm MUST apply the ops (consume nullifiers, debit the pool, credit the fallback, book
/// the penalty) and report `UnsuccessfulPaidExecution` — NOT skip the ops and return
/// `UnpaidConsensusExecutionError`. If the ops were skipped (the old `errors.is_empty()` gate), the
/// nullifiers would never be consumed, letting an attacker force repeated (expensive) Halo 2
/// verification of a valid-proof-but-colliding-key transition for free. This drives the REAL
/// `execute_event` path (the prior conservation test bypassed it by applying converter ops directly).
#[test]
fn failure_path_charge_executes_through_execute_event() {
    use crate::execution::validation::state_transition::state_transitions::shielded_common::read_pool_total_balance;
    use crate::platform_types::event_execution_result::EventExecutionResult;
    use dpp::block::epoch::Epoch;
    use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
    use dpp::identity::accessors::IdentitySettersV0;

    let platform_version = PlatformVersion::latest();
    let platform = setup_platform();
    let block_info = BlockInfo::default();
    let seed = DENOMINATION * 10;

    set_pool_total_balance(&platform, seed);
    insert_anchor_into_state(&platform, &ANCHOR);
    let min_notes = platform_version
        .drive_abci
        .validation_and_processing
        .event_constants
        .minimum_pool_notes_for_outgoing;
    insert_dummy_encrypted_notes(&platform, min_notes.max(1));

    // Pre-register a colliding identity (balance 0 so it doesn't perturb anything we read).
    let (mut existing_identity, keys_with_private): (Identity, Vec<(IdentityPublicKey, [u8; 32])>) =
        Identity::random_identity_with_main_keys_with_private_key(
            3,
            &mut rand::rngs::StdRng::seed_from_u64(77),
            platform_version,
        )
        .expect("random identity");
    existing_identity.set_balance(0);
    let existing_key = keys_with_private
        .iter()
        .find(|(k, _)| k.key_type() == KeyType::ECDSA_SECP256K1)
        .map(|(k, _)| k.clone())
        .expect("an ECDSA_SECP256K1 key");
    platform
        .drive
        .add_new_identity(
            existing_identity,
            false,
            &block_info,
            true,
            None,
            platform_version,
        )
        .expect("add identity");
    let dup_key = IdentityPublicKeyInCreation::V0(IdentityPublicKeyInCreationV0 {
        id: 0,
        key_type: existing_key.key_type(),
        purpose: existing_key.purpose(),
        security_level: existing_key.security_level(),
        contract_bounds: None,
        read_only: false,
        data: existing_key.data().clone(),
        signature: BinaryData::default(),
    });

    // Run validate_state to obtain the real fallback UnshieldAction + the collision errors.
    let st = transition(vec![dup_key], vec![action(30), action(31)]);
    let mut execution_context =
        StateTransitionExecutionContext::default_for_platform_version(platform_version)
            .expect("execution context");
    let success_action =
        build_success_action(&platform, &st, &mut execution_context, platform_version);
    let platform_state = platform.state.load();
    let platform_ref = PlatformRef {
        drive: &platform.drive,
        state: &platform_state,
        config: &platform.config,
        core_rpc: &platform.core_rpc,
    };
    let result = st
        .validate_state_v0(
            &platform_ref,
            success_action,
            &mut execution_context,
            None,
            platform_version,
        )
        .expect("validate_state");
    let errors = result.errors.clone();
    assert!(
        !errors.is_empty(),
        "the fallback must carry the collision errors"
    );
    let fallback_action = result.into_data().expect("fallback action");

    // Build the execution event from the fallback action and EXECUTE it through the real path.
    let event =
        crate::execution::types::execution_event::ExecutionEvent::create_from_state_transition_action(
            fallback_action,
            None,
            &Epoch::new(0).unwrap(),
            execution_context,
            platform_version,
        )
        .expect("create execution event");

    let transaction = platform.drive.grove.start_transaction();
    let fee_versions = CachedEpochIndexFeeVersions::new();
    let exec_result = platform
        .platform
        .execute_event(
            event,
            errors,
            &block_info,
            &transaction,
            None,
            &mut 0,
            platform_version,
            &fee_versions,
        )
        .expect("execute_event should not error");

    // THE FIX: the charge executes (`UnsuccessfulPaidExecution`), NOT `UnpaidConsensusExecutionError`.
    let booked_fee = match exec_result {
        EventExecutionResult::UnsuccessfulPaidExecution(_, fee_result, _) => {
            fee_result.total_base_fee()
        }
        other => panic!(
            "the fallback charge must execute despite the collision errors; expected \
             UnsuccessfulPaidExecution, got {other:?}"
        ),
    };
    assert!(
        booked_fee > 0,
        "the penalty fee must be booked into the fee pools"
    );

    // The ops were APPLIED: the pool is debited by the full denomination (nullifiers/notes inserted,
    // pool decremented) — proving the spend was finalized, not skipped.
    let mut ops = vec![];
    let pool_after = read_pool_total_balance(
        &platform.drive,
        Some(&transaction),
        &mut ops,
        platform_version,
    )
    .expect("read pool balance");
    assert_eq!(
        pool_after,
        seed - DENOMINATION,
        "the pool must be debited by the full denomination when the fallback charge executes"
    );
}

/// BLOCKING regression for the 2026-06-10 devnet paloma chain halt (height 788): CheckTx fee
/// estimation must NEVER mutate committed GroveDB state.
///
/// `check_tx_v0` estimates fees via `validate_fees_of_event(event, last_block_info,
/// transaction: None, ...)` -> `apply_drive_operations(ops, apply: false, ..., transaction: None)`.
/// Pre-#3823, the `ShieldedPoolOperationType::InsertNullifiers` low-level converter arm eagerly
/// called `drive.store_nullifiers_for_block(...)` — a direct write — with whatever
/// `TransactionArg` it was handed, on the assumption it always ran under the block transaction.
/// During CheckTx that argument is `None`, so the eager write committed DIRECTLY to disk on every
/// node that validated the gossiped type-20 transition; the on-disk root diverged from the signed
/// app hash and every proposer panicked ("drive and platform state app hash mismatch" in
/// `prepare_proposal.rs`) — and restarts panic forever in `info.rs` because the write is durable.
/// #3823 made the converter pure; this test pins the invariant for the exact event/op shape that
/// halted paloma. No Halo 2 proving is needed — the event is built from a synthetic action.
#[test]
fn check_tx_fee_estimation_does_not_mutate_committed_state() {
    use crate::execution::types::execution_event::ExecutionEvent;
    use crate::platform_types::platform_state::PlatformStateV0Methods;
    use crate::test::helpers::state_mutation_guard::assert_committed_root_hash_unchanged;
    use dpp::block::epoch::Epoch;
    use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
    use drive::util::batch::drive_op_batch::ShieldedPoolOperationType;
    use drive::util::batch::DriveOperation;

    let platform_version = PlatformVersion::latest();
    let platform = setup_platform();

    set_pool_total_balance(&platform, DENOMINATION * 10);
    insert_anchor_into_state(&platform, &ANCHOR);
    let min_notes = platform_version
        .drive_abci
        .validation_and_processing
        .event_constants
        .minimum_pool_notes_for_outgoing;
    insert_dummy_encrypted_notes(&platform, min_notes.max(1));

    // Unlike the validate_state tests above, fee estimation CONVERTS the `AddNewIdentity` op,
    // which parses the key bytes — so the key must be a real secp256k1 point, not dummy zeros.
    let (valid_master_key, _) =
        IdentityPublicKey::random_ecdsa_master_authentication_key(0, Some(11), platform_version)
            .expect("expected a valid master key");
    let key_in_creation = IdentityPublicKeyInCreation::V0(IdentityPublicKeyInCreationV0 {
        id: 0,
        key_type: valid_master_key.key_type(),
        purpose: valid_master_key.purpose(),
        security_level: valid_master_key.security_level(),
        contract_bounds: None,
        read_only: false,
        data: valid_master_key.data().clone(),
        signature: BinaryData::default(),
    });

    let st = transition(vec![key_in_creation], vec![action(40), action(41)]);
    let mut execution_context =
        StateTransitionExecutionContext::default_for_platform_version(platform_version)
            .expect("execution context");
    let success_action =
        build_success_action(&platform, &st, &mut execution_context, platform_version);

    let event = ExecutionEvent::create_from_state_transition_action(
        StateTransitionAction::IdentityCreateFromShieldedPoolAction(success_action),
        None,
        &Epoch::new(0).unwrap(),
        execution_context,
        platform_version,
    )
    .expect("create execution event");

    // The event must carry ALL the shielded converter ops that ran on paloma — InsertNullifiers
    // (the arm that held the eager write), InsertNote and UpdateTotalBalance — so estimation
    // exercises every arm of the shielded low-level converter.
    let ExecutionEvent::PaidFromShieldedPoolToNewIdentity { operations, .. } = &event else {
        panic!("expected a PaidFromShieldedPoolToNewIdentity execution event");
    };
    let has_op = |pred: fn(&ShieldedPoolOperationType) -> bool| {
        operations.iter().any(|op| match op {
            DriveOperation::ShieldedPoolOperation(shielded_op) => pred(shielded_op),
            _ => false,
        })
    };
    assert!(
        has_op(|op| matches!(op, ShieldedPoolOperationType::InsertNullifiers { .. })),
        "event must carry InsertNullifiers (the arm that eagerly wrote pre-#3823)"
    );
    assert!(
        has_op(|op| matches!(op, ShieldedPoolOperationType::InsertNote { .. })),
        "event must carry InsertNote"
    );
    assert!(
        has_op(|op| matches!(op, ShieldedPoolOperationType::UpdateTotalBalance { .. })),
        "event must carry UpdateTotalBalance"
    );

    // Run the EXACT CheckTx estimation call (transaction = None, apply = false) and assert the
    // committed root hash is byte-identical: a single eager write in any converter arm trips this.
    let platform_state = platform.state.load();
    let fee_versions = CachedEpochIndexFeeVersions::new();
    let fee_result = assert_committed_root_hash_unchanged(
        &platform.drive,
        platform_version,
        "validate_fees_of_event (type-20 pool->new-identity, CheckTx estimation mode)",
        || {
            platform.platform.validate_fees_of_event(
                &event,
                platform_state.last_block_info(),
                None,
                platform_version,
                &fee_versions,
            )
        },
    )
    .expect("fee estimation must not error");
    assert!(
        fee_result.data.is_some(),
        "estimation must produce a fee result"
    );
}

/// Sum-tree credit-conservation regression for the pool->new-identity exit.
///
/// Applies the converter's high-level drive operations through a REAL Drive and asserts the
/// end-of-block invariant `calculate_total_credits_balance().ok()` — the exact check that halts the
/// chain — still balances. This is the regression guard for the `AddToSystemCredits` over-mint:
/// with that op present the balance is off by `denomination` and `.ok()` is false. It needs no
/// Orchard proof because credit conservation is independent of proof verification (the converter
/// only books balances). The full build->prove->execute->prove/verify happy path additionally needs
/// the shared shielded-strategy harness, which is a pre-existing repo-wide TODO disabled for every
/// shielded transition (the `OperationType` build handlers are commented out in strategy.rs).
#[test]
fn converter_ops_preserve_sum_tree_credit_conservation() {
    use dpp::block::epoch::Epoch;
    use dpp::identity::accessors::IdentitySettersV0;
    use dpp::platform_value::Identifier;
    use drive::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
    use drive::state_transition_action::shielded::identity_create_from_shielded_pool::v0::IdentityCreateFromShieldedPoolTransitionActionV0;
    use drive::state_transition_action::shielded::identity_create_from_shielded_pool::IdentityCreateFromShieldedPoolTransitionAction;
    use drive::state_transition_action::shielded::ShieldedActionNote;
    use std::collections::BTreeMap;

    let platform_version = PlatformVersion::latest();
    let platform = setup_platform();
    let drive = &platform.drive;
    let block_info = BlockInfo::default();
    let seed = 50_000_000_000u64;

    // Seed a BALANCED funded pool: `set_pool_total_balance` raises the shielded pool (an RHS balance
    // tree) AND the system-credit scalar (the conservation equation's LHS) by `seed` together,
    // mirroring a prior shield-in, so the starting state is balanced.
    set_pool_total_balance(&platform, seed);
    assert!(
        drive
            .calculate_total_credits_balance(None, &platform_version.drive)
            .expect("calc")
            .ok()
            .expect("ok"),
        "precondition: the seeded pool+system-credits state must be balanced"
    );

    // A new identity holding the full denomination, funded by the pool (no Orchard proof needed —
    // the converter only books balances).
    let mut identity = Identity::new_with_id_and_keys(
        Identifier::from([0xCD; 32]),
        BTreeMap::new(),
        platform_version,
    )
    .expect("identity");
    identity.set_balance(DENOMINATION);
    let action = IdentityCreateFromShieldedPoolTransitionAction::V0(
        IdentityCreateFromShieldedPoolTransitionActionV0 {
            identity,
            notes: vec![ShieldedActionNote {
                nullifier: [0x10; 32],
                cmx: [0x20; 32],
                cv_net: [0x30; 32],
                encrypted_note: vec![0x77; 216],
            }],
            anchor: [0x07; 32],
            denomination: DENOMINATION,
            fee_amount: 500_000_000,
            current_total_balance: seed,
        },
    );

    let ops = action
        .into_high_level_drive_operations(&Epoch::new(0).unwrap(), platform_version)
        .expect("converter ops");
    drive
        .apply_drive_operations(ops, true, &block_info, None, platform_version, None)
        .expect("apply converter ops");

    // The end-of-block conservation invariant must still hold — this FAILS (off by `denomination`)
    // if the converter re-mints via AddToSystemCredits.
    let balance = drive
        .calculate_total_credits_balance(None, &platform_version.drive)
        .expect("calc");
    assert!(
        balance.ok().expect("ok"),
        "credit supply must be conserved after a pool->identity exit; got {balance}"
    );
}

/// Sum-tree credit-conservation regression for the FAILURE path (the fallback `UnshieldAction`).
///
/// On a unique-public-key-hash collision, `validate_state` finalizes the spend as an
/// `UnshieldAction` that decrements the pool by `denomination` and credits the fallback address with
/// `denomination - penalty`; the `penalty` is reconciled into the fee (storage) pool at block
/// finalization (the `PaidFromShieldedPool` execution event). This test exercises the whole booking:
/// it runs `validate_state` with a pre-registered colliding key to obtain the real fallback action,
/// applies that action's converter ops through a REAL Drive, books the penalty into the storage fee
/// pool (standing in for the end-of-block fee distribution), and asserts the end-of-block invariant
/// `calculate_total_credits_balance().ok()` — the exact check that halts the chain — still balances:
/// pool −denom, address +(denom−penalty), pools +penalty nets to zero against the unchanged system
/// credits. Mirrors `converter_ops_preserve_sum_tree_credit_conservation` (the success path).
#[test]
fn failure_path_unshield_converter_ops_preserve_sum_tree_credit_conservation() {
    use dpp::block::epoch::Epoch;
    use drive::drive::credit_pools::operations::update_storage_fee_distribution_pool_operation;
    use drive::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
    use drive::state_transition_action::shielded::unshield::UnshieldTransitionAction;
    use drive::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
    use drive::util::batch::GroveDbOpBatch;

    let platform_version = PlatformVersion::latest();
    let platform = setup_platform();
    let drive = &platform.drive;
    let block_info = BlockInfo::default();
    // The pool must hold at least the denomination plus the minimum-notes headroom; seed generously.
    let seed = DENOMINATION * 10;

    // Seed a BALANCED funded pool (raises the shielded pool AND system credits by `seed` together).
    set_pool_total_balance(&platform, seed);
    insert_anchor_into_state(&platform, &ANCHOR);
    let min_notes = platform_version
        .drive_abci
        .validation_and_processing
        .event_constants
        .minimum_pool_notes_for_outgoing;
    insert_dummy_encrypted_notes(&platform, min_notes.max(1));

    // Pre-register a different identity that owns an ECDSA_SECP256K1 key, then make the new
    // identity's key DUPLICATE that key's hash so the unique-key-hash state check fails and
    // `validate_state` returns the fallback Unshield. The pre-registered identity is added with a
    // ZERO balance so it doesn't perturb the credit-conservation precondition (a non-zero identity
    // balance would write to the Balances sum tree without a matching system-credit increment).
    use dpp::identity::accessors::IdentitySettersV0;
    let (mut existing_identity, keys_with_private): (Identity, Vec<(IdentityPublicKey, [u8; 32])>) =
        Identity::random_identity_with_main_keys_with_private_key(
            3,
            &mut rand::rngs::StdRng::seed_from_u64(99),
            platform_version,
        )
        .expect("random identity");
    existing_identity.set_balance(0);
    let existing_key = keys_with_private
        .iter()
        .find(|(k, _)| k.key_type() == KeyType::ECDSA_SECP256K1)
        .map(|(k, _)| k.clone())
        .expect("an ECDSA_SECP256K1 key");
    platform
        .drive
        .add_new_identity(
            existing_identity,
            false,
            &block_info,
            true,
            None,
            platform_version,
        )
        .expect("should add the key-owning identity");
    let dup_key = IdentityPublicKeyInCreation::V0(IdentityPublicKeyInCreationV0 {
        id: 0,
        key_type: existing_key.key_type(),
        purpose: existing_key.purpose(),
        security_level: existing_key.security_level(),
        contract_bounds: None,
        read_only: false,
        data: existing_key.data().clone(),
        signature: BinaryData::default(),
    });

    // Precondition: the seeded pool + system-credits + the pre-registered identity are balanced.
    assert!(
        drive
            .calculate_total_credits_balance(None, &platform_version.drive)
            .expect("calc")
            .ok()
            .expect("ok"),
        "precondition: the seeded state must be balanced"
    );

    let st = transition(vec![dup_key], vec![action(20), action(21)]);
    let mut execution_context =
        StateTransitionExecutionContext::default_for_platform_version(platform_version)
            .expect("execution context");
    let action = build_success_action(&platform, &st, &mut execution_context, platform_version);

    let platform_state = platform.state.load();
    let platform_ref = PlatformRef {
        drive: &platform.drive,
        state: &platform_state,
        config: &platform.config,
        core_rpc: &platform.core_rpc,
    };
    let result = st
        .validate_state_v0(
            &platform_ref,
            action,
            &mut execution_context,
            None,
            platform_version,
        )
        .expect("validate_state should not error");
    let fallback = result
        .into_data()
        .expect("fallback action data on the failure path");
    let StateTransitionAction::UnshieldAction(unshield) = fallback else {
        panic!("expected a fallback UnshieldAction");
    };
    let penalty = unshield.fee_amount();

    // Apply the fallback Unshield's converter ops (pool −denom, address +(denom−penalty)).
    let ops = unshield
        .into_high_level_drive_operations(&Epoch::new(0).unwrap(), platform_version)
        .expect("converter ops");
    drive
        .apply_drive_operations(ops, true, &block_info, None, platform_version, None)
        .expect("apply converter ops");

    // Book the penalty into the storage fee (Pools) tree — the end-of-block reconciliation the
    // `PaidFromShieldedPool` execution event performs (`fees_to_add_to_pool`). Without this the
    // total is short by exactly `penalty` (the carved fee), which is the point: the fee is conserved
    // into the pools, not burned.
    let existing_pool = drive
        .get_storage_fees_from_distribution_pool(None, platform_version)
        .expect("read storage fee pool");
    let mut batch = GroveDbOpBatch::new();
    batch.push(
        update_storage_fee_distribution_pool_operation(existing_pool + penalty)
            .expect("storage fee pool op"),
    );
    drive
        .grove_apply_batch(batch, false, None, &platform_version.drive)
        .expect("apply storage fee pool booking");

    // The end-of-block conservation invariant must still hold for the failure path.
    let balance = drive
        .calculate_total_credits_balance(None, &platform_version.drive)
        .expect("calc");
    assert!(
        balance.ok().expect("ok"),
        "credit supply must be conserved after the fallback pool->address unshield; got {balance}"
    );
}

/// Regression for the 2026-06-11 devnet (paloma) type-20 proof-fetch failure: an EXECUTED
/// `IdentityCreateFromShieldedPool` must round-trip `prove_state_transition` ->
/// `verify_state_transition_was_executed_with_proof`. The verifier used
/// `verify_query_with_absence_proof` over the merged {nullifiers, full-identity} query, but
/// `full_identity_query`'s all-keys sub-query is an unbounded RangeFull and absence verification
/// enumerates the query's terminal keys — so EVERY honest type-20 result proof was rejected
/// client-side with "terminal keys are not supported with unbounded ranges", while the transition
/// itself executed on-chain (leaving an orphaned-but-live identity the client never persisted).
///
/// Executes the success action through the real `execute_event` path (synthetic action — no Halo 2
/// proving needed), commits, then generates the result proof and verifies it.
#[test]
fn executed_transition_result_proof_roundtrips() {
    use crate::execution::types::execution_event::ExecutionEvent;
    use crate::platform_types::event_execution_result::EventExecutionResult;
    use dpp::block::epoch::Epoch;
    use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
    use dpp::state_transition::identity_create_from_shielded_pool_transition::accessors::IdentityCreateFromShieldedPoolTransitionAccessorsV0;
    use dpp::state_transition::proof_result::StateTransitionProofResult;
    use dpp::state_transition::StateTransition;
    use drive::drive::Drive;

    let platform_version = PlatformVersion::latest();
    let platform = setup_platform();
    let block_info = BlockInfo::default();

    set_pool_total_balance(&platform, DENOMINATION * 10);
    insert_anchor_into_state(&platform, &ANCHOR);
    let min_notes = platform_version
        .drive_abci
        .validation_and_processing
        .event_constants
        .minimum_pool_notes_for_outgoing;
    insert_dummy_encrypted_notes(&platform, min_notes.max(1));

    // Applying `AddNewIdentity` parses the key bytes, so the key must be a real secp256k1 point.
    let (valid_master_key, _) =
        IdentityPublicKey::random_ecdsa_master_authentication_key(0, Some(21), platform_version)
            .expect("expected a valid master key");
    let key_in_creation = IdentityPublicKeyInCreation::V0(IdentityPublicKeyInCreationV0 {
        id: 0,
        key_type: valid_master_key.key_type(),
        purpose: valid_master_key.purpose(),
        security_level: valid_master_key.security_level(),
        contract_bounds: None,
        read_only: false,
        data: valid_master_key.data().clone(),
        signature: BinaryData::default(),
    });

    let st = transition(vec![key_in_creation], vec![action(60), action(61)]);
    let expected_identity_id = st.identity_id();
    let expected_nullifiers: Vec<Vec<u8>> = st.nullifiers();
    let mut execution_context =
        StateTransitionExecutionContext::default_for_platform_version(platform_version)
            .expect("execution context");
    let success_action =
        build_success_action(&platform, &st, &mut execution_context, platform_version);

    let event = ExecutionEvent::create_from_state_transition_action(
        StateTransitionAction::IdentityCreateFromShieldedPoolAction(success_action),
        None,
        &Epoch::new(0).unwrap(),
        execution_context,
        platform_version,
    )
    .expect("create execution event");

    let transaction = platform.drive.grove.start_transaction();
    let fee_versions = CachedEpochIndexFeeVersions::new();
    let exec_result = platform
        .platform
        .execute_event(
            event,
            vec![],
            &block_info,
            &transaction,
            None,
            &mut 0,
            platform_version,
            &fee_versions,
        )
        .expect("execute_event should not error");
    assert_matches!(
        exec_result,
        EventExecutionResult::SuccessfulPaidExecution(..),
        "the success action must execute"
    );

    // Commit so prove_state_transition reads the post-execution committed state.
    platform
        .drive
        .grove
        .commit_transaction(transaction)
        .unwrap()
        .expect("expected to commit transaction");

    let transition = StateTransition::IdentityCreateFromShieldedPool(st);

    let proof_result = platform
        .drive
        .prove_state_transition(&transition, None, platform_version)
        .expect("expected to generate the result proof");
    let proof_bytes = proof_result
        .into_data()
        .expect("expected proof data, not an error");

    let (root_hash, result) = Drive::verify_state_transition_was_executed_with_proof(
        &transition,
        &block_info,
        &proof_bytes,
        &|_| Ok(None),
        platform_version,
    )
    .map(|(root_hash, outcome)| (root_hash, outcome.into_result()))
    .expect(
        "the honest result proof must verify — absence-proof verification rejects the unbounded \
         all-keys identity sub-query",
    );
    assert_ne!(root_hash, [0u8; 32], "root hash should not be zeroed");

    let StateTransitionProofResult::VerifiedIdentityWithShieldedNullifiers(identity, statuses) =
        result
    else {
        panic!("expected VerifiedIdentityWithShieldedNullifiers, got {result:?}");
    };

    assert_eq!(
        identity.id(),
        expected_identity_id,
        "the proven identity must be the one derived from the published nullifiers"
    );
    assert_eq!(
        identity.public_keys().len(),
        1,
        "the proven identity must hold exactly the transition's declared key"
    );

    assert_eq!(
        statuses.len(),
        expected_nullifiers.len(),
        "should have one status per nullifier"
    );
    for (nf, is_spent) in &statuses {
        assert!(is_spent, "nullifier {} should be spent", hex::encode(nf));
        assert!(
            expected_nullifiers.contains(nf),
            "proved nullifier {} should be one of the transition's nullifiers",
            hex::encode(nf)
        );
    }
}
