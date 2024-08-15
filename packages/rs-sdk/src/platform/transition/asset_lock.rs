//! [AssetLockProof] utilities

use crate::{Error, Sdk};
use dapi_grpc::platform::v0::get_epochs_info_request::{self, GetEpochsInfoRequestV0};
use dapi_grpc::platform::v0::GetEpochsInfoRequest;
use dapi_grpc::platform::VersionedGrpcResponse;
use dpp::dashcore::hashes::Hash;
use dpp::prelude::AssetLockProof;
use drive_proof_verifier::error::ContextProviderError;
use drive_proof_verifier::ContextProvider;
use rs_dapi_client::{DapiRequestExecutor, RequestSettings};
#[async_trait::async_trait]
pub trait AssetLockProofVerifier {
    /// Verifies the asset lock proof against the platform.
    ///
    /// This function will return an error if the proof is not yet available on the platform.
    ///
    /// # Errors
    ///
    /// - [Error::CoreLockedHeightNotYetAvailable] if the core locked height in the proof is higher than the
    /// current core locked height on the platform. Try again later.
    /// - [Error::QuorumNotFound] if the quorum public key is not yet available on the platform, what implies that
    /// the quorum is not (yet) available. Try again later.
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
            AssetLockProof::Instant(v) => {
                let quorum_hash = v.instant_lock().cyclehash.to_raw_hash().to_byte_array();
                let quorum_type = sdk.quorum_params().instant_lock_quorum_type;
                // Try to fetch the quorum public key; if it fails, we assume platform does not have this quorum yet
                match context_provider.get_quorum_public_key(
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
                    Ok(_) => Ok(()),
                }
            }
        }
    }
}
