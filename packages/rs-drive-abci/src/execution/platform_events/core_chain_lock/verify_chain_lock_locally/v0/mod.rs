
use dpp::dashcore::{ChainLock, QuorumSigningRequestId, VarInt};
use dpp::dashcore::consensus::Encodable;
use dpp::dashcore::hashes::{Hash, HashEngine, sha256};
use dpp::dashcore::signer::double_sha;
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
        let quorums = platform_state.chain_lock_validating_quorums();

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

        //todo don't choose first, but choose the correct one

        if let Some((quorum_hash, public_key)) = platform_state.chain_lock_validating_quorums().first_key_value() {

        }

        // The signature must verify against the quorum public key and SHA256(llmqType, quorumHash, SHA256(height), blockHash). llmqType and quorumHash must be taken from the quorum selected in 1.

        //todo()
        return Ok(None)
    }
}
