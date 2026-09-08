//! MEASURED regression coverage for the shielded wire-cost model.
//!
//! [`super::wire_cost_tests::shielded_wire_cost_model_matches_measured_transitions`] compares the
//! estimator against the same three literals its constants were *derived* from, so it stays green
//! no matter how Orchard proof encoding or platform serialization moves. The one real-serialization
//! test that existed (`seed_pool_batch_fits_max_state_transition_size`) only checked that a
//! six-action transition stays under the limit — it never compared the measured length with the
//! model, and nothing serialized a real SEVEN-action transition to confirm the rejected boundary.
//!
//! These tests close both gaps. For each of the three envelope shapes the pre-proving gate prices,
//! they BUILD and PROVE a real transition, serialize it through the same
//! [`PlatformSerializable`] path DAPI's byte prefilter reads, and check the measured length against
//! the model at BOTH the largest action count the gate accepts and the next one:
//!
//! - **transfer** — the baseline envelope (`extra_envelope_bytes == 0`), shared with unshield and
//!   withdrawal. At the ceiling this is exactly the five-recipient multi-output shape this PR
//!   introduces (5 recipients + change = 6 actions).
//! - **asset-lock** — `ShieldFromAssetLock`, which prices its asset-lock proof as a delta over
//!   [`SHIELDED_BASELINE_ASSET_LOCK_PROOF_BYTES`]. This is the shape the constants were CALIBRATED
//!   from, so its measured length must still equal the model exactly.
//! - **identity-key** — `IdentityCreateFromShieldedPool` with the maximal six-key set, whose
//!   serialized keys plus [`PER_KEY_SIGNATURE_ALLOWANCE_BYTES`] ride the envelope.
//!
//! Two properties are asserted at every measured point: the model must never UNDER-estimate (or the
//! gate admits a bundle the byte prefilter kills after the proof is paid for), and it must not
//! over-estimate by more than a small documented budget (or encoding shrinkage has silently left
//! the model — and the public recipient ceiling derived from it — needlessly restrictive).
//!
//! Cost: six real Halo 2 proofs plus the one-time proving-key build (~80 s cold in a debug build).
//! They run as ordinary tests rather than `#[ignore]`d ones, matching every other real-proving
//! shielded test in this crate; CI runs the whole shielded suite in one process, so the proving key
//! is built once and shared (see the `Run shielded tests with cargo test (shared process for VK
//! reuse)` step).
#![cfg(all(test, feature = "shielded-client"))]

use crate::address_funds::PlatformAddress;
use crate::identity::identity_public_key::v0::IdentityPublicKeyV0;
use crate::identity::signer::Signer;
use crate::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
use crate::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use crate::prelude::AssetLockProof;
use crate::serialization::PlatformSerializable;
use crate::shielded::builder::test_helpers::{
    test_orchard_address, test_spendable_note, TestProver,
};
use crate::shielded::builder::{
    build_identity_create_from_shielded_pool_transition, build_shield_from_asset_lock_transition,
    build_shielded_transfer_transition_multi, serialized_envelope_bytes, shielded_bundle_action_count,
    ShieldedTransferOutput, SpendableNote, PER_KEY_SIGNATURE_ALLOWANCE_BYTES,
};
use crate::shielded::{
    estimated_shielded_transition_wire_bytes_with_envelope, max_shielded_actions_for_envelope,
    SHIELDED_ACTION_WIRE_BYTES, SHIELDED_BASELINE_ASSET_LOCK_PROOF_BYTES,
    SHIELDED_PROOF_WIRE_BYTES_PER_ACTION,
};
use crate::state_transition::public_key_in_creation::v0::IdentityPublicKeyInCreationV0;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use crate::state_transition::state_transitions::shielded::identity_create_from_shielded_pool_transition::methods::IdentityCreateFromShieldedPoolTransitionMethodsV0;
use crate::state_transition::state_transitions::shielded::identity_create_from_shielded_pool_transition::IdentityCreateFromShieldedPoolTransition;
use crate::state_transition::shield_from_asset_lock_transition::ShieldFromAssetLockTransition;
use crate::state_transition::shielded_transfer_transition::ShieldedTransferTransition;
use crate::state_transition::StateTransition;
use crate::ProtocolError;
use dashcore::OutPoint;
use grovedb_commitment_tree::{
    Anchor, ExtractedNoteCommitment, FullViewingKey, Hashable, Level, MerkleHashOrchard, MerklePath,
    SpendAuthorizingKey, SpendingKey, NOTE_COMMITMENT_TREE_DEPTH,
};
use platform_value::BinaryData;
use platform_version::version::PlatformVersion;

