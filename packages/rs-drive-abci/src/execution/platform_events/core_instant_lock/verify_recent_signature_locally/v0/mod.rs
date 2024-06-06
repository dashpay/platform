use dpp::bls_signatures::G2Element;

use dpp::dashcore::hashes::{sha256d, Hash, HashEngine};
use dpp::dashcore::InstantLock;

use crate::error::Error;

use crate::error::serialization::SerializationError;
use crate::platform_types::core_quorum_set::CoreQuorumSetV0Methods;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;

const SIGN_OFFSET: u32 = 8;

#[inline(always)]
pub(super) fn verify_recent_instant_lock_signature_locally_v0(
    instant_lock: &InstantLock,
    platform_state: &PlatformState,
) -> Result<bool, Error> {
    // First verify that the signature conforms to a signature
    let Ok(signature) = G2Element::from_bytes(instant_lock.signature.as_bytes()) else {
        return Ok(false);
    };

    let instant_lock_height = platform_state.last_committed_core_height();

    let verification_height = instant_lock_height.saturating_sub(SIGN_OFFSET);

    let request_id = instant_lock.request_id().map_err(|e| {
        Error::Serialization(SerializationError::CorruptedSerialization(format!(
            "can't hash instant lock request ID for signature verification: {e}"
        )))
    })?;

    let quorum_set = platform_state.instant_lock_validating_quorums();

    // Based on the deterministic masternode list at the given height, a quorum must be selected
    // that was active at the time this block was mined
    let selected_quorums =
        quorum_set.select_quorums(instant_lock_height, verification_height, request_id);

    for (quorum_hash, quorum) in selected_quorums {
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

        if quorum
            .public_key
            .verify(&signature, message_digest.as_ref())
        {
            return Ok(true);
        }
    }

    tracing::debug!(
        ?instant_lock,
        instant_lock_height,
        verification_height,
        "Instant Lock {} signature verification failed at height {}",
        instant_lock.txid,
        instant_lock_height,
    );

    Ok(false)
}
