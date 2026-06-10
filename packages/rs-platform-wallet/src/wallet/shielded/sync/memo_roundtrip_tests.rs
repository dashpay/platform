//! Round-trip: a [`ShieldedMemo`] attached to a shielded output must
//! survive encryption and come back out of both client-side recovery
//! primitives — byte-for-byte, and back to its decoded text.
//!
//! Two complementary halves, mirroring the two scan-side primitives:
//!
//!   * IVK side — build a real Type 15 Shield transition (exactly as
//!     `operations::shield` does: `OrchardKeySet::from_seed` →
//!     `build_shield_transition` with the `&&prover` double-ref) carrying
//!     a text memo, then assert the FULL incoming decryption
//!     (`try_decrypt_note_with_memo`) under the recipient's IVK recovers
//!     the note AND the exact memo bytes. The shield builder's serialized
//!     actions are stored verbatim on-chain, so this is the real client
//!     round trip with the chain cut out of the middle.
//!
//!   * OVK side — the shield/transfer builders encrypt `out_ciphertext`
//!     with `ovk = None` (a per-output random OVK), so a transition's own
//!     outputs are *not* recoverable under the wallet's OVK. The wallet's
//!     send-history recovery (`try_recover_outgoing_note`) instead opens
//!     notes a wallet encrypted with its OWN OVK. We construct such a note
//!     directly (the same `OrchardNoteEncryption::new(Some(ovk), …)` shape
//!     the OVK-recovery tests use) carrying the same memo bytes and assert
//!     OVK recovery returns them. Building the note this way (rather than
//!     via the transition builder) is required: there is no builder path
//!     that emits an own-OVK-recoverable output.

use dash_sdk::platform::shielded::{try_decrypt_note_with_memo, try_recover_outgoing_note};
use dashcore::Network;
use dpp::address_funds::{
    AddressFundsFeeStrategyStep, AddressWitness, OrchardAddress, PlatformAddress,
};
use dpp::identity::signer::Signer;
use dpp::platform_value::BinaryData;
use dpp::shielded::builder::build_shield_transition;
use dpp::shielded::ShieldedMemo;
use dpp::state_transition::shield_transition::ShieldTransition;
use dpp::state_transition::StateTransition;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use drive_proof_verifier::types::ShieldedEncryptedNote;
use grovedb_commitment_tree::{
    DashMemo, Domain, ExtractedNoteCommitment, Note, NoteValue, Nullifier, OrchardDomain,
    OutgoingViewingKey, PaymentAddress as Address, RandomSeed, Rho, ValueCommitTrapdoor,
    ValueCommitment,
};
use rand::rngs::OsRng;
use rand::RngCore;
use std::collections::BTreeMap;

use crate::wallet::shielded::keys::OrchardKeySet;
use crate::wallet::shielded::prover::CachedOrchardProver;

/// The Orchard note encryptor with Dash's 36-byte memo.
type OrchardNoteEncryption = grovedb_commitment_tree::OrchardNoteEncryption<DashMemo>;

/// Fake transparent-side signer (see `shield_decrypt_tests::DummySigner`):
/// the input witnesses sign the transparent inputs, not the Orchard
/// bundle, so a dummy 65-byte signature suffices for note encryption.
#[derive(Debug)]
struct DummySigner;

#[async_trait::async_trait]
impl Signer<PlatformAddress> for DummySigner {
    async fn sign(
        &self,
        _key: &PlatformAddress,
        _data: &[u8],
    ) -> Result<BinaryData, ProtocolError> {
        Ok(BinaryData::new(vec![0u8; 65]))
    }

    async fn sign_create_witness(
        &self,
        _key: &PlatformAddress,
        _data: &[u8],
    ) -> Result<AddressWitness, ProtocolError> {
        Ok(AddressWitness::P2pkh {
            signature: BinaryData::new(vec![0u8; 65]),
        })
    }

    fn can_sign_with(&self, _key: &PlatformAddress) -> bool {
        true
    }
}

/// The memo text under test. Verified ≤ 32 bytes at the top of each test
/// (the 🍕 is 4 bytes, so "thanks for lunch 🍕" is 17 + 4 = 21 bytes).
const MEMO_TEXT: &str = "thanks for lunch 🍕";