/// 1 DASH in credits — the largest member of the versioned exit-denomination set, chosen so the
/// identity-create fee predictor clears its `fee < denomination` gate at six actions and six keys.
const DENOMINATION: u64 = 100_000_000_000;

/// Value of each note the spend-side shapes are funded with. Large enough that six of them cover
/// the denomination and every carved fee with room to spare.
const NOTE_VALUE: u64 = 500_000_000_000;

// ---------------------------------------------------------------------------
// Per-shape slack budgets
//
// Each is the gap measured between the model and the real serialized length at the time of
// writing. The model is calibrated on `ShieldFromAssetLock`, so other transition shapes come in
// slightly UNDER it; the budgets pin how far under, tightly enough that real encoding shrinkage
// trips them while staying far below one action's
// `SHIELDED_ACTION_WIRE_BYTES + SHIELDED_PROOF_WIRE_BYTES_PER_ACTION` (2,681 B) — i.e. long before
// the slack could cost a recipient slot.
// ---------------------------------------------------------------------------

/// `ShieldFromAssetLock` is the CALIBRATION shape: the constants were derived from exactly these
/// serialized lengths, so it must still match the model to the byte. Any drift here means the
/// constants are stale and every other shape's ceiling is wrong with them.
const ASSET_LOCK_SLACK_BUDGET_BYTES: u64 = 0;

/// A `ShieldedTransfer` envelope is 105 B leaner than the calibrated `ShieldFromAssetLock` one
/// (measured: 18,913 B vs the model's 19,018 B at six actions). The budget leaves headroom for
/// small field-encoding movement.
const TRANSFER_SLACK_BUDGET_BYTES: u64 = 256;

/// `IdentityCreateFromShieldedPool` with six keys comes in 240 B under the model (measured:
/// 19,613 B vs 19,853 B at six actions). Most of that is deliberate:
/// [`PER_KEY_SIGNATURE_ALLOWANCE_BYTES`] budgets a 96-byte BLS signature per key while these
/// ECDSA keys carry 65, and the gate measures the keys BEFORE they are PoP-signed. The budget
/// admits that intentional conservatism and nothing much more.
const IDENTITY_SLACK_BUDGET_BYTES: u64 = 512;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// Witness every leaf in ONE consistent tree: each path's siblings are the real neighbouring
/// subtree roots (empty-subtree roots past the frontier), so all leaves compute the SAME anchor
/// and the Orchard circuit accepts the spends. Generalises the two-leaf construction in
/// `shielded_transfer`'s tests to the six-spend bundles the ceiling needs.
fn witness_all(leaves: &[ExtractedNoteCommitment]) -> (Vec<MerklePath>, Anchor) {
    let mut nodes: Vec<MerkleHashOrchard> =
        leaves.iter().map(MerkleHashOrchard::from_cmx).collect();
    let mut positions: Vec<usize> = (0..leaves.len()).collect();
    let mut auth: Vec<Vec<MerkleHashOrchard>> = vec![Vec::new(); leaves.len()];

    for depth in 0..NOTE_COMMITMENT_TREE_DEPTH {
        let level = Level::from(depth as u8);
        let empty = MerkleHashOrchard::empty_root(level);
        for (leaf, pos) in positions.iter().enumerate() {
            auth[leaf].push(nodes.get(pos ^ 1).copied().unwrap_or(empty));
        }
        let mut next = Vec::with_capacity(nodes.len().div_ceil(2));
        for pair in nodes.chunks(2) {
            let right = pair.get(1).copied().unwrap_or(empty);
            next.push(MerkleHashOrchard::combine(level, &pair[0], &right));
        }
        nodes = next;
        for pos in positions.iter_mut() {
            *pos /= 2;
        }
    }

    let paths: Vec<MerklePath> = auth
        .into_iter()
        .enumerate()
        .map(|(i, path)| {
            let fixed: [MerkleHashOrchard; NOTE_COMMITMENT_TREE_DEPTH] =
                path.try_into().expect("exactly one sibling per level");
            MerklePath::from_parts(i as u32, fixed)
        })
        .collect();

    let anchor = paths[0].root(leaves[0]);
    for (i, path) in paths.iter().enumerate() {
        assert_eq!(
            path.root(leaves[i]).to_bytes(),
            anchor.to_bytes(),
            "every witness must compute the same anchor, but leaf {i} disagrees"
        );
    }
    (paths, anchor)
}

