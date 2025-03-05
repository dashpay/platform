//! [AssetLockProof] utilities

use crate::{Error, Sdk};
use dapi_grpc::platform::v0::get_epochs_info_request::{self, GetEpochsInfoRequestV0};
use dapi_grpc::platform::v0::GetEpochsInfoRequest;
use dapi_grpc::platform::VersionedGrpcResponse;
use dpp::prelude::AssetLockProof;
use rs_dapi_client::{DapiRequestExecutor, RequestSettings};

#[async_trait::async_trait]
pub trait AssetLockProofVerifier {
    /// Verifies the asset lock proof against the platform.
    ///
    /// This function verifies some assertions that are necessary for the asset lock proof to be used by Dash Platform,
    /// and errors if any of them are not met.
    ///
    /// Verification involves fetching some information from DAPI and comparing it with the provided asset lock proof.
    ///
    /// Note that positive verification result does not imply that the asset lock proof is valid. Dash Platform can
    /// still reject the asset lock.
    ///
    /// # Limitations
    ///
    /// Only [AssetLockProof::Chain] is supported.
    ///
    /// # Errors
    ///
    /// - [Error::CoreLockedHeightNotYetAvailable] if the core locked height in the proof is higher than the
    /// current core locked height on the platform. Try again later.
    /// - [Error::QuorumNotFound] if the quorum public key is not yet available on the platform, what implies that
    /// the quorum is not (yet) available. Try again later.
    /// - [Error::InvalidSignature] if the signature in the proof is invalid.
    /// - other errors when something goes wrong.
    ///
    /// # Unstable
    ///
    /// This function is unstable and may change in the future.
    async fn verify(&self, sdk: &Sdk) -> Result<(), Error>;
}

#[async_trait::async_trait]
impl AssetLockProofVerifier for AssetLockProof {
    async fn verify(&self, sdk: &Sdk) -> Result<(), Error> {
        match self {
            AssetLockProof::Chain(asset_lock) => {
                let platform_core_chain_locked_height = fetch_platform_locked_height(sdk).await?;
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
                // To verify instant asset lock, we need to:
                //
                // 1. Determine quorum hash used to sign it.
                // 2. Detch quorum public key for this hash.
                // 3. Verify instant asset lock signature.
                //
                // Unfortunately, determining quorum used to sign the instant asset lock is not straightforward,
                // as it requires processing of SML which is not implemented in the SDK yet.
                //
                // So we just accept the instant asset lock as valid for now.

                Ok(())
            }
        }
    }
}

/// Fetches the current core chain locked height from the platform.
async fn fetch_platform_locked_height(sdk: &Sdk) -> Result<u32, Error> {
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

    Ok(response.metadata()?.core_chain_locked_height)
}
