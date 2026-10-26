//! Proof-backed state of a stored asset-lock outpoint.

use crate::platform::{Query, QuerySettings};
use crate::Error;
use dapi_grpc::platform::v0::get_asset_lock_outpoint_info_request::GetAssetLockOutpointInfoRequestV0;
use dapi_grpc::platform::v0::{
    get_asset_lock_outpoint_info_request, GetAssetLockOutpointInfoRequest,
};
use dpp::dashcore::{hashes::Hash, OutPoint};

/// Query for the state of one asset-lock outpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetLockOutpointInfoQuery {
    /// The Core outpoint to inspect.
    pub outpoint: OutPoint,
}

fn request_for_outpoint(
    outpoint: &OutPoint,
    settings: &QuerySettings<'_>,
) -> Result<GetAssetLockOutpointInfoRequest, Error> {
    let mut bytes = outpoint.txid.to_byte_array().to_vec();
    bytes.extend_from_slice(&outpoint.vout.to_le_bytes());
    Ok(GetAssetLockOutpointInfoRequest {
        version: Some(get_asset_lock_outpoint_info_request::Version::V0(
            GetAssetLockOutpointInfoRequestV0 {
                outpoint: bytes,
                prove: settings.prove,
            },
        )),
    })
}

impl Query<GetAssetLockOutpointInfoRequest> for AssetLockOutpointInfoQuery {
    fn query(
        &self,
        settings: &QuerySettings<'_>,
    ) -> Result<GetAssetLockOutpointInfoRequest, Error> {
        request_for_outpoint(&self.outpoint, settings)
    }
}

impl Query<GetAssetLockOutpointInfoRequest> for OutPoint {
    fn query(
        &self,
        settings: &QuerySettings<'_>,
    ) -> Result<GetAssetLockOutpointInfoRequest, Error> {
        request_for_outpoint(self, settings)
    }
}