/// `count` spendable notes of [`NOTE_VALUE`] witnessed in one tree, with their shared anchor.
fn spendable_notes(count: usize) -> (Vec<SpendableNote>, Anchor) {
    let notes: Vec<_> = (0..count)
        .map(|_| test_spendable_note(NOTE_VALUE).note)
        .collect();
    let cmxs: Vec<ExtractedNoteCommitment> = notes
        .iter()
        .map(|note| ExtractedNoteCommitment::from(note.commitment()))
        .collect();
    let (paths, anchor) = witness_all(&cmxs);
    let spends = notes
        .into_iter()
        .zip(paths)
        .map(|(note, merkle_path)| SpendableNote { note, merkle_path })
        .collect();
    (spends, anchor)
}

/// `PlatformVersion::latest()` with the transition-size limit lifted.
///
/// Used ONLY to get a past-the-ceiling bundle through the pre-proving gate so it can actually be
/// proved and serialized — the rejected boundary cannot be measured otherwise. Every assertion
/// still compares against the REAL limit from `PlatformVersion::latest()`, and
/// `*_rejects_the_next_action_count` separately pins that the real gate refuses that count.
fn size_limit_lifted() -> PlatformVersion {
    let mut version = PlatformVersion::latest().clone();
    version.system_limits.max_state_transition_size = 1_000_000;
    version
}

fn chain_asset_lock_proof() -> AssetLockProof {
    AssetLockProof::Chain(ChainAssetLockProof {
        core_chain_locked_height: 100,
        out_point: OutPoint::from([11u8; 36]),
    })
}

/// The envelope delta `ShieldFromAssetLock` hands the gate for a chain proof.
fn asset_lock_envelope_bytes() -> u64 {
    serialized_envelope_bytes(&chain_asset_lock_proof(), "the asset-lock proof")
        .expect("measurable asset-lock proof")
        .saturating_sub(SHIELDED_BASELINE_ASSET_LOCK_PROOF_BYTES)
}

/// The envelope `IdentityCreateFromShieldedPool` hands the gate for a maximal six-key set.
fn identity_envelope_bytes() -> u64 {
    let keys: Vec<IdentityPublicKeyInCreation> = (0..6u32).map(|id| key_pair(id).1).collect();
    serialized_envelope_bytes(&keys, "the identity key set")
        .expect("measurable key set")
        .saturating_add(keys.len() as u64 * PER_KEY_SIGNATURE_ALLOWANCE_BYTES)
}

/// Fixed 65-byte proof-of-possession stub; the builder fills and never verifies these.
#[derive(Debug)]
struct DummySigner;

#[async_trait::async_trait]
impl Signer<IdentityPublicKey> for DummySigner {
    async fn sign(
        &self,
        _key: &IdentityPublicKey,
        _data: &[u8],
    ) -> Result<BinaryData, ProtocolError> {
        Ok(BinaryData::new(vec![0u8; 65]))
    }

    async fn sign_create_witness(
        &self,
        _key: &IdentityPublicKey,
        _data: &[u8],
    ) -> Result<crate::address_funds::AddressWitness, ProtocolError> {
        Err(ProtocolError::ShieldedBuildError(
            "identity PoP signer never creates address witnesses".to_string(),
        ))
    }

    fn can_sign_with(&self, _key: &IdentityPublicKey) -> bool {
        true
    }
}

