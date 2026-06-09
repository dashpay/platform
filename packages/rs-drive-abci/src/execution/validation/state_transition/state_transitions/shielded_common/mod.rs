use crate::error::Error;
use dpp::consensus::state::shielded::insufficient_pool_notes_error::InsufficientPoolNotesError;
use dpp::consensus::state::shielded::invalid_anchor_error::InvalidAnchorError;
use dpp::consensus::state::shielded::invalid_shielded_proof_error::InvalidShieldedProofError;
use dpp::consensus::state::shielded::nullifier_already_spent_error::NullifierAlreadySpentError;
use dpp::consensus::state::state_error::StateError;
use dpp::fee::Credits;
use dpp::prelude::ConsensusValidationResult;
pub use dpp::shielded::compute_platform_sighash;
use dpp::shielded::SerializedAction;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::fees::op::LowLevelDriveOperation;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::StateTransitionAction;
use grovedb_commitment_tree::{
    redpallas, Action, ActionFromPartsError, Anchor, Authorized, BatchValidator, Bundle, DashMemo,
    ExtractedNoteCommitment, Flags, NoteBytesData, Nullifier, Proof, ProofSizeEnforcement,
    TransmittedNoteCiphertext, ValueCommitment, VerifyingKey,
};
use std::sync::OnceLock;

/// Orchard bundle flags byte: only outputs are real (spends are dummy).
/// Used for shield and shield-from-asset-lock transitions where funds enter the pool.
pub const FLAGS_OUTPUTS_ONLY: u8 = 0x02;

/// Orchard bundle flags byte: both spends and outputs are real.
/// Used for shielded transfers, unshield, and shielded-withdrawal transitions.
pub const FLAGS_SPENDS_AND_OUTPUTS: u8 = 0x03;

/// Cached verifying key for shielded proof verification.
///
/// The key is deterministic (same circuit → same key) and immutable.
/// Building it takes ~5s, so it's lazily initialized on first use.
static SHIELDED_VERIFYING_KEY: OnceLock<VerifyingKey> = OnceLock::new();

fn get_verifying_key() -> &'static VerifyingKey {
    SHIELDED_VERIFYING_KEY.get_or_init(VerifyingKey::build)
}

/// Pre-builds the shielded verifying key so that the first shielded
/// transaction does not pay the ~5-15 s construction cost at check_tx time.
pub fn warmup_shielded_verifying_key() {
    get_verifying_key();
}

const EPK_SIZE: usize = 32;
const ENC_CIPHERTEXT_SIZE: usize = 104;
const OUT_CIPHERTEXT_SIZE: usize = 80;

// Import the canonical constant from DPP (single source of truth).
use dpp::state_transition::state_transitions::shielded::common_validation::ENCRYPTED_NOTE_SIZE;

// Compile-time check: component sizes must sum to the canonical constant.
const _: () = assert!(
    EPK_SIZE + ENC_CIPHERTEXT_SIZE + OUT_CIPHERTEXT_SIZE == ENCRYPTED_NOTE_SIZE,
    "component sizes diverged from ENCRYPTED_NOTE_SIZE"
);

