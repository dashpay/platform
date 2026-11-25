//! Proof-backed state of a stored asset-lock outpoint.

use crate::platform::{Query, QuerySettings};
use crate::Error;
use dapi_grpc::platform::v0::get_asset_lock_outpoint_info_request::GetAssetLockOutpointInfoRequestV0;
use dapi_grpc::platform::v0::{
    get_asset_lock_outpoint_info_request, GetAssetLockOutpointInfoRequest,
};
use dpp::dashcore::OutPoint;

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
    if !settings.prove {
        return Err(Error::Config(
            "unproved queries are not supported for this query; enable proofs".to_string(),
        ));
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::version::PlatformVersion;
    use rs_dapi_client::RequestSettings;

    #[test]
    fn should_encode_outpoint_in_internal_txid_and_little_endian_index_order() {
        let settings = QuerySettings {
            request_settings: &RequestSettings::default(),
            protocol_version: PlatformVersion::latest(),
            verified_protocol_version: 0,
            prove: true,
        };
        let txid_bytes = std::array::from_fn(|i| i as u8);
        let outpoint = OutPoint::new(dpp::dashcore::Txid::from_byte_array(txid_bytes), 0x01020304);
        let query = AssetLockOutpointInfoQuery { outpoint };
        let request = query.query(&settings).expect("proved request");
        assert_eq!(
            request,
            outpoint.query(&settings).expect("outpoint request")
        );
        let Some(get_asset_lock_outpoint_info_request::Version::V0(v0)) = request.version else {
            panic!("expected a V0 request");
        };
        assert_eq!(&v0.outpoint[..32], &txid_bytes);
        assert_eq!(&v0.outpoint[32..], &[4, 3, 2, 1]);
        assert!(v0.prove);
    }

    #[test]
    fn should_reject_unproved_asset_lock_queries() {
        let settings = QuerySettings {
            request_settings: &RequestSettings::default(),
            protocol_version: PlatformVersion::latest(),
            verified_protocol_version: 0,
            prove: false,
        };
        let outpoint = OutPoint::null();
        let query = AssetLockOutpointInfoQuery { outpoint };
        assert!(matches!(query.query(&settings), Err(Error::Config(_))));
        assert!(matches!(outpoint.query(&settings), Err(Error::Config(_))));
    }
}
