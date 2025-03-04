//! [AssetLockProof] utilities

use crate::networks::NetworkSettings;
use crate::{Error, Sdk};
use dapi_grpc::platform::v0::get_epochs_info_request::{self, GetEpochsInfoRequestV0};
use dapi_grpc::platform::v0::GetEpochsInfoRequest;
use dapi_grpc::platform::VersionedGrpcResponse;
use dashcore_rpc::json::QuorumType;
use dpp::bls_signatures::{Bls12381G2Impl, BlsError, Pairing, PublicKey, Signature};
use dpp::dashcore::hashes::{sha256d, Hash, HashEngine};
use dpp::dashcore::InstantLock;
use dpp::prelude::AssetLockProof;
use drive_proof_verifier::error::ContextProviderError;
use drive_proof_verifier::ContextProvider;
use rs_dapi_client::{DapiRequestExecutor, RequestSettings};

#[async_trait::async_trait]
pub trait AssetLockProofVerifier {
    /// Verifies the asset lock proof against the platform.
    ///
    /// This function will return an error if Dash Platform cannot use the provided asset lock proof.
    ///
    /// # Errors
    ///
    /// - [Error::CoreLockedHeightNotYetAvailable] if the core locked height in the proof is higher than the
    /// current core locked height on the platform. Try again later.
    /// - [Error::QuorumNotFound] if the quorum public key is not yet available on the platform, what implies that
    /// the quorum is not (yet) available. Try again later.
    /// - [Error::InvalidSignature] if the signature in the proof is invalid.
    /// - other errors when something goes wrong.
    async fn verify(&self, sdk: &Sdk) -> Result<(), Error>;
}

#[async_trait::async_trait]
impl AssetLockProofVerifier for AssetLockProof {
    async fn verify(&self, sdk: &Sdk) -> Result<(), Error> {
        let context_provider = sdk
            .context_provider()
            .ok_or(Error::Config("Context Provider not configured".to_string()))?;

        // Retrieve current core chain lock info from the platform
        // TODO: implement some caching mechanism to avoid fetching the same data multiple times
        let request = GetEpochsInfoRequest {
            version: Some(get_epochs_info_request::Version::V0(
                GetEpochsInfoRequestV0 {
                    ascending: false,
                    count: 1,
                    prove: true,
                    start_epoch: None,
                },
            )),
        };
        let response = sdk.execute(request, RequestSettings::default()).await?;
        let platform_core_chain_locked_height = response.metadata()?.core_chain_locked_height;

        match self {
            AssetLockProof::Chain(asset_lock) => {
                if asset_lock.core_chain_locked_height > platform_core_chain_locked_height {
                    Err(Error::CoreLockedHeightNotYetAvailable(
                        asset_lock.core_chain_locked_height,
                        platform_core_chain_locked_height,
                    ))
                } else {
                    Ok(())
                }
            }
            AssetLockProof::Instant(instant_asset_lock_proof) => {
                instant_asset_lock_proof.validate_structure(sdk.version())?;

                let quorum_hash = instant_asset_lock_proof
                    .instant_lock()
                    .cyclehash
                    .to_raw_hash()
                    .to_byte_array();
                let quorum_type: QuorumType = sdk.network_settings().chain_locks_quorum_type();
                // Try to fetch the quorum public key; if it fails, we assume platform does not have this quorum yet
                let quorum_pubkey = match context_provider.get_quorum_public_key(
                    quorum_type as u32,
                    quorum_hash,
                    platform_core_chain_locked_height,
                ) {
                    Err(ContextProviderError::InvalidQuorum(s)) => Err(Error::QuorumNotFound {
                        e: ContextProviderError::InvalidQuorum(s),
                        quorum_hash_hex: hex::encode(quorum_hash),
                        quorum_type: quorum_type as u32,
                        core_chain_locked_height: platform_core_chain_locked_height,
                    }),
                    Err(e) => Err(e.into()),
                    Ok(key) => Ok(key),
                }?;

                verify_instant_lock_signature(
                    instant_asset_lock_proof.instant_lock(),
                    &quorum_type,
                    &quorum_hash,
                    &quorum_pubkey,
                )
                .map_err(|e| {
                    Error::InvalidAssetLock(format!(
                        "error during instant asset lock verification: {}",
                        e
                    ))
                })
                .and_then(|correct| {
                    if correct {
                        Ok(())
                    } else {
                        Err(Error::InvalidAssetLock(
                            "invalid asset lock signature".to_string(),
                        ))
                    }
                })
            }
        }
    }
}
/// Verify instant lock signature.
///
/// Retrurns Ok(true) when signature is valid, Ok(false) when signature is invalid, or an error when something goes wrong.
///
/// Note: This is a generalisation [verify_recent_instant_lock_signature_locally_v0] from drive-abci.
///
/// TODO: Discuss moving to some other place to reuse in drive-abci and here
fn verify_instant_lock_signature(
    instant_lock: &InstantLock,
    quorum_type: &QuorumType,
    quorum_hash: &[u8],
    quorum_public_key: &[u8; 48],
) -> Result<bool, Error> {
    // First verify signature format
    let signature: Signature<Bls12381G2Impl> =
        match <Bls12381G2Impl as Pairing>::Signature::from_compressed(
            instant_lock.signature.as_bytes(),
        )
        .into_option()
        {
            Some(signature) => Signature::Basic(signature),
            None => {
                tracing::warn!(
                    instant_lock = ?instant_lock,
                    "Invalid instant lock {} signature format",
                    instant_lock.txid,
                );

                return Ok(false);
            }
        };

    let threshold_public_key =
        PublicKey::try_from(quorum_public_key.as_slice()).map_err(|e| Error::Protocol(e.into()))?;

    let request_id = instant_lock.request_id().map_err(|e| {
        Error::Protocol(dpp::ProtocolError::EncodingError(format!(
            "cannot create instant asset lock request id: {}",
            e
        )))
    })?;

    // The signature must verify against the quorum public key and SHA256(llmqType, quorumHash, SHA256(height), txId).
    // llmqType and quorumHash must be taken from the quorum selected in 1.
    let mut engine = sha256d::Hash::engine();

    let mut reversed_quorum_hash = quorum_hash.to_vec();
    reversed_quorum_hash.reverse();

    engine.input(&[*quorum_type as u8]);
    engine.input(reversed_quorum_hash.as_slice());
    engine.input(request_id.as_byte_array());
    engine.input(instant_lock.txid.as_byte_array());

    let message_digest = sha256d::Hash::from_engine(engine);

    match signature.verify(
        &threshold_public_key,
        message_digest.as_byte_array().as_slice(),
    ) {
        Ok(()) => Ok(true),
        Err(BlsError::InvalidSignature) => Ok(false),
        Err(e) => Err(Error::Protocol(e.into())),
    }
}