/// Reconstructs an orchard `Bundle<Authorized, i64, DashMemo>` from the serialized fields
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
/// `extra_sighash_data` binds transparent fields to the Orchard signatures (built by the
/// shared `dpp::shielded::*_extra_sighash_data` helpers so the signer and verifier agree):
/// - Shield: empty (no transparent outputs)
/// - Shielded transfer: empty (no transparent fields)
/// - Unshield: `output_address || unshielding_amount (u64 LE)`
/// - Shielded withdrawal: `output_script || unshielding_amount (u64 LE) || core_fee_per_byte
///   (u32 LE) || pooling (u8)` — every Core-facing field the withdrawal document commits to.
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
        // Parse encrypted_note (216 bytes = epk 32 + enc 104 + out 80)
        if a.encrypted_note.len() != ENCRYPTED_NOTE_SIZE {
            return Err(InvalidShieldedProofError::new(format!(
                "encrypted note size mismatch: expected {ENCRYPTED_NOTE_SIZE}, got {}",
                a.encrypted_note.len()
            )));
        }
        let epk_bytes: [u8; 32] = a.encrypted_note[..EPK_SIZE]
            .try_into()
            .expect("length verified to be ENCRYPTED_NOTE_SIZE");
        let enc_ciphertext: [u8; ENC_CIPHERTEXT_SIZE] = a.encrypted_note
            [EPK_SIZE..EPK_SIZE + ENC_CIPHERTEXT_SIZE]
            .try_into()
            .expect("length verified to be ENCRYPTED_NOTE_SIZE");
        let out_ciphertext: [u8; OUT_CIPHERTEXT_SIZE] = a.encrypted_note
            [EPK_SIZE + ENC_CIPHERTEXT_SIZE..]
            .try_into()
            .expect("length verified to be ENCRYPTED_NOTE_SIZE");

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

        // `Action::from_parts` rejects malformed actions instead of silently
        // dropping them. In orchard 0.14 it returns `Result<_, ActionFromPartsError>`
        // (was `Option` in 0.13) and now enforces TWO invariants:
        //   - `IdentityRk`: the randomizer key `rk` must be non-identity (the
        //     hardening that already existed in 0.13).
        //   - `InvalidEpk`: the ephemeral public key `epk` must encode a
        //     non-identity point (a NEW invariant in 0.14 — the circuit
        //     soundness fix). Rejecting this is REQUIRED to preserve soundness;
        //     we must not weaken it back to acceptance.
        // We keep the original "identity randomizer key" message for the
        // `IdentityRk` case (byte-for-byte compatible with the 0.13 error path)
        // and surface the new `InvalidEpk` rejection with its own message.
        let action = Action::from_parts(
            nullifier,
            rk,
            cmx,
            TransmittedNoteCiphertext::<DashMemo>::from_parts(
                epk_bytes,
                NoteBytesData(enc_ciphertext),
                out_ciphertext,
            ),
            cv_net,
            redpallas::Signature::from(a.spend_auth_sig),
        )
        .map_err(|e| match e {
            ActionFromPartsError::IdentityRk => {
                InvalidShieldedProofError::new("action has identity randomizer key".to_string())
            }
            ActionFromPartsError::InvalidEpk => InvalidShieldedProofError::new(
                "action has invalid ephemeral public key (identity or undecodable epk)".to_string(),
            ),
            // `ActionFromPartsError` is `#[non_exhaustive]`. Any future
            // rejection variant added upstream MUST also be rejected here —
            // defaulting to acceptance would weaken consensus soundness.
            other => InvalidShieldedProofError::new(format!("malformed orchard action: {other}")),
        })?;
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

    // Reconstruct the `Bundle<Authorized>` (`try_from_parts` is orchard 0.14's only
    // public constructor for it). `ProofSizeEnforcement::Strict` rejects a proof
    // whose byte-length is not canonical for the action count — anti-malleability.
    // Spend-auth signatures were attached per-action in `Action::from_parts` above,
    // so action↔signature pairing is preserved with no separate list to reorder.
    let bundle = Bundle::try_from_parts(
        actions_nonempty,
        orchard_flags,
        value_balance,
        orchard_anchor,
        authorized,
        ProofSizeEnforcement::Strict,
    )
    .map_err(|e| {
        InvalidShieldedProofError::new(format!("failed to reconstruct authorized bundle: {e}"))
    })?;

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

    let rng = rand::rngs::OsRng;
    if !batch.validate(vk, rng) {
        return Err(InvalidShieldedProofError::new(
            "bundle verification failed: proof, spend auth signatures, or binding signature invalid"
                .to_string(),
        ));
    }

    Ok(())
}

/// Read the current shielded pool total balance from GroveDB.
/// Returns 0 if the balance key doesn't exist yet.
///
/// Delegates to `Drive::read_shielded_pool_total_balance`.
pub fn read_pool_total_balance(
    drive: &Drive,
    transaction: TransactionArg,
    drive_operations: &mut Vec<LowLevelDriveOperation>,
    platform_version: &PlatformVersion,
) -> Result<Credits, Error> {
    drive
        .read_shielded_pool_total_balance(transaction, drive_operations, platform_version)
        .map_err(Error::Drive)
}

/// Verify that the anchor exists in the recorded anchors tree.
/// Uses O(1) key lookup instead of scanning the entire tree.
/// Returns a consensus error if the anchor is not found.
///
/// Delegates to `Drive::has_shielded_anchor` for the GroveDB lookup.
pub fn validate_anchor_exists(
    drive: &Drive,
    anchor: &[u8; 32],
    transaction: TransactionArg,
    drive_operations: &mut Vec<LowLevelDriveOperation>,
    platform_version: &PlatformVersion,
) -> Result<Option<ConsensusValidationResult<StateTransitionAction>>, Error> {
    let found = drive
        .has_shielded_anchor(anchor, transaction, drive_operations, platform_version)
        .map_err(Error::Drive)?;

    if !found {
        Ok(Some(ConsensusValidationResult::new_with_error(
            StateError::InvalidAnchorError(InvalidAnchorError::new(*anchor)).into(),
        )))
    } else {
        Ok(None)
    }
}

