use dash_sdk::platform::shielded::try_recover_outgoing_note;
use dashcore::Network;
use drive_proof_verifier::types::ShieldedEncryptedNote;
// All Orchard types come from `grovedb-commitment-tree`'s re-exports of the
// dashpay `orchard` fork — no separate orchard dev-dependency needed.
use grovedb_commitment_tree::{
    DashMemo, Domain, ExtractedNoteCommitment, Note, NoteValue, Nullifier, OrchardDomain,
    OutgoingViewingKey, PaymentAddress as Address, RandomSeed, Rho, ValueCommitTrapdoor,
    ValueCommitment,
};
use rand::rngs::OsRng;
use rand::RngCore;

use crate::changeset::ShieldedChangeSet;
use crate::wallet::shielded::keys::OrchardKeySet;
use crate::wallet::shielded::store::{InMemoryShieldedStore, ShieldedStore, SubwalletId};

/// The Orchard note encryptor with Dash's 36-byte memo (the public
/// `OrchardNoteEncryption` alias defaults its memo param to Zcash's).
type OrchardNoteEncryption = grovedb_commitment_tree::OrchardNoteEncryption<DashMemo>;

const RECIPIENT_SEED: [u8; 32] = [0x42; 32];
const OTHER_SEED: [u8; 32] = [0x99; 32];

fn keyset(seed: &[u8; 32]) -> OrchardKeySet {
    OrchardKeySet::from_seed(seed, Network::Testnet, 0)
        .expect("ZIP-32 derivation from a fixed seed should succeed")
}

/// Build a real Orchard output encrypted to `recipient` with `ovk`
/// set, returning the on-chain `ShieldedEncryptedNote` wire form plus
/// the ground-truth `(recipient_raw, value, memo)` it encodes.
///
/// Mirrors orchard's own `note_encryption::tests::test_vectors`
/// construction (epk + `encrypt_note_plaintext` + `encrypt_outgoing_
/// plaintext` over a `cv` that is used identically on the recovery
/// side via `derive_ock`).
fn make_outgoing_wire_note(
    recipient: Address,
    ovk: OutgoingViewingKey,
    value_credits: u64,
    memo: [u8; 36],
) -> ShieldedEncryptedNote {
    let mut rng = OsRng;

    // In Orchard the output note's rho == the spent note's
    // nullifier, i.e. the same 32-byte Pallas-base element. So draw
    // one valid element and use it for BOTH the wire `nullifier`
    // field and the note's `rho` (`Rho::from_bytes` and
    // `Nullifier::from_bytes` over identical bytes are equivalent to
    // the private `Rho::from_nf_old`).
    let (nf, rho) = loop {
        let mut b = [0u8; 32];
        rng.fill_bytes(&mut b);
        if let (Some(nf), Some(rho)) = (
            Nullifier::from_bytes(&b).into_option(),
            Rho::from_bytes(&b).into_option(),
        ) {
            break (nf, rho);
        }
    };
    let rseed = loop {
        let mut b = [0u8; 32];
        rng.fill_bytes(&mut b);
        if let Some(rseed) = RandomSeed::from_bytes(b, &rho).into_option() {
            break rseed;
        }
    };
    let value = NoteValue::from_raw(value_credits);
    let note = Note::from_parts(recipient, value, rho, rseed)
        .into_option()
        .expect("valid note parts");
    let cmx = ExtractedNoteCommitment::from(note.commitment());

    // A valid value commitment. The SAME `cv` feeds both the
    // outgoing-ciphertext encryption and (stored as `cv_net`) the
    // recovery's `derive_ock`, so it only needs to be a consistent
    // point — its correctness as a commitment to `value` is
    // irrelevant to OVK recovery. Draw a canonical trapdoor scalar
    // via the public `from_bytes` (the `random` ctor is crate-private).
    let rcv = loop {
        let mut b = [0u8; 32];
        rng.fill_bytes(&mut b);
        if let Some(rcv) = ValueCommitTrapdoor::from_bytes(b).into_option() {
            break rcv;
        }
    };
    let cv = ValueCommitment::derive(value - NoteValue::from_raw(0), rcv);

    let ne = OrchardNoteEncryption::new(Some(ovk), note, memo);
    let epk = OrchardDomain::<DashMemo>::epk_bytes(ne.epk());
    let enc = ne.encrypt_note_plaintext();
    let out = ne.encrypt_outgoing_plaintext(&cv, &cmx, &mut rng);

    // Assemble the 216-byte wire `encrypted_note`:
    //   epk(32) || enc_ciphertext(104) || out_ciphertext(80)
    let mut encrypted_note = Vec::with_capacity(216);
    encrypted_note.extend_from_slice(&epk.0);
    encrypted_note.extend_from_slice(enc.as_ref());
    encrypted_note.extend_from_slice(&out);
    assert_eq!(encrypted_note.len(), 216, "wire note must be 216 bytes");

    ShieldedEncryptedNote {
        cmx: cmx.to_bytes().to_vec(),
        nullifier: nf.to_bytes().to_vec(),
        cv_net: cv.to_bytes().to_vec(),
        encrypted_note,
    }
}

