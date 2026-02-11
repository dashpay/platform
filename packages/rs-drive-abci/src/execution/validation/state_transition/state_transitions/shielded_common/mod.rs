use dpp::consensus::state::shielded::invalid_shielded_proof_error::InvalidShieldedProofError;
use dpp::shielded::SerializedAction;
use grovedb_commitment_tree::{
    Action, Anchor, Authorized, BatchValidator, Bundle, ExtractedNoteCommitment, Flags, Nullifier,
    Proof, VerifyingKey,
};
use orchard::note::TransmittedNoteCiphertext;
use orchard::primitives::redpallas;
use orchard::value::ValueCommitment;
use sha2::{Digest, Sha256};
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

/// Domain separator for Platform sighash computation.
const SIGHASH_DOMAIN: &[u8] = b"DashPlatformSighash";

/// Computes the platform sighash from an Orchard bundle commitment and optional
/// transparent field data.
///
/// The sighash is computed as:
///   `SHA-256(SIGHASH_DOMAIN || bundle_commitment || extra_data)`
///
/// This binds transparent state transition fields (like `output_address` and `amount`
/// in unshield transitions) to the Orchard signatures, preventing replay attacks
/// where an attacker substitutes transparent fields while reusing a valid Orchard bundle.
///
/// The same computation must be used on both the signing (client) and verification
/// (platform) sides. For transitions without transparent fields (shield and
/// shielded_transfer), `extra_data` is empty.
pub fn compute_platform_sighash(bundle_commitment: &[u8; 32], extra_data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(SIGHASH_DOMAIN);
    hasher.update(bundle_commitment);
    hasher.update(extra_data);
    hasher.finalize().into()
}

/// Reconstructs an orchard `Bundle<Authorized, i64>` from the serialized fields
/// of a shielded state transition and verifies the Halo 2 ZK proof along with
/// all RedPallas signatures (spend auth + binding).
///
/// Uses `BatchValidator` which verifies:
/// 1. The Halo 2 circuit proof (zero-knowledge proof of spend validity)
/// 2. Spend authorization signatures (proves the spender controls the spending key)
/// 3. The binding signature (binds value_balance to value commitments, preventing manipulation)
///
/// The sighash is computed via `compute_platform_sighash()`, which hashes the
/// Orchard bundle commitment together with `extra_sighash_data` (transparent fields).
/// The same computation must be used when signing the bundle on the client side.
///
/// `extra_sighash_data` binds transparent fields to the Orchard signatures:
/// - Shield: empty (no transparent outputs)
/// - Shielded transfer: empty (no transparent fields)
/// - Unshield: `output_address.to_bytes() || amount.to_le_bytes()`
///
/// Returns `Ok(())` if all verification passes, or an `InvalidShieldedProofError`
/// if reconstruction or any verification step fails.
pub fn reconstruct_and_verify_bundle(
    actions: &[SerializedAction],
    flags: u8,
    value_balance: i64,
    anchor: &[u8; 32],
    proof: &[u8],
    binding_signature: &[u8; 64],
    extra_sighash_data: &[u8],
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

        let nullifier: Nullifier = Option::from(Nullifier::from_bytes(&a.nullifier))
            .ok_or_else(|| InvalidShieldedProofError::new("invalid nullifier bytes".to_string()))?;

        let rk = redpallas::VerificationKey::try_from(a.rk).map_err(|e| {
            InvalidShieldedProofError::new(format!("invalid spend validating key: {e}"))
        })?;

        let cmx: ExtractedNoteCommitment =
            Option::from(ExtractedNoteCommitment::from_bytes(&a.cmx)).ok_or_else(|| {
                InvalidShieldedProofError::new("invalid note commitment bytes".to_string())
            })?;

        let cv_net: ValueCommitment = Option::from(ValueCommitment::from_bytes(&a.cv_net))
            .ok_or_else(|| {
                InvalidShieldedProofError::new("invalid value commitment bytes".to_string())
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
            redpallas::Signature::from(a.spend_auth_sig),
        );
        orchard_actions.push(action);
    }

    // Reconstruct Authorized (proof + binding signature)
    let authorized = Authorized::from_parts(
        Proof::new(proof.to_vec()),
        redpallas::Signature::from(*binding_signature),
    );

    // Reconstruct Bundle
    let orchard_flags = Flags::from_byte(flags).ok_or_else(|| {
        InvalidShieldedProofError::new(format!("invalid bundle flags byte: {flags:#04x}"))
    })?;

    let orchard_anchor = Option::from(Anchor::from_bytes(*anchor))
        .ok_or_else(|| InvalidShieldedProofError::new("invalid anchor bytes".to_string()))?;

    let actions_nonempty = nonempty::NonEmpty::from_vec(orchard_actions)
        .ok_or_else(|| InvalidShieldedProofError::new("bundle has no actions".to_string()))?;

    let bundle = Bundle::from_parts(
        actions_nonempty,
        orchard_flags,
        value_balance,
        orchard_anchor,
        authorized,
    );

    // Compute the platform sighash: SHA-256(domain || bundle_commitment || extra_data).
    // The bundle commitment is the Orchard BundleCommitment (BLAKE2b-256 per ZIP-244),
    // covering: flags, value_balance, anchor, and all action fields
    // (nullifier, rk, cmx, cv_net, encrypted_note) — but NOT the signatures or proof.
    // The extra_sighash_data binds transparent fields (e.g., output_address for unshield).
    let bundle_commitment: [u8; 32] = bundle.commitment().into();
    let sighash = compute_platform_sighash(&bundle_commitment, extra_sighash_data);

    // Verify the Halo 2 proof AND all RedPallas signatures (spend auth + binding)
    // using BatchValidator. This is the correct Orchard verification flow, ensuring:
    // - The ZK circuit proof is valid
    // - Each spend auth signature is valid for (rk, sighash)
    // - The binding signature is valid for (binding_validating_key, sighash)
    let mut batch = BatchValidator::new();
    batch.add_bundle(&bundle, sighash);

    let mut rng = rand::rngs::OsRng;
    if !batch.validate(vk, &mut rng) {
        return Err(InvalidShieldedProofError::new(
            "bundle verification failed: proof, spend auth signatures, or binding signature invalid"
                .to_string(),
        ));
    }

    Ok(())
}
