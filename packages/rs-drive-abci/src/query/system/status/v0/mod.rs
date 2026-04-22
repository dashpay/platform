use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformStateV0Methods;
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
        let next_epoch_version = platform_state.next_epoch_protocol_version();

        let version = get_status_response_v0::Version {
            protocol: Some(get_status_response_v0::version::Protocol {
                tenderdash: None,
                drive: Some(get_status_response_v0::version::protocol::Drive {
                    latest: latest_supported_protocol_version,
                    current: platform_state.current_protocol_version_in_consensus(),
                    next_epoch: next_epoch_version,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::tests::setup_platform;
    use dpp::dashcore::Network;

    #[test]
    fn test_query_partial_status_default_state() {
        let (platform, state, _version) = setup_platform(None, Network::Testnet, None);

        let request = GetStatusRequestV0 {};

        let result = platform
            .query_partial_status_v0(request, &state)
            .expect("expected query to succeed");

        let data = result.into_data().expect("expected data");

        // Version structure must be populated with Drive protocol info
        let version = data.version.expect("expected version");
        let protocol = version.protocol.expect("expected protocol section");
        let drive_proto = protocol.drive.expect("expected drive protocol section");
        assert_eq!(
            drive_proto.latest,
            PlatformVersion::latest().protocol_version
        );
        assert_eq!(
            drive_proto.current,
            state.current_protocol_version_in_consensus()
        );

        // Software version must be the crate version string
        let software = version.software.expect("expected software section");
        assert_eq!(software.drive, Some(env!("CARGO_PKG_VERSION").to_string()));
        assert_eq!(software.dapi, "".to_string());

        // Chain info reports last committed core height and empty placeholders
        let chain = data.chain.expect("expected chain section");
        assert!(!chain.catching_up);
        assert!(chain.latest_block_hash.is_empty());
        assert!(chain.latest_app_hash.is_empty());
        assert_eq!(chain.latest_block_height, 0);
        assert_eq!(
            chain.core_chain_locked_height,
            Some(state.last_committed_core_height())
        );

        // Time info includes epoch and defaults elsewhere
        let time = data.time.expect("expected time section");
        assert_eq!(time.local, 0);
        assert_eq!(
            time.epoch,
            Some(state.last_committed_block_epoch().index as u32)
        );

        // Optional sections are None at genesis
        assert!(data.node.is_none());
        assert!(data.network.is_none());
        assert!(data.state_sync.is_none());
    }
}