/// One AUTHENTICATION/MASTER ECDSA key in both forms the identity-create builder takes.
fn key_pair(id: u32) -> (IdentityPublicKey, IdentityPublicKeyInCreation) {
    let public = IdentityPublicKey::V0(IdentityPublicKeyV0 {
        id,
        purpose: Purpose::AUTHENTICATION,
        security_level: SecurityLevel::MASTER,
        contract_bounds: None,
        key_type: KeyType::ECDSA_SECP256K1,
        read_only: false,
        data: BinaryData::new(vec![0xAB; 33]),
        disabled_at: None,
    });
    let in_creation = IdentityPublicKeyInCreation::V0(IdentityPublicKeyInCreationV0 {
        id,
        key_type: KeyType::ECDSA_SECP256K1,
        purpose: Purpose::AUTHENTICATION,
        security_level: SecurityLevel::MASTER,
        contract_bounds: None,
        read_only: false,
        data: BinaryData::new(vec![0xAB; 33]),
        signature: BinaryData::new(vec![]),
    });
    (public, in_creation)
}

// ---------------------------------------------------------------------------
// Builders: one real, proved, serialized transition of each shape
// ---------------------------------------------------------------------------

/// A `ShieldedTransfer` publishing exactly `num_actions` on-wire actions: one spend plus
/// `num_actions - 1` recipient outputs and the change note (`max(spends, recipients + 1)`).
fn transfer_wire_bytes(num_actions: usize, version: &PlatformVersion) -> usize {
    let spending_key = SpendingKey::from_bytes([42u8; 32]).expect("valid spending key");
    let fvk = FullViewingKey::from(&spending_key);
    let ask = SpendAuthorizingKey::from(&spending_key);
    let recipient = test_orchard_address();
    let change_address = test_orchard_address();
    let (spends, anchor) = spendable_notes(1);

    let outputs: Vec<ShieldedTransferOutput> = (0..num_actions - 1)
        .map(|_| ShieldedTransferOutput {
            recipient,
            amount: 1_000_000_000,
            memo: [0u8; 36],
        })
        .collect();

    let (state_transition, _fee) = build_shielded_transfer_transition_multi(
        spends,
        &outputs,
        &change_address,
        &fvk,
        &ask,
        anchor,
        &TestProver,
        version,
    )
    .expect("a multi-output transfer at this action count must build");

    let bytes = state_transition
        .serialize_to_bytes()
        .expect("serialize the proved transfer");
    assert_action_count(&state_transition, num_actions);
    bytes.len()
}

/// A `ShieldFromAssetLock` publishing exactly `num_actions` on-wire actions: the real recipient
/// output plus `num_actions - 1` zero-value fillers (`max(1 + dummies, 2)`).
fn asset_lock_wire_bytes(num_actions: usize, version: &PlatformVersion) -> usize {
    let state_transition = build_shield_from_asset_lock_transition(
        &test_orchard_address(),
        50_000u64,
        chain_asset_lock_proof(),
        &[7u8; 32],
        &TestProver,
        [0u8; 36],
        None, // sender_ovk
        None, // surplus_output
        num_actions - 1,
        version,
    )
    .expect("a shield-from-asset-lock at this action count must build");

    let bytes = state_transition
        .serialize_to_bytes()
        .expect("serialize the proved shield-from-asset-lock");
    assert_action_count(&state_transition, num_actions);
    bytes.len()
}

/// An `IdentityCreateFromShieldedPool` with the maximal six-key set publishing exactly
/// `num_actions` on-wire actions (one output — the change note — so the spend count drives it).
///
/// The builder hands back the PoP-signed keys and the proved bundle; re-assembling them through
/// `try_from_bundle` is exactly what the SDK broadcast helper does, so this measures the bytes
/// that really go on the wire.
async fn identity_create_wire_bytes(num_actions: usize, version: &PlatformVersion) -> usize {
    let spending_key = SpendingKey::from_bytes([42u8; 32]).expect("valid spending key");
    let fvk = FullViewingKey::from(&spending_key);
    let ask = SpendAuthorizingKey::from(&spending_key);
    let change_address = test_orchard_address();
    let (spends, anchor) = spendable_notes(num_actions);
    let failure_address = PlatformAddress::P2pkh([0u8; 20]);

    let build = build_identity_create_from_shielded_pool_transition(
        (0..6u32).map(key_pair).collect(),
        DENOMINATION,
        failure_address,
        spends,
        &change_address,
        &fvk,
        &ask,
        anchor,
        &TestProver,
        &DummySigner,
        [0u8; 36],
        version,
    )
    .await
    .expect("an identity create at this action count must build");

    let state_transition = IdentityCreateFromShieldedPoolTransition::try_from_bundle(
        build.public_keys,
        DENOMINATION,
        failure_address,
        build.bundle.actions,
        build.bundle.anchor,
        build.bundle.proof,
        build.bundle.binding_signature,
        version,
    )
    .expect("re-assemble the proved identity-create transition");

    let bytes = state_transition
        .serialize_to_bytes()
        .expect("serialize the proved identity create");
    assert_action_count(&state_transition, num_actions);
    bytes.len()
}

