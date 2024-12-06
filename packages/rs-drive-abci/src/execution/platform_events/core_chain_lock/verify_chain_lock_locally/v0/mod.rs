use dpp::bls_signatures::{Bls12381G2Impl, Pairing, Signature};

use dpp::dashcore::hashes::{sha256d, Hash, HashEngine};
use dpp::dashcore::{ChainLock, QuorumSigningRequestId};

use crate::error::Error;

use crate::platform_types::platform::Platform;

use crate::rpc::core::CoreRPCLike;

use crate::error::execution::ExecutionError;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
use crate::platform_types::signature_verification_quorum_set::SignatureVerificationQuorumSetV0Methods;
use dpp::version::PlatformVersion;

const CHAIN_LOCK_REQUEST_ID_PREFIX: &str = "clsig";

const SIGN_OFFSET: u32 = 8;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Returning None here means we were unable to verify the chain lock because of an absence of
    /// the quorum
    #[inline(always)]
    pub(super) fn verify_chain_lock_locally_v0(
        &self,
        round: u32,
        platform_state: &PlatformState,
        chain_lock: &ChainLock,
        _platform_version: &PlatformVersion,
    ) -> Result<Option<bool>, Error> {
        let quorum_set = platform_state.chain_lock_validating_quorums();
        let quorum_config = quorum_set.config();

        // First verify that the signature conforms to a signature

        let decoded_sig = match <Bls12381G2Impl as Pairing>::Signature::from_compressed(
            chain_lock.signature.as_bytes(),
        )
        .into_option()
        {
            Some(signature) => signature,
            None => {
                tracing::error!(
                    ?chain_lock,
                    "chain lock signature was not deserializable: h:{} r:{}",
                    platform_state.last_committed_block_height() + 1,
                    round
                );
                return Err(Error::BLSError(
                    dpp::bls_signatures::BlsError::DeserializationError(
                        "chain lock signature was not deserializable".to_string(),
                    ),
                ));
            }
        };

        let signature = Signature::Basic(decoded_sig);

        // we attempt to verify the chain lock locally
        let chain_lock_height = chain_lock.block_height;

        let window_width = quorum_config.window;

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

        let mut selected_quorums = platform_state
            .chain_lock_validating_quorums()
            .select_quorums(chain_lock_height, verification_height);

        // TODO: We can use chain_lock.request_id()

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

        let probable_quorums = selected_quorums.next().ok_or_else(|| {
            Error::Execution(ExecutionError::CorruptedCodeExecution(
                "No quorums selected for chain lock signature verification for specified height",
            ))
        })?;

        let Some((quorum_hash, quorum)) = probable_quorums.choose_quorum(request_id.as_ref())
        else {
            return Ok(None);
        };

        // The signature must verify against the quorum public key and SHA256(llmqType, quorumHash, SHA256(height), blockHash). llmqType and quorumHash must be taken from the quorum selected in 1.

        let mut engine = sha256d::Hash::engine();

        let mut reversed_quorum_hash = quorum_hash.to_byte_array().to_vec();
        reversed_quorum_hash.reverse();

        engine.input(&[quorum_config.quorum_type as u8]);
        engine.input(reversed_quorum_hash.as_slice());
        engine.input(request_id.as_byte_array());
        engine.input(chain_lock.block_hash.as_byte_array());

        let message_digest = sha256d::Hash::from_engine(engine);

        let mut chain_lock_verified = signature
            .verify(
                &quorum.public_key,
                message_digest.as_byte_array().as_slice(),
            )
            .is_ok();

        tracing::debug!(
            ?chain_lock,
            "h:{} r:{} message_digest for chain lock at core height {} is {}, quorum hash is {}, block hash is {}, chain lock was {}, last committed core height {}, verification height {}, last block in window {}",
            platform_state.last_committed_block_height() + 1,
            round,
            chain_lock.block_height,
            hex::encode(message_digest.as_byte_array()),
            hex::encode(reversed_quorum_hash.as_slice()),
            hex::encode(chain_lock.block_hash.as_byte_array()),
            if chain_lock_verified { "verified"} else {"not verified"},
            platform_state.last_committed_core_height(),
            verification_height,
            last_block_in_window,
        );

        if !chain_lock_verified {
            // We should also check the other quorum, as there could be the situation where the core height wasn't updated every block.
            if let Some(second_to_check_quorums) = selected_quorums.next() {
                let Some((quorum_hash, quorum)) =
                    second_to_check_quorums.choose_quorum(request_id.as_ref())
                else {
                    // we return that we are not able to verify
                    return Ok(None);
                };

                // The signature must verify against the quorum public key and SHA256(llmqType, quorumHash, SHA256(height), blockHash). llmqType and quorumHash must be taken from the quorum selected in 1.

                let mut engine = sha256d::Hash::engine();

                let mut reversed_quorum_hash = quorum_hash.to_byte_array().to_vec();
                reversed_quorum_hash.reverse();

                engine.input(&[quorum_config.quorum_type as u8]);
                engine.input(reversed_quorum_hash.as_slice());
                engine.input(request_id.as_byte_array());
                engine.input(chain_lock.block_hash.as_byte_array());

                let message_digest = sha256d::Hash::from_engine(engine);

                chain_lock_verified = signature
                    .verify(
                        &quorum.public_key,
                        message_digest.as_byte_array().as_slice(),
                    )
                    .is_ok();

                tracing::debug!(
                    ?chain_lock,
                    "h:{} r:{} tried second quorums message_digest for chain lock at height {} is {}, quorum hash is {}, block hash is {}, chain lock was {}",
                    platform_state.last_committed_block_height() + 1,
                    round,
                    chain_lock.block_height,
                    hex::encode(message_digest.as_byte_array()),
                    hex::encode(reversed_quorum_hash.as_slice()),
                    hex::encode(chain_lock.block_hash.as_byte_array()),
                    if chain_lock_verified { "verified"} else {"not verified"}
                );
                if !chain_lock_verified {
                    tracing::debug!(
                        "chain lock was invalid for both recent and old chain lock quorums"
                    );
                }
            } else if !platform_state
                .chain_lock_validating_quorums()
                .has_previous_past_quorums()
            {
                // we don't have old quorums, this means our node is very new.
                tracing::debug!(
                    "we had no previous quorums locally, we should validate through core",
                );
                return Ok(None);
            } else if !selected_quorums.should_be_verifiable() {
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

        Ok(Some(chain_lock_verified))
    }
}

#[cfg(test)]
mod tests {
    use crate::execution::platform_events::core_chain_lock::verify_chain_lock_locally::v0::CHAIN_LOCK_REQUEST_ID_PREFIX;
    use dpp::bls_signatures::{Bls12381G2Impl, Pairing};
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

    #[test]
    // Verify that the signature can be deserialized
    fn verify_signature() {
        let signatures =vec![
            // local devnet
            [
            139, 91, 189, 131, 240, 161, 167, 171, 205, 251, 134, 2, 160, 27, 100, 46, 55, 162, 23,
            224, 18, 130, 100, 147, 255, 29, 128, 110, 111, 138, 195, 219, 243, 137, 110, 60, 243,
            176, 180, 242, 58, 223, 235, 59, 172, 168, 235, 146, 6, 243, 139, 112, 175, 99, 82, 69,
            144, 38, 15, 72, 250, 94, 82, 198, 52, 35, 126, 131, 37, 140, 25, 178, 33, 187, 71,
            167, 87, 81, 15, 210, 220, 201, 44, 245, 222, 66, 252, 70, 227, 109, 43, 60, 102, 187,
            144, 108,
        ],
        // mainnet
        hex::decode("9609d7dea0812c0a6b72d6f20f988d269d3076324c5e51ff6042ce0506370a738bfce420f8174a4e0e34ded7e5792ac217a169b71c7dc3eefde5abba9feaf7d3c7c29fb36ade2e41bc5dfada13d7546c6061ef7e03e894e813f72dd20af8bcbb").unwrap().try_into().unwrap(),
        ];

        for signature in signatures {
            assert!(
                <Bls12381G2Impl as Pairing>::Signature::from_compressed(&signature)
                    .into_option()
                    .is_some(),
            );
        }
    }
}
