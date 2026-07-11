//! Trial decryption + OVK recovery for shielded encrypted notes.
//!
//! Two complementary scan-side primitives operate on the same
//! [`ShieldedEncryptedNote`] wire shape:
//!
//! - [`try_decrypt_note`] — **incoming** notes. Compact trial
//!   decryption under the wallet's Incoming Viewing Key; recovers the
//!   notes the wallet *received*.
//! - [`try_recover_outgoing_note`] — **outgoing** notes. OVK recovery
//!   (the Zcash outgoing-transaction-history mechanism) under the
//!   wallet's Outgoing Viewing Key; recovers the notes the wallet
//!   *sent* (recipient, value, memo) by opening `out_ciphertext`.
//!
//! Only the sender's OVK can open `out_ciphertext`, so a note the
//! wallet merely received does not OVK-recover under its own OVK —
//! exactly the intended asymmetry.

use drive_proof_verifier::types::ShieldedEncryptedNote;
use grovedb_commitment_tree::{
    try_compact_note_decryption, try_note_decryption, try_output_recovery_with_ovk, CompactAction,
    DashMemo, EphemeralKeyBytes, ExtractedNoteCommitment, Note, NoteBytes, NoteBytesData,
    Nullifier, OrchardDomain, OutgoingViewingKey, PaymentAddress, PreparedIncomingViewingKey,
    ValueCommitment, COMPACT_NOTE_SIZE,
};

/// Minimum length of the `encrypted_note` field for compact trial decryption.
///
/// The `encrypted_note` field layout is:
///   `epk(32) || enc_ciphertext(104) || out_ciphertext(80)` = 216 bytes
///
/// For compact decryption we need at least `epk(32) + COMPACT_NOTE_SIZE` bytes
/// of the enc_ciphertext.
const MIN_ENCRYPTED_NOTE_LEN: usize = 32 + COMPACT_NOTE_SIZE;

/// Attempt compact trial decryption on a [`ShieldedEncryptedNote`].
///
/// The [`ShieldedEncryptedNote`] struct (from proof verification) has three
/// separate fields:
///   - `cmx`: note commitment (32 bytes)
///   - `nullifier`: nullifier (32 bytes) — used for Rho derivation
///   - `encrypted_note`: `epk(32) || enc_ciphertext(104) || out_ciphertext(80)`
///
/// Returns `Some((note, address))` if the note decrypts successfully under the
/// given incoming viewing key, or `None` if it does not belong to the viewer
/// (including dummy/padding notes).
pub fn try_decrypt_note(
    ivk: &PreparedIncomingViewingKey,
    encrypted_note: &ShieldedEncryptedNote,
) -> Option<(Note, PaymentAddress)> {
    let data = &encrypted_note.encrypted_note;
    if data.len() < MIN_ENCRYPTED_NOTE_LEN {
        return None;
    }

    // Parse nullifier from the dedicated field (32 bytes)
    let nf_bytes: [u8; 32] = encrypted_note.nullifier.as_slice().try_into().ok()?;
    let nf = Nullifier::from_bytes(&nf_bytes).into_option()?;

    // Parse cmx from the dedicated field (32 bytes)
    let cmx_bytes: [u8; 32] = encrypted_note.cmx.as_slice().try_into().ok()?;
    let cmx = ExtractedNoteCommitment::from_bytes(&cmx_bytes).into_option()?;

    // Parse ephemeral public key (first 32 bytes of encrypted_note)
    let epk_bytes: [u8; 32] = data[0..32].try_into().ok()?;

    // Parse compact ciphertext (first COMPACT_NOTE_SIZE bytes of enc_ciphertext,
    // starting at byte 32)
    let enc_compact: [u8; COMPACT_NOTE_SIZE] = data[32..32 + COMPACT_NOTE_SIZE].try_into().ok()?;

    // Build CompactAction and OrchardDomain for trial decryption
    let compact = CompactAction::from_parts(nf, cmx, EphemeralKeyBytes(epk_bytes), enc_compact);
    let domain = OrchardDomain::<DashMemo>::for_compact_action(&compact);

    try_compact_note_decryption(&domain, ivk, &compact)
}

/// Full `enc_ciphertext` length for a Dash-memo Orchard note
/// (`NoteBytesData<104>` per `orchard::memo::DashMemo`).
const ENC_CIPHERTEXT_SIZE: usize = 104;
/// `out_ciphertext` length (`OUT_PLAINTEXT_SIZE` 64 + `AEAD_TAG_SIZE`
/// 16 in `zcash_note_encryption`).
const OUT_CIPHERTEXT_SIZE: usize = 80;
/// Byte length of the recovered Dash memo (`DashMemo::Memo = [u8; 36]`).
pub const DASH_MEMO_SIZE: usize = 36;
/// Required `encrypted_note` length for OVK recovery:
/// `epk(32) || enc_ciphertext(104) || out_ciphertext(80)` = 216.
const FULL_ENCRYPTED_NOTE_LEN: usize = 32 + ENC_CIPHERTEXT_SIZE + OUT_CIPHERTEXT_SIZE;