/// Build a real Orchard output encrypted to `recipient` with `ovk`
/// set (the wallet's own OVK), returning the on-chain
/// `ShieldedEncryptedNote` wire form. Mirrors `ovk_recovery_tests::
/// make_outgoing_wire_note`.
fn make_own_ovk_wire_note(
    recipient: Address,
    ovk: OutgoingViewingKey,
    value_credits: u64,
    memo: [u8; 36],
) -> ShieldedEncryptedNote {
    let mut rng = OsRng;

    // In Orchard the output note's rho == the spent note's nullifier
    // (the same 32-byte Pallas-base element). Draw one valid element and
    // use it for both the wire `nullifier` field and the note's `rho`.
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

    // A consistent value commitment point — the SAME `cv` feeds both the
    // outgoing-ciphertext encryption and (stored as `cv_net`) the
    // recovery's `derive_ock`.
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

/// IVK side: a memo attached to a Type 15 Shield output is recovered,
/// byte-for-byte and back to its text, by the full incoming-decryption
/// primitive under the recipient's IVK.
#[tokio::test]
async fn shield_memo_round_trips_through_ivk_decryption() {
    let memo_bytes = ShieldedMemo::text(MEMO_TEXT)
        .expect("memo must be within the 32-byte limit")
        .to_bytes();

    let seed = [0x42u8; 32];
    let keys = OrchardKeySet::from_seed(&seed, Network::Testnet, 0)
        .expect("ZIP-32 derivation from a fixed seed should succeed");

    // Recipient = the wallet's own default address, as `operations::shield`
    // derives via `default_orchard_address`.
    let recipient = OrchardAddress::from_raw_bytes(&keys.default_address.to_raw_address_bytes())
        .expect("default address must convert to OrchardAddress");

    let amount: u64 = 200_000_000_000; // 2 DASH in credits
    let mut inputs = BTreeMap::new();
    inputs.insert(
        PlatformAddress::P2pkh([0xAB; 20]),
        (0u32, 500_000_000_000u64),
    );

    // `OrchardProver` is impl'd for `&CachedOrchardProver`, so the
    // builder's `&P` is a double reference — the same shape
    // `operations::shield` passes.
    let prover = CachedOrchardProver::new();
    let st = build_shield_transition(
        &recipient,
        amount,
        inputs,
        vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
        &DummySigner,
        0,
        &&prover,
        memo_bytes,
        Some(keys.outgoing_viewing_key.clone()),
        PlatformVersion::latest(),
    )
    .await
    .expect("shield transition build should succeed");

    let StateTransition::Shield(ShieldTransition::V0(v0)) = st else {
        panic!("expected a Shield state transition");
    };

    // Output-only bundles pad to exactly 2 actions; exactly one (the real
    // recipient output) decrypts under our IVK.
    let ivk = keys.prepared_ivk();
    let recovered: Vec<(u64, [u8; 36])> = v0
        .actions
        .iter()
        .filter_map(|a| {
            let wire = ShieldedEncryptedNote {
                cmx: a.cmx.to_vec(),
                nullifier: a.nullifier.to_vec(),
                cv_net: a.cv_net.to_vec(),
                encrypted_note: a.encrypted_note.clone(),
            };
            try_decrypt_note_with_memo(&ivk, &wire)
                .map(|(note, _addr, memo)| (note.value().inner(), memo))
        })
        .collect();

    assert_eq!(
        recovered.len(),
        1,
        "exactly one action (the real recipient output) must decrypt with its memo"
    );
    let (value, memo) = recovered[0];
    assert_eq!(
        value, amount,
        "decrypted note value must equal the shielded amount"
    );
    assert_eq!(
        memo, memo_bytes,
        "the recovered memo bytes must equal the bytes we encrypted"
    );
    assert_eq!(
        ShieldedMemo::from_bytes(&memo),
        ShieldedMemo::Text(MEMO_TEXT.to_string()),
        "the recovered memo must decode back to the original text"
    );
}

/// OVK side: a memo attached to a note the wallet encrypted with its OWN
/// OVK is recovered, byte-for-byte and back to its text, by the
/// send-history recovery primitive under the sender's OVK.
#[test]
fn shield_memo_round_trips_through_ovk_recovery() {
    let memo_bytes = ShieldedMemo::text(MEMO_TEXT)
        .expect("memo must be within the 32-byte limit")
        .to_bytes();

    let sender = OrchardKeySet::from_seed(&[0x42u8; 32], Network::Testnet, 0)
        .expect("ZIP-32 derivation from a fixed seed should succeed");
    let recipient_addr = sender.address_at(7);
    let value = 123_456u64;

    let wire = make_own_ovk_wire_note(
        recipient_addr,
        sender.outgoing_viewing_key.clone(),
        value,
        memo_bytes,
    );

    let (note, _recipient, recovered_memo) =
        try_recover_outgoing_note(&sender.outgoing_viewing_key, &wire)
            .expect("the wallet's own OVK must recover the note it sent");
    assert_eq!(note.value().inner(), value, "recovered value mismatch");
    assert_eq!(
        recovered_memo, memo_bytes,
        "the OVK-recovered memo bytes must equal the bytes we encrypted"
    );
    assert_eq!(
        ShieldedMemo::from_bytes(&recovered_memo),
        ShieldedMemo::Text(MEMO_TEXT.to_string()),
        "the OVK-recovered memo must decode back to the original text"
    );
}
