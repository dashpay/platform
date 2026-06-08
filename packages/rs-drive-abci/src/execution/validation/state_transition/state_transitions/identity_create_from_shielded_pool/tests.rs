//! Drive-backed tests for the `IdentityCreateFromShieldedPool` transition. Structural checks are
//! covered in the dpp `validate_structure` tests and op-level checks in the drive converter tests.
//!
//! Covered here:
//! - The two consensus-facing early-out branches in `transform_into_action_v0` (mirroring
//!   `IdentityCreate::validate_state`): an identity already existing at the derived id, and a
//!   public-key hash already registered to another identity — both convert what would otherwise be
//!   an internal Drive error during `AddNewIdentity` execution into a clean consensus rejection.
//! - Sum-tree credit conservation: the converter ops applied through a real Drive keep
//!   `calculate_total_credits_balance().ok()` balanced (the end-of-block invariant that halts the
//!   chain) — the regression guard for the `AddToSystemCredits` over-mint.
//!
//! The full build->prove->execute->prove/verify happy path (real Orchard proof + the strict merged
//! nullifier+identity proof roundtrip) is deferred to the shared shielded-strategy harness, a
//! pre-existing repo-wide TODO that is disabled for every shielded transition (the shielded
//! `OperationType` build handlers are commented out in `strategy.rs`).

use super::transform_into_action::v0::IdentityCreateFromShieldedPoolStateTransitionTransformIntoActionValidationV0;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::state_transitions::test_helpers::{
    insert_anchor_into_state, insert_dummy_encrypted_notes, set_pool_total_balance, setup_platform,
};
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
use rand::SeedableRng;

const DENOMINATION: u64 = 10_000_000_000;
const ANCHOR: [u8; 32] = [7u8; 32];

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
        identity_id,
    })
}

#[test]
fn transform_rejects_when_identity_already_exists_at_derived_id() {
    let platform_version = PlatformVersion::latest();
    let platform = setup_platform();

    // Seed enough pool state (balance, the anchor, the minimum note count) that the transformer
    // gets past the pool/anchor/nullifier/balance checks and reaches the identity-creation checks.
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
    let result = st
        .transform_into_action_v0(
            &platform.drive,
            &mut execution_context,
            None,
            platform_version,
        )
        .expect("transform should not error");

    assert!(!result.is_valid(), "expected a consensus rejection");
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
fn transform_rejects_when_a_public_key_hash_is_already_registered() {
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
    // is the one that must reject.
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
    let result = st
        .transform_into_action_v0(
            &platform.drive,
            &mut execution_context,
            None,
            platform_version,
        )
        .expect("transform should not error");

    assert!(!result.is_valid(), "expected a consensus rejection");
    // The latest platform version dispatches the v1 unique-key-hash check, which reports the
    // collision as a StateError (v0 reported it as a BasicError).
    assert_matches!(
        result.errors.as_slice(),
        [ConsensusError::StateError(
            StateError::DuplicatedIdentityPublicKeyIdStateError(_)
        )],
        "got: {:?}",
        result.errors
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
