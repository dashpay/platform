use dpp::block::block_info::BlockInfo;
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
        round: u32,
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

        let verification_height = chain_lock_height.saturating_sub(SIGN_OFFSET);

        if verification_height > last_block_in_window {
            tracing::debug!(
                ?chain_lock,
                "h:{} r:{} skipped message_digest for chain lock at core height {} is {}, verification height {}, last block in window {}",
                platform_state.last_committed_block_height() + 1,
                round,
                chain_lock.block_height,
                platform_state.last_committed_core_height(),
                verification_height,
                last_block_in_window,
            );
            return Ok(None); // the chain lock is too far in the future or the past to verify locally
        }

        let (probable_quorums, second_to_check_quorums, should_be_verifiable) = if let Some((
            previous_quorum_height,
            change_quorum_height,
            previous_quorums_change_height,
            previous_quorums,
        )) =
            platform_state.previous_height_chain_lock_validating_quorums()
        {
            if chain_lock_height > 8 && verification_height >= *change_quorum_height {
                // in this case we are sure that we should be targeting the current quorum
                // We updated core chain lock height from 100 to 105, new chain lock comes in for block 114
                //  ------- 100 (previous_quorum_height) ------ 105 (change_quorum_height) ------ 106 (new chain lock verification height 114 - 8)
                // We are sure that we should use current quorums
                // If we have
                //  ------- 100 (previous_quorum_height) ------ 105 (change_quorum_height) ------ 105 (new chain lock verification height 113 - 8)
                // We should also use current quorums, this is because at 105 we are sure new chain lock validating quorums are active
                (platform_state.chain_lock_validating_quorums(), None, true)
            } else if chain_lock_height > 8 && verification_height <= *previous_quorum_height {
                let should_be_verifiable = previous_quorums_change_height
                    .map(|previous_quorums_change_height| {
                        verification_height > previous_quorums_change_height
                    })
                    .unwrap_or(false);
                // In this case the quorums were changed recently meaning that we should use the previous quorums to verify the chain lock
                // We updated core chain lock height from 100 to 105, new chain lock comes in for block 106
                // -------- 98 (new chain lock verification height 106 - 8) ------- 100 (previous_quorum_height) ------ 105 (change_quorum_height)
                // We are sure that we should use previous quorums
                // If we have
                // -------- 100 (new chain lock verification height 108 - 8) ------- 100 (previous_quorum_height) ------ 105 (change_quorum_height)
                // We should also use previous quorums, this is because at 100 we are sure the old quorum set was active
                (previous_quorums, None, should_be_verifiable)
            } else {
                let should_be_verifiable = previous_quorums_change_height
                    .map(|previous_quorums_change_height| {
                        verification_height > previous_quorums_change_height
                    })
                    .unwrap_or(false);
                // we are in between, so we don't actually know if it was the old one or the new one to be used.
                //  ------- 100 (previous_quorum_height) ------ 104 (new chain lock verification height 112 - 8) -------105 (change_quorum_height)
                // we should just try both, starting with the current quorums
                (
                    platform_state.chain_lock_validating_quorums(),
                    Some(previous_quorums),
                    should_be_verifiable,
                )
            }
        } else {
            (platform_state.chain_lock_validating_quorums(), None, false)
        };

        // From DIP 8: https://github.com/dashpay/dips/blob/master/dip-0008.md#finalization-of-signed-blocks
        // The request id is SHA256("clsig", blockHeight) and the message hash is the block hash of the previously successful attempt.

        let mut engine = QuorumSigningRequestId::engine();

        engine.input(&[CHAIN_LOCK_REQUEST_ID_PREFIX.len() as u8]);
        engine.input(CHAIN_LOCK_REQUEST_ID_PREFIX.as_bytes());
        engine.input(chain_lock.block_height.to_le_bytes().as_slice());

        let request_id = QuorumSigningRequestId::from_engine(engine);

        tracing::trace!(
            ?chain_lock,
            "request id for chain lock at height {} is {}",
            chain_lock.block_height,
            hex::encode(request_id.as_byte_array())
        );

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
        engine.input(quorum_hash.as_slice());
        engine.input(request_id.as_byte_array());
        engine.input(chain_lock.block_hash.as_byte_array());

        let message_digest = sha256d::Hash::from_engine(engine);

        let mut chain_lock_verified = public_key.verify(&signature, message_digest.as_ref());

        tracing::debug!(
            ?chain_lock,
            "h:{} r:{} message_digest for chain lock at core height {} is {}, quorum hash is {}, block hash is {}, chain lock was {}, last committed core height {}, verification height {}, last block in window {}",
            platform_state.last_committed_block_height() + 1,
            round,
            chain_lock.block_height,
            hex::encode(message_digest.as_byte_array()),
            hex::encode(quorum_hash.as_slice()),
            hex::encode(chain_lock.block_hash.as_byte_array()),
            if chain_lock_verified { "verified"} else {"not verified"},
            platform_state.last_committed_core_height(),
            verification_height,
            last_block_in_window,
        );

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
                engine.input(quorum_hash.as_slice());
                engine.input(request_id.as_byte_array());
                engine.input(chain_lock.block_hash.as_byte_array());

                let message_digest = sha256d::Hash::from_engine(engine);

                chain_lock_verified = public_key.verify(&signature, message_digest.as_ref());

                tracing::debug!(
                    ?chain_lock,
                    "h:{} r:{} tried second quorums message_digest for chain lock at height {} is {}, quorum hash is {}, block hash is {}, chain lock was {}",
                    platform_state.last_committed_block_height() + 1,
                    round,
                    chain_lock.block_height,
                    hex::encode(message_digest.as_byte_array()),
                    hex::encode(quorum_hash.as_slice()),
                    hex::encode(chain_lock.block_hash.as_byte_array()),
                    if chain_lock_verified { "verified"} else {"not verified"}
                );
                if !chain_lock_verified {
                    tracing::debug!(
                        "chain lock was invalid for both recent and old chain lock quorums"
                    );
                }
            } else if platform_state
                .previous_height_chain_lock_validating_quorums()
                .is_none()
            {
                // we don't have old quorums, this means our node is very new.
                tracing::debug!(
                    "we had no previous quorums locally, we should validate through core",
                );
                return Ok(None);
            } else if !should_be_verifiable {
                tracing::debug!(
                    "we were in a situation where it would be possible we didn't have all quorums and we couldn't verify locally, we should validate through core",
                );
                return Ok(None);
            } else if round >= 10 {
                tracing::debug!("high round when chain lock was invalid, asking core to verify");
            } else {
                tracing::debug!("chain lock was invalid, and we deemed there was no reason to check old quorums");
            }
        }

        return Ok(Some(chain_lock_verified));
    }
}