/// Defense-in-depth: reject duplicate nullifiers within the same bundle,
/// then check that no nullifier has already been spent in state.
///
/// Phase 1 (intra-bundle HashSet check) stays here.
/// Phase 2 delegates to `Drive::has_nullifier` for each GroveDB lookup.
pub fn validate_nullifiers(
    drive: &Drive,
    nullifiers: &[[u8; 32]],
    transaction: TransactionArg,
    drive_operations: &mut Vec<LowLevelDriveOperation>,
    platform_version: &PlatformVersion,
) -> Result<Option<ConsensusValidationResult<StateTransitionAction>>, Error> {
    // Phase 1: Intra-bundle duplicate check (no GroveDB access)
    let mut seen_nullifiers = std::collections::HashSet::new();
    for nullifier in nullifiers {
        if !seen_nullifiers.insert(nullifier) {
            return Ok(Some(ConsensusValidationResult::new_with_error(
                StateError::NullifierAlreadySpentError(NullifierAlreadySpentError::new(*nullifier))
                    .into(),
            )));
        }
    }
    // Phase 2: Check against state via Drive method
    //
    // SAFETY: No cross-transaction double-spend risk within a block. State
    // transitions are processed sequentially: each transition's nullifier
    // insertions are applied to the GroveDB transaction (via apply_batch)
    // before the next transition's validation runs. GroveDB supports
    // read-your-own-writes, so this lookup sees nullifiers from all prior
    // transitions in the same block. The insert_only_known_to_not_already_exist_op
    // provides an additional safety net at batch application time.
    for nullifier in nullifiers {
        let exists = drive
            .has_nullifier(nullifier, transaction, drive_operations, platform_version)
            .map_err(Error::Drive)?;
        if exists {
            return Ok(Some(ConsensusValidationResult::new_with_error(
                StateError::NullifierAlreadySpentError(NullifierAlreadySpentError::new(*nullifier))
                    .into(),
            )));
        }
    }
    Ok(None)
}

