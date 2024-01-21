use dpp::bls_signatures::G2Element;
use dpp::dashcore::consensus::Encodable;
use dpp::dashcore::hashes::{sha256d, Hash, HashEngine};
use dpp::dashcore::{ChainLock, QuorumSigningRequestId, VarInt};

use crate::error::Error;

use crate::platform_types::platform::Platform;

use crate::rpc::core::CoreRPCLike;

use crate::config::PlatformConfig;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
use dpp::version::PlatformVersion;

const CHAIN_LOCK_REQUEST_ID_PREFIX: &str = "clsig";

const SIGN_OFFSET: u32 = 8;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Returning None here means we were unable to verify the chain lock because of an absence of
    /// the quorum
    pub fn verify_chain_lock_locally_v0(
        &self,
        platform_state: &PlatformState,
        chain_lock: &ChainLock,
        platform_version: &PlatformVersion,
    ) -> Result<Option<bool>, Error> {
        // First verify that the signature conforms to a signature
        let signature = G2Element::from_bytes(chain_lock.signature.as_bytes())?;

        // we attempt to verify the chain lock locally
        let chain_lock_height = chain_lock.block_height;

        let window_width = self.config.chain_lock_quorum_window;

        // The last block in the window where the quorums would be the same
        let last_block_in_window = platform_state.last_committed_core_height()
            - platform_state.last_committed_core_height() % window_width
            + window_width
            - 1;

        let verification_height = chain_lock.block_height.saturating_sub(SIGN_OFFSET);

        if verification_height > last_block_in_window {
            return Ok(None); // the chain lock is too far in the future or the past to verify locally
        }

        let (probable_quorums, second_to_check_quorums) =
            if let Some((previous_quorum_height, change_quorum_height, previous_quorums)) =
                platform_state.previous_height_chain_lock_validating_quorums()
            {
                if chain_lock_height > 8 && chain_lock_height - 8 >= *change_quorum_height {
                    // in this case we are sure that we should be targeting the current quorum
                    // We updated core chain lock height from 100 to 105, new chain lock comes in for block 114
                    //  ------- 100 (previous_quorum_height) ------ 105 (change_quorum_height) ------ 106 (new chain lock verification height 114 - 8)
                    // We are sure that we should use current quorums
                    // If we have
                    //  ------- 100 (previous_quorum_height) ------ 105 (change_quorum_height) ------ 105 (new chain lock verification height 113 - 8)
                    // We should also use current quorums, this is because at 105 we are sure new chain lock validating quorums are active
                    (platform_state.chain_lock_validating_quorums(), None)
                } else if chain_lock_height > 8 && chain_lock_height - 8 <= *previous_quorum_height
                {
                    // In this case the quorums were changed recently meaning that we should use the previous quorums to verify the chain lock
                    // We updated core chain lock height from 100 to 105, new chain lock comes in for block 106
                    // -------- 98 (new chain lock verification height 106 - 8) ------- 100 (previous_quorum_height) ------ 105 (change_quorum_height)
                    // We are sure that we should use previous quorums
                    // If we have
                    // -------- 100 (new chain lock verification height 108 - 8) ------- 100 (previous_quorum_height) ------ 105 (change_quorum_height)
                    // We should also use previous quorums, this is because at 100 we are sure the old quorum set was active
                    (previous_quorums, None)
                } else {
                    // we are in between, so we don't actually know if it was the old one or the new one to be used.
                    //  ------- 100 (previous_quorum_height) ------ 104 (new chain lock verification height 112 - 8) -------105 (change_quorum_height)
                    // we should just try both, starting with the current quorums
                    (
                        platform_state.chain_lock_validating_quorums(),
                        Some(previous_quorums),
                    )
                }
            } else {
                (platform_state.chain_lock_validating_quorums(), None)
            };

        // From DIP 8: https://github.com/dashpay/dips/blob/master/dip-0008.md#finalization-of-signed-blocks
        // The request id is SHA256("clsig", blockHeight) and the message hash is the block hash of the previously successful attempt.

        let mut engine = QuorumSigningRequestId::engine();

        // Prefix
        let prefix_len = VarInt(CHAIN_LOCK_REQUEST_ID_PREFIX.len() as u64);
        prefix_len
            .consensus_encode(&mut engine)
            .expect("expected to encode the prefix");

        engine.input(CHAIN_LOCK_REQUEST_ID_PREFIX.as_bytes());
        engine.input(chain_lock.block_height.to_le_bytes().as_slice());

        let request_id = QuorumSigningRequestId::from_engine(engine);

        // Based on the deterministic masternode list at the given height, a quorum must be selected that was active at the time this block was mined

        let quorum = Platform::<C>::choose_quorum(
            self.config.chain_lock_quorum_type(),
            probable_quorums,
            request_id.as_ref(),
            platform_version,
        )?;

        let Some((quorum_hash, public_key)) = quorum else {
            return Ok(None);
        };

        // The signature must verify against the quorum public key and SHA256(llmqType, quorumHash, SHA256(height), blockHash). llmqType and quorumHash must be taken from the quorum selected in 1.

        let mut engine = sha256d::Hash::engine();

        engine.input(&[self.config.chain_lock_quorum_type() as u8]);
        engine.input(quorum_hash.as_byte_array());
        engine.input(request_id.as_byte_array());
        engine.input(chain_lock.block_hash.as_byte_array());

        let message_digest = sha256d::Hash::from_engine(engine);

        let mut chain_lock_verified = public_key.verify(&signature, message_digest.as_ref());

        if !chain_lock_verified {
            // We should also check the other quorum, as there could be the situation where the core height wasn't updated every block.
            if let Some(second_to_check_quorums) = second_to_check_quorums {
                let quorum = Platform::<C>::choose_quorum(
                    self.config.chain_lock_quorum_type(),
                    second_to_check_quorums,
                    request_id.as_ref(),
                    platform_version,
                )?;

                let Some((quorum_hash, public_key)) = quorum else {
                    // we return that we are not able to verify
                    return Ok(None);
                };

                // The signature must verify against the quorum public key and SHA256(llmqType, quorumHash, SHA256(height), blockHash). llmqType and quorumHash must be taken from the quorum selected in 1.

                let mut engine = sha256d::Hash::engine();

                engine.input(&[self.config.chain_lock_quorum_type() as u8]);
                engine.input(quorum_hash.as_byte_array());
                engine.input(request_id.as_byte_array());
                engine.input(chain_lock.block_hash.as_byte_array());

                let message_digest = sha256d::Hash::from_engine(engine);

                chain_lock_verified = public_key.verify(&signature, message_digest.as_ref());
            }
        }

        return Ok(Some(chain_lock_verified));
    }
}
