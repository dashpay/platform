use dpp::consensus::state::shielded::invalid_shielded_proof_error::InvalidShieldedProofError;
use dpp::shielded::SerializedAction;
use grovedb_commitment_tree::{
    Action, Anchor, Authorized, Bundle, ExtractedNoteCommitment, Flags, Nullifier, Proof,
    VerifyingKey,
};
use orchard::note::TransmittedNoteCiphertext;
use orchard::primitives::redpallas;
use orchard::value::ValueCommitment;
use std::sync::OnceLock;

/// Cached verifying key for shielded proof verification.
///
/// The key is deterministic (same circuit → same key) and immutable.
/// Building it takes ~5s, so it's lazily initialized on first use.
static SHIELDED_VERIFYING_KEY: OnceLock<VerifyingKey> = OnceLock::new();

fn get_verifying_key() -> &'static VerifyingKey {
    SHIELDED_VERIFYING_KEY.get_or_init(VerifyingKey::build)
}

const EPK_SIZE: usize = 32;
const ENC_CIPHERTEXT_SIZE: usize = 580;
const OUT_CIPHERTEXT_SIZE: usize = 80;
const ENCRYPTED_NOTE_SIZE: usize = EPK_SIZE + ENC_CIPHERTEXT_SIZE + OUT_CIPHERTEXT_SIZE; // 692

/// Reconstructs an orchard `Bundle<Authorized, i64>` from the serialized fields
/// of a shielded state transition and verifies the Halo 2 ZK proof.
///
/// Returns `Ok(())` if the proof is valid, or an `InvalidShieldedProofError`
/// wrapped in `Error` if reconstruction or verification fails.
pub fn reconstruct_and_verify_bundle(
    actions: &[SerializedAction],
    flags: u8,
    value_balance: i64,
    anchor: &[u8; 32],
    proof: &[u8],
    binding_signature: &[u8],
) -> Result<(), InvalidShieldedProofError> {
    let vk = get_verifying_key();

    // Reconstruct each Action
    let mut orchard_actions = Vec::with_capacity(actions.len());
    for a in actions {
        // Parse encrypted_note (692 bytes = epk 32 + enc 580 + out 80)
        if a.encrypted_note.len() != ENCRYPTED_NOTE_SIZE {
            return Err(InvalidShieldedProofError::new(format!(
                "encrypted note size mismatch: expected {ENCRYPTED_NOTE_SIZE}, got {}",
                a.encrypted_note.len()
            )));
        }
        let epk_bytes: [u8; 32] = a.encrypted_note[..EPK_SIZE].try_into().unwrap();
        let enc_ciphertext: [u8; ENC_CIPHERTEXT_SIZE] = a.encrypted_note
            [EPK_SIZE..EPK_SIZE + ENC_CIPHERTEXT_SIZE]
            .try_into()
            .unwrap();
        let out_ciphertext: [u8; OUT_CIPHERTEXT_SIZE] = a.encrypted_note
            [EPK_SIZE + ENC_CIPHERTEXT_SIZE..]
            .try_into()
            .unwrap();

        let nullifier: Nullifier =
            Option::from(Nullifier::from_bytes(&a.nullifier)).ok_or_else(|| {
                InvalidShieldedProofError::new("invalid nullifier bytes".to_string())
            })?;

        let rk = redpallas::VerificationKey::try_from(a.rk).map_err(|e| {
            InvalidShieldedProofError::new(format!("invalid spend validating key: {e}"))
        })?;

        let cmx: ExtractedNoteCommitment =
            Option::from(ExtractedNoteCommitment::from_bytes(&a.cmx)).ok_or_else(|| {
                InvalidShieldedProofError::new("invalid note commitment bytes".to_string())
            })?;

        let cv_net: ValueCommitment =
            Option::from(ValueCommitment::from_bytes(&a.cv_net)).ok_or_else(|| {
                InvalidShieldedProofError::new("invalid value commitment bytes".to_string())
            })?;

        let spend_auth_sig_bytes: [u8; 64] =
            a.spend_auth_sig.as_slice().try_into().map_err(|_| {
                InvalidShieldedProofError::new(format!(
                    "spend auth signature size mismatch: expected 64, got {}",
                    a.spend_auth_sig.len()
                ))
            })?;

        let action = Action::from_parts(
            nullifier,
            rk,
            cmx,
            TransmittedNoteCiphertext {
                epk_bytes,
                enc_ciphertext,
                out_ciphertext,
            },
            cv_net,
            redpallas::Signature::from(spend_auth_sig_bytes),
        );
        orchard_actions.push(action);
    }

    // Reconstruct Authorized (proof + binding signature)
    let binding_sig_bytes: [u8; 64] = binding_signature.try_into().map_err(|_| {
        InvalidShieldedProofError::new(format!(
            "binding signature size mismatch: expected 64, got {}",
            binding_signature.len()
        ))
    })?;

    let authorized = Authorized::from_parts(
        Proof::new(proof.to_vec()),
        redpallas::Signature::from(binding_sig_bytes),
    );

    // Reconstruct Bundle
    let orchard_flags = Flags::from_byte(flags).ok_or_else(|| {
        InvalidShieldedProofError::new(format!("invalid bundle flags byte: {flags:#04x}"))
    })?;

    let orchard_anchor = Option::from(Anchor::from_bytes(*anchor)).ok_or_else(|| {
        InvalidShieldedProofError::new("invalid anchor bytes".to_string())
    })?;

    let actions_nonempty = nonempty::NonEmpty::from_vec(orchard_actions).ok_or_else(|| {
        InvalidShieldedProofError::new("bundle has no actions".to_string())
    })?;

    let bundle = Bundle::from_parts(
        actions_nonempty,
        orchard_flags,
        value_balance,
        orchard_anchor,
        authorized,
    );

    // Verify the Halo 2 proof
    bundle.verify_proof(vk).map_err(|e| {
        InvalidShieldedProofError::new(format!("proof verification failed: {e}"))
    })?;

    Ok(())
}
