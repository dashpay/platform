use crate::error::MapGroveDbError;
use crate::verify::{supported_grovedb_proof_bytes, verify_tenderdash_proof};
use crate::{ContextProvider, Error, FromProof};
use dapi_grpc::platform::v0::{
    get_asset_lock_outpoint_info_request, GetAssetLockOutpointInfoRequest,
    GetAssetLockOutpointInfoResponse, Proof, ResponseMetadata,
};
use dapi_grpc::platform::VersionedGrpcResponse;
use dpp::asset_lock::StoredAssetLockInfo;
use dpp::dashcore::Network;
use dpp::platform_value::Bytes36;
use dpp::version::PlatformVersion;
use drive::drive::Drive;

impl FromProof<GetAssetLockOutpointInfoRequest> for StoredAssetLockInfo {
    type Request = GetAssetLockOutpointInfoRequest;
    type Response = GetAssetLockOutpointInfoResponse;

    fn maybe_from_proof_with_metadata<'a, I: Into<Self::Request>, O: Into<Self::Response>>(
        request: I,
        response: O,
        _network: Network,
        platform_version: &PlatformVersion,
        provider: &'a dyn ContextProvider,
    ) -> Result<(Option<Self>, ResponseMetadata, Proof), Error>
    where
        Self: Sized + 'a,
    {
        let request = request.into();
        let response = response.into();
        let outpoint = match request.version.ok_or(Error::EmptyVersion)? {
            get_asset_lock_outpoint_info_request::Version::V0(v0) => Bytes36::from_vec(v0.outpoint)
                .map_err(|_| Error::RequestError {
                    error: "outpoint must be exactly 36 bytes".to_string(),
                })?,
        };
        let metadata = response
            .metadata()
            .or(Err(Error::EmptyResponseMetadata))?
            .clone();
        let proof = response.proof_owned().or(Err(Error::NoProofInResult))?;
        let (root_hash, info) = Drive::verify_asset_lock_outpoint_info(
            supported_grovedb_proof_bytes(&proof)?,
            &outpoint,
            platform_version,
        )
        .map_drive_error(&proof, &metadata)?;
        verify_tenderdash_proof(&proof, &metadata, &root_hash, provider)?;
        Ok((Some(info), metadata, proof))
    }
}