/// Pin that the bundle really published the action count the measurement is attributed to — a
/// silently padded or truncated bundle would compare the model at the wrong point.
fn assert_action_count(state_transition: &StateTransition, expected: usize) {
    let actual = match state_transition {
        StateTransition::ShieldedTransfer(ShieldedTransferTransition::V0(v0)) => v0.actions.len(),
        StateTransition::ShieldFromAssetLock(ShieldFromAssetLockTransition::V0(v0)) => {
            v0.actions.len()
        }
        StateTransition::IdentityCreateFromShieldedPool(
            IdentityCreateFromShieldedPoolTransition::V0(v0),
        ) => v0.actions.len(),
        other => panic!("unexpected transition variant under measurement: {other:?}"),
    };
    assert_eq!(
        actual, expected,
        "the built bundle must publish exactly the action count under test"
    );
}

// ---------------------------------------------------------------------------
// Shared assertions
// ---------------------------------------------------------------------------

/// The model must BRACKET reality at a measured point: never under-estimate (safety), and never
/// over-estimate by more than this shape's documented budget (tightness).
fn assert_model_brackets_reality(
    shape: &str,
    num_actions: usize,
    extra_envelope_bytes: u64,
    actual: usize,
    slack_budget: u64,
) {
    let actual = actual as u64;
    let estimated =
        estimated_shielded_transition_wire_bytes_with_envelope(num_actions, extra_envelope_bytes);

    assert!(
        actual <= estimated,
        "{shape} at {num_actions} actions serializes to {actual} B, ABOVE the model's \
         {estimated} B. The pre-proving gate would admit a bundle DAPI's byte prefilter then \
         rejects — after the ~30 s Halo 2 proof has been paid for. Re-measure \
         SHIELDED_TRANSITION_WIRE_OVERHEAD_BYTES / SHIELDED_PROOF_WIRE_BYTES_PER_ACTION."
    );

    let slack = estimated - actual;
    assert!(
        slack <= slack_budget,
        "{shape} at {num_actions} actions serializes to {actual} B — {slack} B under the model's \
         {estimated} B, past the {slack_budget} B budget for this shape. The encoding shrank (or \
         an envelope allowance went stale), so the model and every ceiling derived from it — \
         including the public multi-recipient ceiling — are needlessly restrictive. Re-measure \
         the constants."
    );
}

/// The measured boundary must be a REAL boundary under the shipping limit: the accepted count
/// fits and the next one does not.
fn assert_real_size_boundary(shape: &str, ceiling: usize, at_ceiling: usize, past_ceiling: usize) {
    let max_size = PlatformVersion::latest()
        .system_limits
        .max_state_transition_size;

    assert!(
        at_ceiling as u64 <= max_size,
        "{shape}: the gate's ceiling of {ceiling} actions serializes to {at_ceiling} B, over the \
         {max_size} B limit — the gate accepts a count that cannot be broadcast"
    );
    assert!(
        past_ceiling as u64 > max_size,
        "{shape}: {} actions serializes to {past_ceiling} B, which FITS the {max_size} B limit — \
         the gate is rejecting a count that would have been accepted on chain, costing a \
         recipient slot",
        ceiling + 1
    );
}

// ---------------------------------------------------------------------------
// Transfer (baseline envelope) — also the five-recipient public ceiling
// ---------------------------------------------------------------------------