/// Check minimum notes threshold for outgoing transitions (anonymity set).
///
/// Delegates to `Drive::shielded_pool_notes_count` for the GroveDB lookup.
/// The threshold check and consensus error wrapping stay here.
pub fn validate_minimum_pool_notes(
    drive: &Drive,
    transaction: TransactionArg,
    drive_operations: &mut Vec<LowLevelDriveOperation>,
    platform_version: &PlatformVersion,
) -> Result<Option<ConsensusValidationResult<StateTransitionAction>>, Error> {
    let min_notes = platform_version
        .drive_abci
        .validation_and_processing
        .event_constants
        .minimum_pool_notes_for_outgoing;
    if min_notes > 0 {
        let encrypted_notes_count = drive
            .shielded_pool_notes_count(transaction, drive_operations, platform_version)
            .map_err(Error::Drive)?;
        if encrypted_notes_count < min_notes {
            return Ok(Some(ConsensusValidationResult::new_with_error(
                StateError::InsufficientPoolNotesError(InsufficientPoolNotesError::new(
                    encrypted_notes_count,
                    min_notes,
                ))
                .into(),
            )));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::validation::state_transition::state_transitions::test_helpers::{
        create_dummy_serialized_action, insert_anchor_into_state, insert_dummy_encrypted_notes,
        insert_nullifier_into_state, set_pool_total_balance, setup_platform,
    };
    use dpp::consensus::state::state_error::StateError;

    // ==========================================
    // reconstruct_and_verify_bundle error paths
    // ==========================================

    mod reconstruct_and_verify_bundle_tests {
        use super::*;

        #[test]
        fn test_encrypted_note_size_mismatch_returns_error() {
            let mut action = create_dummy_serialized_action();
            action.encrypted_note = vec![0u8; 100]; // Wrong size (should be 216)

            let result = reconstruct_and_verify_bundle(
                &[action],
                FLAGS_SPENDS_AND_OUTPUTS,
                0,
                &[42u8; 32],
                &[0u8; 100],
                &[0u8; 64],
                &[],
            );

            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(
                err.message().contains("encrypted note size mismatch"),
                "expected encrypted note size mismatch error, got: {}",
                err.message()
            );
        }

        #[test]
        fn test_empty_actions_returns_bundle_no_actions_error() {
            let result = reconstruct_and_verify_bundle(
                &[], // No actions
                FLAGS_SPENDS_AND_OUTPUTS,
                0,
                &[42u8; 32],
                &[0u8; 100],
                &[0u8; 64],
                &[],
            );

            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(
                err.message().contains("bundle has no actions"),
                "expected 'bundle has no actions' error, got: {}",
                err.message()
            );
        }

        /// Tests that an invalid rk (spend validating key) returns an error.
        /// The dummy serialized action uses rk: [2u8; 32] which is not a valid
        /// RedPallas verification key encoding, triggering this error path.
        #[test]
        fn test_invalid_rk_returns_error() {
            let action = create_dummy_serialized_action();

            let result = reconstruct_and_verify_bundle(
                &[action],
                FLAGS_SPENDS_AND_OUTPUTS,
                0,
                &[42u8; 32],
                &[0u8; 100],
                &[0u8; 64],
                &[],
            );

            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(
                err.message().contains("invalid spend validating key"),
                "expected invalid spend validating key error, got: {}",
                err.message()
            );
        }

        /// Tests the invalid flags byte error path. With empty actions, the action
        /// loop is skipped and `Flags::from_byte(0xFF)` returns None, triggering
        /// the invalid flags error before the `nonempty::NonEmpty::from_vec` check.
        #[test]
        fn test_invalid_flags_byte_returns_error() {
            let result = reconstruct_and_verify_bundle(
                &[],  // No actions -- skip the action loop, hit flags check
                0xFF, // Invalid flags byte
                0,
                &[42u8; 32],
                &[0u8; 100],
                &[0u8; 64],
                &[],
            );

            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(
                err.message().contains("invalid bundle flags byte"),
                "expected invalid bundle flags error, got: {}",
                err.message()
            );
        }

        // ----------------------------------------------------------------
        // `Action::from_parts` rejection paths (orchard 0.14 `map_err` arms)
        //
        // These pin the two consensus-critical rejections that are surfaced
        // ONLY inside the `Action::from_parts(...).map_err(...)` arms of
        // `reconstruct_and_verify_bundle`:
        //   - `ActionFromPartsError::IdentityRk`   ("identity randomizer key")
        //   - `ActionFromPartsError::InvalidEpk`   ("invalid ephemeral public key")
        //
        // The earlier error-path tests above never reach those arms: they fail
        // at the encrypted-note-size check, the empty-actions check, the
        // `rk`-DECODE step ([2u8;32] is not a valid VK encoding, rejected
        // *before* `from_parts`), or the flags check. A future refactor that
        // mistakenly mapped `InvalidEpk` to acceptance would slip past CI
        // without these tests. The `InvalidEpk` rejection is the orchard 0.14
        // circuit-soundness fix and MUST stay a rejection.
        // ----------------------------------------------------------------

        use grovedb_commitment_tree::{
            redpallas, Anchor, Builder, BundleType, DashMemo, Flags as OrchardFlags,
            FullViewingKey, NoteValue, Scope, SpendingKey,
        };

        /// Builds a `SerializedAction` whose `nullifier`, `rk`, `cmx`, `cv_net`,
        /// and `encrypted_note` (including a real, non-identity `epk`) are all
        /// genuine, canonically-encoded Orchard values — so an action built from
        /// it decodes cleanly through nullifier → rk → cmx → cv_net and actually
        /// REACHES `Action::from_parts`. The bytes are read off a real
        /// (unauthorized) output-only Orchard bundle; this needs NO proving key
        /// (we never call `create_proof`), so it is cheap.
        ///
        /// Tests then mutate exactly one field to exercise a single `from_parts`
        /// rejection arm. The function asserts each base field decodes, so if a
        /// future orchard encoding change broke the precondition the test would
        /// fail LOUDLY here rather than silently passing for the wrong reason.
        fn valid_base_serialized_action() -> dpp::shielded::SerializedAction {
            let sk = SpendingKey::from_bytes([0u8; 32]).expect("valid spending key");
            let fvk = FullViewingKey::from(&sk);
            let recipient = fvk.address_at(0u32, Scope::External);

            let mut builder = Builder::<DashMemo>::new(
                BundleType::Transactional {
                    flags: OrchardFlags::SPENDS_DISABLED,
                    bundle_required: false,
                },
                Anchor::empty_tree(),
            );
            builder
                .add_output(None, recipient, NoteValue::from_raw(5_000), [0u8; 36])
                .expect("add_output");

            let mut rng = rand::rngs::OsRng;
            let (unauthorized, _) = builder
                .build::<i64>(&mut rng)
                .expect("build unauthorized bundle")
                .expect("bundle is non-empty");

            // Read genuine, canonically-encoded fields off the first action.
            let action = unauthorized.actions().first();
            let enc = action.encrypted_note();
            let mut encrypted_note = Vec::with_capacity(ENCRYPTED_NOTE_SIZE);
            encrypted_note.extend_from_slice(&enc.epk_bytes);
            encrypted_note.extend_from_slice(enc.enc_ciphertext.as_ref());
            encrypted_note.extend_from_slice(&enc.out_ciphertext);

            let base = dpp::shielded::SerializedAction {
                nullifier: action.nullifier().to_bytes(),
                rk: <[u8; 32]>::from(action.rk()),
                cmx: action.cmx().to_bytes(),
                encrypted_note,
                cv_net: action.cv_net().to_bytes(),
                spend_auth_sig: [6u8; 64],
            };

            // Precondition guards: confirm the base reaches `from_parts` by
            // checking that every field the verifier decodes BEFORE `from_parts`
            // is valid, and that the base epk is itself a valid non-identity
            // point (so flipping it to the identity is what the InvalidEpk test
            // isolates).
            assert_eq!(base.encrypted_note.len(), ENCRYPTED_NOTE_SIZE);
            assert!(
                Option::<Nullifier>::from(Nullifier::from_bytes(&base.nullifier)).is_some(),
                "base nullifier must decode"
            );
            assert!(
                redpallas::VerificationKey::<redpallas::SpendAuth>::try_from(base.rk).is_ok(),
                "base rk must decode as a (non-identity) verification key"
            );
            assert!(
                Option::<ExtractedNoteCommitment>::from(ExtractedNoteCommitment::from_bytes(
                    &base.cmx
                ))
                .is_some(),
                "base cmx must decode"
            );
            assert!(
                Option::<ValueCommitment>::from(ValueCommitment::from_bytes(&base.cv_net))
                    .is_some(),
                "base cv_net must decode"
            );
            base
        }

        /// `Action::from_parts` -> `ActionFromPartsError::IdentityRk`.
        ///
        /// `rk = [0u8; 32]` is the canonical encoding of the RedPallas identity
        /// verification key: it DECODES successfully (so it passes the verifier's
        /// pre-`from_parts` rk-decode step, unlike the [2u8;32] decode-failure
        /// case in `test_invalid_rk_returns_error`), and `from_parts` then
        /// rejects it because the randomizer key is the identity. Pins the
        /// `IdentityRk => "identity randomizer key"` arm.
        #[test]
        fn test_identity_rk_returns_error() {
            let mut action = valid_base_serialized_action();
            // Sanity: the identity VK encoding must DECODE (else we'd be
            // re-testing the decode-failure path, not the from_parts arm).
            assert!(
                redpallas::VerificationKey::<redpallas::SpendAuth>::try_from([0u8; 32]).is_ok(),
                "identity rk [0;32] must decode so it reaches Action::from_parts"
            );
            action.rk = [0u8; 32]; // RedPallas identity verification key

            let result = reconstruct_and_verify_bundle(
                &[action],
                FLAGS_SPENDS_AND_OUTPUTS,
                0,
                &[42u8; 32],
                &[0u8; 100],
                &[0u8; 64],
                &[],
            );

            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(
                err.message().contains("identity randomizer key"),
                "expected 'identity randomizer key' error from the IdentityRk arm, got: {}",
                err.message()
            );
        }

        /// `Action::from_parts` -> `ActionFromPartsError::InvalidEpk`.
        ///
        /// This is the orchard 0.14 circuit-soundness reject. The action is valid
        /// everywhere the verifier checks before `from_parts` — including a
        /// genuine, non-identity `rk` derived exactly like orchard's own
        /// `non_identity_rk()` test helper (the scalar `1` as a RedPallas
        /// `SigningKey`, then its `VerificationKey`) — so it passes the rk-decode
        /// step AND the `IdentityRk` check, reaching the epk invariant. Its `epk`
        /// is then set to `[0u8; 32]`, the canonical Pallas identity encoding,
        /// which is NOT a valid `KA^{Orchard}` public key, so `from_parts`
        /// rejects with `InvalidEpk`. Pins the
        /// `InvalidEpk => "invalid ephemeral public key"` arm.
        #[test]
        fn test_identity_epk_returns_invalid_epk_error() {
            let mut action = valid_base_serialized_action();

            // Non-identity rk: scalar 1 (little-endian) -> SigningKey -> VK -> bytes.
            let mut scalar_one = [0u8; 32];
            scalar_one[0] = 1;
            let signing_key = redpallas::SigningKey::<redpallas::SpendAuth>::try_from(scalar_one)
                .expect("scalar 1 is a valid RedPallas signing key");
            let vk = redpallas::VerificationKey::<redpallas::SpendAuth>::from(&signing_key);
            let non_identity_rk = <[u8; 32]>::from(vk);
            // Guard: this rk must NOT be the identity (else we'd trip IdentityRk
            // instead of reaching the epk check).
            assert_ne!(
                non_identity_rk, [0u8; 32],
                "scalar-1 verification key must be non-identity"
            );
            action.rk = non_identity_rk;

            // Set the ephemeral public key (first 32 bytes of encrypted_note) to
            // the canonical Pallas identity encoding — an invalid epk.
            action.encrypted_note[..EPK_SIZE].copy_from_slice(&[0u8; EPK_SIZE]);

            let result = reconstruct_and_verify_bundle(
                &[action],
                FLAGS_SPENDS_AND_OUTPUTS,
                0,
                &[42u8; 32],
                &[0u8; 100],
                &[0u8; 64],
                &[],
            );

            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(
                err.message().contains("invalid ephemeral public key"),
                "expected 'invalid ephemeral public key' error from the InvalidEpk arm, got: {}",
                err.message()
            );
        }

        /// `Bundle::try_from_parts(.., ProofSizeEnforcement::Strict)` ->
        /// `BundleError::NonCanonicalProofSize`.
        ///
        /// Pins the proof-size policy. The base action is valid, so reconstruction
        /// clears `Action::from_parts` and reaches `try_from_parts`; the proof
        /// byte-length (100) is not canonical for a single-action bundle, so
        /// `Strict` rejects it. This is what distinguishes `Strict` from
        /// `Unenforced`: under `Unenforced` the bundle would build and this test
        /// would fail. The positive round-trip tests use canonical proofs and pass
        /// under either setting, so without this test a refactor could silently flip
        /// the policy. A 32-byte zero anchor (field element 0) is used so anchor
        /// decoding succeeds and we reach the proof-size check.
        #[test]
        fn test_noncanonical_proof_size_rejected_under_strict() {
            let action = valid_base_serialized_action();
            let result = reconstruct_and_verify_bundle(
                &[action],
                FLAGS_SPENDS_AND_OUTPUTS,
                0,
                &[0u8; 32],
                &[0u8; 100], // non-canonical proof length for a 1-action bundle
                &[0u8; 64],
                &[],
            );

            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(
                err.message()
                    .contains("failed to reconstruct authorized bundle"),
                "expected NonCanonicalProofSize rejection from try_from_parts(Strict), got: {}",
                err.message()
            );
        }
    }

    // ==========================================
    // validate_nullifiers tests
    // ==========================================

    mod validate_nullifiers_tests {
        use super::*;

        #[test]
        fn test_intra_bundle_duplicate_nullifiers_returns_error() {
            let platform = setup_platform();
            let platform_version = PlatformVersion::latest();

            let nullifier = [1u8; 32];
            // Same nullifier twice in the bundle
            let nullifiers = vec![nullifier, nullifier];

            let mut drive_operations = vec![];
            let result = validate_nullifiers(
                &platform.drive,
                &nullifiers,
                None,
                &mut drive_operations,
                platform_version,
            )
            .expect("should not return Error");

            assert!(result.is_some(), "should return a consensus error");
            let consensus_result = result.unwrap();
            assert!(!consensus_result.is_valid());
            let errors = consensus_result.errors;
            assert_eq!(errors.len(), 1);
            assert!(
                matches!(
                    &errors[0],
                    dpp::consensus::ConsensusError::StateError(
                        StateError::NullifierAlreadySpentError(_)
                    )
                ),
                "expected NullifierAlreadySpentError, got: {:?}",
                errors[0]
            );
        }

        #[test]
        fn test_nullifier_already_in_state_returns_error() {
            let platform = setup_platform();
            let platform_version = PlatformVersion::latest();

            let nullifier = [99u8; 32];
            insert_nullifier_into_state(&platform, &nullifier);

            let nullifiers = vec![nullifier];

            let mut drive_operations = vec![];
            let result = validate_nullifiers(
                &platform.drive,
                &nullifiers,
                None,
                &mut drive_operations,
                platform_version,
            )
            .expect("should not return Error");

            assert!(result.is_some(), "should return a consensus error");
            let consensus_result = result.unwrap();
            assert!(!consensus_result.is_valid());
            let errors = consensus_result.errors;
            assert_eq!(errors.len(), 1);
            assert!(
                matches!(
                    &errors[0],
                    dpp::consensus::ConsensusError::StateError(
                        StateError::NullifierAlreadySpentError(_)
                    )
                ),
                "expected NullifierAlreadySpentError for state check, got: {:?}",
                errors[0]
            );
        }

        #[test]
        fn test_unique_nullifiers_not_in_state_returns_none() {
            let platform = setup_platform();
            let platform_version = PlatformVersion::latest();

            let nullifiers = vec![[10u8; 32], [20u8; 32]];

            let mut drive_operations = vec![];
            let result = validate_nullifiers(
                &platform.drive,
                &nullifiers,
                None,
                &mut drive_operations,
                platform_version,
            )
            .expect("should not return Error");

            assert!(result.is_none(), "should return None for valid nullifiers");
        }
    }

    // ==========================================
    // validate_anchor_exists tests
    // ==========================================

    mod validate_anchor_exists_tests {
        use super::*;

        #[test]
        fn test_anchor_not_in_state_returns_error() {
            let platform = setup_platform();
            let platform_version = PlatformVersion::latest();

            let anchor = [77u8; 32];
            let mut drive_operations = vec![];

            let result = validate_anchor_exists(
                &platform.drive,
                &anchor,
                None,
                &mut drive_operations,
                platform_version,
            )
            .expect("should not return Error");

            assert!(result.is_some(), "should return a consensus error");
            let consensus_result = result.unwrap();
            assert!(!consensus_result.is_valid());
            let errors = consensus_result.errors;
            assert!(
                matches!(
                    &errors[0],
                    dpp::consensus::ConsensusError::StateError(StateError::InvalidAnchorError(_))
                ),
                "expected InvalidAnchorError, got: {:?}",
                errors[0]
            );
        }

        #[test]
        fn test_anchor_in_state_returns_none() {
            let platform = setup_platform();
            let platform_version = PlatformVersion::latest();

            let anchor = [77u8; 32];
            insert_anchor_into_state(&platform, &anchor);

            let mut drive_operations = vec![];
            let result = validate_anchor_exists(
                &platform.drive,
                &anchor,
                None,
                &mut drive_operations,
                platform_version,
            )
            .expect("should not return Error");

            assert!(result.is_none(), "should return None for existing anchor");
        }
    }

    // ==========================================
    // validate_minimum_pool_notes tests
    // ==========================================

    mod validate_minimum_pool_notes_tests {
        use super::*;

        #[test]
        fn test_insufficient_notes_returns_error() {
            let platform = setup_platform();
            let platform_version = PlatformVersion::latest();

            // The platform has min_notes threshold > 0. An empty pool should fail.
            let min_notes = platform_version
                .drive_abci
                .validation_and_processing
                .event_constants
                .minimum_pool_notes_for_outgoing;

            if min_notes == 0 {
                // If the platform version has no minimum, this test is not applicable.
                return;
            }

            // Insert fewer notes than the threshold
            let insert_count = min_notes.saturating_sub(1);
            if insert_count > 0 {
                insert_dummy_encrypted_notes(&platform, insert_count);
            }

            let mut drive_operations = vec![];
            let result = validate_minimum_pool_notes(
                &platform.drive,
                None,
                &mut drive_operations,
                platform_version,
            )
            .expect("should not return Error");

            assert!(result.is_some(), "should return a consensus error");
            let consensus_result = result.unwrap();
            assert!(!consensus_result.is_valid());
            let errors = consensus_result.errors;
            assert!(
                matches!(
                    &errors[0],
                    dpp::consensus::ConsensusError::StateError(
                        StateError::InsufficientPoolNotesError(_)
                    )
                ),
                "expected InsufficientPoolNotesError, got: {:?}",
                errors[0]
            );
        }

        #[test]
        fn test_sufficient_notes_returns_none() {
            let platform = setup_platform();
            let platform_version = PlatformVersion::latest();

            let min_notes = platform_version
                .drive_abci
                .validation_and_processing
                .event_constants
                .minimum_pool_notes_for_outgoing;

            // Insert enough notes to meet the threshold
            insert_dummy_encrypted_notes(&platform, min_notes.max(1));

            let mut drive_operations = vec![];
            let result = validate_minimum_pool_notes(
                &platform.drive,
                None,
                &mut drive_operations,
                platform_version,
            )
            .expect("should not return Error");

            assert!(result.is_none(), "should return None when enough notes");
        }
    }

    // ==========================================
    // read_pool_total_balance tests
    // ==========================================

    mod read_pool_total_balance_tests {
        use super::*;

        #[test]
        fn test_default_pool_balance_is_zero() {
            let platform = setup_platform();
            let platform_version = PlatformVersion::latest();

            let mut drive_operations = vec![];
            let balance = read_pool_total_balance(
                &platform.drive,
                None,
                &mut drive_operations,
                platform_version,
            )
            .expect("should read pool balance");

            assert_eq!(balance, 0, "default pool balance should be 0");
        }

        #[test]
        fn test_pool_balance_after_set() {
            let platform = setup_platform();
            let platform_version = PlatformVersion::latest();

            set_pool_total_balance(&platform, 500_000_000);

            let mut drive_operations = vec![];
            let balance = read_pool_total_balance(
                &platform.drive,
                None,
                &mut drive_operations,
                platform_version,
            )
            .expect("should read pool balance");

            assert_eq!(balance, 500_000_000);
        }
    }

    // ==========================================
    // warmup_shielded_verifying_key tests
    // ==========================================

    mod warmup_tests {
        use super::*;

        #[test]
        fn test_warmup_does_not_panic() {
            warmup_shielded_verifying_key();
            // Second call should be a no-op (OnceLock already initialized)
            warmup_shielded_verifying_key();
        }
    }

    // ==========================================
    // FLAGS constants tests
    // ==========================================

    mod flags_constants {
        use super::*;

        #[test]
        fn test_flags_constants_are_valid() {
            assert!(Flags::from_byte(FLAGS_OUTPUTS_ONLY).is_some());
            assert!(Flags::from_byte(FLAGS_SPENDS_AND_OUTPUTS).is_some());
            assert!(FLAGS_OUTPUTS_ONLY != FLAGS_SPENDS_AND_OUTPUTS);
        }
    }

    /// Benchmark: how shielded verification scales with the number of actions.
    ///
    /// Run with:
    ///   `cargo test -p drive-abci --lib bench_shielded_proof_verification_scaling -- \
    ///       --ignored --nocapture`
    ///
    /// Halo 2 proof verification is one per-bundle check whose cost grows with the action
    /// count (one circuit instance per action); RedPallas spend-auth signatures are
    /// per-action; the binding signature is per-bundle. So the full consensus verification
    /// cost is roughly `proof_verify(n) + n × spend_auth + binding`. This informs whether the
    /// flat `shielded_proof_verification_fee` should gain a per-action component.
    #[test]
    #[ignore = "benchmark; run manually with --ignored --nocapture"]
    fn bench_shielded_proof_verification_scaling() {
        use grovedb_commitment_tree::{
            Builder, BundleType, FullViewingKey, NoteValue, ProvingKey, Scope, SpendingKey,
        };
        use rand::rngs::OsRng;
        use std::time::Instant;

        let pk = ProvingKey::build();
        let vk = get_verifying_key();

        // Build & prove an n-action (outputs-only) bundle.
        let build = |n: usize| {
            let mut rng = OsRng;
            let sk = SpendingKey::from_bytes([7u8; 32]).unwrap();
            let fvk = FullViewingKey::from(&sk);
            let recipient = fvk.address_at(0u32, Scope::External);
            let anchor: Anchor = Anchor::empty_tree();
            let mut builder = Builder::<DashMemo>::new(
                BundleType::Transactional {
                    flags: Flags::SPENDS_DISABLED,
                    bundle_required: false,
                },
                anchor,
            );
            for _ in 0..n {
                builder
                    .add_output(None, recipient, NoteValue::from_raw(5000), [0u8; 36])
                    .unwrap();
            }
            let (unauth, _) = builder.build::<i64>(&mut rng).unwrap().unwrap();
            let sighash: [u8; 32] = unauth.commitment().into();
            let proven = unauth.create_proof(&pk, &mut rng).unwrap();
            (proven.apply_signatures(rng, sighash, &[]).unwrap(), sighash)
        };

        let k = 30u32;
        eprintln!("\n=== Halo 2 proof verification vs action count ===");
        for n in [1usize, 2, 4, 8, 16] {
            let (bundle, _) = build(n);
            let instances: Vec<_> = bundle
                .actions()
                .iter()
                .map(|a| a.to_instance(*bundle.flags(), *bundle.anchor()))
                .collect();
            let _ = bundle.authorization().proof().verify(vk, &instances); // warm
            let start = Instant::now();
            for _ in 0..k {
                let _ = bundle.authorization().proof().verify(vk, &instances);
            }
            eprintln!(
                "  actions={:2}  proof_verify = {:>7} us",
                n,
                start.elapsed().as_micros() / k as u128
            );
        }

        // Per-action spend-auth sig and per-bundle binding sig (≈ constant each).
        let (bundle, sighash) = build(1);
        let action = &bundle.actions()[0];
        let (rk, sig) = (action.rk(), action.authorization());
        let start = Instant::now();
        for _ in 0..k {
            let _ = rk.verify(&sighash, sig);
        }
        eprintln!(
            "  spend_auth_sig (per action) = {} us",
            start.elapsed().as_micros() / k as u128
        );
        let bvk = bundle.binding_validating_key();
        let bsig = bundle.authorization().binding_signature();
        let start = Instant::now();
        for _ in 0..k {
            let _ = bvk.verify(&sighash, bsig);
        }
        eprintln!(
            "  binding_sig (per bundle) = {} us",
            start.elapsed().as_micros() / k as u128
        );
    }
}
