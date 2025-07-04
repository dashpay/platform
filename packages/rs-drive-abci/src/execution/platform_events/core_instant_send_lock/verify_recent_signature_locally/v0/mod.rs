use dpp::bls_signatures::{Bls12381G2Impl, Pairing, Signature};
use std::fmt::{Debug, Formatter};

use dpp::dashcore::hashes::{sha256d, Hash, HashEngine};
use dpp::dashcore::InstantLock;

use crate::error::execution::ExecutionError;
use crate::error::Error;

use crate::error::serialization::SerializationError;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
use crate::platform_types::signature_verification_quorum_set::{
    SignatureVerificationQuorumSetV0Methods, SIGN_OFFSET,
};

#[inline(always)]
pub(super) fn verify_recent_instant_lock_signature_locally_v0(
    instant_lock: &InstantLock,
    platform_state: &PlatformState,
) -> Result<bool, Error> {
    // First verify that the signature conforms to a signature

    let signature = match <Bls12381G2Impl as Pairing>::Signature::from_compressed(
        instant_lock.signature.as_bytes(),
    )
    .into_option()
    {
        Some(signature) => Signature::Basic(signature),
        None => {
            tracing::trace!(
                instant_lock = ?InstantLockDebug(instant_lock),                "Invalid instant Lock {} signature format",                instant_lock.txid,            );

            return Ok(false);
        }
    };

    let signing_height = platform_state.last_committed_core_height();
    let verification_height = signing_height.saturating_sub(SIGN_OFFSET);

    let quorum_set = platform_state.instant_lock_validating_quorums();

    // Based on the deterministic masternode list at the given height, a quorum must be selected
    // that was active at the time this block was mined
    let selected_quorums = quorum_set.select_quorums(signing_height, verification_height);

    if selected_quorums.is_empty() {
        return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
            "No quorums selected for instant lock signature verification for specified height",
        )));
    };

    let request_id = instant_lock.request_id().map_err(|e| {
        Error::Serialization(SerializationError::CorruptedSerialization(format!(
            "can't hash instant lock request ID for signature verification: {e}"
        )))
    })?;

    let mut selected_quorums_for_logging = Vec::new();
    if tracing::enabled!(tracing::Level::TRACE) {
        selected_quorums_for_logging = Vec::from_iter(selected_quorums.clone());
    }

    for (i, quorums) in selected_quorums.enumerate() {
        let Some((quorum_hash, quorum)) = quorums.choose_quorum(request_id.as_ref()) else {
            if tracing::enabled!(tracing::Level::TRACE) {
                tracing::trace!(
                    quorums_iteration = i + 1,
                    selected_quorums = ?selected_quorums_for_logging,
                    instant_lock = ?InstantLockDebug(instant_lock),
                    ?quorum_set,
                    request_id = request_id.to_string(),
                    quorums = ?quorums.quorums,
                    request_id = request_id.to_string(),
                    signing_height,
                    verification_height,
                    "No chosen for instant Lock {} request ID {}",
                    instant_lock.txid,
                    request_id,
                );
            };

            continue;
        };

        // The signature must verify against the quorum public key and SHA256(llmqType, quorumHash, SHA256(height), txId).
        // llmqType and quorumHash must be taken from the quorum selected in 1.
        let mut engine = sha256d::Hash::engine();

        let mut reversed_quorum_hash = quorum_hash.to_byte_array().to_vec();
        reversed_quorum_hash.reverse();

        engine.input(&[quorum_set.config().quorum_type as u8]);
        engine.input(reversed_quorum_hash.as_slice());
        engine.input(request_id.as_byte_array());
        engine.input(instant_lock.txid.as_byte_array());

        let message_digest = sha256d::Hash::from_engine(engine);

        if signature
            .verify(
                &quorum.public_key,
                message_digest.as_byte_array().as_slice(),
            )
            .is_ok()
        {
            return Ok(true);
        }

        if tracing::enabled!(tracing::Level::TRACE) {
            tracing::trace!(
                quorums_iteration = i + 1,
                selected_quorums = ?selected_quorums_for_logging,
                instant_lock = ?InstantLockDebug(instant_lock),
                ?quorum_set,
                quorum_hash = quorum_hash.to_string(),
                quorum = ?quorum,
                request_id = request_id.to_string(),
                message_digest = message_digest.to_string(),
                signing_height,
                verification_height,
                "Instant Lock {} signature verification failed",
                instant_lock.txid,
            );
        };
    }

    Ok(false)
}

// TODO: The best way is to implement Value trait for InstantLock and hashes
//  in dashcore

/// An additional struct to implement Debug for InstantLock with hex strings
/// instead of byte arrays
struct InstantLockDebug<'a>(&'a InstantLock);

impl Debug for InstantLockDebug<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let instant_lock = &self.0;
        f.debug_struct("InstantLock")
            .field("version", &instant_lock.version)
            .field(
                "inputs",
                &instant_lock
                    .inputs
                    .iter()
                    .map(|input| input.to_string())
                    .collect::<Vec<String>>(),
            )
            .field("txid", &instant_lock.txid.to_string())
            .field("cyclehash", &instant_lock.cyclehash.to_string())
            .field("signature", &instant_lock.signature.to_string())
            .finish()
    }
}
