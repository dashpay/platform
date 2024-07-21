use crate::abci::app::PlatformApplication;
use crate::abci::AbciError;
use crate::error::Error;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::rpc::core::CoreRPCLike;
use dpp::version::PlatformVersion;
use tenderdash_abci::proto::abci as proto;

pub fn info<A, C>(app: &A, request: proto::RequestInfo) -> Result<proto::ResponseInfo, Error>
where
    A: PlatformApplication<C>,
    C: CoreRPCLike,
{
    if !tenderdash_abci::check_version(&request.abci_version) {
        return Err(AbciError::AbciVersionMismatch {
            tenderdash: request.abci_version,
            drive: tenderdash_abci::proto::ABCI_VERSION.to_string(),
        }
        .into());
    }

    let platform_state = app.platform().state.load();

    let state_app_hash = platform_state
        .last_committed_block_app_hash()
        .map(|app_hash| app_hash.to_vec())
        .unwrap_or_default();

    let latest_supported_protocol_version = PlatformVersion::latest().protocol_version;

    let response = proto::ResponseInfo {
        data: "".to_string(),
        app_version: latest_supported_protocol_version as u64,
        last_block_height: platform_state.last_committed_block_height() as i64,
        version: env!("CARGO_PKG_VERSION").to_string(),
        last_block_app_hash: state_app_hash.clone(),
    };

    tracing::debug!(
        latest_supported_protocol_version,
        software_version = env!("CARGO_PKG_VERSION"),
        block_version = request.block_version,
        p2p_version = request.p2p_version,
        app_hash = hex::encode(state_app_hash),
        height = platform_state.last_committed_block_height(),
        "Handshake with consensus engine",
    );

    if tracing::enabled!(tracing::Level::TRACE) {
        tracing::trace!(
            platform_state_fingerprint = hex::encode(platform_state.fingerprint()?),
            "platform runtime state",
        );
    }

    Ok(response)
}