/// THE core stage-3 guarantee: a note the wallet SENT (encrypted with
/// the wallet's own OVK) is recovered — recipient, value, memo — by
/// the scan's OVK-recovery primitive, and lands in the outgoing-note
/// store with the correct fields.
#[test]
fn sent_note_round_trips_through_ovk_recovery() {
    let sender = keyset(&RECIPIENT_SEED);
    let recipient_addr = sender.address_at(7);
    let value = 123_456u64;
    let mut memo = [0u8; 36];
    memo[..5].copy_from_slice(b"hello");

    let wire = make_outgoing_wire_note(
        recipient_addr,
        sender.outgoing_viewing_key.clone(),
        value,
        memo,
    );

    // 1) The SDK recovery primitive (exactly what the scan calls)
    //    opens it under the sender's OVK.
    let (note, recovered_recipient, recovered_memo) =
        try_recover_outgoing_note(&sender.outgoing_viewing_key, &wire)
            .expect("the wallet's own OVK must recover the note it sent");
    assert_eq!(note.value().inner(), value, "recovered value mismatch");
    assert_eq!(
        recovered_recipient.to_raw_address_bytes(),
        recipient_addr.to_raw_address_bytes(),
        "recovered recipient mismatch"
    );
    assert_eq!(&recovered_memo[..], &memo[..], "recovered memo mismatch");

    // 2) The store record path (exactly what `sync_notes_across`
    //    drives) persists it as a `ShieldedOutgoingNote`.
    let mut store = InMemoryShieldedStore::new();
    let id = SubwalletId::new([0x01; 32], 0);
    let mut changeset = ShieldedChangeSet::default();
    let outgoing = super::super::store::ShieldedOutgoingNote {
        cmx: ExtractedNoteCommitment::from(note.commitment()).to_bytes(),
        recipient: recovered_recipient.to_raw_address_bytes().to_vec(),
        value: note.value().inner(),
        memo: recovered_memo.to_vec(),
        block_height: 42,
    };
    assert!(
        store.record_outgoing_note(id, &outgoing).unwrap(),
        "first record of a sent note must be newly stored"
    );
    changeset.record_outgoing_note(id, outgoing);

    let stored = store.get_outgoing_notes(id).unwrap();
    assert_eq!(stored.len(), 1, "one outgoing note must be stored");
    assert_eq!(stored[0].value, value);
    assert_eq!(
        stored[0].recipient.as_slice(),
        &recipient_addr.to_raw_address_bytes()[..]
    );
    assert_eq!(stored[0].memo.as_slice(), &memo[..]);
    assert_eq!(stored[0].block_height, 42);
    // The changeset carries the same single outgoing note for the
    // persister.
    assert_eq!(changeset.outgoing_notes.get(&id).map(Vec::len), Some(1));
}

/// No false positives: a note encrypted with a DIFFERENT wallet's OVK
/// does NOT recover under our OVK. (A note the wallet merely received
/// — its `out_ciphertext` keyed to the *sender's* OVK — is exactly
/// this case, so it stays out of our send history.)
#[test]
fn note_sent_with_a_different_ovk_does_not_recover() {
    let us = keyset(&RECIPIENT_SEED);
    let them = keyset(&OTHER_SEED);

    // Someone else sent a note (encrypted with THEIR ovk) to us.
    let wire = make_outgoing_wire_note(
        us.address_at(0),
        them.outgoing_viewing_key.clone(),
        10_000,
        [0u8; 36],
    );

    assert!(
        try_recover_outgoing_note(&us.outgoing_viewing_key, &wire).is_none(),
        "a note sent under a different OVK must not OVK-recover under ours"
    );
}

/// Idempotency: re-recording the same recovered note (a re-scan of
/// the same chunk) is a no-op keyed by `cmx`.
#[test]
fn re_recording_a_sent_note_is_idempotent() {
    let sender = keyset(&RECIPIENT_SEED);
    let wire = make_outgoing_wire_note(
        sender.address_at(1),
        sender.outgoing_viewing_key.clone(),
        777,
        [0u8; 36],
    );
    let (note, recipient, memo) =
        try_recover_outgoing_note(&sender.outgoing_viewing_key, &wire).unwrap();

    let mut store = InMemoryShieldedStore::new();
    let id = SubwalletId::new([0x02; 32], 3);
    let outgoing = super::super::store::ShieldedOutgoingNote {
        cmx: ExtractedNoteCommitment::from(note.commitment()).to_bytes(),
        recipient: recipient.to_raw_address_bytes().to_vec(),
        value: note.value().inner(),
        memo: memo.to_vec(),
        block_height: 1,
    };

    assert!(
        store.record_outgoing_note(id, &outgoing).unwrap(),
        "first record is new"
    );
    assert!(
        !store.record_outgoing_note(id, &outgoing).unwrap(),
        "re-recording the same cmx must be a no-op"
    );
    assert_eq!(
        store.get_outgoing_notes(id).unwrap().len(),
        1,
        "idempotent re-record must not duplicate the row"
    );
}
