use dpp::bls_signatures::G2Element;
use dpp::dashcore::{ChainLock, QuorumSigningRequestId, VarInt};
use dpp::dashcore::consensus::Encodable;
use dpp::dashcore::hashes::{Hash, HashEngine, sha256d};

use crate::error::Error;

use crate::platform_types::platform::Platform;

use crate::rpc::core::CoreRPCLike;

use dpp::version::PlatformVersion;
use crate::platform_types::platform_state::PlatformState;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;

const CHAIN_LOCK_REQUEST_ID_PREFIX: &str = "clsig";

impl<C> Platform<C>
    where
        C: CoreRPCLike,
{
    /// Returning None here means we were unable to verify the chain lock because of an absence of
    /// the quorum
    pub fn verify_chain_lock_locally_v0(&self, platform_state: &PlatformState, chain_lock: &ChainLock, platform_version: &PlatformVersion) -> Result<Option<bool>, Error> {

        // First verify that the signature conforms to a signature
        let signature = G2Element::from_bytes(chain_lock.signature.as_bytes())?;

        // we attempt to verify the chain lock locally
        let chain_lock_height = chain_lock.block_height;

        let window_width = 578;

        // The last block in the window where the quorums would be the same
        let last_block_in_window = platform_state.last_committed_core_height() - platform_state.last_committed_core_height() % window_width + window_width - 1;

        let verification_height = chain_lock.block_height - 8;

        if verification_height > last_block_in_window  {
            return Ok(None); // the chain lock is too far in the future or the past to verify locally
        }

        let quorums = if let Some((previous_quorum_height, previous_quorums)) = platform_state.previous_height_chain_lock_validating_quorums() {
            if chain_lock_height > 8 && chain_lock_height - 8 <= *previous_quorum_height {
                // In this case the quorums were changed recently meaning that we should use the previous quorums to verify the chain lock
                previous_quorums
            } else {
                platform_state.chain_lock_validating_quorums()
            }
        } else {
            platform_state.chain_lock_validating_quorums()
        };

        // From DIP 8: https://github.com/dashpay/dips/blob/master/dip-0008.md#finalization-of-signed-blocks
        // The request id is SHA256("clsig", blockHeight) and the message hash is the block hash of the previously successful attempt.

        let mut engine = QuorumSigningRequestId::engine();

        // Prefix
        let prefix_len = VarInt(CHAIN_LOCK_REQUEST_ID_PREFIX.len() as u64);
        prefix_len.consensus_encode(&mut engine).expect("expected to encode the prefix");

        engine.input(CHAIN_LOCK_REQUEST_ID_PREFIX.as_bytes());
        engine.input(chain_lock.block_height.to_be_bytes().as_slice());

        let request_id = QuorumSigningRequestId::from_engine(engine);

        // Based on the deterministic masternode list at the given height, a quorum must be selected that was active at the time this block was mined

        let quorum = self.choose_quorum(self.config.chain_lock_quorum_type(), quorums, request_id.as_ref(), platform_version)?;

        let Some((quorum_hash, public_key)) = quorum else {
            return Ok(None);
        };

        // The signature must verify against the quorum public key and SHA256(llmqType, quorumHash, SHA256(height), blockHash). llmqType and quorumHash must be taken from the quorum selected in 1.

        let mut engine = sha256d::Hash::engine();

        engine.input(&[self.config.chain_lock_quorum_type() as u8]);
        engine.input(quorum_hash.as_byte_array());
        engine.input(chain_lock.block_hash.as_byte_array());
        engine.input(chain_lock.block_hash.as_byte_array());

        let message_digest = sha256d::Hash::from_engine(engine);

        let chain_lock_verified = public_key.verify(&signature, message_digest.as_ref());

        return Ok(Some(chain_lock_verified))
    }
}
