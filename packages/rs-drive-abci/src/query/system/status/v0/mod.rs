use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::query::QueryValidationResult;
use dapi_grpc::platform::v0::get_status_request::GetStatusRequestV0;
use dapi_grpc::platform::v0::get_status_response::{get_status_response_v0, GetStatusResponseV0};

use crate::platform_types::platform_state::PlatformState;
use dpp::version::PlatformVersion;

impl<C> Platform<C> {
    pub(super) fn query_partial_status_v0(
        &self,
        _request: GetStatusRequestV0,
        platform_state: &PlatformState,
    ) -> Result<QueryValidationResult<GetStatusResponseV0>, Error> {
        let latest_supported_protocol_version = PlatformVersion::latest().protocol_version;

        let version = get_status_response_v0::Version {
            protocol: Some(get_status_response_v0::version::Protocol {
                tenderdash: None,
                drive: Some(get_status_response_v0::version::protocol::Drive {
                    latest: latest_supported_protocol_version,
                    current: platform_state.current_protocol_version_in_consensus(),
                }),
            }),
            software: Some(get_status_response_v0::version::Software {
                dapi: "".to_string(),
                drive: Some(env!("CARGO_PKG_VERSION").to_string()),
                tenderdash: None,
            }),
        };

        let chain = get_status_response_v0::Chain {
            catching_up: false,
            latest_block_hash: vec![],
            latest_app_hash: vec![],
            latest_block_height: 0,
            earliest_block_hash: vec![],
            earliest_app_hash: vec![],
            earliest_block_height: 0,
            max_peer_block_height: 0,
            core_chain_locked_height: Some(platform_state.last_committed_core_height()),
        };

        let time = get_status_response_v0::Time {
            local: 0,
            block: Some(
                platform_state
                    .last_committed_block_time_ms()
                    .unwrap_or_default(),
            ),
            genesis: Some(
                platform_state
                    .genesis_block_info()
                    .map(|info| info.time_ms)
                    .unwrap_or_default(),
            ),
            epoch: Some(platform_state.last_committed_block_epoch().index as u32),
        };

        let response = GetStatusResponseV0 {
            version: Some(version),
            node: None,
            chain: Some(chain),
            network: None,
            state_sync: None,
            time: Some(time),
        };

        Ok(QueryValidationResult::new_with_data(response))
    }
}
