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
use drive::drive::shielded::paths::{
    shielded_credit_pool_anchors_path, shielded_credit_pool_nullifiers_path,
    shielded_credit_pool_path, SHIELDED_NOTES_KEY, SHIELDED_TOTAL_BALANCE_KEY,
};
use drive::drive::Drive;
use drive::fees::op::LowLevelDriveOperation;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::StateTransitionAction;
use drive::util::grove_operations::DirectQueryType;
use grovedb_commitment_tree::{
    redpallas, Action, Anchor, Authorized, BatchValidator, Bundle, DashMemo,
    ExtractedNoteCommitment, Flags, NoteBytesData, Nullifier, Proof, TransmittedNoteCiphertext,
    ValueCommitment, VerifyingKey,
};
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
const ENC_CIPHERTEXT_SIZE: usize = 104;
const OUT_CIPHERTEXT_SIZE: usize = 80;
const ENCRYPTED_NOTE_SIZE: usize = EPK_SIZE + ENC_CIPHERTEXT_SIZE + OUT_CIPHERTEXT_SIZE; // 216

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

/// Read the current shielded pool total balance from GroveDB.
/// Returns 0 if the balance key doesn't exist yet.
pub fn read_pool_total_balance(
    drive: &Drive,
    transaction: TransactionArg,
    drive_operations: &mut Vec<LowLevelDriveOperation>,
    platform_version: &PlatformVersion,
) -> Result<Credits, Error> {
    let pool_path = shielded_credit_pool_path();
    Ok(drive
        .grove_get_raw_value_u64_from_encoded_var_vec(
            (&pool_path).into(),
            &[SHIELDED_TOTAL_BALANCE_KEY],
            DirectQueryType::StatefulDirectQuery,
            transaction,
            drive_operations,
            &platform_version.drive,
        )?
        .unwrap_or(0))
}

/// Verify that the anchor exists in the recorded anchors tree.
/// Anchors are stored as block_height_be → anchor_bytes in [AddressBalances, "s", [6]].
/// Returns a consensus error if the anchor is not found.
pub fn validate_anchor_exists(
    drive: &Drive,
    anchor: &[u8; 32],
    transaction: TransactionArg,
    _drive_operations: &mut Vec<LowLevelDriveOperation>,
    platform_version: &PlatformVersion,
) -> Result<Option<ConsensusValidationResult<StateTransitionAction>>, Error> {
    use drive::grovedb::query_result_type::QueryResultType;
    use drive::grovedb::{Element, PathQuery, Query, SizedQuery};

    let anchors_path = shielded_credit_pool_anchors_path();
    let path_query = PathQuery {
        path: anchors_path.iter().map(|p| p.to_vec()).collect(),
        query: SizedQuery {
            query: Query::new_range_full(),
            limit: None,
            offset: None,
        },
    };

    let grove_version = &platform_version.drive.grove_version;
    let results = drive
        .grove
        .query_raw(
            &path_query,
            true,
            true,
            true,
            QueryResultType::QueryKeyElementPairResultType,
            transaction,
            grove_version,
        )
        .unwrap()
        .map_err(drive::error::Error::from)?;

    let found = results.0.to_key_elements().into_iter().any(|(_, element)| {
        if let Element::Item(value, _) = element {
            value.as_slice() == anchor
        } else {
            false
        }
    });

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
pub fn validate_nullifiers(
    drive: &Drive,
    nullifiers: &[[u8; 32]],
    transaction: TransactionArg,
    drive_operations: &mut Vec<LowLevelDriveOperation>,
    platform_version: &PlatformVersion,
) -> Result<Option<ConsensusValidationResult<StateTransitionAction>>, Error> {
    // Intra-bundle duplicate check
    let mut seen_nullifiers = std::collections::HashSet::new();
    for nullifier in nullifiers {
        if !seen_nullifiers.insert(nullifier) {
            return Ok(Some(ConsensusValidationResult::new_with_error(
                StateError::NullifierAlreadySpentError(NullifierAlreadySpentError::new(*nullifier))
                    .into(),
            )));
        }
    }
    // Check against state
    let nullifiers_path = shielded_credit_pool_nullifiers_path();
    for nullifier in nullifiers {
        let exists = drive.grove_has_raw(
            (&nullifiers_path).into(),
            nullifier,
            DirectQueryType::StatefulDirectQuery,
            transaction,
            drive_operations,
            &platform_version.drive,
        )?;
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
        let pool_path = shielded_credit_pool_path();
        let encrypted_notes_count = drive.grove_commitment_tree_count(
            (&pool_path).into(),
            &[SHIELDED_NOTES_KEY],
            transaction,
            drive_operations,
            &platform_version.drive,
        )?;
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