#[cfg(test)]
mod tests {

    use dashcore_rpc::json::QuorumType;
    use dpp::dashcore::{consensus::deserialize, hashes::Hash, InstantLock};

    use crate::platform::transition::asset_lock::verify_instant_lock_signature;

    /// Test signature verification on an instant asset lock.
    #[test]
    fn test_verify_instant_lock_signature() {
        const INSTANT_SEND_LOCK_QUORUM_TYPE: QuorumType = QuorumType::Llmq60_75;

        struct TestCase {
            instant_lock_hex: &'static str,
            quorum_hash: &'static str,
            quorum_public_key: &'static str,
            correct_signature: bool,
        }

        // test vector generated with:
        // dash-cli getislocks '["e7ca636c01d65d4e445a802cf69e261676ba5e2a7e0ae57949e2ce5e40bf99ce"]'
        let test_cases = vec![
            TestCase{
                instant_lock_hex: "010c828b35aa1ba4ec8ff7b34335f8d4179080f2ed3a0d63f349b8adc868c73ff201010000007a46cb5ef53943cec615f9054acc460418129e4f13ee33cc1f5c39707c11401b020000005d63eaab8d9d29e48e041b9f5b43e3e41b5dac21cd998e703b631198b77b731c0c0000002f7f367e3e9461b8c6c2d919f276a0e344e673dfc84da21b8ff9606fa64f4429050000009ca17ff3474487cfc718e871696bd1d4d7669efced04aa5cdb4cc69f906da639000000009ca17ff3474487cfc718e871696bd1d4d7669efced04aa5cdb4cc69f906da639010000002e35829452aae4996c47ead73b20183bad78e83cb29185f4be9ca60b80e8c06208000000e9ec07776bc2337daa6ee7549b9ba38d8782f0581a21555bc9dde8156c713d6c080000009ba8c0f9ddc2531b576244930e75e66752e8a3dd70d3262ee1b379f58d19ba7d06000000efb7d48484544cfa06585494b427862f098708707ffa1610b4ed33a724bfe4d002000000efb7d48484544cfa06585494b427862f098708707ffa1610b4ed33a724bfe4d00300000042f2373701b15830d8fba26001b4480963db6cf238936eb8d6705ec3fe4cc9f801000000ce99bf405ecee24979e50a7e2a5eba7616269ef62c805a444e5dd6016c63cae72ed3582b963bbda2fdec219ff84299c525dfadcf4b69218a1400000000000000b0f624683b04c01ad58bcf6233f6e492faf0e7f9adf0609dab04a97796aabf13175262c74056690610c075d8cce2cc9f12549b8d5ab3484be19b9c3ebcc9c29dd8d8762bb6e940cc79acfc2940f709a3fa5ff0be11c1304c219931ce58dbac95",
                quorum_hash: "000000000000000e1fd2afb1f179b1f5c2f50a3614da93c0e40c4a1ddd921200",
                quorum_public_key: "a1ce5102d30de044adccb2a64d1120db355d222a1478b5e2a3f4bf7f90895ac01eccde2ff8cec27c9a886bdfa0b83b1a",
                correct_signature: true,
            },
            TestCase{
                instant_lock_hex: "010c828b35aa1ba4ec8ff7b34335f8d4179080f2ed3a0d63f349b8adc868c73ff201010000007a46cb5ef53943cec615f9054acc460418129e4f13ee33cc1f5c39707c11401b020000005d63eaab8d9d29e48e041b9f5b43e3e41b5dac21cd998e703b631198b77b731c0c0000002f7f367e3e9461b8c6c2d919f276a0e344e673dfc84da21b8ff9606fa64f4429050000009ca17ff3474487cfc718e871696bd1d4d7669efced04aa5cdb4cc69f906da639000000009ca17ff3474487cfc718e871696bd1d4d7669efced04aa5cdb4cc69f906da639010000002e35829452aae4996c47ead73b20183bad78e83cb29185f4be9ca60b80e8c06208000000e9ec07776bc2337daa6ee7549b9ba38d8782f0581a21555bc9dde8156c713d6c080000009ba8c0f9ddc2531b576244930e75e66752e8a3dd70d3262ee1b379f58d19ba7d06000000efb7d48484544cfa06585494b427862f098708707ffa1610b4ed33a724bfe4d002000000efb7d48484544cfa06585494b427862f098708707ffa1610b4ed33a724bfe4d00300000042f2373701b15830d8fba26001b4480963db6cf238936eb8d6705ec3fe4cc9f801000000ce99bf405ecee24979e50a7e2a5eba7616269ef62c805a444e5dd6016c63cae72ed3582b963bbda2fdec219ff84299c525dfadcf4b69218a1400000000000000b0f624683b04c01ad58bcf6233f6e492faf0e7f9adf0609dab04a97796aabf13175262c74056690610c075d8cce2cc9f12549b8d5ab3484be19b9c3ebcc9c29dd8d8762bb6e940cc79acfc2940f709a3fa5ff0be11c1304c219931ce58dbac95",
                quorum_hash: "000000000000000dce79f58fadb53bb32ada6c6f817b1e96e0b95a276248fa7a",
                quorum_public_key: "a1ce5102d30de044adccb2a64d1120db355d222a1478b5e2a3f4bf7f90895ac01eccde2ff8cec27c9a886bdfa0b83b1a",
                correct_signature: false,
            },TestCase{
                instant_lock_hex: "010c828b35aa1ba4ec8ff7b34335f8d4179080f2ed3a0d63f349b8adc868c73ff201010000007a46cb5ef53943cec615f9054acc460418129e4f13ee33cc1f5c39707c11401b020000005d63eaab8d9d29e48e041b9f5b43e3e41b5dac21cd998e703b631198b77b731c0c0000002f7f367e3e9461b8c6c2d919f276a0e344e673dfc84da21b8ff9606fa64f4429050000009ca17ff3474487cfc718e871696bd1d4d7669efced04aa5cdb4cc69f906da639000000009ca17ff3474487cfc718e871696bd1d4d7669efced04aa5cdb4cc69f906da639010000002e35829452aae4996c47ead73b20183bad78e83cb29185f4be9ca60b80e8c06208000000e9ec07776bc2337daa6ee7549b9ba38d8782f0581a21555bc9dde8156c713d6c080000009ba8c0f9ddc2531b576244930e75e66752e8a3dd70d3262ee1b379f58d19ba7d06000000efb7d48484544cfa06585494b427862f098708707ffa1610b4ed33a724bfe4d002000000efb7d48484544cfa06585494b427862f098708707ffa1610b4ed33a724bfe4d00300000042f2373701b15830d8fba26001b4480963db6cf238936eb8d6705ec3fe4cc9f801000000ce99bf405ecee24979e50a7e2a5eba7616269ef62c805a444e5dd6016c63cae72ed3582b963bbda2fdec219ff84299c525dfadcf4b69218a1400000000000000b0f624683b04c01ad58bcf6233f6e492faf0e7f9adf0609dab04a97796aabf13175262c74056690610c075d8cce2cc9f12549b8d5ab3484be19b9c3ebcc9c29dd8d8762bb6e940cc79acfc2940f709a3fa5ff0be11c1304c219931ce58dbac95",
                quorum_hash: "000000000000000e1fd2afb1f179b1f5c2f50a3614da93c0e40c4a1ddd921200",
                quorum_public_key: "aa3d8543ec8e228b548ddcdee60de91b3bfdb1639b5d4ce295c9e8397985e14ebf2a0c371fb9b4d92354589af5ea2683",
                correct_signature: false,
            },
        ];

        for tc in test_cases {
            let hex_decoded = hex::decode(tc.instant_lock_hex).unwrap();
            let instant_lock: InstantLock = deserialize(&hex_decoded).unwrap();
            let quorum_hash = hex::decode(tc.quorum_hash).expect("correct hash");
            let quorum_public_key: [u8; 48] = hex::decode(tc.quorum_public_key)
                .expect("correct public key")
                .try_into()
                .expect("correct public key len");

            assert_eq!(
                verify_instant_lock_signature(
                    &instant_lock,
                    &INSTANT_SEND_LOCK_QUORUM_TYPE,
                    &quorum_hash,
                    &quorum_public_key,
                )
                .expect("signature verification failed"),
                tc.correct_signature
            );
        }
    }
}
