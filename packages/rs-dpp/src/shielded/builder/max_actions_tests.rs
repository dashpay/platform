use super::test_helpers::{test_orchard_address, test_spendable_note, TestProver};
use super::*;
use crate::address_funds::PlatformAddress;
use crate::identity::core_script::CoreScript;
use crate::serialization::PlatformSerializable;
use crate::shielded::max_shielded_actions_per_transition;
use crate::state_transition::StateTransition;
use crate::withdrawal::Pooling;
use grovedb_commitment_tree::{
    Anchor, ExtractedNoteCommitment, FullViewingKey, SpendAuthorizingKey, SpendingKey,
};
use platform_value::Identifier;
use platform_version::version::PlatformVersion;

const NOTE_VALUE: u64 = 100_000_000_000;

fn keys() -> (FullViewingKey, SpendAuthorizingKey) {
    let sk = SpendingKey::from_bytes([42u8; 32]).expect("valid spending key bytes");
    (FullViewingKey::from(&sk), SpendAuthorizingKey::from(&sk))
}

fn notes(count: usize) -> Vec<SpendableNote> {
    (0..count)
        .map(|_| test_spendable_note(NOTE_VALUE))
        .collect()
}

fn anchor() -> Anchor {
    let note = test_spendable_note(NOTE_VALUE);
    note.merkle_path
        .root(ExtractedNoteCommitment::from(note.note.commitment()))
}

fn size(st: &StateTransition) -> usize {
    st.serialize_to_bytes().expect("serialize").len()
}

/// The serialized size of each kind of shielded spend with `actions` spent notes (one action
/// per note), keyed by kind.
fn spend_sizes(actions: usize) -> Vec<(&'static str, usize)> {
    let pv = PlatformVersion::latest();
    let (fvk, ask) = keys();
    let addr = test_orchard_address();
    let (transfer, _) = build_shielded_transfer_transition(
        notes(actions),
        &addr,
        1_000,
        &addr,
        &fvk,
        &ask,
        anchor(),
        &TestProver,
        [0u8; 36],
        pv,
    )
    .expect("transfer");
    let (unshield, _) = build_unshield_transition(
        notes(actions),
        PlatformAddress::P2pkh([1u8; 20]),
        1_000,
        &addr,
        &fvk,
        &ask,
        anchor(),
        &TestProver,
        [0u8; 36],
        pv,
    )
    .expect("unshield");
    let (withdrawal, _) = build_shielded_withdrawal_transition(
        notes(actions),
        1_000_000,
        CoreScript::new_p2pkh([1u8; 20]),
        1,
        Pooling::Never,
        &addr,
        &fvk,
        &ask,
        anchor(),
        &TestProver,
        [0u8; 36],
        pv,
    )
    .expect("withdrawal");
    let (top_up, _) = build_identity_top_up_from_shielded_pool_transition(
        notes(actions),
        Identifier::new([3u8; 32]),
        1_000,
        &addr,
        &fvk,
        &ask,
        anchor(),
        &TestProver,
        [0u8; 36],
        pv,
    )
    .expect("top up");
    vec![
        ("transfer", size(&transfer)),
        ("unshield", size(&unshield)),
        ("withdrawal", size(&withdrawal)),
        ("identity top up", size(&top_up)),
    ]
}

/// The helper's action count fits `max_state_transition_size` in real serializations of every
/// shielded spend kind, and one action more does not fit any of them.
#[test]
fn should_bound_the_actions_of_every_spend_kind_by_the_transition_size_limit() {
    let pv = PlatformVersion::latest();
    let max_size = pv.system_limits.max_state_transition_size as usize;
    let bound = usize::from(max_shielded_actions_per_transition(pv));

    for (kind, bytes) in spend_sizes(bound) {
        assert!(
            bytes <= max_size,
            "{kind}: {bound} actions serialize to {bytes} B, over the {max_size} B limit"
        );
    }
    for (kind, bytes) in spend_sizes(bound + 1) {
        assert!(
            bytes > max_size,
            "{kind}: {} actions serialize to {bytes} B, which still fits the {max_size} B limit",
            bound + 1
        );
    }
}