/// A minimal [`ShieldedOutput`] implementation for OVK recovery.
///
/// OVK recovery (`try_output_recovery_with_ovk`) needs an
/// `impl ShieldedOutput<OrchardDomain<DashMemo>>` exposing the
/// output's `epk`, `cmx`, and **full** `enc_ciphertext` (104 bytes —
/// not just the 52-byte compact prefix the IVK path uses). A real
/// `orchard::Action` would satisfy the trait, but it additionally
/// carries `rk` (the spend-auth randomized verification key), `cv`,
/// and a proof — none of which a client stores per note. So this
/// struct carries exactly the three quantities the trait reads and
/// nothing more; `cv` and the domain are supplied separately to the
/// recovery call from the stored `cv_net` and `nullifier`.
struct RecoverableOutput {
    /// Extracted note commitment (cmx).
    cmx: ExtractedNoteCommitment,
    /// Ephemeral public key bytes (epk).
    epk: EphemeralKeyBytes,
    /// Full enc_ciphertext (104 bytes for a Dash-memo note).
    enc: NoteBytesData<ENC_CIPHERTEXT_SIZE>,
}

impl grovedb_commitment_tree::ShieldedOutput<OrchardDomain<DashMemo>> for RecoverableOutput {
    fn ephemeral_key(&self) -> EphemeralKeyBytes {
        EphemeralKeyBytes(self.epk.0)
    }

    fn cmstar(&self) -> &ExtractedNoteCommitment {
        &self.cmx
    }

    fn enc_ciphertext(&self) -> Option<&NoteBytesData<ENC_CIPHERTEXT_SIZE>> {
        // MUST be `Some` for full recovery: the OVK path splits the
        // AEAD tag off this full ciphertext to recover the note
        // plaintext (the compact path that returns `None` here can
        // only ever trial-decrypt, never recover the sender's view).
        Some(&self.enc)
    }

    fn enc_ciphertext_compact(&self) -> NoteBytesData<COMPACT_NOTE_SIZE> {
        // The first `COMPACT_NOTE_SIZE` (52) bytes of the full
        // ciphertext. `from_slice` is infallible here because `self.enc`
        // is exactly `ENC_CIPHERTEXT_SIZE` (104) >= 52 bytes.
        NoteBytesData::<COMPACT_NOTE_SIZE>::from_slice(&self.enc.as_ref()[..COMPACT_NOTE_SIZE])
            .expect("enc is at least COMPACT_NOTE_SIZE bytes")
    }
}

