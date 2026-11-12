use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::response_metadata::CheckpointUsed;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_asset_lock_outpoint_info_request::GetAssetLockOutpointInfoRequestV0;
use dapi_grpc::platform::v0::get_asset_lock_outpoint_info_response::{
    get_asset_lock_outpoint_info_response_v0, AssetLockOutpointInfo,
    GetAssetLockOutpointInfoResponseV0, Status,
};
use dpp::asset_lock::{reduced_asset_lock_value::AssetLockValueGettersV0, StoredAssetLockInfo};
use dpp::platform_value::Bytes36;
use dpp::version::PlatformVersion;
use drive::util::grove_operations::GroveDBToUse;

impl<C> Platform<C> {
    pub(super) fn query_asset_lock_outpoint_info_v0(
        &self,
        request: GetAssetLockOutpointInfoRequestV0,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetAssetLockOutpointInfoResponseV0>, Error> {
        let outpoint = match Bytes36::from_vec(request.outpoint) {
            Ok(outpoint) => outpoint,
            Err(_) => {
                return Ok(QueryValidationResult::new_with_error(
                    QueryError::InvalidArgument("outpoint must be exactly 36 bytes".to_string()),
                ));
            }
        };
        let result = if request.prove {
            let proof =
                self.drive
                    .prove_asset_lock_outpoint_info(&outpoint, None, platform_version)?;
            GetAssetLockOutpointInfoResponseV0 {
                result: Some(get_asset_lock_outpoint_info_response_v0::Result::Proof(
                    self.response_proof_v0(platform_state, proof, GroveDBToUse::Current)
                        .map(|(_, proof)| proof)?,
                )),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        } else {
            let info = self.drive.fetch_asset_lock_outpoint_info(
                &outpoint,
                None,
                &platform_version.drive,
            )?;
            GetAssetLockOutpointInfoResponseV0 {
                result: Some(
                    get_asset_lock_outpoint_info_response_v0::Result::AssetLockOutpointInfo(
                        asset_lock_outpoint_info(info),
                    ),
                ),
                metadata: Some(self.response_metadata_v0(platform_state, CheckpointUsed::Current)),
            }
        };
        Ok(QueryValidationResult::new_with_data(result))
    }
}

fn asset_lock_outpoint_info(info: StoredAssetLockInfo) -> AssetLockOutpointInfo {
    match info {
        StoredAssetLockInfo::NotPresent => AssetLockOutpointInfo {
            status: Status::NotPresent as i32,
            remaining_credits: None,
        },
        StoredAssetLockInfo::PartiallyConsumed(value) => AssetLockOutpointInfo {
            status: Status::PartiallyConsumed as i32,
            remaining_credits: Some(value.remaining_credit_value()),
        },
        StoredAssetLockInfo::FullyConsumed => AssetLockOutpointInfo {
            status: Status::FullyConsumed as i32,
            remaining_credits: Some(0),
        },
    }
}
