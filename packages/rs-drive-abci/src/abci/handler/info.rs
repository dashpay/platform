use crate::abci::app::PlatformApplication;
use crate::abci::AbciError;
use crate::error::Error;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::rpc::core::CoreRPCLike;
use dpp::dashcore::Network;
use dpp::version::DESIRED_PLATFORM_VERSION;
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

    let last_block_height = platform_state.last_committed_block_height() as i64;

    // Verify that Platform State corresponds to Drive commited state
    let drive_storage_root_hash = platform_state
        .last_committed_block_app_hash()
        .unwrap_or_default();

    let platform_state_app_hash = app
        .platform()
        .drive
        .grove
        .root_hash(
            None,
            &platform_state
                .current_platform_version()?
                .drive
                .grove_version,
        )
        .unwrap()?;

    // TODO: Document this
    // TODO: verify that chain id is evo1
    #[allow(clippy::collapsible_if)]
    if !(app.platform().config.network == Network::Dash && last_block_height == 32326) {
        if drive_storage_root_hash != platform_state_app_hash {
            return Err(AbciError::AppHashMismatch {
                drive_storage_root_hash,
                platform_state_app_hash,
            }
            .into());
        }
    }

    let desired_protocol_version = DESIRED_PLATFORM_VERSION.protocol_version;

    let response = proto::ResponseInfo {
        data: "".to_string(),
        app_version: desired_protocol_version as u64,
        last_block_height,
        version: env!("CARGO_PKG_VERSION").to_string(),
        last_block_app_hash: platform_state_app_hash.to_vec(),
    };

    tracing::debug!(
        desired_protocol_version,
        software_version = env!("CARGO_PKG_VERSION"),
        block_version = request.block_version,
        p2p_version = request.p2p_version,
        app_hash = hex::encode(platform_state_app_hash),
        last_block_height,
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