/// Attempt OVK recovery on a [`ShieldedEncryptedNote`].
///
/// This is the sender-side counterpart to [`try_decrypt_note`]: it
/// uses the wallet's **Outgoing** Viewing Key to recover a note the
/// wallet *sent* — the Zcash outgoing-transaction-history mechanism.
/// On success it yields the `(note, recipient, memo)` the wallet
/// originally encrypted, so the wallet can reconstruct its own
/// send history (who it paid, how much, with what memo) purely from
/// the on-chain encrypted notes.
///
/// `out_ciphertext` is keyed by an outgoing cipher key derived from
/// the OVK **and** the value commitment `cv_net` (`derive_ock`), so
/// the stored `cv_net` (retained per note on-chain in stage 1/2) is
/// the missing input that makes this recoverable client-side.
///
/// Returns `Some((note, recipient, memo))` when the note opens under
/// `ovk`, or `None` when it does not (a note the wallet received but
/// did not send, a dummy/padding output, or any malformed field).
/// The returned `memo` is the raw [`DASH_MEMO_SIZE`]-byte Dash memo.
pub fn try_recover_outgoing_note(
    ovk: &OutgoingViewingKey,
    encrypted_note: &ShieldedEncryptedNote,
) -> Option<(Note, PaymentAddress, [u8; DASH_MEMO_SIZE])> {
    let data = &encrypted_note.encrypted_note;
    if data.len() < FULL_ENCRYPTED_NOTE_LEN {
        return None;
    }

    // cmx (32 bytes) from the dedicated field.
    let cmx_bytes: [u8; 32] = encrypted_note.cmx.as_slice().try_into().ok()?;
    let cmx = ExtractedNoteCommitment::from_bytes(&cmx_bytes).into_option()?;

    // Nullifier (32 bytes) drives the domain's `rho` — the output
    // note's rho equals the spent note's nullifier in Orchard, so
    // `OrchardDomain::for_nullifier` reconstructs the same domain a
    // full `Action` would (`for_action` reads only `act.rho()`).
    let nf_bytes: [u8; 32] = encrypted_note.nullifier.as_slice().try_into().ok()?;
    let nf = Nullifier::from_bytes(&nf_bytes).into_option()?;

    // Value commitment `cv_net` (32 bytes) — the input to `derive_ock`
    // that unlocks `out_ciphertext`.
    let cv_bytes: [u8; 32] = encrypted_note.cv_net.as_slice().try_into().ok()?;
    let cv = ValueCommitment::from_bytes(&cv_bytes).into_option()?;

    // Parse the three slices out of `encrypted_note`:
    //   epk            [0..32]
    //   enc_ciphertext [32..136]   (104 bytes)
    //   out_ciphertext [136..216]  (80 bytes)
    let epk_bytes: [u8; 32] = data[0..32].try_into().ok()?;
    let enc_bytes: [u8; ENC_CIPHERTEXT_SIZE] =
        data[32..32 + ENC_CIPHERTEXT_SIZE].try_into().ok()?;
    let out_bytes: [u8; OUT_CIPHERTEXT_SIZE] = data
        [32 + ENC_CIPHERTEXT_SIZE..32 + ENC_CIPHERTEXT_SIZE + OUT_CIPHERTEXT_SIZE]
        .try_into()
        .ok()?;

    let output = RecoverableOutput {
        cmx,
        epk: EphemeralKeyBytes(epk_bytes),
        enc: NoteBytesData(enc_bytes),
    };
    let domain = OrchardDomain::<DashMemo>::for_nullifier(nf);

    try_output_recovery_with_ovk(&domain, ovk, &output, &cv, &out_bytes)
}

/// Attempt FULL incoming trial decryption on a [`ShieldedEncryptedNote`],
/// recovering the memo alongside the note.
///
/// The sibling [`try_decrypt_note`] uses the 52-byte *compact* prefix
/// of `enc_ciphertext` and so cannot recover the memo (the memo lives
/// past the compact region). This variant decrypts the FULL 104-byte
/// `enc_ciphertext` via `try_note_decryption`, yielding the
/// `(note, recipient, memo)` the sender encrypted to the wallet's
/// incoming viewing key. It reuses the same [`RecoverableOutput`] the
/// OVK path uses (it exposes the full ciphertext) and reconstructs the
/// domain from the stored nullifier — the output note's `rho` equals
/// the spent note's nullifier in Orchard, so
/// `OrchardDomain::for_nullifier` builds the same domain a full
/// `Action` would.
///
/// Returns `Some((note, address, memo))` if the note decrypts under the
/// given incoming viewing key, or `None` if it does not belong to the
/// viewer (including dummy/padding notes or any malformed field). The
/// returned `memo` is the raw [`DASH_MEMO_SIZE`]-byte Dash memo.
pub fn try_decrypt_note_with_memo(
    ivk: &PreparedIncomingViewingKey,
    encrypted_note: &ShieldedEncryptedNote,
) -> Option<(Note, PaymentAddress, [u8; DASH_MEMO_SIZE])> {
    let data = &encrypted_note.encrypted_note;
    if data.len() < FULL_ENCRYPTED_NOTE_LEN {
        return None;
    }

    // cmx (32 bytes) from the dedicated field.
    let cmx_bytes: [u8; 32] = encrypted_note.cmx.as_slice().try_into().ok()?;
    let cmx = ExtractedNoteCommitment::from_bytes(&cmx_bytes).into_option()?;

    // Nullifier (32 bytes) drives the domain's `rho` (== the spent
    // note's nullifier), same as the OVK path above.
    let nf_bytes: [u8; 32] = encrypted_note.nullifier.as_slice().try_into().ok()?;
    let nf = Nullifier::from_bytes(&nf_bytes).into_option()?;

    // epk [0..32] and the FULL enc_ciphertext [32..136] (104 bytes) —
    // unlike the compact path, the memo lives inside this region.
    let epk_bytes: [u8; 32] = data[0..32].try_into().ok()?;
    let enc_bytes: [u8; ENC_CIPHERTEXT_SIZE] =
        data[32..32 + ENC_CIPHERTEXT_SIZE].try_into().ok()?;

    let output = RecoverableOutput {
        cmx,
        epk: EphemeralKeyBytes(epk_bytes),
        enc: NoteBytesData(enc_bytes),
    };
    let domain = OrchardDomain::<DashMemo>::for_nullifier(nf);

    try_note_decryption(&domain, ivk, &output)
}
