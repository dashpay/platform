//! Compact trial decryption for shielded encrypted notes.

use drive_proof_verifier::types::ShieldedEncryptedNote;
use grovedb_commitment_tree::{
    try_compact_note_decryption, CompactAction, DashMemo, EphemeralKeyBytes,
    ExtractedNoteCommitment, Note, Nullifier, OrchardDomain, PaymentAddress,
    PreparedIncomingViewingKey, COMPACT_NOTE_SIZE,
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