#[tokio::test]
async fn shielded_transfer_measured_wire_bytes_match_the_model_at_the_action_ceiling() {
    let version = PlatformVersion::latest();
    let ceiling = max_shielded_actions_for_envelope(version, 0);
    assert_eq!(
        ceiling, 6,
        "the baseline ceiling is the five-recipients-plus-change shape this PR exposes"
    );

    let lifted = size_limit_lifted();
    let at_ceiling = transfer_wire_bytes(ceiling, version);
    let past_ceiling = transfer_wire_bytes(ceiling + 1, &lifted);

    assert_model_brackets_reality(
        "ShieldedTransfer",
        ceiling,
        0,
        at_ceiling,
        TRANSFER_SLACK_BUDGET_BYTES,
    );
    assert_model_brackets_reality(
        "ShieldedTransfer",
        ceiling + 1,
        0,
        past_ceiling,
        TRANSFER_SLACK_BUDGET_BYTES,
    );
    assert_real_size_boundary("ShieldedTransfer", ceiling, at_ceiling, past_ceiling);

    // The measured per-action growth must be the constant the model prices actions at, or the
    // model is only accidentally right at these two points.
    assert_eq!(
        (past_ceiling - at_ceiling) as u64,
        SHIELDED_ACTION_WIRE_BYTES + SHIELDED_PROOF_WIRE_BYTES_PER_ACTION,
        "one more action must cost exactly the modelled per-action bytes"
    );

    // And the shipping gate must refuse the count that genuinely does not fit.
    shielded_bundle_action_count(1, ceiling + 1, 0, version)
        .expect_err("the real gate must reject the first action count that does not fit");
}

// ---------------------------------------------------------------------------
// Asset-lock envelope — the CALIBRATION shape
// ---------------------------------------------------------------------------

#[tokio::test]
async fn shield_from_asset_lock_measured_wire_bytes_match_the_model_exactly() {
    let version = PlatformVersion::latest();
    let envelope = asset_lock_envelope_bytes();
    let ceiling = max_shielded_actions_for_envelope(version, envelope);

    let lifted = size_limit_lifted();
    let at_ceiling = asset_lock_wire_bytes(ceiling, version);
    let past_ceiling = asset_lock_wire_bytes(ceiling + 1, &lifted);

    // Budget 0: the constants were derived from exactly these lengths. This is the assertion
    // `shielded_wire_cost_model_matches_measured_transitions` could never make, because it
    // compares the estimator with the literals instead of with a real transition.
    assert_model_brackets_reality(
        "ShieldFromAssetLock",
        ceiling,
        envelope,
        at_ceiling,
        ASSET_LOCK_SLACK_BUDGET_BYTES,
    );
    assert_model_brackets_reality(
        "ShieldFromAssetLock",
        ceiling + 1,
        envelope,
        past_ceiling,
        ASSET_LOCK_SLACK_BUDGET_BYTES,
    );
    assert_real_size_boundary("ShieldFromAssetLock", ceiling, at_ceiling, past_ceiling);

    shielded_bundle_action_count(0, ceiling + 1, envelope, version)
        .expect_err("the real gate must reject the first action count that does not fit");
}

// ---------------------------------------------------------------------------
// Identity-key envelope
// ---------------------------------------------------------------------------

#[tokio::test]
async fn shielded_identity_create_measured_wire_bytes_match_the_model_with_a_maximal_key_set() {
    let version = PlatformVersion::latest();
    let envelope = identity_envelope_bytes();
    let ceiling = max_shielded_actions_for_envelope(version, envelope);
    assert!(
        ceiling >= 2,
        "a maximal key set must not brick identity creation"
    );

    let lifted = size_limit_lifted();
    let at_ceiling = identity_create_wire_bytes(ceiling, version).await;
    let past_ceiling = identity_create_wire_bytes(ceiling + 1, &lifted).await;

    assert_model_brackets_reality(
        "IdentityCreateFromShieldedPool",
        ceiling,
        envelope,
        at_ceiling,
        IDENTITY_SLACK_BUDGET_BYTES,
    );
    assert_model_brackets_reality(
        "IdentityCreateFromShieldedPool",
        ceiling + 1,
        envelope,
        past_ceiling,
        IDENTITY_SLACK_BUDGET_BYTES,
    );
    assert_real_size_boundary(
        "IdentityCreateFromShieldedPool",
        ceiling,
        at_ceiling,
        past_ceiling,
    );

    shielded_bundle_action_count(ceiling + 1, 1, envelope, version)
        .expect_err("the real gate must reject the first action count that does not fit");
}