#[cfg(test)]
mod tests {
    use crate::execution::platform_events::core_chain_lock::verify_chain_lock_locally::v0::CHAIN_LOCK_REQUEST_ID_PREFIX;
    use dpp::dashcore::hashes::{sha256d, Hash, HashEngine};
    use dpp::dashcore::QuorumSigningRequestId;

    #[test]
    fn verify_request_id() {
        assert_eq!(
            hex::encode(CHAIN_LOCK_REQUEST_ID_PREFIX.as_bytes()),
            "636c736967"
        );
        assert_eq!(hex::encode(122u32.to_le_bytes()), "7a000000");

        let mut engine = QuorumSigningRequestId::engine();

        engine.input(&[CHAIN_LOCK_REQUEST_ID_PREFIX.len() as u8]);
        engine.input(CHAIN_LOCK_REQUEST_ID_PREFIX.as_bytes());
        engine.input(122u32.to_le_bytes().as_slice());

        let request_id = QuorumSigningRequestId::from_engine(engine);

        assert_eq!(
            hex::encode(request_id.as_byte_array()),
            "e1a9d40e5145fdc168819125b5ae1b8f12d5115471624eb363a6c7a3693be2e6"
        );

        //
        // let mut chain_lock_verified = public_key.verify(&signature, message_digest.as_ref());
    }

    #[test]
    fn verify_message_digest() {
        let mut engine = QuorumSigningRequestId::engine();

        engine.input(&[CHAIN_LOCK_REQUEST_ID_PREFIX.len() as u8]);
        engine.input(CHAIN_LOCK_REQUEST_ID_PREFIX.as_bytes());
        engine.input(956087u32.to_le_bytes().as_slice());

        let request_id = QuorumSigningRequestId::from_engine(engine);

        assert_eq!(
            hex::encode(request_id.as_byte_array()),
            "ea04b27adfaa698487aee46b922fb8f1c77a562787b6afe65eecf7e685888928"
        );

        let mut block_hash =
            hex::decode("000000d94ea7ac4f86c5f583e7da0feb90b9f8d038f25e55cc305524c5327266")
                .unwrap();
        let mut quorum_hash =
            hex::decode("0000009d376e73a22aa997bb6542bd1fc3018f61c2301a817126a737ffdcdc80")
                .unwrap();

        block_hash.reverse();
        quorum_hash.reverse();

        let mut engine = sha256d::Hash::engine();

        engine.input(&[1u8]);
        engine.input(quorum_hash.as_slice());
        engine.input(request_id.to_byte_array().as_slice());
        engine.input(block_hash.as_slice());

        let mut message_digest = sha256d::Hash::from_engine(engine).as_byte_array().to_vec();

        message_digest.reverse();

        assert_eq!(
            hex::encode(message_digest.as_slice()),
            "5ec53e83b8ff390b970e28db21da5b8e45fbe3b69d9f11a2c39062769b1f5e47"
        );
    }
}
